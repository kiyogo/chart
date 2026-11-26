pub mod rpc_bin;

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, header},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use hkdf::Hkdf;
use p256::{EncodedPoint, PublicKey, SecretKey, ecdh::diffie_hellman};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, parse_cookie},
    },
    error::{AppError, AppResult},
};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_CHANNEL_CAPACITY: usize = 128;

// ── CWBF1 Encrypted Binary WebSocket Protocol (Spec 069) ────────────────────
// 28-byte big-endian header:
//   [0..2]   Magic "CW" (0x43, 0x57)
//   [2]      Version (0x01)
//   [3]      Opcode (0x01 HELLO, 0x02 HELLO_ACK, 0x03 EVENT, 0x05 PING, 0x06 PONG)
//   [4..12]  seq (u64 BE)
//   [12..24] nonce (12B random CSPRNG for encrypted frames; zeroed for handshake/control)
//   [24..28] payload_len (u32 BE)
//   [28..]   payload (for OP_EVENT: AES-256-GCM ciphertext + 16B tag with AAD = header[0..28])
pub const CWBF_MAGIC: [u8; 2] = *b"CW";
pub const CWBF_VERSION: u8 = 1;
pub const CWBF_HEADER_LEN: usize = 28;
pub const OP_HELLO: u8 = 0x01;
pub const OP_HELLO_ACK: u8 = 0x02;
pub const OP_EVENT: u8 = 0x03;
pub const OP_PING: u8 = 0x05;
pub const OP_PONG: u8 = 0x06;
const CWBF_HKDF_INFO: &[u8] = b"chatw-ws-bin-v1";
const BINARY_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Deserialize, Default)]
struct WsQuery {
    proto: Option<String>,
}

fn build_cwbf_header(op: u8, seq: u64, nonce: &[u8; 12], payload_len: u32) -> [u8; CWBF_HEADER_LEN] {
    let mut hdr = [0u8; CWBF_HEADER_LEN];
    hdr[0] = CWBF_MAGIC[0];
    hdr[1] = CWBF_MAGIC[1];
    hdr[2] = CWBF_VERSION;
    hdr[3] = op;
    hdr[4..12].copy_from_slice(&seq.to_be_bytes());
    hdr[12..24].copy_from_slice(nonce);
    hdr[24..28].copy_from_slice(&payload_len.to_be_bytes());
    hdr
}

pub fn encode_cwbf_plain_frame(op: u8, seq: u64, payload: &[u8]) -> Vec<u8> {
    let hdr = build_cwbf_header(op, seq, &[0u8; 12], payload.len() as u32);
    let mut out = Vec::with_capacity(CWBF_HEADER_LEN + payload.len());
    out.extend_from_slice(&hdr);
    out.extend_from_slice(payload);
    out
}

pub fn decode_cwbf_header(frame: &[u8]) -> Option<(u8, u64, [u8; 12], &[u8])> {
    if frame.len() < CWBF_HEADER_LEN {
        return None;
    }
    if frame[0..2] != CWBF_MAGIC || frame[2] != CWBF_VERSION {
        return None;
    }
    let op = frame[3];
    let seq = u64::from_be_bytes(frame[4..12].try_into().ok()?);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&frame[12..24]);
    let len = u32::from_be_bytes(frame[24..28].try_into().ok()?) as usize;
    if frame.len() != CWBF_HEADER_LEN + len {
        return None;
    }
    Some((op, seq, nonce, &frame[CWBF_HEADER_LEN..]))
}

pub fn generate_ephemeral_p256() -> Result<(SecretKey, Vec<u8>), &'static str> {
    let mut raw = [0u8; 32];
    for _ in 0..16 {
        getrandom::fill(&mut raw).map_err(|_| "csprng_failed")?;
        if let Ok(sk) = SecretKey::from_slice(&raw) {
            let pk = sk.public_key();
            let point = EncodedPoint::from(&pk);
            return Ok((sk, point.as_bytes().to_vec()));
        }
    }
    Err("ecdh_keygen_failed")
}

#[allow(deprecated)]
pub fn derive_cwbf_session_key(
    local_sk: &SecretKey,
    peer_uncompressed_sec1: &[u8],
    salt: &[u8; 16],
) -> Result<[u8; 32], &'static str> {
    if peer_uncompressed_sec1.len() != 65 || peer_uncompressed_sec1[0] != 0x04 {
        return Err("invalid_peer_ecdh_key");
    }
    let peer_pk = PublicKey::from_sec1_bytes(peer_uncompressed_sec1)
        .map_err(|_| "invalid_peer_ecdh_point")?;
    let shared = diffie_hellman(local_sk.to_nonzero_scalar(), peer_pk.as_affine());
    let hk = Hkdf::<Sha256>::new(Some(salt), shared.raw_secret_bytes().as_slice());
    let mut key = [0u8; 32];
    hk.expand(CWBF_HKDF_INFO, &mut key)
        .map_err(|_| "hkdf_expand_failed")?;
    Ok(key)
}

#[allow(deprecated)]
pub fn encode_cwbf_encrypted_frame(
    cipher: &Aes256Gcm,
    op: u8,
    seq: u64,
    plaintext: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|_| "csprng_failed")?;
    let ct_len = plaintext
        .len()
        .checked_add(16)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or("frame_too_large")?;
    let hdr = build_cwbf_header(op, seq, &nonce_bytes, ct_len);
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: &hdr,
            },
        )
        .map_err(|_| "aead_encrypt_failed")?;
    let mut out = Vec::with_capacity(CWBF_HEADER_LEN + ct.len());
    out.extend_from_slice(&hdr);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[cfg(test)]
#[allow(deprecated)]
pub fn decode_cwbf_encrypted_frame(cipher: &Aes256Gcm, frame: &[u8]) -> Result<(u8, u64, Vec<u8>), &'static str> {
    let (op, seq, nonce_bytes, ct) = decode_cwbf_header(frame).ok_or("invalid_header")?;
    let hdr = &frame[..CWBF_HEADER_LEN];
    let pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload { msg: ct, aad: hdr },
        )
        .map_err(|_| "aead_decrypt_failed")?;
    Ok((op, seq, pt))
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    #[serde(rename = "message.new")]
    MessageNew {
        id: Uuid,
        seq: i64,
        sender_id: Uuid,
        recipient_id: Uuid,
        envelope: String, // Base64url encoded envelope
        #[serde(default)]
        reply_to_id: Option<Uuid>,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "friend.request")]
    FriendRequest {
        id: Uuid,
        sender_id: Uuid,
        sender_username: String,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "friend.accepted")]
    FriendAccepted { user_id: Uuid, username: String },
    #[serde(rename = "typing.start")]
    TypingStart { user_id: Uuid },
    #[serde(rename = "typing.stop")]
    TypingStop { user_id: Uuid },
    #[serde(rename = "reaction.update")]
    ReactionUpdate {
        message_id: Uuid,
        user_id: Uuid,
        emoji: String,
        action: String,
    },
    #[serde(rename = "message.pinned")]
    MessagePinned {
        message_id: Uuid,
        is_pinned: bool,
    },
    #[serde(rename = "message.deleted")]
    MessageDeleted {
        message_id: Uuid,
    },
    #[serde(rename = "message.edited")]
    MessageEdited {
        message_id: Uuid,
        envelope: String,
    },
    #[serde(rename = "channel.message.new")]
    ChannelMessageNew {
        message: crate::models::ChannelMessageItem,
    },
    #[serde(rename = "webrtc.signal")]
    WebRtcSignal {
        sender_id: Uuid,
        channel_id: Option<Uuid>,
        signal_type: String,
        data: serde_json::Value,
    },
    #[serde(rename = "voice.state.update")]
    VoiceStateUpdate(crate::models::VoiceStateItem),
    #[serde(rename = "user.presence.update")]
    UserPresenceUpdate {
        user_id: Uuid,
        presence: String,
        status_text: Option<String>,
        status_emoji: Option<String>,
    },
    #[serde(rename = "group.message.new")]
    GroupMessageNew {
        message: crate::models::GroupMessageItem,
    },
    #[serde(rename = "group.reaction.update")]
    GroupReactionUpdate {
        group_id: Uuid,
        message_id: Uuid,
        user_id: Uuid,
        emoji: String,
        action: String,
    },
    #[serde(rename = "group.channel.new")]
    GroupChannelNew {
        group_id: Uuid,
        channel: crate::models::GroupChannelItem,
    },
    #[serde(rename = "group.channel.deleted")]
    GroupChannelDeleted {
        group_id: Uuid,
        channel_id: Uuid,
    },
    #[serde(rename = "group.member.joined")]
    GroupMemberJoined {
        group_id: Uuid,
        member: crate::models::GroupMemberItem,
    },
    #[serde(rename = "group.member.left")]
    GroupMemberLeft {
        group_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "group.updated")]
    GroupUpdated {
        group: crate::models::GroupItem,
    },
    #[serde(rename = "call.incoming")]
    CallIncoming {
        session: crate::models::CallSessionItem,
    },
    #[serde(rename = "call.accepted")]
    CallAccepted {
        call_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "call.rejected")]
    CallRejected {
        call_id: Uuid,
        user_id: Uuid,
        reason: String,
    },
    #[serde(rename = "call.ended")]
    CallEnded {
        call_id: Uuid,
        reason: String,
    },
    #[serde(rename = "call.signal")]
    CallSignal {
        call_id: Uuid,
        sender_id: Uuid,
        target_user_id: Option<Uuid>,
        signal_type: String,
        data: serde_json::Value,
    },
    #[serde(rename = "call.participant.joined")]
    CallParticipantJoined {
        call_id: Uuid,
        participant: crate::models::CallParticipantItem,
    },
    #[serde(rename = "call.participant.left")]
    CallParticipantLeft {
        call_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "call.state.update")]
    CallStateUpdate {
        call_id: Uuid,
        user_id: Uuid,
        muted: bool,
        camera_off: bool,
    },
    /// Partner read DMs up to `seq` (only sent when the reader shares receipts).
    #[serde(rename = "message.read")]
    MessageRead(serde_json::Value),
    /// A conversation's shared wallpaper changed.
    #[serde(rename = "wallpaper.updated")]
    WallpaperUpdated(serde_json::Value),
    /// New message in a user <-> bot conversation.
    #[serde(rename = "bot.message")]
    BotMessage(serde_json::Value),
    /// Someone asked to join a group that requires approval.
    #[serde(rename = "group.join_request")]
    GroupJoinRequest(serde_json::Value),
    /// Group management settings changed.
    #[serde(rename = "group.settings")]
    GroupSettings(serde_json::Value),
    /// Labels / pins / conversation preferences changed on another device.
    #[serde(rename = "prefs.updated")]
    PrefsUpdated(serde_json::Value),
    /// A friend's (or my own) avatar motion / profile details changed.
    #[serde(rename = "profile.updated")]
    ProfileUpdated(serde_json::Value),
    /// Sign-in from a device this account never used before.
    #[serde(rename = "security.alert")]
    SecurityAlert(serde_json::Value),
    /// Enterprise workspace state updated (config, tasks, HR, approvals, docs, meetings).
    #[serde(rename = "enterprise.event")]
    EnterpriseEvent(serde_json::Value),
    /// Threads-style social feed post / reply / delete update.
    #[serde(rename = "feed.updated")]
    FeedUpdated(serde_json::Value),
    /// New untrusted device requested login & sync approval on an online trusted device.
    #[serde(rename = "device.approval.request")]
    DeviceApprovalRequest(serde_json::Value),
    /// Pending device login request was approved or rejected.
    #[serde(rename = "device.approval.resolved")]
    DeviceApprovalResolved(serde_json::Value),
    /// A device was revoked and must immediately lock/wipe active session.
    #[serde(rename = "device.revoked")]
    DeviceRevoked(serde_json::Value),
    /// Two-way DM file recovery request arriving at peer/same-account device.
    #[serde(rename = "recovery.request.incoming")]
    RecoveryRequestIncoming(serde_json::Value),
    /// Status update for an active file recovery request.
    #[serde(rename = "recovery.request.updated")]
    RecoveryRequestUpdated(serde_json::Value),
    /// WebRTC DataChannel signaling relay scoped to a recovery request.
    #[serde(rename = "recovery.p2p.signal")]
    RecoveryP2pSignal(serde_json::Value),
    /// Authoritative remote cache wipe when sender deletes media for everyone.
    #[serde(rename = "media.delete_for_all")]
    MediaDeleteForAll(serde_json::Value),
    /// Realtime multi-device / server / group Plugin & Permission Sync event.
    #[serde(rename = "plugin.sync")]
    PluginSync(serde_json::Value),
    /// Spec 165 (Section 39): Message Chunk Manifest updated (small notice; client fetches details via HTTP).
    #[serde(rename = "manifest.updated")]
    ManifestUpdated {
        scope_type: String,
        scope_id: String,
        manifest_version: i64,
        chunk_id: Option<Uuid>,
        event_id: Option<i64>,
    },
    /// Spec 165 (Section 39): Message Chunk compacted / tier promoted / mutated.
    #[serde(rename = "chunk.changed")]
    ChunkChanged {
        scope_type: String,
        scope_id: String,
        chunk_id: Uuid,
        action: String,
        event_id: Option<i64>,
    },
    /// Spec 165 (Section 39): Scope / group / channel / device permission changed.
    #[serde(rename = "permissions.changed")]
    PermissionsChanged {
        scope_type: String,
        scope_id: String,
        reason: String,
        event_id: Option<i64>,
    },
}

struct WsClientConn {
    session_id: Option<Uuid>,
    device_id: Option<Uuid>,
    tx: mpsc::Sender<WsEvent>,
}

pub struct WsHub {
    clients: RwLock<HashMap<Uuid, HashMap<u64, WsClientConn>>>,
    next_id: AtomicU64,
    total_connections: AtomicUsize,
    max_connections: usize,
    valkey: RwLock<Option<Arc<crate::cache::ValkeyCoordinator>>>,
}

impl WsHub {
    pub fn new(max_connections: usize) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            total_connections: AtomicUsize::new(0),
            max_connections,
            valkey: RwLock::new(None),
        }
    }

    pub async fn attach_valkey(&self, valkey: Arc<crate::cache::ValkeyCoordinator>) {
        let mut guard = self.valkey.write().await;
        *guard = Some(valkey);
    }

    async fn get_valkey(&self) -> Option<Arc<crate::cache::ValkeyCoordinator>> {
        self.valkey.read().await.clone()
    }

    pub async fn register(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        device_id: Option<Uuid>,
    ) -> Option<(u64, mpsc::Receiver<WsEvent>)> {
        if self.total_connections.load(Ordering::Relaxed) >= self.max_connections {
            return None;
        }

        let conn_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);

        let mut clients = self.clients.write().await;
        let user_map = clients.entry(user_id).or_default();
        if user_map.len() >= 16 {
            return None;
        }
        user_map.insert(
            conn_id,
            WsClientConn {
                session_id,
                device_id,
                tx,
            },
        );
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        drop(clients);

        if let Some(vk) = self.get_valkey().await {
            vk.set_user_online(user_id, device_id).await;
        }

        Some((conn_id, rx))
    }

    pub async fn unregister(&self, user_id: &Uuid, conn_id: u64) {
        let mut now_empty = false;
        {
            let mut clients = self.clients.write().await;
            if let Some(user_conns) = clients.get_mut(user_id)
                && user_conns.remove(&conn_id).is_some()
            {
                if user_conns.is_empty() {
                    clients.remove(user_id);
                    now_empty = true;
                }
                self.total_connections.fetch_sub(1, Ordering::Relaxed);
            }
        }
        if now_empty
            && let Some(vk) = self.get_valkey().await
        {
            vk.clear_user_online(*user_id).await;
        }
    }

    pub async fn disconnect_device(&self, user_id: &Uuid, target_device_id: Uuid) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id) {
            let before = user_conns.len();
            user_conns.retain(|_, c| c.device_id != Some(target_device_id));
            let removed = before.saturating_sub(user_conns.len());
            if removed > 0 {
                self.total_connections.fetch_sub(removed, Ordering::Relaxed);
            }
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
        }
    }

    pub async fn disconnect_session(&self, user_id: &Uuid, target_session_id: Uuid) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id) {
            let before = user_conns.len();
            user_conns.retain(|_, c| c.session_id != Some(target_session_id));
            let removed = before.saturating_sub(user_conns.len());
            if removed > 0 {
                self.total_connections.fetch_sub(removed, Ordering::Relaxed);
            }
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
        }
    }

    pub async fn disconnect_user(&self, user_id: &Uuid) {
        let mut clients = self.clients.write().await;
        if let Some(removed_map) = clients.remove(user_id)
            && !removed_map.is_empty()
        {
            self.total_connections.fetch_sub(removed_map.len(), Ordering::Relaxed);
        }
    }

    /// Deliver directly to local in-process WebSocket connections without re-publishing to Valkey.
    pub async fn deliver_local_users(&self, user_ids: &[Uuid], event: WsEvent) {
        let clients = self.clients.read().await;
        for uid in user_ids {
            if let Some(user_conns) = clients.get(uid) {
                for c in user_conns.values() {
                    let _ = c.tx.try_send(event.clone());
                }
            }
        }
    }

    pub async fn send_to_user(&self, user_id: &Uuid, event: WsEvent) {
        self.deliver_local_users(&[*user_id], event.clone()).await;
        if let Some(vk) = self.get_valkey().await {
            vk.publish_ws_event(&[*user_id], &event).await;
        }
    }

    pub async fn send_to_users(&self, user_ids: &[Uuid], event: WsEvent) {
        self.deliver_local_users(user_ids, event.clone()).await;
        if let Some(vk) = self.get_valkey().await {
            vk.publish_ws_event(user_ids, &event).await;
        }
    }

    pub async fn online_user_ids(&self) -> std::collections::HashSet<Uuid> {
        let clients = self.clients.read().await;
        clients.keys().copied().collect()
    }
}

/// Persist an important realtime/sync event durably into PostgreSQL `ws_event_log` so
/// clients reconnecting after a WebSocket drop OR a Valkey restart can catch up cleanly
/// via `GET /api/sync/events?after_event_id=` without ever losing a single event!
pub async fn record_durable_ws_event(
    db: &sqlx::PgPool,
    user_id: Uuid,
    event_type: &str,
    scope_type: Option<&str>,
    scope_id: Option<&str>,
    message_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Option<i64> {
    sqlx::query_scalar(
        r#"
        INSERT INTO ws_event_log (user_id, event_type, scope_type, scope_id, message_id, payload)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING event_id
        "#,
    )
    .bind(user_id)
    .bind(event_type)
    .bind(scope_type)
    .bind(scope_id)
    .bind(message_id)
    .bind(payload)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    // 1. Verify Origin header against public_origin and explicit cors_allowed_origins
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin_str = origin
            .to_str()
            .map_err(|_| AppError::Forbidden("invalid_origin"))?;
        if !crate::auth::is_allowed_cors_origin_for_config(
            origin_str,
            &state.config.public_origin,
            &state.config.cors_allowed_origins,
        ) {
            return Err(AppError::Forbidden("origin_mismatch"));
        }
    }

    // 2. Authenticate session from HttpOnly Cookie
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;

    let user =
        crate::auth::session::authenticate_session_state_with_headers(&state, session_token, Some(&headers)).await?;

    // 3. Register connection in hub
    let (conn_id, rx) = state
        .ws_hub
        .register(user.id, user.session_id, user.device_id)
        .await
        .ok_or(AppError::TooManyRequests)?;

    let hub = state.ws_hub.clone();
    let user_id = user.id;
    let proto_mode = query.proto.unwrap_or_default();
    let session_tok_owned = session_token.to_string();
    let state_clone = state.clone();

    Ok(ws.on_upgrade(move |socket| async move {
        if proto_mode == "rpc-bin" {
            rpc_bin::handle_socket_rpc_bin(socket, state_clone, user_id, session_tok_owned, conn_id, rx, hub).await;
        } else if proto_mode == "bin" {
            handle_socket_binary(socket, user_id, conn_id, rx, hub).await;
        } else {
            handle_socket(socket, user_id, conn_id, rx, hub).await;
        }
    }))
}

async fn handle_socket_binary(
    socket: WebSocket,
    user_id: Uuid,
    conn_id: u64,
    mut rx: mpsc::Receiver<WsEvent>,
    hub: Arc<WsHub>,
) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Ephemeral P-256 ECDH + 16-byte salt handshake (zero plaintext key on the wire!)
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
    let hello_frame = encode_cwbf_plain_frame(OP_HELLO, 0, &hello_payload);
    if sender.send(Message::Binary(hello_frame.into())).await.is_err() {
        hub.unregister(&user_id, conn_id).await;
        return;
    }

    // Wait for client's OP_HELLO_ACK containing its 65-byte ephemeral uncompressed P-256 public key.
    let handshake_res = tokio::time::timeout(BINARY_HANDSHAKE_TIMEOUT, async {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Binary(bytes)) => {
                    if let Some((op, _seq, _nonce, payload)) = decode_cwbf_header(&bytes)
                        && op == OP_HELLO_ACK
                    {
                        return derive_cwbf_session_key(&server_sk, payload, &salt).ok();
                    }
                    return None;
                }
                Ok(Message::Ping(p)) => {
                    let _ = sender.send(Message::Pong(p)).await;
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

    let mut out_seq: u64 = 1;
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        if let Ok(json_bytes) = serde_json::to_vec(&event) {
                            if let Ok(frame) = encode_cwbf_encrypted_frame(&cipher, OP_EVENT, out_seq, &json_bytes) {
                                out_seq = out_seq.wrapping_add(1);
                                if sender.send(Message::Binary(frame.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    None => break,
                }
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Some((op, seq, _nonce, payload)) = decode_cwbf_header(&bytes)
                            && op == OP_PING
                            && payload.len() <= 32
                        {
                            let pong = encode_cwbf_plain_frame(OP_PONG, seq, payload);
                            if sender.send(Message::Binary(pong.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    hub.unregister(&user_id, conn_id).await;
}

async fn handle_socket(
    socket: WebSocket,
    user_id: Uuid,
    conn_id: u64,
    mut rx: mpsc::Receiver<WsEvent>,
    hub: Arc<WsHub>,
) {
    let (mut sender, mut receiver) = socket.split();

    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            // Heartbeat ping
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            // Outgoing event to client
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        if let Ok(json) = serde_json::to_string(&event)
                            && sender.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // Incoming message from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        // Healthy heartbeat
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    // Reject client data messages - WebSocket is strictly server-to-client for events!
                    _ => {}
                }
            }
        }
    }

    hub.unregister(&user_id, conn_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cwbf1_ecdh_and_aead_roundtrip_and_tamper_detection() {
        let (server_sk, server_pub) = generate_ephemeral_p256().expect("server ecdh");
        let (client_sk, client_pub) = generate_ephemeral_p256().expect("client ecdh");
        assert_eq!(server_pub.len(), 65);
        assert_eq!(client_pub.len(), 65);

        let salt = [0x42u8; 16];
        let server_key = derive_cwbf_session_key(&server_sk, &client_pub, &salt).expect("server key");
        let client_key = derive_cwbf_session_key(&client_sk, &server_pub, &salt).expect("client key");
        assert_eq!(server_key, client_key, "ECDH + HKDF-SHA256 must match on both peers");

        let server_cipher = Aes256Gcm::new_from_slice(&server_key).unwrap();
        let client_cipher = Aes256Gcm::new_from_slice(&client_key).unwrap();

        let payload = br#"{"type":"typing.start","data":{"user_id":"00000000-0000-0000-0000-000000000001"}}"#;
        let frame = encode_cwbf_encrypted_frame(&server_cipher, OP_EVENT, 7, payload).expect("encrypt frame");
        assert_eq!(&frame[0..2], b"CW");
        assert_eq!(frame[2], CWBF_VERSION);
        assert_eq!(frame[3], OP_EVENT);
        // Plaintext JSON must NOT appear anywhere in the binary frame!
        assert!(!frame.windows(6).any(|w| w == b"typing"));

        let (op, seq, pt) = decode_cwbf_encrypted_frame(&client_cipher, &frame).expect("decrypt frame");
        assert_eq!(op, OP_EVENT);
        assert_eq!(seq, 7);
        assert_eq!(pt, payload);

        // Tampering with header seq (AAD) must fail AES-GCM authentication!
        let mut tampered_hdr = frame.clone();
        tampered_hdr[11] ^= 0x01;
        assert!(decode_cwbf_encrypted_frame(&client_cipher, &tampered_hdr).is_err());

        // Tampering with ciphertext must fail AES-GCM authentication!
        let mut tampered_ct = frame.clone();
        let last = tampered_ct.len() - 1;
        tampered_ct[last] ^= 0x80;
        assert!(decode_cwbf_encrypted_frame(&client_cipher, &tampered_ct).is_err());
    }
}
