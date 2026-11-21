#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ExportDownloadResult {
    pub filename: String,
    pub byte_size: u64,
    pub job_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedFileMeta {
    pub name: String,
    pub byte_size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UploadBackupResult {
    pub status: u16,
    pub body: String,
    pub filename: String,
    pub byte_size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevicePasskeyCreated {
    pub credential_id: String,
    pub public_key_spki_b64: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevicePasskeySignature {
    pub credential_id: String,
    pub signature_b64: String,
}

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

/// Notification sounds synthesised with WebAudio (no audio files to load).
pub fn play_sound(name: &str) {
    let n = serde_json::to_string(name).unwrap_or_else(|_| "\"pop\"".into());
    let _ = dioxus::document::eval(&format!(
        r#"
        try {{
            const AC = window.AudioContext || window.webkitAudioContext;
            if (AC) {{
                const ctx = window.__chatwAudioCtx || new AC();
                if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                const tone = (f0, f1, start, dur, type, vol) => {{
                    const osc = ctx.createOscillator();
                    const gain = ctx.createGain();
                    osc.type = type;
                    const t = ctx.currentTime + start;
                    osc.frequency.setValueAtTime(f0, t);
                    osc.frequency.exponentialRampToValueAtTime(f1, t + dur * 0.8);
                    gain.gain.setValueAtTime(0.0001, t);
                    gain.gain.exponentialRampToValueAtTime(vol, t + 0.015);
                    gain.gain.exponentialRampToValueAtTime(0.0001, t + dur);
                    osc.connect(gain); gain.connect(ctx.destination);
                    osc.start(t); osc.stop(t + dur + 0.02);
                }};
                const play = () => {{
                    switch ({n}) {{
                        case 'chime': tone(880, 660, 0, 0.3, 'sine', 0.18); tone(1320, 990, 0.12, 0.35, 'sine', 0.1); break;
                        case 'soft': tone(520, 500, 0, 0.45, 'sine', 0.12); break;
                        case 'bubble': tone(300, 900, 0, 0.12, 'sine', 0.16); tone(500, 1200, 0.1, 0.12, 'sine', 0.12); break;
                        default: tone(660, 990, 0, 0.14, 'triangle', 0.2);
                    }}
                }};
                if (ctx.state === 'suspended') ctx.resume().then(play); else play();
            }}
        }} catch (e) {{}}
        "#
    ));
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
    process_avatar_from_input("avatar-file-input").await
}

/// Same as [`process_selected_avatar_bytes`] for an arbitrary file input id.
pub async fn process_avatar_from_input(input_id: &str) -> Result<Vec<u8>, String> {
    let js = r#"
        (async () => {
            const input = document.getElementById('__INPUT_ID__');
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
    let mut eval = dioxus::document::eval(&js.replace("__INPUT_ID__", input_id));
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
fn hex_luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let ch = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f64 / 255.0;
    let lin = |c: f64| if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(ch(0)) + 0.7152 * lin(ch(2)) + 0.0722 * lin(ch(4))
}

/// Readable text colour on top of a custom background colour.
pub fn contrast_text(hex: &str) -> &'static str {
    if hex_luminance(hex) > 0.42 { "#111419" } else { "#FFFFFF" }
}

/// Apply the user's deep customisation (custom accent, bubbles, shapes, widths,
/// fonts, app wallpaper). Values are validated by `Prefs::sanitize` first.
pub fn apply_customization(p: &crate::prefs::Prefs) {
    let (wp_kind, wp_value) = if p.app_wallpaper.is_empty() {
        ("none".to_string(), String::new())
    } else if let Some(c) = p.app_wallpaper.strip_prefix("color:") {
        ("color".to_string(), c.to_string())
    } else if p.app_wallpaper.starts_with("/uploads/media/") {
        ("image".to_string(), format!("url('{}')", resolve_asset_url(&p.app_wallpaper)))
    } else {
        (format!("preset-{}", p.app_wallpaper), String::new())
    };
    let vars = serde_json::json!({
        "--accent": p.custom_accent,
        "--accent-hover": if p.custom_accent.is_empty() { String::new() } else { format!("color-mix(in srgb, {} 86%, #000)", p.custom_accent) },
        "--accent-soft": if p.custom_accent.is_empty() { String::new() } else { format!("color-mix(in srgb, {} 14%, transparent)", p.custom_accent) },
        "--accent-soft-strong": if p.custom_accent.is_empty() { String::new() } else { format!("color-mix(in srgb, {} 24%, transparent)", p.custom_accent) },
        "--on-accent": if p.custom_accent.is_empty() { "" } else { contrast_text(&p.custom_accent) },
        "--msg-sent": p.bubble_self,
        "--msg-sent-text": if p.bubble_self.is_empty() { "" } else { contrast_text(&p.bubble_self) },
        "--msg-recv": p.bubble_other,
        "--msg-recv-text": if p.bubble_other.is_empty() { "" } else { contrast_text(&p.bubble_other) },
        "--app-wp-color": if wp_kind == "color" { wp_value.clone() } else { String::new() },
        "--app-wp-image": if wp_kind == "image" { wp_value.clone() } else { String::new() },
        "--app-wp-dim": format!("{}", p.app_wallpaper_dim as f32 / 100.0),
        "--app-wp-blur": format!("{}px", p.app_wallpaper_blur),
        "--panel-opacity": format!("{}%", p.panel_opacity),
        "--panel-blur": format!("{}px", p.panel_blur),
    });
    let attrs = serde_json::json!({
        "data-bubble-shape": p.bubble_shape,
        "data-avatar-shape": p.avatar_shape,
        "data-chat-width": p.chat_width,
        "data-font": p.font_family,
        "data-reduce-motion": p.reduce_motion.to_string(),
        "data-show-avatars": p.show_avatars.to_string(),
        "data-app-wp": wp_kind,
        "data-panel-glass": (p.panel_opacity < 100).to_string(),
        "data-custom-accent": (!p.custom_accent.is_empty()).to_string(),
    });
    let _ = dioxus::document::eval(&format!(
        r#"
        (function() {{
            const root = document.documentElement;
            const vars = {vars};
            for (const [k, v] of Object.entries(vars)) {{
                if (v) root.style.setProperty(k, v); else root.style.removeProperty(k);
            }}
            const attrs = {attrs};
            for (const [k, v] of Object.entries(attrs)) root.setAttribute(k, v);
        }})();
        "#
    ));
}

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
    process_banner_from_input("banner-file-input").await
}

/// Same as [`process_selected_banner_bytes`] for an arbitrary file input id.
pub async fn process_banner_from_input(input_id: &str) -> Result<Vec<u8>, String> {
    let js = r#"
        (async () => {
            const input = document.getElementById('__INPUT_ID__');
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
    let mut eval = dioxus::document::eval(&js.replace("__INPUT_ID__", input_id));
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

/// Read the image picked in `<input type=file id=input_id>` for use as a sticker.
/// Animated GIFs up to 512 KB are kept as-is; other images are fitted into 320x320
/// and re-encoded as WebP. Returns (bytes, mime).
pub async fn process_selected_sticker(input_id: &str) -> Result<(Vec<u8>, String), String> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            const input = document.getElementById({id_json});
            if (!input || !input.files || input.files.length === 0) {{ dioxus.send({{ error: 'no_file' }}); return; }}
            const file = input.files[0];
            const toB64 = (buf) => {{
                let s = ''; const bytes = new Uint8Array(buf);
                for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
                return btoa(s);
            }};
            try {{
                if (file.type === 'image/gif' && file.size <= 512 * 1024) {{
                    dioxus.send({{ b64: toB64(await file.arrayBuffer()), mime: 'image/gif' }});
                    input.value = '';
                    return;
                }}
                const bitmap = await createImageBitmap(file);
                const size = 320;
                const scale = Math.min(size / bitmap.width, size / bitmap.height, 1);
                const w = Math.round(bitmap.width * scale), h = Math.round(bitmap.height * scale);
                const canvas = document.createElement('canvas');
                canvas.width = size; canvas.height = size;
                const ctx = canvas.getContext('2d');
                ctx.imageSmoothingQuality = 'high';
                ctx.drawImage(bitmap, Math.round((size - w) / 2), Math.round((size - h) / 2), w, h);
                const blob = await new Promise((res) => canvas.toBlob(res, 'image/webp', 0.9));
                if (!blob) throw new Error('encode_failed');
                dioxus.send({{ b64: toB64(await blob.arrayBuffer()), mime: 'image/webp' }});
                input.value = '';
            }} catch (e) {{
                dioxus.send({{ error: 'unsupported' }});
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| "eval_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    use base64::Engine;
    let b64 = val.get("b64").and_then(|v| v.as_str()).ok_or("no_data")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "decode_failed".to_string())?;
    let mime = val.get("mime").and_then(|v| v.as_str()).unwrap_or("image/webp").to_string();
    Ok((bytes, mime))
}

/// Read the raw bytes of the file picked in `<input type=file id=input_id>` (max 2 MB).
/// Returns (bytes, mime). Used for profile artwork where animation (GIF) must be kept.
pub async fn read_selected_file_raw(input_id: &str) -> Result<(Vec<u8>, String), String> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            const input = document.getElementById({id_json});
            if (!input || !input.files || input.files.length === 0) {{ dioxus.send({{ error: 'no_file' }}); return; }}
            const file = input.files[0];
            if (file.size > 2 * 1024 * 1024) {{ dioxus.send({{ error: 'too_large' }}); input.value = ''; return; }}
            const buf = new Uint8Array(await file.arrayBuffer());
            let s = '';
            for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
            dioxus.send({{ b64: btoa(s), mime: file.type || 'application/octet-stream' }});
            input.value = '';
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| "eval_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    use base64::Engine;
    let b64 = val.get("b64").and_then(|v| v.as_str()).ok_or("no_data")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "decode_failed".to_string())?;
    Ok((bytes, val.get("mime").and_then(|v| v.as_str()).unwrap_or("image/png").to_string()))
}

// ---------------------------------------------------------------------------
// Spec 007 helpers: notifications, deep links, idle lock, paste, table images
// ---------------------------------------------------------------------------

/// Ask the browser for notification permission. Resolves to "granted" / "denied" / "unsupported".
pub async fn request_notification_permission() -> String {
    let mut ev = dioxus::document::eval(
        r#"
        (async () => {
            if (!('Notification' in window)) { dioxus.send('unsupported'); return; }
            try { dioxus.send(await Notification.requestPermission()); } catch (e) { dioxus.send('denied'); }
        })();
        "#,
    );
    ev.recv::<String>().await.unwrap_or_else(|_| "unsupported".into())
}

/// Show a system notification (no-op when permission is missing).
pub fn show_notification(title: &str, body: &str, tag: &str) {
    let t = serde_json::to_string(title).unwrap_or_default();
    let b = serde_json::to_string(body).unwrap_or_default();
    let g = serde_json::to_string(tag).unwrap_or_default();
    let _ = dioxus::document::eval(&format!(
        "try {{ if ('Notification' in window && Notification.permission === 'granted') {{ const n = new Notification({t}, {{ body: {b}, tag: {g}, icon: '/logo.svg' }}); n.onclick = () => {{ window.focus(); n.close(); }}; }} }} catch (e) {{}}"
    ));
}

/// True when the app window is visible and focused.
pub async fn is_window_active() -> bool {
    let mut ev = dioxus::document::eval("dioxus.send(!document.hidden && document.hasFocus());");
    ev.recv::<bool>().await.unwrap_or(true)
}

/// Read and clear a deep link (`/invite/g/CODE`, `/invite/s/CODE`, `?dm=name`, `?bot=name`).
pub async fn take_deep_link() -> Option<String> {
    let mut ev = dioxus::document::eval(
        r#"
        const p = window.location.pathname + window.location.search;
        if (p && p !== '/' && (p.startsWith('/invite/') || p.startsWith('/meet/') || window.location.search.length > 1)) {
            try { history.replaceState(null, '', '/'); } catch (e) {}
            dioxus.send(p);
        } else { dioxus.send(''); }
        "#,
    );
    ev.recv::<String>().await.ok().filter(|s| !s.is_empty())
}

/// A same-origin asset path that is safe to interpolate into CSS `url('...')`.
pub fn safe_asset_path(p: &str) -> bool {
    p.starts_with('/') && !p.contains("..") && p.len() < 300 && p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

/// Open a URL in a new tab / the system browser (http/https only).
pub fn open_external_url(url: &str) {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return;
    }
    let u = serde_json::to_string(url).unwrap_or_default();
    let _ = dioxus::document::eval(&format!("try {{ window.open({u}, '_blank', 'noopener,noreferrer'); }} catch (e) {{}}"));
}

/// Move keyboard focus into a freshly opened dialog so Escape and Tab work.
pub fn focus_selector(selector: &str) {
    let s = serde_json::to_string(selector).unwrap_or_default();
    let _ = dioxus::document::eval(&format!(
        "setTimeout(() => {{ const el = document.querySelector({s}); if (el) {{ if (!el.hasAttribute('tabindex')) el.setAttribute('tabindex', '-1'); el.focus({{ preventScroll: true }}); }} }}, 40);"
    ));
}

/// Public origin of the web app (for shareable links).
pub async fn app_origin() -> String {
    let mut ev = dioxus::document::eval("dioxus.send(window.location.origin || '');");
    let o = ev.recv::<String>().await.unwrap_or_default();
    if o.starts_with("http") { o } else { api_base() }
}

/// Share a link with the OS share sheet when available, otherwise copy it.
pub fn share_link(title: &str, url: &str) {
    let t = serde_json::to_string(title).unwrap_or_default();
    let u = serde_json::to_string(url).unwrap_or_default();
    let _ = dioxus::document::eval(&format!(
        "(async () => {{ try {{ if (navigator.share) {{ await navigator.share({{ title: {t}, url: {u} }}); return; }} }} catch (e) {{}} try {{ await navigator.clipboard.writeText({u}); }} catch (e) {{}} }})();"
    ));
}

/// Install the inactivity watcher once. Resolves every time the user comes back
/// after being idle (or away) for at least `minutes`.
pub fn idle_watch(minutes: u32) -> dioxus::document::Eval {
    dioxus::document::eval(&format!(
        r#"
        window.__cwIdleLimit = {minutes} * 60 * 1000;
        window.__cwIdleSend = (m) => dioxus.send(m);
        if (!window.__cwIdle) {{
            window.__cwIdle = true;
            const KEY = 'chatw_last_active';
            window.__cwLast = Number(localStorage.getItem(KEY) || Date.now());
            const mark = () => {{
                const now = Date.now();
                if (now - window.__cwLast >= window.__cwIdleLimit) window.__cwIdleSend('lock');
                window.__cwLast = now;
                localStorage.setItem(KEY, String(now));
            }};
            ['pointerdown', 'keydown', 'wheel', 'touchstart'].forEach((e) => window.addEventListener(e, mark, {{ passive: true }}));
            document.addEventListener('visibilitychange', () => {{ if (!document.hidden) mark(); }});
            setInterval(() => {{ if (Date.now() - window.__cwLast >= window.__cwIdleLimit) window.__cwIdleSend('lock'); }}, 30000);
        }}
        if (Date.now() - window.__cwLast >= window.__cwIdleLimit) window.__cwIdleSend('lock');
        "#
    ))
}

/// Watch paste events on a composer textarea: images and spreadsheet (tab-separated)
/// text are reported instead of being pasted when `intercept_tables` is true.
pub fn paste_watch(input_id: &str, intercept_tables: bool) -> dioxus::document::Eval {
    let id = serde_json::to_string(input_id).unwrap_or_default();
    dioxus::document::eval(&format!(
        r#"
        const attach = () => {{
            const el = document.getElementById({id});
            if (!el) {{ setTimeout(attach, 300); return; }}
            if (el.__cwPaste) return;
            el.__cwPaste = true;
            el.addEventListener('paste', async (e) => {{
                const cd = e.clipboardData;
                if (!cd) return;
                const file = [...(cd.files || [])].find((f) => f.type.startsWith('image/'));
                if (file) {{
                    e.preventDefault();
                    if (file.size > 8 * 1024 * 1024) {{ dioxus.send({{ kind: 'error', text: 'too_large' }}); return; }}
                    const buf = new Uint8Array(await file.arrayBuffer());
                    let s = '';
                    for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
                    dioxus.send({{ kind: 'image', b64: btoa(s), mime: file.type }});
                    return;
                }}
                const text = cd.getData('text/plain') || '';
                if ({intercept_tables} && text.includes('\t')) {{
                    const rows = text.replace(/\r/g, '').split('\n').filter((r) => r.length);
                    if (rows.length >= 1 && rows.some((r) => r.includes('\t'))) {{
                        e.preventDefault();
                        dioxus.send({{ kind: 'table', text: rows.join('\n') }});
                    }}
                }}
            }});
        }};
        attach();
        "#
    ))
}

/// Render tab-separated text as a PNG table image (for "paste Excel as image").
pub async fn render_table_png(tsv: &str) -> Result<Vec<u8>, String> {
    let t = serde_json::to_string(tsv).unwrap_or_default();
    let mut ev = dioxus::document::eval(&format!(
        r#"
        (async () => {{
            const rows = {t}.split('\n').slice(0, 60).map((r) => r.split('\t').slice(0, 20));
            const cols = Math.max(...rows.map((r) => r.length));
            const c = document.createElement('canvas');
            const ctx = c.getContext('2d');
            const font = '14px Inter, "Segoe UI", Roboto, "Noto Sans", Arial, sans-serif';
            ctx.font = font;
            const pad = 10, rowH = 30;
            const widths = Array.from({{ length: cols }}, (_, i) => Math.min(280, Math.max(48, ...rows.map((r) => ctx.measureText(r[i] || '').width + pad * 2))));
            const W = Math.ceil(widths.reduce((a, b) => a + b, 0)), H = rows.length * rowH;
            const scale = 2;
            c.width = W * scale; c.height = H * scale;
            ctx.scale(scale, scale);
            ctx.fillStyle = '#FFFFFF'; ctx.fillRect(0, 0, W, H);
            rows.forEach((r, ri) => {{
                if (ri === 0) {{ ctx.fillStyle = '#EAF0FD'; ctx.fillRect(0, 0, W, rowH); }}
                else if (ri % 2 === 0) {{ ctx.fillStyle = '#F6F8FB'; ctx.fillRect(0, ri * rowH, W, rowH); }}
                let x = 0;
                for (let ci = 0; ci < cols; ci++) {{
                    const txt = r[ci] || '';
                    ctx.fillStyle = ri === 0 ? '#1B2440' : '#2B3245';
                    ctx.font = (ri === 0 ? '600 ' : '') + font;
                    let shown = txt;
                    while (shown.length > 1 && ctx.measureText(shown).width > widths[ci] - pad * 2) shown = shown.slice(0, -2) + '…';
                    const num = /^-?[\d.,]+%?$/.test(txt.trim());
                    ctx.textAlign = num ? 'right' : 'left';
                    ctx.fillText(shown, num ? x + widths[ci] - pad : x + pad, ri * rowH + 20);
                    x += widths[ci];
                }}
            }});
            const blob = await new Promise((res) => c.toBlob(res, 'image/png'));
            const buf = new Uint8Array(await blob.arrayBuffer());
            let s = '';
            for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
            dioxus.send(btoa(s));
        }})();
        "#
    ));
    let b64 = ev.recv::<String>().await.map_err(|_| "render_failed".to_string())?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "decode_failed".to_string())
}

/// Image size (width, height) of encoded bytes, via the browser decoder.
pub async fn image_bytes_to_data_url(bytes: &[u8], mime: &str) -> String {
    use base64::Engine;
    format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Minutes left until local midnight (custom status "until the end of today").
pub fn minutes_until_midnight() -> i64 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let used = now.hour() as i64 * 60 + now.minute() as i64;
    (24 * 60 - used).clamp(1, 24 * 60)
}

/// If the chosen file is a GIF or WebP, return its bytes untouched (so an
/// animated cover keeps moving); `Ok(None)` for other images. The server checks
/// the real type again.
pub async fn read_animated_image(input_id: &str, max_bytes: u32) -> Result<Option<Vec<u8>>, String> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            const input = document.getElementById({id_json});
            if (!input || !input.files || input.files.length === 0) {{ dioxus.send({{ error: 'no_file' }}); return; }}
            const file = input.files[0];
            if (file.type !== 'image/gif' && file.type !== 'image/webp') {{ dioxus.send({{ skip: true }}); return; }}
            if (file.size > {max_bytes}) {{ input.value = ''; dioxus.send({{ error: 'too_large' }}); return; }}
            const buf = new Uint8Array(await file.arrayBuffer());
            let bin = '';
            for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
            input.value = '';
            dioxus.send({{ b64: btoa(bin) }});
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val: serde_json::Value = eval.recv().await.map_err(|_| "eval_failed".to_string())?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    if val.get("skip").is_some() {
        return Ok(None);
    }
    let b64 = val.get("b64").and_then(|v| v.as_str()).ok_or_else(|| "missing_b64".to_string())?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(b64).map(Some).map_err(|_| "bad_b64".to_string())
}
