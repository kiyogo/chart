//! CWBF2 Encrypted Binary RPC & Chunked Media Multiplexing Protocol (Spec 071).
//!
//! Matches Telegram Web K (`tweb`) wire architecture:
//! - Ephemeral ECDH P-256 + HKDF-SHA256 session key agreement (zero plaintext key on wire)
//! - AES-256-GCM AEAD per frame with full 34-byte header authenticated as AAD
//! - Strict monotonic `seq` counter anti-replay protection
//! - Automatic Zlib deflate compression for payloads > 512 bytes (RFC 1950, native browser `deflate`)
//! - 16-byte aligned CSPRNG random padding against traffic size analysis
//! - Concurrent in-process `Router::oneshot` RPC dispatch (zero duplicate auth/RBAC/DB logic)
//! - O(1) seek range chunk streaming (`OP_MEDIA_CHUNK_REQ`), resumable chunked uploads (`OP_UPLOAD_PART`),
//!   and cursor-based PTS delta sync (`OP_DELTA_SYNC_REQ`).

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use axum::{
    body::{Body, to_bytes},
    extract::ws::{Message, WebSocket},
    http::{HeaderValue, Method, Request, header},
};
use flate2::{
    Compression,
    read::ZlibDecoder,
    write::ZlibEncoder,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Semaphore, mpsc};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    auth::{AppState, session::SESSION_COOKIE_NAME},
    websocket::{
        CWBF_MAGIC, OP_EVENT, OP_HELLO, OP_HELLO_ACK, OP_PING, OP_PONG, WsEvent, WsHub,
        derive_cwbf_session_key, generate_ephemeral_p256,
    },
};

pub const CWBF2_VERSION: u8 = 2;
pub const CWBF2_HEADER_LEN: usize = 34;

pub const OP_RPC_CALL: u8 = 0x10;
pub const OP_RPC_RES: u8 = 0x11;
pub const OP_MEDIA_CHUNK_REQ: u8 = 0x12;
pub const OP_MEDIA_CHUNK_RES: u8 = 0x13;
pub const OP_UPLOAD_PART: u8 = 0x14;
pub const OP_UPLOAD_ACK: u8 = 0x15;
pub const OP_DELTA_SYNC_REQ: u8 = 0x16;
pub const OP_DELTA_SYNC_RES: u8 = 0x17;

pub const FLAG_COMPRESSED: u8 = 0x01;
pub const FLAG_PADDED: u8 = 0x02;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(6);
const PING_INTERVAL: Duration = Duration::from_secs(30);
const MAX_CONCURRENT_RPCS: usize = 16;
const MAX_RPC_BODY_BYTES: usize = 36 * 1024 * 1024;
const COMPRESS_THRESHOLD: usize = 512;

fn build_cwbf2_header(
    op: u8,
    flags: u8,
    seq: u64,
    req_id: u32,
    nonce: &[u8; 12],
    payload_len: u32,
) -> [u8; CWBF2_HEADER_LEN] {
    let mut hdr = [0u8; CWBF2_HEADER_LEN];
    hdr[0] = CWBF_MAGIC[0];
    hdr[1] = CWBF_MAGIC[1];
    hdr[2] = CWBF2_VERSION;
    hdr[3] = op;
    hdr[4] = flags;
    hdr[5] = 0;
    hdr[6..14].copy_from_slice(&seq.to_be_bytes());
    hdr[14..18].copy_from_slice(&req_id.to_be_bytes());
    hdr[18..30].copy_from_slice(nonce);
    hdr[30..34].copy_from_slice(&payload_len.to_be_bytes());
    hdr
}

pub fn encode_cwbf2_plain(op: u8, seq: u64, req_id: u32, payload: &[u8]) -> Vec<u8> {
    let hdr = build_cwbf2_header(op, 0, seq, req_id, &[0u8; 12], payload.len() as u32);
    let mut out = Vec::with_capacity(CWBF2_HEADER_LEN + payload.len());
    out.extend_from_slice(&hdr);
    out.extend_from_slice(payload);
    out
}

pub struct ParsedCwbf2Header<'a> {
    pub op: u8,
    pub flags: u8,
    pub seq: u64,
    pub req_id: u32,
    pub nonce: [u8; 12],
    pub header_bytes: &'a [u8],
    pub payload: &'a [u8],
}

pub fn decode_cwbf2_header(frame: &[u8]) -> Option<ParsedCwbf2Header<'_>> {
    if frame.len() < CWBF2_HEADER_LEN {
        return None;
    }
    if frame[0..2] != CWBF_MAGIC || frame[2] != CWBF2_VERSION {
        return None;
    }
    let op = frame[3];
    let flags = frame[4];
    let seq = u64::from_be_bytes(frame[6..14].try_into().ok()?);
    let req_id = u32::from_be_bytes(frame[14..18].try_into().ok()?);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&frame[18..30]);
    let len = u32::from_be_bytes(frame[30..34].try_into().ok()?) as usize;
    if frame.len() != CWBF2_HEADER_LEN + len {
        return None;
    }
    Some(ParsedCwbf2Header {
        op,
        flags,
        seq,
        req_id,
        nonce,
        header_bytes: &frame[..CWBF2_HEADER_LEN],
        payload: &frame[CWBF2_HEADER_LEN..],
    })
}

fn pad_16(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let rem = data.len() % 16;
    let base_pad = if rem == 0 { 16 } else { 16 - rem };
    // Add 0 or 16 random extra bytes for small frames to obscure traffic length fingerprints.
    let mut extra_byte = [0u8; 1];
    getrandom::fill(&mut extra_byte).map_err(|_| "csprng_failed")?;
    let extra = if data.len() < 256 && (extra_byte[0] & 1) == 1 { 16 } else { 0 };
    let pad_len = base_pad + extra; // strictly in 1..=32
    let mut out = Vec::with_capacity(data.len() + pad_len);
    out.extend_from_slice(data);
    let rand_count = pad_len - 1;
    if rand_count > 0 {
        let mut rbuf = [0u8; 32];
        getrandom::fill(&mut rbuf[..rand_count]).map_err(|_| "csprng_failed")?;
        out.extend_from_slice(&rbuf[..rand_count]);
    }
    out.push(pad_len as u8);
    Ok(out)
}

fn unpad_16(data: &[u8]) -> Result<&[u8], &'static str> {
    if data.is_empty() || data.len() % 16 != 0 {
        return Err("invalid_padded_length");
    }
    let pad_len = *data.last().unwrap() as usize;
    if pad_len == 0 || pad_len > 32 || pad_len > data.len() {
        return Err("invalid_pad_byte");
    }
    Ok(&data[..data.len() - pad_len])
}

#[allow(deprecated)]
pub fn encode_cwbf2_encrypted(
    cipher: &Aes256Gcm,
    op: u8,
    seq: u64,
    req_id: u32,
    raw_plaintext: &[u8],
    allow_compress: bool,
) -> Result<Vec<u8>, &'static str> {
    let mut flags = FLAG_PADDED;
    let body_buf: Vec<u8>;
    let body_slice: &[u8] = if allow_compress && raw_plaintext.len() > COMPRESS_THRESHOLD {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        if enc.write_all(raw_plaintext).is_ok()
            && let Ok(compressed) = enc.finish()
            && compressed.len() < raw_plaintext.len()
        {
            flags |= FLAG_COMPRESSED;
            body_buf = compressed;
            &body_buf
        } else {
            raw_plaintext
        }
    } else {
        raw_plaintext
    };

    let padded = pad_16(body_slice)?;
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|_| "csprng_failed")?;
    let ct_len = padded
        .len()
        .checked_add(16)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or("frame_too_large")?;
    let hdr = build_cwbf2_header(op, flags, seq, req_id, &nonce_bytes, ct_len);
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: &padded,
                aad: &hdr,
            },
        )
        .map_err(|_| "aead_encrypt_failed")?;
    let mut out = Vec::with_capacity(CWBF2_HEADER_LEN + ct.len());
    out.extend_from_slice(&hdr);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[allow(deprecated)]
pub fn decode_cwbf2_encrypted(
    cipher: &Aes256Gcm,
    frame: &[u8],
) -> Result<(u8, u64, u32, Vec<u8>), &'static str> {
    let h = decode_cwbf2_header(frame).ok_or("invalid_header")?;
    let decrypted = cipher
        .decrypt(
            Nonce::from_slice(&h.nonce),
            Payload {
                msg: h.payload,
                aad: h.header_bytes,
            },
        )
        .map_err(|_| "aead_decrypt_failed")?;
    let unpadded = if (h.flags & FLAG_PADDED) != 0 {
        unpad_16(&decrypted)?
    } else {
        &decrypted[..]
    };
    let final_bytes = if (h.flags & FLAG_COMPRESSED) != 0 {
        let dec = ZlibDecoder::new(unpadded);
        let mut out = Vec::new();
        dec.take((MAX_RPC_BODY_BYTES + 1) as u64)
            .read_to_end(&mut out)
            .map_err(|_| "zlib_decompress_failed")?;
        if out.len() > MAX_RPC_BODY_BYTES {
            return Err("decompressed_too_large");
        }
        out
    } else {
        unpadded.to_vec()
    };
    Ok((h.op, h.seq, h.req_id, final_bytes))
}

fn is_safe_internal_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 8192 || !path.starts_with('/') {
        return false;
    }
    if path.contains('\0') {
        return false;
    }
    let bare = path.split('?').next().unwrap_or(path);
    if bare.contains("..") || bare.contains('\\') {
        return false;
    }
    if bare == "/api/ws" || bare.starts_with("/stream/") {
        return false;
    }
    bare.starts_with("/api/")
        || bare.starts_with("/uploads/")
        || bare.starts_with("/locales/")
        || bare.starts_with("/stickers/")
        || bare.starts_with("/wallpapers/")
}

fn parse_total_from_content_range(cr: &str) -> Option<u64> {
    let (_, total_str) = cr.split_once('/')?;
    total_str.trim().parse::<u64>().ok()
}

fn encode_rpc_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let ct_bytes = content_type.as_bytes();
    let ct_len = ct_bytes.len().min(u16::MAX as usize);
    let mut out = Vec::with_capacity(4 + ct_len + body.len());
    out.extend_from_slice(&status.to_be_bytes());
    out.extend_from_slice(&(ct_len as u16).to_be_bytes());
    out.extend_from_slice(&ct_bytes[..ct_len]);
    out.extend_from_slice(body);
    out
}

fn encode_media_chunk_response(status: u16, total_size: u64, content_type: &str, chunk: &[u8]) -> Vec<u8> {
    let ct_bytes = content_type.as_bytes();
    let ct_len = ct_bytes.len().min(u16::MAX as usize);
    let mut out = Vec::with_capacity(12 + ct_len + chunk.len());
    out.extend_from_slice(&status.to_be_bytes());
    out.extend_from_slice(&total_size.to_be_bytes());
    out.extend_from_slice(&(ct_len as u16).to_be_bytes());
    out.extend_from_slice(&ct_bytes[..ct_len]);
    out.extend_from_slice(chunk);
    out
}

fn parse_rpc_call_payload(buf: &[u8]) -> Option<(Method, String, String, Vec<(String, String)>, Vec<u8>)> {
    if buf.len() < 6 {
        return None;
    }
    let method = match buf[0] {
        0 => Method::GET,
        1 => Method::POST,
        2 => Method::PUT,
        3 => Method::DELETE,
        4 => Method::PATCH,
        _ => return None,
    };
    let mut pos = 1usize;
    let path_len = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
    pos += 2;
    if pos + path_len + 2 > buf.len() {
        return None;
    }
    let path = std::str::from_utf8(&buf[pos..pos + path_len]).ok()?.to_string();
    pos += path_len;

    let ct_len = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
    pos += 2;
    if pos + ct_len + 1 > buf.len() {
        return None;
    }
    let content_type = std::str::from_utf8(&buf[pos..pos + ct_len]).ok()?.to_string();
    pos += ct_len;

    let hdr_count = buf[pos] as usize;
    pos += 1;
    if hdr_count > 16 {
        return None;
    }
    let mut extra_headers = Vec::with_capacity(hdr_count);
    for _ in 0..hdr_count {
        if pos + 2 > buf.len() {
            return None;
        }
        let kl = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
        pos += 2;
        if pos + kl + 2 > buf.len() {
            return None;
        }
        let k = std::str::from_utf8(&buf[pos..pos + kl]).ok()?.to_string();
        pos += kl;
        let vl = u16::from_be_bytes([buf[pos], buf[pos + 1]]) as usize;
        pos += 2;
        if pos + vl > buf.len() {
            return None;
        }
        let v = std::str::from_utf8(&buf[pos..pos + vl]).ok()?.to_string();
        pos += vl;
        extra_headers.push((k, v));
    }
    let body = buf[pos..].to_vec();
    Some((method, path, content_type, extra_headers, body))
}

async fn invoke_internal_router(
    state: &AppState,
    cookie_header: &str,
    method: Method,
    path: &str,
    content_type: &str,
    extra_headers: &[(String, String)],
    body: Vec<u8>,
) -> (u16, String, Vec<u8>, Option<u64>) {
    let Some(router) = state.rpc_router.get() else {
        return (503, "application/json".into(), br#"{"error":"rpc_router_unavailable"}"#.to_vec(), None);
    };
    if !is_safe_internal_path(path) {
        return (403, "application/json".into(), br#"{"error":"forbidden_rpc_path"}"#.to_vec(), None);
    }

    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie_header)
        .header(header::ORIGIN, state.config.public_origin.as_str())
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-dest", "empty");

    if !content_type.is_empty() {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    for (k, v) in extra_headers {
        let kl = k.to_ascii_lowercase();
        let allowed = kl == "range"
            || kl == "accept"
            || kl.starts_with("x-file-")
            || kl.starts_with("x-scope-")
            || kl.starts_with("x-mime-")
            || kl.starts_with("x-sha256-")
            || kl == "x-export-token"
            || kl == "x-backup-password"
            || kl == "x-chatw-stream-token";
        if allowed
            && let Ok(hv) = HeaderValue::from_str(v)
            && let Ok(hn) = header::HeaderName::from_bytes(kl.as_bytes())
        {
            builder = builder.header(hn, hv);
        }
    }

    let Ok(req) = builder.body(Body::from(body)) else {
        return (400, "application/json".into(), br#"{"error":"invalid_rpc_request"}"#.to_vec(), None);
    };

    match router.clone().oneshot(req).await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ct = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let total_from_range = resp
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_total_from_content_range);
            let cl_total = resp
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            match to_bytes(resp.into_body(), MAX_RPC_BODY_BYTES).await {
                Ok(bytes) => {
                    let total = total_from_range
                        .or(cl_total)
                        .or(Some(bytes.len() as u64));
                    (status, ct, bytes.to_vec(), total)
                }
                Err(_) => (502, "application/json".into(), br#"{"error":"rpc_body_too_large"}"#.to_vec(), None),
            }
        }
        Err(_) => (500, "application/json".into(), br#"{"error":"rpc_internal_error"}"#.to_vec(), None),
    }
}

/// Standard 64-bit sliding anti-replay window (RFC 6479).
/// Rejects `seq == 0`, duplicate sequences, and frames older than `max_seq - 63`
/// while tolerating minor benign out-of-order delivery across concurrent pipelines.
#[derive(Default)]
struct ReplayWindow {
    max_seq: u64,
    bitmap: u64,
}

impl ReplayWindow {
    fn check(&self, seq: u64) -> bool {
        if seq == 0 {
            return false;
        }
        if seq > self.max_seq {
            return true;
        }
        let diff = self.max_seq - seq;
        if diff >= 64 {
            return false;
        }
        (self.bitmap & (1u64 << diff)) == 0
    }

    fn record(&mut self, seq: u64) {
        if seq == 0 {
            return;
        }
        if seq > self.max_seq {
            let shift = seq - self.max_seq;
            if shift >= 64 {
                self.bitmap = 1;
            } else {
                self.bitmap = (self.bitmap << shift) | 1;
            }
            self.max_seq = seq;
        } else {
            let diff = self.max_seq - seq;
            if diff < 64 {
                self.bitmap |= 1u64 << diff;
            }
        }
    }
}

enum OutboundFrame {
    Plain(Vec<u8>),
    Encrypted {
        op: u8,
        req_id: u32,
        payload: Vec<u8>,
        allow_compress: bool,
    },
}

pub async fn handle_socket_rpc_bin(
    socket: WebSocket,
    state: AppState,
    user_id: Uuid,
    session_token: String,
    conn_id: u64,
    mut rx: mpsc::Receiver<WsEvent>,
    hub: Arc<WsHub>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 1. Ephemeral P-256 ECDH + 16-byte salt handshake
    let Ok((server_sk, server_pub_sec1)) = generate_ephemeral_p256() else {
        hub.unregister(&user_id, conn_id).await;
        return;
    };
    let mut salt = [0u8; 16];
    if getrandom::fill(&mut salt).is_err() {
        hub.unregister(&user_id, conn_id).await;
        return;
    }

    let mut hello_payload = Vec::with_capacity(server_pub_sec1.len() + salt.len());
    hello_payload.extend_from_slice(&server_pub_sec1);
    hello_payload.extend_from_slice(&salt);
    let hello_frame = encode_cwbf2_plain(OP_HELLO, 0, 0, &hello_payload);
    if ws_sender.send(Message::Binary(hello_frame.into())).await.is_err() {
        hub.unregister(&user_id, conn_id).await;
        return;
    }

    // Wait for client's OP_HELLO_ACK containing its 65-byte uncompressed P-256 public key
    let handshake_res = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(bytes)) => {
                    if let Some(h) = decode_cwbf2_header(&bytes)
                        && h.op == OP_HELLO_ACK
                    {
                        return derive_cwbf_session_key(&server_sk, h.payload, &salt).ok();
                    }
                    return None;
                }
                Ok(Message::Ping(p)) => {
                    let _ = ws_sender.send(Message::Pong(p)).await;
                }
                Ok(Message::Pong(_)) => {}
                _ => return None,
            }
        }
        None
    })
    .await;

    let Some(Some(aes_key)) = handshake_res.ok() else {
        hub.unregister(&user_id, conn_id).await;
        return;
    };
    let Ok(cipher) = Aes256Gcm::new_from_slice(&aes_key) else {
        hub.unregister(&user_id, conn_id).await;
        return;
    };
    let cipher = Arc::new(cipher);
    let rpc_sem = Arc::new(Semaphore::new(MAX_CONCURRENT_RPCS));
    let cookie_header = Arc::new(format!("{SESSION_COOKIE_NAME}={session_token}"));

    // Outgoing frame queue: a single writer_task assigns strictly monotonic `server_seq`
    // and encrypts in wire order, eliminating any concurrent sequence reordering race.
    let (out_tx, mut out_rx) = mpsc::channel::<OutboundFrame>(256);

    let writer_cipher = cipher.clone();
    let writer_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut server_seq: u64 = 1;
        loop {
            tokio::select! {
                _ = ping_interval.tick() => {
                    if ws_sender.send(Message::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
                maybe_msg = out_rx.recv() => {
                    match maybe_msg {
                        Some(OutboundFrame::Plain(raw)) => {
                            if ws_sender.send(Message::Binary(raw.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(OutboundFrame::Encrypted { op, req_id, payload, allow_compress }) => {
                            let s = server_seq;
                            server_seq = server_seq.saturating_add(1);
                            if let Ok(frame) = encode_cwbf2_encrypted(&writer_cipher, op, s, req_id, &payload, allow_compress)
                                && ws_sender.send(Message::Binary(frame.into())).await.is_err()
                            {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // Forward real-time WsHub events as encrypted CWBF2 OP_EVENT frames
    let event_tx = out_tx.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(json_bytes) = serde_json::to_vec(&event)
                && event_tx
                    .send(OutboundFrame::Encrypted {
                        op: OP_EVENT,
                        req_id: 0,
                        payload: json_bytes,
                        allow_compress: true,
                    })
                    .await
                    .is_err()
            {
                break;
            }
        }
    });

    // 64-bit sliding anti-replay window for client encrypted frames
    let mut replay_win = ReplayWindow::default();

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(bytes)) => {
                let Some(h) = decode_cwbf2_header(&bytes) else {
                    continue;
                };
                if h.op == OP_PING && h.payload.len() <= 32 {
                    let pong = encode_cwbf2_plain(OP_PONG, h.seq, h.req_id, h.payload);
                    if out_tx.send(OutboundFrame::Plain(pong)).await.is_err() {
                        break;
                    }
                    continue;
                }
                // Replay check on encrypted client frames
                if !replay_win.check(h.seq) {
                    continue;
                }
                let Ok((op, seq, req_id, payload)) = decode_cwbf2_encrypted(&cipher, &bytes) else {
                    continue;
                };
                replay_win.record(seq);

                let Ok(permit) = rpc_sem.clone().acquire_owned().await else {
                    break;
                };
                let st = state.clone();
                let ck = cookie_header.clone();
                let otx = out_tx.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    match op {
                        OP_RPC_CALL => {
                            let resp_bytes = if let Some((method, path, ct, hdrs, body)) = parse_rpc_call_payload(&payload) {
                                let (status, resp_ct, resp_body, _) =
                                    invoke_internal_router(&st, &ck, method, &path, &ct, &hdrs, body).await;
                                if status >= 400
                                    && (path.starts_with("/api/messages")
                                        || path.starts_with("/api/attachments")
                                        || path.starts_with("/api/storage/")
                                        || path.starts_with("/api/dm/")
                                        || path.starts_with("/api/groups")
                                        || path.starts_with("/api/channels"))
                                {
                                    let preview = std::str::from_utf8(&resp_body).unwrap_or("");
                                    let path_preview: String = path.chars().take(240).collect();
                                    tracing::warn!(path = %path_preview, status, body = %preview, "cwbf2 rpc rejected");
                                }
                                encode_rpc_response(status, &resp_ct, &resp_body)
                            } else {
                                encode_rpc_response(400, "application/json", br#"{"error":"malformed_rpc_frame"}"#)
                            };
                            let _ = otx
                                .send(OutboundFrame::Encrypted {
                                    op: OP_RPC_RES,
                                    req_id,
                                    payload: resp_bytes,
                                    allow_compress: true,
                                })
                                .await;
                        }
                        OP_MEDIA_CHUNK_REQ => {
                            let resp_bytes = if payload.len() >= 14 {
                                let plen = u16::from_be_bytes([payload[0], payload[1]]) as usize;
                                if 2 + plen + 12 == payload.len()
                                    && let Ok(path) = std::str::from_utf8(&payload[2..2 + plen])
                                {
                                    let offset = u64::from_be_bytes(payload[2 + plen..10 + plen].try_into().unwrap());
                                    let limit = u32::from_be_bytes(payload[10 + plen..14 + plen].try_into().unwrap());
                                    let mut hdrs = Vec::new();
                                    if limit > 0 {
                                        let capped = (limit as u64).min(2 * 1024 * 1024);
                                        let end = offset.saturating_add(capped.saturating_sub(1));
                                        hdrs.push(("range".to_string(), format!("bytes={offset}-{end}")));
                                    }
                                    let (status, ct, body, total_opt) =
                                        invoke_internal_router(&st, &ck, Method::GET, path, "", &hdrs, Vec::new()).await;
                                    let total = total_opt.unwrap_or(body.len() as u64);
                                    encode_media_chunk_response(status, total, &ct, &body)
                                } else {
                                    encode_media_chunk_response(400, 0, "text/plain", b"bad_chunk_req")
                                }
                            } else {
                                encode_media_chunk_response(400, 0, "text/plain", b"short_chunk_req")
                            };
                            // Media chunks are already compressed/encrypted; skip Zlib to keep CPU O(1).
                            let _ = otx
                                .send(OutboundFrame::Encrypted {
                                    op: OP_MEDIA_CHUNK_RES,
                                    req_id,
                                    payload: resp_bytes,
                                    allow_compress: false,
                                })
                                .await;
                        }
                        OP_UPLOAD_PART => {
                            let resp_bytes = if payload.len() >= 6 {
                                let id_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
                                if 2 + id_len + 4 <= payload.len()
                                    && let Ok(uid_str) = std::str::from_utf8(&payload[2..2 + id_len])
                                    && Uuid::parse_str(uid_str).is_ok()
                                {
                                    let part_idx = u32::from_be_bytes(payload[2 + id_len..6 + id_len].try_into().unwrap());
                                    let chunk = payload[6 + id_len..].to_vec();
                                    let path = format!("/api/storage/uploads/{uid_str}/chunks/{part_idx}");
                                    let (status, _, body, _) = invoke_internal_router(
                                        &st,
                                        &ck,
                                        Method::PUT,
                                        &path,
                                        "application/octet-stream",
                                        &[],
                                        chunk,
                                    )
                                    .await;
                                    let mut out = Vec::with_capacity(6 + body.len());
                                    out.extend_from_slice(&status.to_be_bytes());
                                    out.extend_from_slice(&part_idx.to_be_bytes());
                                    out.extend_from_slice(&body);
                                    out
                                } else {
                                    let mut out = Vec::with_capacity(6);
                                    out.extend_from_slice(&400u16.to_be_bytes());
                                    out.extend_from_slice(&0u32.to_be_bytes());
                                    out
                                }
                            } else {
                                let mut out = Vec::with_capacity(6);
                                out.extend_from_slice(&400u16.to_be_bytes());
                                out.extend_from_slice(&0u32.to_be_bytes());
                                out
                            };
                            let _ = otx
                                .send(OutboundFrame::Encrypted {
                                    op: OP_UPLOAD_ACK,
                                    req_id,
                                    payload: resp_bytes,
                                    allow_compress: false,
                                })
                                .await;
                        }
                        OP_DELTA_SYNC_REQ => {
                            let resp_bytes = if payload.len() == 24 {
                                let dm_pts = i64::from_be_bytes(payload[0..8].try_into().unwrap());
                                let grp_pts = i64::from_be_bytes(payload[8..16].try_into().unwrap());
                                let ch_pts = i64::from_be_bytes(payload[16..24].try_into().unwrap());
                                let path = format!("/api/sync/delta?dm_since={dm_pts}&group_since={grp_pts}&channel_since={ch_pts}");
                                let (status, _, body, _) =
                                    invoke_internal_router(&st, &ck, Method::GET, &path, "", &[], Vec::new()).await;
                                let mut out = Vec::with_capacity(2 + body.len());
                                out.extend_from_slice(&status.to_be_bytes());
                                out.extend_from_slice(&body);
                                out
                            } else {
                                let mut out = Vec::with_capacity(2);
                                out.extend_from_slice(&400u16.to_be_bytes());
                                out
                            };
                            let _ = otx
                                .send(OutboundFrame::Encrypted {
                                    op: OP_DELTA_SYNC_RES,
                                    req_id,
                                    payload: resp_bytes,
                                    allow_compress: true,
                                })
                                .await;
                        }
                        _ => {}
                    }
                });
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    drop(out_tx);
    event_task.abort();
    let _ = writer_task.await;
    hub.unregister(&user_id, conn_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cwbf2_compress_pad16_aead_roundtrip_and_tamper_rejection() {
        let (srv_sk, srv_pub) = generate_ephemeral_p256().unwrap();
        let (cli_sk, cli_pub) = generate_ephemeral_p256().unwrap();
        let salt = [0x9Au8; 16];
        let k1 = derive_cwbf_session_key(&srv_sk, &cli_pub, &salt).unwrap();
        let k2 = derive_cwbf_session_key(&cli_sk, &srv_pub, &salt).unwrap();
        assert_eq!(k1, k2);

        let c_srv = Aes256Gcm::new_from_slice(&k1).unwrap();
        let c_cli = Aes256Gcm::new_from_slice(&k2).unwrap();

        // Large compressible JSON payload (> 512 bytes) must compress + pad to multiple of 16B
        let large_json = format!("{{\"items\":[{}]}}", (0..40).map(|i| format!("{{\"id\":{i},\"text\":\"hello telegram web k\"}}")).collect::<Vec<_>>().join(","));
        assert!(large_json.len() > 1000);
        let frame = encode_cwbf2_encrypted(&c_srv, OP_RPC_RES, 42, 777, large_json.as_bytes(), true).unwrap();
        let hdr = decode_cwbf2_header(&frame).unwrap();
        assert_eq!(hdr.op, OP_RPC_RES);
        assert_eq!(hdr.seq, 42);
        assert_eq!(hdr.req_id, 777);
        assert_ne!(hdr.flags & FLAG_COMPRESSED, 0, "large JSON should set FLAG_COMPRESSED");
        assert_ne!(hdr.flags & FLAG_PADDED, 0, "all frames must set FLAG_PADDED");
        // Ciphertext payload minus 16B GCM tag must be a multiple of 16 bytes!
        assert_eq!((hdr.payload.len() - 16) % 16, 0);
        // Zero plaintext leakage on wire
        assert!(!frame.windows(8).any(|w| w == b"telegram"));

        let (op, seq, req_id, pt) = decode_cwbf2_encrypted(&c_cli, &frame).unwrap();
        assert_eq!((op, seq, req_id), (OP_RPC_RES, 42, 777));
        assert_eq!(pt, large_json.as_bytes());

        // Tampering req_id in header (AAD) must fail decryption
        let mut tampered = frame.clone();
        tampered[17] ^= 0x01;
        assert!(decode_cwbf2_encrypted(&c_cli, &tampered).is_err());

        // Internal path validation blocks traversal, /api/ws recursion, and /stream/ bypass
        assert!(is_safe_internal_path("/api/messages/123?limit=30"));
        assert!(is_safe_internal_path("/api/attachments/upload?file_name=photo..v2.png&mime=image/png"));
        let long_upload = format!("/api/attachments/upload?meta={}", "a".repeat(2600));
        assert!(is_safe_internal_path(&long_upload));
        assert!(is_safe_internal_path("/uploads/avatars/u1.avif"));
        assert!(is_safe_internal_path("/locales/vi.json"));
        assert!(!is_safe_internal_path("/api/ws"));
        assert!(!is_safe_internal_path("/stream/abc"));
        assert!(!is_safe_internal_path("/api/../etc/passwd"));
        assert!(!is_safe_internal_path("/api/attachments/upload\0?x=1"));
    }

    #[test]
    fn replay_window_allows_minor_reordering_and_rejects_duplicates_and_stale() {
        let mut w = ReplayWindow::default();
        assert!(!w.check(0));
        assert!(w.check(1));
        w.record(1);
        assert!(!w.check(1)); // duplicate rejected

        assert!(w.check(5));
        w.record(5);
        assert!(w.check(3)); // slightly out-of-order inside 64-window accepted once
        w.record(3);
        assert!(!w.check(3)); // duplicate rejected

        assert!(w.check(70));
        w.record(70);
        assert!(!w.check(6)); // 70 - 6 = 64 -> outside window, rejected
        assert!(w.check(7));  // 70 - 7 = 63 -> inside window, accepted once
        w.record(7);
        assert!(!w.check(7));
    }
}
