#![allow(deprecated)]
//! Client-side encryption for attachments (images in E2EE chats).
//!
//! Each file gets a fresh random AES-256-GCM key and nonce. Only the ciphertext is
//! uploaded; the key and nonce travel inside the end-to-end encrypted message.

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};

pub struct SealedBlob {
    pub cipher: Vec<u8>,
    pub key_b64: String,
    pub nonce_b64: String,
}

pub fn seal(bytes: &[u8]) -> Result<SealedBlob, String> {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut key);
    OsRng.fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| "bad_key".to_string())?
        .encrypt(Nonce::from_slice(&nonce), bytes)
        .map_err(|_| "encrypt_failed".to_string())?;
    Ok(SealedBlob { cipher, key_b64: URL_SAFE_NO_PAD.encode(key), nonce_b64: URL_SAFE_NO_PAD.encode(nonce) })
}

pub fn open(cipher: &[u8], key_b64: &str, nonce_b64: &str) -> Result<Vec<u8>, String> {
    let key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| "bad_key".to_string())?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| "bad_nonce".to_string())?;
    if key.len() != 32 || nonce.len() != 12 {
        return Err("bad_key_material".into());
    }
    Aes256Gcm::new_from_slice(&key)
        .map_err(|_| "bad_key".to_string())?
        .decrypt(Nonce::from_slice(&nonce), cipher)
        .map_err(|_| "decrypt_failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper_detection() {
        let data = b"hello encrypted image bytes";
        let sealed = seal(data).unwrap();
        assert_ne!(sealed.cipher.as_slice(), data.as_slice());
        assert_eq!(open(&sealed.cipher, &sealed.key_b64, &sealed.nonce_b64).unwrap(), data);
        let mut bad = sealed.cipher.clone();
        bad[0] ^= 1;
        assert!(open(&bad, &sealed.key_b64, &sealed.nonce_b64).is_err());
    }
}
