#![allow(deprecated)]
//! Client-side AEAD encryption for attachments (images, audio, video & files in E2EE chats).
//!
//! Supports ALL 3 versions seamlessly (100% backward-compatible):
//! - Single-blob AES-256-GCM (v1, backward-compatible with all existing messages)
//! - Chunked streaming AES-256-GCM (`CWCH2` v2)
//! - Authenticated HKDF-nonce Chunked streaming AES-256-GCM (`CWCH3` v3 via `chatw-media-core`,
//!   binding media_tag, variant_tag, chunk_index, total_chunks, and finality in AAD!)

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chatw_media_core::e2ee as core_e2ee;
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use zeroize::Zeroize;

pub const CHUNKED_MAGIC: &[u8; 5] = b"CWCH2";
pub const CHUNKED_V3_MAGIC: &[u8; 5] = b"CWCH3";
pub const DEFAULT_STREAM_CHUNK_SIZE: usize = 512 * 1024; // 512 KiB per encrypted segment
pub const CHUNKED_HEADER_LEN: usize = 5 + 4 + 4 + 8; // magic + chunk_size + chunk_count + total_plain_len = 21 bytes

pub struct SealedBlob {
    pub cipher: Vec<u8>,
    pub key_b64: String,
    pub nonce_b64: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkedHeader {
    pub chunk_size: u32,
    pub chunk_count: u32,
    pub total_plain_len: u64,
}

fn derive_chunk_nonce(base_nonce: &[u8; 12], chunk_index: u32) -> [u8; 12] {
    let mut n = *base_nonce;
    let idx_be = chunk_index.to_be_bytes();
    for i in 0..4 {
        n[8 + i] ^= idx_be[i];
    }
    n
}

fn chunk_aad(chunk_index: u32, is_final: bool) -> [u8; 10] {
    let mut aad = [0u8; 10];
    aad[0..5].copy_from_slice(CHUNKED_MAGIC);
    aad[5..9].copy_from_slice(&chunk_index.to_be_bytes());
    aad[9] = u8::from(is_final);
    aad
}

/// Parse the 21-byte `CWCH2` chunked header if present.
pub fn parse_chunked_header(bytes: &[u8]) -> Option<ChunkedHeader> {
    if bytes.len() < CHUNKED_HEADER_LEN || &bytes[0..5] != CHUNKED_MAGIC {
        return None;
    }
    let chunk_size = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
    let chunk_count = u32::from_be_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);
    let total_plain_len = u64::from_be_bytes([
        bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20],
    ]);
    Some(ChunkedHeader {
        chunk_size,
        chunk_count,
        total_plain_len,
    })
}

/// Seal bytes using single-shot AEAD for small files (< 512 KiB) or `CWCH3` HKDF-derived chunked streaming AEAD
/// automatically for larger video/audio/files.
pub fn seal(bytes: &[u8]) -> Result<SealedBlob, String> {
    if bytes.len() >= DEFAULT_STREAM_CHUNK_SIZE {
        return seal_cwch3(bytes, DEFAULT_STREAM_CHUNK_SIZE, "chatw-media", "default");
    }
    seal_single(bytes)
}

/// Seal bytes using `CWCH3` chunked streaming AEAD (HKDF per-chunk nonces + media/variant/finality AAD).
pub fn seal_cwch3(
    bytes: &[u8],
    chunk_size: usize,
    media_id: &str,
    variant_id: &str,
) -> Result<SealedBlob, String> {
    let sealed = core_e2ee::seal_cwch3(bytes, media_id, variant_id, chunk_size)
        .map_err(|e| e.to_string())?;
    Ok(SealedBlob {
        cipher: sealed.cipher,
        key_b64: sealed.key_b64,
        nonce_b64: sealed.nonce_b64,
    })
}

/// Single-shot AES-256-GCM encryption (v1 format).
pub fn seal_single(bytes: &[u8]) -> Result<SealedBlob, String> {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut key);
    OsRng.fill_bytes(&mut nonce);
    let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| "bad_key".to_string());
    let key_b64 = URL_SAFE_NO_PAD.encode(key);
    key.zeroize();
    let cipher = cipher_inst?
        .encrypt(Nonce::from_slice(&nonce), bytes)
        .map_err(|_| "encrypt_failed".to_string())?;
    Ok(SealedBlob {
        cipher,
        key_b64,
        nonce_b64: URL_SAFE_NO_PAD.encode(nonce),
    })
}

/// Chunked AEAD encryption (`CWCH2`) for progressive streaming video/audio & large E2EE files.
/// Each segment has its own unique derived nonce and AAD binding `(chunk_index, is_final)`.
pub fn seal_chunked(bytes: &[u8], chunk_size: usize) -> Result<SealedBlob, String> {
    let seg_len = chunk_size.clamp(16 * 1024, 4 * 1024 * 1024);
    let mut key = [0u8; 32];
    let mut base_nonce = [0u8; 12];
    OsRng.fill_bytes(&mut key);
    OsRng.fill_bytes(&mut base_nonce);

    let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| "bad_key".to_string());
    let key_b64 = URL_SAFE_NO_PAD.encode(key);
    key.zeroize();
    let cipher_inst = cipher_inst?;

    let chunks: Vec<&[u8]> = if bytes.is_empty() {
        vec![&[][..]]
    } else {
        bytes.chunks(seg_len).collect()
    };
    let chunk_count = chunks.len() as u32;
    let total_plain_len = bytes.len() as u64;

    let mut out = Vec::with_capacity(CHUNKED_HEADER_LEN + bytes.len() + chunks.len() * 20);
    out.extend_from_slice(CHUNKED_MAGIC);
    out.extend_from_slice(&(seg_len as u32).to_be_bytes());
    out.extend_from_slice(&chunk_count.to_be_bytes());
    out.extend_from_slice(&total_plain_len.to_be_bytes());

    for (idx, chunk) in chunks.iter().enumerate() {
        let idx_u32 = idx as u32;
        let is_final = idx + 1 == chunks.len();
        let cn = derive_chunk_nonce(&base_nonce, idx_u32);
        let aad = chunk_aad(idx_u32, is_final);
        let ct = cipher_inst
            .encrypt(
                Nonce::from_slice(&cn),
                Payload {
                    msg: chunk,
                    aad: &aad,
                },
            )
            .map_err(|_| "chunk_encrypt_failed".to_string())?;
        out.extend_from_slice(&(ct.len() as u32).to_be_bytes());
        out.extend_from_slice(&ct);
    }

    Ok(SealedBlob {
        cipher: out,
        key_b64,
        nonce_b64: URL_SAFE_NO_PAD.encode(base_nonce),
    })
}

/// Decrypt a single encrypted segment `chunk_index` independently for streaming playback/seek.
pub fn open_chunk(
    chunk_ct: &[u8],
    chunk_index: u32,
    is_final: bool,
    key_b64: &str,
    nonce_b64: &str,
) -> Result<Vec<u8>, String> {
    let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| "bad_key".to_string())?;
    let nonce_vec = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| "bad_nonce".to_string())?;
    if key.len() != 32 || nonce_vec.len() != 12 {
        key.zeroize();
        return Err("bad_key_material".into());
    }
    let mut base_nonce = [0u8; 12];
    base_nonce.copy_from_slice(&nonce_vec);
    let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| "bad_key".to_string());
    key.zeroize();
    let cn = derive_chunk_nonce(&base_nonce, chunk_index);
    let aad = chunk_aad(chunk_index, is_final);
    cipher_inst?
        .decrypt(
            Nonce::from_slice(&cn),
            Payload {
                msg: chunk_ct,
                aad: &aad,
            },
        )
        .map_err(|_| "chunk_decrypt_failed".to_string())
}

/// Transparently decrypt ALL formats: `CWCH3` v3, `CWCH2` v2 chunked AEAD streams, and legacy v1 single-blob ciphertexts.
pub fn open(cipher: &[u8], key_b64: &str, nonce_b64: &str) -> Result<Vec<u8>, String> {
    if core_e2ee::parse_cwch3_header(cipher).is_some() {
        return core_e2ee::open_any_media(cipher, key_b64, nonce_b64).map_err(|e| e.to_string());
    }
    if let Some(hdr) = parse_chunked_header(cipher) {
        let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| "bad_key".to_string())?;
        let nonce_vec = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| "bad_nonce".to_string())?;
        if key.len() != 32 || nonce_vec.len() != 12 {
            key.zeroize();
            return Err("bad_key_material".into());
        }
        let mut base_nonce = [0u8; 12];
        base_nonce.copy_from_slice(&nonce_vec);
        let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| "bad_key".to_string());
        key.zeroize();
        let cipher_inst = cipher_inst?;

        let mut plain = Vec::with_capacity(hdr.total_plain_len.min(128 * 1024 * 1024) as usize);
        let mut pos = CHUNKED_HEADER_LEN;
        for idx in 0..hdr.chunk_count {
            if pos + 4 > cipher.len() {
                return Err("truncated_chunk_stream".into());
            }
            let ct_len = u32::from_be_bytes([
                cipher[pos],
                cipher[pos + 1],
                cipher[pos + 2],
                cipher[pos + 3],
            ]) as usize;
            pos += 4;
            if pos + ct_len > cipher.len() {
                return Err("truncated_chunk_payload".into());
            }
            let ct_slice = &cipher[pos..pos + ct_len];
            pos += ct_len;
            let is_final = idx + 1 == hdr.chunk_count;
            let cn = derive_chunk_nonce(&base_nonce, idx);
            let aad = chunk_aad(idx, is_final);
            let mut seg_pt = cipher_inst
                .decrypt(
                    Nonce::from_slice(&cn),
                    Payload {
                        msg: ct_slice,
                        aad: &aad,
                    },
                )
                .map_err(|_| "chunk_decrypt_failed".to_string())?;
            plain.append(&mut seg_pt);
        }
        if pos != cipher.len() || plain.len() as u64 != hdr.total_plain_len {
            return Err("chunk_stream_length_mismatch".into());
        }
        return Ok(plain);
    }

    let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| "bad_key".to_string())?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| "bad_nonce".to_string())?;
    if key.len() != 32 || nonce.len() != 12 {
        key.zeroize();
        return Err("bad_key_material".into());
    }
    let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| "bad_key".to_string());
    key.zeroize();
    cipher_inst?
        .decrypt(Nonce::from_slice(&nonce), cipher)
        .map_err(|_| "decrypt_failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper_detection() {
        let data = b"hello encrypted image bytes";
        let sealed = seal_single(data).unwrap();
        assert_ne!(sealed.cipher.as_slice(), data.as_slice());
        assert_eq!(open(&sealed.cipher, &sealed.key_b64, &sealed.nonce_b64).unwrap(), data);
        let mut bad = sealed.cipher.clone();
        bad[0] ^= 1;
        assert!(open(&bad, &sealed.key_b64, &sealed.nonce_b64).is_err());
    }

    #[test]
    fn chunked_streaming_roundtrip_and_reorder_truncation_detection() {
        let data: Vec<u8> = (0..(96 * 1024)).map(|i| (i % 251) as u8).collect();
        let sealed = seal_chunked(&data, 32 * 1024).unwrap();
        let hdr = parse_chunked_header(&sealed.cipher).expect("chunked header");
        assert_eq!(hdr.chunk_count, 3);
        assert_eq!(hdr.total_plain_len, data.len() as u64);

        let opened = open(&sealed.cipher, &sealed.key_b64, &sealed.nonce_b64).unwrap();
        assert_eq!(opened, data);

        // Truncating the final chunk must fail integrity verification
        let truncated = &sealed.cipher[..sealed.cipher.len() - 32];
        assert!(open(truncated, &sealed.key_b64, &sealed.nonce_b64).is_err());

        // Single-chunk random-access decryption (segment 0)
        let seg0_len = u32::from_be_bytes([
            sealed.cipher[CHUNKED_HEADER_LEN],
            sealed.cipher[CHUNKED_HEADER_LEN + 1],
            sealed.cipher[CHUNKED_HEADER_LEN + 2],
            sealed.cipher[CHUNKED_HEADER_LEN + 3],
        ]) as usize;
        let seg0_ct = &sealed.cipher[CHUNKED_HEADER_LEN + 4..CHUNKED_HEADER_LEN + 4 + seg0_len];
        let seg0_pt = open_chunk(seg0_ct, 0, false, &sealed.key_b64, &sealed.nonce_b64).unwrap();
        assert_eq!(seg0_pt, &data[..32 * 1024]);
        // Claiming segment 0 is index 1 must fail AAD/nonce authentication
        assert!(open_chunk(seg0_ct, 1, false, &sealed.key_b64, &sealed.nonce_b64).is_err());
    }

    #[test]
    fn cwch3_hkdf_streaming_roundtrip_and_tamper_detection() {
        let data: Vec<u8> = (0..(128 * 1024)).map(|i| ((i * 13) % 251) as u8).collect();
        let sealed = seal_cwch3(&data, 32 * 1024, "media-e2ee-110", "av1-720p").unwrap();
        assert_eq!(&sealed.cipher[0..5], CHUNKED_V3_MAGIC);
        let opened = open(&sealed.cipher, &sealed.key_b64, &sealed.nonce_b64).unwrap();
        assert_eq!(opened, data);

        let mut tampered = sealed.cipher.clone();
        let mid = tampered.len() / 2;
        tampered[mid] ^= 0x40;
        assert!(open(&tampered, &sealed.key_b64, &sealed.nonce_b64).is_err());
    }
}
