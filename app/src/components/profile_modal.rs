//! Full profile view and the profile editor (with live preview).

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        icons::{CloseIcon, MessageIcon},
        profile_card::{DECORATIONS, DecoratedAvatar, EFFECTS, ProfileCard, THEME_PRESETS},
    },
    models::UserProfileResponse,
    platform,
    services::api::{get_user_profile, update_my_profile_fields, users_upload_avatar, users_upload_banner},
};

#[derive(Clone, Copy, PartialEq)]
enum EditTab {
    Info,
    Style,
}

#[component]
pub fn ProfileModal(
    user_id: String,
    is_self: bool,
    on_close: EventHandler<()>,
    #[props(default)] on_profile_updated: Option<EventHandler<UserProfileResponse>>,
    #[props(default)] on_message: Option<EventHandler<String>>,
    #[props(default = true)] effects_enabled: bool,
    #[props(default)] start_editing: bool,
) -> Element {
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let mut load_error = use_signal(|| false);
    let mut draft = use_signal(|| Option::<UserProfileResponse>::None);
    let mut tab = use_signal(|| EditTab::Info);
    let mut saving = use_signal(|| false);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);

    let uid = user_id.clone();
    use_effect(move || {
        let u = uid.clone();
        spawn(async move {
            match get_user_profile(&u).await {
                Ok(p) => {
                    if start_editing && is_self {
                        draft.set(Some(p.clone()));
                    }
                    profile.set(Some(p));
                }
                Err(_) => load_error.set(true),
            }
        });
    });

    let mut edit = move |f: &dyn Fn(&mut UserProfileResponse)| {
        draft.with_mut(|d| {
            if let Some(p) = d {
                f(p);
            }
        });
    };

    let save = move |_| {
        let Some(d) = draft() else { return };
        saving.set(true);
        notice.set(None);
        spawn(async move {
            let fields = json!({
                "display_name": d.display_name.clone().unwrap_or_default(),
                "bio": d.bio.clone().unwrap_or_default(),
                "status_text": d.status_text.clone().unwrap_or_default(),
                "status_emoji": d.status_emoji.clone().unwrap_or_default(),
                "presence": d.presence.clone(),
                "profile_color": d.profile_color.clone().unwrap_or_default(),
                "profile_accent": d.profile_accent.clone().unwrap_or_default(),
                "avatar_decoration": d.avatar_decoration.clone().unwrap_or_else(|| "none".into()),
                "profile_effect": d.profile_effect.clone().unwrap_or_else(|| "none".into()),
            });
            match update_my_profile_fields(fields).await {
                Ok(updated) => {
                    profile.set(Some(updated.clone()));
                    draft.set(None);
                    if let Some(h) = on_profile_updated {
                        h.call(updated);
                    }
                }
                Err(e) => notice.set(Some((true, format!("Không lưu được hồ sơ ({})", e.code)))),
            }
            saving.set(false);
        });
    };

    let editing = draft().is_some();
    let dialog_cls = if editing { "modal-dialog profile-modal editing" } else { "modal-dialog profile-modal" };

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            aria_label: "Hồ sơ người dùng",
            onclick: move |_| on_close.call(()),
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            div { class: "{dialog_cls}", onclick: move |evt| evt.stop_propagation(),
                if load_error() {
                    div { class: "modal-body",
                        div { class: "status-badge error", "Không tải được hồ sơ" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Đóng" }
                    }
                } else if let Some(d) = draft() {
                    div { class: "profile-editor",
                        div { class: "profile-editor-form",
                            div { class: "modal-header",
                                h2 { class: "modal-title", "Chỉnh sửa hồ sơ" }
                                button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                            }
                            div { class: "segmented", style: "margin: 0 var(--space-5);", role: "tablist",
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Info { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Info), "Thông tin" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Style { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Style), "Trang trí" }
                            }
                            div { class: "modal-body",
                                if tab() == EditTab::Info {
                                    div { class: "form-field",
                                        span { class: "form-label", "Ảnh" }
                                        div { style: "display: flex; gap: var(--space-2); flex-wrap: wrap;",
                                            label { class: "btn-avatar-upload",
                                                input {
                                                    id: "profile-avatar-input",
                                                    r#type: "file",
                                                    accept: "image/*",
                                                    style: "display: none;",
                                                    onchange: move |_| {
                                                        notice.set(None);
                                                        spawn(async move {
                                                            match platform::process_avatar_from_input("profile-avatar-input").await {
                                                                Ok(bytes) => match users_upload_avatar(bytes).await {
                                                                    Ok(resp) => {
                                                                        let url = resp.avatar_url.clone();
                                                                        draft.with_mut(|d| if let Some(p) = d { p.avatar_url = Some(url.clone()); });
                                                                        profile.with_mut(|d| if let Some(p) = d { p.avatar_url = Some(url.clone()); });
                                                                        notice.set(Some((false, "Đã cập nhật ảnh đại diện".into())));
                                                                    }
                                                                    Err(_) => notice.set(Some((true, "Không thể tải ảnh đại diện".into()))),
                                                                },
                                                                Err(e) if e == "no_file" => {}
                                                                Err(_) => notice.set(Some((true, "Không thể xử lý ảnh".into()))),
                                                            }
                                                        });
                                                    },
                                                }
                                                "Đổi ảnh đại diện"
                                            }
                                            label { class: "btn-avatar-upload",
                                                input {
                                                    id: "profile-banner-input",
                                                    r#type: "file",
                                                    accept: "image/*",
                                                    style: "display: none;",
                                                    onchange: move |_| {
                                                        notice.set(None);
                                                        spawn(async move {
                                                            match platform::process_banner_from_input("profile-banner-input").await {
                                                                Ok(bytes) => match users_upload_banner(bytes).await {
                                                                    Ok(url) => {
                                                                        draft.with_mut(|d| if let Some(p) = d { p.custom_banner_url = Some(url.clone()); });
                                                                        profile.with_mut(|d| if let Some(p) = d { p.custom_banner_url = Some(url.clone()); });
                                                                        notice.set(Some((false, "Đã cập nhật ảnh bìa".into())));
                                                                    }
                                                                    Err(_) => notice.set(Some((true, "Không thể tải ảnh bìa".into()))),
                                                                },
                                                                Err(e) if e == "no_file" => {}
                                                                Err(_) => notice.set(Some((true, "Không thể xử lý ảnh".into()))),
                                                            }
                                                        });
                                                    },
                                                }
                                                "Đổi ảnh bìa"
                                            }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Tên hiển thị" }
                                        input {
                                            maxlength: "64",
                                            placeholder: "{d.username}",
                                            value: "{d.display_name.clone().unwrap_or_default()}",
                                            oninput: move |e| { let v = e.value(); edit(&|p| p.display_name = Some(v.clone())); },
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Trạng thái tùy chỉnh" }
                                        div { style: "display: flex; gap: var(--space-2);",
                                            input {
                                                style: "width: 4rem; text-align: center;",
                                                maxlength: "8",
                                                aria_label: "Emoji trạng thái",
                                                placeholder: "😊",
                                                value: "{d.status_emoji.clone().unwrap_or_default()}",
                                                oninput: move |e| { let v = e.value(); edit(&|p| p.status_emoji = Some(v.clone())); },
                                            }
                                            input {
                                                maxlength: "128",
                                                aria_label: "Nội dung trạng thái",
                                                placeholder: "Bạn đang làm gì?",
                                                value: "{d.status_text.clone().unwrap_or_default()}",
                                                oninput: move |e| { let v = e.value(); edit(&|p| p.status_text = Some(v.clone())); },
                                            }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Chế độ" }
                                        select {
                                            value: "{d.presence}",
                                            onchange: move |e| { let v = e.value(); edit(&|p| p.presence = v.clone()); },
                                            option { value: "online", "Sẵn sàng" }
                                            option { value: "idle", "Đang vắng" }
                                            option { value: "dnd", "Không làm phiền" }
                                            option { value: "offline", "Ẩn" }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Giới thiệu ({d.bio.clone().unwrap_or_default().chars().count()}/256)" }
                                        textarea {
                                            rows: "4",
                                            maxlength: "256",
                                            placeholder: "Vài dòng về bạn",
                                            value: "{d.bio.clone().unwrap_or_default()}",
                                            oninput: move |e| { let v = e.value(); edit(&|p| p.bio = Some(v.clone())); },
                                        }
                                    }
                                } else {
                                    div { class: "form-field",
                                        span { class: "form-label", "Màu chủ đề hồ sơ" }
                                        div { class: "theme-swatches",
                                            for (c1, c2, label) in THEME_PRESETS {
                                                button {
                                                    key: "{label}",
                                                    r#type: "button",
                                                    title: "{label}",
                                                    aria_label: "Chủ đề {label}",
                                                    class: if d.profile_color.as_deref() == Some(c1) && d.profile_accent.as_deref() == Some(c2) { "theme-swatch active" } else { "theme-swatch" },
                                                    style: "background: linear-gradient(135deg, {c1}, {c2});",
                                                    onclick: move |_| edit(&|p| { p.profile_color = Some(c1.into()); p.profile_accent = Some(c2.into()); }),
                                                }
                                            }
                                        }
                                        div { style: "display: flex; gap: var(--space-3); align-items: center; flex-wrap: wrap;",
                                            label { class: "color-field",
                                                input {
                                                    r#type: "color",
                                                    value: "{d.profile_color.clone().unwrap_or_else(|| \"#2764E7\".into())}",
                                                    oninput: move |e| { let v = e.value(); edit(&|p| p.profile_color = Some(v.clone())); },
                                                }
                                                "Màu chính"
                                            }
                                            label { class: "color-field",
                                                input {
                                                    r#type: "color",
                                                    value: "{d.profile_accent.clone().unwrap_or_else(|| \"#7C3AED\".into())}",
                                                    oninput: move |e| { let v = e.value(); edit(&|p| p.profile_accent = Some(v.clone())); },
                                                }
                                                "Màu phụ"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: move |_| edit(&|p| { p.profile_color = None; p.profile_accent = None; }),
                                                "Dùng màu mặc định"
                                            }
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Khung ảnh đại diện" }
                                        div { class: "option-tiles",
                                            for (val, label) in DECORATIONS {
                                                button {
                                                    key: "{val}",
                                                    r#type: "button",
                                                    class: if d.avatar_decoration.as_deref().unwrap_or("none") == val { "option-tile active" } else { "option-tile" },
                                                    onclick: move |_| edit(&|p| p.avatar_decoration = Some(val.into())),
                                                    DecoratedAvatar { name: d.username.clone(), avatar_url: d.avatar_url.clone(), size: 44, decoration: Some(val.to_string()) }
                                                    span { "{label}" }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Hiệu ứng hồ sơ" }
                                        div { class: "option-tiles",
                                            for (val, label) in EFFECTS {
                                                button {
                                                    key: "{val}",
                                                    r#type: "button",
                                                    class: if d.profile_effect.as_deref().unwrap_or("none") == val { "option-tile active" } else { "option-tile" },
                                                    onclick: move |_| edit(&|p| p.profile_effect = Some(val.into())),
                                                    span { class: "effect-thumb",
                                                        if val != "none" {
                                                            span { class: "profile-effect effect-{val}", for n in 0..8 { i { key: "{n}", style: "--i: {n};" } } }
                                                        }
                                                    }
                                                    span { "{label}" }
                                                }
                                            }
                                        }
                                        span { class: "form-hint", "Hiệu ứng tự tắt khi hệ điều hành bật giảm chuyển động. Người xem có thể tắt trong Cài đặt." }
                                    }
                                }
                                if let Some((is_err, msg)) = notice() {
                                    div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
                                }
                            }
                            div { class: "modal-footer",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| draft.set(None), "Hủy" }
                                button { r#type: "button", class: "btn-primary", disabled: saving(), onclick: save,
                                    if saving() { span { class: "spinner" } } else { "Lưu thay đổi" }
                                }
                            }
                        }
                        div { class: "profile-editor-preview",
                            div { class: "form-label", "Xem trước" }
                            ProfileCard { profile: d.clone(), effects_enabled: effects_enabled }
                        }
                    }
                } else if let Some(p) = profile() {
                    div { class: "profile-modal-view",
                        button { r#type: "button", class: "icon-btn profile-close", aria_label: "Đóng hồ sơ", onclick: move |_| on_close.call(()), CloseIcon {} }
                        ProfileCard { profile: p.clone(), effects_enabled: effects_enabled,
                            div { class: "profile-card-actions",
                                if is_self {
                                    button { r#type: "button", class: "btn-primary btn-block", onclick: move |_| draft.set(profile()), "Chỉnh sửa hồ sơ" }
                                } else if let Some(h) = on_message {
                                    button {
                                        r#type: "button",
                                        class: "btn-primary btn-block",
                                        onclick: { let id = p.id.clone(); move |_| h.call(id.clone()) },
                                        MessageIcon {}
                                        span { "Nhắn tin" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "modal-body", div { class: "empty-chat-inner", "Đang tải hồ sơ" } }
                }
            }
        }
    }
}

/// Small profile card anchored near the click position (bottom sheet on phones).
#[component]
pub fn ProfilePopover(
    user_id: String,
    x: f64,
    y: f64,
    is_self: bool,
    #[props(default = true)] effects_enabled: bool,
    on_close: EventHandler<()>,
    on_view_full: EventHandler<String>,
    #[props(default)] on_message: Option<EventHandler<String>>,
) -> Element {
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let uid = user_id.clone();
    use_effect(use_reactive!(|uid| {
        profile.set(None);
        spawn(async move {
            if let Ok(p) = get_user_profile(&uid).await {
                profile.set(Some(p));
            }
        });
    }));
    use_effect(move || {
        // Move focus into the popover so Escape and keyboard navigation work immediately.
        let _ = dioxus::document::eval("setTimeout(() => { const el = document.querySelector('.profile-popover'); if (el) el.focus(); }, 0);");
    });
    let left = x + 12.0;
    let top = y - 60.0;
    rsx! {
        button { r#type: "button", class: "menu-dismiss", aria_label: "Đóng hồ sơ", tabindex: "-1", onclick: move |_| on_close.call(()) }
        div {
            class: "profile-popover",
            tabindex: "-1",
            role: "dialog",
            aria_label: "Hồ sơ",
            style: "--pop-x: {left}px; --pop-y: {top}px;",
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            if let Some(p) = profile() {
                ProfileCard { profile: p.clone(), compact: true, effects_enabled: effects_enabled,
                    div { class: "profile-card-actions",
                        if !is_self {
                            if let Some(h) = on_message {
                                button { r#type: "button", class: "btn-primary", onclick: { let id = p.id.clone(); move |_| h.call(id.clone()) }, MessageIcon {} span { "Nhắn tin" } }
                            }
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: { let id = p.id.clone(); move |_| on_view_full.call(id.clone()) }, "Xem hồ sơ đầy đủ" }
                    }
                }
            } else {
                div { class: "empty-chat-inner", "Đang tải hồ sơ" }
            }
        }
    }
}
