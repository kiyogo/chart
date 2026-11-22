use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{Mutex, OnceLock},
};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use p256::{
    PublicKey, SecretKey,
    ecdsa::{
        Signature, SigningKey, VerifyingKey,
        signature::{Signer, Verifier},
    },
    elliptic_curve::{
        rand_core::{OsRng, RngCore},
        sec1::ToEncodedPoint,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::platform;

pub const PROTOCOL_VERSION: u8 = 1;
pub const PROTOCOL_VERSION_V2: u8 = 2;

const ARGON2_MEM_KIB: u32 = 19_456;
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const MAX_SKIPPED_RATCHET_KEYS: usize = 64;
const MAX_SEEN_REPLAY_IDS: usize = 512;

type HmacSha256 = Hmac<Sha256>;

/// In-memory session cache for unlocked passphrase-protected E2EE vaults.
/// Never persisted to disk/localStorage; explicitly wiped on lock/logout.
static UNLOCKED_SESSION_KEYS: OnceLock<Mutex<HashMap<String, UserKeys>>> = OnceLock::new();

/// Per-conversation monotonic sending counters and replay-detection state in memory.
static CONVO_SEND_COUNTERS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static SEEN_MESSAGE_IDS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

fn unlocked_store() -> &'static Mutex<HashMap<String, UserKeys>> {
    UNLOCKED_SESSION_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn send_counters() -> &'static Mutex<HashMap<String, u64>> {
    CONVO_SEND_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn seen_replay_ids() -> &'static Mutex<VecDeque<String>> {
    SEEN_MESSAGE_IDS.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_SEEN_REPLAY_IDS)))
}

#[derive(Clone)]
pub struct UserKeys {
    pub ecdh_secret: [u8; 32],
    pub ecdsa_secret: [u8; 32],
    /// 256-bit CSPRNG Account Root Key used to derive backup, local cache & sync subkeys.
    pub root_key: [u8; 32],
    pub ecdh_raw: [u8; 65],
    pub ecdsa_raw: [u8; 65],
    /// Legacy v1 fingerprint = SHA-256(ecdh_raw) for backward compatibility with existing envelopes.
    pub fingerprint: [u8; 32],
    /// Composite v2 identity fingerprint = SHA-256("chatw-id-v2:" || ecdh_raw || ":" || ecdsa_raw).
    pub identity_fingerprint: [u8; 32],
    pub ecdh_raw_b64: String,
    pub ecdsa_raw_b64: String,
    pub fingerprint_b64: String,
    pub identity_fingerprint_b64: String,
}

impl fmt::Debug for UserKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserKeys")
            .field("ecdh_secret", &"<redacted>")
            .field("ecdsa_secret", &"<redacted>")
            .field("root_key", &"<redacted>")
            .field("fingerprint_b64", &self.fingerprint_b64)
            .field("identity_fingerprint_b64", &self.identity_fingerprint_b64)
            .finish()
    }
}

impl Drop for UserKeys {
    fn drop(&mut self) {
        self.ecdh_secret.zeroize();
        self.ecdsa_secret.zeroize();
        self.root_key.zeroize();
    }
}

impl PartialEq for UserKeys {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
            && self.identity_fingerprint == other.identity_fingerprint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerKeyChangeAlert {
    pub peer_id: String,
    pub previous_fingerprint: String,
    pub new_fingerprint: String,
    pub previous_identity_fingerprint: String,
    pub new_identity_fingerprint: String,
}

#[derive(Clone, Debug)]
pub struct PeerKeys {
    pub ecdh_raw: [u8; 65],
    pub ecdsa_raw: [u8; 65],
    pub fingerprint: [u8; 32],
    pub identity_fingerprint: [u8; 32],
    pub fingerprint_b64: String,
    pub identity_fingerprint_b64: String,
    pub key_changed_warning: Option<PeerKeyChangeAlert>,
}

#[derive(Serialize, Deserialize)]
struct StoredUserKeys {
    ecdh_secret_b64: String,
    ecdsa_secret_b64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root_key_b64: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredPeerKeys {
    ecdh_b64: String,
    ecdsa_b64: String,
    fingerprint_b64: String,
    #[serde(default)]
    identity_fingerprint_b64: String,
    #[serde(default)]
    verified: bool,
}

pub fn base64url_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn base64url_decode(s: &str) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| "invalid_base64url".to_string())
}

/// Legacy v1 fingerprint = SHA-256(ecdh_raw).
pub fn compute_fingerprint(ecdh_raw: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ecdh_raw);
    hasher.finalize().into()
}

/// Composite v2 identity fingerprint binding BOTH 65-byte ECDH and 65-byte ECDSA public keys.
pub fn compute_identity_fingerprint(ecdh_raw: &[u8], ecdsa_raw: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"chatw-id-v2:");
    hasher.update(ecdh_raw);
    hasher.update(b":");
    hasher.update(ecdsa_raw);
    hasher.finalize().into()
}

/// Derive a 256-bit key from a user passphrase and 16-byte random salt using Argon2id.
pub fn derive_argon2id_key(
    passphrase: &str,
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<[u8; 32], String> {
    if salt.len() < 8 {
        return Err("invalid_kdf_salt".into());
    }
    let params = Params::new(
        mem_kib.clamp(8_192, 262_144),
        time_cost.clamp(1, 16),
        parallelism.clamp(1, 8),
        Some(32),
    )
    .map_err(|_| "invalid_argon2_params".to_string())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut out)
        .map_err(|_| "argon2id_derive_failed".to_string())?;
    Ok(out)
}

impl UserKeys {
    pub fn generate() -> Self {
        let ecdh_sk = SecretKey::random(&mut OsRng);
        let ecdsa_sk = SigningKey::random(&mut OsRng);
        let mut ecdh_secret = [0u8; 32];
        ecdh_secret.copy_from_slice(&ecdh_sk.to_bytes());
        let mut ecdsa_secret = [0u8; 32];
        ecdsa_secret.copy_from_slice(&ecdsa_sk.to_bytes());
        let mut root_key = [0u8; 32];
        OsRng.fill_bytes(&mut root_key);
        Self::from_secrets_with_root(ecdh_secret, ecdsa_secret, Some(root_key))
            .expect("fresh keys are valid")
    }

    pub fn from_secrets(ecdh_secret: [u8; 32], ecdsa_secret: [u8; 32]) -> Result<Self, String> {
        Self::from_secrets_with_root(ecdh_secret, ecdsa_secret, None)
    }

    pub fn from_secrets_with_root(
        ecdh_secret: [u8; 32],
        ecdsa_secret: [u8; 32],
        root_key_opt: Option<[u8; 32]>,
    ) -> Result<Self, String> {
        let ecdh_sk =
            SecretKey::from_slice(&ecdh_secret).map_err(|_| "invalid_ecdh_secret".to_string())?;
        let ecdsa_sk = SigningKey::from_slice(&ecdsa_secret)
            .map_err(|_| "invalid_ecdsa_secret".to_string())?;

        let ecdh_pub = ecdh_sk.public_key().to_encoded_point(false);
        let mut ecdh_raw = [0u8; 65];
        ecdh_raw.copy_from_slice(ecdh_pub.as_bytes());

        let ecdsa_pub = ecdsa_sk.verifying_key().to_encoded_point(false);
        let mut ecdsa_raw = [0u8; 65];
        ecdsa_raw.copy_from_slice(ecdsa_pub.as_bytes());

        let fingerprint = compute_fingerprint(&ecdh_raw);
        let identity_fingerprint = compute_identity_fingerprint(&ecdh_raw, &ecdsa_raw);
        let ecdh_raw_b64 = base64url_encode(&ecdh_raw);
        let ecdsa_raw_b64 = base64url_encode(&ecdsa_raw);
        let fingerprint_b64 = base64url_encode(&fingerprint);
        let identity_fingerprint_b64 = base64url_encode(&identity_fingerprint);

        // If importing a legacy v1 keypair without an explicit Account Root Key, deterministically
        // derive a domain-separated Account Root Key from the private identity secrets via HKDF
        // so existing accounts seamlessly gain deterministic local cache encryption across devices.
        let root_key = match root_key_opt {
            Some(rk) => rk,
            None => {
                let mut ikm = [0u8; 64];
                ikm[..32].copy_from_slice(&ecdh_secret);
                ikm[32..].copy_from_slice(&ecdsa_secret);
                let hk = Hkdf::<Sha256>::new(Some(b"chatw-account-root-v1"), &ikm);
                ikm.zeroize();
                let mut rk = [0u8; 32];
                hk.expand(b"account-root-key", &mut rk)
                    .map_err(|_| "hkdf_root_failed".to_string())?;
                rk
            }
        };

        Ok(Self {
            ecdh_secret,
            ecdsa_secret,
            root_key,
            ecdh_raw,
            ecdsa_raw,
            fingerprint,
            identity_fingerprint,
            ecdh_raw_b64,
            ecdsa_raw_b64,
            fingerprint_b64,
            identity_fingerprint_b64,
        })
    }

    /// True if `candidate_fp` matches either this key's v1 ECDH fingerprint or v2 composite fingerprint.
    pub fn matches_fingerprint(&self, candidate_fp: &[u8]) -> bool {
        candidate_fp.len() == 32
            && (candidate_fp == self.fingerprint || candidate_fp == self.identity_fingerprint)
    }

    /// Derive a domain-separated 32-byte subkey from the Account Root Key (never exposing root_key directly).
    pub fn derive_subkey(&self, domain: &str) -> Result<[u8; 32], String> {
        let hk = Hkdf::<Sha256>::new(Some(b"chatw-root-subkey-v1"), &self.root_key);
        let mut out = [0u8; 32];
        hk.expand(domain.as_bytes(), &mut out)
            .map_err(|_| "hkdf_subkey_failed".to_string())?;
        Ok(out)
    }
}

fn key_slot(user_id: Option<&str>) -> String {
    match user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_keys_{uid}"),
        _ => "chatw_e2ee_keys_current".to_string(),
    }
}

/// Returns true if the account has an Argon2id passphrase-protected E2EE vault stored locally
/// that is currently locked (not present in the in-memory unlocked session map).
pub fn has_locked_passphrase_vault(user_id: &str) -> bool {
    let slot = key_slot(Some(user_id));
    if let Ok(guard) = unlocked_store().lock()
        && guard.contains_key(&slot)
    {
        return false;
    }
    platform::storage_get(&slot)
        .map(|raw| raw.trim_start().starts_with("enc:v2:argon2id:"))
        .unwrap_or(false)
}

/// Returns true if the account's local vault is already upgraded to Argon2id passphrase wrapping.
pub fn is_passphrase_vault_enabled(user_id: &str) -> bool {
    let slot = key_slot(Some(user_id));
    platform::storage_get(&slot)
        .map(|raw| raw.trim_start().starts_with("enc:v2:argon2id:"))
        .unwrap_or(false)
}

/// Wipe unlocked E2EE keys from memory on logout or manual lock.
pub fn lock_and_wipe_session_keys(user_id: Option<&str>) {
    if let Ok(mut guard) = unlocked_store().lock() {
        match user_id {
            Some(uid) if !uid.is_empty() => {
                guard.remove(&key_slot(Some(uid)));
            }
            _ => {
                guard.clear();
            }
        }
    }
}

pub fn get_user_keys(user_id: Option<&str>) -> Option<UserKeys> {
    let primary_key = key_slot(user_id);

    // 1. Check in-memory unlocked session keys first (for passphrase-protected enc:v2:argon2id vaults).
    if let Ok(guard) = unlocked_store().lock()
        && let Some(cached) = guard.get(&primary_key)
    {
        return Some(cached.clone());
    }

    // Never fall back to chatw_e2ee_keys_current when user_id is specified;
    // doing so leaks Account A's private key into Account B on the same browser.
    let raw = platform::storage_get(&primary_key)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Passphrase-protected vault cannot be unlocked silently without the user's passphrase.
    if trimmed.starts_with("enc:v2:argon2id:") {
        return None;
    }

    let was_legacy_plaintext = !trimmed.starts_with("enc:v1:");
    let keys = parse_stored_user_keys(trimmed, &primary_key).ok()?;
    if was_legacy_plaintext {
        // Transparent non-destructive migration: upgrade legacy plaintext keys to AES-256-GCM wrapped format in-place.
        let _ = store_user_keys(user_id, &keys);
    }
    Some(keys)
}

#[allow(deprecated)]
fn derive_local_wrapping_key(storage_slot: &str) -> Result<[u8; 32], String> {
    let seed_b64 = match platform::storage_get("__chatw_device_kwrap_v1") {
        Some(s) if s.len() >= 40 => s,
        _ => {
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let encoded = base64url_encode(&seed);
            seed.zeroize();
            platform::storage_set("__chatw_device_kwrap_v1", &encoded);
            encoded
        }
    };
    let mut seed_bytes = base64url_decode(&seed_b64)?;
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-local-keywrap-v1"), &seed_bytes);
    seed_bytes.zeroize();
    let mut kek = [0u8; 32];
    hk.expand(format!("slot:{storage_slot}").as_bytes(), &mut kek)
        .map_err(|_| "hkdf_wrap_failed".to_string())?;
    Ok(kek)
}

#[allow(deprecated)]
fn encrypt_stored_payload(storage_slot: &str, plaintext_json: &str) -> Result<String, String> {
    let mut kek = derive_local_wrapping_key(storage_slot)?;
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string());
    kek.zeroize();
    let cipher = cipher?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext_json.as_bytes(),
                aad: storage_slot.as_bytes(),
            },
        )
        .map_err(|_| "aes_wrap_encrypt_failed".to_string())?;
    Ok(format!(
        "enc:v1:{}:{}",
        base64url_encode(&nonce_bytes),
        base64url_encode(&ct)
    ))
}

#[allow(deprecated)]
fn decrypt_stored_payload(storage_slot: &str, wrapped: &str) -> Result<String, String> {
    let rest = wrapped
        .trim()
        .strip_prefix("enc:v1:")
        .ok_or_else(|| "not_encrypted_v1".to_string())?;
    let (nonce_b64, ct_b64) = rest
        .split_once(':')
        .ok_or_else(|| "invalid_enc_v1_format".to_string())?;
    let nonce_vec = base64url_decode(nonce_b64)?;
    if nonce_vec.len() != 12 {
        return Err("invalid_wrap_nonce".into());
    }
    let ct_vec = base64url_decode(ct_b64)?;
    let mut kek = derive_local_wrapping_key(storage_slot)?;
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string());
    kek.zeroize();
    let cipher = cipher?;
    let pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce_vec),
            Payload {
                msg: &ct_vec,
                aad: storage_slot.as_bytes(),
            },
        )
        .map_err(|_| "aes_wrap_decrypt_failed".to_string())?;
    String::from_utf8(pt).map_err(|_| "invalid_wrap_utf8".into())
}

/// Upgrade an account's local E2EE key storage to Argon2id passphrase protection (`enc:v2:argon2id:...`).
/// After calling this, stealing localStorage alone is insufficient to decrypt the user's E2EE private keys.
#[allow(deprecated)]
pub fn enable_passphrase_vault(
    user_id: &str,
    keys: &UserKeys,
    passphrase: &str,
) -> Result<(), String> {
    if passphrase.chars().count() < 8 {
        return Err("The E2EE key password needs at least 8 characters".into());
    }
    let slot = key_slot(Some(user_id));
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    let mut kek = derive_argon2id_key(
        passphrase,
        &salt,
        ARGON2_MEM_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
    )?;
    let mut plain_json = export_user_keys_json(keys);
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string());
    kek.zeroize();
    let cipher = cipher?;
    let aad = format!("v2:argon2id:{slot}");
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plain_json.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "aes_wrap_encrypt_failed".to_string());
    plain_json.zeroize();
    let ct = ct?;

    let wrapped = format!(
        "enc:v2:argon2id:{}:{}:{}",
        base64url_encode(&salt),
        base64url_encode(&nonce),
        base64url_encode(&ct)
    );
    platform::storage_set(&slot, &wrapped);
    if let Ok(mut guard) = unlocked_store().lock() {
        guard.insert(slot, keys.clone());
    }
    Ok(())
}

/// Unlock an `enc:v2:argon2id:...` local vault using the user's E2EE passphrase.
#[allow(deprecated)]
pub fn unlock_passphrase_vault(user_id: &str, passphrase: &str) -> Result<UserKeys, String> {
    let slot = key_slot(Some(user_id));
    let raw = platform::storage_get(&slot).ok_or_else(|| "vault_not_found".to_string())?;
    let rest = raw
        .trim()
        .strip_prefix("enc:v2:argon2id:")
        .ok_or_else(|| "not_passphrase_vault".to_string())?;
    let mut parts = rest.splitn(3, ':');
    let salt_b64 = parts.next().ok_or("invalid_vault_format")?;
    let nonce_b64 = parts.next().ok_or("invalid_vault_format")?;
    let ct_b64 = parts.next().ok_or("invalid_vault_format")?;

    let salt = base64url_decode(salt_b64)?;
    let nonce = base64url_decode(nonce_b64)?;
    let ct = base64url_decode(ct_b64)?;
    if nonce.len() != 12 {
        return Err("invalid_vault_nonce".into());
    }
    let mut kek = derive_argon2id_key(
        passphrase,
        &salt,
        ARGON2_MEM_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
    )?;
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string());
    kek.zeroize();
    let cipher = cipher?;
    let aad = format!("v2:argon2id:{slot}");
    let mut pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ct,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Wrong E2EE unlock password".to_string())?;
    let json_str = std::str::from_utf8(&pt)
        .map_err(|_| "invalid_vault_utf8".to_string())?
        .to_string();
    pt.zeroize();

    let keys = parse_stored_user_keys(&json_str, &slot)?;
    if let Ok(mut guard) = unlocked_store().lock() {
        guard.insert(slot, keys.clone());
    }
    Ok(keys)
}

fn parse_stored_user_keys(raw: &str, storage_slot: &str) -> Result<UserKeys, String> {
    let trimmed = raw.trim();
    let mut json_payload = if trimmed.starts_with("enc:v1:") {
        decrypt_stored_payload(storage_slot, trimmed)?
    } else {
        trimmed.to_string()
    };
    let stored: StoredUserKeys =
        serde_json::from_str(&json_payload).map_err(|_| "invalid_key_json".to_string())?;
    json_payload.zeroize();

    let mut ecdh_vec = base64url_decode(&stored.ecdh_secret_b64)?;
    let mut ecdsa_vec = base64url_decode(&stored.ecdsa_secret_b64)?;
    if ecdh_vec.len() != 32 || ecdsa_vec.len() != 32 {
        ecdh_vec.zeroize();
        ecdsa_vec.zeroize();
        return Err("invalid_key_length".into());
    }
    let mut ecdh_secret = [0u8; 32];
    ecdh_secret.copy_from_slice(&ecdh_vec);
    ecdh_vec.zeroize();
    let mut ecdsa_secret = [0u8; 32];
    ecdsa_secret.copy_from_slice(&ecdsa_vec);
    ecdsa_vec.zeroize();

    let root_key_opt = if let Some(ref rk_b64) = stored.root_key_b64 {
        let mut rk_vec = base64url_decode(rk_b64)?;
        if rk_vec.len() == 32 {
            let mut rk = [0u8; 32];
            rk.copy_from_slice(&rk_vec);
            rk_vec.zeroize();
            Some(rk)
        } else {
            rk_vec.zeroize();
            None
        }
    } else {
        None
    };

    UserKeys::from_secrets_with_root(ecdh_secret, ecdsa_secret, root_key_opt)
}

pub fn create_and_store_user_keys(user_id: Option<&str>) -> Result<UserKeys, String> {
    let keys = UserKeys::generate();
    store_user_keys(user_id, &keys)?;
    Ok(keys)
}

/// Persist user keys wrapped with AEAD. NEVER falls back to storing plaintext on encryption error.
pub fn store_user_keys(user_id: Option<&str>, keys: &UserKeys) -> Result<(), String> {
    let stored = StoredUserKeys {
        ecdh_secret_b64: base64url_encode(&keys.ecdh_secret),
        ecdsa_secret_b64: base64url_encode(&keys.ecdsa_secret),
        root_key_b64: Some(base64url_encode(&keys.root_key)),
    };
    let slot = key_slot(user_id);
    let mut json = serde_json::to_string(&stored).map_err(|_| "key_serialize_failed".to_string())?;
    let wrapped = encrypt_stored_payload(&slot, &json);
    json.zeroize();
    let to_store = wrapped?;
    platform::storage_set(&slot, &to_store);
    if let Ok(mut guard) = unlocked_store().lock() {
        guard.insert(slot, keys.clone());
    }
    Ok(())
}

pub fn export_user_keys_json(keys: &UserKeys) -> String {
    let stored = StoredUserKeys {
        ecdh_secret_b64: base64url_encode(&keys.ecdh_secret),
        ecdsa_secret_b64: base64url_encode(&keys.ecdsa_secret),
        root_key_b64: Some(base64url_encode(&keys.root_key)),
    };
    serde_json::to_string(&stored).unwrap_or_default()
}

pub fn import_user_keys_json(user_id: &str, raw_json: &str) -> Result<UserKeys, String> {
    let slot = format!("chatw_e2ee_keys_{user_id}");
    let keys = parse_stored_user_keys(raw_json.trim(), &slot)?;
    store_user_keys(Some(user_id), &keys)?;
    Ok(keys)
}

pub fn clear_legacy_active_key() {
    platform::storage_set("chatw_e2ee_keys_current", "");
    lock_and_wipe_session_keys(None);
}

// -----------------------------------------------------------------------------
// Account-Scoped Encrypted Local Cache (Previews, Reminders, Metadata)
// -----------------------------------------------------------------------------

#[allow(deprecated)]
pub fn encrypt_account_local_data(user_id: &str, slot_name: &str, plaintext: &str) -> Result<String, String> {
    let keys = get_user_keys(Some(user_id)).ok_or_else(|| "e2ee_keys_locked".to_string())?;
    let mut subkey = keys.derive_subkey(&format!("local-cache:{user_id}:{slot_name}"))?;
    let cipher = Aes256Gcm::new_from_slice(&subkey).map_err(|_| "aes_init_failed".to_string());
    subkey.zeroize();
    let cipher = cipher?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let aad = format!("cache:{user_id}:{slot_name}");
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "cache_encrypt_failed".to_string())?;
    Ok(format!(
        "lenc:v1:{}:{}",
        base64url_encode(&nonce),
        base64url_encode(&ct)
    ))
}

#[allow(deprecated)]
pub fn decrypt_account_local_data(user_id: &str, slot_name: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("lenc:v1:") {
        // Legacy plaintext cache: return so caller can transparently re-encrypt on next write.
        return Ok(trimmed.to_string());
    }
    let keys = get_user_keys(Some(user_id)).ok_or_else(|| "e2ee_keys_locked".to_string())?;
    let rest = trimmed.strip_prefix("lenc:v1:").ok_or("invalid_lenc")?;
    let (nonce_b64, ct_b64) = rest.split_once(':').ok_or("invalid_lenc_parts")?;
    let nonce = base64url_decode(nonce_b64)?;
    let ct = base64url_decode(ct_b64)?;
    if nonce.len() != 12 {
        return Err("invalid_lenc_nonce".into());
    }
    let mut subkey = keys.derive_subkey(&format!("local-cache:{user_id}:{slot_name}"))?;
    let cipher = Aes256Gcm::new_from_slice(&subkey).map_err(|_| "aes_init_failed".to_string());
    subkey.zeroize();
    let cipher = cipher?;
    let aad = format!("cache:{user_id}:{slot_name}");
    let pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ct,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "cache_decrypt_failed".to_string())?;
    String::from_utf8(pt).map_err(|_| "invalid_cache_utf8".into())
}

// -----------------------------------------------------------------------------
// Device-to-Device E2EE Sync Key Transfer (wrapped_sync_key) & Recovery Proofs
// -----------------------------------------------------------------------------

/// Create or load an ephemeral P-256 ECDH transport keypair on a pending device so an existing
/// trusted device can E2EE-wrap the account's UserKeys directly to this device without server access.
pub fn get_or_create_device_transport_pubkey(user_id: &str) -> Result<String, String> {
    let slot = format!("chatw_e2ee_transport_{user_id}");
    if let Some(raw) = platform::storage_get(&slot)
        && !raw.trim().is_empty()
        && let Ok(sk_b64) = decrypt_stored_payload(&slot, raw.trim())
        && let Ok(sk_bytes) = base64url_decode(&sk_b64)
        && let Ok(sk) = SecretKey::from_slice(&sk_bytes)
    {
        return Ok(base64url_encode(sk.public_key().to_encoded_point(false).as_bytes()));
    }
    let sk = SecretKey::random(&mut OsRng);
    let mut sk_bytes = [0u8; 32];
    sk_bytes.copy_from_slice(&sk.to_bytes());
    let sk_b64 = base64url_encode(&sk_bytes);
    sk_bytes.zeroize();
    let wrapped = encrypt_stored_payload(&slot, &sk_b64)?;
    platform::storage_set(&slot, &wrapped);
    Ok(base64url_encode(sk.public_key().to_encoded_point(false).as_bytes()))
}

#[derive(Serialize, Deserialize)]
struct SyncKeyEnvelope {
    v: u8,
    ephem_pub_b64: String,
    nonce_b64: String,
    ct_b64: String,
    sig_b64: String,
}

/// Called by a TRUSTED device when approving a new device login request.
/// Encrypts `my_keys` (ECDH, ECDSA, Account Root Key) to `target_device_ecdh_b64` and signs with ECDSA.
#[allow(deprecated)]
pub fn wrap_sync_key_for_device(
    my_keys: &UserKeys,
    target_device_ecdh_b64: &str,
    request_id: &str,
) -> Result<String, String> {
    let target_pub_bytes = base64url_decode(target_device_ecdh_b64)?;
    let target_pk = PublicKey::from_sec1_bytes(&target_pub_bytes)
        .map_err(|_| "invalid_target_device_ecdh".to_string())?;

    let ephem_sk = SecretKey::random(&mut OsRng);
    let ephem_pub = ephem_sk.public_key().to_encoded_point(false);
    let shared = p256::ecdh::diffie_hellman(ephem_sk.to_nonzero_scalar(), target_pk.as_affine());

    let mut aes_key = [0u8; 32];
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-device-sync-v1"), shared.raw_secret_bytes());
    hk.expand(format!("req:{request_id}").as_bytes(), &mut aes_key)
        .map_err(|_| "hkdf_sync_wrap_failed".to_string())?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|_| "aes_init_failed".to_string());
    aes_key.zeroize();
    let cipher = cipher?;

    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let mut payload_json = export_user_keys_json(my_keys);
    let aad = format!("device-sync-v1:{request_id}");
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: payload_json.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "sync_key_encrypt_failed".to_string());
    payload_json.zeroize();
    let ct = ct?;

    let mut to_sign = Vec::with_capacity(65 + 12 + ct.len() + aad.len());
    to_sign.extend_from_slice(aad.as_bytes());
    to_sign.extend_from_slice(ephem_pub.as_bytes());
    to_sign.extend_from_slice(&nonce);
    to_sign.extend_from_slice(&ct);

    let signing_key = SigningKey::from_slice(&my_keys.ecdsa_secret)
        .map_err(|_| "invalid_signing_key".to_string())?;
    let sig: Signature = signing_key.sign(&to_sign);

    let env = SyncKeyEnvelope {
        v: 1,
        ephem_pub_b64: base64url_encode(ephem_pub.as_bytes()),
        nonce_b64: base64url_encode(&nonce),
        ct_b64: base64url_encode(&ct),
        sig_b64: base64url_encode(sig.to_bytes().as_slice()),
    };
    let json = serde_json::to_string(&env).map_err(|_| "sync_env_encode_failed".to_string())?;
    Ok(format!("sync:v1:{}", base64url_encode(json.as_bytes())))
}

/// Called by the NEWLY APPROVED device upon receiving `wrapped_sync_key` from the trusted approver.
#[allow(deprecated)]
pub fn unwrap_sync_key_on_device(
    user_id: &str,
    wrapped_sync_key: &str,
    request_id: &str,
    expected_identity_ecdsa_b64: Option<&str>,
) -> Result<UserKeys, String> {
    let encoded = wrapped_sync_key
        .trim()
        .strip_prefix("sync:v1:")
        .ok_or_else(|| "invalid_sync_key_prefix".to_string())?;
    let env_bytes = base64url_decode(encoded)?;
    let env: SyncKeyEnvelope =
        serde_json::from_slice(&env_bytes).map_err(|_| "invalid_sync_key_json".to_string())?;
    if env.v != 1 {
        return Err("unsupported_sync_key_version".into());
    }

    let slot = format!("chatw_e2ee_transport_{user_id}");
    let raw_sk = platform::storage_get(&slot).ok_or_else(|| "missing_device_transport_key".to_string())?;
    let sk_b64 = decrypt_stored_payload(&slot, raw_sk.trim())?;
    let mut sk_bytes = base64url_decode(&sk_b64)?;
    let my_transport_sk = SecretKey::from_slice(&sk_bytes)
        .map_err(|_| "invalid_device_transport_key".to_string())?;
    sk_bytes.zeroize();

    let ephem_pub_bytes = base64url_decode(&env.ephem_pub_b64)?;
    let nonce = base64url_decode(&env.nonce_b64)?;
    let ct = base64url_decode(&env.ct_b64)?;
    let sig_bytes = base64url_decode(&env.sig_b64)?;
    if nonce.len() != 12 {
        return Err("invalid_sync_nonce".into());
    }

    let aad = format!("device-sync-v1:{request_id}");
    let mut signed_data = Vec::with_capacity(aad.len() + ephem_pub_bytes.len() + nonce.len() + ct.len());
    signed_data.extend_from_slice(aad.as_bytes());
    signed_data.extend_from_slice(&ephem_pub_bytes);
    signed_data.extend_from_slice(&nonce);
    signed_data.extend_from_slice(&ct);

    if let Some(ecdsa_b64) = expected_identity_ecdsa_b64 {
        let ecdsa_bytes = base64url_decode(ecdsa_b64)?;
        let vk = VerifyingKey::from_sec1_bytes(&ecdsa_bytes)
            .map_err(|_| "invalid_expected_ecdsa".to_string())?;
        let sig = Signature::from_slice(&sig_bytes).map_err(|_| "invalid_sync_sig".to_string())?;
        vk.verify(&signed_data, &sig)
            .map_err(|_| "sync_key_signature_mismatch".to_string())?;
    }

    let ephem_pk = PublicKey::from_sec1_bytes(&ephem_pub_bytes)
        .map_err(|_| "invalid_sync_ephem_key".to_string())?;
    let shared = p256::ecdh::diffie_hellman(my_transport_sk.to_nonzero_scalar(), ephem_pk.as_affine());

    let mut aes_key = [0u8; 32];
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-device-sync-v1"), shared.raw_secret_bytes());
    hk.expand(format!("req:{request_id}").as_bytes(), &mut aes_key)
        .map_err(|_| "hkdf_sync_unwrap_failed".to_string())?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|_| "aes_init_failed".to_string());
    aes_key.zeroize();
    let cipher = cipher?;
    let mut pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ct,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "sync_key_decrypt_failed".to_string())?;
    let json_str = std::str::from_utf8(&pt)
        .map_err(|_| "invalid_sync_utf8".to_string())?
        .to_string();
    pt.zeroize();

    let keys = import_user_keys_json(user_id, &json_str)?;

    // Also self-verify the signature against the recovered ECDSA key in case expected_identity_ecdsa_b64 was omitted.
    let vk = VerifyingKey::from_sec1_bytes(&keys.ecdsa_raw)
        .map_err(|_| "invalid_recovered_ecdsa".to_string())?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "invalid_sync_sig".to_string())?;
    vk.verify(&signed_data, &sig)
        .map_err(|_| "sync_key_self_signature_mismatch".to_string())?;

    // Wipe one-time device transport private key after successful transfer.
    platform::storage_set(&slot, "");
    Ok(keys)
}

/// Produce an ECDSA cryptographic proof of identity key ownership for `POST /api/devices/recover-lost`.
#[allow(deprecated)]
pub fn sign_device_recovery_proof(keys: &UserKeys, device_id_uuid: &str) -> Result<String, String> {
    let did = Uuid::parse_str(device_id_uuid).map_err(|_| "invalid_device_uuid".to_string())?;
    let signing_key = SigningKey::from_slice(&keys.ecdsa_secret)
        .map_err(|_| "invalid_signing_key".to_string())?;
    let mut msg = Vec::with_capacity(24 + 16);
    msg.extend_from_slice(b"chatw-recover-device-v1:");
    msg.extend_from_slice(did.as_bytes());
    let sig: Signature = signing_key.sign(&msg);
    Ok(base64url_encode(sig.to_bytes().as_slice()))
}

/// Sign a fresh signed prekey + batch of one-time prekeys for `PUT /api/keys/prekeys`.
#[allow(deprecated)]
pub fn build_signed_prekey_bundle(keys: &UserKeys, count: usize) -> Result<(String, String, Vec<String>), String> {
    let spk_sk = SecretKey::random(&mut OsRng);
    let spk_pub = spk_sk.public_key().to_encoded_point(false);
    let signing_key = SigningKey::from_slice(&keys.ecdsa_secret)
        .map_err(|_| "invalid_signing_key".to_string())?;
    let mut msg = Vec::with_capacity(13 + 65);
    msg.extend_from_slice(b"chatw-spk-v2:");
    msg.extend_from_slice(spk_pub.as_bytes());
    let sig: Signature = signing_key.sign(&msg);

    let mut otks = Vec::with_capacity(count.min(32));
    for _ in 0..count.min(32) {
        let otk_sk = SecretKey::random(&mut OsRng);
        otks.push(base64url_encode(otk_sk.public_key().to_encoded_point(false).as_bytes()));
    }
    Ok((
        base64url_encode(spk_pub.as_bytes()),
        base64url_encode(sig.to_bytes().as_slice()),
        otks,
    ))
}

// -----------------------------------------------------------------------------
// Strict Peer Key Verification (TOFU + Key Substitution Detection + Safety Number)
// -----------------------------------------------------------------------------

pub fn get_peer_key_change_alert(my_user_id: Option<&str>, peer_id: &str) -> Option<PeerKeyChangeAlert> {
    let slot = match my_user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_peer_alert_{uid}_{peer_id}"),
        _ => format!("chatw_e2ee_peer_alert_{peer_id}"),
    };
    let raw = platform::storage_get(&slot)?;
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

pub fn acknowledge_peer_key_change(my_user_id: Option<&str>, peer_id: &str) {
    let alert_slot = match my_user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_peer_alert_{uid}_{peer_id}"),
        _ => format!("chatw_e2ee_peer_alert_{peer_id}"),
    };
    platform::storage_set(&alert_slot, "");
    let peer_slot = match my_user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_peer_{uid}_{peer_id}"),
        _ => format!("chatw_e2ee_peer_{peer_id}"),
    };
    if let Some(raw) = platform::storage_get(&peer_slot)
        && let Ok(mut stored) = serde_json::from_str::<StoredPeerKeys>(&raw)
    {
        stored.verified = true;
        if let Ok(json) = serde_json::to_string(&stored) {
            platform::storage_set(&peer_slot, &json);
        }
    }
}

/// Compute a deterministic, symmetric 60-digit Safety Number (12 groups of 5 digits) for
/// out-of-band / QR verification between two users, binding BOTH ECDH and ECDSA keys.
pub fn compute_safety_number(
    my_uid: &str,
    my_keys: &UserKeys,
    peer_uid: &str,
    peer_keys: &PeerKeys,
) -> String {
    let mut entries = [
        (my_uid, my_keys.identity_fingerprint),
        (peer_uid, peer_keys.identity_fingerprint),
    ];
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut digest = [0u8; 32];
    let mut h = Sha256::new();
    h.update(b"chatw-safety-number-v2:");
    h.update(entries[0].0.as_bytes());
    h.update(&entries[0].1);
    h.update(entries[1].0.as_bytes());
    h.update(&entries[1].1);
    digest.copy_from_slice(&h.finalize());

    let mut groups = Vec::with_capacity(12);
    for i in 0..12 {
        let mut step = Sha256::new();
        step.update(&digest);
        step.update([i as u8]);
        let out = step.finalize();
        let val = u64::from_be_bytes([
            out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7],
        ]) % 100_000;
        groups.push(format!("{val:05}"));
    }
    groups.join(" ")
}

pub fn get_or_verify_peer_keys(
    peer_id: &str,
    ecdh_raw_b64: &str,
    ecdsa_raw_b64: &str,
    fingerprint_b64: &str,
) -> Result<PeerKeys, String> {
    get_or_verify_peer_keys_scoped(None, peer_id, ecdh_raw_b64, ecdsa_raw_b64, fingerprint_b64)
}

pub fn get_or_verify_peer_keys_scoped(
    my_user_id: Option<&str>,
    peer_id: &str,
    ecdh_raw_b64: &str,
    ecdsa_raw_b64: &str,
    fingerprint_b64: &str,
) -> Result<PeerKeys, String> {
    let ecdh_vec = base64url_decode(ecdh_raw_b64)?;
    let ecdsa_vec = base64url_decode(ecdsa_raw_b64)?;
    let fp_vec = base64url_decode(fingerprint_b64)?;

    if ecdh_vec.len() != 65 || ecdsa_vec.len() != 65 || fp_vec.len() != 32 {
        return Err("invalid_peer_key_length".into());
    }

    let mut ecdh_raw = [0u8; 65];
    ecdh_raw.copy_from_slice(&ecdh_vec);
    let mut ecdsa_raw = [0u8; 65];
    ecdsa_raw.copy_from_slice(&ecdsa_vec);
    let mut fingerprint = [0u8; 32];
    fingerprint.copy_from_slice(&fp_vec);

    // Validate P-256 curve points
    PublicKey::from_sec1_bytes(&ecdh_raw).map_err(|_| "invalid_peer_ecdh_key".to_string())?;
    VerifyingKey::from_sec1_bytes(&ecdsa_raw)
        .map_err(|_| "invalid_peer_ecdsa_key".to_string())?;

    // Verify fingerprint against v1 (SHA-256(ecdh)) or v2 composite (SHA-256(ecdh || ecdsa)).
    let computed_v1 = compute_fingerprint(&ecdh_raw);
    let identity_fingerprint = compute_identity_fingerprint(&ecdh_raw, &ecdsa_raw);
    let identity_fingerprint_b64 = base64url_encode(&identity_fingerprint);
    if computed_v1 != fingerprint && identity_fingerprint != fingerprint {
        return Err("Invalid peer key fingerprint".into());
    }

    // Account-scoped TOFU peer storage + key-substitution / rotation detection.
    let peer_key_slot = match my_user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_peer_{uid}_{peer_id}"),
        _ => format!("chatw_e2ee_peer_{peer_id}"),
    };
    let alert_slot = match my_user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_peer_alert_{uid}_{peer_id}"),
        _ => format!("chatw_e2ee_peer_alert_{peer_id}"),
    };

    let existing_raw = platform::storage_get(&peer_key_slot)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if my_user_id.is_some() {
                platform::storage_get(&format!("chatw_e2ee_peer_{peer_id}"))
                    .filter(|s| !s.trim().is_empty())
            } else {
                None
            }
        });

    let mut key_changed_warning = get_peer_key_change_alert(my_user_id, peer_id);

    if let Some(prev_json) = existing_raw
        && let Ok(prev) = serde_json::from_str::<StoredPeerKeys>(&prev_json)
    {
        let prev_id_fp = if prev.identity_fingerprint_b64.is_empty() {
            prev.fingerprint_b64.clone()
        } else {
            prev.identity_fingerprint_b64.clone()
        };
        if prev.ecdh_b64 != ecdh_raw_b64 || prev.ecdsa_b64 != ecdsa_raw_b64 {
            // Peer's identity key changed! Record prominent TOFU warning instead of silently hiding it.
            let alert = PeerKeyChangeAlert {
                peer_id: peer_id.to_string(),
                previous_fingerprint: prev.fingerprint_b64,
                new_fingerprint: fingerprint_b64.to_string(),
                previous_identity_fingerprint: prev_id_fp,
                new_identity_fingerprint: identity_fingerprint_b64.clone(),
            };
            if let Ok(alert_json) = serde_json::to_string(&alert) {
                platform::storage_set(&alert_slot, &alert_json);
            }
            key_changed_warning = Some(alert);
        }
    }

    let serialized = serde_json::to_string(&StoredPeerKeys {
        ecdh_b64: ecdh_raw_b64.to_string(),
        ecdsa_b64: ecdsa_raw_b64.to_string(),
        fingerprint_b64: fingerprint_b64.to_string(),
        identity_fingerprint_b64: identity_fingerprint_b64.clone(),
        verified: key_changed_warning.is_none(),
    })
    .ok();

    if let Some(json) = serialized {
        platform::storage_set(&peer_key_slot, &json);
    }

    Ok(PeerKeys {
        ecdh_raw,
        ecdsa_raw,
        fingerprint,
        identity_fingerprint,
        fingerprint_b64: fingerprint_b64.to_string(),
        identity_fingerprint_b64,
        key_changed_warning,
    })
}

// -----------------------------------------------------------------------------
// Message Envelope Encryption (Protocol v2 Authenticated Ratchet + v1 Fallback)
// -----------------------------------------------------------------------------

fn next_ratchet_step(sender_id: Uuid, recipient_id: Uuid) -> u64 {
    let key = format!("{sender_id}:{recipient_id}");
    if let Ok(mut guard) = send_counters().lock() {
        let entry = guard.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    } else {
        1
    }
}

fn record_seen_message_id(msg_id: &str) {
    if let Ok(mut guard) = seen_replay_ids().lock() {
        if !guard.iter().any(|x| x == msg_id) {
            if guard.len() >= MAX_SEEN_REPLAY_IDS {
                guard.pop_front();
            }
            guard.push_back(msg_id.to_string());
        }
    }
}

#[allow(deprecated)]
fn build_envelope_v2(
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    recipient_fingerprint: &[u8; 32],
    recipient_ecdh_raw: &[u8; 65],
    sender_ecdsa_secret: &[u8; 32],
    sender_ecdsa_raw: &[u8; 65],
    ratchet_step: u64,
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let ephemeral_sk = SecretKey::random(&mut OsRng);
    let ephemeral_pub = ephemeral_sk.public_key().to_encoded_point(false);

    let recip_pk =
        PublicKey::from_sec1_bytes(recipient_ecdh_raw).map_err(|_| "invalid_recipient_ecdh")?;
    let shared =
        p256::ecdh::diffie_hellman(ephemeral_sk.to_nonzero_scalar(), recip_pk.as_affine());

    // Pack 8-byte big-endian ratchet_step + 8-byte random salt into the 16-byte salt header slot.
    let mut salt = [0u8; 16];
    salt[0..8].copy_from_slice(&ratchet_step.to_be_bytes());
    OsRng.fill_bytes(&mut salt[8..16]);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut header_aad = [0u8; 162];
    header_aad[0] = PROTOCOL_VERSION_V2;
    header_aad[1..17].copy_from_slice(message_id.as_bytes());
    header_aad[17..33].copy_from_slice(sender_id.as_bytes());
    header_aad[33..49].copy_from_slice(recipient_id.as_bytes());
    header_aad[49..81].copy_from_slice(recipient_fingerprint);
    header_aad[81..146].copy_from_slice(ephemeral_pub.as_bytes());
    header_aad[146..162].copy_from_slice(&salt);

    // Derive per-message ratchet key via HKDF binding message metadata, sender identity key, and ratchet step.
    let mut info = Vec::with_capacity(18 + 48 + 65 + 8);
    info.extend_from_slice(b"chatw-v2-ratchet:");
    info.extend_from_slice(message_id.as_bytes());
    info.extend_from_slice(sender_id.as_bytes());
    info.extend_from_slice(recipient_id.as_bytes());
    info.extend_from_slice(sender_ecdsa_raw);
    info.extend_from_slice(&ratchet_step.to_be_bytes());

    let hk = Hkdf::<Sha256>::new(Some(&salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(&info, &mut aes_key)
        .map_err(|_| "hkdf_expand_failed")?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|_| "aes_init_failed");
    aes_key.zeroize();
    let cipher = cipher?;

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: &header_aad,
            },
        )
        .map_err(|_| "aes_encrypt_failed")?;

    let mut envelope = Vec::with_capacity(174 + ciphertext.len() + 64);
    envelope.extend_from_slice(&header_aad);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    let signing_key =
        SigningKey::from_slice(sender_ecdsa_secret).map_err(|_| "invalid_signing_key")?;
    let sig: Signature = signing_key.sign(&envelope);
    envelope.extend_from_slice(sig.to_bytes().as_slice());

    Ok(envelope)
}

pub fn encrypt_chat_message(
    message_id: &str,
    sender_id: &str,
    recipient_id: &str,
    sender_keys: &UserKeys,
    recipient_keys: &PeerKeys,
    text: &str,
) -> Result<(String, String), String> {
    let msg_uuid = Uuid::parse_str(message_id).map_err(|_| "invalid_message_id")?;
    let sender_uuid = Uuid::parse_str(sender_id).map_err(|_| "invalid_sender_id")?;
    let recip_uuid = Uuid::parse_str(recipient_id).map_err(|_| "invalid_recipient_id")?;
    let plaintext = text.as_bytes();
    let step = next_ratchet_step(sender_uuid, recip_uuid);

    let recipient_env = build_envelope_v2(
        msg_uuid,
        sender_uuid,
        recip_uuid,
        &recipient_keys.fingerprint,
        &recipient_keys.ecdh_raw,
        &sender_keys.ecdsa_secret,
        &sender_keys.ecdsa_raw,
        step,
        plaintext,
    )?;

    let sender_env = build_envelope_v2(
        msg_uuid,
        sender_uuid,
        sender_uuid,
        &sender_keys.fingerprint,
        &sender_keys.ecdh_raw,
        &sender_keys.ecdsa_secret,
        &sender_keys.ecdsa_raw,
        step,
        plaintext,
    )?;

    Ok((
        base64url_encode(&recipient_env),
        base64url_encode(&sender_env),
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecryptedMessage {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecryptError {
    WrongDeviceKey,
    SignatureMismatch,
    Corrupted(String),
}

impl DecryptError {
    pub fn user_message(&self) -> String {
        match self {
            DecryptError::WrongDeviceKey => {
                "Messages are encrypted using another device key".to_string()
            }
            DecryptError::SignatureMismatch => {
                "Unable to verify sender's signature".to_string()
            }
            DecryptError::Corrupted(_) => "Encrypted data is invalid".to_string(),
        }
    }
}

#[allow(deprecated)]
pub fn decrypt_chat_message_detailed(
    envelope_b64: &str,
    my_keys: &UserKeys,
    sender_ecdsa_raw: &[u8; 65],
) -> Result<DecryptedMessage, DecryptError> {
    let raw = base64url_decode(envelope_b64).map_err(DecryptError::Corrupted)?;
    if raw.len() < 255 {
        return Err(DecryptError::Corrupted("Envelope too short".into()));
    }
    let version = raw[0];
    if version != PROTOCOL_VERSION && version != PROTOCOL_VERSION_V2 {
        return Err(DecryptError::Corrupted("Unsupported protocol version".into()));
    }

    let message_id =
        Uuid::from_slice(&raw[1..17]).map_err(|_| DecryptError::Corrupted("invalid_msg_uuid".into()))?;
    let sender_id = Uuid::from_slice(&raw[17..33])
        .map_err(|_| DecryptError::Corrupted("invalid_sender_uuid".into()))?;
    let recipient_id = Uuid::from_slice(&raw[33..49])
        .map_err(|_| DecryptError::Corrupted("invalid_recip_uuid".into()))?;
    let recipient_fp = &raw[49..81];
    let aad = if version == PROTOCOL_VERSION_V2 {
        &raw[0..162]
    } else {
        &raw[0..81]
    };
    let ephemeral_pub_raw = &raw[81..146];
    let salt = &raw[146..162];
    let nonce_bytes = &raw[162..174];
    let sig_start = raw.len() - 64;
    let ciphertext = &raw[174..sig_start];
    let sig_bytes = &raw[sig_start..];
    let body = &raw[0..sig_start];

    // Check recipient fingerprint first so we distinguish old/other device key honestly.
    if !my_keys.matches_fingerprint(recipient_fp) {
        return Err(DecryptError::WrongDeviceKey);
    }

    // 1. Verify ECDSA signature before performing ECDH or AES decryption.
    let verifying_key = VerifyingKey::from_sec1_bytes(sender_ecdsa_raw)
        .map_err(|_| DecryptError::Corrupted("invalid_sender_ecdsa".into()))?;
    let signature = Signature::from_slice(sig_bytes)
        .map_err(|_| DecryptError::Corrupted("invalid_signature_format".into()))?;
    verifying_key
        .verify(body, &signature)
        .map_err(|_| DecryptError::SignatureMismatch)?;

    // 2. ECDH shared secret
    let ephem_pk = PublicKey::from_sec1_bytes(ephemeral_pub_raw)
        .map_err(|_| DecryptError::Corrupted("invalid_ephemeral_key".into()))?;
    let my_sk = SecretKey::from_slice(&my_keys.ecdh_secret)
        .map_err(|_| DecryptError::Corrupted("invalid_my_ecdh_secret".into()))?;
    let shared = p256::ecdh::diffie_hellman(my_sk.to_nonzero_scalar(), ephem_pk.as_affine());

    // 3. HKDF-SHA-256 (v2 authenticated ratchet derivation vs v1 legacy derivation)
    let hk = Hkdf::<Sha256>::new(Some(salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    if version == PROTOCOL_VERSION_V2 {
        let ratchet_step = u64::from_be_bytes([
            salt[0], salt[1], salt[2], salt[3], salt[4], salt[5], salt[6], salt[7],
        ]);
        let mut info = Vec::with_capacity(18 + 48 + 65 + 8);
        info.extend_from_slice(b"chatw-v2-ratchet:");
        info.extend_from_slice(message_id.as_bytes());
        info.extend_from_slice(sender_id.as_bytes());
        info.extend_from_slice(recipient_id.as_bytes());
        info.extend_from_slice(sender_ecdsa_raw);
        info.extend_from_slice(&ratchet_step.to_be_bytes());
        hk.expand(&info, &mut aes_key)
            .map_err(|_| DecryptError::Corrupted("hkdf_expand_failed".into()))?;
    } else {
        let info = format!("chatw-v1-msg:{message_id}");
        hk.expand(info.as_bytes(), &mut aes_key)
            .map_err(|_| DecryptError::Corrupted("hkdf_expand_failed".into()))?;
    }

    // 4. AES-256-GCM decrypt and immediately zeroize key buffer
    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|_| DecryptError::Corrupted("aes_init_failed".into()));
    aes_key.zeroize();
    let cipher = cipher?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| DecryptError::Corrupted("aes_decrypt_failed".into()))?;

    let text = String::from_utf8(plaintext)
        .map_err(|_| DecryptError::Corrupted("invalid_utf8".into()))?;
    let mid_str = message_id.to_string();
    record_seen_message_id(&mid_str);
    Ok(DecryptedMessage {
        message_id: mid_str,
        sender_id: sender_id.to_string(),
        recipient_id: recipient_id.to_string(),
        text,
    })
}

pub fn decrypt_chat_message(
    envelope_b64: &str,
    my_keys: &UserKeys,
    sender_ecdsa_raw: &[u8; 65],
) -> Result<DecryptedMessage, String> {
    decrypt_chat_message_detailed(envelope_b64, my_keys, sender_ecdsa_raw).map_err(|e| match e {
        DecryptError::WrongDeviceKey => "Wrong device key".into(),
        DecryptError::SignatureMismatch => "Signature verification failed".into(),
        DecryptError::Corrupted(s) => s,
    })
}

// -----------------------------------------------------------------------------
// Full Signal-Style Double Ratchet Session (DH Ratchet + Symmetric KDF Chain
// + Skipped-Message-Key Cache + Replay Protection + Zeroize-on-Consume)
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetHeader {
    pub dh_pub_b64: String,
    pub prev_chain_len: u32,
    pub msg_num: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetMessage {
    pub header: RatchetHeader,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

pub struct DoubleRatchetSession {
    root_key: [u8; 32],
    dh_send_sk: [u8; 32],
    dh_send_pub: [u8; 65],
    dh_recv_pub: Option<[u8; 65]>,
    send_chain_key: Option<[u8; 32]>,
    recv_chain_key: Option<[u8; 32]>,
    send_num: u32,
    recv_num: u32,
    prev_send_len: u32,
    skipped_keys: HashMap<(String, u32), [u8; 32]>,
    consumed_keys: HashMap<(String, u32), ()>,
}

impl Drop for DoubleRatchetSession {
    fn drop(&mut self) {
        self.root_key.zeroize();
        self.dh_send_sk.zeroize();
        if let Some(ref mut k) = self.send_chain_key {
            k.zeroize();
        }
        if let Some(ref mut k) = self.recv_chain_key {
            k.zeroize();
        }
        for (_, v) in self.skipped_keys.iter_mut() {
            v.zeroize();
        }
    }
}

fn kdf_root_step(root_key: &[u8; 32], dh_out: &[u8]) -> Result<([u8; 32], [u8; 32]), String> {
    let hk = Hkdf::<Sha256>::new(Some(root_key), dh_out);
    let mut okm = [0u8; 64];
    hk.expand(b"chatw-dr-rk-v1", &mut okm)
        .map_err(|_| "dr_hkdf_failed".to_string())?;
    let mut next_rk = [0u8; 32];
    let mut chain_key = [0u8; 32];
    next_rk.copy_from_slice(&okm[..32]);
    chain_key.copy_from_slice(&okm[32..]);
    okm.zeroize();
    Ok((next_rk, chain_key))
}

fn kdf_chain_step(chain_key: &[u8; 32]) -> Result<([u8; 32], [u8; 32]), String> {
    let mut mac_msg = <HmacSha256 as Mac>::new_from_slice(chain_key)
        .map_err(|_| "hmac_init_failed".to_string())?;
    mac_msg.update(&[0x01]);
    let mut msg_key = [0u8; 32];
    msg_key.copy_from_slice(&mac_msg.finalize().into_bytes());

    let mut mac_next = <HmacSha256 as Mac>::new_from_slice(chain_key)
        .map_err(|_| "hmac_init_failed".to_string())?;
    mac_next.update(&[0x02]);
    let mut next_ck = [0u8; 32];
    next_ck.copy_from_slice(&mac_next.finalize().into_bytes());
    Ok((next_ck, msg_key))
}

impl DoubleRatchetSession {
    pub fn init_initiator(shared_secret: [u8; 32], responder_dh_pub: [u8; 65]) -> Result<Self, String> {
        let sk = SecretKey::random(&mut OsRng);
        let mut dh_send_sk = [0u8; 32];
        dh_send_sk.copy_from_slice(&sk.to_bytes());
        let mut dh_send_pub = [0u8; 65];
        dh_send_pub.copy_from_slice(sk.public_key().to_encoded_point(false).as_bytes());

        let peer_pk = PublicKey::from_sec1_bytes(&responder_dh_pub)
            .map_err(|_| "invalid_responder_dh_pub".to_string())?;
        let dh_out = p256::ecdh::diffie_hellman(sk.to_nonzero_scalar(), peer_pk.as_affine());
        let (next_rk, send_ck) = kdf_root_step(&shared_secret, dh_out.raw_secret_bytes())?;

        Ok(Self {
            root_key: next_rk,
            dh_send_sk,
            dh_send_pub,
            dh_recv_pub: Some(responder_dh_pub),
            send_chain_key: Some(send_ck),
            recv_chain_key: None,
            send_num: 0,
            recv_num: 0,
            prev_send_len: 0,
            skipped_keys: HashMap::new(),
            consumed_keys: HashMap::new(),
        })
    }

    pub fn init_responder(
        shared_secret: [u8; 32],
        my_dh_sk: [u8; 32],
    ) -> Result<Self, String> {
        let sk = SecretKey::from_slice(&my_dh_sk).map_err(|_| "invalid_dh_sk".to_string())?;
        let mut dh_send_pub = [0u8; 65];
        dh_send_pub.copy_from_slice(sk.public_key().to_encoded_point(false).as_bytes());
        Ok(Self {
            root_key: shared_secret,
            dh_send_sk: my_dh_sk,
            dh_send_pub,
            dh_recv_pub: None,
            send_chain_key: None,
            recv_chain_key: None,
            send_num: 0,
            recv_num: 0,
            prev_send_len: 0,
            skipped_keys: HashMap::new(),
            consumed_keys: HashMap::new(),
        })
    }

    #[allow(deprecated)]
    pub fn ratchet_encrypt(&mut self, plaintext: &[u8], ad: &[u8]) -> Result<RatchetMessage, String> {
        let send_ck = self
            .send_chain_key
            .ok_or_else(|| "ratchet_send_chain_uninitialized".to_string())?;
        let (next_ck, mut msg_key) = kdf_chain_step(&send_ck)?;
        self.send_chain_key = Some(next_ck);

        let header = RatchetHeader {
            dh_pub_b64: base64url_encode(&self.dh_send_pub),
            prev_chain_len: self.prev_send_len,
            msg_num: self.send_num,
        };
        self.send_num = self.send_num.saturating_add(1);

        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let mut full_aad = ad.to_vec();
        full_aad.extend_from_slice(
            serde_json::to_string(&header)
                .unwrap_or_default()
                .as_bytes(),
        );

        let cipher = Aes256Gcm::new_from_slice(&msg_key).map_err(|_| "dr_aes_init".to_string());
        msg_key.zeroize();
        let cipher = cipher?;
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &full_aad,
                },
            )
            .map_err(|_| "dr_encrypt_failed".to_string())?;

        Ok(RatchetMessage {
            header,
            nonce_b64: base64url_encode(&nonce),
            ciphertext_b64: base64url_encode(&ct),
        })
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), String> {
        if self.recv_num.saturating_add(MAX_SKIPPED_RATCHET_KEYS as u32) < until {
            return Err("too_many_skipped_messages".into());
        }
        let Some(ref peer_dh) = self.dh_recv_pub else {
            return Ok(());
        };
        let peer_dh_b64 = base64url_encode(peer_dh);
        while self.recv_num < until {
            let ck = self
                .recv_chain_key
                .ok_or_else(|| "recv_chain_missing".to_string())?;
            let (next_ck, mk) = kdf_chain_step(&ck)?;
            self.recv_chain_key = Some(next_ck);
            if self.skipped_keys.len() >= MAX_SKIPPED_RATCHET_KEYS {
                return Err("skipped_key_cache_full".into());
            }
            self.skipped_keys.insert((peer_dh_b64.clone(), self.recv_num), mk);
            self.recv_num += 1;
        }
        Ok(())
    }

    fn dh_ratchet_step(&mut self, remote_dh_pub: [u8; 65]) -> Result<(), String> {
        self.prev_send_len = self.send_num;
        self.send_num = 0;
        self.recv_num = 0;
        self.dh_recv_pub = Some(remote_dh_pub);

        let remote_pk = PublicKey::from_sec1_bytes(&remote_dh_pub)
            .map_err(|_| "invalid_remote_dh_pub".to_string())?;
        let cur_sk = SecretKey::from_slice(&self.dh_send_sk)
            .map_err(|_| "invalid_local_dh_sk".to_string())?;
        let dh_recv = p256::ecdh::diffie_hellman(cur_sk.to_nonzero_scalar(), remote_pk.as_affine());
        let (rk1, recv_ck) = kdf_root_step(&self.root_key, dh_recv.raw_secret_bytes())?;
        self.root_key = rk1;
        self.recv_chain_key = Some(recv_ck);

        // Rotate our local DH sending ratchet key for post-compromise security!
        let new_sk = SecretKey::random(&mut OsRng);
        self.dh_send_sk.copy_from_slice(&new_sk.to_bytes());
        self.dh_send_pub
            .copy_from_slice(new_sk.public_key().to_encoded_point(false).as_bytes());
        let dh_send = p256::ecdh::diffie_hellman(new_sk.to_nonzero_scalar(), remote_pk.as_affine());
        let (rk2, send_ck) = kdf_root_step(&self.root_key, dh_send.raw_secret_bytes())?;
        self.root_key = rk2;
        self.send_chain_key = Some(send_ck);
        Ok(())
    }

    #[allow(deprecated)]
    pub fn ratchet_decrypt(&mut self, msg: &RatchetMessage, ad: &[u8]) -> Result<Vec<u8>, String> {
        let key_id = (msg.header.dh_pub_b64.clone(), msg.header.msg_num);
        if self.consumed_keys.contains_key(&key_id) {
            return Err("replay_detected".into());
        }

        let nonce = base64url_decode(&msg.nonce_b64)?;
        let ct = base64url_decode(&msg.ciphertext_b64)?;
        if nonce.len() != 12 {
            return Err("invalid_dr_nonce".into());
        }
        let mut full_aad = ad.to_vec();
        full_aad.extend_from_slice(
            serde_json::to_string(&msg.header)
                .unwrap_or_default()
                .as_bytes(),
        );

        // 1. Check skipped-message-key cache for out-of-order delivery.
        if let Some(mut mk) = self.skipped_keys.remove(&key_id) {
            let cipher = Aes256Gcm::new_from_slice(&mk).map_err(|_| "dr_aes_init".to_string());
            mk.zeroize();
            let pt = cipher?
                .decrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &ct,
                        aad: &full_aad,
                    },
                )
                .map_err(|_| "dr_decrypt_failed".to_string())?;
            self.consumed_keys.insert(key_id, ());
            return Ok(pt);
        }

        let remote_dh_vec = base64url_decode(&msg.header.dh_pub_b64)?;
        if remote_dh_vec.len() != 65 {
            return Err("invalid_dr_dh_pub".into());
        }
        let mut remote_dh = [0u8; 65];
        remote_dh.copy_from_slice(&remote_dh_vec);

        // 2. If remote rotated its DH ratchet key, advance previous chain then step DH ratchet.
        if self.dh_recv_pub.as_ref() != Some(&remote_dh) {
            self.skip_message_keys(msg.header.prev_chain_len)?;
            self.dh_ratchet_step(remote_dh)?;
        }

        if msg.header.msg_num < self.recv_num {
            return Err("replay_or_stale_message".into());
        }
        self.skip_message_keys(msg.header.msg_num)?;

        let ck = self
            .recv_chain_key
            .ok_or_else(|| "recv_chain_missing".to_string())?;
        let (next_ck, mut mk) = kdf_chain_step(&ck)?;
        self.recv_chain_key = Some(next_ck);
        self.recv_num += 1;

        let cipher = Aes256Gcm::new_from_slice(&mk).map_err(|_| "dr_aes_init".to_string());
        mk.zeroize();
        let pt = cipher?
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &ct,
                    aad: &full_aad,
                },
            )
            .map_err(|_| "dr_decrypt_failed".to_string())?;
        self.consumed_keys.insert(key_id, ());
        Ok(pt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_encrypt_decrypt_roundtrip_and_tamper_detection() {
        let alice = UserKeys::generate();
        let bob = UserKeys::generate();

        let bob_peer = get_or_verify_peer_keys(
            "bob-test",
            &bob.ecdh_raw_b64,
            &bob.ecdsa_raw_b64,
            &bob.fingerprint_b64,
        )
        .expect("verify bob peer keys");

        let msg_id = Uuid::new_v4().to_string();
        let alice_id = Uuid::new_v4().to_string();
        let bob_id = Uuid::new_v4().to_string();
        let text = "Hello from Dioxus E2EE v2! 🔒";

        let (recip_env, sender_env) =
            encrypt_chat_message(&msg_id, &alice_id, &bob_id, &alice, &bob_peer, text)
                .expect("encrypt message");

        // Verify v2 protocol version byte
        let raw_recip = base64url_decode(&recip_env).unwrap();
        assert_eq!(raw_recip[0], PROTOCOL_VERSION_V2);

        // Bob decrypts recipient envelope using Alice's ECDSA public key
        let dec_bob =
            decrypt_chat_message(&recip_env, &bob, &alice.ecdsa_raw).expect("bob decrypt");
        assert_eq!(dec_bob.text, text);
        assert_eq!(dec_bob.message_id, msg_id);
        assert_eq!(dec_bob.sender_id, alice_id);
        assert_eq!(dec_bob.recipient_id, bob_id);

        // Alice decrypts sender self-envelope
        let dec_alice =
            decrypt_chat_message(&sender_env, &alice, &alice.ecdsa_raw).expect("alice decrypt");
        assert_eq!(dec_alice.text, text);

        // Tampered envelope must fail signature verification
        let mut tampered = base64url_decode(&recip_env).unwrap();
        tampered[180] ^= 0xff;
        let tampered_b64 = base64url_encode(&tampered);
        assert!(decrypt_chat_message(&tampered_b64, &bob, &alice.ecdsa_raw).is_err());

        // Wrong sender key must fail with SignatureMismatch
        assert_eq!(
            decrypt_chat_message_detailed(&recip_env, &bob, &bob.ecdsa_raw),
            Err(DecryptError::SignatureMismatch)
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_e2ee_v1_backward_compatible_decryption() {
        let alice = UserKeys::generate();
        let bob = UserKeys::generate();
        let msg_uuid = Uuid::new_v4();
        let alice_uuid = Uuid::new_v4();
        let bob_uuid = Uuid::new_v4();

        // Construct a legacy v1 envelope manually
        let ephem_sk = SecretKey::random(&mut OsRng);
        let ephem_pub = ephem_sk.public_key().to_encoded_point(false);
        let recip_pk = PublicKey::from_sec1_bytes(&bob.ecdh_raw).unwrap();
        let shared = p256::ecdh::diffie_hellman(ephem_sk.to_nonzero_scalar(), recip_pk.as_affine());
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce);
        let hk = Hkdf::<Sha256>::new(Some(&salt), shared.raw_secret_bytes());
        let mut aes_key = [0u8; 32];
        hk.expand(format!("chatw-v1-msg:{msg_uuid}").as_bytes(), &mut aes_key).unwrap();
        let mut aad = [0u8; 81];
        aad[0] = PROTOCOL_VERSION;
        aad[1..17].copy_from_slice(msg_uuid.as_bytes());
        aad[17..33].copy_from_slice(alice_uuid.as_bytes());
        aad[33..49].copy_from_slice(bob_uuid.as_bytes());
        aad[49..81].copy_from_slice(&bob.fingerprint);
        #[allow(deprecated)]
        let ct = Aes256Gcm::new_from_slice(&aes_key)
            .unwrap()
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: b"legacy v1 message", aad: &aad })
            .unwrap();
        let mut env = Vec::new();
        env.extend_from_slice(&aad);
        env.extend_from_slice(ephem_pub.as_bytes());
        env.extend_from_slice(&salt);
        env.extend_from_slice(&nonce);
        env.extend_from_slice(&ct);
        let sig: Signature = SigningKey::from_slice(&alice.ecdsa_secret).unwrap().sign(&env);
        env.extend_from_slice(sig.to_bytes().as_slice());

        let dec = decrypt_chat_message(&base64url_encode(&env), &bob, &alice.ecdsa_raw)
            .expect("must decrypt historical v1 envelope");
        assert_eq!(dec.text, "legacy v1 message");
    }

    #[test]
    fn test_e2ee_passphrase_vault_and_device_sync_transfer() {
        let uid = format!("vault_u_{}", Uuid::new_v4());
        let keys = UserKeys::generate();

        // Upgrade to Argon2id passphrase vault
        enable_passphrase_vault(&uid, &keys, "SuperSecretPass#2026").expect("enable vault");
        assert!(is_passphrase_vault_enabled(&uid));

        // Locking wipes memory so get_user_keys returns None without passphrase
        lock_and_wipe_session_keys(Some(&uid));
        assert!(has_locked_passphrase_vault(&uid));
        assert!(get_user_keys(Some(&uid)).is_none());

        // Wrong passphrase must fail
        assert!(unlock_passphrase_vault(&uid, "wrong-password").is_err());

        // Correct passphrase unlocks exact original keys + root_key
        let unlocked = unlock_passphrase_vault(&uid, "SuperSecretPass#2026").expect("unlock");
        assert_eq!(unlocked.ecdh_secret, keys.ecdh_secret);
        assert_eq!(unlocked.root_key, keys.root_key);

        // Device-to-device E2EE sync key wrap & unwrap
        let dev_b_uid = format!("dev_b_{}", Uuid::new_v4());
        let dev_b_pub = get_or_create_device_transport_pubkey(&dev_b_uid).expect("transport pub");
        let req_id = Uuid::new_v4().to_string();
        let wrapped = wrap_sync_key_for_device(&unlocked, &dev_b_pub, &req_id).expect("wrap sync");
        let transferred =
            unwrap_sync_key_on_device(&dev_b_uid, &wrapped, &req_id, Some(&unlocked.ecdsa_raw_b64))
                .expect("unwrap sync");
        assert_eq!(transferred.ecdh_secret, unlocked.ecdh_secret);
        assert_eq!(transferred.root_key, unlocked.root_key);
    }

    #[test]
    fn test_tofu_key_change_detection_and_safety_number() {
        let my_uid = format!("me_{}", Uuid::new_v4());
        let peer_uid = format!("peer_{}", Uuid::new_v4());
        let me = UserKeys::generate();
        let peer1 = UserKeys::generate();
        let peer2 = UserKeys::generate();

        let p1 = get_or_verify_peer_keys_scoped(
            Some(&my_uid),
            &peer_uid,
            &peer1.ecdh_raw_b64,
            &peer1.ecdsa_raw_b64,
            &peer1.fingerprint_b64,
        )
        .unwrap();
        assert!(p1.key_changed_warning.is_none());

        // Safety number must be symmetric between both parties
        let me_as_peer = PeerKeys {
            ecdh_raw: me.ecdh_raw,
            ecdsa_raw: me.ecdsa_raw,
            fingerprint: me.fingerprint,
            identity_fingerprint: me.identity_fingerprint,
            fingerprint_b64: me.fingerprint_b64.clone(),
            identity_fingerprint_b64: me.identity_fingerprint_b64.clone(),
            key_changed_warning: None,
        };
        assert_eq!(
            compute_safety_number(&my_uid, &me, &peer_uid, &p1),
            compute_safety_number(&peer_uid, &peer1, &my_uid, &me_as_peer)
        );

        // Substituting peer's identity key must trigger key_changed_warning
        let p2 = get_or_verify_peer_keys_scoped(
            Some(&my_uid),
            &peer_uid,
            &peer2.ecdh_raw_b64,
            &peer2.ecdsa_raw_b64,
            &peer2.fingerprint_b64,
        )
        .unwrap();
        assert!(p2.key_changed_warning.is_some());
        assert!(get_peer_key_change_alert(Some(&my_uid), &peer_uid).is_some());

        acknowledge_peer_key_change(Some(&my_uid), &peer_uid);
        assert!(get_peer_key_change_alert(Some(&my_uid), &peer_uid).is_none());
    }

    #[test]
    fn test_double_ratchet_forward_secrecy_out_of_order_and_replay_protection() {
        let mut shared_secret = [0u8; 32];
        OsRng.fill_bytes(&mut shared_secret);
        let bob_dh_sk_obj = SecretKey::random(&mut OsRng);
        let mut bob_dh_sk = [0u8; 32];
        bob_dh_sk.copy_from_slice(&bob_dh_sk_obj.to_bytes());
        let mut bob_dh_pub = [0u8; 65];
        bob_dh_pub.copy_from_slice(bob_dh_sk_obj.public_key().to_encoded_point(false).as_bytes());

        let mut alice = DoubleRatchetSession::init_initiator(shared_secret, bob_dh_pub).unwrap();
        let mut bob = DoubleRatchetSession::init_responder(shared_secret, bob_dh_sk).unwrap();

        let m1 = alice.ratchet_encrypt(b"msg 1", b"ad").unwrap();
        let m2 = alice.ratchet_encrypt(b"msg 2", b"ad").unwrap();
        let m3 = alice.ratchet_encrypt(b"msg 3", b"ad").unwrap();

        // Out-of-order delivery: m3 arrives before m1 and m2
        assert_eq!(bob.ratchet_decrypt(&m3, b"ad").unwrap(), b"msg 3");
        assert_eq!(bob.ratchet_decrypt(&m1, b"ad").unwrap(), b"msg 1");
        assert_eq!(bob.ratchet_decrypt(&m2, b"ad").unwrap(), b"msg 2");

        // Replay of m1 MUST be rejected
        assert_eq!(bob.ratchet_decrypt(&m1, b"ad"), Err("replay_detected".into()));

        // Bob replies -> triggers DH ratchet step (post-compromise healing)
        let r1 = bob.ratchet_encrypt(b"reply 1", b"ad").unwrap();
        assert_eq!(alice.ratchet_decrypt(&r1, b"ad").unwrap(), b"reply 1");

        // Alice replies again -> second DH ratchet turn
        let m4 = alice.ratchet_encrypt(b"msg 4 after heal", b"ad").unwrap();
        assert_eq!(bob.ratchet_decrypt(&m4, b"ad").unwrap(), b"msg 4 after heal");
    }

    #[test]
    fn test_e2ee_wrong_device_key_and_backup_recovery() {
        let alice = UserKeys::generate();
        let bob_old = UserKeys::generate();
        let bob_new = UserKeys::generate();

        let bob_peer = get_or_verify_peer_keys(
            "bob-old-peer",
            &bob_old.ecdh_raw_b64,
            &bob_old.ecdsa_raw_b64,
            &bob_old.fingerprint_b64,
        )
        .unwrap();

        let (recip_env, _) = encrypt_chat_message(
            &Uuid::new_v4().to_string(),
            &Uuid::new_v4().to_string(),
            &Uuid::new_v4().to_string(),
            &alice,
            &bob_peer,
            "Secure messages need recovery",
        )
        .unwrap();

        assert_eq!(
            decrypt_chat_message_detailed(&recip_env, &bob_new, &alice.ecdsa_raw),
            Err(DecryptError::WrongDeviceKey)
        );

        let backup = export_user_keys_json(&bob_old);
        let restored = import_user_keys_json("bob-restore-test", &backup).expect("import keys");
        let dec = decrypt_chat_message_detailed(&recip_env, &restored, &alice.ecdsa_raw)
            .expect("decrypt with restored key");
        assert_eq!(dec.text, "Secure messages need recovery");
    }

    #[test]
    fn test_e2ee_cross_account_isolation() {
        let uid_a = format!("iso_a_{}", Uuid::new_v4());
        let uid_b = format!("iso_b_{}", Uuid::new_v4());
        let keys_a = create_and_store_user_keys(Some(&uid_a)).unwrap();
        assert_eq!(get_user_keys(Some(&uid_a)).unwrap().fingerprint, keys_a.fingerprint);
        assert!(get_user_keys(Some(&uid_b)).is_none());
    }

    #[test]
    fn test_e2ee_non_destructive_legacy_plaintext_migration() {
        let uid = format!("legacy_mig_{}", Uuid::new_v4());
        let slot = format!("chatw_e2ee_keys_{uid}");
        let legacy_keys = UserKeys::generate();
        let legacy_plaintext_json = serde_json::to_string(&StoredUserKeys {
            ecdh_secret_b64: base64url_encode(&legacy_keys.ecdh_secret),
            ecdsa_secret_b64: base64url_encode(&legacy_keys.ecdsa_secret),
            root_key_b64: None,
        })
        .unwrap();

        platform::storage_set(&slot, &legacy_plaintext_json);

        let loaded = get_user_keys(Some(&uid)).expect("load legacy keys without data loss");
        assert_eq!(loaded.fingerprint, legacy_keys.fingerprint);
        assert_eq!(loaded.ecdh_secret, legacy_keys.ecdh_secret);
        assert_eq!(loaded.ecdsa_secret, legacy_keys.ecdsa_secret);

        let after_raw = platform::storage_get(&slot).expect("migrated slot exists");
        assert!(
            after_raw.starts_with("enc:v1:"),
            "legacy plaintext key must be migrated to enc:v1: ciphertext, got: {after_raw}"
        );
        assert!(
            !after_raw.contains(&base64url_encode(&legacy_keys.ecdh_secret)),
            "migrated storage must not contain raw private scalar in plaintext"
        );
    }
}
