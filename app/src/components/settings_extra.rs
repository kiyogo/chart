//! Extra settings pages: navigation & chat customization, and the sticker manager.

use dioxus::prelude::*;

use crate::{
    components::{
        guide::Illustration,
        icons::{ChevronRightIcon, CloseIcon, DevicesIcon, LockIcon, PlusIcon},
        media_picker::BUILTIN_GIFS,
    },
    models::{LibraryItem, SessionItem, StickerPackItem},
    platform,
    prefs::PrefsCtx,
    services::{library, stickers},
};

#[component]
fn Toggle(label: String, sub: String, checked: bool, on_change: EventHandler<bool>) -> Element {
    rsx! {
        label { class: "settings-row toggle-row",
            div { class: "settings-row-text",
                span { class: "settings-label", "{label}" }
                span { class: "settings-sublabel", "{sub}" }
            }
            input {
                r#type: "checkbox",
                role: "switch",
                class: "switch",
                checked: checked,
                aria_label: "{label}",
                onchange: move |e| on_change.call(e.checked()),
            }
        }
    }
}

pub const WALLPAPERS: [(&str, &str); 5] = [
    ("none", "Không"),
    ("dots", "Chấm bi"),
    ("bubbles", "Bong bóng"),
    ("aurora", "Cực quang"),
    ("paper", "Giấy ấm"),
];

#[component]
pub fn CustomizationSettings() -> Element {
    let ctx = use_context::<PrefsCtx>();
    let p = ctx.prefs.read().clone();
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Cột điều hướng bên trái" }
            div { class: "settings-group-desc", "Cột trái chỉ hiện biểu tượng: Tin nhắn, Bạn bè, Cộng đồng, Tìm nhanh. Rê chuột để xem tên. Bạn chọn thêm lối tắt được ghim bên dưới." }
            div { class: "settings-list",
                Toggle {
                    label: "Ghim cộng đồng".to_string(),
                    sub: "Hiện biểu tượng các cộng đồng bạn tham gia ngay trên cột trái.".to_string(),
                    checked: p.pin_communities,
                    on_change: move |v| ctx.update(|p| p.pin_communities = v),
                }
                Toggle {
                    label: "Ghim nhóm trò chuyện".to_string(),
                    sub: "Hiện ảnh đại diện các nhóm để mở nhanh.".to_string(),
                    checked: p.pin_groups,
                    on_change: move |v| ctx.update(|p| p.pin_groups = v),
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Hình nền trò chuyện" }
            div { class: "wallpaper-grid", role: "radiogroup", aria_label: "Hình nền trò chuyện",
                for (val, label) in WALLPAPERS {
                    button {
                        key: "{val}",
                        r#type: "button",
                        role: "radio",
                        aria_checked: "{p.wallpaper == val}",
                        class: if p.wallpaper == val { "wallpaper-tile active" } else { "wallpaper-tile" },
                        onclick: move |_| ctx.update(|p| p.wallpaper = val.to_string()),
                        span { class: "wallpaper-sample wp-{val}" }
                        span { "{label}" }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Thông báo và hiệu ứng" }
            div { class: "settings-list",
                Toggle {
                    label: "Âm báo tin nhắn mới".to_string(),
                    sub: "Phát âm ngắn khi có tin nhắn hoặc lời mời mới.".to_string(),
                    checked: p.sound,
                    on_change: move |v| {
                        ctx.update(|p| p.sound = v);
                        if v { platform::play_notification_chime(); }
                    },
                }
                Toggle {
                    label: "Hiệu ứng hồ sơ động".to_string(),
                    sub: "Hiển thị hiệu ứng như tuyết rơi, lấp lánh trên hồ sơ người khác.".to_string(),
                    checked: p.profile_effects,
                    on_change: move |v| ctx.update(|p| p.profile_effects = v),
                }
            }
        }
    }
}

#[component]
pub fn StickerSettings(current_user_id: String) -> Element {
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut loading = use_signal(|| true);
    let mut new_pack = use_signal(String::new);
    let mut sticker_name = use_signal(String::new);
    let mut target_pack = use_signal(|| Option::<String>::None);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);

    let reload = move || {
        spawn(async move {
            match stickers::stickers_list_packs().await {
                Ok(list) => {
                    packs.set(list);
                    notice.with_mut(|n| if matches!(n, Some((true, _))) { *n = None });
                }
                Err(_) => notice.set(Some((true, "Không tải được danh sách sticker".into()))),
            }
            loading.set(false);
        });
    };
    use_effect(move || reload());

    let me = current_user_id.clone();
    let mine: Vec<StickerPackItem> = packs().into_iter().filter(|p| p.author_id.as_deref() == Some(me.as_str())).collect();
    let builtin: Vec<StickerPackItem> = packs().into_iter().filter(|p| p.author_id.is_none()).collect();
    if target_pack().is_none() {
        if let Some(first) = mine.first() {
            target_pack.set(Some(first.id.clone()));
        }
    }

    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Bộ sticker của bạn" }
            div { class: "settings-group-desc", "Ảnh PNG, WebP, AVIF hoặc GIF động, tối đa 512 KB. Ảnh tĩnh được tự thu về 320×320." }
            form {
                class: "inline-form",
                onsubmit: move |e| {
                    e.prevent_default();
                    let name = new_pack().trim().to_string();
                    if name.is_empty() || busy() { return; }
                    busy.set(true);
                    spawn(async move {
                        match stickers::stickers_create_pack(&name, None, None).await {
                            Ok(p) => {
                                new_pack.set(String::new());
                                target_pack.set(Some(p.id.clone()));
                                notice.set(Some((false, format!("Đã tạo bộ \"{}\"", p.name))));
                                reload();
                            }
                            Err(_) => notice.set(Some((true, "Không tạo được bộ sticker".into()))),
                        }
                        busy.set(false);
                    });
                },
                input { maxlength: "64", placeholder: "Tên bộ sticker mới", aria_label: "Tên bộ sticker mới", value: "{new_pack}", oninput: move |e| new_pack.set(e.value()) }
                button { r#type: "submit", class: "btn-primary", disabled: new_pack().trim().is_empty() || busy(), PlusIcon { size: 16 } span { "Tạo bộ" } }
            }

            if !mine.is_empty() {
                div { class: "sticker-upload",
                    select {
                        aria_label: "Chọn bộ để thêm sticker",
                        value: "{target_pack().unwrap_or_default()}",
                        onchange: move |e| target_pack.set(Some(e.value())),
                        for p in mine.iter() { option { key: "{p.id}", value: "{p.id}", "{p.name}" } }
                    }
                    input { maxlength: "64", placeholder: "Tên sticker", aria_label: "Tên sticker", value: "{sticker_name}", oninput: move |e| sticker_name.set(e.value()) }
                    label { class: if busy() { "btn-avatar-upload disabled" } else { "btn-avatar-upload" },
                        input {
                            id: "sticker-file-input",
                            r#type: "file",
                            accept: "image/png,image/webp,image/gif,image/avif",
                            style: "display: none;",
                            disabled: busy(),
                            onchange: move |_| {
                                let Some(pid) = target_pack() else { return };
                                let name = { let n = sticker_name().trim().to_string(); if n.is_empty() { "Sticker".to_string() } else { n } };
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    let result: Result<(), String> = async {
                                        let (bytes, mime) = platform::process_selected_sticker("sticker-file-input").await.map_err(|e| match e.as_str() {
                                            "no_file" => e,
                                            _ => "Không đọc được tệp ảnh này. Hãy chọn PNG, WebP, AVIF hoặc GIF.".to_string(),
                                        })?;
                                        if bytes.len() > 512 * 1024 { return Err("Tệp vượt quá 512 KB".into()); }
                                        let (url, animated) = stickers::stickers_upload_image(bytes, &mime).await.map_err(|e| format!("Tải lên thất bại ({})", e.code))?;
                                        stickers::stickers_add_sticker(&pid, &name, &url, animated).await.map_err(|e| format!("Không thêm được sticker ({})", e.code))?;
                                        Ok(())
                                    }.await;
                                    match result {
                                        Ok(()) => { sticker_name.set(String::new()); notice.set(Some((false, "Đã thêm sticker".into()))); reload(); }
                                        Err(e) if e == "no_file" => {}
                                        Err(e) => notice.set(Some((true, e))),
                                    }
                                    busy.set(false);
                                });
                            },
                        }
                        if busy() { span { class: "spinner" } } else { "Chọn ảnh và tải lên" }
                    }
                }
            }
            if let Some((is_err, msg)) = notice() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
            }

            if loading() {
                div { class: "empty-chat-inner", "Đang tải sticker" }
            } else if mine.is_empty() {
                div { class: "sidebar-empty", "Bạn chưa có bộ sticker riêng. Tạo bộ đầu tiên ở trên." }
            }
            for pack in mine.clone() {
                div { key: "{pack.id}", class: "sticker-pack-card",
                    div { class: "sticker-pack-head",
                        span { class: "settings-label", "{pack.name}" }
                        span { class: "settings-sublabel", "{pack.stickers.len()} sticker" }
                        button {
                            r#type: "button",
                            class: "btn-danger-outline",
                            onclick: { let pid = pack.id.clone(); move |_| {
                                let pid = pid.clone();
                                spawn(async move {
                                    if stickers::stickers_delete_pack(&pid).await.is_ok() {
                                        if target_pack().as_deref() == Some(pid.as_str()) { target_pack.set(None); }
                                        reload();
                                    } else {
                                        notice.set(Some((true, "Không xóa được bộ sticker".into())));
                                    }
                                });
                            }},
                            "Xóa bộ"
                        }
                    }
                    div { class: "sticker-grid",
                        for st in pack.stickers.clone() {
                            div { key: "{st.id}", class: "sticker-cell manage", title: "{st.name}",
                                img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{st.name}", loading: "lazy" }
                                button {
                                    r#type: "button",
                                    class: "sticker-remove",
                                    aria_label: "Xóa sticker {st.name}",
                                    onclick: { let pid = pack.id.clone(); let sid = st.id.clone(); move |_| {
                                        let (pid, sid) = (pid.clone(), sid.clone());
                                        spawn(async move {
                                            if stickers::stickers_delete_sticker(&pid, &sid).await.is_ok() { reload(); }
                                            else { notice.set(Some((true, "Không xóa được sticker".into()))); }
                                        });
                                    }},
                                    CloseIcon { size: 14 }
                                }
                            }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Bộ có sẵn" }
            for pack in builtin {
                div { key: "{pack.id}", class: "sticker-pack-card",
                    div { class: "sticker-pack-head",
                        span { class: "settings-label", "{pack.name}" }
                        span { class: "settings-sublabel", "{pack.stickers.len()} sticker" }
                    }
                    div { class: "sticker-grid",
                        for st in pack.stickers.clone() {
                            div { key: "{st.id}", class: "sticker-cell", title: "{st.name}",
                                img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{st.name}", loading: "lazy" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Account: username, email/phone policy, signed-in devices
// ---------------------------------------------------------------------------

fn relative_time(iso: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else { return String::new() };
    let secs = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds().max(0);
    match secs {
        0..=59 => "vừa xong".into(),
        60..=3599 => format!("{} phút trước", secs / 60),
        3600..=86_399 => format!("{} giờ trước", secs / 3600),
        _ => format!("{} ngày trước", secs / 86_400),
    }
}

fn device_label(ua: Option<&str>) -> String {
    let ua = ua.unwrap_or("");
    let browser = if ua.contains("Edg/") { "Edge" } else if ua.contains("Firefox/") { "Firefox" } else if ua.contains("Chrome/") { "Chrome" } else if ua.contains("Safari/") { "Safari" } else if ua.is_empty() { "Thiết bị" } else { "Ứng dụng" };
    let os = if ua.contains("Windows") { "Windows" } else if ua.contains("Android") { "Android" } else if ua.contains("iPhone") || ua.contains("iPad") { "iOS" } else if ua.contains("Mac OS") { "macOS" } else if ua.contains("Linux") { "Linux" } else { "" };
    if os.is_empty() { browser.to_string() } else { format!("{browser} trên {os}") }
}

#[derive(Clone, Copy, PartialEq)]
enum UserStep { Edit, Confirm, Done }

#[component]
fn UsernameDialog(current: String, on_close: EventHandler<()>, on_changed: EventHandler<String>) -> Element {
    let mut step = use_signal(|| UserStep::Edit);
    let mut value = use_signal(|| current.clone());
    let mut error = use_signal(|| Option::<String>::None);
    let mut busy = use_signal(|| false);
    let v = value().trim().to_string();
    let valid = (2..=32).contains(&v.chars().count()) && v != current;
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog story-dialog", role: "dialog", aria_modal: "true", aria_label: "Đổi tên đăng nhập", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: "user".to_string() }
                }
                match step() {
                    UserStep::Edit => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Đổi tên đăng nhập" }
                            p { class: "story-text", "Bạn bè tìm và kết bạn với bạn bằng tên này. 2–32 ký tự, không trùng với người khác." }
                            label { class: "form-field", style: "text-align: left;",
                                span { class: "form-label", "Tên đăng nhập mới" }
                                input { maxlength: "32", value: "{value}", autocomplete: "off", oninput: move |e| { value.set(e.value()); error.set(None); } }
                            }
                            if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                        }
                        div { class: "story-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy bỏ" }
                            button { r#type: "button", class: "btn-primary", disabled: !valid, onclick: move |_| step.set(UserStep::Confirm), "Tiếp tục" }
                        }
                    },
                    UserStep::Confirm => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Xác nhận đổi tên" }
                            p { class: "story-text", "Tên đăng nhập sẽ đổi từ " strong { "{current}" } " thành " strong { "{v}" } ". Bạn bè hiện tại vẫn giữ kết nối với bạn." }
                            if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                        }
                        div { class: "story-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| step.set(UserStep::Edit), "Quay lại" }
                            button { r#type: "button", class: "btn-primary", disabled: busy(),
                                onclick: move |_| {
                                    let name = value().trim().to_string();
                                    busy.set(true);
                                    spawn(async move {
                                        match library::change_username(&name).await {
                                            Ok(_) => { step.set(UserStep::Done); on_changed.call(name); }
                                            Err(e) => {
                                                error.set(Some(match e.code.as_str() {
                                                    "username_taken" => "Tên này đã có người dùng".to_string(),
                                                    "invalid_username" | "username_invalid" => "Tên không hợp lệ".to_string(),
                                                    other => format!("Không đổi được tên ({other})"),
                                                }));
                                                step.set(UserStep::Edit);
                                            }
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { span { class: "spinner" } } else { "Đổi tên" }
                            }
                        }
                    },
                    UserStep::Done => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Đã đổi tên" }
                            p { class: "story-text", "Tên đăng nhập mới của bạn là " strong { "{v}" } "." }
                        }
                        div { class: "story-actions single",
                            button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Xong" }
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn DevicesDialog(on_close: EventHandler<()>, on_count: EventHandler<usize>) -> Element {
    let mut sessions = use_signal(Vec::<SessionItem>::new);
    let mut loading = use_signal(|| true);
    let mut confirm_all = use_signal(|| false);
    let mut need_code = use_signal(|| false);
    let mut twofa_code = use_signal(String::new);
    let mut confirm_one = use_signal(|| Option::<String>::None);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut renaming = use_signal(|| Option::<(String, String)>::None);
    let reload = move || {
        spawn(async move {
            if let Ok(list) = library::sessions_list().await {
                on_count.call(list.len());
                sessions.set(list);
            }
            loading.set(false);
        });
    };
    use_effect(move || reload());
    let others = sessions.read().iter().filter(|s| !s.is_current).count();
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog story-dialog wide", role: "dialog", aria_modal: "true", aria_label: "Thiết bị đã đăng nhập", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: "devices".to_string() }
                }
                div { class: "story-body",
                    h2 { class: "story-title", "Thiết bị đã đăng nhập" }
                    p { class: "story-text", "Đăng xuất những thiết bị bạn không nhận ra. Thiết bị bị đăng xuất cần đăng nhập lại." }
                }
                div { class: "device-list",
                    if loading() { div { class: "empty-chat-inner", "Đang tải" } }
                    for s in sessions() {
                        div { key: "{s.id}", class: "device-row",
                            span { class: "device-icon", DevicesIcon { size: 20 } }
                            div { class: "settings-row-text",
                                if renaming().as_ref().is_some_and(|(id, _)| id == &s.id) {
                                    form { class: "inline-form",
                                        onsubmit: { let id = s.id.clone(); move |e: Event<FormData>| {
                                            e.prevent_default();
                                            let id = id.clone();
                                            let name = renaming().map(|(_, n)| n.trim().to_string()).unwrap_or_default();
                                            if name.is_empty() { return; }
                                            spawn(async move {
                                                match crate::services::social::session_rename(&id, &name).await {
                                                    Ok(_) => notice.set(Some((false, "Đã đổi tên thiết bị".into()))),
                                                    Err(_) => notice.set(Some((true, "Không đổi được tên thiết bị".into()))),
                                                }
                                                renaming.set(None);
                                                reload();
                                            });
                                        } },
                                        input { r#type: "text", maxlength: "40", aria_label: "Tên thiết bị", value: "{renaming().map(|(_, n)| n).unwrap_or_default()}",
                                            oninput: { let id = s.id.clone(); move |e: Event<FormData>| renaming.set(Some((id.clone(), e.value()))) } }
                                        button { r#type: "submit", class: "btn-primary", "Lưu" }
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| renaming.set(None), "Hủy" }
                                    }
                                } else {
                                    span { class: "settings-label",
                                        "{s.device_label.clone().unwrap_or_else(|| device_label(s.device_name.as_deref()))}"
                                        if s.is_current { span { class: "chip-current", "Thiết bị này" } }
                                        button { r#type: "button", class: "btn-ghost small", title: "Đổi tên thiết bị",
                                            onclick: { let id = s.id.clone(); let cur = s.device_label.clone().unwrap_or_default(); move |_| renaming.set(Some((id.clone(), cur.clone()))) },
                                            "Đổi tên"
                                        }
                                    }
                                }
                                span { class: "settings-sublabel",
                                    "{s.ip_address.clone().unwrap_or_else(|| \"IP không rõ\".into())} · hoạt động {s.last_active.as_deref().map(relative_time).unwrap_or_default()}"
                                }
                            }
                            if !s.is_current {
                                if confirm_one().as_deref() == Some(s.id.as_str()) {
                                    div { class: "device-confirm",
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| confirm_one.set(None), "Hủy" }
                                        button { r#type: "button", class: "btn-danger",
                                            onclick: { let id = s.id.clone(); move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match library::sessions_revoke(&id).await {
                                                        Ok(_) => notice.set(Some((false, "Đã đăng xuất thiết bị".into()))),
                                                        Err(_) => notice.set(Some((true, "Không đăng xuất được thiết bị".into()))),
                                                    }
                                                    confirm_one.set(None);
                                                    reload();
                                                });
                                            }},
                                            "Đăng xuất"
                                        }
                                    }
                                } else {
                                    button { r#type: "button", class: "btn-secondary", onclick: { let id = s.id.clone(); move |_| confirm_one.set(Some(id.clone())) }, "Đăng xuất" }
                                }
                            }
                        }
                    }
                }
                if let Some((is_err, msg)) = notice() {
                    div { style: "padding: 0 var(--space-5);", div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" } }
                }
                div { class: "story-actions",
                    if confirm_all() {
                        if need_code() {
                            input { r#type: "text", maxlength: "12", aria_label: "Mã xác minh hai bước", placeholder: "Mã 6 số hoặc mã dự phòng", value: "{twofa_code}", oninput: move |e| twofa_code.set(e.value()) }
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| { confirm_all.set(false); need_code.set(false); }, "Hủy" }
                        button { r#type: "button", class: "btn-danger",
                            onclick: move |_| {
                                spawn(async move {
                                    // Spec 011: the server asks for the second step when it is on.
                                    let c = twofa_code().trim().to_string();
                                    match crate::services::security::revoke_other_sessions(if c.is_empty() { None } else { Some(&c) }).await {
                                        Ok(_) => notice.set(Some((false, "Đã đăng xuất mọi thiết bị khác".into()))),
                                        Err(e) if e.code == "second_factor_required" => {
                                            need_code.set(true);
                                            notice.set(Some((true, "Nhập mã xác minh hai bước để tiếp tục".into())));
                                            return;
                                        }
                                        Err(e) => notice.set(Some((true, crate::services::security::error_text(&e.code)))),
                                    }
                                    twofa_code.set(String::new());
                                    need_code.set(false);
                                    confirm_all.set(false);
                                    reload();
                                });
                            },
                            "Xác nhận đăng xuất {others} thiết bị"
                        }
                    } else {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Đóng" }
                        button { r#type: "button", class: "btn-danger-outline", disabled: others == 0, onclick: move |_| confirm_all.set(true), "Đăng xuất tất cả thiết bị khác" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AccountSecurity(current_username: String, on_username_changed: EventHandler<String>) -> Element {
    let mut username = use_signal(|| current_username.clone());
    let mut dialog = use_signal(|| Option::<&'static str>::None);
    let mut device_count = use_signal(|| Option::<usize>::None);
    use_effect(move || {
        spawn(async move {
            if let Ok(list) = library::sessions_list().await {
                device_count.set(Some(list.len()));
            }
        });
    });
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Thông tin tài khoản" }
            div { class: "settings-list",
                div { class: "settings-row",
                    div { class: "settings-row-text", span { class: "settings-label", "Tên đăng nhập" } span { class: "settings-sublabel", "{username}" } }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| dialog.set(Some("username")), "Chỉnh sửa" }
                }
                div { class: "settings-row",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Email" }
                        span { class: "settings-sublabel", "ChatW không lưu email. Bạn đăng nhập bằng Google; máy chủ chỉ giữ mã định danh ẩn danh." }
                    }
                    span { class: "chip-privacy", LockIcon { size: 14 } "Không lưu" }
                }
                div { class: "settings-row",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Số điện thoại" }
                        span { class: "settings-sublabel", "Không cần số điện thoại để dùng ChatW." }
                    }
                    span { class: "chip-privacy", LockIcon { size: 14 } "Không yêu cầu" }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Mật khẩu và bảo mật" }
            div { class: "settings-list",
                div { class: "settings-row",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Phương thức đăng nhập" }
                        span { class: "settings-sublabel", "Google OAuth (PKCE). Bật xác minh 2 bước trên tài khoản Google để bảo vệ thêm." }
                    }
                }
                button { r#type: "button", class: "settings-row row-button", onclick: move |_| dialog.set(Some("devices")),
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Thiết bị đã đăng nhập" }
                        span { class: "settings-sublabel", "Xem và đăng xuất từ xa" }
                    }
                    span { class: "row-trail",
                        if let Some(n) = device_count() { "{n} thiết bị" }
                        ChevronRightIcon { size: 16 }
                    }
                }
            }
        }
        if dialog() == Some("username") {
            UsernameDialog {
                current: username(),
                on_close: move |_| dialog.set(None),
                on_changed: move |n: String| { username.set(n.clone()); on_username_changed.call(n); },
            }
        }
        if dialog() == Some("devices") {
            DevicesDialog { on_close: move |_| dialog.set(None), on_count: move |n| device_count.set(Some(n)) }
        }
    }
}

// ---------------------------------------------------------------------------
// Library: custom emoji and GIFs
// ---------------------------------------------------------------------------

#[component]
pub fn EmojiSettings() -> Element {
    let mut items = use_signal(Vec::<LibraryItem>::new);
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let reload = move || { spawn(async move { if let Ok(l) = library::emojis_list().await { items.set(l); } }); };
    use_effect(move || reload());
    let n = name().trim().to_string();
    let valid = (2..=32).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Custom emoji" }
            div { class: "settings-group-desc", "Tên tiếng Anh gồm chữ, số, gạch dưới (ví dụ party_cat). Ảnh vuông PNG/GIF/WebP ≤ 2 MB, hiển thị như :party_cat: trong tin nhắn." }
            div { class: "sticker-upload",
                input { maxlength: "32", placeholder: "emoji_name", aria_label: "Tên emoji", value: "{name}", oninput: move |e| name.set(e.value()) }
                label { class: if busy() || !valid { "btn-avatar-upload disabled" } else { "btn-avatar-upload" },
                    input {
                        id: "emoji-file-input", r#type: "file", accept: "image/png,image/gif,image/webp,image/avif", style: "display: none;",
                        disabled: busy() || !valid,
                        onchange: move |_| {
                            let nm = name().trim().to_string();
                            busy.set(true);
                            notice.set(None);
                            spawn(async move {
                                let res: Result<(), String> = async {
                                    let (bytes, mime) = platform::read_selected_file_raw("emoji-file-input").await?;
                                    let url = library::upload_media(bytes, &mime).await.map_err(|e| format!("Upload failed ({})", e.code))?;
                                    library::emojis_create(&nm, &url).await.map_err(|e| if e.code == "emoji_name_taken" { "Tên emoji đã tồn tại".to_string() } else { format!("Không thêm được emoji ({})", e.code) })?;
                                    Ok(())
                                }.await;
                                match res {
                                    Ok(()) => { name.set(String::new()); notice.set(Some((false, format!(":{nm}: đã sẵn sàng")))); reload(); }
                                    Err(e) if e == "no_file" => {}
                                    Err(e) if e == "too_large" => notice.set(Some((true, "Tệp lớn hơn 2 MB".into()))),
                                    Err(e) => notice.set(Some((true, e))),
                                }
                                busy.set(false);
                            });
                        },
                    }
                    if busy() { span { class: "spinner" } } else { "Chọn ảnh và thêm" }
                }
            }
            if let Some((is_err, msg)) = notice() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" } }
            if items.read().is_empty() {
                div { class: "sidebar-empty", "Chưa có emoji riêng." }
            } else {
                div { class: "library-grid",
                    for it in items() {
                        div { key: "{it.id}", class: "library-cell", title: ":{it.name}:",
                            img { src: "{platform::resolve_asset_url(&it.file_url)}", alt: ":{it.name}:", loading: "lazy" }
                            span { class: "library-name", ":{it.name}:" }
                            button { r#type: "button", class: "sticker-remove", aria_label: "Delete :{it.name}:",
                                onclick: { let id = it.id.clone(); move |_| { let id = id.clone(); spawn(async move { let _ = library::emojis_delete(&id).await; reload(); }); } },
                                CloseIcon { size: 14 }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GifSettings() -> Element {
    let mut items = use_signal(Vec::<LibraryItem>::new);
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let reload = move || { spawn(async move { if let Ok(l) = library::gifs_list().await { items.set(l); } }); };
    use_effect(move || reload());
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "My GIFs" }
            div { class: "settings-group-desc", "Tải GIF động (≤ 2 MB) hoặc lưu ảnh động có sẵn của ChatW vào bộ sưu tập để gửi nhanh." }
            div { class: "sticker-upload",
                input { maxlength: "64", placeholder: "GIF name", aria_label: "Tên GIF", value: "{name}", oninput: move |e| name.set(e.value()) }
                label { class: if busy() { "btn-avatar-upload disabled" } else { "btn-avatar-upload" },
                    input {
                        id: "gif-file-input", r#type: "file", accept: "image/gif,image/webp,image/png", style: "display: none;",
                        disabled: busy(),
                        onchange: move |_| {
                            let nm = { let n = name().trim().to_string(); if n.is_empty() { "GIF".to_string() } else { n } };
                            busy.set(true);
                            notice.set(None);
                            spawn(async move {
                                let res: Result<(), String> = async {
                                    let (bytes, mime) = platform::read_selected_file_raw("gif-file-input").await?;
                                    let url = library::upload_media(bytes, &mime).await.map_err(|e| format!("Upload failed ({})", e.code))?;
                                    library::gifs_create(&nm, &url).await.map_err(|e| format!("Không lưu được GIF ({})", e.code))?;
                                    Ok(())
                                }.await;
                                match res {
                                    Ok(()) => { name.set(String::new()); notice.set(Some((false, "Đã thêm GIF".into()))); reload(); }
                                    Err(e) if e == "no_file" => {}
                                    Err(e) if e == "too_large" => notice.set(Some((true, "Tệp lớn hơn 2 MB".into()))),
                                    Err(e) => notice.set(Some((true, e))),
                                }
                                busy.set(false);
                            });
                        },
                    }
                    if busy() { span { class: "spinner" } } else { "Chọn GIF và tải lên" }
                }
            }
            if let Some((is_err, msg)) = notice() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" } }
            div { class: "library-grid gifs",
                for it in items() {
                    div { key: "{it.id}", class: "library-cell", title: "{it.name}",
                        img { src: "{platform::resolve_asset_url(&it.file_url)}", alt: "{it.name}", loading: "lazy" }
                        span { class: "library-name", "{it.name}" }
                        button { r#type: "button", class: "sticker-remove", aria_label: "Delete {it.name}",
                            onclick: { let id = it.id.clone(); move |_| { let id = id.clone(); spawn(async move { let _ = library::gifs_delete(&id).await; reload(); }); } },
                            CloseIcon { size: 14 }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "ChatW Motion" }
            div { class: "library-grid gifs",
                for (nm, url) in BUILTIN_GIFS {
                    {
                        let saved = items.read().iter().any(|g| g.file_url == url);
                        rsx! {
                            div { key: "{url}", class: "library-cell", title: "{nm}",
                                img { src: "{url}", alt: "{nm}", loading: "lazy" }
                                span { class: "library-name", "{nm}" }
                                if !saved {
                                    button { r#type: "button", class: "save-chip", aria_label: "Save {nm}",
                                        onclick: move |_| { spawn(async move { let _ = library::gifs_create(nm, url).await; reload(); }); },
                                        "Save"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LibTab { Sticker, Emoji, Gif }

#[component]
pub fn LibrarySettings(current_user_id: String) -> Element {
    let mut tab = use_signal(|| LibTab::Sticker);
    rsx! {
        div { class: "guide-hero compact",
            Illustration { kind: "library".to_string() }
            div {
                div { class: "settings-group-title", "Thư viện của bạn" }
                div { class: "settings-group-desc", "Sticker, emoji và GIF bạn thêm ở đây dùng được trong mọi cuộc trò chuyện. Tên luôn là tiếng Anh." }
                div { class: "segmented", role: "tablist",
                    button { r#type: "button", role: "tab", class: if tab() == LibTab::Sticker { "active" } else { "" }, onclick: move |_| tab.set(LibTab::Sticker), "Sticker" }
                    button { r#type: "button", role: "tab", class: if tab() == LibTab::Emoji { "active" } else { "" }, onclick: move |_| tab.set(LibTab::Emoji), "Emoji" }
                    button { r#type: "button", role: "tab", class: if tab() == LibTab::Gif { "active" } else { "" }, onclick: move |_| tab.set(LibTab::Gif), "GIF" }
                }
            }
        }
        match tab() {
            LibTab::Sticker => rsx! { StickerSettings { current_user_id: current_user_id.clone() } },
            LibTab::Emoji => rsx! { EmojiSettings {} },
            LibTab::Gif => rsx! { GifSettings {} },
        }
    }
}
