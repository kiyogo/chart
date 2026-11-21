use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
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

use crate::platform;

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Clone, Debug)]
pub struct UserKeys {
    pub ecdh_secret: [u8; 32],
    pub ecdsa_secret: [u8; 32],
    pub ecdh_raw: [u8; 65],
    pub ecdsa_raw: [u8; 65],
    pub fingerprint: [u8; 32],
    pub ecdh_raw_b64: String,
    pub ecdsa_raw_b64: String,
    pub fingerprint_b64: String,
}

impl PartialEq for UserKeys {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
    }
}

#[derive(Clone, Debug)]
pub struct PeerKeys {
    pub ecdh_raw: [u8; 65],
    pub ecdsa_raw: [u8; 65],
    pub fingerprint: [u8; 32],
    pub fingerprint_b64: String,
}

#[derive(Serialize, Deserialize)]
struct StoredUserKeys {
    ecdh_secret_b64: String,
    ecdsa_secret_b64: String,
}

#[derive(Serialize, Deserialize)]
struct StoredPeerKeys {
    ecdh_b64: String,
    ecdsa_b64: String,
    fingerprint_b64: String,
}

pub fn base64url_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn base64url_decode(s: &str) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| "invalid_base64url".to_string())
}

pub fn compute_fingerprint(ecdh_raw: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ecdh_raw);
    hasher.finalize().into()
}

impl UserKeys {
    pub fn generate() -> Self {
        let ecdh_sk = SecretKey::random(&mut OsRng);
        let ecdsa_sk = SigningKey::random(&mut OsRng);
        let mut ecdh_secret = [0u8; 32];
        ecdh_secret.copy_from_slice(&ecdh_sk.to_bytes());
        let mut ecdsa_secret = [0u8; 32];
        ecdsa_secret.copy_from_slice(&ecdsa_sk.to_bytes());
        Self::from_secrets(ecdh_secret, ecdsa_secret).expect("fresh keys are valid")
    }

    pub fn from_secrets(ecdh_secret: [u8; 32], ecdsa_secret: [u8; 32]) -> Result<Self, String> {
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
        let ecdh_raw_b64 = base64url_encode(&ecdh_raw);
        let ecdsa_raw_b64 = base64url_encode(&ecdsa_raw);
        let fingerprint_b64 = base64url_encode(&fingerprint);

        Ok(Self {
            ecdh_secret,
            ecdsa_secret,
            ecdh_raw,
            ecdsa_raw,
            fingerprint,
            ecdh_raw_b64,
            ecdsa_raw_b64,
            fingerprint_b64,
        })
    }
}

pub fn get_user_keys(user_id: Option<&str>) -> Option<UserKeys> {
    let primary_key = match user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_keys_{uid}"),
        _ => "chatw_e2ee_keys_current".to_string(),
    };
    // Never fall back to chatw_e2ee_keys_current when user_id is specified;
    // doing so leaks Account A's private key into Account B on the same browser.
    let raw = platform::storage_get(&primary_key)?;
    if raw.trim().is_empty() {
        return None;
    }
    let was_legacy_plaintext = !raw.trim_start().starts_with("enc:v1:");
    let keys = parse_stored_user_keys(&raw, &primary_key).ok()?;
    if was_legacy_plaintext {
        // Transparent non-destructive migration: upgrade legacy plaintext keys to AES-256-GCM wrapped format in-place
        store_user_keys(user_id, &keys);
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
            platform::storage_set("__chatw_device_kwrap_v1", &encoded);
            encoded
        }
    };
    let seed_bytes = base64url_decode(&seed_b64)?;
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-local-keywrap-v1"), &seed_bytes);
    let mut kek = [0u8; 32];
    hk.expand(format!("slot:{storage_slot}").as_bytes(), &mut kek)
        .map_err(|_| "hkdf_wrap_failed".to_string())?;
    Ok(kek)
}

#[allow(deprecated)]
fn encrypt_stored_payload(storage_slot: &str, plaintext_json: &str) -> Result<String, String> {
    let kek = derive_local_wrapping_key(storage_slot)?;
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string())?;
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
    let kek = derive_local_wrapping_key(storage_slot)?;
    let cipher = Aes256Gcm::new_from_slice(&kek).map_err(|_| "aes_wrap_init_failed".to_string())?;
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

fn parse_stored_user_keys(raw: &str, storage_slot: &str) -> Result<UserKeys, String> {
    let trimmed = raw.trim();
    let json_payload = if trimmed.starts_with("enc:v1:") {
        decrypt_stored_payload(storage_slot, trimmed)?
    } else {
        trimmed.to_string()
    };
    let stored: StoredUserKeys =
        serde_json::from_str(&json_payload).map_err(|_| "invalid_key_json")?;
    let ecdh_vec = base64url_decode(&stored.ecdh_secret_b64)?;
    let ecdsa_vec = base64url_decode(&stored.ecdsa_secret_b64)?;
    if ecdh_vec.len() != 32 || ecdsa_vec.len() != 32 {
        return Err("invalid_key_length".into());
    }
    let mut ecdh_secret = [0u8; 32];
    ecdh_secret.copy_from_slice(&ecdh_vec);
    let mut ecdsa_secret = [0u8; 32];
    ecdsa_secret.copy_from_slice(&ecdsa_vec);
    UserKeys::from_secrets(ecdh_secret, ecdsa_secret)
}

pub fn create_and_store_user_keys(user_id: Option<&str>) -> UserKeys {
    let keys = UserKeys::generate();
    store_user_keys(user_id, &keys);
    keys
}

pub fn store_user_keys(user_id: Option<&str>, keys: &UserKeys) {
    let stored = StoredUserKeys {
        ecdh_secret_b64: base64url_encode(&keys.ecdh_secret),
        ecdsa_secret_b64: base64url_encode(&keys.ecdsa_secret),
    };
    let slot = match user_id {
        Some(uid) if !uid.is_empty() => format!("chatw_e2ee_keys_{uid}"),
        _ => "chatw_e2ee_keys_current".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&stored) {
        let to_store = encrypt_stored_payload(&slot, &json).unwrap_or(json);
        platform::storage_set(&slot, &to_store);
    }
}

pub fn export_user_keys_json(keys: &UserKeys) -> String {
    let stored = StoredUserKeys {
        ecdh_secret_b64: base64url_encode(&keys.ecdh_secret),
        ecdsa_secret_b64: base64url_encode(&keys.ecdsa_secret),
    };
    serde_json::to_string(&stored).unwrap_or_default()
}

pub fn import_user_keys_json(user_id: &str, raw_json: &str) -> Result<UserKeys, String> {
    let slot = format!("chatw_e2ee_keys_{user_id}");
    let keys = parse_stored_user_keys(raw_json.trim(), &slot)?;
    store_user_keys(Some(user_id), &keys);
    Ok(keys)
}

pub fn clear_legacy_active_key() {
    platform::storage_set("chatw_e2ee_keys_current", "");
}

pub fn get_or_verify_peer_keys(
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

    // Verify fingerprint = SHA-256(ecdh_raw)
    let computed_fp = compute_fingerprint(&ecdh_raw);
    if computed_fp != fingerprint {
        return Err("Invalid peer key fingerprint".into());
    }

    // TOFU peer storage (matches web/frontend/src/crypto/e2ee.ts behavior)
    let peer_key = format!("chatw_e2ee_peer_{peer_id}");
    let serialized = serde_json::to_string(&StoredPeerKeys {
        ecdh_b64: ecdh_raw_b64.to_string(),
        ecdsa_b64: ecdsa_raw_b64.to_string(),
        fingerprint_b64: fingerprint_b64.to_string(),
    })
    .ok();

    if let Some(json) = serialized {
        platform::storage_set(&peer_key, &json);
    }

    Ok(PeerKeys {
        ecdh_raw,
        ecdsa_raw,
        fingerprint,
        fingerprint_b64: fingerprint_b64.to_string(),
    })
}

#[allow(deprecated)]
fn build_envelope(
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    recipient_fingerprint: &[u8; 32],
    recipient_ecdh_raw: &[u8; 65],
    sender_ecdsa_secret: &[u8; 32],
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let ephemeral_sk = SecretKey::random(&mut OsRng);
    let ephemeral_pub = ephemeral_sk.public_key().to_encoded_point(false);

    let recip_pk =
        PublicKey::from_sec1_bytes(recipient_ecdh_raw).map_err(|_| "invalid_recipient_ecdh")?;
    let shared =
        p256::ecdh::diffie_hellman(ephemeral_sk.to_nonzero_scalar(), recip_pk.as_affine());

    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    let info = format!("chatw-v1-msg:{message_id}");
    let hk = Hkdf::<Sha256>::new(Some(&salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key)
        .map_err(|_| "hkdf_expand_failed")?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|_| "aes_init_failed")?;

    let mut aad = [0u8; 81];
    aad[0] = PROTOCOL_VERSION;
    aad[1..17].copy_from_slice(message_id.as_bytes());
    aad[17..33].copy_from_slice(sender_id.as_bytes());
    aad[33..49].copy_from_slice(recipient_id.as_bytes());
    aad[49..81].copy_from_slice(recipient_fingerprint);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| "aes_encrypt_failed")?;

    let mut envelope = Vec::with_capacity(174 + ciphertext.len() + 64);
    envelope.extend_from_slice(&aad);
    envelope.extend_from_slice(ephemeral_pub.as_bytes());
    envelope.extend_from_slice(&salt);
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

    let recipient_env = build_envelope(
        msg_uuid,
        sender_uuid,
        recip_uuid,
        &recipient_keys.fingerprint,
        &recipient_keys.ecdh_raw,
        &sender_keys.ecdsa_secret,
        plaintext,
    )?;

    let sender_env = build_envelope(
        msg_uuid,
        sender_uuid,
        sender_uuid,
        &sender_keys.fingerprint,
        &sender_keys.ecdh_raw,
        &sender_keys.ecdsa_secret,
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
                "Tin nhắn được mã hóa bằng khóa thiết bị khác".to_string()
            }
            DecryptError::SignatureMismatch => {
                "Không xác minh được chữ ký người gửi".to_string()
            }
            DecryptError::Corrupted(_) => "Dữ liệu mã hóa không hợp lệ".to_string(),
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
    if raw[0] != PROTOCOL_VERSION {
        return Err(DecryptError::Corrupted("Unsupported protocol version".into()));
    }

    let message_id =
        Uuid::from_slice(&raw[1..17]).map_err(|_| DecryptError::Corrupted("invalid_msg_uuid".into()))?;
    let sender_id = Uuid::from_slice(&raw[17..33])
        .map_err(|_| DecryptError::Corrupted("invalid_sender_uuid".into()))?;
    let recipient_id = Uuid::from_slice(&raw[33..49])
        .map_err(|_| DecryptError::Corrupted("invalid_recip_uuid".into()))?;
    let recipient_fp = &raw[49..81];
    let aad = &raw[0..81];
    let ephemeral_pub_raw = &raw[81..146];
    let salt = &raw[146..162];
    let nonce_bytes = &raw[162..174];
    let sig_start = raw.len() - 64;
    let ciphertext = &raw[174..sig_start];
    let sig_bytes = &raw[sig_start..];
    let body = &raw[0..sig_start];

    // Check recipient fingerprint first so we distinguish old/other device key honestly
    if recipient_fp != my_keys.fingerprint {
        return Err(DecryptError::WrongDeviceKey);
    }

    // 1. Verify ECDSA signature
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

    // 3. HKDF-SHA-256
    let info = format!("chatw-v1-msg:{message_id}");
    let hk = Hkdf::<Sha256>::new(Some(salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key)
        .map_err(|_| DecryptError::Corrupted("hkdf_expand_failed".into()))?;

    // 4. AES-256-GCM decrypt
    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|_| DecryptError::Corrupted("aes_init_failed".into()))?;
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
    Ok(DecryptedMessage {
        message_id: message_id.to_string(),
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
        let text = "Xin chào từ Dioxus E2EE! 🔒";

        let (recip_env, sender_env) =
            encrypt_chat_message(&msg_id, &alice_id, &bob_id, &alice, &bob_peer, text)
                .expect("encrypt message");

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
            "Tin nhắn bảo mật cần khôi phục",
        )
        .unwrap();

        // New device key cannot decrypt envelope meant for old device key
        assert_eq!(
            decrypt_chat_message_detailed(&recip_env, &bob_new, &alice.ecdsa_raw),
            Err(DecryptError::WrongDeviceKey)
        );

        // Exporting old key and importing it restores decryption cleanly
        let backup = export_user_keys_json(&bob_old);
        let restored = import_user_keys_json("bob-restore-test", &backup).expect("import keys");
        let dec = decrypt_chat_message_detailed(&recip_env, &restored, &alice.ecdsa_raw)
            .expect("decrypt with restored key");
        assert_eq!(dec.text, "Tin nhắn bảo mật cần khôi phục");
    }

    #[test]
    fn test_e2ee_cross_account_isolation() {
        let uid_a = format!("iso_a_{}", Uuid::new_v4());
        let uid_b = format!("iso_b_{}", Uuid::new_v4());
        let keys_a = create_and_store_user_keys(Some(&uid_a));
        assert_eq!(get_user_keys(Some(&uid_a)).unwrap().fingerprint, keys_a.fingerprint);
        // Account B must NEVER inherit Account A's keys
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
        })
        .unwrap();

        // Simulate legacy plaintext key stored before Spec 051
        platform::storage_set(&slot, &legacy_plaintext_json);

        // First read must transparently decrypt/parse AND migrate in-place to enc:v1:...
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

        // Subsequent reads continue to return the exact same keypair
        let loaded_again = get_user_keys(Some(&uid)).expect("reload migrated keys");
        assert_eq!(loaded_again.fingerprint, legacy_keys.fingerprint);
    }
}
