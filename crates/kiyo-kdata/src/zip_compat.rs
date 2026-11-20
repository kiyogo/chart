use std::io::{Read, Seek, Write};

use sha2::{Digest, Sha256};
use zip::{
    AesMode, CompressionMethod, ZipArchive, ZipWriter,
    write::SimpleFileOptions,
};

use crate::{
    error::KdataError,
    model::{ImportLimits, sanitize_relative_path},
};

pub enum ZipEncryptionMode<'a> {
    UnencryptedAuthorized,
    Aes256(&'a str),
}

pub struct ZipEntryInput<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug)]
pub struct ExtractedZipEntry {
    pub path: String,
    pub bytes: Vec<u8>,
    pub sha256_hex: String,
}

/// Writes a ZIP archive using Deflate and either WinZip AE-2 AES-256 or authorized plaintext.
/// Legacy ZipCrypto is explicitly never used.
pub fn write_zip_archive<W: Write + Seek>(
    writer: W,
    entries: &[ZipEntryInput<'_>],
    mode: ZipEncryptionMode<'_>,
) -> Result<W, KdataError> {
    let mut zw = ZipWriter::new(writer);
    let base_opts = match mode {
        ZipEncryptionMode::UnencryptedAuthorized => {
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
        }
        ZipEncryptionMode::Aes256(password) => {
            if password.is_empty() {
                return Err(KdataError::WrongPasswordOrTampered);
            }
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(AesMode::Aes256, password)
        }
    };

    for item in entries {
        let clean = sanitize_relative_path(item.path)?;
        zw.start_file(&clean, base_opts.clone())
            .map_err(|e| KdataError::Zip(e.to_string()))?;
        zw.write_all(item.bytes)?;
    }

    zw.finish().map_err(|e| KdataError::Zip(e.to_string()))
}

/// Validates magic bytes (`PK\x03\x04` or empty archive `PK\x05\x06`), enforces path traversal
/// sanitization and runtime decompression ZIP-bomb guards (never trusting declared sizes).
pub fn read_zip_archive<R: Read + Seek>(
    mut reader: R,
    compressed_archive_bytes: u64,
    password: Option<&str>,
    limits: &ImportLimits,
) -> Result<Vec<ExtractedZipEntry>, KdataError> {
    let mut sig = [0u8; 4];
    reader.read_exact(&mut sig).map_err(|_| KdataError::FormatViolation("không phải tệp ZIP hợp lệ".into()))?;
    if &sig != b"PK\x03\x04" && &sig != b"PK\x05\x06" {
        return Err(KdataError::FormatViolation("sai chữ ký magic bytes của ZIP".into()));
    }
    reader.seek(std::io::SeekFrom::Start(0))?;

    let mut archive = ZipArchive::new(reader).map_err(|e| KdataError::Zip(e.to_string()))?;
    if archive.len() > limits.max_entries {
        return Err(KdataError::ResourceLimitExceeded("quá nhiều tệp trong ZIP".into()));
    }

    let mut out = Vec::new();
    let mut total_uncompressed = 0u64;
    let ratio_limit_bytes = compressed_archive_bytes
        .max(1024)
        .saturating_mul(limits.max_zip_expansion_ratio);

    for idx in 0..archive.len() {
        let file_res = match password {
            Some(pw) if !pw.is_empty() => archive.by_index_decrypt(idx, pw.as_bytes()),
            _ => archive.by_index(idx),
        };
        let mut file = file_res.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Password") || msg.contains("password") || msg.contains("decrypt") {
                KdataError::WrongPasswordOrTampered
            } else {
                KdataError::Zip(msg)
            }
        })?;

        if file.is_dir() {
            continue;
        }
        let clean_path = sanitize_relative_path(file.name())?;

        // Stream read with strict byte-by-byte cap against spoofed header sizes / ZIP bombs
        let mut buf = Vec::new();
        let mut chunk = [0u8; 32 * 1024];
        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            if (buf.len() as u64) + (n as u64) > limits.max_single_entry_bytes {
                return Err(KdataError::ResourceLimitExceeded(format!("tệp {clean_path} vượt giới hạn dung lượng")));
            }
            total_uncompressed = total_uncompressed
                .checked_add(n as u64)
                .ok_or_else(|| KdataError::ResourceLimitExceeded("overflow tổng dung lượng giải nén".into()))?;
            if total_uncompressed > limits.max_total_plain_bytes || total_uncompressed > ratio_limit_bytes {
                return Err(KdataError::ResourceLimitExceeded("phát hiện ZIP bomb hoặc tỷ lệ nén bất thường".into()));
            }
            buf.extend_from_slice(&chunk[..n]);
        }

        let sha256_hex = hex::encode(Sha256::digest(&buf));
        out.push(ExtractedZipEntry {
            path: clean_path,
            bytes: buf,
            sha256_hex,
        });
    }

    Ok(out)
}
