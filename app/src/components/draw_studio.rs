//! In-app drawing studio: draw simple artwork (avatar decoration, nameplate or
//! profile effect) on a canvas, then upload it as a PNG to the media library.

use dioxus::prelude::*;

use crate::{
    components::icons::{CloseIcon, SparkleIcon},
    services::library,
};

const STUDIO_JS: &str = r#"
window.__cwStudio = window.__cwStudio || {
  init(id) {
    const c = document.getElementById(id);
    if (!c || c.__cw) return;
    c.__cw = { color: '#2764E7', size: 8, mode: 'pen', undo: [] };
    const ctx = c.getContext('2d');
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    let drawing = false, last = null;
    const pos = (e) => { const r = c.getBoundingClientRect(); return { x: (e.clientX - r.left) * c.width / r.width, y: (e.clientY - r.top) * c.height / r.height }; };
    const snap = () => { const st = c.__cw; st.undo.push(ctx.getImageData(0, 0, c.width, c.height)); if (st.undo.length > 40) st.undo.shift(); };
    const style = () => { const st = c.__cw; ctx.globalCompositeOperation = st.mode === 'eraser' ? 'destination-out' : 'source-over'; ctx.strokeStyle = st.color; ctx.fillStyle = st.color; ctx.lineWidth = st.size; };
    const star = (p) => {
      const st = c.__cw, r = st.size * 2.6; style(); ctx.beginPath();
      for (let i = 0; i < 8; i++) { const a = Math.PI / 4 * i - Math.PI / 2, rr = i % 2 ? r * 0.32 : r; ctx.lineTo(p.x + Math.cos(a) * rr, p.y + Math.sin(a) * rr); }
      ctx.closePath(); ctx.fill();
    };
    c.addEventListener('pointerdown', (e) => {
      e.preventDefault(); snap(); const p = pos(e);
      if (c.__cw.mode === 'star') { star(p); return; }
      drawing = true; last = p; c.setPointerCapture(e.pointerId); style();
      ctx.beginPath(); ctx.arc(p.x, p.y, c.__cw.size / 2, 0, Math.PI * 2); ctx.fill();
    });
    c.addEventListener('pointermove', (e) => {
      if (!drawing) return; const p = pos(e); style();
      ctx.beginPath(); ctx.moveTo(last.x, last.y); ctx.lineTo(p.x, p.y); ctx.stroke(); last = p;
    });
    const end = () => { drawing = false; };
    c.addEventListener('pointerup', end); c.addEventListener('pointercancel', end); c.addEventListener('pointerleave', end);
  },
  cmd(id, name, arg) {
    const c = document.getElementById(id); if (!c || !c.__cw) return null;
    const ctx = c.getContext('2d'), st = c.__cw;
    if (name === 'color') st.color = arg;
    if (name === 'size') st.size = Number(arg);
    if (name === 'mode') st.mode = arg;
    if (name === 'clear') { st.undo.push(ctx.getImageData(0, 0, c.width, c.height)); ctx.clearRect(0, 0, c.width, c.height); }
    if (name === 'undo') { const d = st.undo.pop(); if (d) ctx.putImageData(d, 0, 0); }
    if (name === 'export') return c.toDataURL('image/png');
    return null;
  }
};
"#;

pub const PALETTE: [&str; 12] = [
    "#1B2440", "#FFFFFF", "#2764E7", "#7C3AED", "#DB2777", "#EF4444", "#F97316", "#F59E0B", "#22C55E", "#14B8A6", "#0EA5E9", "#94A3B8",
];

fn cmd(id: &str, name: &str, arg: &str) {
    let _ = dioxus::document::eval(&format!(
        "{STUDIO_JS}; window.__cwStudio.cmd({}, {}, {});",
        serde_json::to_string(id).unwrap_or_default(),
        serde_json::to_string(name).unwrap_or_default(),
        serde_json::to_string(arg).unwrap_or_default()
    ));
}

/// `target`: "decoration" (square, avatar guide), "nameplate" (wide) or "effect".
#[component]
pub fn DrawStudio(target: String, on_saved: EventHandler<String>, on_close: EventHandler<()>) -> Element {
    let canvas_id = format!("studio-{target}");
    let (w, h) = match target.as_str() {
        "nameplate" => (600, 120),
        "effect" => (480, 240),
        _ => (320, 320),
    };
    let mut color = use_signal(|| "#2764E7".to_string());
    let mut size = use_signal(|| 8u32);
    let mut mode = use_signal(|| "pen");
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    let id_init = canvas_id.clone();
    use_effect(move || {
        let _ = dioxus::document::eval(&format!(
            "{STUDIO_JS}; setTimeout(() => window.__cwStudio.init({}), 0);",
            serde_json::to_string(&id_init).unwrap_or_default()
        ));
    });

    let id_save = canvas_id.clone();
    let save = move |_| {
        let id = id_save.clone();
        busy.set(true);
        error.set(None);
        spawn(async move {
            let mut ev = dioxus::document::eval(&format!(
                "{STUDIO_JS}; dioxus.send(window.__cwStudio.cmd({}, 'export', null));",
                serde_json::to_string(&id).unwrap_or_default()
            ));
            let result: Result<String, String> = async {
                let data_url = ev.recv::<String>().await.map_err(|_| "Không xuất được hình vẽ".to_string())?;
                let b64 = data_url.split(',').nth(1).ok_or("Dữ liệu ảnh không hợp lệ")?;
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "Dữ liệu ảnh không hợp lệ".to_string())?;
                library::upload_media(bytes, "image/png").await.map_err(|_e| "Tải lên thất bại".to_string())
            }
            .await;
            match result {
                Ok(url) => on_saved.call(url),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let label = match target.as_str() {
        "nameplate" => "Vẽ bảng tên",
        "effect" => "Vẽ hiệu ứng hồ sơ",
        _ => "Vẽ khung ảnh đại diện",
    };
    let hint = match target.as_str() {
        "nameplate" => "Bảng tên hiển thị phía sau tên bạn trong danh sách trò chuyện và trên hồ sơ.",
        "effect" => "Hình vẽ được phủ lên hồ sơ; chọn kiểu chuyển động sau khi lưu.",
        _ => "Vòng tròn mờ là vị trí ảnh đại diện. Vẽ quanh viền để làm khung.",
    };
    let c_id = canvas_id.clone();
    let c_id2 = canvas_id.clone();
    let c_id3 = canvas_id.clone();
    let c_id4 = canvas_id.clone();
    let c_id5 = canvas_id.clone();

    rsx! {
        div { class: "studio",
            div { class: "studio-head",
                div {
                    div { class: "settings-group-title", "{label}" }
                    div { class: "form-hint", "{hint}" }
                }
                button { r#type: "button", class: "icon-btn", aria_label: "Đóng xưởng vẽ", onclick: move |_| on_close.call(()), CloseIcon {} }
            }
            div { class: "studio-stage studio-{target}",
                canvas { id: "{canvas_id}", width: "{w}", height: "{h}", class: "studio-canvas", aria_label: "{label}" }
            }
            div { class: "studio-tools",
                div { class: "studio-swatches", role: "radiogroup", aria_label: "Màu vẽ",
                    for c in PALETTE {
                        {
                            let id = c_id.clone();
                            rsx! {
                                button {
                                    key: "{c}",
                                    r#type: "button",
                                    class: if color() == c { "studio-swatch active" } else { "studio-swatch" },
                                    style: "background: {c};",
                                    aria_label: "Màu {c}",
                                    onclick: move |_| { color.set(c.to_string()); cmd(&id, "color", c); if mode() == "eraser" { mode.set("pen"); cmd(&id, "mode", "pen"); } },
                                }
                            }
                        }
                    }
                    input {
                        r#type: "color",
                        class: "studio-color",
                        aria_label: "Chọn màu tự do",
                        value: "{color}",
                        oninput: move |e| { let v = e.value(); cmd(&c_id2, "color", &v); color.set(v); },
                    }
                }
                div { class: "studio-row",
                    div { class: "segmented",
                        for (m, l) in [("pen", "Bút"), ("eraser", "Tẩy"), ("star", "Dấu sao")] {
                            {
                                let id = c_id3.clone();
                                rsx! {
                                    button { key: "{m}", r#type: "button", class: if mode() == m { "active" } else { "" },
                                        onclick: move |_| { mode.set(m); cmd(&id, "mode", m); },
                                        if m == "star" { SparkleIcon { size: 14 } }
                                        "{l}"
                                    }
                                }
                            }
                        }
                    }
                    label { class: "range-row", style: "max-width: 14rem;",
                        span { class: "form-label", "Cỡ" }
                        input { r#type: "range", min: "2", max: "40", value: "{size}", aria_label: "Cỡ nét",
                            oninput: move |e| { let v = e.value(); cmd(&c_id4, "size", &v); size.set(v.parse().unwrap_or(8)); } }
                        span { class: "range-value", "{size}px" }
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: { let id = c_id5.clone(); move |_| cmd(&id, "undo", "") }, "Hoàn tác" }
                    button { r#type: "button", class: "btn-secondary", onclick: { let id = canvas_id.clone(); move |_| cmd(&id, "clear", "") }, "Xóa hết" }
                }
            }
            if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
            div { class: "studio-foot",
                button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: save,
                    if busy() { span { class: "spinner" } } else { "Lưu và dùng" }
                }
            }
        }
    }
}
