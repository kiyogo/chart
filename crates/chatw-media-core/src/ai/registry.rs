//! On-Device AI Model Registry with Strict SPDX Commercial/MIT License Enforcement
//! and SHA-256 Integrity Verification (Sections IX & X.4).
//!
//! Verified license audit:
//! - APPROVED (Permissive): SPAN (Apache-2.0), Real-ESRGAN (BSD-3-Clause), Real-CUGAN (MIT),
//!   SwinIR (Apache-2.0), HAT (Apache-2.0), GFPGAN (Apache-2.0), NanoVSR (MIT), FlashVSR (Apache-2.0),
//!   BasicVSR++ (Apache-2.0), Built-in ChatW SPAB-Lite (MIT).
//! - HARD-BLOCKED: CodeFormer (NTU S-Lab 1.0 Non-Commercial), RVRT (CC-BY-NC-4.0),
//!   APISR (GPL-3.0 copyleft), Upscayl UI/code (AGPL-3.0).

use crate::{
    MediaError, MediaResult,
    ai::hardware::{AccelerationBackend, DeviceTier},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDomain {
    PhotoGeneral,
    AnimeIllustration,
    FaceConservative,
    ArtifactDeblock,
    VideoTemporal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelQuantization {
    Fp32,
    Fp16,
    Int8,
    BuiltinAnalyticalWeights,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub model_id: String,
    pub version: String,
    pub sha256_hex: String,
    pub spdx_license: String,
    pub commercial_allowed: bool,
    pub source_repo: String,
    pub domain: ModelDomain,
    pub scale_factor: u8,
    pub model_size_bytes: u64,
    pub min_ram_mb: u64,
    pub recommended_ram_mb: u64,
    pub min_device_tier: DeviceTier,
    pub supported_backends: Vec<AccelerationBackend>,
    pub quantization: ModelQuantization,
    pub built_in: bool,
    pub notes: String,
}

const PERMITTED_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "CC-BY-4.0",
    "ISC",
    "CC0-1.0",
];

/// Hard license compliance gate — rejects Non-Commercial, GPL, and AGPL models before load/registration!
pub fn verify_model_license(model_id: &str, spdx_license: &str) -> MediaResult<()> {
    let clean = spdx_license.trim();
    if PERMITTED_LICENSES.iter().any(|allowed| allowed.eq_ignore_ascii_case(clean)) {
        Ok(())
    } else {
        Err(MediaError::LicenseViolation {
            model_id: model_id.to_string(),
            license: clean.to_string(),
        })
    }
}

/// Verify downloaded `.onnx` model bytes against the registry's pinned SHA-256 digest before execution!
pub fn verify_model_checksum(bytes: &[u8], expected_sha256_hex: &str) -> MediaResult<()> {
    if expected_sha256_hex.len() != 64 {
        return Err(MediaError::InvalidParameter("model checksum must be 64-char sha256 hex".into()));
    }
    let mut h = Sha256::new();
    h.update(bytes);
    let actual = hex::encode(h.finalize());
    if actual.eq_ignore_ascii_case(expected_sha256_hex) {
        Ok(())
    } else {
        Err(MediaError::IntegrityMismatch {
            expected: expected_sha256_hex.to_lowercase(),
            actual,
        })
    }
}

/// Canonical ChatW Model Registry (lazy-downloaded on demand; built-in SPAB-Lite works offline with 0 MB download).
pub struct ModelRegistry {
    models: Vec<ModelDescriptor>,
    blocked_noncommercial: Vec<(&'static str, &'static str, &'static str)>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::canonical()
    }
}

impl ModelRegistry {
    pub fn canonical() -> Self {
        let models = vec![
            ModelDescriptor {
                model_id: "chatw-spab-lite-v1".into(),
                version: "1.0.0".into(),
                sha256_hex: "0000000000000000000000000000000000000000000000000000000000000000".into(),
                spdx_license: "MIT".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/kiyogo/chart".into(),
                domain: ModelDomain::ArtifactDeblock,
                scale_factor: 1,
                model_size_bytes: 0,
                min_ram_mb: 256,
                recommended_ram_mb: 1024,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::SpabLiteCpuSimd,
                ],
                quantization: ModelQuantization::BuiltinAnalyticalWeights,
                built_in: true,
                notes: "Zero-download built-in SPAB-Lite residual attention restoration (inspired by Apache-2.0 SPAN). Runs via WebGPU WGSL or Rust SIMD.".into(),
            },
            ModelDescriptor {
                model_id: "clearreality_span_x4".into(),
                version: "1.0-span".into(),
                sha256_hex: "10306bafd8f8168cf06b8433a76fbe59639169c3e036d14f28e49517531ed698".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://huggingface.co/Kim2091/ClearRealityV1".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 4,
                model_size_bytes: 852_596,
                min_ram_mb: 512,
                recommended_ram_mb: 2048,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::CoreMl,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::NnApi,
                    AccelerationBackend::Qnn,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: true,
                notes: "4x-ClearRealityV1 SPAN (6xSPAB 48ch, CWSP1 FP16 833 KB). Studio 4K hair/eyelashes on clean/high-res AVIF photos.".into(),
            },
            ModelDescriptor {
                model_id: "nomosuni_span_x4".into(),
                version: "1.0-multijpg".into(),
                sha256_hex: "36efb817e89a303e861eefcf5e518fc409250b6377ad8d2834a0dfa1547785fc".into(),
                spdx_license: "CC-BY-4.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/Phhofm/models/releases/tag/4xNomosUni_span_multijpg".into(),
                domain: ModelDomain::ArtifactDeblock,
                scale_factor: 4,
                model_size_bytes: 852_596,
                min_ram_mb: 512,
                recommended_ram_mb: 2048,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::CoreMl,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::NnApi,
                    AccelerationBackend::Qnn,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: true,
                notes: "4xNomosUni_span_multijpg SPAN (CWSP1 FP16 833 KB). Eliminates heavy AVIF/JPEG ringing & crosshatch on low-res compressed chat photos.".into(),
            },
            ModelDescriptor {
                model_id: "lsdir_compact3_x4".into(),
                version: "3.0-c75r25".into(),
                sha256_hex: "cdca85df9b05cd669cce84f34a06c932f2531732c91f6a9d322b40d2ecc981e8".into(),
                spdx_license: "CC-BY-4.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/Phhofm/models".into(),
                domain: ModelDomain::ArtifactDeblock,
                scale_factor: 4,
                model_size_bytes: 1_242_872,
                min_ram_mb: 1024,
                recommended_ram_mb: 2048,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: true,
                notes: "4xLSDIRCompact3 SRVGGNetCompact (CWSR1 FP16 1.18 MB) trained on 85K LSDIR real photos.".into(),
            },
            ModelDescriptor {
                model_id: "realesr_animevideov3".into(),
                version: "0.2.5.0".into(),
                sha256_hex: "41e7d3494aa3af4371700598cdcf060e2d4e0bbfc236d69421286c9ad363666f".into(),
                spdx_license: "BSD-3-Clause".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/xinntao/Real-ESRGAN".into(),
                domain: ModelDomain::AnimeIllustration,
                scale_factor: 4,
                model_size_bytes: 1_242_872,
                min_ram_mb: 1024,
                recommended_ram_mb: 2048,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::CoreMl,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::NnApi,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: true,
                notes: "Official Real-ESRGAN AnimeVideo v3 (CWSR1 FP16 1.18 MB) for crisp flat anime/manga line art.".into(),
            },
            ModelDescriptor {
                model_id: "realesr_general_x4v3".into(),
                version: "0.2.5.0-dni35".into(),
                sha256_hex: "e536b660fce9ceb738cbd9d1ee0690afc30fcd582dc2b28f09dbdc2781472fff".into(),
                spdx_license: "BSD-3-Clause".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/xinntao/Real-ESRGAN".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 4,
                model_size_bytes: 2_426_616,
                min_ram_mb: 1024,
                recommended_ram_mb: 4096,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: true,
                notes: "Official Real-ESRGAN General x4v3 DNI-35 (CWSR1 FP16 2.31 MB) lightweight fast general upscaler.".into(),
            },
            ModelDescriptor {
                model_id: "realesrgan_x4plus".into(),
                version: "0.1.0".into(),
                sha256_hex: "pending-lazy-onnx-pin".into(),
                spdx_license: "BSD-3-Clause".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/xinntao/Real-ESRGAN".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 4,
                model_size_bytes: 33_500_000,
                min_ram_mb: 4096,
                recommended_ram_mb: 8192,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Cuda,
                    AccelerationBackend::Metal,
                    AccelerationBackend::CoreMl,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "High-capacity 23-RRDB Real-ESRGAN x4plus (lazy-downloaded ONNX/NCNN FP16 for high-end desktops).".into(),
            },
            ModelDescriptor {
                model_id: "realesrgan_anime_6b".into(),
                version: "0.2.2.4".into(),
                sha256_hex: "pending-lazy-onnx-pin".into(),
                spdx_license: "BSD-3-Clause".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/xinntao/Real-ESRGAN".into(),
                domain: ModelDomain::AnimeIllustration,
                scale_factor: 4,
                model_size_bytes: 8_900_000,
                min_ram_mb: 2048,
                recommended_ram_mb: 4096,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::NcnnVulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::CoreMl,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "6-RRDB Real-ESRGAN Anime 6B (lazy-downloaded ONNX/NCNN FP16) for detailed anime & manga illustrations.".into(),
            },
            ModelDescriptor {
                model_id: "scunet-denoise-x1".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-lazy-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/cszn/SCUNet".into(),
                domain: ModelDomain::ArtifactDeblock,
                scale_factor: 1,
                model_size_bytes: 36_000_000,
                min_ram_mb: 6144,
                recommended_ram_mb: 12288,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Cuda,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "SCUNet blind denoiser for severely degraded low-light noisy photos (lazy-downloaded desktop ONNX).".into(),
            },
            ModelDescriptor {
                model_id: "fsrcnn-x2-fast".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-lazy-onnx-pin".into(),
                spdx_license: "MIT".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/igv/FSRCNN-TensorFlow".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 2,
                model_size_bytes: 180_000,
                min_ram_mb: 256,
                recommended_ram_mb: 512,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::WebGl2,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::NnApi,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Int8,
                built_in: false,
                notes: "Ultra-lightweight FSRCNN 2x fallback for constrained low-RAM / low-battery devices.".into(),
            },
            ModelDescriptor {
                model_id: "span-x2-int8".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/hongyuanyu/SPAN".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 2,
                model_size_bytes: 2_100_000,
                min_ram_mb: 1024,
                recommended_ram_mb: 2048,
                min_device_tier: DeviceTier::Constrained,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::OnnxCpuSimd,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::NnApi,
                    AccelerationBackend::CoreMl,
                ],
                quantization: ModelQuantization::Int8,
                built_in: false,
                notes: "Primary ultra-fast super-resolution model for mobile, web & constrained laptops (NTIRE 2024 winner).".into(),
            },
            ModelDescriptor {
                model_id: "real-cugan-anime-x2".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "MIT".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/nihui/realcugan-ncnn-vulkan".into(),
                domain: ModelDomain::AnimeIllustration,
                scale_factor: 2,
                model_size_bytes: 5_400_000,
                min_ram_mb: 2048,
                recommended_ram_mb: 4096,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "MIT-licensed anime/manga line-art denoiser & restoration (replaces GPL-3.0 APISR for MIT compatibility).".into(),
            },
            ModelDescriptor {
                model_id: "realesrgan-x2plus-fp16".into(),
                version: "0.2.5".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "BSD-3-Clause".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/xinntao/Real-ESRGAN".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 2,
                model_size_bytes: 16_800_000,
                min_ram_mb: 4096,
                recommended_ram_mb: 8192,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Balanced photo restoration & artifact removal for standard laptops & desktops.".into(),
            },
            ModelDescriptor {
                model_id: "swinir-real-sr-x2".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/JingyunLiang/SwinIR".into(),
                domain: ModelDomain::ArtifactDeblock,
                scale_factor: 2,
                model_size_bytes: 24_000_000,
                min_ram_mb: 6144,
                recommended_ram_mb: 12288,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Transformer restoration for high-end desktops; strong compression artifact suppression.".into(),
            },
            ModelDescriptor {
                model_id: "hat-s-x2".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/XPixelGroup/HAT".into(),
                domain: ModelDomain::PhotoGeneral,
                scale_factor: 2,
                model_size_bytes: 38_000_000,
                min_ram_mb: 8192,
                recommended_ram_mb: 16384,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Hybrid Attention Transformer for top-tier desktop GPUs when VRAM & thermal headroom allow.".into(),
            },
            ModelDescriptor {
                model_id: "gfpgan-v1.4-identity-safe".into(),
                version: "1.4.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/TencentARC/GFPGAN".into(),
                domain: ModelDomain::FaceConservative,
                scale_factor: 1,
                model_size_bytes: 42_000_000,
                min_ram_mb: 6144,
                recommended_ram_mb: 12288,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Conservative portrait restoration blended <= 35% with source to prevent facial identity drift.".into(),
            },
            ModelDescriptor {
                model_id: "nanovsr-x2-temporal".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "MIT".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/filippawlicki/nanovsr".into(),
                domain: ModelDomain::VideoTemporal,
                scale_factor: 2,
                model_size_bytes: 1_800_000,
                min_ram_mb: 2048,
                recommended_ram_mb: 4096,
                min_device_tier: DeviceTier::Standard,
                supported_backends: vec![
                    AccelerationBackend::WebGpu,
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                    AccelerationBackend::OnnxCpuSimd,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Real-time temporal video super-resolution (MIT) with cross-frame consistency.".into(),
            },
            ModelDescriptor {
                model_id: "flashvsr-x2-temporal".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/OpenImagingLab/FlashVSR".into(),
                domain: ModelDomain::VideoTemporal,
                scale_factor: 2,
                model_size_bytes: 18_000_000,
                min_ram_mb: 8192,
                recommended_ram_mb: 16384,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "Fast temporal video restoration for high-end desktops.".into(),
            },
            ModelDescriptor {
                model_id: "basicvsrpp-x2-temporal".into(),
                version: "1.0.0".into(),
                sha256_hex: "pending-official-onnx-pin".into(),
                spdx_license: "Apache-2.0".into(),
                commercial_allowed: true,
                source_repo: "https://github.com/ckkelvinchan/BasicVSR_PlusPlus".into(),
                domain: ModelDomain::VideoTemporal,
                scale_factor: 2,
                model_size_bytes: 29_000_000,
                min_ram_mb: 12288,
                recommended_ram_mb: 24576,
                min_device_tier: DeviceTier::HighEnd,
                supported_backends: vec![
                    AccelerationBackend::Vulkan,
                    AccelerationBackend::Metal,
                    AccelerationBackend::DirectMl,
                ],
                quantization: ModelQuantization::Fp16,
                built_in: false,
                notes: "High-fidelity multi-frame video restoration for desktop workstations.".into(),
            },
        ];

        Self {
            models,
            blocked_noncommercial: vec![
                ("codeformer", "NTU-SLab-1.0-NonCommercial", "Non-commercial research license only"),
                ("rvrt", "CC-BY-NC-4.0", "Non-commercial license only"),
                ("apisr", "GPL-3.0", "Incompatible copyleft license for MIT ChatW release; use Real-CUGAN (MIT)"),
                ("upscayl", "AGPL-3.0", "Incompatible strong copyleft; architectural tiling concepts only, zero code reused"),
            ],
        }
    }

    pub fn list(&self) -> &[ModelDescriptor] {
        &self.models
    }

    pub fn blocked_models(&self) -> &[(&'static str, &'static str, &'static str)] {
        &self.blocked_noncommercial
    }

    pub fn get(&self, model_id: &str) -> MediaResult<&ModelDescriptor> {
        let clean = model_id.trim();
        // Resolve Upscayl architecture aliases to permissive open models rewritten in Rust
        if clean.eq_ignore_ascii_case("upscayl-standard") || clean.eq_ignore_ascii_case("upscayl-photo") {
            return self.get("realesrgan-x2plus-fp16");
        }
        if clean.eq_ignore_ascii_case("upscayl-digital-art") || clean.eq_ignore_ascii_case("upscayl-anime") {
            return self.get("real-cugan-anime-x2");
        }
        if clean.eq_ignore_ascii_case("upscayl-lite") || clean.eq_ignore_ascii_case("upscayl-fast") {
            return self.get("span-x2-int8");
        }
        if let Some((_, lic, _)) = self
            .blocked_noncommercial
            .iter()
            .find(|(id, _, _)| id.eq_ignore_ascii_case(clean))
        {
            return Err(MediaError::LicenseViolation {
                model_id: model_id.to_string(),
                license: (*lic).to_string(),
            });
        }
        self.models
            .iter()
            .find(|m| m.model_id == clean)
            .ok_or_else(|| MediaError::ModelNotFound(model_id.to_string()))
    }

    pub fn register_custom(&mut self, desc: ModelDescriptor) -> MediaResult<()> {
        verify_model_license(&desc.model_id, &desc.spdx_license)?;
        if !desc.commercial_allowed {
            return Err(MediaError::LicenseViolation {
                model_id: desc.model_id,
                license: desc.spdx_license,
            });
        }
        self.models.retain(|m| m.model_id != desc.model_id);
        self.models.push(desc);
        Ok(())
    }

    /// Merge a slice of `ModelDescriptor` entries after strict commercial license verification.
    pub fn merge_descriptors(&mut self, entries: Vec<ModelDescriptor>) -> MediaResult<usize> {
        let mut added = 0usize;
        for desc in entries {
            self.register_custom(desc)?;
            added += 1;
        }
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_gate_blocks_codeformer_rvrt_apisr_upscayl_and_approves_permissive_models() {
        let reg = ModelRegistry::canonical();
        assert!(matches!(reg.get("codeformer"), Err(MediaError::LicenseViolation { .. })));
        assert!(matches!(reg.get("rvrt"), Err(MediaError::LicenseViolation { .. })));
        assert!(matches!(reg.get("apisr"), Err(MediaError::LicenseViolation { .. })));
        assert!(matches!(reg.get("upscayl"), Err(MediaError::LicenseViolation { .. })));

        assert!(reg.get("chatw-spab-lite-v1").is_ok());
        assert!(reg.get("span-x2-int8").is_ok());
        assert!(reg.get("real-cugan-anime-x2").is_ok());
        assert!(reg.get("realesrgan-x2plus-fp16").is_ok());
        assert!(reg.get("nanovsr-x2-temporal").is_ok());
    }
}
