//! ChatW Ultra Media Core (`chatw-media-core`)
//!
//! Portable Rust engine shared by ChatW backend (`web/backend`) and cross-platform clients (`app`: Web WASM, Desktop, Mobile).
//!
//! Capabilities:
//! - Magic-byte media classification, Shannon entropy analysis & 10-class image content analyzer
//! - Perceptual quality evaluation (PSNR, SSIM, MS-SSIM, Laplacian edge sharpness, Color Drift, Anti-Degradation Gate)
//! - Genuine Adaptive Ultra AVIF Encoder (`ravif` + `rav1e`, multi-candidate RD loop, Animated AVIF support, ZERO silent fallback to JPEG/PNG/WebP)
//! - Genuine AV1 Video Encoder (`rav1e`) + Motion/Scene analyzer + IVF & Fragmented MP4 (CMAF) muxer + ABR planner
//! - Lossless FLAC (`flacenc` + `claxon`) + Opus adaptive planner/parser + lightweight spectral speech denoiser
//! - Smart Lossless Multi-Codec File Compressor (ZSTD / Brotli / LZMA2) with `CWCMP1` authenticated envelope, decompression-bomb guard & byte-for-byte BLAKE3 verification
//! - Unified Chunked Streaming E2EE (`CWCH3` with media/variant/chunk AAD binding + authenticated manifest, 100% compatible with `CWCH2` & v1)
//! - Privacy-safe Smart Deduplication (BLAKE3, SHA-256, BSD-3 PDQ 256-bit DCT image hash, vPDQ video fingerprint, strict E2EE scope isolation)
//! - Intelligent Local Cache Engine (W-TinyLFU Count-Min Sketch + Segmented LRU + Cost-Aware & Size-Aware Eviction + Adaptive Capacity + AI Output Cache Key)
//! - On-Device AI Restoration (Hardware Detector, License-Enforced Model Registry, Smart Model Router, Overlapping Hann-Feathered Tiled Inference, Pure-Rust SPAB-Lite CNN, Real `tract-onnx` ONNX Runtime, Temporal Video AI Scheduler)

pub mod ai;
pub mod audio;
pub mod av1;
pub mod avif;
pub mod cache;
pub mod classifier;
pub mod dedup;
pub mod e2ee;
pub mod files;
pub mod quality;
pub mod streaming;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Strictly distinguishes the three data provenance classes (Section XXVI):
/// Original bytes MUST never be conflated with Optimized or AI-Enhanced outputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaProvenance {
    /// Exact unmodified byte-for-byte source file.
    Original,
    /// Lossy or adaptively encoded media (AVIF / Animated AVIF / AV1 / Opus).
    Optimized {
        codec: String,
        quality_ssim: Option<u32>, // scaled * 10000 for Eq
    },
    /// On-device perceptual restoration output. Never labeled as exact original bytes.
    AiEnhanced {
        model_id: String,
        model_version: String,
        backend: String,
    },
}

#[derive(Debug)]
pub enum MediaError {
    EmptyInput,
    UnsupportedFormat(String),
    DecodeFailed(String),
    AvifEncodeFailed(String),
    Av1EncodeFailed(String),
    AudioCodecFailed(String),
    CompressionFailed(String),
    DecompressionBomb { limit: usize, actual: usize },
    IntegrityMismatch { expected: String, actual: String },
    CryptoError(String),
    LicenseViolation { model_id: String, license: String },
    ModelNotFound(String),
    ResourceLimitExceeded(String),
    Cancelled,
    InvalidParameter(String),
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "empty media input"),
            Self::UnsupportedFormat(m) => write!(f, "unsupported media format: {m}"),
            Self::DecodeFailed(m) => write!(f, "media decode failed: {m}"),
            Self::AvifEncodeFailed(m) => write!(f, "genuine AVIF encoding failed: {m}"),
            Self::Av1EncodeFailed(m) => write!(f, "genuine AV1 encoding failed: {m}"),
            Self::AudioCodecFailed(m) => write!(f, "audio codec error: {m}"),
            Self::CompressionFailed(m) => write!(f, "lossless compression error: {m}"),
            Self::DecompressionBomb { limit, actual } => {
                write!(f, "decompression bomb blocked (limit {limit} bytes, exceeded at {actual})")
            }
            Self::IntegrityMismatch { expected, actual } => {
                write!(f, "integrity mismatch: expected {expected}, got {actual}")
            }
            Self::CryptoError(m) => write!(f, "e2ee crypto error: {m}"),
            Self::LicenseViolation { model_id, license } => {
                write!(f, "model '{model_id}' rejected by commercial/MIT license policy ({license})")
            }
            Self::ModelNotFound(id) => write!(f, "AI model not found in registry: {id}"),
            Self::ResourceLimitExceeded(m) => write!(f, "resource limit exceeded: {m}"),
            Self::Cancelled => write!(f, "media processing cancelled"),
            Self::InvalidParameter(m) => write!(f, "invalid parameter: {m}"),
        }
    }
}

impl std::error::Error for MediaError {}

pub type MediaResult<T> = Result<T, MediaError>;
