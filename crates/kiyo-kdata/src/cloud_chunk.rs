use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::{Aead, Payload}};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::KdataError;

type HmacSha256 = Hmac<Sha256>;
pub const KCHUNK_MAGIC: &[u8; 8] = b"KCHUNK01";

#[derive(Clone)]
pub struct AccountCloudKeys {
    pub chunk_enc_key: Zeroizing<[u8; 32]>,
    pub chunk_dedup_key: Zeroizing<[u8; 32]>,
    pub manifest_key: Zeroizing<[u8; 32]>,
}

/// Derives isolated per-account keys from a backup secret/master key + user_id.
/// Because `chunk_dedup_key` is keyed per user, identical plaintext files across
/// two different accounts NEVER share the same chunk hash or leak existence.
pub fn derive_account_cloud_keys(master_secret: &[u8], user_id: &Uuid) -> Result<AccountCloudKeys, KdataError> {
    let hk = Hkdf::<Sha256>::new(Some(user_id.as_bytes()), master_secret);
    let mut chunk_enc_key = Zeroizing::new([0u8; 32]);
    let mut chunk_dedup_key = Zeroizing::new([0u8; 32]);
    let mut manifest_key = Zeroizing::new([0u8; 32]);
    hk.expand(b"kiyo-cloud-chunk-enc-v1", &mut *chunk_enc_key)
        .map_err(|_| KdataError::InvalidHeader("hkdf".into()))?;
    hk.expand(b"kiyo-cloud-chunk-dedup-v1", &mut *chunk_dedup_key)
        .map_err(|_| KdataError::InvalidHeader("hkdf".into()))?;
    hk.expand(b"kiyo-cloud-manifest-enc-v1", &mut *manifest_key)
        .map_err(|_| KdataError::InvalidHeader("hkdf".into()))?;
    Ok(AccountCloudKeys {
        chunk_enc_key,
        chunk_dedup_key,
        manifest_key,
    })
}

pub struct SealedCloudChunk {
    pub chunk_id_hex: String,
    pub plain_sha256_hex: String,
    pub cipher_sha256_hex: String,
    pub wire_bytes: Vec<u8>,
}

/// Computes keyed per-account chunk ID (`HMAC-SHA256(dedup_key, plain_sha256)`)
/// and encrypts chunk with `XChaCha20-Poly1305` bound to `(user_id, chunk_id_hex)`.
pub fn seal_cloud_chunk(
    keys: &AccountCloudKeys,
    user_id: &Uuid,
    plain: &[u8],
) -> Result<SealedCloudChunk, KdataError> {
    let plain_sha: [u8; 32] = Sha256::digest(plain).into();
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&*keys.chunk_dedup_key)
        .map_err(|_| KdataError::InvalidHeader("dedup hmac".into()))?;
    mac.update(&plain_sha);
    let chunk_id: [u8; 32] = mac.finalize().into_bytes().into();
    let chunk_id_hex = hex::encode(chunk_id);

    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce).map_err(|e| KdataError::InvalidHeader(format!("rng: {e}")))?;

    let mut aad = Vec::with_capacity(8 + 16 + 32);
    aad.extend_from_slice(KCHUNK_MAGIC);
    aad.extend_from_slice(user_id.as_bytes());
    aad.extend_from_slice(&chunk_id);

    let cipher = XChaCha20Poly1305::new((&*keys.chunk_enc_key).into());
    let ct = cipher
        .encrypt((&nonce).into(), Payload { msg: plain, aad: &aad })
        .map_err(|_| KdataError::AuthenticationFailed)?;

    let mut wire = Vec::with_capacity(8 + 24 + ct.len());
    wire.extend_from_slice(KCHUNK_MAGIC);
    wire.extend_from_slice(&nonce);
    wire.extend_from_slice(&ct);
    let cipher_sha_hex = hex::encode(Sha256::digest(&wire));

    Ok(SealedCloudChunk {
        chunk_id_hex,
        plain_sha256_hex: hex::encode(plain_sha),
        cipher_sha256_hex: cipher_sha_hex,
        wire_bytes: wire,
    })
}

pub fn open_cloud_chunk(
    keys: &AccountCloudKeys,
    user_id: &Uuid,
    chunk_id_hex: &str,
    wire: &[u8],
) -> Result<Vec<u8>, KdataError> {
    if wire.len() < 8 + 24 + 16 || &wire[0..8] != KCHUNK_MAGIC {
        return Err(KdataError::FormatViolation("sai header kchunk_v1".into()));
    }
    let chunk_id_bytes = hex::decode(chunk_id_hex).map_err(|_| KdataError::FormatViolation("bad chunk_id".into()))?;
    if chunk_id_bytes.len() != 32 {
        return Err(KdataError::FormatViolation("bad chunk_id len".into()));
    }
    let nonce: [u8; 24] = wire[8..32].try_into().unwrap();
    let mut aad = Vec::with_capacity(8 + 16 + 32);
    aad.extend_from_slice(KCHUNK_MAGIC);
    aad.extend_from_slice(user_id.as_bytes());
    aad.extend_from_slice(&chunk_id_bytes);

    let cipher = XChaCha20Poly1305::new((&*keys.chunk_enc_key).into());
    let plain = cipher
        .decrypt((&nonce).into(), Payload { msg: &wire[32..], aad: &aad })
        .map_err(|_| KdataError::WrongPasswordOrTampered)?;

    // Verify keyed chunk ID matches decrypted content
    let plain_sha: [u8; 32] = Sha256::digest(&plain).into();
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&*keys.chunk_dedup_key)
        .map_err(|_| KdataError::InvalidHeader("dedup hmac".into()))?;
    mac.update(&plain_sha);
    mac.verify_slice(&chunk_id_bytes)
        .map_err(|_| KdataError::IntegrityMismatch("nội dung chunk không khớp chunk_id".into()))?;

    Ok(plain)
}

pub fn seal_cloud_manifest(keys: &AccountCloudKeys, user_id: &Uuid, snapshot_id: &Uuid, json: &[u8]) -> Result<Vec<u8>, KdataError> {
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce).map_err(|e| KdataError::InvalidHeader(format!("rng: {e}")))?;
    let mut aad = Vec::with_capacity(40);
    aad.extend_from_slice(b"KMNF0001");
    aad.extend_from_slice(user_id.as_bytes());
    aad.extend_from_slice(snapshot_id.as_bytes());

    let cipher = XChaCha20Poly1305::new((&*keys.manifest_key).into());
    let ct = cipher
        .encrypt((&nonce).into(), Payload { msg: json, aad: &aad })
        .map_err(|_| KdataError::AuthenticationFailed)?;
    let mut out = Vec::with_capacity(8 + 24 + ct.len());
    out.extend_from_slice(b"KMNF0001");
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open_cloud_manifest(keys: &AccountCloudKeys, user_id: &Uuid, snapshot_id: &Uuid, wire: &[u8]) -> Result<Vec<u8>, KdataError> {
    if wire.len() < 8 + 24 + 16 || &wire[0..8] != b"KMNF0001" {
        return Err(KdataError::FormatViolation("sai header manifest cloud".into()));
    }
    let nonce: [u8; 24] = wire[8..32].try_into().unwrap();
    let mut aad = Vec::with_capacity(40);
    aad.extend_from_slice(b"KMNF0001");
    aad.extend_from_slice(user_id.as_bytes());
    aad.extend_from_slice(snapshot_id.as_bytes());

    let cipher = XChaCha20Poly1305::new((&*keys.manifest_key).into());
    cipher
        .decrypt((&nonce).into(), Payload { msg: &wire[32..], aad: &aad })
        .map_err(|_| KdataError::WrongPasswordOrTampered)
}
