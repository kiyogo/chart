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
    use crate::storage::realesrgan_gpu::{
        abort_gpu_jobs, probe_gpu, restore_image_realesrgan_rust, upscale_fsr1_rust, warmup_gpu,
    };
    use chatw_media_core::{
        ai::{
            realesrgan::{
                OriginalQualityProfile, RealEsrganModelKind, extract_original_quality_profile,
                select_realesrgan_model_adaptive,
            },
            registry::ModelRegistry,
        },
        avif::{AvifCompressionMode, AvifEncodeOptions, optimize_image_to_avif},
        classifier::ImageClass,
        dedup::{
            PdqHash256, PerceptualMatchVerdict, compare_perceptual_images_v2,
            compute_audio_fingerprint_pcm, compute_pdq_hash_rgba, compute_pow_digest_hex,
            fastcdc_chunk_bytes, hash_bytes_exact,
        },
        files::{
            COMP_ENVELOPE_MAGIC, CompressionGoal, LosslessCodec, compress_file_smart,
            decompress_file_verified,
        },
    };
    use js_sys::{Array, Float32Array, Object, Reflect, Uint8Array};
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
                "balanced" | "fast" => AvifCompressionMode::FastAdaptive,
                "aggressive" | "data_saver" => AvifCompressionMode::UltraAggressive,
                "ultra" | _ => AvifCompressionMode::UltraAdaptive,
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
        let _ = Reflect::set(&out, &"use8Mrt".into(), &JsValue::from(info.use_8mrt));
        let _ = Reflect::set(&out, &"featFp16".into(), &JsValue::from(info.feat_fp16));
        let _ = Reflect::set(&out, &"maxTexSize".into(), &JsValue::from(info.max_tex_size));
        out.into()
    });

    // 6. selectRealEsrganModel(rgba, w, h, origProfileJson, requestedModelId?, srcBytes?) -> { skip, reason, modelId, modelLabel, cwsrUrl, version }
    let select_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue, JsValue, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, prof_val: JsValue, req_model_val: JsValue, src_bytes_val: JsValue| {
            let out = Object::new();
            let raw = rgba.to_vec();
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());
            let probe = probe_gpu();
            let src_bytes = src_bytes_val.as_f64().map(|v| v.max(0.0) as usize).unwrap_or(0);

            let req_str = req_model_val.as_string().unwrap_or_default();
            let chosen = if !req_str.is_empty()
                && matches!(
                    req_str.as_str(),
                    "nomosuni_span_x2"
                        | "nomosuni_span_x4"
                        | "clearreality_span_x4"
                        | "lsdir_compact3_x4"
                        | "realesr_animevideov3"
                        | "realesr_general_x4v3"
                )
            {
                Some(RealEsrganModelKind::from_str_name(&req_str))
            } else {
                select_realesrgan_model_adaptive(&raw, width, height, src_bytes, orig_prof.as_ref(), probe.is_software)
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
                    let _ = Reflect::set(&out, &"reason".into(), &JsValue::from("crisp_text_qr_or_uniform_image"));
                }
            }
            out.into()
        },
    );

    // 7. restoreImageRealEsrgan(rgba, w, h, modelId, modelBytesCwsr1, origProfileJson, onProgress?) -> Promise<{ ok, ... }>
    let restore_cb = Closure::<dyn Fn(Uint8Array, u32, u32, JsValue, Uint8Array, JsValue, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, model_id_val: JsValue, model_bytes: Uint8Array, prof_val: JsValue, prog_val: JsValue| {
            let raw = rgba.to_vec();
            let mbytes = model_bytes.to_vec();
            let model_str = model_id_val.as_string().unwrap_or_else(|| "realesr_animevideov3".into());
            let kind = RealEsrganModelKind::from_str_name(&model_str);
            let orig_prof: Option<OriginalQualityProfile> = prof_val
                .as_string()
                .filter(|s| !s.is_empty())
                .and_then(|s| serde_json::from_str(&s).ok());
            let on_progress: Option<js_sys::Function> = prog_val.dyn_into::<js_sys::Function>().ok();

            future_to_promise(async move {
                let out = Object::new();
                match restore_image_realesrgan_rust(&raw, width, height, kind, &mbytes, orig_prof.as_ref(), on_progress).await {
                    Ok(res) => {
                        let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                        if let Some(canvas) = res.canvas {
                            let _ = Reflect::set(&out, &"canvas".into(), &canvas.into());
                        }
                        if let Some(rgba) = res.rgba {
                            let _ = Reflect::set(&out, &"rgba".into(), &Uint8Array::from(rgba.as_slice()).into());
                        }
                        let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
                        let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
                        let _ = Reflect::set(&out, &"scale".into(), &JsValue::from(res.scale));
                        let _ = Reflect::set(&out, &"hasAlpha".into(), &JsValue::from(res.has_alpha));
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

    // 7b. upscaleFsr1(rgba, w, h, targetW, targetH, sharpness?, onProgress?) -> Promise<{ ok, canvas, ... }>
    let fsr1_cb = Closure::<dyn Fn(Uint8Array, u32, u32, u32, u32, JsValue, JsValue) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32, target_w: u32, target_h: u32, sharp_val: JsValue, prog_val: JsValue| {
            let raw = rgba.to_vec();
            let sharpness = sharp_val.as_f64().map(|v| v as f32).unwrap_or(0.72);
            let on_progress: Option<js_sys::Function> = prog_val.dyn_into::<js_sys::Function>().ok();

            future_to_promise(async move {
                let out = Object::new();
                match upscale_fsr1_rust(&raw, width, height, target_w, target_h, sharpness, on_progress).await {
                    Ok(res) => {
                        let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
                        if let Some(canvas) = res.canvas {
                            let _ = Reflect::set(&out, &"canvas".into(), &canvas.into());
                        }
                        if let Some(rgba) = res.rgba {
                            let _ = Reflect::set(&out, &"rgba".into(), &Uint8Array::from(rgba.as_slice()).into());
                        }
                        let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
                        let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
                        let _ = Reflect::set(&out, &"scale".into(), &JsValue::from(res.scale));
                        let _ = Reflect::set(&out, &"hasAlpha".into(), &JsValue::from(res.has_alpha));
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
            "nomosuni_span_x2"
                | "nomosuni_span_x4"
                | "clearreality_span_x4"
                | "lsdir_compact3_x4"
                | "fsr1_easu_rcas"
                | "fidelity_cas_1x"
                | "realesr_animevideov3"
                | "realesr_general_x4v3"
                | "realesrgan_x4plus"
                | "realesrgan_anime_6b"
                | "scunet-denoise-x1"
                | "fsrcnn-x2-fast"
                | "spab_lite_builtin"
                | "span_x2_s"
                | "realesrgan_x2plus"
                | "realcugan_2x"
                | "nanovsr_lite"
        );
        JsValue::from(!blocked && approved)
    });

    // 9b. getModelRegistryJson() -> string (canonical Rust ModelRegistry JSON manifest)
    let reg_cb = Closure::<dyn Fn() -> JsValue>::new(move || {
        let reg = ModelRegistry::canonical();
        JsValue::from(serde_json::to_string(reg.list()).unwrap_or_else(|_| "[]".into()))
    });

    // 10. computeImagePdq(rgba, w, h) -> { pdqHex, quality, captionBandHash, colorMomentHash, aspectRatioMilli, mihBands }
    let pdq_cb = Closure::<dyn Fn(Uint8Array, u32, u32) -> JsValue>::new(
        move |rgba: Uint8Array, width: u32, height: u32| {
            let h = compute_pdq_hash_rgba(&rgba.to_vec(), width, height);
            let out = Object::new();
            let _ = Reflect::set(&out, &"pdqHex".into(), &JsValue::from(h.to_hex()));
            let _ = Reflect::set(&out, &"quality".into(), &JsValue::from(h.quality));
            let _ = Reflect::set(&out, &"captionBandHash".into(), &JsValue::from(h.caption_band_hash.to_string()));
            let _ = Reflect::set(&out, &"colorMomentHash".into(), &JsValue::from(h.color_moment_hash.to_string()));
            let _ = Reflect::set(&out, &"aspectRatioMilli".into(), &JsValue::from(h.aspect_ratio_milli));
            let bands = Array::new();
            for b in h.mih_bands() {
                bands.push(&JsValue::from(b));
            }
            let _ = Reflect::set(&out, &"mihBands".into(), &bands.into());
            out.into()
        },
    );

    // 11. computePowProof(bytesU8) -> { blake3Hex, sha256Hex, powDigestHex, byteSize }
    let pow_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |raw_u8: Uint8Array| {
        let raw = raw_u8.to_vec();
        let h = hash_bytes_exact(&raw);
        let pow = compute_pow_digest_hex(&raw);
        let out = Object::new();
        let _ = Reflect::set(&out, &"blake3Hex".into(), &JsValue::from(h.blake3_hex));
        let _ = Reflect::set(&out, &"sha256Hex".into(), &JsValue::from(h.sha256_hex));
        let _ = Reflect::set(&out, &"powDigestHex".into(), &JsValue::from(pow));
        let _ = Reflect::set(&out, &"byteSize".into(), &JsValue::from(h.byte_size as f64));
        out.into()
    });

    // 12. computeFastCdc(bytesU8) -> { blake3Hex, sha256Hex, powDigestHex, chunks: [{ index, offset, length, blake3Hex }] }
    let cdc_cb = Closure::<dyn Fn(Uint8Array) -> JsValue>::new(move |raw_u8: Uint8Array| {
        let raw = raw_u8.to_vec();
        let h = hash_bytes_exact(&raw);
        let pow = compute_pow_digest_hex(&raw);
        let chunks = fastcdc_chunk_bytes(&raw);
        let arr = Array::new();
        for (idx, c) in chunks.iter().enumerate() {
            let co = Object::new();
            let _ = Reflect::set(&co, &"index".into(), &JsValue::from(idx as u32));
            let _ = Reflect::set(&co, &"offset".into(), &JsValue::from(c.offset as f64));
            let _ = Reflect::set(&co, &"length".into(), &JsValue::from(c.length));
            let _ = Reflect::set(&co, &"blake3Hex".into(), &JsValue::from(c.blake3_hex.clone()));
            arr.push(&co.into());
        }
        let out = Object::new();
        let _ = Reflect::set(&out, &"blake3Hex".into(), &JsValue::from(h.blake3_hex));
        let _ = Reflect::set(&out, &"sha256Hex".into(), &JsValue::from(h.sha256_hex));
        let _ = Reflect::set(&out, &"powDigestHex".into(), &JsValue::from(pow));
        let _ = Reflect::set(&out, &"chunks".into(), &arr.into());
        out.into()
    });

    // 13. computeAudioFingerprint(pcmF32, durationMs) -> { bitsHex, durationMs, energyScore, json }
    let audio_fp_cb = Closure::<dyn Fn(Float32Array, f64) -> JsValue>::new(
        move |pcm: Float32Array, duration_ms: f64| {
            let samples = pcm.to_vec();
            let fp = compute_audio_fingerprint_pcm(&samples, duration_ms.max(0.0) as u64);
            let out = Object::new();
            let _ = Reflect::set(&out, &"bitsHex".into(), &JsValue::from(fp.to_hex()));
            let _ = Reflect::set(&out, &"durationMs".into(), &JsValue::from(fp.duration_ms as f64));
            let _ = Reflect::set(&out, &"energyScore".into(), &JsValue::from(fp.energy_score));
            if let Ok(js) = serde_json::to_string(&fp) {
                let _ = Reflect::set(&out, &"json".into(), &JsValue::from(js));
            }
            out.into()
        },
    );

    // 14. comparePdqLocal(pdqObjA, pdqObjB) -> { matched, hamming }
    let cmp_pdq_cb = Closure::<dyn Fn(JsValue, JsValue) -> JsValue>::new(
        move |obj_a: JsValue, obj_b: JsValue| {
            let out = Object::new();
            let parse_pdq = |obj: &JsValue| -> Option<PdqHash256> {
                let hex = Reflect::get(obj, &"pdqHex".into()).ok()?.as_string()?;
                let q = Reflect::get(obj, &"pdqQuality".into()).ok().and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 100.0) as u8;
                let parse_u64 = |k: &str| -> u64 {
                    Reflect::get(obj, &k.into()).ok().and_then(|v| {
                        v.as_string()
                            .and_then(|s| s.parse::<u64>().ok())
                            .or_else(|| v.as_f64().map(|f| f as u64))
                    }).unwrap_or(0)
                };
                let cap = parse_u64("captionBandHash");
                let col = parse_u64("colorMomentHash");
                let asp = Reflect::get(obj, &"aspectRatioMilli".into()).ok().and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 10000.0) as u16;
                PdqHash256::from_hex_full(&hex, q, cap, col, asp)
            };
            if let (Some(pa), Some(pb)) = (parse_pdq(&obj_a), parse_pdq(&obj_b)) {
                if let PerceptualMatchVerdict::EquivalentCandidate { hamming } =
                    compare_perceptual_images_v2(&pa, &pb, 0, 0, true)
                {
                    let _ = Reflect::set(&out, &"matched".into(), &JsValue::TRUE);
                    let _ = Reflect::set(&out, &"hamming".into(), &JsValue::from(hamming));
                    return out.into();
                }
            }
            let _ = Reflect::set(&out, &"matched".into(), &JsValue::FALSE);
            let _ = Reflect::set(&out, &"hamming".into(), &JsValue::from(256u32));
            out.into()
        },
    );

    // 15. warmupRealEsrganGpu() -> bool (idle pre-compile of WebGL2 shaders so first click has 0ms compile hitch!)
    let warmup_cb = Closure::<dyn Fn() -> JsValue>::new(move || JsValue::from(warmup_gpu()));
    // 16. abortActiveGpuJob() -> u64 (cancels any timed-out WebGL2 async future before starting the next job!)
    let abort_cb = Closure::<dyn Fn() -> JsValue>::new(move || JsValue::from(abort_gpu_jobs() as f64));

    let _ = Reflect::set(&api, &"version".into(), &JsValue::from("chatw-media-core-v2"));
    let _ = Reflect::set(&api, &"cnnEngine".into(), &JsValue::from("rust-realesrgan-srvgg-upscayl-v8"));
    let _ = Reflect::set(&api, &"dedupEngine".into(), &JsValue::from("rust-hyper-dedup-fastcdc-pdq-mih-v1"));
    let _ = Reflect::set(&api, &"encodeAvifAdaptive".into(), enc_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"compressFileSmart".into(), cmp_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"decompressFileVerified".into(), dec_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"blake3Hex".into(), b3_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"probeRealEsrganGpu".into(), probe_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"warmupRealEsrganGpu".into(), warmup_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"abortActiveGpuJob".into(), abort_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"selectRealEsrganModel".into(), select_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"restoreImageRealEsrgan".into(), restore_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"upscaleFsr1".into(), fsr1_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"extractQualityProfile".into(), prof_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"isModelLicenseAllowed".into(), lic_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"getModelRegistryJson".into(), reg_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"computeImagePdq".into(), pdq_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"computePowProof".into(), pow_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"computeFastCdc".into(), cdc_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"computeAudioFingerprint".into(), audio_fp_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&api, &"comparePdqLocal".into(), cmp_pdq_cb.as_ref().unchecked_ref());
    let _ = Reflect::set(&win, &"__chatwWasmMedia".into(), &api.into());

    enc_cb.forget();
    cmp_cb.forget();
    dec_cb.forget();
    b3_cb.forget();
    probe_cb.forget();
    warmup_cb.forget();
    abort_cb.forget();
    select_cb.forget();
    restore_cb.forget();
    fsr1_cb.forget();
    prof_cb.forget();
    lic_cb.forget();
    reg_cb.forget();
    pdq_cb.forget();
    pow_cb.forget();
    cdc_cb.forget();
    audio_fp_cb.forget();
    cmp_pdq_cb.forget();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_wasm_media_bridge() {}
