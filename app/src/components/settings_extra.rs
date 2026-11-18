//! Extra settings pages: navigation & chat customization, and the sticker manager.

use dioxus::prelude::*;

use crate::{
    components::icons::{CloseIcon, PlusIcon},
    models::StickerPackItem,
    platform,
    prefs::PrefsCtx,
    services::stickers,
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
