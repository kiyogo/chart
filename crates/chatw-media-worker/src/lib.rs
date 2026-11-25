//! Dedicated Off-Main-Thread Release WASM Media Worker Engine (Spec 153)
//! Compiles with `--release` (`opt-level = 3`) and runs inside `/media_worker.js`
//! so AVIF encoding, PDQ 256-bit hashing, BLAKE3, and CWCMP1 compression NEVER block the UI thread.

use chatw_media_core::{
    avif::{
        AdaptiveAvifOutput, AvifCompressionMode, AvifEncodeOptions, encode_rgba_adaptive_avif,
        optimize_image_to_avif,
    },
    classifier::ImageClass,
    dedup::{compute_pdq_hash_rgba, hash_bytes_exact},
    files::{CompressionGoal, LosslessCodec, compress_file_smart},
};
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

fn parse_mode(mode: &str) -> AvifCompressionMode {
    match mode {
        "high" => AvifCompressionMode::HighQuality,
        "balanced" | "fast" => AvifCompressionMode::FastAdaptive,
        "aggressive" | "data_saver" => AvifCompressionMode::UltraAggressive,
        _ => AvifCompressionMode::UltraAdaptive,
    }
}

fn build_avif_result(res: AdaptiveAvifOutput) -> Object {
    let out = Object::new();
    let bytes = Uint8Array::from(res.avif_bytes.as_slice());
    let is_crisp = matches!(
        res.analysis.class,
        ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic
    );
    let super_res = res.width < res.target_width || res.height < res.target_height;
    let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
    let _ = Reflect::set(&out, &"bytes".into(), &bytes.into());
    let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
    let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
    let _ = Reflect::set(&out, &"targetWidth".into(), &JsValue::from(res.target_width));
    let _ = Reflect::set(&out, &"targetHeight".into(), &JsValue::from(res.target_height));
    let _ = Reflect::set(&out, &"superResRecommended".into(), &JsValue::from(super_res));
    let _ = Reflect::set(&out, &"isCrispText".into(), &JsValue::from(is_crisp));
    let _ = Reflect::set(
        &out,
        &"aiRecommended".into(),
        &JsValue::from(res.ai_restoration_recommended),
    );
    if let Some(ssim) = res.estimated_ssim {
        let _ = Reflect::set(&out, &"estimatedSsim".into(), &JsValue::from(ssim));
    }
    if let Some(ref prof) = res.orig_profile {
        if let Ok(js_prof) = serde_json::to_string(prof) {
            let _ = Reflect::set(&out, &"origProfileJson".into(), &JsValue::from(js_prof));
        }
    }
    out
}

fn error_obj(msg: &str) -> JsValue {
    let out = Object::new();
    let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
    let _ = Reflect::set(&out, &"error".into(), &JsValue::from(msg));
    out.into()
}

#[wasm_bindgen(js_name = encodeRgbaAvifAdaptive)]
pub fn encode_rgba_avif_adaptive(
    rgba: &[u8],
    width: u32,
    height: u32,
    original_encoded_size: usize,
    raw_file_bytes: &[u8],
    mode: &str,
) -> JsValue {
    let opts = AvifEncodeOptions {
        mode: parse_mode(mode),
        max_edge_override: None,
        min_ssim_floor: None,
        generate_thumbnail: false,
        threads: Some(1),
    };
    let raw_opt = if raw_file_bytes.is_empty() {
        None
    } else {
        Some(raw_file_bytes)
    };
    match encode_rgba_adaptive_avif(rgba, width, height, original_encoded_size, raw_opt, &opts) {
        Ok(res) => build_avif_result(res).into(),
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = encodeBytesAvifAdaptive)]
pub fn encode_bytes_avif_adaptive(raw_bytes: &[u8], mode: &str) -> JsValue {
    let opts = AvifEncodeOptions {
        mode: parse_mode(mode),
        max_edge_override: None,
        min_ssim_floor: None,
        generate_thumbnail: false,
        threads: Some(1),
    };
    match optimize_image_to_avif(raw_bytes, &opts) {
        Ok(res) => build_avif_result(res).into(),
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = computeImagePdq)]
pub fn compute_image_pdq(rgba: &[u8], width: u32, height: u32) -> JsValue {
    let pdq = compute_pdq_hash_rgba(rgba, width, height);
    let out = Object::new();
    let _ = Reflect::set(&out, &"pdqHex".into(), &JsValue::from(pdq.to_hex()));
    let _ = Reflect::set(&out, &"quality".into(), &JsValue::from(pdq.quality));
    let _ = Reflect::set(
        &out,
        &"captionBandHash".into(),
        &JsValue::from(pdq.caption_band_hash.to_string()),
    );
    let _ = Reflect::set(
        &out,
        &"colorMomentHash".into(),
        &JsValue::from(pdq.color_moment_hash.to_string()),
    );
    let _ = Reflect::set(
        &out,
        &"aspectRatioMilli".into(),
        &JsValue::from(pdq.aspect_ratio_milli),
    );
    out.into()
}

#[wasm_bindgen(js_name = blake3Hex)]
pub fn blake3_hex(bytes: &[u8]) -> String {
    hash_bytes_exact(bytes).blake3_hex
}

#[wasm_bindgen(js_name = compressFileSmart)]
pub fn compress_file_smart_worker(bytes: &[u8]) -> JsValue {
    let out = Object::new();
    match compress_file_smart(bytes, CompressionGoal::Balanced) {
        Ok(res) => {
            let out_bytes = Uint8Array::from(res.payload.as_slice());
            let compressed = res.codec != LosslessCodec::None;
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"compressed".into(), &JsValue::from(compressed));
            let _ = Reflect::set(&out, &"codec".into(), &JsValue::from(res.codec.name()));
            let _ = Reflect::set(&out, &"bytes".into(), &out_bytes.into());
            let _ = Reflect::set(&out, &"originalSize".into(), &JsValue::from(res.original_size as f64));
            let _ = Reflect::set(&out, &"compressedSize".into(), &JsValue::from(res.final_size as f64));
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}
