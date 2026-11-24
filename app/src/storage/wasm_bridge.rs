//! Pure-Rust WASM Media Codec Bridge (`window.__chatwWasmMedia`, Spec 110)
//! Exposes ChatW's shared `chatw-media-core` directly to JS via raw `Uint8Array` (zero Base64):
//! - `encodeAvifAdaptive(u8, mode)` -> genuine `ravif`/`rav1e` AVIF (NEVER silent JPEG/WebP/PNG!)
//! - `compressFileSmart(u8)` -> lossless `CWCMP1` (Brotli / LZMA2 / ZSTD with entropy skip)
//! - `decompressFileVerified(u8)` -> verified byte-for-byte BLAKE3 decompression with bomb guard
//! - `blake3Hex(u8)` -> fast BLAKE3 content hash

#[cfg(target_arch = "wasm32")]
pub fn install_wasm_media_bridge() {
    use chatw_media_core::{
        ai::cnn_sr::{CnnModelKind, raw_cnn_weights_bytes, run_cnn_sr_tiled},
        avif::{AvifCompressionMode, AvifEncodeOptions, optimize_image_to_avif},
        classifier::ImageClass,
        dedup::hash_bytes_exact,
        files::{
            COMP_ENVELOPE_MAGIC, CompressionGoal, LosslessCodec, compress_file_smart,
            decompress_file_verified,
        },
    };
    use js_sys::{Object, Reflect, Uint8Array};
    use std::sync::atomic::{AtomicBool, Ordering};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let Some(win) = web_sys::window() else {
        return;
    };

    let api = Object::new();

    // 1. encodeAvifAdaptive(u8: Uint8Array, mode?: string) -> { ok, bytes, width, height, targetWidth, targetHeight, superResRecommended, isCrispText, estimatedSsim, aiRecommended }
    let enc_cb = Closure::<dyn Fn(Uint8Array, JsValue) -> JsValue>::new(
        move |input: Uint8Array, mode_val: JsValue| {
            let raw = input.to_vec();
            let mode_str = mode_val.as_string().unwrap_or_else(|| "ultra".into());
            let mode = match mode_str.as_str() {
                "high" => AvifCompressionMode::HighQuality,
                "balanced" => AvifCompressionMode::FastAdaptive,
                "ultra" | "aggressive" | "fast" | _ => AvifCompressionMode::UltraAggressive,
            };
            let opts = AvifEncodeOptions {
                mode,
                max_edge_override: None,
                min_ssim_floor: None,
                generate_thumbnail: false,
                threads: Some(1),
            };
            let out = Object::new();
            match optimize_image_to_avif(&raw, &opts) {
                Ok(res) => {
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
                }
                Err(err) => {
                    let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                    let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err.to_string()));
                }
            }
            out.into()
        },
    );

    // 2. compressFileSmart(u8: Uint8Array) -> { ok, compressed, codec, bytes, originalSize, compressedSize }
    let cmp_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |input: Uint8Array| {
        let raw = input.to_vec();
        let out = Object::new();
        match compress_file_smart(&raw, CompressionGoal::Balanced) {
            Ok(res) => {
                let bytes = Uint8Array::from(res.payload.as_slice());
                let compressed = res.codec != LosslessCodec::None;
                let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                let _ = Reflect::set(&out, &"compressed".into(), &JsValue::from(compressed));
                let _ = Reflect::set(&out, &"codec".into(), &JsValue::from(res.codec.name()));
                let _ = Reflect::set(&out, &"bytes".into(), &bytes.into());
                let _ = Reflect::set(&out, &"originalSize".into(), &JsValue::from(res.original_size as f64));
                let _ = Reflect::set(&out, &"compressedSize".into(), &JsValue::from(res.final_size as f64));
            }
            Err(err) => {
                let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err.to_string()));
            }
        }
        out.into()
    });

    // 3. decompressFileVerified(u8: Uint8Array) -> { ok, wasEnvelope, bytes }
    let dec_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |input: Uint8Array| {
        let raw = input.to_vec();
        let out = Object::new();
        if !raw.starts_with(COMP_ENVELOPE_MAGIC) {
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"wasEnvelope".into(), &JsValue::FALSE);
            let _ = Reflect::set(&out, &"bytes".into(), &Uint8Array::from(raw.as_slice()).into());
            return out.into();
        }
        match decompress_file_verified(&raw, 256 * 1024 * 1024) {
            Ok((plain, _codec)) => {
                let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                let _ = Reflect::set(&out, &"wasEnvelope".into(), &JsValue::TRUE);
                let _ = Reflect::set(&out, &"bytes".into(), &Uint8Array::from(plain.as_slice()).into());
            }
            Err(err) => {
                let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err.to_string()));
            }
        }
        out.into()
    });

    // 4. blake3Hex(u8: Uint8Array) -> string
    let b3_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |input: Uint8Array| {
        let h = hash_bytes_exact(&input.to_vec());
        JsValue::from(h.blake3_hex)
    });

    // 5. restoreImageCnn(rgba: Uint8Array, width: u32, height: u32, modelKind?: string) -> { ok, rgba, width, height, scale, modelId }
    let cnn_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, model_val: JsValue| {
            let raw = rgba.to_vec();
            let model_str = model_val.as_string().unwrap_or_else(|| "auto".into());
            let kind = CnnModelKind::from_str_name(&model_str);
            let out = Object::new();
            match run_cnn_sr_tiled(&raw, width, height, kind, 192, None) {
                Ok(res) => {
                    let bytes = Uint8Array::from(res.rgba.as_slice());
                    let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                    let _ = Reflect::set(&out, &"rgba".into(), &bytes.into());
                    let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
                    let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
                    let _ = Reflect::set(&out, &"scale".into(), &JsValue::from(res.scale));
                    let _ = Reflect::set(&out, &"modelId".into(), &JsValue::from(res.model_id));
                }
                Err(err) => {
                    let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                    let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err.to_string()));
                }
            }
            out.into()
        },
    );

    // 6. getCnnWeights(modelKind?: string) -> Uint8Array (raw CWNN1 binary weights shared with WebGPU WGSL shaders)
    let weights_cb = Closure::<dyn Fn(JsValue) -> JsValue>::new(move |model_val: JsValue| {
        let model_str = model_val.as_string().unwrap_or_else(|| "anime".into());
        let kind = CnnModelKind::from_str_name(&model_str);
        Uint8Array::from(raw_cnn_weights_bytes(kind)).into()
    });

    let _ = Reflect::set(&api, &"version".into(), &JsValue::from("chatw-media-core-v2"));
    let _ = Reflect::set(&api, &"cnnEngine".into(), &JsValue::from("anime4k-crelu-8layer-v1"));
    let _ = Reflect::set(&api, &"encodeAvifAdaptive".into(), enc_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"compressFileSmart".into(), cmp_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"decompressFileVerified".into(), dec_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"blake3Hex".into(), b3_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"restoreImageCnn".into(), cnn_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"getCnnWeights".into(), weights_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&win, &"__chatwWasmMedia".into(), &api.into());

    enc_cb.forget();
    cmp_cb.forget();
    dec_cb.forget();
    b3_cb.forget();
    cnn_cb.forget();
    weights_cb.forget();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_wasm_media_bridge() {}
