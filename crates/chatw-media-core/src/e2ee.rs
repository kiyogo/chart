#![allow(deprecated)]
//! Unified Chunked Streaming E2EE (`CWCH3`, backward-compatible with `CWCH2` & v1 single-blob)
//! (Sections XIV.4 & XXV)
//!
//! - Uses random 256-bit per-media key wrapped per recipient/group by existing ChatW E2EE layer.
//! - `CWCH3` binds `(magic, version=3, media_id_hash, variant_id_hash, chunk_index, total_chunks, is_final)` into AEAD AAD.
//! - Derives domain-separated unique per-chunk keys/nonces via HKDF-SHA-256 (ZERO nonce reuse across chunks or ABR variants!).
//! - Supports O(1) random-access segment decryption for video/audio seeking without downloading the whole file!
//! - Signs/authenticates streaming manifest using the same content key with distinct HKDF domain `"chatw-manifest-v3"`.

use crate::{MediaError, MediaResult};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

pub const CWCH2_MAGIC: &[u8; 5] = b"CWCH2";
pub const CWCH3_MAGIC: &[u8; 5] = b"CWCH3";
pub const CWCH2_HEADER_LEN: usize = 21;
/// CWCH3 Header: magic(5) + version(1) + chunk_size(4) + chunk_count(4) + total_plain_len(8) + media_tag(8) + variant_tag(4) = 34 bytes
pub const CWCH3_HEADER_LEN: usize = 34;
pub const DEFAULT_STREAM_CHUNK_SIZE: usize = 512 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealedMedia {
    pub cipher: Vec<u8>,
    pub key_b64: String,
    pub nonce_b64: String,
    pub format_version: u8, // 1 = single-blob, 2 = CWCH2, 3 = CWCH3
    pub chunk_count: u32,
    pub chunk_offsets: Vec<ChunkOffsetEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkOffsetEntry {
    pub chunk_index: u32,
    pub cipher_offset: u64,
    pub cipher_len: u32,
    pub plain_len: u32,
    pub cipher_blake3_hex: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cwch3Header {
    pub version: u8,
    pub chunk_size: u32,
    pub chunk_count: u32,
    pub total_plain_len: u64,
    pub media_tag: [u8; 8],
    pub variant_tag: [u8; 4],
}

fn fill_random(dest: &mut [u8]) -> MediaResult<()> {
    getrandom::getrandom(dest).map_err(|e| MediaError::CryptoError(format!("os_rng: {e}")))
}

fn hash_tag_8(s: &str) -> [u8; 8] {
    let h = blake3::hash(s.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h.as_bytes()[..8]);
    out
}

fn hash_tag_4(s: &str) -> [u8; 4] {
    let h = blake3::hash(s.as_bytes());
    let mut out = [0u8; 4];
    out.copy_from_slice(&h.as_bytes()[..4]);
    out
}

/// Derive a unique 12-byte chunk nonce bound to `(base_nonce, media_tag, variant_tag, chunk_index)` via HKDF-SHA-256.
fn derive_cwch3_nonce(
    key: &[u8; 32],
    base_nonce: &[u8; 12],
    media_tag: &[u8; 8],
    variant_tag: &[u8; 4],
    chunk_index: u32,
) -> MediaResult<[u8; 12]> {
    let hk = Hkdf::<Sha256>::new(Some(base_nonce), key);
    let mut info = [0u8; 28];
    info[0..12].copy_from_slice(b"chatw-ch3-n!");
    info[12..20].copy_from_slice(media_tag);
    info[20..24].copy_from_slice(variant_tag);
    info[24..28].copy_from_slice(&chunk_index.to_be_bytes());
    let mut out = [0u8; 12];
    hk.expand(&info, &mut out)
        .map_err(|_| MediaError::CryptoError("hkdf_nonce_expand_failed".into()))?;
    Ok(out)
}

fn cwch3_aad(
    media_tag: &[u8; 8],
    variant_tag: &[u8; 4],
    chunk_index: u32,
    chunk_count: u32,
    is_final: bool,
) -> [u8; 27] {
    let mut aad = [0u8; 27];
    aad[0..5].copy_from_slice(CWCH3_MAGIC);
    aad[5] = 3;
    aad[6..14].copy_from_slice(media_tag);
    aad[14..18].copy_from_slice(variant_tag);
    aad[18..22].copy_from_slice(&chunk_index.to_be_bytes());
    aad[22..26].copy_from_slice(&chunk_count.to_be_bytes());
    aad[26] = u8::from(is_final);
    aad
}

pub fn parse_cwch3_header(bytes: &[u8]) -> Option<Cwch3Header> {
    if bytes.len() < CWCH3_HEADER_LEN || &bytes[0..5] != CWCH3_MAGIC {
        return None;
    }
    let version = bytes[5];
    let chunk_size = u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    let chunk_count = u32::from_be_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
    let total_plain_len = u64::from_be_bytes([
        bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
    ]);
    let mut media_tag = [0u8; 8];
    media_tag.copy_from_slice(&bytes[22..30]);
    let mut variant_tag = [0u8; 4];
    variant_tag.copy_from_slice(&bytes[30..34]);
    Some(Cwch3Header {
        version,
        chunk_size,
        chunk_count,
        total_plain_len,
        media_tag,
        variant_tag,
    })
}

/// Seal bytes using `CWCH3` chunked streaming AEAD bound to `(media_id, variant_id)`.
pub fn seal_cwch3(
    bytes: &[u8],
    media_id: &str,
    variant_id: &str,
    chunk_size: usize,
) -> MediaResult<SealedMedia> {
    let seg_len = chunk_size.clamp(16 * 1024, 4 * 1024 * 1024);
    let mut key = [0u8; 32];
    let mut base_nonce = [0u8; 12];
    fill_random(&mut key)?;
    fill_random(&mut base_nonce)?;

    let media_tag = hash_tag_8(media_id);
    let variant_tag = hash_tag_4(variant_id);

    let cipher_inst = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| MediaError::CryptoError("bad_aes_key".into()))?;

    let chunks: Vec<&[u8]> = if bytes.is_empty() {
        vec![&[][..]]
    } else {
        bytes.chunks(seg_len).collect()
    };
    let chunk_count = chunks.len() as u32;
    let total_plain_len = bytes.len() as u64;

    let mut out = Vec::with_capacity(CWCH3_HEADER_LEN + bytes.len() + chunks.len() * 20);
    out.extend_from_slice(CWCH3_MAGIC);
    out.push(3);
    out.extend_from_slice(&(seg_len as u32).to_be_bytes());
    out.extend_from_slice(&chunk_count.to_be_bytes());
    out.extend_from_slice(&total_plain_len.to_be_bytes());
    out.extend_from_slice(&media_tag);
    out.extend_from_slice(&variant_tag);

    let mut chunk_offsets = Vec::with_capacity(chunks.len());
    for (idx, chunk) in chunks.iter().enumerate() {
        let idx_u32 = idx as u32;
        let is_final = idx + 1 == chunks.len();
        let cn = derive_cwch3_nonce(&key, &base_nonce, &media_tag, &variant_tag, idx_u32)?;
        let aad = cwch3_aad(&media_tag, &variant_tag, idx_u32, chunk_count, is_final);
        let ct = cipher_inst
            .encrypt(
                Nonce::from_slice(&cn),
                Payload {
                    msg: chunk,
                    aad: &aad,
                },
            )
            .map_err(|_| MediaError::CryptoError("cwch3_encrypt_failed".into()))?;
        let offset = out.len() as u64;
        out.extend_from_slice(&(ct.len() as u32).to_be_bytes());
        out.extend_from_slice(&ct);
        chunk_offsets.push(ChunkOffsetEntry {
            chunk_index: idx_u32,
            cipher_offset: offset,
            cipher_len: ct.len() as u32,
            plain_len: chunk.len() as u32,
            cipher_blake3_hex: blake3::hash(&ct).to_hex().to_string(),
        });
    }

    let key_b64 = URL_SAFE_NO_PAD.encode(key);
    key.zeroize();
    let nonce_b64 = URL_SAFE_NO_PAD.encode(base_nonce);

    Ok(SealedMedia {
        cipher: out,
        key_b64,
        nonce_b64,
        format_version: 3,
        chunk_count,
        chunk_offsets,
    })
}

/// Decrypt a SINGLE `CWCH3` segment for O(1) streaming playback & seeking without loading the full media.
pub fn open_cwch3_chunk(
    chunk_ct: &[u8],
    header: &Cwch3Header,
    chunk_index: u32,
    key_b64: &str,
    nonce_b64: &str,
) -> MediaResult<Vec<u8>> {
    if chunk_index >= header.chunk_count {
        return Err(MediaError::CryptoError("chunk_index_out_of_range".into()));
    }
    let mut key_vec = URL_SAFE_NO_PAD
        .decode(key_b64)
        .map_err(|_| MediaError::CryptoError("bad_key_b64".into()))?;
    let nonce_vec = URL_SAFE_NO_PAD
        .decode(nonce_b64)
        .map_err(|_| MediaError::CryptoError("bad_nonce_b64".into()))?;
    if key_vec.len() != 32 || nonce_vec.len() != 12 {
        key_vec.zeroize();
        return Err(MediaError::CryptoError("bad_key_material_len".into()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_vec);
    key_vec.zeroize();
    let mut base_nonce = [0u8; 12];
    base_nonce.copy_from_slice(&nonce_vec);

    let cipher_inst = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| MediaError::CryptoError("bad_aes_key".into()))?;
    let cn = derive_cwch3_nonce(&key, &base_nonce, &header.media_tag, &header.variant_tag, chunk_index)?;
    key.zeroize();

    let is_final = chunk_index + 1 == header.chunk_count;
    let aad = cwch3_aad(&header.media_tag, &header.variant_tag, chunk_index, header.chunk_count, is_final);
    cipher_inst
        .decrypt(
            Nonce::from_slice(&cn),
            Payload {
                msg: chunk_ct,
                aad: &aad,
            },
        )
        .map_err(|_| MediaError::CryptoError("cwch3_chunk_auth_failed".into()))
}

/// Transparently decrypt `CWCH3`, legacy `CWCH2`, AND legacy v1 single-blob AES-256-GCM ciphertexts!
pub fn open_any_media(cipher: &[u8], key_b64: &str, nonce_b64: &str) -> MediaResult<Vec<u8>> {
    if let Some(hdr) = parse_cwch3_header(cipher) {
        let mut plain = Vec::with_capacity(hdr.total_plain_len.min(128 * 1024 * 1024) as usize);
        let mut pos = CWCH3_HEADER_LEN;
        for idx in 0..hdr.chunk_count {
            if pos + 4 > cipher.len() {
                return Err(MediaError::CryptoError("truncated_cwch3_len".into()));
            }
            let ct_len = u32::from_be_bytes([cipher[pos], cipher[pos + 1], cipher[pos + 2], cipher[pos + 3]]) as usize;
            pos += 4;
            if pos + ct_len > cipher.len() {
                return Err(MediaError::CryptoError("truncated_cwch3_chunk".into()));
            }
            let mut pt = open_cwch3_chunk(&cipher[pos..pos + ct_len], &hdr, idx, key_b64, nonce_b64)?;
            pos += ct_len;
            plain.append(&mut pt);
        }
        if pos != cipher.len() || plain.len() as u64 != hdr.total_plain_len {
            return Err(MediaError::CryptoError("cwch3_length_mismatch".into()));
        }
        return Ok(plain);
    }

    // Legacy CWCH2 fallback
    if cipher.len() >= CWCH2_HEADER_LEN && &cipher[0..5] == CWCH2_MAGIC {
        let chunk_count = u32::from_be_bytes([cipher[9], cipher[10], cipher[11], cipher[12]]);
        let total_plain_len = u64::from_be_bytes([
            cipher[13], cipher[14], cipher[15], cipher[16], cipher[17], cipher[18], cipher[19], cipher[20],
        ]);
        let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
        let nonce_vec = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| MediaError::CryptoError("bad_nonce".into()))?;
        if key.len() != 32 || nonce_vec.len() != 12 {
            key.zeroize();
            return Err(MediaError::CryptoError("bad_key_len".into()));
        }
        let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
        key.zeroize();
        let mut base_nonce = [0u8; 12];
        base_nonce.copy_from_slice(&nonce_vec);

        let mut plain = Vec::with_capacity(total_plain_len.min(128 * 1024 * 1024) as usize);
        let mut pos = CWCH2_HEADER_LEN;
        for idx in 0..chunk_count {
            if pos + 4 > cipher.len() {
                return Err(MediaError::CryptoError("truncated_cwch2".into()));
            }
            let ct_len = u32::from_be_bytes([cipher[pos], cipher[pos + 1], cipher[pos + 2], cipher[pos + 3]]) as usize;
            pos += 4;
            if pos + ct_len > cipher.len() {
                return Err(MediaError::CryptoError("truncated_cwch2_payload".into()));
            }
            let mut cn = base_nonce;
            let idx_be = idx.to_be_bytes();
            for i in 0..4 {
                cn[8 + i] ^= idx_be[i];
            }
            let mut aad = [0u8; 10];
            aad[0..5].copy_from_slice(CWCH2_MAGIC);
            aad[5..9].copy_from_slice(&idx_be);
            aad[9] = u8::from(idx + 1 == chunk_count);
            let mut pt = cipher_inst
                .decrypt(
                    Nonce::from_slice(&cn),
                    Payload {
                        msg: &cipher[pos..pos + ct_len],
                        aad: &aad,
                    },
                )
                .map_err(|_| MediaError::CryptoError("cwch2_decrypt_failed".into()))?;
            pos += ct_len;
            plain.append(&mut pt);
        }
        if pos != cipher.len() || plain.len() as u64 != total_plain_len {
            return Err(MediaError::CryptoError("cwch2_length_mismatch".into()));
        }
        return Ok(plain);
    }

    // Legacy v1 single-blob AES-256-GCM
    let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| MediaError::CryptoError("bad_nonce".into()))?;
    if key.len() != 32 || nonce.len() != 12 {
        key.zeroize();
        return Err(MediaError::CryptoError("bad_key_len".into()));
    }
    let cipher_inst = Aes256Gcm::new_from_slice(&key).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    key.zeroize();
    cipher_inst
        .decrypt(Nonce::from_slice(&nonce), cipher)
        .map_err(|_| MediaError::CryptoError("v1_decrypt_failed".into()))
}

/// Seal a JSON streaming manifest using an HKDF-derived manifest authentication key so manifests
/// cannot be tampered with by an untrusted relay/CDN/S3 bucket!
pub fn seal_authenticated_manifest(
    manifest_json: &[u8],
    key_b64: &str,
    nonce_b64: &str,
    media_id: &str,
) -> MediaResult<Vec<u8>> {
    let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| MediaError::CryptoError("bad_nonce".into()))?;
    if key.len() != 32 || nonce.len() != 12 {
        key.zeroize();
        return Err(MediaError::CryptoError("bad_key_len".into()));
    }
    let hk = Hkdf::<Sha256>::new(Some(&nonce), &key);
    key.zeroize();
    let mut mkey = [0u8; 32];
    let mut mnonce = [0u8; 12];
    hk.expand(b"chatw-manifest-key-v3", &mut mkey).map_err(|_| MediaError::CryptoError("hkdf".into()))?;
    hk.expand(b"chatw-manifest-nonce-v3", &mut mnonce).map_err(|_| MediaError::CryptoError("hkdf".into()))?;
    let inst = Aes256Gcm::new_from_slice(&mkey).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    mkey.zeroize();
    let aad = format!("CWMAN3:{media_id}");
    inst.encrypt(
        Nonce::from_slice(&mnonce),
        Payload {
            msg: manifest_json,
            aad: aad.as_bytes(),
        },
    )
    .map_err(|_| MediaError::CryptoError("manifest_seal_failed".into()))
}

pub fn open_authenticated_manifest(
    sealed_manifest: &[u8],
    key_b64: &str,
    nonce_b64: &str,
    media_id: &str,
) -> MediaResult<Vec<u8>> {
    let mut key = URL_SAFE_NO_PAD.decode(key_b64).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce_b64).map_err(|_| MediaError::CryptoError("bad_nonce".into()))?;
    if key.len() != 32 || nonce.len() != 12 {
        key.zeroize();
        return Err(MediaError::CryptoError("bad_key_len".into()));
    }
    let hk = Hkdf::<Sha256>::new(Some(&nonce), &key);
    key.zeroize();
    let mut mkey = [0u8; 32];
    let mut mnonce = [0u8; 12];
    hk.expand(b"chatw-manifest-key-v3", &mut mkey).map_err(|_| MediaError::CryptoError("hkdf".into()))?;
    hk.expand(b"chatw-manifest-nonce-v3", &mut mnonce).map_err(|_| MediaError::CryptoError("hkdf".into()))?;
    let inst = Aes256Gcm::new_from_slice(&mkey).map_err(|_| MediaError::CryptoError("bad_key".into()))?;
    mkey.zeroize();
    let aad = format!("CWMAN3:{media_id}");
    inst.decrypt(
        Nonce::from_slice(&mnonce),
        Payload {
            msg: sealed_manifest,
            aad: aad.as_bytes(),
        },
    )
    .map_err(|_| MediaError::CryptoError("manifest_auth_failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cwch3_random_access_seek_and_cross_variant_replay_rejection() {
        let data: Vec<u8> = (0..(100 * 1024)).map(|i| (i % 241) as u8).collect();
        let sealed = seal_cwch3(&data, "media-abc", "av1_720p", 32 * 1024).expect("seal");
        assert_eq!(sealed.chunk_count, 4);
        assert_eq!(sealed.chunk_offsets.len(), 4);

        let full = open_any_media(&sealed.cipher, &sealed.key_b64, &sealed.nonce_b64).expect("open full");
        assert_eq!(full, data);

        // Random-access decrypt chunk #2 directly (video seek!)
        let hdr = parse_cwch3_header(&sealed.cipher).unwrap();
        let off = sealed.chunk_offsets[2].cipher_offset as usize + 4;
        let len = sealed.chunk_offsets[2].cipher_len as usize;
        let seg2 = open_cwch3_chunk(&sealed.cipher[off..off + len], &hdr, 2, &sealed.key_b64, &sealed.nonce_b64)
            .expect("random seek chunk 2");
        assert_eq!(seg2, &data[64 * 1024..96 * 1024]);

        // Replaying chunk #2 as chunk #1 or under a different variant tag MUST fail AEAD!
        assert!(open_cwch3_chunk(&sealed.cipher[off..off + len], &hdr, 1, &sealed.key_b64, &sealed.nonce_b64).is_err());
        let mut wrong_var = hdr;
        wrong_var.variant_tag = hash_tag_4("av1_1080p");
        assert!(open_cwch3_chunk(&sealed.cipher[off..off + len], &wrong_var, 2, &sealed.key_b64, &sealed.nonce_b64).is_err());

        // Manifest authentication binds media_id
        let man = b"{\"segments\":4}";
        let sealed_man = seal_authenticated_manifest(man, &sealed.key_b64, &sealed.nonce_b64, "media-abc").unwrap();
        assert_eq!(open_authenticated_manifest(&sealed_man, &sealed.key_b64, &sealed.nonce_b64, "media-abc").unwrap(), man);
        assert!(open_authenticated_manifest(&sealed_man, &sealed.key_b64, &sealed.nonce_b64, "media-other").is_err());
    }
}
