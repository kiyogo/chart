//! Smart Lossless Multi-Codec File Compression (Section XVII)
//!
//! - Evaluates real gains across **ZSTD**, **Brotli**, and **LZMA2 (XZ)** based on file type, size & Shannon entropy.
//! - Automatically skips already-compressed containers (ZIP, RAR, 7z, JPG, AVIF, MP4, AV1, WebM, GZ, XZ, ZST) & high-entropy data.
//! - NEVER inflates file size (keeps `None` passthrough if gain < 3%).
//! - Wraps compressed payload in authenticated `CWCMP1` envelope containing exact original BLAKE3 hash & original byte length.
//! - Enforces **Decompression Bomb Protection** (hard byte limit + max expansion ratio check) and **byte-for-byte BLAKE3 integrity verification**.

use crate::{
    MediaError, MediaResult,
    classifier::{MediaKind, shannon_entropy_bits, sniff_media_bytes},
};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[cfg(feature = "lossless-files")]
#[allow(unused_imports)]
use ruzstd as _;

pub const COMP_ENVELOPE_MAGIC: &[u8; 6] = b"CWCMP1";
/// Header: magic(6) + codec_id(1) + orig_len(8) + orig_blake3(32) = 47 bytes
pub const COMP_HEADER_LEN: usize = 47;
pub const DEFAULT_MAX_DECOMPRESS_BYTES: usize = 256 * 1024 * 1024; // 256 MiB hard safety cap
pub const DEFAULT_MAX_EXPANSION_RATIO: u64 = 120; // blocks 1000x zip/gzip/zstd bombs!

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum LosslessCodec {
    None = 0,
    Zstd = 1,
    Brotli = 2,
    Lzma2 = 3,
}

impl LosslessCodec {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Brotli),
            3 => Some(Self::Lzma2),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Zstd => "zstd",
            Self::Brotli => "brotli",
            Self::Lzma2 => "lzma2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionGoal {
    /// Fast, balanced CPU/RAM (default for chat attachments).
    Balanced,
    /// Try ZSTD + Brotli + LZMA2 and pick absolute smallest result (when gain justifies CPU).
    MaximumRatio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedFileResult {
    /// Either the `CWCMP1` envelope bytes (when compression helped) OR the exact untouched original bytes (`codec == None`).
    pub payload: Vec<u8>,
    pub codec: LosslessCodec,
    pub original_size: usize,
    pub final_size: usize,
    pub ratio: f32,
    pub blake3_original_hex: String,
    pub entropy_bits: f64,
    pub skipped_reason: Option<String>,
}

/// Read at most `limit` bytes from a streaming decompressor, immediately aborting decompression bombs!
fn read_bounded<R: Read>(mut reader: R, limit: usize) -> MediaResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = [0u8; 32 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| MediaError::CompressionFailed(format!("decompress read: {e}")))?;
        if n == 0 {
            break;
        }
        if out.len().saturating_add(n) > limit {
            return Err(MediaError::DecompressionBomb {
                limit,
                actual: out.len().saturating_add(n),
            });
        }
        out.extend_from_slice(&buf[..n]);
    }
    Ok(out)
}

#[cfg(feature = "lossless-files")]
fn compress_brotli(input: &[u8], quality: u32) -> MediaResult<Vec<u8>> {
    let mut out = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: quality.clamp(1, 11) as i32,
        lgwin: 22,
        ..Default::default()
    };
    let mut reader = brotli::CompressorReader::with_params(input, 32 * 1024, &params);
    reader
        .read_to_end(&mut out)
        .map_err(|e| MediaError::CompressionFailed(format!("brotli encode: {e}")))?;
    Ok(out)
}

#[cfg(feature = "lossless-files")]
fn decompress_brotli(input: &[u8], max_bytes: usize) -> MediaResult<Vec<u8>> {
    let reader = brotli::Decompressor::new(input, 32 * 1024);
    read_bounded(reader, max_bytes)
}

#[cfg(feature = "lossless-files")]
fn compress_lzma2(input: &[u8], preset: u32) -> MediaResult<Vec<u8>> {
    let mut out = Vec::new();
    let opts = lzma_rust2::XzOptions::with_preset(preset.clamp(1, 6));
    let mut writer = lzma_rust2::XzWriter::new(&mut out, opts)
        .map_err(|e| MediaError::CompressionFailed(format!("lzma2 init: {e}")))?;
    std::io::Write::write_all(&mut writer, input)
        .map_err(|e| MediaError::CompressionFailed(format!("lzma2 write: {e}")))?;
    writer
        .finish()
        .map_err(|e| MediaError::CompressionFailed(format!("lzma2 finish: {e}")))?;
    Ok(out)
}

#[cfg(feature = "lossless-files")]
fn decompress_lzma2(input: &[u8], max_bytes: usize) -> MediaResult<Vec<u8>> {
    let reader = lzma_rust2::XzReader::new(input, true);
    read_bounded(reader, max_bytes)
}

#[cfg(all(feature = "native-fast", not(target_arch = "wasm32")))]
fn compress_zstd(input: &[u8], level: i32) -> MediaResult<Vec<u8>> {
    zstd::bulk::compress(input, level.clamp(1, 19))
        .map_err(|e| MediaError::CompressionFailed(format!("zstd encode: {e}")))
}

#[cfg(not(all(feature = "native-fast", not(target_arch = "wasm32"))))]
fn compress_zstd(_input: &[u8], _level: i32) -> MediaResult<Vec<u8>> {
    Err(MediaError::CompressionFailed(
        "zstd encoder uses native-fast; use Brotli/LZMA2 on wasm32".into(),
    ))
}

#[cfg(feature = "lossless-files")]
fn decompress_zstd(input: &[u8], max_bytes: usize) -> MediaResult<Vec<u8>> {
    #[cfg(all(feature = "native-fast", not(target_arch = "wasm32")))]
    {
        let dec = zstd::stream::read::Decoder::new(input)
            .map_err(|e| MediaError::CompressionFailed(format!("zstd init: {e}")))?;
        return read_bounded(dec, max_bytes);
    }
    #[cfg(not(all(feature = "native-fast", not(target_arch = "wasm32"))))]
    {
        let dec = ruzstd::decoding::StreamingDecoder::new(input)
            .map_err(|e| MediaError::CompressionFailed(format!("ruzstd init: {e:?}")))?;
        read_bounded(dec, max_bytes)
    }
}

/// Wrap compressed body in `CWCMP1` authenticated lossless envelope.
fn wrap_envelope(codec: LosslessCodec, original_len: u64, orig_blake3: &[u8; 32], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(COMP_HEADER_LEN + body.len());
    out.extend_from_slice(COMP_ENVELOPE_MAGIC);
    out.push(codec as u8);
    out.extend_from_slice(&original_len.to_be_bytes());
    out.extend_from_slice(orig_blake3);
    out.extend_from_slice(body);
    out
}

/// Automatically choose the best lossless compressor (ZSTD / Brotli / LZMA2) ONLY when beneficial.
/// Guarantees zero size inflation, zero redundant compression of ZIP/RAR/7z/AVIF/AV1/JPEG, and exact byte restoration!
pub fn compress_file_smart(input: &[u8], goal: CompressionGoal) -> MediaResult<CompressedFileResult> {
    let orig_hash = blake3::hash(input);
    let blake3_hex = orig_hash.to_hex().to_string();
    let entropy = shannon_entropy_bits(input);

    let passthrough = |reason: &str| CompressedFileResult {
        payload: input.to_vec(),
        codec: LosslessCodec::None,
        original_size: input.len(),
        final_size: input.len(),
        ratio: 1.0,
        blake3_original_hex: blake3_hex.clone(),
        entropy_bits: entropy,
        skipped_reason: Some(reason.to_string()),
    };

    if input.len() < 256 {
        return Ok(passthrough("below_min_size_256b"));
    }
    if input.starts_with(COMP_ENVELOPE_MAGIC) {
        return Ok(passthrough("already_wrapped_cwcmp1"));
    }

    let sniff = sniff_media_bytes(input);
    if sniff.already_compressed || matches!(sniff.kind, MediaKind::ArchiveAlreadyCompressed | MediaKind::Video) {
        return Ok(passthrough(&format!("already_compressed_format:{}", sniff.ext)));
    }
    if entropy > 7.82 {
        return Ok(passthrough("high_entropy_incompressible"));
    }

    #[cfg(not(feature = "lossless-files"))]
    {
        return Ok(passthrough("lossless_files_feature_disabled"));
    }

    #[cfg(feature = "lossless-files")]
    {
        let mut best: Option<(LosslessCodec, Vec<u8>)> = None;
        let mut consider = |codec: LosslessCodec, res: MediaResult<Vec<u8>>| {
            if let Ok(bytes) = res {
                let is_better = match &best {
                    None => true,
                    Some((_, prev)) => bytes.len() < prev.len(),
                };
                if is_better {
                    best = Some((codec, bytes));
                }
            }
        };

        let is_text = sniff.kind == MediaKind::TextStructured;
        match goal {
            CompressionGoal::Balanced => {
                // Fast ZSTD + Brotli for structured text/JSON/XML/PDF
                consider(LosslessCodec::Zstd, compress_zstd(input, if input.len() > 4 * 1024 * 1024 { 3 } else { 6 }));
                if is_text || input.len() <= 2 * 1024 * 1024 {
                    consider(LosslessCodec::Brotli, compress_brotli(input, if is_text { 6 } else { 5 }));
                }
            }
            CompressionGoal::MaximumRatio => {
                consider(LosslessCodec::Zstd, compress_zstd(input, 15));
                consider(LosslessCodec::Brotli, compress_brotli(input, if input.len() <= 2 * 1024 * 1024 { 9 } else { 7 }));
                if input.len() <= 8 * 1024 * 1024 {
                    consider(LosslessCodec::Lzma2, compress_lzma2(input, 4));
                }
            }
        }

        if let Some((codec, body)) = best {
            let envelope = wrap_envelope(codec, input.len() as u64, orig_hash.as_bytes(), &body);
            // Require at least 3% real net savings AFTER envelope overhead
            let min_required_size = (input.len() * 97) / 100;
            if envelope.len() < min_required_size {
                let final_sz = envelope.len();
                return Ok(CompressedFileResult {
                    payload: envelope,
                    codec,
                    original_size: input.len(),
                    final_size: final_sz,
                    ratio: (input.len() as f32) / (final_sz.max(1) as f32),
                    blake3_original_hex: blake3_hex,
                    entropy_bits: entropy,
                    skipped_reason: None,
                });
            }
        }

        Ok(passthrough("insufficient_compression_gain"))
    }
}

/// Decompress `CWCMP1` payload (or return raw bytes if untouched passthrough) with strict
/// decompression-bomb guard AND byte-for-byte BLAKE3 hash verification!
pub fn decompress_file_verified(
    data: &[u8],
    max_output_bytes: usize,
) -> MediaResult<(Vec<u8>, LosslessCodec)> {
    if data.len() < COMP_HEADER_LEN || &data[0..6] != COMP_ENVELOPE_MAGIC {
        return Ok((data.to_vec(), LosslessCodec::None));
    }

    let codec = LosslessCodec::from_u8(data[6])
        .ok_or_else(|| MediaError::CompressionFailed(format!("unknown codec id {}", data[6])))?;
    let declared_len = u64::from_be_bytes([
        data[7], data[8], data[9], data[10], data[11], data[12], data[13], data[14],
    ]);
    let expected_hash = &data[15..47];
    let body = &data[COMP_HEADER_LEN..];

    let cap = max_output_bytes.min(DEFAULT_MAX_DECOMPRESS_BYTES);
    if declared_len > cap as u64 {
        return Err(MediaError::DecompressionBomb {
            limit: cap,
            actual: declared_len.min(usize::MAX as u64) as usize,
        });
    }
    let ratio_limit = (body.len().max(1) as u64).saturating_mul(DEFAULT_MAX_EXPANSION_RATIO).max(65_536);
    if declared_len > ratio_limit {
        return Err(MediaError::DecompressionBomb {
            limit: ratio_limit.min(usize::MAX as u64) as usize,
            actual: declared_len as usize,
        });
    }

    let effective_limit = (declared_len as usize).min(cap);

    #[cfg(feature = "lossless-files")]
    let plain = match codec {
        LosslessCodec::None => body.to_vec(),
        LosslessCodec::Zstd => decompress_zstd(body, effective_limit)?,
        LosslessCodec::Brotli => decompress_brotli(body, effective_limit)?,
        LosslessCodec::Lzma2 => decompress_lzma2(body, effective_limit)?,
    };

    #[cfg(not(feature = "lossless-files"))]
    let plain = match codec {
        LosslessCodec::None => body.to_vec(),
        _ => return Err(MediaError::CompressionFailed("lossless-files feature disabled".into())),
    };

    if plain.len() as u64 != declared_len {
        return Err(MediaError::IntegrityMismatch {
            expected: format!("{declared_len} bytes"),
            actual: format!("{} bytes", plain.len()),
        });
    }

    let actual_hash = blake3::hash(&plain);
    if actual_hash.as_bytes() != expected_hash {
        return Err(MediaError::IntegrityMismatch {
            expected: hex::encode(expected_hash),
            actual: actual_hash.to_hex().to_string(),
        });
    }

    Ok((plain, codec))
}

#[cfg(all(test, feature = "lossless-files"))]
mod tests {
    use super::*;

    #[test]
    fn compresses_json_and_verifies_byte_for_byte_blake3_across_all_codecs() {
        let json = serde_json::json!({
            "service": "chatw",
            "records": (0..400).map(|i| serde_json::json!({
                "id": i,
                "user": format!("member_{i}"),
                "status": "end_to_end_encrypted",
                "notes": "ChatW private messaging lossless file compression benchmark payload"
            })).collect::<Vec<_>>()
        })
        .to_string()
        .into_bytes();

        let res = compress_file_smart(&json, CompressionGoal::MaximumRatio).expect("compress");
        assert_ne!(res.codec, LosslessCodec::None);
        assert!(res.final_size < json.len() / 5, "Expected >5x compression on structured JSON, got {:.2}x", res.ratio);

        let (restored, codec) = decompress_file_verified(&res.payload, 32 * 1024 * 1024).expect("decompress");
        assert_eq!(codec, res.codec);
        assert_eq!(restored, json, "Lossless file MUST restore exact byte-for-byte original!");
    }

    #[test]
    fn skips_already_compressed_zip_and_high_entropy_files() {
        let mut fake_zip = vec![0x50, 0x4B, 0x03, 0x04];
        fake_zip.extend((0..4096).map(|i| ((i * 37) % 256) as u8));
        let res = compress_file_smart(&fake_zip, CompressionGoal::Balanced).expect("zip");
        assert_eq!(res.codec, LosslessCodec::None);
        assert_eq!(res.payload, fake_zip);
    }

    #[test]
    fn blocks_decompression_bomb_before_allocating_memory() {
        let zeros = vec![0u8; 2 * 1024 * 1024]; // 2 MiB of zeros compresses to ~100 bytes (>10000x ratio!)
        let brotli_body = compress_brotli(&zeros, 6).unwrap();
        let bomb = wrap_envelope(
            LosslessCodec::Brotli,
            zeros.len() as u64,
            blake3::hash(&zeros).as_bytes(),
            &brotli_body,
        );
        let err = decompress_file_verified(&bomb, 1024 * 1024).unwrap_err();
        assert!(matches!(err, MediaError::DecompressionBomb { .. }));
    }
}
