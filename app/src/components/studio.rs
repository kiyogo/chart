//! ChatW Studio: a layered drawing and frame-by-frame animation editor.
//! The canvas work happens in `studio_engine.js`; this file is the interface,
//! project storage, GIF export and "use as" (avatar, cover, sticker, emoji,
//! GIF, decoration, nameplate, profile effect).

use base64::Engine;
use dioxus::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::{
    components::icons::CloseIcon,
    platform,
    services::{api, drawings, gif_export, library, stickers},
};

const ENGINE: &str = include_str!("studio_engine.js");

/// (id, label, width, height)
pub const TARGETS: [(&str, &str, u32, u32); 8] = [
    ("sticker", "Sticker", 320, 320),
    ("emoji", "Emoji", 128, 128),
    ("avatar", "Ảnh đại diện", 512, 512),
    ("cover", "Ảnh bìa", 960, 320),
    ("decoration", "Trang trí avatar", 320, 320),
    ("nameplate", "Bảng tên", 600, 120),
    ("effect", "Hiệu ứng hồ sơ", 480, 240),
    ("free", "Tự do", 512, 512),
];

const PALETTE: [&str; 18] = [
    "#000000", "#FFFFFF", "#1B2440", "#64748B", "#EF4444", "#F97316", "#F59E0B", "#FDE047", "#22C55E", "#14B8A6",
    "#0EA5E9", "#2764E7", "#6366F1", "#7C3AED", "#DB2777", "#F9A8D4", "#A16207", "#7F1D1D",
];

const TOOLS: [(&str, &str, &str); 12] = [
    ("brush", "Cọ vẽ", "M4 20c3 0 5-2 5-4.5S7 12 9.5 11L20 4l-7.5 10.5C11 17 9 15 7.5 15"),
    ("pencil", "Bút chì", "M4 20l1-4L16 5l3 3L8 19zM14 7l3 3"),
    ("eraser", "Tẩy", "M8 20h12M5 15l9-9 5 5-9 9H8z"),
    ("fill", "Tô màu", "M5 11l6-6 7 7-6 6zM19 15s2 2.5 2 3.8a2 2 0 01-4 0C17 17.5 19 15 19 15z"),
    ("line", "Đường thẳng", "M5 19L19 5"),
    ("rect", "Hình chữ nhật", "M5 6h14v12H5z"),
    ("ellipse", "Hình elip", "M12 5c4.4 0 8 3.1 8 7s-3.6 7-8 7-8-3.1-8-7 3.6-7 8-7z"),
    ("polygon", "Ngôi sao / Đa giác", "M12 3l2.6 5.6 6.1.7-4.5 4.2 1.2 6L12 16.6 6.6 19.5l1.2-6L3.3 9.3l6.1-.7z"),
    ("text", "Văn bản", "M5 6h14M12 6v13M9 19h6"),
    ("picker", "Hút màu", "M15 4l5 5-2 2-5-5zM13 6l-8 8v5h5l8-8"),
    ("move", "Di chuyển", "M12 3v18M3 12h18M12 3l-3 3M12 3l3 3M12 21l-3-3M12 21l3-3M3 12l3-3M3 12l3 3M21 12l-3-3M21 12l-3 3"),
    ("clear", "Xóa lớp", "M6 7h12M9 7V5h6v2M8 7l1 12h6l1-12"),
];

const PRESETS: [(&str, &str); 10] = [
    ("bounce", "Bounce"),
    ("spin", "Spin"),
    ("fadein", "Fade In"),
    ("fadeout", "Fade Out"),
    ("slidein", "Slide In"),
    ("pulse", "Pulse"),
    ("shake", "Shake"),
    ("zoomin", "Zoom In"),
    ("float", "Float"),
    ("wobble", "Wobble"),
];

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct LayerSt {
    name: String,
    visible: bool,
    opacity: f64,
    blend: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct St {
    w: u32,
    h: u32,
    fps: u32,
    #[serde(rename = "loop")]
    looping: bool,
    frame: usize,
    layer: usize,
    frames: usize,
    layers: Vec<LayerSt>,
    tool: String,
    color: String,
    size: f64,
    opacity: f64,
    fill: bool,
    tolerance: f64,
    text: String,
    font: f64,
    sides: u32,
    star: bool,
    zoom: f64,
    grid: bool,
    snap: bool,
    onion: bool,
    playing: bool,
    undo: usize,
    redo: usize,
    recent: Vec<String>,
    draft: String,
}

fn js(code: &str) {
    let _ = dioxus::document::eval(&format!("if (window.ChatwStudio) {{ {code} }}"));
}

fn q(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

fn decode_data_url(u: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(u.split(',').nth(1)?).ok()
}

/// Run an engine call (sync or async) and get its result as text.
async fn eval_string(code: &str) -> Option<String> {
    let mut ev = dioxus::document::eval(&format!(
        "(async () => {{ try {{ const r = await ({code}); dioxus.send(String(r)); }} catch (e) {{ dioxus.send('error:' + ((e && e.message) || e)); }} }})();"
    ));
    ev.recv::<String>().await.ok()
}

/// Every frame as RGBA, then an animated GIF.
async fn export_gif(st: &St) -> Result<Vec<u8>, String> {
    let mut frames = Vec::with_capacity(st.frames);
    for i in 0..st.frames {
        let b64 = eval_string(&format!("ChatwStudio.rgba({i})")).await.ok_or("Không đọc được khung hình")?;
        frames.push(base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "Dữ liệu khung hình hỏng")?);
    }
    gif_export::encode(st.w as u16, st.h as u16, &mut frames, st.fps, if st.looping { 0 } else { 1 })
}

async fn current_png() -> Result<Vec<u8>, String> {
    let url = eval_string("ChatwStudio.png()").await.ok_or("Không xuất được ảnh")?;
    decode_data_url(&url).ok_or_else(|| "Không xuất được ảnh".to_string())
}

fn slug(name: &str) -> String {
    let s: String = crate::components::composer::fold(name)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let s = s.trim_matches('_').chars().take(32).collect::<String>();
    if s.len() < 2 { "drawing".into() } else { s }
}

fn err_text(code: &str) -> String {
    match code {
        "too_many_layers" => "Tối đa 12 lớp".into(),
        "too_many_frames" => "Tối đa 60 khung hình".into(),
        "last_layer" => "Cần giữ ít nhất một lớp".into(),
        "last_frame" => "Cần giữ ít nhất một khung hình".into(),
        "project_limit_reached" => "Bạn đã có 100 dự án, hãy xóa bớt".into(),
        "project_too_large" | "payload_too_large" => "Dự án lớn hơn 6 MB, hãy giảm số khung hoặc kích thước".into(),
        "invalid_sticker_size" | "invalid_media_size" | "banner_too_large" => "Tệp quá lớn, hãy giảm số khung hoặc kích thước".into(),
        "emoji_name_taken" => "Đã có emoji trùng tên, hãy đổi tên dự án".into(),
        _ => "Không thực hiện được".into(),
    }
}

/// The full-screen studio. `target` picks the canvas preset for a new drawing.
#[component]
pub fn Studio(
    my_id: String,
    #[props(default)] target: Option<String>,
    #[props(default)] open_project: Option<String>,
    /// Called after "Dùng làm …" succeeds: (target, url).
    #[props(default)] on_used: Option<EventHandler<(String, String)>>,
    on_close: EventHandler<()>,
) -> Element {
    let start_target = target.clone().filter(|t| TARGETS.iter().any(|x| x.0 == t)).unwrap_or_else(|| "sticker".into());
    let (_, _, w0, h0) = TARGETS.iter().copied().find(|x| x.0 == start_target).unwrap_or(TARGETS[0]);
    let mut st = use_signal(|| St { w: w0, h: h0, zoom: 1.0, ..Default::default() });
    let mut kind = use_signal(|| start_target.clone());
    let mut name = use_signal(|| "Untitled".to_string());
    let mut project_id = use_signal(|| open_project.clone());
    let mut busy = use_signal(|| false);
    let mut toast = use_signal(|| Option::<(bool, String)>::None);
    let mut menu = use_signal(|| Option::<&'static str>::None);
    let mut has_draft = use_signal(|| false);
    let mut tween_preset = use_signal(|| "bounce".to_string());
    let mut tween_frames = use_signal(|| 8u32);
    let mut tween_easing = use_signal(|| "ease-in-out".to_string());
    let mut tween_custom = use_signal(|| false);
    let mut tw = use_signal(|| [0.0f64, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0]);
    let mut layer_drag_from = use_signal(|| Option::<usize>::None);
    let mut layer_drag_over = use_signal(|| Option::<usize>::None);
    let draft_key = format!("chatw_studio_draft_{my_id}");
    let me_sig = use_signal(|| my_id.clone());

    // Load the engine once, bind the canvases and stream its state into `st`.
    let dk = draft_key.clone();
    let open0 = open_project.clone();
    use_hook(move || {
        let init = format!(
            "{ENGINE}; ChatwStudio.init('studio-view', 'studio-overlay', {w0}, {h0}, {}); ChatwStudio.listen((s) => dioxus.send(s)); \
             dioxus.send({{ hasDraft: !!ChatwStudio.draft() }});",
            q(&dk)
        );
        spawn(async move {
            // Canvases are in the DOM after the first render.
            platform::sleep_ms(30).await;
            let mut ev = dioxus::document::eval(&init);
            if let Some(id) = open0 {
                spawn(async move {
                    platform::sleep_ms(120).await;
                    match drawings::open(&id).await {
                        Ok((n, k, data)) => {
                            name.set(n);
                            kind.set(k);
                            let _ = dioxus::document::eval(&format!("(async () => {{ await ChatwStudio.load({}); setTimeout(() => ChatwStudio.init('studio-view', 'studio-overlay', 0, 0, null), 30); }})();", data));
                        }
                        Err(e) => toast.set(Some((true, err_text(&e.code)))),
                    }
                });
            }
            while let Ok(v) = ev.recv::<serde_json::Value>().await {
                if let Some(d) = v.get("hasDraft").and_then(|x| x.as_bool()) {
                    has_draft.set(d && project_id.peek().is_none());
                    continue;
                }
                if let Ok(s) = serde_json::from_value::<St>(v) {
                    let resized = { let o = st.peek(); o.w != s.w || o.h != s.h };
                    st.set(s);
                    // Re-render after the canvases got their new size; refresh thumbnails.
                    let _ = dioxus::document::eval(if resized {
                        "setTimeout(() => { ChatwStudio.init('studio-view', 'studio-overlay', 0, 0, null); ChatwStudio.thumbs(); }, 30);"
                    } else {
                        "setTimeout(() => ChatwStudio.thumbs(), 0);"
                    });
                }
            }
        });
    });

    let s = st();
    let tool_set = |t: &str| js(&format!("ChatwStudio.set('tool', {})", q(t)));

    let save_project = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        spawn(async move {
            let data = eval_string("ChatwStudio.project()").await.unwrap_or_default();
            let thumb = eval_string("ChatwStudio.thumbnail()").await.unwrap_or_default();
            let id = project_id();
            match drawings::save(id.as_deref(), &name(), &kind(), &data, &thumb).await {
                Ok(new_id) => {
                    project_id.set(Some(new_id));
                    js("ChatwStudio.dropDraft()");
                    toast.set(Some((false, "Đã lưu dự án".into())));
                }
                Err(e) => toast.set(Some((true, err_text(&e.code)))),
            }
            busy.set(false);
        });
    };

    let mut export = move |what: &'static str| {
        menu.set(None);
        busy.set(true);
        spawn(async move {
            let st_now = st.peek().clone();
            let base = slug(&name());
            let result: Result<(), String> = async {
                match what {
                    "png" => {
                        let b = current_png().await?;
                        js(&format!("ChatwStudio.download({}, {}, 'image/png')", q(&format!("{base}.png")), q(&base64::engine::general_purpose::STANDARD.encode(b))));
                    }
                    "gif" => {
                        let b = export_gif(&st_now).await?;
                        js(&format!("ChatwStudio.download({}, {}, 'image/gif')", q(&format!("{base}.gif")), q(&base64::engine::general_purpose::STANDARD.encode(b))));
                    }
                    _ => {
                        let data = eval_string("ChatwStudio.project()").await.ok_or("Không xuất được dự án")?;
                        js(&format!("ChatwStudio.download({}, {}, 'application/json')", q(&format!("{base}.chatwdraw.json")), q(&base64::engine::general_purpose::STANDARD.encode(data))));
                    }
                }
                Ok(())
            }
            .await;
            toast.set(Some(match result {
                Ok(()) => (false, "Đã xuất tệp".into()),
                Err(e) => (true, e),
            }));
            busy.set(false);
        });
    };

    let mut use_as = move |target: &'static str| {
        menu.set(None);
        busy.set(true);
        toast.set(None);
        spawn(async move {
            let st_now = st.peek().clone();
            let animated = st_now.frames > 1;
            let nm = name();
            let result: Result<(String, String), String> = async {
                let (bytes, mime) = if animated && target != "avatar" {
                    (export_gif(&st_now).await?, "image/gif")
                } else {
                    (current_png().await?, "image/png")
                };
                let e = |x: crate::models::ApiError| err_text(&x.code);
                match target {
                    "avatar" => {
                        let r = api::users_upload_avatar(bytes).await.map_err(e)?;
                        Ok(("Đã đặt làm ảnh đại diện".into(), r.avatar_url))
                    }
                    "cover" => {
                        let url = api::users_upload_banner(bytes).await.map_err(e)?;
                        Ok((if animated { "Đã đặt làm ảnh bìa động".into() } else { "Đã đặt làm ảnh bìa".into() }, url))
                    }
                    "sticker" => {
                        let (url, anim) = stickers::stickers_upload_image(bytes, mime).await.map_err(e)?;
                        let me = Some(me_sig.peek().clone());
                        let packs = stickers::stickers_list_packs().await.map_err(e)?;
                        let pack = match packs.into_iter().find(|p| p.name == "My Drawings" && p.author_id == me) {
                            Some(p) => p,
                            None => stickers::stickers_create_pack("My Drawings", Some("Stickers drawn in ChatW Studio"), Some(&url)).await.map_err(e)?,
                        };
                        stickers::stickers_add_sticker(&pack.id, &nm, &url, anim).await.map_err(e)?;
                        Ok(("Đã thêm vào bộ sticker My Drawings".into(), url))
                    }
                    "emoji" => {
                        let url = library::upload_media(bytes, mime).await.map_err(e)?;
                        library::emojis_create(&slug(&nm), &url).await.map_err(e)?;
                        Ok((format!("Đã thêm emoji :{}:", slug(&nm)), url))
                    }
                    "gif" => {
                        if !animated {
                            return Err("GIF cần ít nhất 2 khung hình".into());
                        }
                        let url = library::upload_media(bytes, mime).await.map_err(e)?;
                        library::gifs_create(&nm, &url).await.map_err(e)?;
                        Ok(("Đã thêm vào My GIFs".into(), url))
                    }
                    _ => {
                        let url = library::upload_media(bytes, mime).await.map_err(e)?;
                        let fields = match target {
                            "decoration" => json!({ "custom_decoration_url": url, "avatar_decoration": "custom" }),
                            "nameplate" => json!({ "custom_nameplate_url": url, "nameplate": "custom" }),
                            _ => json!({ "custom_effect_url": url, "profile_effect": "custom" }),
                        };
                        api::update_my_profile_fields(fields).await.map_err(e)?;
                        Ok(("Đã áp dụng lên hồ sơ".into(), url))
                    }
                }
            }
            .await;
            match result {
                Ok((msg, url)) => {
                    toast.set(Some((false, msg)));
                    if let Some(h) = on_used {
                        h.call((target.to_string(), url));
                    }
                }
                Err(m) => toast.set(Some((true, m))),
            }
            busy.set(false);
        });
    };

    let run = move |code: &'static str| {
        spawn(async move {
            if let Some(r) = eval_string(code).await {
                if r != "null" && r != "undefined" && !r.is_empty() {
                    toast.set(Some((true, err_text(&r))));
                }
            }
        });
    };

    let apply_tween = move |_| {
        let t = tw();
        let opt = if tween_custom() {
            json!({ "frames": tween_frames(), "easing": tween_easing(),
                "from": { "x": t[0], "y": t[1], "scale": t[2], "rot": t[3], "alpha": t[4] },
                "to": { "x": t[5], "y": t[6], "scale": t[7], "rot": t[8], "alpha": t[9] } })
        } else {
            json!({ "frames": tween_frames(), "easing": tween_easing(), "preset": tween_preset() })
        };
        menu.set(None);
        spawn(async move {
            if let Some(r) = eval_string(&format!("ChatwStudio.tween({opt})")).await {
                if r != "null" {
                    toast.set(Some((true, err_text(&r))));
                } else {
                    toast.set(Some((false, "Đã tạo chuyển động trên lớp đang chọn".into())));
                }
            }
        });
    };

    let zoom_px = (s.w as f64 * s.zoom).round();
    let zoom_py = (s.h as f64 * s.zoom).round();
    let kind_label = TARGETS.iter().find(|x| x.0 == kind()).map(|x| x.1).unwrap_or("Tự do");

    rsx! {
        div { class: "studio-shell", role: "dialog", aria_modal: "true", aria_label: "ChatW Studio", tabindex: "-1",
            onkeydown: move |e| {
                if e.key() == Key::Escape { if menu().is_some() { menu.set(None) } else { on_close.call(()) } }
            },
            header { class: "studio-top",
                div { class: "studio-brand", "ChatW Studio" }
                input { class: "studio-name", aria_label: "Tên dự án", maxlength: "64", value: "{name}", oninput: move |e| name.set(e.value()) }
                span { class: "studio-kind", "{kind_label} · {s.w}×{s.h}" }
                div { class: "studio-top-actions",
                    button { r#type: "button", class: "btn-small-secondary", title: "Hoàn tác", disabled: s.undo == 0, onclick: move |_| js("ChatwStudio.undo()"), "Hoàn tác" }
                    button { r#type: "button", class: "btn-small-secondary", title: "Làm lại", disabled: s.redo == 0, onclick: move |_| js("ChatwStudio.redo()"), "Làm lại" }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| menu.set(if menu() == Some("new") { None } else { Some("new") }), "Mới" }
                    label { class: "btn-small-secondary", r#for: "studio-import", role: "button", tabindex: "0", "Nhập" }
                    input { id: "studio-import", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/gif,image/avif,.json,application/json", style: "display: none;",
                        onchange: move |_| {
                            spawn(async move {
                                let r = eval_string("ChatwStudio.readFile('studio-import')").await.unwrap_or_default();
                                toast.set(Some(match r.as_str() {
                                    "project" => (false, "Đã mở tệp dự án".into()),
                                    "image" => (false, "Đã nhập ảnh vào lớp mới".into()),
                                    "no_file" => return,
                                    "too_large" => (true, "Tệp lớn hơn 8 MB".into()),
                                    "unsupported" => (true, "Chỉ nhận PNG, JPEG, WebP, GIF, AVIF hoặc tệp .chatwdraw.json".into()),
                                    other => (true, err_text(other)),
                                }));
                            });
                        },
                    }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| menu.set(if menu() == Some("export") { None } else { Some("export") }), "Xuất" }
                    button { r#type: "button", class: "btn-secondary", disabled: busy(), onclick: save_project, if project_id().is_some() { "Lưu" } else { "Lưu dự án" } }
                    button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: move |_| menu.set(if menu() == Some("use") { None } else { Some("use") }), "Dùng làm" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng ChatW Studio", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                if menu() == Some("new") {
                    div { class: "account-menu studio-menu", role: "menu",
                        div { class: "menu-label", "Bản vẽ mới" }
                        for (id, label, w, h) in TARGETS {
                            button { key: "{id}", r#type: "button", role: "menuitem", class: "account-menu-item",
                                onclick: move |_| { menu.set(None); kind.set(id.to_string()); project_id.set(None); js(&format!("ChatwStudio.newDoc({w}, {h})")); },
                                "{label}" span { class: "member-sub", " {w}×{h}" }
                            }
                        }
                    }
                }
                if menu() == Some("export") {
                    div { class: "account-menu studio-menu", role: "menu",
                        button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| export("png"), "PNG" }
                        button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| export("gif"), "GIF" }
                        button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| export("project"), "Tệp dự án" }
                    }
                }
                if menu() == Some("use") {
                    div { class: "account-menu studio-menu right", role: "menu",
                        for (id, label) in [("avatar", "Ảnh đại diện"), ("cover", "Ảnh bìa"), ("sticker", "Sticker"), ("emoji", "Emoji"), ("gif", "My GIFs"), ("decoration", "Trang trí avatar"), ("nameplate", "Bảng tên"), ("effect", "Hiệu ứng hồ sơ")] {
                            button { key: "{id}", r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| use_as(id), "{label}" }
                        }
                    }
                }
            }
            if has_draft() {
                div { class: "studio-banner",
                    span { "Có bản nháp chưa lưu từ lần trước." }
                    button { r#type: "button", class: "btn-small-secondary",
                        onclick: move |_| { has_draft.set(false); js("const d = ChatwStudio.draft(); if (d) ChatwStudio.load(d).then(() => setTimeout(() => { ChatwStudio.init('studio-view', 'studio-overlay', 0, 0, null); }, 30));"); },
                        "Khôi phục" }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| { has_draft.set(false); js("ChatwStudio.dropDraft()"); }, "Bỏ" }
                }
            }
            if let Some((err, m)) = toast() {
                div { class: if err { "status-badge error studio-toast" } else { "status-badge success studio-toast" }, role: "status",
                    "{m}"
                    button { r#type: "button", class: "link-btn", onclick: move |_| toast.set(None), " Đóng" }
                }
            }
            div { class: "studio-main",
                nav { class: "studio-tools", aria_label: "Công cụ vẽ",
                    for (id, label, path) in TOOLS {
                        button { key: "{id}", r#type: "button", title: "{label}", aria_label: "{label}", aria_pressed: "{s.tool == id}",
                            class: if s.tool == id { "studio-tool active" } else { "studio-tool" },
                            onclick: move |_| if id == "clear" { js("ChatwStudio.clearLayer()") } else { tool_set(id) },
                            svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                                path { d: "{path}" }
                            }
                        }
                    }
                }
                div { class: "studio-stage-wrap",
                    onwheel: move |e| {
                        if e.modifiers().ctrl() {
                            e.prevent_default();
                            let dy = e.delta().strip_units().y;
                            let z = (st.peek().zoom * if dy < 0.0 { 1.25 } else { 0.8 }).clamp(0.25, 8.0);
                            js(&format!("ChatwStudio.set('zoom', {z})"));
                        }
                    },
                    div { class: "studio-board", style: "width: {zoom_px}px; height: {zoom_py}px;",
                        canvas { id: "studio-view", class: "studio-canvas", width: "{s.w}", height: "{s.h}", aria_hidden: "true" }
                        canvas { id: "studio-overlay", class: "studio-canvas overlay tool-{s.tool}", width: "{s.w}", height: "{s.h}", aria_label: "Vùng vẽ {s.w}×{s.h}", tabindex: "0",
                            onkeydown: move |e| {
                                let k = e.key().to_string();
                                if k == " " || k == "Spacebar" { e.prevent_default(); }
                                let code = format!("ChatwStudio.key({{ key: {}, ctrlKey: {}, metaKey: {}, shiftKey: {}, target: {{}} }})", q(&k), e.modifiers().ctrl(), e.modifiers().meta(), e.modifiers().shift());
                                js(&code);
                            },
                        }
                    }
                }
                aside { class: "studio-side",
                    div { class: "studio-panel",
                        div { class: "studio-panel-title", "Màu" }
                        div { class: "studio-palette",
                            input { r#type: "color", aria_label: "Chọn màu", value: "{s.color}", oninput: move |e| js(&format!("ChatwStudio.set('color', {})", q(&e.value().to_uppercase()))) }
                            for c in PALETTE {
                                button { key: "{c}", r#type: "button", class: if s.color.eq_ignore_ascii_case(c) { "studio-swatch active" } else { "studio-swatch" }, style: "background: {c};", aria_label: "Màu {c}",
                                    onclick: move |_| js(&format!("ChatwStudio.set('color', {})", q(c))) }
                            }
                        }
                        if !s.recent.is_empty() {
                            div { class: "studio-palette recent", aria_label: "Màu vừa dùng",
                                for c in s.recent.clone() {
                                    button { key: "r{c}", r#type: "button", class: "studio-swatch small", style: "background: {c};", aria_label: "Màu vừa dùng {c}",
                                        onclick: move |_| js(&format!("ChatwStudio.set('color', {})", q(&c))) }
                                }
                            }
                        }
                        label { class: "range-row",
                            span { "Cỡ" }
                            input { r#type: "range", min: "1", max: "128", value: "{s.size}", aria_label: "Cỡ nét", oninput: move |e| js(&format!("ChatwStudio.set('size', {})", e.value().parse::<f64>().unwrap_or(8.0))) }
                            span { class: "range-value", "{s.size}px" }
                        }
                        label { class: "range-row",
                            span { "Độ đậm" }
                            input { r#type: "range", min: "5", max: "100", value: "{(s.opacity * 100.0).round()}", aria_label: "Độ đậm của nét", oninput: move |e| js(&format!("ChatwStudio.set('opacity', {})", e.value().parse::<f64>().unwrap_or(100.0) / 100.0)) }
                            span { class: "range-value", "{(s.opacity * 100.0).round()}%" }
                        }
                        if ["rect", "ellipse", "polygon"].contains(&s.tool.as_str()) {
                            label { class: "demo-check", input { r#type: "checkbox", checked: s.fill, onchange: move |e| js(&format!("ChatwStudio.set('fill', {})", e.checked())) } "Tô đặc hình" }
                        }
                        if s.tool == "polygon" {
                            label { class: "range-row",
                                span { "Số cạnh" }
                                input { r#type: "range", min: "3", max: "12", value: "{s.sides}", aria_label: "Số cạnh", oninput: move |e| js(&format!("ChatwStudio.set('sides', {})", e.value().parse::<u32>().unwrap_or(5))) }
                                span { class: "range-value", "{s.sides}" }
                            }
                            label { class: "demo-check", input { r#type: "checkbox", checked: s.star, onchange: move |e| js(&format!("ChatwStudio.set('star', {})", e.checked())) } "Hình sao" }
                        }
                        if s.tool == "fill" {
                            label { class: "range-row",
                                span { "Dung sai" }
                                input { r#type: "range", min: "0", max: "128", value: "{s.tolerance}", aria_label: "Dung sai tô màu", oninput: move |e| js(&format!("ChatwStudio.set('tolerance', {})", e.value().parse::<f64>().unwrap_or(24.0))) }
                                span { class: "range-value", "{s.tolerance}" }
                            }
                        }
                        if s.tool == "text" {
                            textarea { rows: "2", maxlength: "200", aria_label: "Chữ sẽ đặt lên hình", value: "{s.text}", oninput: move |e| js(&format!("ChatwStudio.set('text', {})", q(&e.value()))) }
                            label { class: "range-row",
                                span { "Cỡ chữ" }
                                input { r#type: "range", min: "8", max: "160", value: "{s.font}", aria_label: "Cỡ chữ", oninput: move |e| js(&format!("ChatwStudio.set('font', {})", e.value().parse::<f64>().unwrap_or(32.0))) }
                                span { class: "range-value", "{s.font}px" }
                            }
                        }
                        div { class: "studio-toggles",
                            label { class: "demo-check", input { r#type: "checkbox", checked: s.grid, onchange: move |e| js(&format!("ChatwStudio.set('grid', {})", e.checked())) } "Lưới" }
                            label { class: "demo-check", input { r#type: "checkbox", checked: s.snap, onchange: move |e| js(&format!("ChatwStudio.set('snap', {})", e.checked())) } "Bám lưới" }
                            label { class: "demo-check", input { r#type: "checkbox", checked: s.onion, onchange: move |e| js(&format!("ChatwStudio.set('onion', {})", e.checked())) } "Onion skin" }
                        }
                        div { class: "studio-zoom",
                            button { r#type: "button", class: "btn-small-secondary", aria_label: "Thu nhỏ", onclick: move |_| js(&format!("ChatwStudio.set('zoom', {})", (st.peek().zoom * 0.8).max(0.25))), "−" }
                            span { "{(s.zoom * 100.0).round()}%" }
                            button { r#type: "button", class: "btn-small-secondary", aria_label: "Phóng to", onclick: move |_| js(&format!("ChatwStudio.set('zoom', {})", (st.peek().zoom * 1.25).min(8.0))), "+" }
                            button { r#type: "button", class: "btn-small-secondary", onclick: move |_| js("ChatwStudio.set('zoom', 1)"), "100%" }
                        }
                    }
                    div { class: "studio-panel",
                        div { class: "studio-panel-head",
                            span { class: "studio-panel-title", "Lớp" }
                            button { r#type: "button", class: "btn-small-secondary", onclick: move |_| run("ChatwStudio.addLayer()"), "+ Lớp" }
                        }
                        div { class: "studio-layers", "data-drag-group": "studio-layers",
                            for (i, l) in s.layers.iter().enumerate().rev() {
                                {
                                    let dragging_now = layer_drag_from() == Some(i);
                                    let over_cls = match (layer_drag_from(), layer_drag_over()) {
                                        (Some(from), Some(ov)) if ov == i && from != i => {
                                            if i > from { " drag-over-before" } else { " drag-over-after" }
                                        }
                                        _ => "",
                                    };
                                    let row_cls = format!(
                                        "studio-layer draggable-item{}{}{}",
                                        if i == s.layer { " active" } else { "" },
                                        if dragging_now { " is-dragging" } else { "" },
                                        over_cls
                                    );
                                    rsx! {
                                        div {
                                            key: "{i}-{l.name}",
                                            class: "{row_cls}",
                                            draggable: "true",
                                            "data-drag-item": "true",
                                            "data-drag-key": "{i}",
                                            onclick: move |_| js(&format!("ChatwStudio.selectLayer({i})")),
                                            ondragstart: move |_| {
                                                layer_drag_from.set(Some(i));
                                                layer_drag_over.set(None);
                                            },
                                            ondragover: move |e| {
                                                e.prevent_default();
                                                if layer_drag_from().is_some() && layer_drag_over() != Some(i) {
                                                    layer_drag_over.set(Some(i));
                                                }
                                            },
                                            ondrop: move |e| {
                                                e.prevent_default();
                                                if let Some(from) = layer_drag_from() {
                                                    if from != i {
                                                        js(&format!("ChatwStudio.reorderLayer({from}, {i})"));
                                                    }
                                                }
                                                layer_drag_from.set(None);
                                                layer_drag_over.set(None);
                                            },
                                            ondragend: move |_| {
                                                layer_drag_from.set(None);
                                                layer_drag_over.set(None);
                                            },
                                            button { r#type: "button", class: "studio-eye", aria_label: if l.visible { "Ẩn lớp" } else { "Hiện lớp" }, aria_pressed: "{l.visible}",
                                                onclick: { let v = l.visible; move |e: Event<MouseData>| { e.stop_propagation(); js(&format!("ChatwStudio.layerProp({i}, 'visible', {})", !v)) } },
                                                if l.visible { "◉" } else { "○" }
                                            }
                                            canvas { class: "studio-layer-thumb", width: "40", height: "40", "data-studio-layer": "{i}" }
                                            input { class: "studio-layer-name", value: "{l.name}", maxlength: "40", aria_label: "Tên lớp",
                                                onchange: move |e| js(&format!("ChatwStudio.layerProp({i}, 'name', {})", q(&e.value()))) }
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(l) = s.layers.get(s.layer).cloned() {
                            label { class: "range-row",
                                span { "Độ mờ lớp" }
                                input { r#type: "range", min: "0", max: "100", value: "{(l.opacity * 100.0).round()}", aria_label: "Độ mờ lớp",
                                    oninput: move |e| js(&format!("ChatwStudio.layerProp({}, 'opacity', {})", st.peek().layer, e.value().parse::<f64>().unwrap_or(100.0) / 100.0)) }
                                span { class: "range-value", "{(l.opacity * 100.0).round()}%" }
                            }
                            label { class: "range-row",
                                span { "Hòa trộn" }
                                select { aria_label: "Chế độ hòa trộn", value: "{l.blend}",
                                    onchange: move |e| js(&format!("ChatwStudio.layerProp({}, 'blend', {})", st.peek().layer, q(&e.value()))),
                                    for (b, bl) in [("normal", "Normal"), ("multiply", "Multiply"), ("screen", "Screen"), ("overlay", "Overlay"), ("darken", "Darken"), ("lighten", "Lighten")] {
                                        option { key: "{b}", value: "{b}", selected: l.blend == b, "{bl}" }
                                    }
                                }
                            }
                        }
                        div { class: "studio-btn-row",
                            button { r#type: "button", class: "btn-small-secondary", onclick: move |_| run("ChatwStudio.duplicateLayer()"), "Nhân bản" }
                            button { r#type: "button", class: "btn-small-secondary", onclick: move |_| js("ChatwStudio.mergeDown()"), "Gộp xuống" }
                            button { r#type: "button", class: "btn-small-secondary danger", onclick: move |_| run("ChatwStudio.deleteLayer()"), "Xóa lớp" }
                        }
                    }
                }
            }
            footer { class: "studio-timeline",
                div { class: "studio-timeline-bar",
                    button { r#type: "button", class: "btn-small-secondary", aria_label: if s.playing { "Tạm dừng" } else { "Phát" },
                        onclick: move |_| js(if st.peek().playing { "ChatwStudio.pause()" } else { "ChatwStudio.play()" }),
                        if s.playing { "❚❚ Dừng" } else { "▶ Phát" }
                    }
                    label { class: "studio-inline",
                        "FPS"
                        input { r#type: "number", min: "1", max: "30", value: "{s.fps}", aria_label: "Số khung hình mỗi giây",
                            onchange: move |e| js(&format!("ChatwStudio.setDoc('fps', {})", e.value().parse::<u32>().unwrap_or(8).clamp(1, 30))) }
                    }
                    label { class: "demo-check", input { r#type: "checkbox", checked: s.looping, onchange: move |e| js(&format!("ChatwStudio.setDoc('loop', {})", e.checked())) } "Lặp lại" }
                    span { class: "member-sub", "Khung {s.frame + 1}/{s.frames}" }
                    div { class: "grow" }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| run("ChatwStudio.addFrame()"), "+ Khung" }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| run("ChatwStudio.duplicateFrame()"), "Nhân bản khung" }
                    button { r#type: "button", class: "btn-small-secondary", aria_label: "Dời khung sang trái", onclick: move |_| js("ChatwStudio.moveFrame(-1)"), "←" }
                    button { r#type: "button", class: "btn-small-secondary", aria_label: "Dời khung sang phải", onclick: move |_| js("ChatwStudio.moveFrame(1)"), "→" }
                    button { r#type: "button", class: "btn-small-secondary danger", onclick: move |_| run("ChatwStudio.deleteFrame()"), "Xóa khung" }
                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| menu.set(if menu() == Some("tween") { None } else { Some("tween") }), "Chuyển động" }
                }
                if menu() == Some("tween") {
                    div { class: "studio-tween",
                        div { class: "segmented",
                            button { r#type: "button", class: if !tween_custom() { "active" } else { "" }, onclick: move |_| tween_custom.set(false), "Motion Presets" }
                            button { r#type: "button", class: if tween_custom() { "active" } else { "" }, onclick: move |_| tween_custom.set(true), "Keyframes" }
                        }
                        if !tween_custom() {
                            div { class: "studio-presets",
                                for (id, label) in PRESETS {
                                    button { key: "{id}", r#type: "button", class: if tween_preset() == id { "studio-chip active" } else { "studio-chip" }, onclick: move |_| tween_preset.set(id.to_string()), "{label}" }
                                }
                            }
                        } else {
                            div { class: "studio-keys",
                                span {} span { class: "member-sub", "X" } span { class: "member-sub", "Y" } span { class: "member-sub", "Scale" } span { class: "member-sub", "Rotate°" } span { class: "member-sub", "Opacity" }
                                for (row, label) in [(0usize, "Đầu"), (5usize, "Cuối")] {
                                    span { key: "l{row}", class: "member-sub", "{label}" }
                                    for k in 0..5usize {
                                        input { key: "{row}-{k}", r#type: "number", step: if k == 2 || k == 4 { "0.1" } else { "1" }, value: "{tw()[row + k]}", aria_label: "Giá trị {label} {k}",
                                            oninput: move |e| { let v = e.value().parse::<f64>().unwrap_or(0.0); tw.with_mut(|a| a[row + k] = v); } }
                                    }
                                }
                            }
                        }
                        div { class: "studio-inline-row",
                            label { class: "studio-inline", "Số khung"
                                input { r#type: "number", min: "2", max: "60", value: "{tween_frames}", aria_label: "Số khung cho chuyển động", oninput: move |e| tween_frames.set(e.value().parse::<u32>().unwrap_or(8).clamp(2, 60)) } }
                            label { class: "studio-inline", "Đường cong"
                                select { aria_label: "Đường cong chuyển động", value: "{tween_easing}", onchange: move |e| tween_easing.set(e.value()),
                                    for (id, l) in [("linear", "Linear"), ("ease-in", "Ease In"), ("ease-out", "Ease Out"), ("ease-in-out", "Ease In-Out"), ("bounce", "Bounce"), ("elastic", "Elastic")] {
                                        option { key: "{id}", value: "{id}", selected: tween_easing() == id, "{l}" }
                                    }
                                }
                            }
                            span { class: "member-sub grow", "Lấy hình trên lớp đang chọn ở khung hiện tại, tạo các khung tiếp theo; các lớp khác được giữ nguyên." }
                            button { r#type: "button", class: "btn-primary", onclick: apply_tween, "Tạo chuyển động" }
                        }
                    }
                }
                div { class: "studio-frames", role: "listbox", aria_label: "Khung hình",
                    for i in 0..s.frames {
                        button { key: "{i}", r#type: "button", role: "option", aria_selected: "{i == s.frame}",
                            class: if i == s.frame { "studio-frame active" } else { "studio-frame" },
                            onclick: move |_| js(&format!("ChatwStudio.selectFrame({i})")),
                            canvas { width: "{thumb_w(s.w, s.h)}", height: "{thumb_h(s.w, s.h)}", "data-studio-thumb": "{i}" }
                            span { "{i + 1}" }
                        }
                    }
                }
                div { class: "studio-status member-sub",
                    "Lịch sử: {s.undo} bước"
                    if s.draft == "saved" { " · Bản nháp đã lưu trên thiết bị" }
                    if s.draft == "too_large" { " · Bản nháp quá lớn để lưu trên thiết bị, hãy Lưu dự án" }
                    " · Phím tắt: B P E G L R O S T I V, [ ] cỡ nét, , . đổi khung, Space phát"
                }
            }
        }
    }
}

fn thumb_w(w: u32, h: u32) -> u32 {
    ((w as f64 / h as f64) * 48.0).round().clamp(24.0, 144.0) as u32
}

fn thumb_h(_w: u32, _h: u32) -> u32 {
    48
}

/// Landing page in Settings: start a drawing for a target or reopen a project.
#[component]
pub fn StudioHome(my_id: String) -> Element {
    let mut projects = use_signal(Vec::<drawings::ProjectItem>::new);
    let mut loading = use_signal(|| true);
    let mut open = use_signal(|| Option::<(Option<String>, Option<String>)>::None);
    let mut status = use_signal(|| Option::<String>::None);
    let reload = move || {
        spawn(async move {
            match drawings::list().await {
                Ok(l) => projects.set(l),
                Err(e) => status.set(Some(err_text(&e.code))),
            }
            loading.set(false);
        });
    };
    use_hook(move || reload());
    rsx! {
        div { class: "studio-home",
            div { class: "settings-group-title", "Mới" }
            div { class: "studio-targets",
                for (id, label, w, h) in TARGETS {
                    button { key: "{id}", r#type: "button", class: "studio-target", onclick: move |_| open.set(Some((Some(id.to_string()), None))),
                        span { class: if w > h { "studio-target-shape wide" } else { "studio-target-shape tall" }, style: "aspect-ratio: {w} / {h};" }
                        span { class: "settings-label", "{label}" }
                        span { class: "member-sub", "{w}×{h}" }
                    }
                }
            }
            div { class: "settings-group-title", "Dự án" }
            if let Some(s) = status() { div { class: "status-badge error", "{s}" } }
            if loading() {
                div { class: "sidebar-empty", "Đang tải dự án" }
            } else if projects.read().is_empty() {
                div { class: "sidebar-empty", "Chưa có dự án" }
            } else {
                div { class: "studio-projects",
                    for p in projects() {
                        {
                            let (id_open, id_del) = (p.id.clone(), p.id.clone());
                            rsx! {
                                div { key: "{p.id}", class: "studio-project",
                                    button { r#type: "button", class: "studio-project-open", onclick: move |_| open.set(Some((None, Some(id_open.clone())))),
                                        if p.has_thumb { img { src: "{drawings::thumb_url(&p.id)}", alt: "", loading: "lazy" } } else { span { class: "studio-target-shape" } }
                                        span { class: "settings-label", "{p.name}" }
                                        span { class: "member-sub", "{p.width}×{p.height} · {p.layers} lớp · {p.frames} khung · {p.size / 1024} KB" }
                                    }
                                    button { r#type: "button", class: "btn-small-secondary danger", aria_label: "Xóa dự án {p.name}",
                                        onclick: move |_| {
                                            let id = id_del.clone();
                                            spawn(async move {
                                                if drawings::delete(&id).await.is_ok() {
                                                    projects.with_mut(|l| l.retain(|x| x.id != id));
                                                }
                                            });
                                        },
                                        "Xóa"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some((target, project)) = open() {
                Studio { my_id: my_id.clone(), target, open_project: project, on_close: move |_| { open.set(None); reload(); } }
            }
        }
    }
}
