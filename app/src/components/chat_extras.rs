//! Chat page extras: pinned-message bar, shared per-chat wallpaper picker,
//! device rename dialog (for "Chính mình") and the conversation "more" menu.

use dioxus::prelude::*;

use crate::{
    components::icons::{BlockIcon, ChevronDownIcon, CloseIcon, ImageIcon, LinkIcon, PinIcon, TagIcon, TrashIcon, UserPlusIcon},
    platform,
    prefs::WALLPAPERS,
    services::library,
};

pub const WALLPAPER_NAMES: [(&str, &str); 9] = [
    ("none", "None"),
    ("dots", "Dots"),
    ("bubbles", "Bubble"),
    ("aurora", "Aurora"),
    ("paper", "Paper"),
    ("sunset", "Sunset"),
    ("ocean", "Ocean"),
    ("mint", "Mint"),
    ("night", "Starry night"),
];

fn safe_path(p: &str) -> bool {
    !p.contains("..") && p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

/// CSS class + inline style for a chat column given the shared wallpaper value
/// (from the other participant, so it is validated strictly here) and the
/// user's own default.
pub fn wallpaper_attrs(shared: Option<&str>, fallback: &str) -> (String, String) {
    let fallback = if WALLPAPERS.contains(&fallback) { fallback } else { "none" };
    match shared.filter(|v| !v.is_empty() && *v != "none") {
        Some(v) if WALLPAPERS.contains(&v) => (format!("wp-{v}"), String::new()),
        Some(v) if v.strip_prefix("color:#").is_some_and(|h| h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit())) => {
            ("wp-custom".into(), format!("--chat-wp-color: {};", &v[6..]))
        }
        Some(v) if (v.starts_with("/uploads/media/") || v.starts_with("/wallpapers/")) && safe_path(v) => {
            ("wp-custom wp-image".into(), format!("--chat-wp-image: url('{}');", platform::resolve_asset_url(v)))
        }
        _ => (format!("wp-{fallback}"), String::new()),
    }
}

/// Pinned messages of the open conversation: (message id, preview, sender).
#[component]
pub fn PinnedBar(items: Vec<(String, String, String)>, on_jump: EventHandler<String>, #[props(default)] on_unpin: Option<EventHandler<String>>) -> Element {
    let mut expanded = use_signal(|| false);
    let Some((first_id, first_text, first_sender)) = items.last().cloned() else {
        return rsx! {};
    };
    let count = items.len();
    rsx! {
        div { class: if expanded() { "pinned-bar expanded" } else { "pinned-bar" },
            div { class: "pinned-bar-main",
                span { class: "pinned-bar-icon", PinIcon { size: 16 } }
                button { r#type: "button", class: "pinned-bar-text", title: "Go to the pinned message",
                    onclick: move |_| on_jump.call(first_id.clone()),
                    span { class: "pinned-bar-label", if count > 1 { "Pinned messages · {count}" } else { "Pinned message" } }
                    span { class: "pinned-bar-preview", "{first_sender}: {first_text}" }
                }
                if count > 1 {
                    button { r#type: "button", class: "icon-btn", aria_expanded: "{expanded()}", aria_label: "View all pinned messages",
                        onclick: move |_| expanded.set(!expanded()),
                        ChevronDownIcon { size: 16 }
                    }
                }
            }
            if expanded() {
                div { class: "pinned-list",
                    for (id, text, sender) in items.iter().rev().cloned() {
                        {
                            let jump = id.clone();
                            let unpin = id.clone();
                            rsx! {
                                div { key: "{id}", class: "pinned-item",
                                    button { r#type: "button", class: "pinned-item-text", onclick: move |_| { expanded.set(false); on_jump.call(jump.clone()); },
                                        span { class: "pinned-item-sender", "{sender}" }
                                        span { "{text}" }
                                    }
                                    if let Some(h) = on_unpin {
                                        button { r#type: "button", class: "icon-btn", title: "Unpin", aria_label: "Unpin", onclick: move |_| h.call(unpin.clone()), CloseIcon { size: 14 } }
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

/// Pick a wallpaper for one conversation. `shared` explains that both sides see it.
#[component]
pub fn WallpaperPicker(current: Option<String>, shared: bool, on_pick: EventHandler<String>, on_close: EventHandler<()>) -> Element {
    let mut color = use_signal(|| current.as_deref().and_then(|c| c.strip_prefix("color:")).unwrap_or("#DCE8F7").to_string());
    let mut uploading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    use_hook(|| platform::focus_selector("[aria-labelledby='wp-title']"));
    let cur = current.clone().unwrap_or_else(|| "none".into());
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content", role: "dialog", aria_modal: "true", aria_labelledby: "wp-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header", h3 { id: "wp-title", class: "modal-title", "Chat wallpaper" } }
                p { class: "modal-sub",
                    if shared { "This background image applies to both you and the people in the chat." } else { "This wallpaper applies only on your devices." }
                }
                div { class: "wallpaper-grid",
                    for (id, name) in WALLPAPER_NAMES {
                        button { key: "{id}", r#type: "button", class: if cur == id { "wallpaper-tile active" } else { "wallpaper-tile" },
                            aria_pressed: "{cur == id}",
                            onclick: move |_| on_pick.call(id.to_string()),
                            span { class: "wallpaper-sample wp-{id}" }
                            span { "{name}" }
                        }
                    }
                }
                div { class: "settings-group-title", "Own color" }
                div { class: "inline-form",
                    input { r#type: "color", aria_label: "Choose background color", value: "{color}", oninput: move |e| color.set(e.value()) }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| on_pick.call(format!("color:{}", color().to_uppercase())), "Use this color" }
                }
                div { class: "settings-group-title", "Your photo" }
                div { class: "inline-form",
                    input { id: "chat-wp-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                        onchange: move |_| {
                            uploading.set(true);
                            error.set(None);
                            spawn(async move {
                                match platform::process_selected_wallpaper_webp("chat-wp-file").await {
                                    Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                        Ok(url) => on_pick.call(url),
                                        Err(err) => {
                                            let reason = match err.code.as_str() {
                                                "invalid_media_size" => "Photo larger than 10 MB",
                                                "unsupported_media_format" => "Image format not supported",
                                                "svg_upload_not_allowed" => "SVG images are not supported",
                                                _ => "Unable to upload photo",
                                            };
                                            error.set(Some(reason.into()));
                                        }
                                    },
                                    Err(e) if e == "no_file" => {}
                                    Err(e) if e == "svg_not_allowed" => error.set(Some("SVG images are not supported".into())),
                                    Err(e) if e == "too_large" => error.set(Some("Photo larger than 8 MB".into())),
                                    Err(e) if e == "unsupported" => error.set(Some("Image format not supported".into())),
                                    Err(_) => error.set(Some("Unable to process image".into())),
                                }
                                uploading.set(false);
                            });
                        },
                    }
                    label { r#for: "chat-wp-file", class: "btn-secondary", role: "button", tabindex: "0",
                        ImageIcon { size: 16 }
                        if uploading() { "Uploading" } else { "Upload image" }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Close" }
                }
            }
        }
    }
}

/// Rename one of my devices (sessions), e.g. "Linux 1" → "Laptop công ty".
#[component]
pub fn RenameDeviceDialog(current: String, on_save: EventHandler<String>, on_close: EventHandler<()>) -> Element {
    let mut name = use_signal(|| current.clone());
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content compact", role: "dialog", aria_modal: "true", aria_labelledby: "rename-dev-title", onclick: move |e| e.stop_propagation(),
                h3 { id: "rename-dev-title", class: "modal-title", "Rename the device" }
                p { class: "modal-sub", "This name displays in \"Yourself\" and the list of logged in devices." }
                form { onsubmit: move |e| { e.prevent_default(); let n = name().trim().to_string(); if !n.is_empty() { on_save.call(n); } },
                    input { r#type: "text", maxlength: "40", aria_label: "Device name", value: "{name}", autofocus: true, oninput: move |e| name.set(e.value()) }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                        button { r#type: "submit", class: "btn-primary", disabled: name().trim().is_empty(), "Save" }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChatAction {
    Wallpaper,
    Labels,
    Block,
    Unblock,
    InviteFriends,
    CopyLink,
    ManageGroup,
    ClearSelf,
}

/// The "more" menu in a chat header.
#[component]
pub fn ChatMoreMenu(actions: Vec<ChatAction>, on_action: EventHandler<ChatAction>, on_close: EventHandler<()>) -> Element {
    rsx! {
        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Close", onclick: move |_| on_close.call(()) }
        div { class: "account-menu chat-more-menu", role: "menu",
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            for a in actions {
                button { key: "{a:?}", r#type: "button", role: "menuitem",
                    class: if matches!(a, ChatAction::Block | ChatAction::ClearSelf) { "account-menu-item danger" } else { "account-menu-item" },
                    onclick: move |_| { on_close.call(()); on_action.call(a) },
                    match a {
                        ChatAction::Wallpaper => rsx! { ImageIcon { size: 16 } "Change chat wallpaper" },
                        ChatAction::Labels => rsx! { TagIcon { size: 16 } "Classification card" },
                        ChatAction::Block => rsx! { BlockIcon { size: 16 } "Block user" },
                        ChatAction::Unblock => rsx! { BlockIcon { size: 16 } "Unblock" },
                        ChatAction::InviteFriends => rsx! { UserPlusIcon { size: 16 } "Invite friends" },
                        ChatAction::CopyLink => rsx! { LinkIcon { size: 16 } "Copy invite link" },
                        ChatAction::ManageGroup => rsx! { crate::components::icons::SettingsIcon {} "Manage group" },
                        ChatAction::ClearSelf => rsx! { TrashIcon { size: 16 } "Delete \"Yourself\"" },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_values_are_validated() {
        assert_eq!(wallpaper_attrs(Some("ocean"), "none").0, "wp-ocean");
        assert_eq!(wallpaper_attrs(Some("color:#12ab34"), "none").1, "--chat-wp-color: #12ab34;");
        // Anything that could break out of the CSS value falls back.
        assert_eq!(wallpaper_attrs(Some("color:red;}"), "dots").0, "wp-dots");
        assert_eq!(wallpaper_attrs(Some("/uploads/media/a.png') ; x"), "none").0, "wp-none");
        assert_eq!(wallpaper_attrs(None, "bogus").0, "wp-none");
    }
}
