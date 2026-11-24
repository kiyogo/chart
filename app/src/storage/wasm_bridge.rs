//! Pure-Rust WASM Media Codec Bridge (`window.__chatwWasmMedia`, Spec 110)
//! Exposes ChatW's shared `chatw-media-core` directly to JS via raw `Uint8Array` (zero Base64):
//! - `encodeAvifAdaptive(u8, mode)` -> genuine `ravif`/`rav1e` AVIF (NEVER silent JPEG/WebP/PNG!)
//! - `compressFileSmart(u8)` -> lossless `CWCMP1` (Brotli / LZMA2 / ZSTD with entropy skip)
//! - `decompressFileVerified(u8)` -> verified byte-for-byte BLAKE3 decompression with bomb guard
//! - `blake3Hex(u8)` -> fast BLAKE3 content hash

#[cfg(target_arch = "wasm32")]
pub fn install_wasm_media_bridge() {
    use chatw_media_core::{
        ai::cnn_sr::{
            CnnModelKind, OriginalQualityProfile, calibrate_to_original_profile_with_base,
            extract_original_quality_profile, raw_cnn_weights_bytes, restore_guided_hq,
            run_cnn_sr_tiled, upscale_catmull_rom_2x,
        },
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
                    if let Some(ref prof) = res.orig_profile {
                        if let Ok(js_prof) = serde_json::to_string(prof) {
                            let _ = Reflect::set(&out, &"origProfileJson".into(), &JsValue::from(js_prof));
                        }
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
        let model_str = model_val.as_string().unwrap_or_else(|| "anime_l".into());
        let kind = CnnModelKind::from_str_name(&model_str);
        Uint8Array::from(raw_cnn_weights_bytes(kind)).into()
    });

    // 7. restoreImageGuided(rgba, w, h, want2x, preRestore, origProfileJson) -> { ok, rgba, width, height, scale, modelId, modelLabel, matchPct }
    let guided_cb = Closure::<dyn Fn(Uint8Array, u32, u32, bool, bool, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, want_2x: bool, pre_restore: bool, prof_val: JsValue| {
            let raw = rgba.to_vec();
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());
            let out = Object::new();
            match restore_guided_hq(&raw, width, height, want_2x, pre_restore, orig_prof.as_ref(), 192, None) {
                Ok(res) => {
                    let bytes = Uint8Array::from(res.rgba.as_slice());
                    let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                    let _ = Reflect::set(&out, &"rgba".into(), &bytes.into());
                    let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
                    let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
                    let _ = Reflect::set(&out, &"scale".into(), &JsValue::from(res.scale));
                    let _ = Reflect::set(&out, &"modelId".into(), &JsValue::from(res.model_id));
                    let _ = Reflect::set(&out, &"modelLabel".into(), &JsValue::from(res.model_label));
                    let _ = Reflect::set(&out, &"matchPct".into(), &JsValue::from(res.match_pct));
                }
                Err(err) => {
                    let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                    let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err.to_string()));
                }
            }
            out.into()
        },
    );

    // 8. calibrateToProfile(rgba, w, h, origProfileJson, srcRgba?, srcW?, srcH?) -> { ok, rgba, matchPct }
    let calib_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue, JsValue, JsValue, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, prof_val: JsValue, src_val: JsValue, src_w_val: JsValue, src_h_val: JsValue| {
            let mut buf = rgba.to_vec();
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());
            let base_buf: Option<Vec<u8>> = if !src_val.is_undefined() && !src_val.is_null() {
                let sw = src_w_val.as_f64().unwrap_or(0.0) as u32;
                let sh = src_h_val.as_f64().unwrap_or(0.0) as u32;
                let src_u8 = Uint8Array::new(&src_val).to_vec();
                if sw * 2 == width && sh * 2 == height && src_u8.len() == (sw as usize * sh as usize * 4) {
                    Some(upscale_catmull_rom_2x(&src_u8, sw, sh))
                } else if sw == width && sh == height && src_u8.len() == buf.len() {
                    Some(src_u8)
                } else {
                    None
                }
            } else {
                None
            };
            let match_pct = calibrate_to_original_profile_with_base(
                &mut buf,
                base_buf.as_deref(),
                width,
                height,
                orig_prof.as_ref(),
            );
            let out = Object::new();
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"rgba".into(), &Uint8Array::from(buf.as_slice()).into());
            let _ = Reflect::set(&out, &"matchPct".into(), &JsValue::from(match_pct));
            out.into()
        },
    );

    // 9. extractQualityProfile(rgba, w, h) -> string (JSON OriginalQualityProfile)
    let prof_cb = Closure::<dyn Fn(Uint8Array, u32, u32) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32| {
            let prof = extract_original_quality_profile(&rgba.to_vec(), width, height);
            JsValue::from(serde_json::to_string(&prof).unwrap_or_default())
        },
    );

    let _ = Reflect::set(&api, &"version".into(), &JsValue::from("chatw-media-core-v2"));
    let _ = Reflect::set(&api, &"cnnEngine".into(), &JsValue::from("anime4k-crelu-15layer-guided-v4"));
    let _ = Reflect::set(&api, &"encodeAvifAdaptive".into(), enc_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"compressFileSmart".into(), cmp_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"decompressFileVerified".into(), dec_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"blake3Hex".into(), b3_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"restoreImageCnn".into(), cnn_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"getCnnWeights".into(), weights_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"restoreImageGuided".into(), guided_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"calibrateToProfile".into(), calib_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"extractQualityProfile".into(), prof_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&win, &"__chatwWasmMedia".into(), &api.into());

    enc_cb.forget();
    cmp_cb.forget();
    dec_cb.forget();
    b3_cb.forget();
    cnn_cb.forget();
    weights_cb.forget();
    guided_cb.forget();
    calib_cb.forget();
    prof_cb.forget();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_wasm_media_bridge() {}
