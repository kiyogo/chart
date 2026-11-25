use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptimizedMediaPayload {
    pub bytes: Vec<u8>,
    pub mime: String,
    pub ext: String,
    pub file_name: String,
    pub original_size: usize,
    pub optimized_size: usize,
    pub is_crisp_text: bool,
    pub media_kind: String, // "image" | "video" | "audio" | "file"
    pub sha256_plain: String,
    pub thumb_b64: Option<String>,
    pub thumb_mime: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub blur_data_url: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub target_width: Option<u32>,
    #[serde(default)]
    pub target_height: Option<u32>,
    #[serde(default)]
    pub waveform: Vec<u8>,
    #[serde(default)]
    pub instant_preview_url: Option<String>,
    #[serde(default)]
    pub codec: Option<String>,
    /// Compact (~95-byte) perceptual fingerprint of sender's original image for receiver closed-loop AI calibration.
    #[serde(default)]
    pub orig_profile_json: Option<String>,
    /// Spec 111 Hyper-Dedup metadata computed in Pure-Rust WASM before upload!
    #[serde(default)]
    pub orig_blake3: Option<String>,
    #[serde(default)]
    pub pdq_hex: Option<String>,
    #[serde(default)]
    pub pdq_quality: Option<u8>,
    #[serde(default)]
    pub caption_band_hash: Option<u64>,
    #[serde(default)]
    pub color_moment_hash: Option<u64>,
    #[serde(default)]
    pub aspect_ratio_milli: Option<u16>,
    #[serde(default)]
    pub ai_recommended: bool,
    #[serde(default)]
    pub video_fingerprint_json: Option<String>,
    #[serde(default)]
    pub audio_fingerprint_json: Option<String>,
    /// Populated immediately if the image/file was copied from another chat (`__chatwDedupLocal` hit -> 0-byte upload!).
    #[serde(default)]
    pub copied_from_ref_id: Option<String>,
    #[serde(default)]
    pub copied_key_b64: Option<String>,
    #[serde(default)]
    pub copied_nonce_b64: Option<String>,
    /// True when the user copied/pasted a blurry/un-enhanced or server-deduped AI image -> sharpen BOTH sender & receiver!
    #[serde(default)]
    pub needs_sender_ai: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstantPickPreview {
    pub handle_id: String,
    pub instant_preview_url: String,
    pub file_name: String,
    pub mime: String,
    pub original_size: usize,
    pub media_kind: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Spec 126: Strip raw file extensions from media filenames so names are stored cleanly without `.jpg`, `.png`, etc.
pub fn strip_file_extension(name: &str) -> String {
    let s = name.trim();
    if let Some(idx) = s.rfind('.') {
        if idx > 0 {
            return s[..idx].to_string();
        }
    }
    s.to_string()
}

/// Real magic byte sniffer shared on native & WASM so extension/MIME are always truthful.
pub fn sniff_mime_from_bytes(bytes: &[u8]) -> (&'static str, &'static str) {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return ("image/png", "png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ("image/jpeg", "jpg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ("image/gif", "gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return ("image/webp", "webp");
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let b = &bytes[8..12];
        if b == b"avif" || b == b"avis" {
            return ("image/avif", "avif");
        }
        if matches!(b, b"isom" | b"mp42" | b"qt  " | b"avc1") {
            return ("video/mp4", "mp4");
        }
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return ("video/webm", "webm");
    }
    if bytes.starts_with(b"OggS") {
        return ("audio/ogg", "ogg");
    }
    if bytes.starts_with(b"%PDF-") {
        return ("application/pdf", "pdf");
    }
    ("application/octet-stream", "bin")
}

fn parse_optimized_payload(v: &serde_json::Value) -> Result<OptimizedMediaPayload, String> {
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    let b64 = v.get("bytesB64").and_then(|s| s.as_str()).ok_or("missing_bytes")?;
    let bytes = B64.decode(b64).map_err(|_| "decode_failed".to_string())?;
    let sha = super::sha256_hex(&bytes);
    let (sniff_mime, sniff_ext) = sniff_mime_from_bytes(&bytes);
    let reported_mime = v.get("mime").and_then(|s| s.as_str()).unwrap_or(sniff_mime);
    let mime = if reported_mime == "application/octet-stream" && sniff_mime != "application/octet-stream" {
        sniff_mime.to_string()
    } else {
        reported_mime.to_string()
    };
    let ext = v.get("ext").and_then(|s| s.as_str()).unwrap_or(sniff_ext).to_string();

    let parse_u64 = |k: &str| -> Option<u64> {
        v.get(k).and_then(|val| {
            val.as_u64()
                .or_else(|| val.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
    };

    Ok(OptimizedMediaPayload {
        bytes,
        mime,
        ext,
        file_name: strip_file_extension(v.get("fileName").and_then(|s| s.as_str()).unwrap_or("attachment")),
        original_size: v.get("originalSize").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        optimized_size: v.get("optimizedSize").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        is_crisp_text: v.get("isCrispText").and_then(|b| b.as_bool()).unwrap_or(false),
        media_kind: v.get("mediaKind").and_then(|s| s.as_str()).unwrap_or("file").to_string(),
        sha256_plain: v.get("sha256").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(sha),
        thumb_b64: v.get("thumbB64").and_then(|s| s.as_str()).map(str::to_string),
        thumb_mime: v.get("thumbMime").and_then(|s| s.as_str()).map(str::to_string),
        duration_ms: v.get("durationMs").and_then(|n| n.as_u64()),
        blur_data_url: v.get("blurDataUrl").and_then(|s| s.as_str()).map(str::to_string),
        width: v.get("width").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
        height: v.get("height").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
        target_width: v.get("targetWidth").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
        target_height: v.get("targetHeight").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
        waveform: Vec::new(),
        instant_preview_url: v.get("instantPreviewUrl").and_then(|s| s.as_str()).map(str::to_string),
        codec: v.get("codec").and_then(|s| s.as_str()).map(str::to_string),
        orig_profile_json: v.get("origProfileJson").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        orig_blake3: v.get("origBlake3").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        pdq_hex: v.get("pdqHex").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        pdq_quality: v.get("pdqQuality").and_then(|n| n.as_u64()).map(|n| n.min(100) as u8),
        caption_band_hash: parse_u64("captionBandHash"),
        color_moment_hash: parse_u64("colorMomentHash"),
        aspect_ratio_milli: v.get("aspectRatioMilli").and_then(|n| n.as_u64()).map(|n| n.min(10000) as u16),
        ai_recommended: v.get("aiRecommended").and_then(|b| b.as_bool()).unwrap_or(false),
        video_fingerprint_json: v.get("videoFingerprintJson").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        audio_fingerprint_json: v.get("audioFingerprintJson").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        copied_from_ref_id: v.get("copiedFromRefId").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        copied_key_b64: v.get("copiedKeyB64").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        copied_nonce_b64: v.get("copiedNonceB64").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        needs_sender_ai: v.get("needsSenderAi").and_then(|b| b.as_bool()).unwrap_or(false),
    })
}

/// Optimize & fingerprint raw bytes (e.g., pasted from clipboard `Ctrl+V`!) through the exact same
/// Hyper-Dedup + AVIF pipeline so copied chat images NEVER create duplicate files on the server!
pub async fn optimize_pasted_image_bytes(
    bytes: &[u8],
    mime: &str,
    file_name: &str,
) -> Result<OptimizedMediaPayload, String> {
    let b64 = B64.encode(bytes);
    let b64_json = serde_json::to_string(&b64).unwrap_or_default();
    let mime_json = serde_json::to_string(mime).unwrap_or_default();
    let name_json = serde_json::to_string(file_name).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwMediaOpt || !window.__chatwMediaOpt.optimizePastedBytes) {{
                dioxus.send({{ error: 'optimizer_unavailable' }});
                return;
            }}
            const res = await window.__chatwMediaOpt.optimizePastedBytes({b64_json}, {mime_json}, {name_json});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    parse_optimized_payload(&v)
}

/// Phase 1 (Instant Sender Original Preview): grab crisp local `blob:` preview URL in < 20ms!
pub async fn pick_instant_preview(input_id: &str) -> Result<InstantPickPreview, String> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwMediaOpt || !window.__chatwMediaOpt.pickInstantPreview) {{
                dioxus.send({{ error: 'optimizer_unavailable' }});
                return;
            }}
            const res = await window.__chatwMediaOpt.pickInstantPreview({id_json});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    Ok(InstantPickPreview {
        handle_id: v.get("handleId").and_then(|s| s.as_str()).ok_or("missing_handle")?.to_string(),
        instant_preview_url: v.get("instantPreviewUrl").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
        file_name: strip_file_extension(v.get("fileName").and_then(|s| s.as_str()).unwrap_or("attachment")),
        mime: v.get("mime").and_then(|s| s.as_str()).unwrap_or("application/octet-stream").to_string(),
        original_size: v.get("originalSize").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        media_kind: v.get("mediaKind").and_then(|s| s.as_str()).unwrap_or("file").to_string(),
        width: v.get("width").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
        height: v.get("height").and_then(|n| n.as_u64()).filter(|&n| n > 0).map(|n| n as u32),
    })
}

/// Phase 2: compress the stashed sender file off-main-thread while sender already sees the original preview!
pub async fn process_pending_handle(
    handle_id: &str,
    preserve_original: bool,
) -> Result<OptimizedMediaPayload, String> {
    let hid_json = serde_json::to_string(handle_id).unwrap_or_default();
    let pres = if preserve_original { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwMediaOpt || !window.__chatwMediaOpt.processPendingHandle) {{
                dioxus.send({{ error: 'optimizer_unavailable' }});
                return;
            }}
            const res = await window.__chatwMediaOpt.processPendingHandle({hid_json}, {pres});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    parse_optimized_payload(&v)
}

/// Remove a stale or unauthorized reference from the browser's local dedup index.
pub fn evict_local_dedup_entry(ref_id: &str) {
    let rid_json = serde_json::to_string(ref_id).unwrap_or_default();
    let _ = dioxus::document::eval(&format!(
        "(() => {{ if (window.__chatwDedupLocal && window.__chatwDedupLocal.removeMedia) window.__chatwDedupLocal.removeMedia({rid_json}); }})()"
    ));
}

/// Re-encode the stashed source file for `preview_url` bypassing local dedup when a server claim misses.
pub async fn reencode_from_preview_url(preview_url: &str) -> Result<OptimizedMediaPayload, String> {
    let url_json = serde_json::to_string(preview_url).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwMediaOpt || !window.__chatwMediaOpt.reencodeFromPreviewUrl) {{
                dioxus.send({{ error: 'optimizer_unavailable' }});
                return;
            }}
            const res = await window.__chatwMediaOpt.reencodeFromPreviewUrl({url_json});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    parse_optimized_payload(&v)
}

pub fn release_pending_handle(handle_id: &str, revoke_url: bool) {
    let hid_json = serde_json::to_string(handle_id).unwrap_or_default();
    let rev = if revoke_url { "true" } else { "false" };
    let _ = dioxus::document::eval(&format!(
        "(() => {{ if (window.__chatwMediaOpt && window.__chatwMediaOpt.releasePendingHandle) window.__chatwMediaOpt.releasePendingHandle({hid_json}, {rev}); }})()"
    ));
}

/// After an upload completes, save the sender's 100% untouched original file into local app storage
/// so the sender immediately views their crisp original file & retains it across reloads.
pub async fn commit_sender_original(
    preview_url: &str,
    download_url: &str,
    key_b64: &str,
    nonce_b64: &str,
    ref_id: &str,
) {
    let p_json = serde_json::to_string(preview_url).unwrap_or_default();
    let d_json = serde_json::to_string(download_url).unwrap_or_default();
    let k_json = serde_json::to_string(key_b64).unwrap_or_default();
    let n_json = serde_json::to_string(nonce_b64).unwrap_or_default();
    let r_json = serde_json::to_string(ref_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (window.__chatwMediaOpt && window.__chatwMediaOpt.commitSenderOriginal) {{
                await window.__chatwMediaOpt.commitSenderOriginal({p_json}, {d_json}, {k_json}, {n_json}, {r_json});
            }}
            dioxus.send(true);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let _ = eval.recv::<bool>().await;
}

/// Pick and adaptively optimize image/video/audio/file from a DOM `<input type="file">`.
pub async fn pick_and_optimize_from_input(
    input_id: &str,
    preserve_original: bool,
) -> Result<OptimizedMediaPayload, String> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let pres = if preserve_original { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwMediaOpt) {{ dioxus.send({{ error: 'optimizer_unavailable' }}); return; }}
            const res = await window.__chatwMediaOpt.optimizeFromInput({id_json}, {pres});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    parse_optimized_payload(&v)
}

pub async fn start_voice_recording() -> Result<String, String> {
    let js = r#"
    (async () => {
        try {
            if (!window.__chatwMediaOpt) { dioxus.send({ error: 'no_recorder' }); return; }
            const res = await window.__chatwMediaOpt.startVoiceRecording();
            dioxus.send(res);
        } catch (e) {
            dioxus.send({ error: String(e && e.message ? e.message : e) });
        }
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    Ok(v.get("mime").and_then(|s| s.as_str()).unwrap_or("audio/webm").to_string())
}

pub async fn stop_voice_recording() -> Result<OptimizedMediaPayload, String> {
    let js = r#"
    (async () => {
        try {
            if (!window.__chatwMediaOpt) { dioxus.send({ error: 'no_recorder' }); return; }
            const res = await window.__chatwMediaOpt.stopVoiceRecording();
            dioxus.send(res);
        } catch (e) {
            dioxus.send({ error: String(e && e.message ? e.message : e) });
        }
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    let b64 = v.get("bytesB64").and_then(|s| s.as_str()).ok_or("missing_audio")?;
    let bytes = B64.decode(b64).map_err(|_| "decode_failed".to_string())?;
    let len = bytes.len();
    let sha = super::sha256_hex(&bytes);
    let mime = v.get("mime").and_then(|s| s.as_str()).unwrap_or("audio/webm").to_string();
    let ext = if mime.contains("ogg") { "ogg" } else { "webm" }.to_string();
    let dur = v.get("durationMs").and_then(|n| n.as_u64());
    let waveform: Vec<u8> = v
        .get("waveform")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_u64())
                .map(|x| x.min(31) as u8)
                .take(64)
                .collect()
        })
        .unwrap_or_default();
    Ok(OptimizedMediaPayload {
        bytes,
        mime,
        ext: ext.clone(),
        file_name: format!("voice_{}.{ext}", chrono::Utc::now().timestamp()),
        original_size: len,
        optimized_size: len,
        is_crisp_text: false,
        media_kind: "audio".into(),
        sha256_plain: sha,
        thumb_b64: None,
        thumb_mime: None,
        duration_ms: dur,
        blur_data_url: None,
        width: None,
        height: None,
        target_width: None,
        target_height: None,
        waveform,
        instant_preview_url: None,
        codec: Some("opus".into()),
        orig_profile_json: None,
        orig_blake3: None,
        pdq_hex: None,
        pdq_quality: None,
        caption_band_hash: None,
        color_moment_hash: None,
        aspect_ratio_milli: None,
        ai_recommended: false,
        video_fingerprint_json: None,
        audio_fingerprint_json: None,
        copied_from_ref_id: None,
        copied_key_b64: None,
        copied_nonce_b64: None,
        needs_sender_ai: false,
    })
}
