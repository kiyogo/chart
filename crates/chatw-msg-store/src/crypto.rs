//! E2EE Chunk Sealing (`CWMC1_AEAD`), Domain-Separated HKDF Key Derivation,
//! and Compression Side-Channel (CRIME/BREACH) Quantized Padding — Section 17.
//!
//! Security Properties:
//! - Domain-separated HKDF-SHA-256 subkeys derived from the account `root_key` so
//!   storage keys, local cache keys, search index keys, and private dictionary keys
//!   are strictly cryptographically isolated and NEVER reuse expired messaging ratchet keys.
//! - Fresh 96-bit CSPRNG nonce per sealed chunk (never reused).
//! - Full authenticated AAD binding `(magic, format_version, chunk_id, account_id, scope_type,
//!   scope_id, chunk_version, access_epoch, key_version, seq_start, seq_end, dict_id, plaintext_blake3)`
//!   to block replay, rollback, tampering, and cross-conversation/cross-account substitution.
//! - Quantized power-of-two / bucketed padding (`CWPD1`) obscuring fine-grained compressed size
//!   variations to mitigate compression length side-channels.

use crate::model::{
    ENCRYPTION_VERSION_V1, MsgStoreError, MsgStoreResult, ScopeKind,
};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SEALED_CHUNK_MAGIC: &[u8; 6] = b"CWEC01";
pub const PADDING_MAGIC: &[u8; 4] = b"CWPD";

/// Zeroized 256-bit symmetric key material.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey([u8; 32]);

impl DerivedKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Derive a domain-separated 256-bit subkey via HKDF-SHA-256.
pub fn derive_domain_key(root_key: &[u8], domain_info: &str) -> MsgStoreResult<DerivedKey> {
    if root_key.len() < 16 {
        return Err(MsgStoreError::CryptoError("root_key too short (<16B)".into()));
    }
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-msg-store-hkdf-v1"), root_key);
    let mut okm = [0u8; 32];
    hk.expand(domain_info.as_bytes(), &mut okm)
        .map_err(|_| MsgStoreError::CryptoError("hkdf expand failed".into()))?;
    Ok(DerivedKey(okm))
}

pub fn derive_chunk_storage_key(
    root_key: &[u8],
    account_id: &str,
    scope_type: ScopeKind,
    scope_id: &str,
    key_version: u32,
) -> MsgStoreResult<DerivedKey> {
    derive_domain_key(
        root_key,
        &format!(
            "chunk-storage-v1:{account_id}:{}:{scope_id}:kv{key_version}",
            scope_type.as_str()
        ),
    )
}

pub fn derive_local_cache_key(
    root_key: &[u8],
    account_id: &str,
    device_id: &str,
) -> MsgStoreResult<DerivedKey> {
    derive_domain_key(root_key, &format!("local-cache-v1:{account_id}:{device_id}"))
}

pub fn derive_search_index_key(
    root_key: &[u8],
    account_id: &str,
    key_version: u32,
) -> MsgStoreResult<DerivedKey> {
    derive_domain_key(root_key, &format!("search-index-v1:{account_id}:kv{key_version}"))
}

pub fn derive_private_dict_key(
    root_key: &[u8],
    account_id: &str,
    scope_id: &str,
    dict_version: u32,
) -> MsgStoreResult<DerivedKey> {
    derive_domain_key(
        root_key,
        &format!("private-dict-v1:{account_id}:{scope_id}:dv{dict_version}"),
    )
}

/// Cryptographic binding parameters authenticated as AES-256-GCM AAD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkAadContext {
    pub chunk_id: String,
    pub account_id: String,
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub chunk_version: u32,
    pub access_epoch: u32,
    pub key_version: u32,
    pub seq_start: i64,
    pub seq_end: i64,
    pub dictionary_id: u32,
    pub plaintext_blake3_hex: String,
}

impl ChunkAadContext {
    pub fn to_aad_bytes(&self) -> Vec<u8> {
        format!(
            "CWEC01|ev={}|cid={}|acc={}|st={}|sid={}|cv={}|ep={}|kv={}|s0={}|s1={}|dict={}|pb3={}",
            ENCRYPTION_VERSION_V1,
            self.chunk_id,
            self.account_id,
            self.scope_type.as_str(),
            self.scope_id,
            self.chunk_version,
            self.access_epoch,
            self.key_version,
            self.seq_start,
            self.seq_end,
            self.dictionary_id,
            self.plaintext_blake3_hex
        )
        .into_bytes()
    }
}

/// Quantized bucket padding (Section 17.1) to obscure exact compressed sizes against
/// compression length side-channel attacks (CRIME/BREACH).
///
/// Bucket step scales smoothly with size:
/// - `<= 4 KiB` -> multiple of `256 B` (min `256 B`)
/// - `<= 64 KiB` -> multiple of `1 KiB`
/// - `<= 1 MiB` -> multiple of `4 KiB`
/// - `> 1 MiB` -> multiple of `16 KiB`
///
/// Format: `CWPD`(4B) || `payload_len: u32 LE`(4B) || `payload` || `zero_pad`
pub fn pad_for_side_channel_resistance(compressed: &[u8]) -> Vec<u8> {
    let raw_total = 8 + compressed.len();
    let bucket: usize = if raw_total <= 4 * 1024 {
        256
    } else if raw_total <= 64 * 1024 {
        1024
    } else if raw_total <= 1024 * 1024 {
        4096
    } else {
        16384
    };
    let padded_len = raw_total.div_ceil(bucket) * bucket;
    let mut out = vec![0u8; padded_len];
    out[..4].copy_from_slice(PADDING_MAGIC);
    out[4..8].copy_from_slice(&(compressed.len() as u32).to_le_bytes());
    out[8..8 + compressed.len()].copy_from_slice(compressed);
    out
}

pub fn unpad_side_channel_frame(padded: &[u8]) -> MsgStoreResult<&[u8]> {
    if padded.len() < 8 || &padded[..4] != PADDING_MAGIC {
        return Err(MsgStoreError::InvalidFormat("bad CWPD padding header".into()));
    }
    let payload_len = u32::from_le_bytes([padded[4], padded[5], padded[6], padded[7]]) as usize;
    if 8 + payload_len > padded.len() {
        return Err(MsgStoreError::InvalidFormat("CWPD payload length OOB".into()));
    }
    // Verify padding tail bytes are strictly zero (constant-ish integrity hygiene)
    if padded[8 + payload_len..].iter().any(|&b| b != 0) {
        return Err(MsgStoreError::InvalidFormat("non-zero padding tail".into()));
    }
    Ok(&padded[8..8 + payload_len])
}

pub fn random_nonce_12() -> MsgStoreResult<[u8; 12]> {
    let mut nonce = [0u8; 12];
    getrandom::getrandom(&mut nonce)
        .map_err(|e| MsgStoreError::CryptoError(format!("csprng failed: {e}")))?;
    Ok(nonce)
}

/// Seal padded compressed payload into `CWEC01` AEAD envelope:
/// `CWEC01`(6B) || `enc_ver: u16 LE`(2B) || `nonce: 12B` || `AES-256-GCM(padded, AAD)`.
pub fn seal_chunk_payload(
    key: &DerivedKey,
    aad: &ChunkAadContext,
    compressed_payload: &[u8],
) -> MsgStoreResult<(Vec<u8>, String, usize)> {
    let padded = pad_for_side_channel_resistance(compressed_payload);
    let padded_len = padded.len();
    let nonce_bytes = random_nonce_12()?;
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| MsgStoreError::CryptoError(format!("aes init: {e}")))?;
    let aad_bytes = aad.to_aad_bytes();
    let ct = cipher
        .encrypt(
            &Nonce::from(nonce_bytes),
            Payload {
                msg: &padded,
                aad: &aad_bytes,
            },
        )
        .map_err(|e| MsgStoreError::CryptoError(format!("aes encrypt: {e}")))?;

    let mut env = Vec::with_capacity(6 + 2 + 12 + ct.len());
    env.extend_from_slice(SEALED_CHUNK_MAGIC);
    env.extend_from_slice(&ENCRYPTION_VERSION_V1.to_le_bytes());
    env.extend_from_slice(&nonce_bytes);
    env.extend_from_slice(&ct);
    Ok((env, B64.encode(nonce_bytes), padded_len))
}

/// Verify AAD + AEAD tag, decrypt `CWEC01` envelope, and unpad back to exact compressed bytes.
pub fn open_chunk_payload(
    key: &DerivedKey,
    aad: &ChunkAadContext,
    sealed_env: &[u8],
) -> MsgStoreResult<Vec<u8>> {
    if sealed_env.len() < 6 + 2 + 12 + 16 || &sealed_env[..6] != SEALED_CHUNK_MAGIC {
        return Err(MsgStoreError::InvalidFormat("bad CWEC01 magic or length".into()));
    }
    let ver = u16::from_le_bytes([sealed_env[6], sealed_env[7]]);
    if ver != ENCRYPTION_VERSION_V1 {
        return Err(MsgStoreError::UnsupportedVersion(ver));
    }
    let nonce_arr: [u8; 12] = sealed_env[8..20]
        .try_into()
        .map_err(|_| MsgStoreError::InvalidFormat("nonce slice".into()))?;
    let ct = &sealed_env[20..];

    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| MsgStoreError::CryptoError(format!("aes init: {e}")))?;
    let aad_bytes = aad.to_aad_bytes();
    let padded = cipher
        .decrypt(
            &Nonce::from(nonce_arr),
            Payload {
                msg: ct,
                aad: &aad_bytes,
            },
        )
        .map_err(|_| MsgStoreError::CryptoError("chunk AEAD authentication failed".into()))?;
    let unpadded = unpad_side_channel_frame(&padded)?;
    Ok(unpadded.to_vec())
}

/// Generic authenticated encryption helper for search shards, local cache, and private dictionaries.
pub fn seal_aead_blob(key: &DerivedKey, aad: &[u8], plaintext: &[u8]) -> MsgStoreResult<Vec<u8>> {
    let nonce = random_nonce_12()?;
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| MsgStoreError::CryptoError(format!("aes init: {e}")))?;
    let ct = cipher
        .encrypt(&Nonce::from(nonce), Payload { msg: plaintext, aad })
        .map_err(|e| MsgStoreError::CryptoError(format!("aes encrypt: {e}")))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open_aead_blob(key: &DerivedKey, aad: &[u8], blob: &[u8]) -> MsgStoreResult<Vec<u8>> {
    if blob.len() < 12 + 16 {
        return Err(MsgStoreError::InvalidFormat("aead blob too short".into()));
    }
    let nonce_arr: [u8; 12] = blob[..12]
        .try_into()
        .map_err(|_| MsgStoreError::InvalidFormat("nonce slice".into()))?;
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| MsgStoreError::CryptoError(format!("aes init: {e}")))?;
    cipher
        .decrypt(
            &Nonce::from(nonce_arr),
            Payload {
                msg: &blob[12..],
                aad,
            },
        )
        .map_err(|_| MsgStoreError::CryptoError("aead authentication failed".into()))
}
