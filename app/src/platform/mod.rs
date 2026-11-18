#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

/// Resolve relative backend URLs (e.g. `/uploads/avatars/...`) for the current platform.
pub fn resolve_asset_url(url: &str) -> String {
    let base = api_base();
    if base.is_empty() || url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        url.to_string()
    } else if url.starts_with('/') {
        format!("{base}{url}")
    } else {
        format!("{base}/{url}")
    }
}

/// Format an ISO-8601 timestamp as local HH:MM matching JS `toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })`.
pub fn format_local_time(iso_string: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(iso_string) {
        Ok(dt) => {
            let local: chrono::DateTime<chrono::Local> = dt.into();
            local.format("%H:%M").to_string()
        }
        Err(_) => String::new(),
    }
}

/// Update browser/webview document title.
pub fn set_document_title(title: &str) {
    let escaped = serde_json::to_string(title).unwrap_or_else(|_| "\"chatw\"".to_string());
    let _ = dioxus::document::eval(&format!("document.title = {escaped};"));
}

/// Scroll `.chat-messages` (column-reverse: scrollTop = 0 is bottom).
pub fn scroll_chat_to_bottom(instant: bool) {
    let behavior = if instant { "instant" } else { "smooth" };
    let _ = dioxus::document::eval(&format!(
        "const el = document.getElementById('chat-messages-scroll'); if (el) el.scrollTo({{ top: 0, behavior: '{behavior}' }});"
    ));
}

/// Focus the DM message input box.
pub fn focus_chat_input() {
    let _ = dioxus::document::eval(
        "const el = document.getElementById('dm-chat-input'); if (el) el.focus();",
    );
}

/// Unlock Web Audio API on first user gesture and play the exact 880Hz -> 660Hz notification chime.
pub fn install_audio_unlock() {
    let _ = dioxus::document::eval(
        r#"
        if (!window.__chatwAudioInstalled) {
            window.__chatwAudioInstalled = true;
            window.__chatwAudioCtx = null;
            const unlock = () => {
                try {
                    const AC = window.AudioContext || window.webkitAudioContext;
                    if (AC && !window.__chatwAudioCtx) {
                        window.__chatwAudioCtx = new AC();
                        if (window.__chatwAudioCtx.state === 'suspended') window.__chatwAudioCtx.resume();
                    }
                } catch {}
            };
            document.addEventListener('click', unlock, { once: true });
            document.addEventListener('keydown', unlock, { once: true });
        }
        "#,
    );
}

pub fn play_notification_chime() {
    let _ = dioxus::document::eval(
        r#"
        try {
            const AC = window.AudioContext || window.webkitAudioContext;
            if (AC) {
                const ctx = window.__chatwAudioCtx || new AC();
                if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                const play = () => {
                    try {
                        const osc = ctx.createOscillator();
                        const gain = ctx.createGain();
                        osc.type = 'sine';
                        osc.frequency.setValueAtTime(880, ctx.currentTime);
                        osc.frequency.exponentialRampToValueAtTime(660, ctx.currentTime + 0.15);
                        gain.gain.setValueAtTime(0.18, ctx.currentTime);
                        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3);
                        osc.connect(gain);
                        gain.connect(ctx.destination);
                        osc.start();
                        osc.stop(ctx.currentTime + 0.3);
                    } catch {}
                };
                if (ctx.state === 'suspended') ctx.resume().then(play);
                else play();
            }
        } catch {}
        "#,
    );
}

/// Center-crop and compress selected file from `#avatar-file-input` to 512x512 AVIF (exact parity with image.ts).
pub async fn process_selected_avatar_bytes() -> Result<Vec<u8>, String> {
    let js = r#"
        (async () => {
            const input = document.getElementById('avatar-file-input');
            if (!input || !input.files || input.files.length === 0) {
                dioxus.send({ error: 'no_file' });
                return;
            }
            const file = input.files[0];
            try {
                let sourceWidth, sourceHeight, drawableSource;
                if (typeof createImageBitmap === 'function') {
                    const bitmap = await createImageBitmap(file);
                    sourceWidth = bitmap.width;
                    sourceHeight = bitmap.height;
                    drawableSource = bitmap;
                } else {
                    const img = new Image();
                    const url = URL.createObjectURL(file);
                    await new Promise((resolve, reject) => {
                        img.onload = () => resolve();
                        img.onerror = () => reject(new Error('Không thể tải tệp hình ảnh'));
                        img.src = url;
                    });
                    URL.revokeObjectURL(url);
                    sourceWidth = img.naturalWidth || img.width;
                    sourceHeight = img.naturalHeight || img.height;
                    drawableSource = img;
                }
                const targetSize = 512;
                let sx = 0, sy = 0;
                const sSize = Math.min(sourceWidth, sourceHeight);
                if (sourceWidth > sourceHeight) sx = Math.floor((sourceWidth - sourceHeight) / 2);
                else if (sourceHeight > sourceWidth) sy = Math.floor((sourceHeight - sourceWidth) / 2);

                const canvas = document.createElement('canvas');
                canvas.width = targetSize;
                canvas.height = targetSize;
                const ctx = canvas.getContext('2d');
                if (!ctx) throw new Error('Canvas không được hỗ trợ');
                ctx.imageSmoothingEnabled = true;
                ctx.imageSmoothingQuality = 'high';
                ctx.drawImage(drawableSource, sx, sy, sSize, sSize, 0, 0, targetSize, targetSize);

                const quality = file.size > 2 * 1024 * 1024 ? 0.75 : 0.85;
                let blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/avif', quality));
                if (!blob) blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/webp', quality));
                if (!blob) blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/jpeg', quality));
                if (!blob) blob = await new Promise((r) => canvas.toBlob((b) => r(b)));
                if (!blob) throw new Error('Không thể xử lý hình ảnh');

                const buf = new Uint8Array(await blob.arrayBuffer());
                let bin = '';
                for (let i = 0; i < buf.length; i++) bin += String.fromCharCode(buf[i]);
                input.value = '';
                dioxus.send({ b64: btoa(bin) });
            } catch (e) {
                input.value = '';
                dioxus.send({ error: String(e && e.message ? e.message : e) });
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let val: serde_json::Value = eval.recv().await.map_err(|_| "avatar_eval_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    let b64 = val
        .get("b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing_avatar_b64".to_string())?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| "invalid_avatar_b64".to_string())
}

/// Insert an emoji at the current cursor selection of `#dm-chat-input` and return updated value.
pub async fn insert_emoji_in_dm_input(current_text: &str, emoji: &str) -> String {
    insert_text_in_input("dm-chat-input", current_text, emoji).await
}

/// Insert text at the cursor of the input/textarea with `input_id` and return the updated value.
pub async fn insert_text_in_input(input_id: &str, current_text: &str, emoji: &str) -> String {
    let id_json = serde_json::to_string(input_id).unwrap_or_else(|_| "\"\"".to_string());
    let emoji_json = serde_json::to_string(emoji).unwrap_or_else(|_| "\"\"".to_string());
    let cur_json = serde_json::to_string(current_text).unwrap_or_else(|_| "\"\"".to_string());
    let js = format!(
        r#"
        (() => {{
            const input = document.getElementById({id_json});
            const emoji = {emoji_json};
            const cur = {cur_json};
            if (!input) {{
                dioxus.send({{ text: cur + emoji }});
                return;
            }}
            const start = input.selectionStart ?? cur.length;
            const end = input.selectionEnd ?? cur.length;
            const updated = cur.substring(0, start) + emoji + cur.substring(end);
            setTimeout(() => {{
                input.selectionStart = input.selectionEnd = start + emoji.length;
                input.focus();
            }}, 0);
            dioxus.send({{ text: updated }});
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    if let Ok(val) = eval.recv::<serde_json::Value>().await
        && let Some(t) = val.get("text").and_then(|v| v.as_str()) {
            return t.to_string();
        }
    format!("{current_text}{emoji}")
}

/// Query current viewport width from WebView/Browser.
pub async fn get_viewport_width() -> u32 {
    let mut eval = dioxus::document::eval("dioxus.send(window.innerWidth || 0);");
    eval.recv::<u32>().await.unwrap_or(0)
}

/// User appearance preferences. Persisted in local storage and applied as
/// `data-*` attributes on `<html>` so the CSS token layer reacts without zoom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Appearance {
    /// "light" | "dark" | "system"
    pub theme: String,
    /// "blue" | "violet" | "green" | "orange" | "rose"
    pub accent: String,
    /// "compact" | "comfortable" | "spacious"
    pub density: String,
    /// "auto" or a pixel size between 13 and 20
    pub font_size: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            accent: "blue".to_string(),
            density: "comfortable".to_string(),
            font_size: "auto".to_string(),
        }
    }
}

fn pick(value: Option<String>, allowed: &[&str], fallback: &str) -> String {
    match value {
        Some(v) if allowed.contains(&v.as_str()) => v,
        _ => fallback.to_string(),
    }
}

pub fn load_appearance() -> Appearance {
    let font_size = match storage_get("chatw_font_size") {
        Some(v) if v == "auto" => v,
        Some(v) if v.parse::<u32>().map(|n| (13..=20).contains(&n)).unwrap_or(false) => v,
        _ => "auto".to_string(),
    };
    Appearance {
        theme: pick(storage_get("chatw_theme"), &["light", "dark", "system"], "system"),
        accent: pick(storage_get("chatw_accent"), &["blue", "violet", "green", "orange", "rose"], "blue"),
        density: pick(storage_get("chatw_density"), &["compact", "comfortable", "spacious"], "comfortable"),
        font_size,
    }
}

/// Persist and apply appearance. "system" follows `prefers-color-scheme` live.
pub fn apply_appearance(a: &Appearance) {
    storage_set("chatw_theme", &a.theme);
    storage_set("chatw_accent", &a.accent);
    storage_set("chatw_density", &a.density);
    storage_set("chatw_font_size", &a.font_size);
    let font_px = if a.font_size == "auto" { String::new() } else { format!("{}px", a.font_size) };
    let script = format!(
        r#"
        (function() {{
            const root = document.documentElement;
            const pref = {theme:?};
            const mq = window.matchMedia ? window.matchMedia('(prefers-color-scheme: dark)') : null;
            const resolve = () => pref === 'system' ? ((mq && mq.matches) ? 'dark' : 'light') : pref;
            root.setAttribute('data-theme', resolve());
            root.setAttribute('data-theme-pref', pref);
            root.setAttribute('data-accent', {accent:?});
            root.setAttribute('data-density', {density:?});
            root.setAttribute('data-font-size', {font_size:?});
            if ({font_px:?}) root.style.setProperty('--font-size-root', {font_px:?});
            else root.style.removeProperty('--font-size-root');
            const meta = document.querySelector('meta[name="theme-color"]');
            if (meta) meta.setAttribute('content', resolve() === 'dark' ? '#111419' : '#FFFFFF');
            if (mq && !window.__chatwThemeListener) {{
                window.__chatwThemeListener = true;
                mq.addEventListener('change', () => {{
                    if (root.getAttribute('data-theme-pref') === 'system') {{
                        root.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
                    }}
                }});
            }}
        }})();
        "#,
        theme = a.theme,
        accent = a.accent,
        density = a.density,
        font_size = a.font_size,
        font_px = font_px,
    );
    let _ = dioxus::document::eval(&script);
}

pub fn copy_to_clipboard(text: &str) {
    let escaped = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());
    let _ = dioxus::document::eval(&format!(
        "navigator.clipboard && navigator.clipboard.writeText({escaped}).catch(() => {{}});"
    ));
}

/// Process a banner image selected from `#banner-file-input` to 960x320 AVIF/WebP/JPEG.
pub async fn process_selected_banner_bytes() -> Result<Vec<u8>, String> {
    let js = r#"
        (async () => {
            const input = document.getElementById('banner-file-input');
            if (!input || !input.files || input.files.length === 0) {
                dioxus.send({ error: 'no_file' });
                return;
            }
            const file = input.files[0];
            try {
                const bitmap = await createImageBitmap(file);
                const w = 960, h = 320;
                const canvas = document.createElement('canvas');
                canvas.width = w;
                canvas.height = h;
                const ctx = canvas.getContext('2d');
                if (!ctx) throw new Error('no_canvas');
                const scale = Math.max(w / bitmap.width, h / bitmap.height);
                const sw = w / scale, sh = h / scale;
                const sx = (bitmap.width - sw) / 2, sy = (bitmap.height - sh) / 2;
                ctx.drawImage(bitmap, sx, sy, sw, sh, 0, 0, w, h);
                let blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/avif', 0.8));
                if (!blob) blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/webp', 0.8));
                if (!blob) blob = await new Promise((r) => canvas.toBlob((b) => r(b), 'image/jpeg', 0.85));
                if (!blob) throw new Error('encode_failed');
                const buf = new Uint8Array(await blob.arrayBuffer());
                let bin = '';
                for (let i = 0; i < buf.length; i++) bin += String.fromCharCode(buf[i]);
                input.value = '';
                dioxus.send({ b64: btoa(bin) });
            } catch (e) {
                input.value = '';
                dioxus.send({ error: String(e && e.message ? e.message : e) });
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let val: serde_json::Value = eval.recv().await.map_err(|_| "banner_eval_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    let b64 = val
        .get("b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing_banner_b64".to_string())?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| "invalid_banner_b64".to_string())
}

/// Start real browser microphone recording using MediaRecorder.
pub async fn start_voice_recording() -> Result<(), String> {
    let js = r#"
        (async () => {
            try {
                const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
                const chunks = [];
                const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
                    ? 'audio/webm;codecs=opus' : 'audio/webm';
                const rec = new MediaRecorder(stream, { mimeType: mime });
                rec.ondataavailable = (e) => { if (e.data && e.data.size > 0) chunks.push(e.data); };
                window.__chatwVoiceRec = { rec, stream, chunks, mime };
                rec.start();
                dioxus.send({ ok: true });
            } catch (e) {
                dioxus.send({ error: String(e && e.message ? e.message : e) });
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let val: serde_json::Value = eval.recv().await.map_err(|_| "rec_start_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    Ok(())
}

/// Stop active MediaRecorder and return recorded audio bytes + mime type.
pub async fn stop_voice_recording() -> Result<(Vec<u8>, String), String> {
    let js = r#"
        (async () => {
            const state = window.__chatwVoiceRec;
            if (!state) { dioxus.send({ error: 'no_recorder' }); return; }
            window.__chatwVoiceRec = null;
            try {
                await new Promise((resolve) => {
                    state.rec.onstop = resolve;
                    if (state.rec.state !== 'inactive') state.rec.stop();
                    else resolve();
                });
                state.stream.getTracks().forEach((t) => t.stop());
                const blob = new Blob(state.chunks, { type: state.mime || 'audio/webm' });
                const buf = new Uint8Array(await blob.arrayBuffer());
                let bin = '';
                for (let i = 0; i < buf.length; i++) bin += String.fromCharCode(buf[i]);
                dioxus.send({ b64: btoa(bin), mime: blob.type || 'audio/webm' });
            } catch (e) {
                dioxus.send({ error: String(e && e.message ? e.message : e) });
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let val: serde_json::Value = eval.recv().await.map_err(|_| "rec_stop_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    let b64 = val.get("b64").and_then(|v| v.as_str()).ok_or("missing_audio")?;
    let mime = val
        .get("mime")
        .and_then(|v| v.as_str())
        .unwrap_or("audio/webm")
        .to_string();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| "invalid_audio_b64".to_string())?;
    Ok((bytes, mime))
}
