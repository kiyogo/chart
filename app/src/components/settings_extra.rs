//! Extra settings pages: navigation & chat customization, and the sticker manager.

use dioxus::prelude::*;

use crate::{
    components::{
        guide::Illustration,
        icons::{ChevronRightIcon, CloseIcon, LockIcon, PlusIcon},
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
    ("none", "None"),
    ("dots", "Dots"),
    ("bubbles", "Bubble"),
    ("aurora", "Aurora"),
    ("paper", "Warm paper"),
];

#[component]
pub fn CustomizationSettings() -> Element {
    let ctx = use_context::<PrefsCtx>();
    let p = ctx.prefs.read().clone();
    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-desc", "Select the items you want to pin to the left sidebar to open them quickly" }
            div { class: "settings-list",
                Toggle {
                    label: "Pin communities".to_string(),
                    sub: "Show community icon on sidebar".to_string(),
                    checked: p.pin_communities,
                    on_change: move |v| ctx.update(|p| p.pin_communities = v),
                }
                Toggle {
                    label: "Pin groups".to_string(),
                    sub: "Show group avatar on the sidebar".to_string(),
                    checked: p.pin_groups,
                    on_change: move |v| ctx.update(|p| p.pin_groups = v),
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-list",
                Toggle {
                    label: "Message sound".to_string(),
                    sub: "Sound a bell when there is a new message or invitation".to_string(),
                    checked: p.sound,
                    on_change: move |v| {
                        ctx.update(|p| p.sound = v);
                        if v { platform::play_notification_chime(); }
                    },
                }
                Toggle {
                    label: "Profile effects".to_string(),
                    sub: "Show decorative effects when viewing profiles".to_string(),
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
                Err(_) => notice.set(Some((true, "Unable to load sticker list".into()))),
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
            div { class: "settings-group-title", "Your sticker set" }
            div { class: "settings-group-desc", "PNG, WebP, AVIF or animated GIF images, maximum 512 KB. Still images are automatically captured to 320×320." }
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
                                notice.set(Some((false, format!("Created set \"{}\"", p.name))));
                                reload();
                            }
                            Err(_) => notice.set(Some((true, "Unable to create sticker set".into()))),
                        }
                        busy.set(false);
                    });
                },
                input { maxlength: "64", placeholder: "New sticker set name", aria_label: "New sticker set name", value: "{new_pack}", oninput: move |e| new_pack.set(e.value()) }
                button { r#type: "submit", class: "btn-primary", disabled: new_pack().trim().is_empty() || busy(), PlusIcon { size: 16 } span { "Create sets" } }
            }

            if !mine.is_empty() {
                div { class: "sticker-upload",
                    select {
                        aria_label: "Select a set to add stickers",
                        value: "{target_pack().unwrap_or_default()}",
                        onchange: move |e| target_pack.set(Some(e.value())),
                        for p in mine.iter() { option { key: "{p.id}", value: "{p.id}", "{p.name}" } }
                    }
                    input { maxlength: "64", placeholder: "Sticker name", aria_label: "Sticker name", value: "{sticker_name}", oninput: move |e| sticker_name.set(e.value()) }
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
                                            _ => "This image file cannot be read. Choose PNG, WebP, AVIF or GIF.".to_string(),
                                        })?;
                                        if bytes.len() > 512 * 1024 { return Err("The file exceeds 512 KB".into()); }
                                        let (url, animated) = stickers::stickers_upload_image(bytes, &mime).await.map_err(|_| "Upload failed".to_string())?;
                                        stickers::stickers_add_sticker(&pid, &name, &url, animated).await.map_err(|_| "Cannot add stickers".to_string())?;
                                        Ok(())
                                    }.await;
                                    match result {
                                        Ok(()) => { sticker_name.set(String::new()); notice.set(Some((false, "Added stickers".into()))); reload(); }
                                        Err(e) if e == "no_file" => {}
                                        Err(e) => notice.set(Some((true, e))),
                                    }
                                    busy.set(false);
                                });
                            },
                        }
                        if busy() { span { class: "spinner" } } else { "Select photo and upload" }
                    }
                }
            }
            if let Some((is_err, msg)) = notice() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
            }

            if loading() {
                div { class: "empty-chat-inner", "Loading stickers" }
            } else if mine.is_empty() {
                div { class: "sidebar-empty", "You don't have your own sticker set yet. Create the first set above." }
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
                                        notice.set(Some((true, "The sticker set cannot be deleted".into())));
                                    }
                                });
                            }},
                            "Delete set"
                        }
                    }
                    div { class: "sticker-grid",
                        for st in pack.stickers.clone() {
                            div { key: "{st.id}", class: "sticker-cell manage", title: "{st.name}",
                                img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{st.name}", loading: "lazy" }
                                button {
                                    r#type: "button",
                                    class: "sticker-remove",
                                    aria_label: "Delete sticker {st.name}",
                                    onclick: { let pid = pack.id.clone(); let sid = st.id.clone(); move |_| {
                                        let (pid, sid) = (pid.clone(), sid.clone());
                                        spawn(async move {
                                            if stickers::stickers_delete_sticker(&pid, &sid).await.is_ok() { reload(); }
                                            else { notice.set(Some((true, "Cannot delete stickers".into()))); }
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
            div { class: "settings-group-title", "Set available" }
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
        0..=59 => "just finished".into(),
        60..=3599 => format!("{} minutes ago", secs / 60),
        3600..=86_399 => format!("{} hours ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
}

fn device_label(ua: Option<&str>) -> String {
    let ua = ua.unwrap_or("");
    let browser = if ua.contains("Edg/") { "Edge" } else if ua.contains("Firefox/") { "Firefox" } else if ua.contains("Chrome/") { "Chrome" } else if ua.contains("Safari/") { "Safari" } else if ua.is_empty() { "Devices" } else { "Application" };
    let os = if ua.contains("Windows") { "Windows" } else if ua.contains("Android") { "Android" } else if ua.contains("iPhone") || ua.contains("iPad") { "iOS" } else if ua.contains("Mac OS") { "macOS" } else if ua.contains("Linux") { "Linux" } else { "" };
    if os.is_empty() { browser.to_string() } else { format!("{browser} on {os}") }
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
            div { class: "modal-dialog story-dialog", role: "dialog", aria_modal: "true", aria_label: "Change login name", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: "user".to_string() }
                }
                match step() {
                    UserStep::Edit => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Change login name" }
                            p { class: "story-text", "Friends find you by this name. Write from 2 to 32 letters or numbers at once." }
                            label { class: "form-field", style: "text-align: left;",
                                span { class: "form-label", "New login name" }
                                input { maxlength: "32", value: "{value}", autocomplete: "off", oninput: move |e| { value.set(e.value()); error.set(None); } }
                            }
                            if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                        }
                        div { class: "story-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                            button { r#type: "button", class: "btn-primary", disabled: !valid, onclick: move |_| step.set(UserStep::Confirm), "Continue" }
                        }
                    },
                    UserStep::Confirm => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Confirm name change" }
                            p { class: "story-text", "The login name will change from " strong { "{current}" } " into " strong { "{v}" } ". Existing friends still stay connected to you." }
                            if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                        }
                        div { class: "story-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| step.set(UserStep::Edit), "Back" }
                            button { r#type: "button", class: "btn-primary", disabled: busy(),
                                onclick: move |_| {
                                    let name = value().trim().to_string();
                                    busy.set(true);
                                    spawn(async move {
                                        match library::change_username(&name).await {
                                            Ok(_) => { step.set(UserStep::Done); on_changed.call(name); }
                                            Err(e) => {
                                                error.set(Some(match e.code.as_str() {
                                                    "username_taken" => "This name already has a user".to_string(),
                                                    "invalid_username" | "username_invalid" => "Invalid name".to_string(),
                                                    _ => "Cannot change name".to_string(),
                                                }));
                                                step.set(UserStep::Edit);
                                            }
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { span { class: "spinner" } } else { "Rename" }
                            }
                        }
                    },
                    UserStep::Done => rsx! {
                        div { class: "story-body",
                            h2 { class: "story-title", "Name changed" }
                            p { class: "story-text", "Your new username is " strong { "{v}" } "." }
                        }
                        div { class: "story-actions single",
                            button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Done" }
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
            div { class: "modal-dialog story-dialog wide", role: "dialog", aria_modal: "true", aria_label: "Device is logged in", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: "devices".to_string() }
                }
                div { class: "story-body",
                    h2 { class: "story-title", "Device is logged in" }
                    p { class: "story-text", "Manage devices using your account. Click log out if you see a strange device." }
                }
                div { style: "padding: 0 var(--space-5);",
                    crate::components::storage_ui::DeviceTrustPanel {}
                }
                div { class: "device-list",
                    if loading() { div { class: "empty-chat-inner", "Loading" } }
                    for s in sessions() {
                        div { key: "{s.id}", class: "device-row cw-brand-row",
                            crate::components::brand_logos::DeviceBrandPair {
                                text: format!("{} {}", s.device_name.clone().unwrap_or_default(), s.device_label.clone().unwrap_or_default()),
                                size: 30,
                            }
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
                                                    Ok(_) => notice.set(Some((false, "Device name changed".into()))),
                                                    Err(_) => notice.set(Some((true, "Cannot change device name".into()))),
                                                }
                                                renaming.set(None);
                                                reload();
                                            });
                                        } },
                                        input { r#type: "text", maxlength: "40", aria_label: "Device name", value: "{renaming().map(|(_, n)| n).unwrap_or_default()}",
                                            oninput: { let id = s.id.clone(); move |e: Event<FormData>| renaming.set(Some((id.clone(), e.value()))) } }
                                        button { r#type: "submit", class: "btn-primary", "Save" }
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| renaming.set(None), "Cancel" }
                                    }
                                } else {
                                    span { class: "settings-label",
                                        "{s.device_label.clone().unwrap_or_else(|| device_label(s.device_name.as_deref()))}"
                                        if s.is_current { span { class: "chip-current", "This device" } }
                                        button { r#type: "button", class: "btn-ghost small", title: "Rename the device",
                                            onclick: { let id = s.id.clone(); let cur = s.device_label.clone().unwrap_or_default(); move |_| renaming.set(Some((id.clone(), cur.clone()))) },
                                            "Rename"
                                        }
                                    }
                                }
                                span { class: "settings-sublabel",
                                    "{s.ip_address.clone().unwrap_or_else(|| \"Unknown IP\".into())} · active {s.last_active.as_deref().map(relative_time).unwrap_or_default()}"
                                }
                            }
                            if !s.is_current {
                                if confirm_one().as_deref() == Some(s.id.as_str()) {
                                    div { class: "device-confirm",
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| confirm_one.set(None), "Cancel" }
                                        button { r#type: "button", class: "btn-danger",
                                            onclick: { let id = s.id.clone(); move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match library::sessions_revoke(&id).await {
                                                        Ok(_) => notice.set(Some((false, "Device logged out".into()))),
                                                        Err(_) => notice.set(Some((true, "Unable to log out of device".into()))),
                                                    }
                                                    confirm_one.set(None);
                                                    reload();
                                                });
                                            }},
                                            "Log out"
                                        }
                                    }
                                } else {
                                    button { r#type: "button", class: "btn-secondary", onclick: { let id = s.id.clone(); move |_| confirm_one.set(Some(id.clone())) }, "Log out" }
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
                            input { r#type: "text", maxlength: "12", aria_label: "Two-step verification code", placeholder: "6-digit code or backup code", value: "{twofa_code}", oninput: move |e| twofa_code.set(e.value()) }
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| { confirm_all.set(false); need_code.set(false); }, "Cancel" }
                        button { r#type: "button", class: "btn-danger",
                            onclick: move |_| {
                                spawn(async move {
                                    // Spec 011: the server asks for the second step when it is on.
                                    let c = twofa_code().trim().to_string();
                                    match crate::services::security::revoke_other_sessions(if c.is_empty() { None } else { Some(&c) }).await {
                                        Ok(_) => notice.set(Some((false, "Signed out of all other devices".into()))),
                                        Err(e) if e.code == "second_factor_required" => {
                                            need_code.set(true);
                                            notice.set(Some((true, "Enter the two-step verification code to continue".into())));
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
                            "Confirm logout of {others} device"
                        }
                    } else {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Close" }
                        button { r#type: "button", class: "btn-danger-outline", disabled: others == 0, onclick: move |_| confirm_all.set(true), "Sign out of all other devices" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AccountSecurity(
    current_username: String,
    #[props(default)] current_user_id: String,
    on_username_changed: EventHandler<String>,
) -> Element {
    let mut username = use_signal(|| current_username.clone());
    let mut dialog = use_signal(|| Option::<&'static str>::None);
    let mut device_count = use_signal(|| Option::<usize>::None);
    let mut has_password = use_signal(|| false);
    use_effect(move || {
        spawn(async move {
            if let Ok(list) = library::sessions_list().await {
                device_count.set(Some(list.len()));
            }
            if let Ok(me) = crate::services::api::auth_me().await {
                has_password.set(me.has_password);
            }
        });
    });
    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text", span { class: "settings-label", "Username" } span { class: "settings-sublabel", "{username}" } }
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| dialog.set(Some("username")), "Edit" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Email" }
                                span { class: "settings-sublabel", "Do not save emails. You sign in securely with Google." }
                            }
                            span { class: "chip-privacy", LockIcon { size: 14 } "Not stored" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Phone number" }
                                span { class: "settings-sublabel", "No phone number needed." }
                            }
                            span { class: "chip-privacy", LockIcon { size: 14 } "Not required" }
                        }
                    }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Account password & sync" }
                                span { class: "settings-sublabel",
                                    if has_password() {
                                        "Sign in on any device using just your password to sync all messages and settings"
                                    } else {
                                        "Set an account password to sign in and sync all devices without extra codes"
                                    }
                                }
                            }
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| dialog.set(Some("password")),
                                if has_password() { "Change" } else { "Set password" }
                            }
                        }
                        div { class: "settings-row cw-brand-row",
                            crate::components::brand_logos::GoogleAuthLogo { size: 28 }
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Sign-in method" }
                                span { class: "settings-sublabel", "Sign in securely with your Google account." }
                            }
                        }
                        button { r#type: "button", class: "settings-row row-button", onclick: move |_| dialog.set(Some("devices")),
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Device is logged in" }
                                span { class: "settings-sublabel", "View currently logged in devices" }
                            }
                            span { class: "row-trail",
                                if let Some(n) = device_count() { "{n} device" }
                                ChevronRightIcon { size: 16 }
                            }
                        }
                    }
                }
                if !current_user_id.is_empty() {
                    div { class: "settings-group",
                        div { class: "settings-list",
                            div { class: "settings-row",
                                span { class: "settings-label", "Account name" }
                                span { class: "settings-value", "{username}" }
                            }
                            div { class: "settings-row column-layout",
                                span { class: "settings-label", "Identifier" }
                                span { class: "settings-value mono", "{current_user_id}" }
                            }
                        }
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
        if dialog() == Some("password") {
            PasswordDialog {
                has_password: has_password(),
                current_user_id: current_user_id.clone(),
                on_close: move |_| dialog.set(None),
                on_saved: move |_| { has_password.set(true); },
            }
        }
    }
}

#[component]
fn PasswordDialog(
    has_password: bool,
    current_user_id: String,
    on_close: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let mut current_pw = use_signal(String::new);
    let mut new_pw = use_signal(String::new);
    let mut confirm_pw = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| Option::<String>::None);
    let mut done = use_signal(|| false);

    let submit = move |e: Event<FormData>| {
        e.prevent_default();
        if busy() {
            return;
        }
        let n = new_pw();
        if n.chars().count() < 6 || n.len() > 128 {
            err.set(Some("Password must be between 6 and 128 characters".into()));
            return;
        }
        if n != confirm_pw() {
            err.set(Some("Passwords do not match".into()));
            return;
        }
        let cur = current_pw();
        if has_password && cur.is_empty() {
            err.set(Some("Enter your current account password".into()));
            return;
        }
        let uid = current_user_id.clone();
        busy.set(true);
        err.set(None);
        spawn(async move {
            let cur_opt = if has_password { Some(cur.as_str()) } else { None };
            match crate::services::api::auth_set_password(cur_opt, &n).await {
                Ok(()) => {
                    if let Some(keys) = crate::crypto::e2ee::get_user_keys(Some(&uid)) {
                        let _ = crate::storage::sync_manager::ensure_password_key_backup(&keys, &n, true).await;
                    }
                    crate::crypto::e2ee::clear_ephemeral_account_password();
                    current_pw.set(String::new());
                    new_pw.set(String::new());
                    confirm_pw.set(String::new());
                    done.set(true);
                    on_saved.call(());
                }
                Err(e) => {
                    let msg = match e.code.as_str() {
                        "wrong_current_password" => "Current password is incorrect".to_string(),
                        "invalid_password_length" => "Password must be between 6 and 128 characters".to_string(),
                        _ => "Unable to update account password".to_string(),
                    };
                    err.set(Some(msg));
                }
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog story-dialog", role: "dialog", aria_modal: "true", aria_label: "Account password", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: "user".to_string() }
                }
                if done() {
                    div { class: "story-body",
                        h3 { class: "story-title", "Account password updated" }
                        p { class: "story-desc", "Your E2EE key backup is synced and protected by your account password. Use it to sign in cleanly on any device." }
                    }
                    div { class: "story-actions",
                        button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Done" }
                    }
                } else {
                    form { class: "story-body", onsubmit: submit,
                        h3 { class: "story-title", if has_password { "Change account password" } else { "Set account password" } }
                        p { class: "story-desc", "Your password lets you sign in and restore all encrypted messages and settings automatically on any device." }
                        if has_password {
                            input {
                                r#type: "password",
                                placeholder: "Current password",
                                aria_label: "Current password",
                                autocomplete: "current-password",
                                value: "{current_pw}",
                                oninput: move |e| { current_pw.set(e.value()); err.set(None); },
                            }
                        }
                        input {
                            r#type: "password",
                            placeholder: "New password (min 6 characters)",
                            aria_label: "New password",
                            autocomplete: "new-password",
                            value: "{new_pw}",
                            oninput: move |e| { new_pw.set(e.value()); err.set(None); },
                        }
                        input {
                            r#type: "password",
                            placeholder: "Confirm new password",
                            aria_label: "Confirm new password",
                            autocomplete: "new-password",
                            value: "{confirm_pw}",
                            oninput: move |e| { confirm_pw.set(e.value()); err.set(None); },
                        }
                        if let Some(m) = err() {
                            div { class: "status-badge error", "{m}" }
                        }
                        div { class: "story-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                            button { r#type: "submit", class: "btn-primary", disabled: busy(),
                                if busy() { "Saving" } else { "Save password" }
                            }
                        }
                    }
                }
            }
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
            div { class: "settings-group-title", "Custom emojis" }
            div { class: "settings-group-desc", "English names include letters, numbers, and underscores. Square image PNG/GIF/WebP ≤ 2 MB, displayed as :party_cat: in message." }
            div { class: "sticker-upload",
                input { maxlength: "32", placeholder: "party_cat", aria_label: "Emoji name", value: "{name}", oninput: move |e| name.set(e.value()) }
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
                                    let url = library::upload_media(bytes, &mime).await.map_err(|_| "Upload failed".to_string())?;
                                    library::emojis_create(&nm, &url).await.map_err(|e| if e.code == "emoji_name_taken" { "The emoji name already exists".to_string() } else { "Unable to add emoji".to_string() })?;
                                    Ok(())
                                }.await;
                                match res {
                                    Ok(()) => { name.set(String::new()); notice.set(Some((false, format!(":{nm}: ready")))); reload(); }
                                    Err(e) if e == "no_file" => {}
                                    Err(e) if e == "too_large" => notice.set(Some((true, "File larger than 2 MB".into()))),
                                    Err(e) => notice.set(Some((true, e))),
                                }
                                busy.set(false);
                            });
                        },
                    }
                    if busy() { span { class: "spinner" } } else { "Select photo and add" }
                }
            }
            if let Some((is_err, msg)) = notice() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" } }
            if items.read().is_empty() {
                div { class: "sidebar-empty", "There are no separate emojis yet." }
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
            div { class: "settings-group-title", "Your GIFs" }
            div { class: "settings-group-desc", "Download animated GIFs up to 2 MB or save ChatW's existing animations to your collection for quick sending." }
            div { class: "sticker-upload",
                input { maxlength: "64", placeholder: "GIF name", aria_label: "GIF name", value: "{name}", oninput: move |e| name.set(e.value()) }
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
                                    let url = library::upload_media(bytes, &mime).await.map_err(|_| "Upload failed".to_string())?;
                                    library::gifs_create(&nm, &url).await.map_err(|_| "GIF cannot be saved".to_string())?;
                                    Ok(())
                                }.await;
                                match res {
                                    Ok(()) => { name.set(String::new()); notice.set(Some((false, "GIFs added".into()))); reload(); }
                                    Err(e) if e == "no_file" => {}
                                    Err(e) if e == "too_large" => notice.set(Some((true, "File larger than 2 MB".into()))),
                                    Err(e) => notice.set(Some((true, e))),
                                }
                                busy.set(false);
                            });
                        },
                    }
                    if busy() { span { class: "spinner" } } else { "Select GIF and upload" }
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
                div { class: "settings-group-desc", "The stickers, emojis and GIFs you add here can be used in any conversation. The name is always in English." }
                select {
                    class: "cw-dropdown",
                    aria_label: "Library",
                    value: match tab() {
                        LibTab::Sticker => "sticker",
                        LibTab::Emoji => "emoji",
                        LibTab::Gif => "gif",
                    },
                    onchange: move |e| match e.value().as_str() {
                        "emoji" => tab.set(LibTab::Emoji),
                        "gif" => tab.set(LibTab::Gif),
                        _ => tab.set(LibTab::Sticker),
                    },
                    option { value: "sticker", selected: tab() == LibTab::Sticker, "Sticker" }
                    option { value: "emoji", selected: tab() == LibTab::Emoji, "Emoji" }
                    option { value: "gif", selected: tab() == LibTab::Gif, "GIF" }
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
