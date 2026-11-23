//! Unified On-Device AI Restoration Orchestrator (Sections VIII, IX, X, XI, XII, XV).

pub mod hardware;
pub mod onnx_runtime;
pub mod registry;
pub mod router;
pub mod spab_lite;
pub mod tiling;
pub mod video_ai;

use crate::{
    MediaProvenance, MediaResult,
    ai::{
        hardware::HardwareProfile,
        registry::ModelRegistry,
        router::{RoutingDecision, route_image_restoration},
        spab_lite::{SpabLiteParams, infer_spab_lite_tile},
        tiling::run_tiled_rgba,
    },
    classifier::{ImageClass, analyze_rgba_image},
    quality::{QualityReport, ai_output_passes_quality_gate},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::atomic::AtomicBool};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RestorationOutcome {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub applied_ai: bool,
    pub provenance: MediaProvenance,
    pub routing: RoutingDecision,
    pub quality_report: Option<QualityReport>,
}

/// Complete on-device image restoration pipeline:
/// Analyze -> Route -> Overlapping Hann-Feathered Tiled SPAB-Lite Inference -> Anti-Degradation Quality Gate -> Provenance.
/// Automatically falls back to the clean decoded image if AI is unnecessary or would degrade text/colors/edges!
pub fn restore_rgba_on_device(
    decoded_rgba: &[u8],
    width: u32,
    height: u32,
    encoded_byte_size: usize,
    hw: &HardwareProfile,
    ai_enabled: bool,
    is_original_lossless: bool,
    cancel: Option<&AtomicBool>,
) -> MediaResult<RestorationOutcome> {
    let analysis = analyze_rgba_image(decoded_rgba, width, height, encoded_byte_size, None);
    let registry = ModelRegistry::canonical();
    let cached_onnx = HashSet::new();
    let routing = route_image_restoration(&analysis, hw, &registry, &cached_onnx, ai_enabled, is_original_lossless);

    if !routing.should_run_ai {
        return Ok(RestorationOutcome {
            rgba: decoded_rgba.to_vec(),
            width,
            height,
            applied_ai: false,
            provenance: if is_original_lossless {
                MediaProvenance::Original
            } else {
                MediaProvenance::Optimized {
                    codec: "avif".into(),
                    quality_ssim: None,
                }
            },
            routing,
            quality_report: None,
        });
    }

    let params = match analysis.class {
        ImageClass::AlreadyHeavilyCompressed | ImageClass::LowLightNoisy => SpabLiteParams {
            strength: routing.blend_strength,
            deblock_sigma: 0.095,
            detail_gain: 0.45,
        },
        ImageClass::AnimeManga => SpabLiteParams {
            strength: routing.blend_strength,
            deblock_sigma: 0.075,
            detail_gain: 0.50,
        },
        ImageClass::Portrait => SpabLiteParams {
            strength: routing.blend_strength,
            deblock_sigma: 0.055,
            detail_gain: 0.28, // Conservative detail gain to protect natural facial features
        },
        ImageClass::CrispTextUi | ImageClass::Meme => SpabLiteParams {
            strength: routing.blend_strength.min(0.25),
            deblock_sigma: 0.035,
            detail_gain: 0.20,
        },
        _ => SpabLiteParams {
            strength: routing.blend_strength,
            deblock_sigma: 0.065,
            detail_gain: 0.42,
        },
    };

    let (enhanced, ow, oh) = run_tiled_rgba(
        decoded_rgba,
        width,
        height,
        routing.tile_size,
        routing.tile_overlap,
        1,
        cancel,
        |tile, tw, th| infer_spab_lite_tile(tile, tw, th, params),
    )?;

    let (passed, report) = ai_output_passes_quality_gate(
        decoded_rgba,
        &enhanced,
        ow,
        oh,
        routing.is_text_sensitive,
    );

    if !passed {
        // Anti-Degradation Gate: NEVER display an AI image worse than the decoded AVIF!
        return Ok(RestorationOutcome {
            rgba: decoded_rgba.to_vec(),
            width,
            height,
            applied_ai: false,
            provenance: MediaProvenance::Optimized {
                codec: "avif".into(),
                quality_ssim: Some((report.ssim * 10_000.0) as u32),
            },
            routing,
            quality_report: Some(report),
        });
    }

    Ok(RestorationOutcome {
        rgba: enhanced,
        width: ow,
        height: oh,
        applied_ai: true,
        provenance: MediaProvenance::AiEnhanced {
            model_id: routing.selected_model_id.clone(),
            model_version: "1.0.0".into(),
            backend: format!("{:?}", routing.selected_backend),
        },
        routing,
        quality_report: Some(report),
    })
}
