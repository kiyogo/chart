//! Intelligent On-Device AI Model Router (Section XI)
//!
//! Automatically chooses whether to enhance an image/video, which model & backend to run,
//! and tile/overlap dimensions based on:
//! 1. Image content class (text/QR/UI safety!)
//! 2. Compression level & image resolution
//! 3. Device hardware tier, RAM, GPU, battery, thermal state
//! 4. Whether an external ONNX model is already cached locally vs zero-download SPAB-Lite.

use crate::{
    ai::{
        hardware::{AccelerationBackend, DeviceTier, HardwareProfile},
        registry::ModelRegistry,
    },
    classifier::{ImageAnalysis, ImageClass},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub should_run_ai: bool,
    pub skip_reason: Option<String>,
    pub selected_model_id: String,
    pub selected_backend: AccelerationBackend,
    pub tile_size: u32,
    pub tile_overlap: u32,
    pub blend_strength: f32, // [0.0..=1.0] conservative blend preventing over-sharpening/identity drift
    pub max_concurrent_tasks: usize,
    pub is_text_sensitive: bool,
}

/// Route an incoming decoded image to the optimal on-device restoration plan (or clean `Skip`).
pub fn route_image_restoration(
    analysis: &ImageAnalysis,
    hw: &HardwareProfile,
    registry: &ModelRegistry,
    cached_onnx_models: &HashSet<String>,
    ai_enabled_by_user: bool,
    is_original_lossless: bool,
) -> RoutingDecision {
    let skip = |reason: &str| RoutingDecision {
        should_run_ai: false,
        skip_reason: Some(reason.to_string()),
        selected_model_id: "none".into(),
        selected_backend: hw.preferred_backend,
        tile_size: 256,
        tile_overlap: 16,
        blend_strength: 0.0,
        max_concurrent_tasks: 1,
        is_text_sensitive: analysis.has_text_risk,
    };

    if !ai_enabled_by_user {
        return skip("ai_disabled_by_user");
    }
    if is_original_lossless {
        return skip("source_is_exact_original_bytes");
    }
    if hw.thermal_throttled || hw.battery_percent.map(|b| b < 15).unwrap_or(false) {
        return skip("device_thermal_or_critical_battery_guard");
    }
    if analysis.width < 48 || analysis.height < 48 {
        return skip("image_too_small_icon_or_emoji");
    }
    // Protect crisp text, screenshots, QR codes & flat UI diagrams from hallucination!
    if matches!(analysis.class, ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic)
        && analysis.source_bpp > 0.65
    {
        return skip("crisp_text_or_ui_already_sharp_no_ai_hallucination");
    }

    // Determine tile size & concurrency strictly bounded to prevent OOM on mobile/browsers
    let (tile_size, tile_overlap, max_concurrent_tasks) = match hw.device_tier {
        DeviceTier::Constrained => (128, 12, 1),
        DeviceTier::Standard => (256, 16, 1),
        DeviceTier::HighEnd => {
            if hw.is_under_resource_pressure() {
                (192, 16, 1)
            } else {
                (384, 24, 2)
            }
        }
    };

    // Preferred external ONNX model order by content class & hardware tier
    let preferred_onnx: &[&str] = match analysis.class {
        ImageClass::AnimeManga | ImageClass::Meme => match hw.device_tier {
            DeviceTier::Constrained => &["span-x2-int8"],
            _ => &["real-cugan-anime-x2", "span-x2-int8"],
        },
        ImageClass::Portrait => match hw.device_tier {
            DeviceTier::HighEnd => &["gfpgan-v1.4-identity-safe", "realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Standard => &["realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Constrained => &["span-x2-int8"],
        },
        ImageClass::AlreadyHeavilyCompressed | ImageClass::LowLightNoisy => match hw.device_tier {
            DeviceTier::HighEnd => &["swinir-real-sr-x2", "realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Standard => &["realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Constrained => &["span-x2-int8"],
        },
        _ => match hw.device_tier {
            DeviceTier::HighEnd => &["hat-s-x2", "realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Standard => &["realesrgan-x2plus-fp16", "span-x2-int8"],
            DeviceTier::Constrained => &["span-x2-int8"],
        },
    };

    // Pick highest-priority licensed ONNX model that is ALREADY cached locally and fits RAM;
    // otherwise seamlessly use built-in zero-download SPAB-Lite neural restoration!
    let mut selected_model_id = "chatw-spab-lite-v1".to_string();
    for &cand_id in preferred_onnx {
        if cached_onnx_models.contains(cand_id) {
            if let Ok(desc) = registry.get(cand_id) {
                if hw.available_ram_mb == 0 || hw.available_ram_mb >= desc.min_ram_mb {
                    selected_model_id = desc.model_id.clone();
                    break;
                }
            }
        }
    }

    // Conservative blend strength prevents over-sharpening, ringing & facial identity drift!
    let blend_strength = match analysis.class {
        ImageClass::CrispTextUi | ImageClass::Meme => 0.25,
        ImageClass::Portrait => 0.35, // Identity preservation guard
        ImageClass::AnimeManga => 0.65,
        ImageClass::AlreadyHeavilyCompressed | ImageClass::LowLightNoisy => 0.72,
        _ => 0.55,
    };

    let selected_backend = if selected_model_id == "chatw-spab-lite-v1" {
        if hw.preferred_backend == AccelerationBackend::WebGpu {
            AccelerationBackend::WebGpu
        } else {
            AccelerationBackend::SpabLiteCpuSimd
        }
    } else {
        hw.preferred_backend
    };

    RoutingDecision {
        should_run_ai: true,
        skip_reason: None,
        selected_model_id,
        selected_backend,
        tile_size,
        tile_overlap,
        blend_strength,
        max_concurrent_tasks,
        is_text_sensitive: analysis.has_text_risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_analysis(class: ImageClass, has_text: bool) -> ImageAnalysis {
        ImageAnalysis {
            width: 1280,
            height: 720,
            class,
            has_alpha: false,
            is_hdr_candidate: false,
            sharp_edge_ratio: 0.03,
            flat_pixel_ratio: 0.5,
            noise_estimate: 0.1,
            mean_luma: 120.0,
            skin_tone_ratio: 0.05,
            color_saturation_mean: 0.3,
            palette_sparsity: 0.4,
            source_bpp: 0.85,
            has_text_risk: has_text,
            recommended_max_edge: None,
        }
    }

    #[test]
    fn skips_sharp_text_ui_and_respects_critical_battery() {
        let hw = HardwareProfile::detect_local();
        let reg = ModelRegistry::canonical();
        let empty = HashSet::new();

        let text_dec = route_image_restoration(&sample_analysis(ImageClass::CrispTextUi, true), &hw, &reg, &empty, true, false);
        assert!(!text_dec.should_run_ai);

        let mut low_bat = hw.clone();
        low_bat.battery_percent = Some(10);
        let bat_dec = route_image_restoration(&sample_analysis(ImageClass::Photo, false), &low_bat, &reg, &empty, true, false);
        assert!(!bat_dec.should_run_ai);
    }

    #[test]
    fn routes_anime_to_mit_real_cugan_when_cached_and_falls_back_to_builtin_spab_lite() {
        let mut hw = HardwareProfile::detect_local();
        hw.device_tier = DeviceTier::Standard;
        hw.available_ram_mb = 8192;
        let reg = ModelRegistry::canonical();

        let empty = HashSet::new();
        let d1 = route_image_restoration(&sample_analysis(ImageClass::AnimeManga, false), &hw, &reg, &empty, true, false);
        assert!(d1.should_run_ai);
        assert_eq!(d1.selected_model_id, "chatw-spab-lite-v1");

        let mut cached = HashSet::new();
        cached.insert("real-cugan-anime-x2".to_string());
        let d2 = route_image_restoration(&sample_analysis(ImageClass::AnimeManga, false), &hw, &reg, &cached, true, false);
        assert!(d2.should_run_ai);
        assert_eq!(d2.selected_model_id, "real-cugan-anime-x2");
    }
}
