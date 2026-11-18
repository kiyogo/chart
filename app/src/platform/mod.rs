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
    let emoji_json = serde_json::to_string(emoji).unwrap_or_else(|_| "\"\"".to_string());
    let cur_json = serde_json::to_string(current_text).unwrap_or_else(|_| "\"\"".to_string());
    let js = format!(
        r#"
        (() => {{
            const input = document.getElementById('dm-chat-input');
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
