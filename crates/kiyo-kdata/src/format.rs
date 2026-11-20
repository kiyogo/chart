use std::io::{Read, Write};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305,
    aead::{Aead, Payload, stream},
};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    error::KdataError,
    model::{
        CIPHER_XCHACHA20_POLY1305_BE32, FLAG_COMPRESSED_META,
        FLAG_INCREMENTAL, ImportLimits, KDATA_FOOTER_MAGIC, KDATA_FORMAT_VERSION, KDATA_MAGIC,
        KdataIndexEntry, KdataManifest, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, sanitize_relative_path,
    },
};

type HmacSha256 = Hmac<Sha256>;

pub const HEADER_LEN: usize = 172;
pub const FOOTER_LEN: usize = 80;
const HEADER_PREFIX_LEN: usize = 68;
const HEADER_MAC_OFFSET: usize = 140;

#[derive(Clone, Debug)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost_kib: 19_456, // 19 MiB (OWASP Argon2id baseline, WASM & mobile safe)
            t_cost: 2,
            p_cost: 1,
        }
    }
}

impl KdfParams {
    pub fn fast_for_tests() -> Self {
        Self {
            m_cost_kib: 512,
            t_cost: 1,
            p_cost: 1,
        }
    }

    pub fn validate(&self) -> Result<(), KdataError> {
        if !(256..=131_072).contains(&self.m_cost_kib) {
            return Err(KdataError::InvalidHeader("argon2 m_cost ngoài giới hạn an toàn".into()));
        }
        if !(1..=16).contains(&self.t_cost) || !(1..=8).contains(&self.p_cost) {
            return Err(KdataError::InvalidHeader("argon2 t_cost/p_cost ngoài giới hạn an toàn".into()));
        }
        Ok(())
    }
}

fn fill_random(buf: &mut [u8]) -> Result<(), KdataError> {
    getrandom::fill(buf).map_err(|e| KdataError::InvalidHeader(format!("rng error: {e}")))
}

fn derive_kek(password: &str, salt: &[u8; 16], kdf: &KdfParams) -> Result<Zeroizing<[u8; 32]>, KdataError> {
    if password.is_empty() {
        return Err(KdataError::WrongPasswordOrTampered);
    }
    kdf.validate()?;
    let params = Params::new(kdf.m_cost_kib, kdf.t_cost, kdf.p_cost, Some(32))
        .map_err(|e| KdataError::InvalidHeader(format!("argon2 params: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut kek = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut *kek)
        .map_err(|e| KdataError::InvalidHeader(format!("argon2 derive: {e}")))?;
    Ok(kek)
}

fn hkdf_subkey(master: &[u8; 32], backup_id: &Uuid, info: &[u8]) -> Result<Zeroizing<[u8; 32]>, KdataError> {
    let hk = Hkdf::<Sha256>::new(Some(backup_id.as_bytes()), master);
    let mut out = Zeroizing::new([0u8; 32]);
    hk.expand(info, &mut *out)
        .map_err(|_| KdataError::InvalidHeader("hkdf expand failed".into()))?;
    Ok(out)
}

fn make_section_aad(backup_id: &Uuid, version: u16, section: &[u8; 4]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(22);
    aad.extend_from_slice(backup_id.as_bytes());
    aad.extend_from_slice(&version.to_le_bytes());
    aad.extend_from_slice(section);
    aad
}

fn seal_section(key: &[u8; 32], aad: &[u8], plain: &[u8], compress: bool) -> Result<Vec<u8>, KdataError> {
    let body = if compress {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(plain)?;
        enc.finish()?
    } else {
        plain.to_vec()
    };
    let mut nonce = [0u8; 24];
    fill_random(&mut nonce)?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let ct = cipher
        .encrypt((&nonce).into(), Payload { msg: &body, aad })
        .map_err(|_| KdataError::AuthenticationFailed)?;
    let mut frame = Vec::with_capacity(4 + 24 + ct.len());
    let len = u32::try_from(24 + ct.len()).map_err(|_| KdataError::ResourceLimitExceeded("section quá lớn".into()))?;
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&nonce);
    frame.extend_from_slice(&ct);
    Ok(frame)
}

fn open_section<R: Read>(
    reader: &mut R,
    transcript: &mut Sha256,
    key: &[u8; 32],
    aad: &[u8],
    compressed: bool,
    max_raw_bytes: usize,
    max_plain_bytes: usize,
) -> Result<Vec<u8>, KdataError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    transcript.update(&len_buf);
    let raw_len = u32::from_le_bytes(len_buf) as usize;
    if raw_len < 24 + 16 || raw_len > max_raw_bytes {
        return Err(KdataError::ResourceLimitExceeded("section frame vượt giới hạn".into()));
    }
    let mut raw = vec![0u8; raw_len];
    reader.read_exact(&mut raw)?;
    transcript.update(&raw);
    let nonce: [u8; 24] = raw[..24].try_into().map_err(|_| KdataError::AuthenticationFailed)?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let decrypted = cipher
        .decrypt((&nonce).into(), Payload { msg: &raw[24..], aad })
        .map_err(|_| KdataError::WrongPasswordOrTampered)?;
    if !compressed {
        if decrypted.len() > max_plain_bytes {
            return Err(KdataError::ResourceLimitExceeded("section giải nén vượt giới hạn".into()));
        }
        return Ok(decrypted);
    }
    let mut dec = ZlibDecoder::new(decrypted.as_slice());
    let mut out = Vec::new();
    let mut limited = (&mut dec).take((max_plain_bytes as u64) + 1);
    limited.read_to_end(&mut out)?;
    if out.len() > max_plain_bytes {
        return Err(KdataError::ResourceLimitExceeded("section vượt giới hạn giải nén".into()));
    }
    Ok(out)
}

/// Streaming `.kdata` v1 writer: writes Header, Encrypted Manifest, Encrypted Index,
/// streaming BE32 XChaCha20-Poly1305 data frames, and Integrity Footer in one forward pass
/// with O(chunk_size) memory.
pub struct KdataStreamWriter<W: Write> {
    writer: W,
    backup_id: Uuid,
    chunk_size: usize,
    dek: Zeroizing<[u8; 32]>,
    trailer_mac_key: Zeroizing<[u8; 32]>,
    encryptor: Option<stream::EncryptorBE32<XChaCha20Poly1305>>,
    stage: Vec<u8>,
    data_aad: Vec<u8>,
    frame_count: u64,
    transcript: Sha256,
    finished: bool,
}

impl<W: Write> KdataStreamWriter<W> {
    pub fn begin(
        mut writer: W,
        password: &str,
        manifest: &KdataManifest,
        index: &[KdataIndexEntry],
        chunk_size: u32,
        kdf: KdfParams,
        created_at_unix: i64,
    ) -> Result<Self, KdataError> {
        if password.is_empty() {
            return Err(KdataError::WrongPasswordOrTampered);
        }
        if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) {
            return Err(KdataError::InvalidHeader("chunk_size ngoài giới hạn 16 KiB..16 MiB".into()));
        }
        kdf.validate()?;
        for item in index {
            sanitize_relative_path(&item.logical_path)?;
        }

        let backup_id = manifest.backup_id;
        let mut salt = [0u8; 16];
        fill_random(&mut salt)?;
        let kek = derive_kek(password, &salt, &kdf)?;

        let mut dek = Zeroizing::new([0u8; 32]);
        fill_random(&mut *dek)?;

        let flags = FLAG_COMPRESSED_META | if manifest.incremental { FLAG_INCREMENTAL } else { 0 };

        let mut hdr = [0u8; HEADER_LEN];
        hdr[0..8].copy_from_slice(KDATA_MAGIC);
        hdr[8..10].copy_from_slice(&KDATA_FORMAT_VERSION.to_le_bytes());
        hdr[10] = CIPHER_XCHACHA20_POLY1305_BE32;
        hdr[11] = flags;
        hdr[12..16].copy_from_slice(&chunk_size.to_le_bytes());
        hdr[16..32].copy_from_slice(backup_id.as_bytes());
        hdr[32..40].copy_from_slice(&created_at_unix.to_le_bytes());
        hdr[40..56].copy_from_slice(&salt);
        hdr[56..60].copy_from_slice(&kdf.m_cost_kib.to_le_bytes());
        hdr[60..64].copy_from_slice(&kdf.t_cost.to_le_bytes());
        hdr[64..68].copy_from_slice(&kdf.p_cost.to_le_bytes());

        let mut wrap_nonce = [0u8; 24];
        fill_random(&mut wrap_nonce)?;
        let kek_cipher = XChaCha20Poly1305::new((&*kek).into());
        let wrapped = kek_cipher
            .encrypt(
                (&wrap_nonce).into(),
                Payload {
                    msg: &*dek,
                    aad: &hdr[..HEADER_PREFIX_LEN],
                },
            )
            .map_err(|_| KdataError::AuthenticationFailed)?;
        if wrapped.len() != 48 {
            return Err(KdataError::InvalidHeader("wrapped_dek length".into()));
        }
        hdr[68..92].copy_from_slice(&wrap_nonce);
        hdr[92..140].copy_from_slice(&wrapped);

        let hdr_mac_key = hkdf_subkey(&kek, &backup_id, b"kiyo-kdata-hdr-mac-v1")?;
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&*hdr_mac_key)
            .map_err(|_| KdataError::InvalidHeader("hmac key".into()))?;
        mac.update(&hdr[..HEADER_MAC_OFFSET]);
        hdr[HEADER_MAC_OFFSET..HEADER_LEN].copy_from_slice(&mac.finalize().into_bytes());

        let mut transcript = Sha256::new();
        writer.write_all(&hdr)?;
        transcript.update(&hdr);

        // Manifest
        let mnf_key = hkdf_subkey(&dek, &backup_id, b"kiyo-kdata-manifest-v1")?;
        let mnf_aad = make_section_aad(&backup_id, KDATA_FORMAT_VERSION, b"MNF1");
        let mnf_json = serde_json::to_vec(manifest).map_err(|e| KdataError::FormatViolation(e.to_string()))?;
        let mnf_frame = seal_section(&mnf_key, &mnf_aad, &mnf_json, true)?;
        writer.write_all(&mnf_frame)?;
        transcript.update(&mnf_frame);

        // Index
        let idx_key = hkdf_subkey(&dek, &backup_id, b"kiyo-kdata-index-v1")?;
        let idx_aad = make_section_aad(&backup_id, KDATA_FORMAT_VERSION, b"IDX1");
        let idx_json = serde_json::to_vec(index).map_err(|e| KdataError::FormatViolation(e.to_string()))?;
        let idx_frame = seal_section(&idx_key, &idx_aad, &idx_json, true)?;
        writer.write_all(&idx_frame)?;
        transcript.update(&idx_frame);

        // Stream base nonce (19 bytes for XChaCha20-Poly1305 STREAM BE32)
        let mut stream_nonce = [0u8; 19];
        fill_random(&mut stream_nonce)?;
        writer.write_all(&stream_nonce)?;
        transcript.update(&stream_nonce);

        let stream_key = hkdf_subkey(&dek, &backup_id, b"kiyo-kdata-stream-v1")?;
        let stream_aead = XChaCha20Poly1305::new((&*stream_key).into());
        let encryptor = stream::EncryptorBE32::from_aead(stream_aead, (&stream_nonce).into());
        let trailer_mac_key = hkdf_subkey(&dek, &backup_id, b"kiyo-kdata-trailer-mac-v1")?;
        let data_aad = make_section_aad(&backup_id, KDATA_FORMAT_VERSION, b"DAT1");

        Ok(Self {
            writer,
            backup_id,
            chunk_size: chunk_size as usize,
            dek,
            trailer_mac_key,
            encryptor: Some(encryptor),
            stage: Vec::with_capacity(chunk_size as usize),
            data_aad,
            frame_count: 0,
            transcript,
            finished: false,
        })
    }

    fn flush_full_stage(&mut self) -> Result<(), KdataError> {
        let Some(enc) = self.encryptor.as_mut() else {
            return Err(KdataError::FormatViolation("encryptor closed".into()));
        };
        let ct = enc
            .encrypt_next(Payload {
                msg: &self.stage,
                aad: &self.data_aad,
            })
            .map_err(|_| KdataError::AuthenticationFailed)?;
        let len_u32 = u32::try_from(ct.len()).map_err(|_| KdataError::ResourceLimitExceeded("frame overflow".into()))?;
        let len_bytes = len_u32.to_le_bytes();
        self.writer.write_all(&len_bytes)?;
        self.writer.write_all(&ct)?;
        self.transcript.update(&len_bytes);
        self.transcript.update(&ct);
        self.frame_count += 1;
        self.stage.clear();
        Ok(())
    }

    fn write_plain_slice(&mut self, mut slice: &[u8]) -> Result<(), KdataError> {
        while !slice.is_empty() {
            let room = self.chunk_size - self.stage.len();
            let take = room.min(slice.len());
            self.stage.extend_from_slice(&slice[..take]);
            slice = &slice[take..];
            if self.stage.len() == self.chunk_size {
                self.flush_full_stage()?;
            }
        }
        Ok(())
    }

    /// Streams an entry from any `Read` source (e.g. multi-GB file) without buffering the entry in RAM.
    /// Returns the actual bytes read and SHA-256 hex digest of the entry payload.
    pub fn write_entry_from_reader<R: Read>(
        &mut self,
        entry_id: u32,
        declared_len: u64,
        reader: &mut R,
    ) -> Result<(u64, String), KdataError> {
        self.write_plain_slice(&entry_id.to_le_bytes())?;
        self.write_plain_slice(&declared_len.to_le_bytes())?;

        let mut hasher = Sha256::new();
        let mut total = 0u64;
        let mut buf = vec![0u8; self.chunk_size.min(256 * 1024)];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > declared_len {
                return Err(KdataError::FormatViolation("entry đọc vượt quá kích thước khai báo".into()));
            }
            hasher.update(&buf[..n]);
            self.write_plain_slice(&buf[..n])?;
        }
        if total != declared_len {
            return Err(KdataError::FormatViolation(format!(
                "entry {entry_id} thiếu dữ liệu: khai báo {declared_len}, thực đọc {total}"
            )));
        }
        Ok((total, hex::encode(hasher.finalize())))
    }

    /// Convenience helper for in-memory JSON/metadata entries.
    pub fn write_entry_bytes(&mut self, entry_id: u32, bytes: &[u8]) -> Result<String, KdataError> {
        let mut cursor = std::io::Cursor::new(bytes);
        let (_, sha) = self.write_entry_from_reader(entry_id, bytes.len() as u64, &mut cursor)?;
        Ok(sha)
    }

    /// Finalizes the BE32 stream (`LAST` tag), writes end-of-stream marker + integrity trailer,
    /// and flushes the underlying writer.
    pub fn finish(mut self) -> Result<W, KdataError> {
        let enc = self
            .encryptor
            .take()
            .ok_or_else(|| KdataError::FormatViolation("already finished".into()))?;
        let last_ct = enc
            .encrypt_last(Payload {
                msg: &self.stage,
                aad: &self.data_aad,
            })
            .map_err(|_| KdataError::AuthenticationFailed)?;
        let len_u32 = u32::try_from(last_ct.len()).map_err(|_| KdataError::ResourceLimitExceeded("last frame overflow".into()))?;
        let len_bytes = len_u32.to_le_bytes();
        self.writer.write_all(&len_bytes)?;
        self.writer.write_all(&last_ct)?;
        self.transcript.update(&len_bytes);
        self.transcript.update(&last_ct);
        self.frame_count += 1;
        self.stage.clear();

        // Stream end marker (u32 == 0)
        let eof_marker = 0u32.to_le_bytes();
        self.writer.write_all(&eof_marker)?;
        self.transcript.update(&eof_marker);

        let transcript_hash: [u8; 32] = self.transcript.clone().finalize().into();

        let mut trailer = [0u8; FOOTER_LEN];
        trailer[0..8].copy_from_slice(KDATA_FOOTER_MAGIC);
        trailer[8..16].copy_from_slice(&self.frame_count.to_le_bytes());
        trailer[16..48].copy_from_slice(&transcript_hash);

        let mut tmac = <HmacSha256 as Mac>::new_from_slice(&*self.trailer_mac_key)
            .map_err(|_| KdataError::InvalidHeader("trailer hmac".into()))?;
        tmac.update(&transcript_hash);
        tmac.update(&self.frame_count.to_le_bytes());
        tmac.update(self.backup_id.as_bytes());
        trailer[48..80].copy_from_slice(&tmac.finalize().into_bytes());

        self.writer.write_all(&trailer)?;
        self.writer.flush()?;
        self.dek.zeroize();
        self.finished = true;
        Ok(self.writer)
    }
}

pub struct ParsedKdataHeader {
    pub format_version: u16,
    pub cipher_id: u8,
    pub flags: u8,
    pub chunk_size: u32,
    pub backup_id: Uuid,
    pub created_at_unix: i64,
    pub salt: [u8; 16],
    pub kdf: KdfParams,
}

pub struct OpenedKdataArchive {
    pub header: ParsedKdataHeader,
    pub manifest: KdataManifest,
    pub index: Vec<KdataIndexEntry>,
    pub frame_count: u64,
    pub transcript_sha256_hex: String,
}

/// Inspects only the unencrypted header prefix (without password) to detect format & KDF params.
pub fn probe_kdata_header(prefix: &[u8]) -> Result<ParsedKdataHeader, KdataError> {
    if prefix.len() < HEADER_LEN {
        return Err(KdataError::InvalidMagic);
    }
    if &prefix[0..8] != KDATA_MAGIC {
        return Err(KdataError::InvalidMagic);
    }
    let format_version = u16::from_le_bytes([prefix[8], prefix[9]]);
    if format_version != KDATA_FORMAT_VERSION {
        return Err(KdataError::UnsupportedVersion(format_version));
    }
    let cipher_id = prefix[10];
    if cipher_id != CIPHER_XCHACHA20_POLY1305_BE32 {
        return Err(KdataError::UnsupportedCipher(cipher_id));
    }
    let flags = prefix[11];
    let chunk_size = u32::from_le_bytes([prefix[12], prefix[13], prefix[14], prefix[15]]);
    if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) {
        return Err(KdataError::InvalidHeader("chunk_size ngoài giới hạn".into()));
    }
    let backup_id = Uuid::from_slice(&prefix[16..32]).map_err(|_| KdataError::InvalidHeader("backup_id".into()))?;
    let created_at_unix = i64::from_le_bytes(prefix[32..40].try_into().unwrap());
    let salt: [u8; 16] = prefix[40..56].try_into().unwrap();
    let kdf = KdfParams {
        m_cost_kib: u32::from_le_bytes(prefix[56..60].try_into().unwrap()),
        t_cost: u32::from_le_bytes(prefix[60..64].try_into().unwrap()),
        p_cost: u32::from_le_bytes(prefix[64..68].try_into().unwrap()),
    };
    kdf.validate()?;
    Ok(ParsedKdataHeader {
        format_version,
        cipher_id,
        flags,
        chunk_size,
        backup_id,
        created_at_unix,
        salt,
        kdf,
    })
}

/// Streams, decrypts, and verifies an entire `.kdata` v1 archive in bounded memory,
/// passing each decoded entry chunk to `on_chunk(entry, slice, is_entry_end)`.
pub fn stream_read_kdata<R, F>(
    reader: &mut R,
    password: &str,
    limits: &ImportLimits,
    mut on_chunk: F,
) -> Result<OpenedKdataArchive, KdataError>
where
    R: Read,
    F: FnMut(&KdataIndexEntry, &[u8], bool) -> Result<(), KdataError>,
{
    let mut hdr = [0u8; HEADER_LEN];
    reader.read_exact(&mut hdr).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            KdataError::InvalidMagic
        } else {
            KdataError::Io(e)
        }
    })?;
    let parsed = probe_kdata_header(&hdr)?;
    let kek = derive_kek(password, &parsed.salt, &parsed.kdf)?;

    // Verify header MAC before unwrapping DEK
    let hdr_mac_key = hkdf_subkey(&kek, &parsed.backup_id, b"kiyo-kdata-hdr-mac-v1")?;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&*hdr_mac_key)
        .map_err(|_| KdataError::InvalidHeader("hdr hmac".into()))?;
    mac.update(&hdr[..HEADER_MAC_OFFSET]);
    mac.verify_slice(&hdr[HEADER_MAC_OFFSET..HEADER_LEN])
        .map_err(|_| KdataError::WrongPasswordOrTampered)?;

    let wrap_nonce: [u8; 24] = hdr[68..92].try_into().unwrap();
    let kek_cipher = XChaCha20Poly1305::new((&*kek).into());
    let dek_vec = kek_cipher
        .decrypt(
            (&wrap_nonce).into(),
            Payload {
                msg: &hdr[92..140],
                aad: &hdr[..HEADER_PREFIX_LEN],
            },
        )
        .map_err(|_| KdataError::WrongPasswordOrTampered)?;
    if dek_vec.len() != 32 {
        return Err(KdataError::WrongPasswordOrTampered);
    }
    let mut dek = Zeroizing::new([0u8; 32]);
    dek.copy_from_slice(&dek_vec);

    let mut transcript = Sha256::new();
    transcript.update(&hdr);

    let compressed = (parsed.flags & FLAG_COMPRESSED_META) != 0;

    // 1. Open Manifest
    let mnf_key = hkdf_subkey(&dek, &parsed.backup_id, b"kiyo-kdata-manifest-v1")?;
    let mnf_aad = make_section_aad(&parsed.backup_id, parsed.format_version, b"MNF1");
    let mnf_bytes = open_section(
        reader,
        &mut transcript,
        &mnf_key,
        &mnf_aad,
        compressed,
        limits.max_manifest_bytes,
        limits.max_manifest_bytes,
    )?;
    let manifest: KdataManifest =
        serde_json::from_slice(&mnf_bytes).map_err(|e| KdataError::FormatViolation(format!("manifest json: {e}")))?;
    if manifest.backup_id != parsed.backup_id {
        return Err(KdataError::IntegrityMismatch("backup_id manifest không khớp header".into()));
    }

    // 2. Open Index
    let idx_key = hkdf_subkey(&dek, &parsed.backup_id, b"kiyo-kdata-index-v1")?;
    let idx_aad = make_section_aad(&parsed.backup_id, parsed.format_version, b"IDX1");
    let idx_bytes = open_section(
        reader,
        &mut transcript,
        &idx_key,
        &idx_aad,
        compressed,
        limits.max_index_bytes,
        limits.max_index_bytes,
    )?;
    let index: Vec<KdataIndexEntry> =
        serde_json::from_slice(&idx_bytes).map_err(|e| KdataError::FormatViolation(format!("index json: {e}")))?;
    if index.len() > limits.max_entries {
        return Err(KdataError::ResourceLimitExceeded("số lượng entry vượt giới hạn".into()));
    }
    for entry in &index {
        sanitize_relative_path(&entry.logical_path)?;
        if entry.plain_size > limits.max_single_entry_bytes {
            return Err(KdataError::ResourceLimitExceeded("một tệp vượt giới hạn kích thước cho phép".into()));
        }
    }

    // 3. Stream Data Frames
    let mut stream_nonce = [0u8; 19];
    reader.read_exact(&mut stream_nonce)?;
    transcript.update(&stream_nonce);

    let stream_key = hkdf_subkey(&dek, &parsed.backup_id, b"kiyo-kdata-stream-v1")?;
    let stream_aead = XChaCha20Poly1305::new((&*stream_key).into());
    let mut decryptor = Some(stream::DecryptorBE32::from_aead(stream_aead, (&stream_nonce).into()));
    let data_aad = make_section_aad(&parsed.backup_id, parsed.format_version, b"DAT1");

    // Incremental record parser state across decrypted frames
    let mut frame_count = 0u64;
    let mut total_plain_bytes = 0u64;
    let mut entry_idx = 0usize;
    let mut rec_hdr_buf = Vec::<u8>::with_capacity(12);
    let mut cur_entry_rem = 0u64;
    let mut cur_hasher: Option<Sha256> = None;
    let mut in_entry = false;

    let max_ct_frame = (parsed.chunk_size as usize) + 64;
    let mut next_frame_len = {
        let mut lb = [0u8; 4];
        reader.read_exact(&mut lb)?;
        transcript.update(&lb);
        u32::from_le_bytes(lb) as usize
    };
    if next_frame_len == 0 {
        return Err(KdataError::FormatViolation("thiếu data stream frame cuối".into()));
    }

    loop {
        if next_frame_len < 16 || next_frame_len > max_ct_frame {
            return Err(KdataError::FormatViolation("kích thước frame mã hóa không hợp lệ".into()));
        }
        let mut ct = vec![0u8; next_frame_len];
        reader.read_exact(&mut ct)?;
        transcript.update(&ct);

        let mut peek_lb = [0u8; 4];
        reader.read_exact(&mut peek_lb)?;
        transcript.update(&peek_lb);
        let upcoming_len = u32::from_le_bytes(peek_lb) as usize;
        frame_count += 1;

        let plain = if upcoming_len == 0 {
            let dec = decryptor.take().ok_or(KdataError::AuthenticationFailed)?;
            dec.decrypt_last(Payload { msg: &ct, aad: &data_aad })
                .map_err(|_| KdataError::AuthenticationFailed)?
        } else {
            let dec = decryptor.as_mut().ok_or(KdataError::AuthenticationFailed)?;
            dec.decrypt_next(Payload { msg: &ct, aad: &data_aad })
                .map_err(|_| KdataError::AuthenticationFailed)?
        };

        total_plain_bytes = total_plain_bytes
            .checked_add(plain.len() as u64)
            .ok_or_else(|| KdataError::ResourceLimitExceeded("overflow tổng dung lượng".into()))?;
        if total_plain_bytes > limits.max_total_plain_bytes {
            return Err(KdataError::ResourceLimitExceeded("tổng dữ liệu giải mã vượt giới hạn".into()));
        }

        // Feed `plain` into record demuxer
        let mut pos = 0usize;
        while pos < plain.len() {
            if !in_entry {
                let need = 12 - rec_hdr_buf.len();
                let take = need.min(plain.len() - pos);
                rec_hdr_buf.extend_from_slice(&plain[pos..pos + take]);
                pos += take;
                if rec_hdr_buf.len() == 12 {
                    let eid = u32::from_le_bytes(rec_hdr_buf[0..4].try_into().unwrap());
                    let decl_len = u64::from_le_bytes(rec_hdr_buf[4..12].try_into().unwrap());
                    rec_hdr_buf.clear();
                    let expected = index
                        .get(entry_idx)
                        .ok_or_else(|| KdataError::FormatViolation("dư bản ghi so với index".into()))?;
                    if expected.entry_id != eid || expected.plain_size != decl_len {
                        return Err(KdataError::IntegrityMismatch("entry id hoặc kích thước không khớp index".into()));
                    }
                    in_entry = true;
                    cur_entry_rem = decl_len;
                    cur_hasher = Some(Sha256::new());
                    if cur_entry_rem == 0 {
                        let got_sha = hex::encode(cur_hasher.take().unwrap().finalize());
                        if got_sha != expected.sha256_hex {
                            return Err(KdataError::IntegrityMismatch(format!("sai SHA-256 ở {}", expected.logical_path)));
                        }
                        on_chunk(expected, &[], true)?;
                        in_entry = false;
                        entry_idx += 1;
                    }
                }
            } else {
                let expected = &index[entry_idx];
                let take = (plain.len() - pos).min(cur_entry_rem as usize);
                let slice = &plain[pos..pos + take];
                if let Some(h) = cur_hasher.as_mut() {
                    h.update(slice);
                }
                cur_entry_rem -= take as u64;
                pos += take;
                let done = cur_entry_rem == 0;
                if done {
                    let got_sha = hex::encode(cur_hasher.take().unwrap().finalize());
                    if got_sha != expected.sha256_hex {
                        return Err(KdataError::IntegrityMismatch(format!("sai SHA-256 ở {}", expected.logical_path)));
                    }
                }
                on_chunk(expected, slice, done)?;
                if done {
                    in_entry = false;
                    entry_idx += 1;
                }
            }
        }

        if upcoming_len == 0 {
            break;
        }
        next_frame_len = upcoming_len;
    }

    if in_entry || !rec_hdr_buf.is_empty() || entry_idx != index.len() {
        return Err(KdataError::IntegrityMismatch("thiếu dữ liệu bản ghi so với manifest/index".into()));
    }

    // 4. Verify Footer & Trailer MAC + check no trailing garbage bytes
    let mut trailer = [0u8; FOOTER_LEN];
    reader.read_exact(&mut trailer)?;
    if &trailer[0..8] != KDATA_FOOTER_MAGIC {
        return Err(KdataError::FormatViolation("thiếu footer KIYOEND1".into()));
    }
    let declared_frames = u64::from_le_bytes(trailer[8..16].try_into().unwrap());
    if declared_frames != frame_count {
        return Err(KdataError::IntegrityMismatch("số lượng frame dữ liệu không khớp footer".into()));
    }
    let expected_transcript: [u8; 32] = transcript.finalize().into();
    if trailer[16..48] != expected_transcript {
        return Err(KdataError::IntegrityMismatch("transcript SHA-256 không khớp".into()));
    }

    let trailer_mac_key = hkdf_subkey(&dek, &parsed.backup_id, b"kiyo-kdata-trailer-mac-v1")?;
    let mut tmac = <HmacSha256 as Mac>::new_from_slice(&*trailer_mac_key)
        .map_err(|_| KdataError::InvalidHeader("trailer mac".into()))?;
    tmac.update(&expected_transcript);
    tmac.update(&frame_count.to_le_bytes());
    tmac.update(parsed.backup_id.as_bytes());
    tmac.verify_slice(&trailer[48..80])
        .map_err(|_| KdataError::IntegrityMismatch("trailer MAC không hợp lệ".into()))?;

    let mut extra = [0u8; 1];
    if reader.read(&mut extra)? != 0 {
        return Err(KdataError::FormatViolation("phát hiện dữ liệu thừa sau footer".into()));
    }

    Ok(OpenedKdataArchive {
        header: parsed,
        manifest,
        index,
        frame_count,
        transcript_sha256_hex: hex::encode(expected_transcript),
    })
}
