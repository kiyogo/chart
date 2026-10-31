pub mod keys;

use p256::{
    PublicKey,
    ecdsa::{Signature, VerifyingKey, signature::Verifier},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 174;
pub const SIGNATURE_LEN: usize = 64;
pub const MIN_CIPHERTEXT_LEN: usize = 17; // 1 byte UTF-8 + 16 byte AES-GCM tag
pub const MAX_CIPHERTEXT_LEN: usize = 8192;
pub const MIN_ENVELOPE_LEN: usize = HEADER_LEN + MIN_CIPHERTEXT_LEN + SIGNATURE_LEN; // 255
pub const MAX_ENVELOPE_LEN: usize = HEADER_LEN + MAX_CIPHERTEXT_LEN + SIGNATURE_LEN; // 8430

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

/// Compute 32-byte SHA-256 fingerprint of a public key (usually the 65-byte uncompressed ECDH key).
pub fn compute_fingerprint(public_key_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(public_key_bytes);
    hasher.finalize().into()
}

/// Validate that a byte slice is a valid uncompressed P-256 point (65 bytes, 0x04 prefix, on curve).
pub fn validate_p256_public_key(bytes: &[u8]) -> AppResult<()> {
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(AppError::BadRequest("invalid_public_key_format"));
    }
    PublicKey::from_sec1_bytes(bytes)
        .map_err(|_| AppError::BadRequest("invalid_public_key_point"))?;
    Ok(())
}

impl<'a> Envelope<'a> {
    pub fn parse(raw: &'a [u8]) -> AppResult<Self> {
        if raw.len() < MIN_ENVELOPE_LEN {
            return Err(AppError::BadRequest("envelope_too_short"));
        }
        if raw.len() > MAX_ENVELOPE_LEN {
            return Err(AppError::BadRequest("envelope_too_large"));
        }

        let version = raw[0];
        if version != PROTOCOL_VERSION {
            return Err(AppError::BadRequest("unsupported_protocol_version"));
        }

        let message_id = Uuid::from_slice(&raw[1..17])
            .map_err(|_| AppError::BadRequest("invalid_message_id"))?;
        let sender_id = Uuid::from_slice(&raw[17..33])
            .map_err(|_| AppError::BadRequest("invalid_sender_id"))?;
        let recipient_id = Uuid::from_slice(&raw[33..49])
            .map_err(|_| AppError::BadRequest("invalid_recipient_id"))?;

        let recipient_key_fingerprint: &'a [u8; 32] = raw[49..81]
            .try_into()
            .map_err(|_| AppError::BadRequest("invalid_fingerprint"))?;

        let ephemeral_public_key: &'a [u8; 65] = raw[81..146]
            .try_into()
            .map_err(|_| AppError::BadRequest("invalid_ephemeral_key"))?;

        // Validate that the ephemeral key is a valid point on P-256
        validate_p256_public_key(ephemeral_public_key)?;

        let salt: &'a [u8; 16] = raw[146..162]
            .try_into()
            .map_err(|_| AppError::BadRequest("invalid_salt"))?;

        let nonce: &'a [u8; 12] = raw[162..174]
            .try_into()
            .map_err(|_| AppError::BadRequest("invalid_nonce"))?;

        let sig_start = raw.len() - SIGNATURE_LEN;
        let ciphertext = &raw[174..sig_start];
        if ciphertext.len() < MIN_CIPHERTEXT_LEN || ciphertext.len() > MAX_CIPHERTEXT_LEN {
            return Err(AppError::BadRequest("invalid_ciphertext_length"));
        }

        let signature: &'a [u8; 64] = raw[sig_start..]
            .try_into()
            .map_err(|_| AppError::BadRequest("invalid_signature_length"))?;

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

    /// Bytes covered by the ECDSA signature: the entire envelope except the final 64-byte signature.
    pub fn signed_data(&self) -> &'a [u8] {
        &self.raw[..self.raw.len() - SIGNATURE_LEN]
    }

    /// AAD used for AES-GCM: binds version, message_id, sender_id, recipient_id, recipient_key_fingerprint.
    pub fn aad(&self) -> &'a [u8] {
        &self.raw[..81]
    }

    /// Verify signature using the sender's registered ECDSA public key (65 bytes uncompressed SEC1).
    pub fn verify_signature(&self, sender_ecdsa_pubkey: &[u8]) -> AppResult<()> {
        let verifying_key = VerifyingKey::from_sec1_bytes(sender_ecdsa_pubkey)
            .map_err(|_| AppError::BadRequest("invalid_sender_verifying_key"))?;

        let sig = Signature::from_bytes(self.signature.into())
            .map_err(|_| AppError::BadRequest("invalid_signature_format"))?;

        verifying_key
            .verify(self.signed_data(), &sig)
            .map_err(|_| AppError::BadRequest("signature_verification_failed"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::{
        ecdsa::{SigningKey, signature::Signer},
        elliptic_curve::{rand_core::OsRng, sec1::ToEncodedPoint},
    };

    #[test]
    fn test_envelope_construction_and_verification() {
        // Generate sender ECDSA key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let sender_ecdsa_pub = verifying_key.to_encoded_point(false);
        assert_eq!(sender_ecdsa_pub.as_bytes().len(), 65);

        // Generate recipient ECDH key
        let recipient_secret = p256::SecretKey::random(&mut OsRng);
        let recipient_ecdh_pub = recipient_secret.public_key().to_encoded_point(false);
        let fingerprint = compute_fingerprint(recipient_ecdh_pub.as_bytes());

        // Generate ephemeral key
        let ephemeral_secret = p256::SecretKey::random(&mut OsRng);
        let ephemeral_pub = ephemeral_secret.public_key().to_encoded_point(false);

        let msg_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();
        let salt = [42u8; 16];
        let nonce = [7u8; 12];
        let ciphertext = vec![0xaa; 32]; // dummy ciphertext with tag

        let mut raw = Vec::new();
        raw.push(PROTOCOL_VERSION);
        raw.extend_from_slice(msg_id.as_bytes());
        raw.extend_from_slice(sender_id.as_bytes());
        raw.extend_from_slice(recipient_id.as_bytes());
        raw.extend_from_slice(&fingerprint);
        raw.extend_from_slice(ephemeral_pub.as_bytes());
        raw.extend_from_slice(&salt);
        raw.extend_from_slice(&nonce);
        raw.extend_from_slice(&ciphertext);

        let sig: Signature = signing_key.sign(&raw);
        raw.extend_from_slice(sig.to_bytes().as_slice());

        let env = Envelope::parse(&raw).expect("parse envelope");
        assert_eq!(env.version, 1);
        assert_eq!(env.message_id, msg_id);
        assert_eq!(env.sender_id, sender_id);
        assert_eq!(env.recipient_id, recipient_id);
        assert_eq!(env.ciphertext, &ciphertext[..]);

        // Verification should succeed
        env.verify_signature(sender_ecdsa_pub.as_bytes())
            .expect("verify");

        // Tampering with any byte must fail verification
        let mut tampered = raw.clone();
        tampered[174] ^= 0x01; // flip ciphertext bit
        let env_tampered = Envelope::parse(&tampered).unwrap();
        assert!(
            env_tampered
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Tampering with sender_id must fail verification
        let mut tampered_sender = raw.clone();
        tampered_sender[20] ^= 0x01;
        let env_tampered_sender = Envelope::parse(&tampered_sender).unwrap();
        assert!(
            env_tampered_sender
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Tampering with message_id must fail verification
        let mut tampered_msg = raw.clone();
        tampered_msg[5] ^= 0x01;
        let env_tampered_msg = Envelope::parse(&tampered_msg).unwrap();
        assert!(
            env_tampered_msg
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Tampering with recipient_id must fail verification
        let mut tampered_recip = raw.clone();
        tampered_recip[35] ^= 0x01;
        let env_tampered_recip = Envelope::parse(&tampered_recip).unwrap();
        assert!(
            env_tampered_recip
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Tampering with fingerprint must fail verification
        let mut tampered_fp = raw.clone();
        tampered_fp[55] ^= 0x01;
        let env_tampered_fp = Envelope::parse(&tampered_fp).unwrap();
        assert!(
            env_tampered_fp
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Tampering with salt or nonce must fail verification
        let mut tampered_nonce = raw.clone();
        tampered_nonce[165] ^= 0x01;
        let env_tampered_nonce = Envelope::parse(&tampered_nonce).unwrap();
        assert!(
            env_tampered_nonce
                .verify_signature(sender_ecdsa_pub.as_bytes())
                .is_err()
        );

        // Wrong signer key must fail verification
        let other_signing_key = SigningKey::random(&mut OsRng);
        let other_verifying_key = other_signing_key.verifying_key();
        let other_pub = other_verifying_key.to_encoded_point(false);
        assert!(env.verify_signature(other_pub.as_bytes()).is_err());
    }

    #[test]
    fn test_public_key_point_validation() {
        // Valid point
        let key = p256::SecretKey::random(&mut OsRng);
        let pubkey = key.public_key().to_encoded_point(false);
        assert!(validate_p256_public_key(pubkey.as_bytes()).is_ok());

        // Invalid length
        assert!(validate_p256_public_key(&[0x04; 64]).is_err());
        assert!(validate_p256_public_key(&[0x04; 66]).is_err());

        // Compressed point (must be rejected - uncompressed 0x04 required)
        let compressed = key.public_key().to_encoded_point(true);
        assert!(validate_p256_public_key(compressed.as_bytes()).is_err());

        // Point not on curve
        let mut bogus = [0x04; 65];
        bogus[1..].copy_from_slice(&[0xff; 64]);
        assert!(validate_p256_public_key(&bogus).is_err());
    }
}
