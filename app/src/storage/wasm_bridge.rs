//! Pure-Rust WASM Media Codec & Real-ESRGAN GPU Bridge (`window.__chatwWasmMedia`, Spec 110)
//! Exposes ChatW's 100% Pure-Rust media core + Real-ESRGAN / Upscayl GPU engine directly to thin JS DOM bindings:
//! - `encodeAvifAdaptive(u8, mode)` -> genuine 10-bit `ravif`/`rav1e` AVIF + `OriginalQualityProfile`
//! - `compressFileSmart(u8)` / `decompressFileVerified(u8)` -> lossless `CWCMP1` (Brotli / LZMA2 / ZSTD)
//! - `blake3Hex(u8)` -> fast BLAKE3 content hash
//! - `probeRealEsrganGpu()` -> probes Rust WebGL2 GPU engine status
//! - `selectRealEsrganModel(rgba, w, h, origProfileJson)` -> chooses Anime v3 vs Upscayl-Lite General v3 in Rust
//! - `restoreImageRealEsrgan(rgba, w, h, modelId, modelBytesCwsr1, origProfileJson)` -> async 100% Pure-Rust Real-ESRGAN 4×!
//! - `isModelLicenseAllowed(modelId)` -> strict SPDX commercial license gate in Rust

#[cfg(target_arch = "wasm32")]
pub fn install_wasm_media_bridge() {
    use crate::storage::realesrgan_gpu::{probe_gpu, restore_image_realesrgan_rust};
    use chatw_media_core::{
        ai::realesrgan::{
            OriginalQualityProfile, RealEsrganModelKind, extract_original_quality_profile,
            select_realesrgan_model,
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
    use wasm_bindgen_futures::future_to_promise;

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let Some(win) = web_sys::window() else {
        return;
    };
    let api = Object::new();

    // 1. encodeAvifAdaptive(rawBytesU8, mode?: string) -> { ok, bytes, width, height, targetWidth, targetHeight, origProfileJson, ... }
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
                    let _ = Reflect::set(&out, &"aiRecommended".into(), &JsValue::from(res.ai_restoration_recommended));
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

    // 4. blake3Hex(bytesU8)
    let b3_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |raw_u8: Uint8Array| {
        let h = hash_bytes_exact(&raw_u8.to_vec());
        JsValue::from(h.blake3_hex)
    });

    // 5. probeRealEsrganGpu() -> { webgl2Available, isSoftware, renderer, backend }
    let probe_cb = Closure::<dyn Fn() -> JsValue>::new(move || {
        let info = probe_gpu();
        let out = Object::new();
        let _ = Reflect::set(&out, &"webgl2Available".into(), &JsValue::from(info.webgl2_available));
        let _ = Reflect::set(&out, &"isSoftware".into(), &JsValue::from(info.is_software));
        let _ = Reflect::set(&out, &"renderer".into(), &JsValue::from(info.renderer));
        let _ = Reflect::set(&out, &"backend".into(), &JsValue::from(info.backend));
        out.into()
    });

    // 6. selectRealEsrganModel(rgba, w, h, origProfileJson, requestedModelId?) -> { skip, reason, modelId, modelLabel, cwsrUrl, version }
    let select_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, prof_val: JsValue, req_model_val: JsValue| {
            let out = Object::new();
            let raw = rgba.to_vec();
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());
            let probe = probe_gpu();

            let req_str = req_model_val.as_string().unwrap_or_default();
            let chosen = if !req_str.is_empty() && (req_str == "realesr_animevideov3" || req_str == "realesr_general_x4v3") {
                Some(RealEsrganModelKind::from_str_name(&req_str))
            } else {
                select_realesrgan_model(&raw, width, height, orig_prof.as_ref(), probe.is_software)
            };

            match chosen {
                Some(kind) => {
                    let _ = Reflect::set(&out, &"skip".into(), &JsValue::FALSE);
                    let _ = Reflect::set(&out, &"modelId".into(), &JsValue::from(kind.model_id()));
                    let _ = Reflect::set(&out, &"modelLabel".into(), &JsValue::from(kind.model_label()));
                    let _ = Reflect::set(&out, &"cwsrUrl".into(), &JsValue::from(kind.cwsr_url()));
                    let _ = Reflect::set(&out, &"version".into(), &JsValue::from(kind.version()));
                    let _ = Reflect::set(&out, &"isAnime".into(), &JsValue::from(kind == RealEsrganModelKind::AnimeVideoV3));
                }
                None => {
                    let _ = Reflect::set(&out, &"skip".into(), &JsValue::TRUE);
                    let _ = Reflect::set(&out, &"reason".into(), &JsValue::from("solid_flat_image"));
                }
            }
            out.into()
        },
    );

    // 7. restoreImageRealEsrgan(rgba, w, h, modelId, modelBytesCwsr1, origProfileJson) -> Promise<{ ok, rgba, width, height, scale, modelId, modelLabel, backend, ssim, meanColorDrift, matchPct }>
    let restore_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue, Uint8Array, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, model_id_val: JsValue, model_bytes: Uint8Array, prof_val: JsValue| {
            let raw = rgba.to_vec();
            let mbytes = model_bytes.to_vec();
            let model_str = model_id_val.as_string().unwrap_or_else(|| "realesr_animevideov3".into());
            let kind = RealEsrganModelKind::from_str_name(&model_str);
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());

            future_to_promise(async move {
                let out = Object::new();
                match restore_image_realesrgan_rust(&raw, width, height, kind, &mbytes, orig_prof.as_ref()).await {
                    Ok(res) => {
                        let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                        let _ = Reflect::set(&out, &"rgba".into(), &Uint8Array::from(res.rgba.as_slice()).into());
                        let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
                        let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
                        let _ = Reflect::set(&out, &"scale".into(), &JsValue::from(res.scale));
                        let _ = Reflect::set(&out, &"modelId".into(), &JsValue::from(res.model_id));
                        let _ = Reflect::set(&out, &"modelLabel".into(), &JsValue::from(res.model_label));
                        let _ = Reflect::set(&out, &"backend".into(), &JsValue::from(res.backend));
                        let _ = Reflect::set(&out, &"ssim".into(), &JsValue::from(res.ssim));
                        let _ = Reflect::set(&out, &"meanColorDrift".into(), &JsValue::from(res.mean_color_drift));
                        let _ = Reflect::set(&out, &"matchPct".into(), &JsValue::from(res.match_pct));
                    }
                    Err(err) => {
                        let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
                        let _ = Reflect::set(&out, &"error".into(), &JsValue::from(err));
                    }
                }
                Ok(out.into())
            })
            .into()
        },
    );

    // 8. extractQualityProfile(rgba, w, h) -> string (JSON OriginalQualityProfile)
    let prof_cb = Closure::<dyn Fn(Uint8Array, u32, u32) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32| {
            let prof = extract_original_quality_profile(&rgba.to_vec(), width, height);
            JsValue::from(serde_json::to_string(&prof).unwrap_or_default())
        },
    );

    // 9. isModelLicenseAllowed(modelId) -> bool (strict Rust SPDX commercial license gate!)
    let lic_cb = Closure::<dyn Fn(JsValue) -> JsValue>::new(move |id_val: JsValue| {
        let k = id_val.as_string().unwrap_or_default().to_ascii_lowercase();
        let blocked = ["codeformer", "rvrt", "apisr", "upscayl"].iter().any(|b| k.contains(b));
        let approved = matches!(
            k.as_str(),
            "realesr_animevideov3"
                | "realesr_general_x4v3"
                | "spab_lite_builtin"
                | "span_x2_s"
                | "realesrgan_x2plus"
                | "realcugan_2x"
                | "nanovsr_lite"
        );
        JsValue::from(!blocked && approved)
    });

    let _ = Reflect::set(&api, &"version".into(), &JsValue::from("chatw-media-core-v2"));
    let _ = Reflect::set(&api, &"cnnEngine".into(), &JsValue::from("rust-realesrgan-srvgg-upscayl-v6"));
    let _ = Reflect::set(&api, &"encodeAvifAdaptive".into(), enc_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"compressFileSmart".into(), cmp_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"decompressFileVerified".into(), dec_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"blake3Hex".into(), b3_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"probeRealEsrganGpu".into(), probe_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"selectRealEsrganModel".into(), select_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"restoreImageRealEsrgan".into(), restore_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"extractQualityProfile".into(), prof_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"isModelLicenseAllowed".into(), lic_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&win, &"__chatwWasmMedia".into(), &api.into());

    enc_cb.forget();
    cmp_cb.forget();
    dec_cb.forget();
    b3_cb.forget();
    probe_cb.forget();
    select_cb.forget();
    restore_cb.forget();
    prof_cb.forget();
    lic_cb.forget();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_wasm_media_bridge() {}
