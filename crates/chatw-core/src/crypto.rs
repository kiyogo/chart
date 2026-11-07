use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use p256::{
    PublicKey, SecretKey,
    elliptic_curve::sec1::ToEncodedPoint,
    ecdsa::{Signature, SigningKey, VerifyingKey, signature::Signer, signature::Verifier},
};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{CoreError, Result};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 174;
pub const SIGNATURE_LEN: usize = 64;
pub const MIN_CIPHERTEXT_LEN: usize = 17; // 1 byte UTF-8 + 16 byte AES-GCM tag
pub const MAX_CIPHERTEXT_LEN: usize = 8192;
pub const MIN_ENVELOPE_LEN: usize = HEADER_LEN + MIN_CIPHERTEXT_LEN + SIGNATURE_LEN; // 255
pub const MAX_ENVELOPE_LEN: usize = HEADER_LEN + MAX_CIPHERTEXT_LEN + SIGNATURE_LEN; // 8430

/// Compute 32-byte SHA-256 fingerprint of a public key.
pub fn compute_fingerprint(public_key_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(public_key_bytes);
    hasher.finalize().into()
}

/// Validate that a byte slice is a valid uncompressed P-256 point (65 bytes, 0x04 prefix, on curve).
pub fn validate_p256_public_key(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(CoreError::BadRequest("invalid_public_key_format"));
    }
    PublicKey::from_sec1_bytes(bytes)
        .map_err(|_| CoreError::BadRequest("invalid_public_key_point"))?;
    Ok(())
}

/// Parsed message envelope wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope<'a> {
    pub raw: &'a [u8],
    pub version: u8,
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub recipient_key_fingerprint: &'a [u8; 32],
    pub ephemeral_public_key: &'a [u8; 65],
    pub salt: &'a [u8; 16],
    pub nonce: &'a [u8; 12],
    pub ciphertext: &'a [u8],
    pub signature: &'a [u8; 64],
}

impl<'a> Envelope<'a> {
    pub fn parse(raw: &'a [u8]) -> Result<Self> {
        if raw.len() < MIN_ENVELOPE_LEN {
            return Err(CoreError::BadRequest("envelope_too_short"));
        }
        if raw.len() > MAX_ENVELOPE_LEN {
            return Err(CoreError::BadRequest("envelope_too_large"));
        }

        let version = raw[0];
        if version != PROTOCOL_VERSION {
            return Err(CoreError::BadRequest("unsupported_protocol_version"));
        }

        let message_id = Uuid::from_slice(&raw[1..17])
            .map_err(|_| CoreError::BadRequest("invalid_message_id"))?;
        let sender_id = Uuid::from_slice(&raw[17..33])
            .map_err(|_| CoreError::BadRequest("invalid_sender_id"))?;
        let recipient_id = Uuid::from_slice(&raw[33..49])
            .map_err(|_| CoreError::BadRequest("invalid_recipient_id"))?;

        let recipient_key_fingerprint: &'a [u8; 32] = raw[49..81]
            .try_into()
            .map_err(|_| CoreError::BadRequest("invalid_fingerprint"))?;

        let ephemeral_public_key: &'a [u8; 65] = raw[81..146]
            .try_into()
            .map_err(|_| CoreError::BadRequest("invalid_ephemeral_key"))?;

        validate_p256_public_key(ephemeral_public_key)?;

        let salt: &'a [u8; 16] = raw[146..162]
            .try_into()
            .map_err(|_| CoreError::BadRequest("invalid_salt"))?;

        let nonce: &'a [u8; 12] = raw[162..174]
            .try_into()
            .map_err(|_| CoreError::BadRequest("invalid_nonce"))?;

        let sig_start = raw.len() - SIGNATURE_LEN;
        let ciphertext = &raw[174..sig_start];
        if ciphertext.len() < MIN_CIPHERTEXT_LEN || ciphertext.len() > MAX_CIPHERTEXT_LEN {
            return Err(CoreError::BadRequest("invalid_ciphertext_length"));
        }

        let signature: &'a [u8; 64] = raw[sig_start..]
            .try_into()
            .map_err(|_| CoreError::BadRequest("invalid_signature_length"))?;

        Ok(Self {
            raw,
            version,
            message_id,
            sender_id,
            recipient_id,
            recipient_key_fingerprint,
            ephemeral_public_key,
            salt,
            nonce,
            ciphertext,
            signature,
        })
    }

    pub fn signed_payload(&self) -> &'a [u8] {
        let sig_start = self.raw.len() - SIGNATURE_LEN;
        &self.raw[..sig_start]
    }

    pub fn aad(&self) -> &'a [u8] {
        &self.raw[..81]
    }

    pub fn verify_signature(&self, sender_ecdsa_pub_bytes: &[u8; 65]) -> Result<()> {
        let vk = VerifyingKey::from_sec1_bytes(sender_ecdsa_pub_bytes)
            .map_err(|_| CoreError::BadRequest("invalid_verifying_key"))?;
        let sig = Signature::from_bytes(self.signature.into())
            .map_err(|_| CoreError::InvalidSignature)?;
        vk.verify(self.signed_payload(), &sig)
            .map_err(|_| CoreError::InvalidSignature)?;
        Ok(())
    }
}

/// Client cryptographic keys for E2EE messaging.
#[derive(Clone)]
pub struct ClientKeys {
    pub ecdh_secret: SecretKey,
    pub ecdh_pub_bytes: [u8; 65],
    pub fingerprint: [u8; 32],
    pub ecdsa_signing: SigningKey,
    pub ecdsa_pub_bytes: [u8; 65],
}

impl ClientKeys {
    /// Generate a brand new random keypair for ECDH and ECDSA P-256.
    pub fn generate() -> Self {
        let ecdh_secret = SecretKey::random(&mut OsRng);
        let ecdh_pub = ecdh_secret.public_key().to_encoded_point(false);
        let mut ecdh_pub_bytes = [0u8; 65];
        ecdh_pub_bytes.copy_from_slice(ecdh_pub.as_bytes());

        let fingerprint = compute_fingerprint(&ecdh_pub_bytes);

        let ecdsa_signing = SigningKey::random(&mut OsRng);
        let ecdsa_verifying = ecdsa_signing.verifying_key();
        let ecdsa_pub = ecdsa_verifying.to_encoded_point(false);
        let mut ecdsa_pub_bytes = [0u8; 65];
        ecdsa_pub_bytes.copy_from_slice(ecdsa_pub.as_bytes());

        Self {
            ecdh_secret,
            ecdh_pub_bytes,
            fingerprint,
            ecdsa_signing,
            ecdsa_pub_bytes,
        }
    }

    /// Export keys as hex strings for local storage.
    pub fn export_raw(&self) -> (String, String) {
        (
            hex::encode(self.ecdh_secret.to_bytes()),
            hex::encode(self.ecdsa_signing.to_bytes()),
        )
    }

    /// Import keys from hex strings.
    pub fn import_raw(ecdh_hex: &str, ecdsa_hex: &str) -> Result<Self> {
        let ecdh_bytes = hex::decode(ecdh_hex)
            .map_err(|_| CoreError::BadRequest("invalid_ecdh_hex"))?;
        let ecdsa_bytes = hex::decode(ecdsa_hex)
            .map_err(|_| CoreError::BadRequest("invalid_ecdsa_hex"))?;

        let ecdh_secret = SecretKey::from_slice(&ecdh_bytes)
            .map_err(|_| CoreError::BadRequest("invalid_ecdh_secret"))?;
        let ecdh_pub = ecdh_secret.public_key().to_encoded_point(false);
        let mut ecdh_pub_bytes = [0u8; 65];
        ecdh_pub_bytes.copy_from_slice(ecdh_pub.as_bytes());
        let fingerprint = compute_fingerprint(&ecdh_pub_bytes);

        let ecdsa_signing = SigningKey::from_slice(&ecdsa_bytes)
            .map_err(|_| CoreError::BadRequest("invalid_ecdsa_signing"))?;
        let ecdsa_verifying = ecdsa_signing.verifying_key();
        let ecdsa_pub = ecdsa_verifying.to_encoded_point(false);
        let mut ecdsa_pub_bytes = [0u8; 65];
        ecdsa_pub_bytes.copy_from_slice(ecdsa_pub.as_bytes());

        Ok(Self {
            ecdh_secret,
            ecdh_pub_bytes,
            fingerprint,
            ecdsa_signing,
            ecdsa_pub_bytes,
        })
    }

    /// Base64 (URL-safe unpadded) representations for server registration.
    pub fn ecdh_pub_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.ecdh_pub_bytes)
    }

    pub fn ecdsa_pub_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.ecdsa_pub_bytes)
    }

    pub fn fingerprint_hex(&self) -> String {
        hex::encode(self.fingerprint)
    }
}

/// Encrypt a plaintext string into a full signed E2EE envelope.
pub fn encrypt_message(
    sender_keys: &ClientKeys,
    recipient_ecdh_pub_bytes: &[u8; 65],
    recipient_fingerprint: &[u8; 32],
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    plaintext: &str,
) -> Result<Vec<u8>> {
    let ephemeral_secret = SecretKey::random(&mut OsRng);
    let ephemeral_pub = ephemeral_secret.public_key().to_encoded_point(false);
    let recip_point = PublicKey::from_sec1_bytes(recipient_ecdh_pub_bytes)
        .map_err(|_| CoreError::BadRequest("invalid_recipient_ecdh_key"))?;
    let shared = p256::ecdh::diffie_hellman(
        ephemeral_secret.to_nonzero_scalar(),
        recip_point.as_affine(),
    );

    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand_core::RngCore::fill_bytes(&mut OsRng, &mut salt);
    rand_core::RngCore::fill_bytes(&mut OsRng, &mut nonce_bytes);

    let info = format!("chatw-v1-msg:{message_id}");
    let hk = Hkdf::<Sha256>::new(Some(&salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key)
        .map_err(|_| CoreError::KeyError("hkdf expand failed".into()))?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|_| CoreError::KeyError("aes init failed".into()))?;

    let mut aad = Vec::with_capacity(81);
    aad.push(PROTOCOL_VERSION);
    aad.extend_from_slice(message_id.as_bytes());
    aad.extend_from_slice(sender_id.as_bytes());
    aad.extend_from_slice(recipient_id.as_bytes());
    aad.extend_from_slice(recipient_fingerprint);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext.as_bytes(),
                aad: &aad,
            },
        )
        .map_err(|_| CoreError::BadRequest("encryption_failed"))?;

    let mut envelope = Vec::with_capacity(HEADER_LEN + ciphertext.len() + SIGNATURE_LEN);
    envelope.extend_from_slice(&aad);
    envelope.extend_from_slice(ephemeral_pub.as_bytes());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    let sig: Signature = sender_keys.ecdsa_signing.sign(&envelope);
    envelope.extend_from_slice(sig.to_bytes().as_slice());

    Ok(envelope)
}

/// Decrypt an incoming E2EE envelope and verify sender signature.
pub fn decrypt_message(
    recipient_keys: &ClientKeys,
    sender_ecdsa_pub_bytes: &[u8; 65],
    envelope_raw: &[u8],
) -> Result<String> {
    let env = Envelope::parse(envelope_raw)?;
    env.verify_signature(sender_ecdsa_pub_bytes)?;

    let ephem_point = PublicKey::from_sec1_bytes(env.ephemeral_public_key)
        .map_err(|_| CoreError::BadRequest("invalid_ephemeral_key"))?;
    let shared = p256::ecdh::diffie_hellman(
        recipient_keys.ecdh_secret.to_nonzero_scalar(),
        ephem_point.as_affine(),
    );

    let info = format!("chatw-v1-msg:{}", env.message_id);
    let hk = Hkdf::<Sha256>::new(Some(env.salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key)
        .map_err(|_| CoreError::KeyError("hkdf expand failed".into()))?;

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|_| CoreError::KeyError("aes init failed".into()))?;

    let plaintext_bytes = cipher
        .decrypt(
            Nonce::from_slice(env.nonce),
            Payload {
                msg: env.ciphertext,
                aad: env.aad(),
            },
        )
        .map_err(|_| CoreError::DecryptionFailed)?;

    String::from_utf8(plaintext_bytes)
        .map_err(|_| CoreError::BadRequest("invalid_utf8_plaintext"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_encrypt_decrypt_roundtrip() {
        let alice = ClientKeys::generate();
        let bob = ClientKeys::generate();

        let message_id = Uuid::new_v4();
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();
        let original_text = "Xin chào Bob! Đây là tin nhắn bảo mật E2EE thuần Rust 100%!";

        let envelope = encrypt_message(
            &alice,
            &bob.ecdh_pub_bytes,
            &bob.fingerprint,
            message_id,
            alice_id,
            bob_id,
            original_text,
        )
        .expect("encryption should succeed");

        let decrypted = decrypt_message(&bob, &alice.ecdsa_pub_bytes, &envelope)
            .expect("decryption should succeed");

        assert_eq!(decrypted, original_text);
    }

    #[test]
    fn test_tampered_envelope_rejected() {
        let alice = ClientKeys::generate();
        let bob = ClientKeys::generate();

        let mut envelope = encrypt_message(
            &alice,
            &bob.ecdh_pub_bytes,
            &bob.fingerprint,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Secret",
        )
        .unwrap();

        // Tamper with one byte in the ciphertext
        let tamper_idx = 180;
        envelope[tamper_idx] ^= 0xFF;

        let res = decrypt_message(&bob, &alice.ecdsa_pub_bytes, &envelope);
        assert!(res.is_err(), "Tampered message must be rejected by signature or MAC");
    }

    #[test]
    fn test_key_export_import_roundtrip() {
        let original = ClientKeys::generate();
        let (ecdh_hex, ecdsa_hex) = original.export_raw();

        let restored = ClientKeys::import_raw(&ecdh_hex, &ecdsa_hex).expect("import should succeed");
        assert_eq!(original.ecdh_pub_bytes, restored.ecdh_pub_bytes);
        assert_eq!(original.ecdsa_pub_bytes, restored.ecdsa_pub_bytes);
        assert_eq!(original.fingerprint, restored.fingerprint);
    }
}
