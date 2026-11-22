//! More settings pages: chat behaviour, notifications, privacy & blocking,
//! app lock (PIN), automatic replies, language and deep appearance options.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        chat_extras::WALLPAPER_NAMES,
        icons::{ImageIcon, LockIcon, SearchIcon},
    },
    i18n::{LANGUAGES, t},
    models::{BlockedUser, PrivacySettings},
    platform,
    prefs::{Prefs, PrefsCtx},
    services::{library, social},
};

fn ctx() -> PrefsCtx {
    use_context::<PrefsCtx>()
}

#[component]
pub fn Toggle(label: String, #[props(default)] desc: String, checked: bool, on_change: EventHandler<bool>, #[props(default)] disabled: bool) -> Element {
    rsx! {
        label { class: "settings-row toggle-row",
            span { class: "grow",
                span { class: "settings-label", "{label}" }
                if !desc.is_empty() { span { class: "settings-row-desc", "{desc}" } }
            }
            input { r#type: "checkbox", class: "switch", checked: checked, disabled: disabled, onchange: move |e| on_change.call(e.checked()) }
        }
    }
}

#[component]
pub fn SelectMenu(options: Vec<(String, String)>, value: String, on_change: EventHandler<String>, #[props(default)] label: String) -> Element {
    rsx! {
        select {
            class: "cw-dropdown",
            aria_label: "{label}",
            value: "{value}",
            onchange: move |e| on_change.call(e.value()),
            for (v, l) in options {
                option { key: "{v}", value: "{v}", selected: value == v, "{l}" }
            }
        }
    }
}

fn opts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// Server-side privacy settings shared by several pages.
fn use_privacy() -> (Signal<Option<PrivacySettings>>, Signal<Option<String>>) {
    let mut p = use_signal(|| Option::<PrivacySettings>::None);
    let err = use_signal(|| Option::<String>::None);
    use_hook(move || {
        spawn(async move {
            if let Ok(s) = social::privacy_get().await {
                p.set(Some(s));
            }
        });
    });
    (p, err)
}

fn save_privacy(mut p: Signal<Option<PrivacySettings>>, mut err: Signal<Option<String>>, fields: serde_json::Value) {
    spawn(async move {
        match social::privacy_set(fields).await {
            Ok(s) => {
                p.set(Some(s));
                err.set(None);
            }
            Err(_) => err.set(Some("Changes cannot be saved".into())),
        }
    });
}

#[component]
pub fn ChatPrefsSettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let (privacy, err) = use_privacy();
    let pv = privacy().unwrap_or_default();
    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    Toggle { label: "Double-click to reply", desc: "Double-click next to the message to quickly reply", checked: pr.double_click_reply, on_change: move |v| c.update(|p| p.double_click_reply = v) }
                    Toggle { label: "Press Enter to send", desc: "Press the Enter key to send the message", checked: pr.enter_to_send, on_change: move |v| c.update(|p| p.enter_to_send = v) }
                    Toggle { label: "Sticker suggestions", desc: "Self-suggesting stickers when typing", checked: pr.sticker_suggest, on_change: move |v| c.update(|p| p.sticker_suggest = v) }
                    Toggle { label: "Mention suggestions", desc: "Type @ to call your friend's name", checked: pr.mention_suggest, on_change: move |v| c.update(|p| p.mention_suggest = v) }
                    Toggle { label: "Emoji suggestions", desc: "Type two dots to find the smiley face", checked: pr.emoji_suggest, on_change: move |v| c.update(|p| p.emoji_suggest = v) }
                    Toggle { label: "Show avatars in messages", checked: pr.show_avatars, on_change: move |v| c.update(|p| p.show_avatars = v) }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
                    Toggle { label: "Typing indicator", desc: "Let your friends know when you're texting", checked: pv.typing, disabled: privacy().is_none(),
                        on_change: move |v: bool| save_privacy(privacy, err, json!({ "typing": v })) }
                    Toggle { label: "Read receipts", desc: "Let your friends know when you've read the news", checked: pv.read_receipts, disabled: privacy().is_none(),
                        on_change: move |v: bool| save_privacy(privacy, err, json!({ "read_receipts": v })) }
                }
                div { class: "settings-group",
                    div { class: "settings-row",
                        span { class: "settings-label", "Spreadsheet paste" }
                        SelectMenu { label: "How to paste spreadsheets", value: pr.paste_mode.clone(), options: opts(&[("ask", "Ask first"), ("image", "As image"), ("table", "As table"), ("text", "As text")]), on_change: move |v: String| c.update(|p| p.paste_mode = v) }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NotificationSettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut perm = use_signal(|| Option::<String>::None);
    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    Toggle { label: "Enable message notifications", desc: "Notify when a message is sent", checked: pr.notify,
                        on_change: move |v: bool| {
                            c.update(|p| p.notify = v);
                            if v {
                                spawn(async move {
                                    let r = platform::request_notification_permission().await;
                                    perm.set(Some(r));
                                });
                            }
                        },
                    }
                    if let Some(p) = perm() {
                        div { class: if p == "granted" { "status-badge success" } else { "status-badge error" },
                            match p.as_str() {
                                "granted" => "Notifications allowed",
                                "denied" => "The browser is blocking notifications. Please allow in site settings.",
                                _ => "This device does not support notifications",
                            }
                        }
                    }
                    Toggle { label: "Show message preview", desc: "Preview message content", checked: pr.notify_preview, on_change: move |v| c.update(|p| p.notify_preview = v) }
                    Toggle { label: "Group message notifications", checked: pr.notify_groups, on_change: move |v| c.update(|p| p.notify_groups = v) }
                    Toggle { label: "Incoming call notifications", checked: pr.notify_calls, on_change: move |v| c.update(|p| p.notify_calls = v) }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    Toggle { label: "Play notification sound", checked: pr.sound, on_change: move |v| c.update(|p| p.sound = v) }
                    div { class: "settings-row",
                        span { class: "settings-label", "Alert tone" }
                        SelectMenu { label: "Alert tone", value: pr.sound_name.clone(), options: opts(&[("pop", "Pop"), ("chime", "Chime"), ("soft", "Gentle"), ("bubble", "Bubble")]),
                            on_change: move |v: String| { platform::play_sound(&v); c.update(|p| p.sound_name = v); },
                        }
                    }
                    p { class: "settings-group-desc", "Turn off notifications for each chat in that chat's settings" }
                }
            }
        }
    }
}

#[component]
pub fn PrivacySettingsPanel() -> Element {
    let (privacy, err) = use_privacy();
    let pv = privacy().unwrap_or_default();
    let mut blocked = use_signal(Vec::<BlockedUser>::new);
    use_hook(move || {
        spawn(async move {
            if let Ok(b) = social::blocks_list().await {
                blocked.set(b);
            }
        });
    });
    rsx! {
        if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-row",
                        span { class: "settings-label", "Who can send friend requests" }
                        SelectMenu { label: "Friend requests", value: pv.friend_requests.clone(), options: opts(&[("everyone", "Everyone"), ("nobody", "Nobody")]),
                            on_change: move |v: String| save_privacy(privacy, err, json!({ "friend_requests": v })) }
                    }
                    div { class: "settings-row",
                        span { class: "settings-label", "Who can call you" }
                        SelectMenu { label: "Call", value: pv.calls.clone(), options: opts(&[("friends", "Friends"), ("nobody", "Nobody")]),
                            on_change: move |v: String| save_privacy(privacy, err, json!({ "calls": v })) }
                    }
                    div { class: "settings-row",
                        span { class: "settings-label", "Who can add you to groups" }
                        SelectMenu { label: "Add to group", value: pv.group_invites.clone(), options: opts(&[("everyone", "Everyone"), ("friends", "Friends"), ("nobody", "Nobody")]),
                            on_change: move |v: String| save_privacy(privacy, err, json!({ "group_invites": v })) }
                    }
                    Toggle { label: "Allow finding me by username", desc: "Others can find you through your account name", checked: pv.discoverable, disabled: privacy().is_none(),
                        on_change: move |v: bool| save_privacy(privacy, err, json!({ "discoverable": v })) }
                    Toggle { label: "Read receipts", checked: pv.read_receipts, disabled: privacy().is_none(), on_change: move |v: bool| save_privacy(privacy, err, json!({ "read_receipts": v })) }
                    Toggle { label: "Typing indicator", checked: pv.typing, disabled: privacy().is_none(), on_change: move |v: bool| save_privacy(privacy, err, json!({ "typing": v })) }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Blocked list" }
                    p { class: "settings-group-desc", "The blocked person cannot text or call you" }
                    if blocked.read().is_empty() { div { class: "sidebar-empty", "You haven't blocked anyone" } }
                    for b in blocked() {
                        {
                            let id = b.id.clone();
                            rsx! {
                                div { key: "{b.id}", class: "member-row",
                                    Avatar { name: b.username.clone(), avatar_url: b.avatar_url.clone(), size: 36 }
                                    span { class: "grow member-name", "{b.username}" }
                                    button { r#type: "button", class: "btn-secondary",
                                        onclick: move |_| { let id = id.clone(); spawn(async move {
                                            if social::unblock_user(&id).await.is_ok() { blocked.with_mut(|l| l.retain(|x| x.id != id)); }
                                        }); },
                                        "Unblock"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "settings-group",
                    p { class: "settings-group-desc", "Private messages between two people are securely protected and cannot be read by outsiders and servers" }
                }
            }
        }
    }
}

/// Salted PBKDF2-SHA256 (210 000 rounds) via WebCrypto; the PIN never leaves memory.
pub async fn hash_pin(pin: &str, salt_b64: &str) -> Option<String> {
    let p = serde_json::to_string(pin).ok()?;
    let s = serde_json::to_string(salt_b64).ok()?;
    let mut ev = dioxus::document::eval(&format!(
        r#"
        (async () => {{
            try {{
                const enc = new TextEncoder();
                const salt = Uint8Array.from(atob({s}), (c) => c.charCodeAt(0));
                const key = await crypto.subtle.importKey('raw', enc.encode({p}), 'PBKDF2', false, ['deriveBits']);
                const bits = await crypto.subtle.deriveBits({{ name: 'PBKDF2', hash: 'SHA-256', salt, iterations: 210000 }}, key, 256);
                dioxus.send(btoa(String.fromCharCode(...new Uint8Array(bits))));
            }} catch (e) {{ dioxus.send(''); }}
        }})();
        "#
    ));
    ev.recv::<String>().await.ok().filter(|h| !h.is_empty())
}

pub async fn new_salt() -> String {
    let mut ev = dioxus::document::eval("const a = new Uint8Array(16); crypto.getRandomValues(a); dioxus.send(btoa(String.fromCharCode(...a)));");
    ev.recv::<String>().await.unwrap_or_default()
}

pub fn same(a: &str, b: &str) -> bool {
    let (x, y) = (a.as_bytes(), b.as_bytes());
    x.len() == y.len() && x.iter().zip(y).fold(0u8, |acc, (p, q)| acc | (p ^ q)) == 0
}

pub fn valid_pin(pin: &str) -> bool {
    (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit())
}

#[component]
pub fn LockSettings(on_lock_now: EventHandler<()>) -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut pin1 = use_signal(String::new);
    let mut pin2 = use_signal(String::new);
    let mut current = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);
    let has_pin = !pr.pin_hash.is_empty();
    let set_pin = move |_| {
        let (a, b) = (pin1(), pin2());
        if !valid_pin(&a) {
            msg.set(Some((true, "PIN code consists of 4 to 8 digits".into())));
            return;
        }
        if a != b {
            msg.set(Some((true, "The two PIN entries do not match".into())));
            return;
        }
        busy.set(true);
        let old = current();
        spawn(async move {
            let p = c.prefs.peek().clone();
            if !p.pin_hash.is_empty() {
                let ok = hash_pin(&old, &p.pin_salt).await.is_some_and(|h| same(&h, &p.pin_hash));
                if !ok {
                    msg.set(Some((true, "The current PIN code is incorrect".into())));
                    busy.set(false);
                    return;
                }
            }
            let salt = new_salt().await;
            match hash_pin(&a, &salt).await {
                Some(h) => {
                    c.update(move |p| {
                        p.pin_salt = salt;
                        p.pin_hash = h;
                        p.lock_enabled = true;
                    });
                    pin1.set(String::new());
                    pin2.set(String::new());
                    current.set(String::new());
                    msg.set(Some((false, "PIN set. ChatW will lock when you leave for too long.".into())));
                }
                None => msg.set(Some((true, "The device does not support secure PIN generation".into()))),
            }
            busy.set(false);
        });
    };
    let remove_pin = move |_| {
        let old = current();
        spawn(async move {
            let p = c.prefs.peek().clone();
            if hash_pin(&old, &p.pin_salt).await.is_some_and(|h| same(&h, &p.pin_hash)) {
                c.update(|p| {
                    p.pin_hash.clear();
                    p.pin_salt.clear();
                    p.lock_enabled = false;
                });
                current.set(String::new());
                msg.set(Some((false, "PIN code disabled".into())));
            } else {
                msg.set(Some((true, "The current PIN code is incorrect".into())));
            }
        });
    };
    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    p { class: "settings-group-desc", "Lock the application with a digital password to protect messages on this device" }
                    if has_pin {
                        Toggle { label: "Enable app lock", checked: pr.lock_enabled, on_change: move |v| c.update(|p| p.lock_enabled = v) }
                        Toggle { label: "Lock every time app opens", checked: pr.lock_on_start, on_change: move |v| c.update(|p| p.lock_on_start = v) }
                        div { class: "settings-row",
                            span { class: "settings-label", "Lock after inactivity" }
                            SelectMenu { label: "Lock time", value: pr.lock_minutes.to_string(), options: opts(&[("1", "1 minute"), ("5", "5 minutes"), ("15", "15 minutes"), ("30", "30 minutes"), ("60", "1 hour")]),
                                on_change: move |v: String| c.update(|p| p.lock_minutes = v.parse().unwrap_or(5)) }
                        }
                        div { class: "inline-form",
                            button { r#type: "button", class: "btn-secondary", disabled: !pr.lock_enabled, onclick: move |_| on_lock_now.call(()), LockIcon { size: 16 } "Lock now" }
                        }
                    } else {
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Lock status" }
                                span { class: "settings-sublabel", "Set a numeric password on the right to lock the application on this device" }
                            }
                            span { class: "chip-privacy", "Off" }
                        }
                    }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", if has_pin { "Change lock PIN" } else { "Set lock PIN" } }
                    div { class: "form-grid",
                        if has_pin {
                            label { class: "form-field", span { "Current PIN" }
                                input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "off", maxlength: "8", value: "{current}", oninput: move |e| current.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                            }
                        }
                        label { class: "form-field", span { "New PIN (4–8 digits)" }
                            input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "new-password", maxlength: "8", value: "{pin1}", oninput: move |e| pin1.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                        }
                        label { class: "form-field", span { "Confirm new PIN" }
                            input { r#type: "password", class: "pin-input", inputmode: "numeric", autocomplete: "new-password", maxlength: "8", value: "{pin2}", oninput: move |e| pin2.set(e.value().chars().filter(|c| c.is_ascii_digit()).collect()) }
                        }
                    }
                    if let Some((is_err, m)) = msg() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{m}" } }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: set_pin, if has_pin { "Change PIN" } else { "Set PIN" } }
                        if has_pin { button { r#type: "button", class: "btn-danger-outline", onclick: remove_pin, "Disable lock" } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AutoReplySettings() -> Element {
    let c = ctx();
    let pr = c.prefs.read().clone();
    let mut text = use_signal(|| pr.auto_reply_text.clone());
    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    p { class: "settings-group-desc", "Automatically text back when someone texts while you're busy" }
                    Toggle { label: "Enable auto reply", checked: pr.auto_reply, on_change: move |v| c.update(|p| p.auto_reply = v) }
                    div { class: "settings-row",
                        span { class: "settings-label", "When to reply" }
                        SelectMenu { label: "When to reply", value: pr.auto_reply_mode.clone(), options: opts(&[("away", "When away"), ("hours", "Scheduled hours"), ("always", "Always")]),
                            on_change: move |v: String| c.update(|p| p.auto_reply_mode = v) }
                    }
                    if pr.auto_reply_mode == "hours" {
                        div { class: "inline-form",
                            label { class: "form-field", span { "From" } input { r#type: "time", value: "{pr.auto_reply_from}", onchange: move |e| { let v = e.value(); c.update(move |p| p.auto_reply_from = v) } } }
                            label { class: "form-field", span { "To" } input { r#type: "time", value: "{pr.auto_reply_to}", onchange: move |e| { let v = e.value(); c.update(move |p| p.auto_reply_to = v) } } }
                        }
                    }
                }
            }
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Auto-reply message" }
                    label { class: "form-field",
                        span { "Outgoing message" }
                        textarea { rows: "4", maxlength: "500", value: "{text}", oninput: move |e| text.set(e.value()), onchange: move |_| { let v = text(); c.update(move |p| p.auto_reply_text = v) } }
                    }
                    p { class: "settings-group-desc", "Automatic messages have the word [Auto] added at the beginning for the recipient to know" }
                }
            }
        }
    }
}

#[component]
pub fn LanguageSettings() -> Element {
    let c = ctx();
    let cur = c.prefs.read().language.clone();
    let mut query = use_signal(String::new);
    let _ = t;
    let q = query().trim().to_lowercase();
    let filtered: Vec<_> = LANGUAGES
        .iter()
        .copied()
        .filter(|l| {
            q.is_empty()
                || l.native.to_lowercase().contains(&q)
                || l.english.to_lowercase().contains(&q)
                || l.code.to_lowercase().contains(&q)
        })
        .collect();
    let total_shown = filtered.len();
    rsx! {
        div { class: "settings-group",
            div { class: "lang-search-row",
                div { class: "search-field lang-search-field",
                    SearchIcon { size: 15 }
                    input {
                        r#type: "search",
                        aria_label: "Search languages",
                        placeholder: "Search languages",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
                span { class: "lang-count-badge", "{total_shown} languages" }
            }
            if filtered.is_empty() {
                div { class: "empty-inline", "No matching languages found" }
            } else {
                div { class: "lang-grid", role: "radiogroup", aria_label: "Language",
                    for lang in filtered {
                        button {
                            key: "{lang.code}",
                            r#type: "button",
                            role: "radio",
                            aria_checked: "{cur == lang.code}",
                            lang: "{lang.code}",
                            translate: "no",
                            title: "{lang.english} ({lang.code})",
                            class: if cur == lang.code { "lang-tile active notranslate" } else { "lang-tile notranslate" },
                            onclick: move |_| c.update(|p| p.language = lang.code.to_string()),
                            crate::components::brand_logos::LanguageFlagLogo { code: lang.code.to_string(), size: 24 }
                            span { class: "lang-native", "{lang.native}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AppearanceColorsShapes() -> Element {
    let c = ctx();
    let pr: Prefs = c.prefs.read().clone();
    let accent_val = if pr.custom_accent.is_empty() {
        platform::accent_preset_hex(platform::load_appearance().accent.as_str()).to_string()
    } else {
        pr.custom_accent.clone()
    };
    let self_val = if pr.bubble_self.is_empty() { accent_val.clone() } else { pr.bubble_self.clone() };
    let self_sync = if pr.bubble_self.is_empty() { "true" } else { "false" };
    let other_val = if pr.bubble_other.is_empty() { "#EEF1F5".to_string() } else { pr.bubble_other.clone() };
    rsx! {
        div { class: "settings-group",
            div { class: "settings-row",
                span { class: "grow", span { class: "settings-label", "Your bubble" } }
                if !pr.bubble_self.is_empty() { button { r#type: "button", class: "btn-secondary", onclick: move |_| c.update(|p| p.bubble_self.clear()), "Default" } }
                input { r#type: "color", aria_label: "Your bubble color", "data-sync-accent": "{self_sync}", value: "{self_val}", onchange: move |e| { let v = e.value().to_uppercase(); c.update(move |p| p.bubble_self = v) } }
            }
            div { class: "settings-row",
                span { class: "grow", span { class: "settings-label", "Other bubble" } }
                if !pr.bubble_other.is_empty() { button { r#type: "button", class: "btn-secondary", onclick: move |_| c.update(|p| p.bubble_other.clear()), "Default" } }
                input { r#type: "color", aria_label: "Other bubble color", value: "{other_val}", onchange: move |e| { let v = e.value().to_uppercase(); c.update(move |p| p.bubble_other = v) } }
            }
        }
        div { class: "settings-group",
            div { class: "settings-row",
                span { class: "settings-label", "Bubble" }
                SelectMenu { label: "Bubble", value: pr.bubble_shape.clone(), options: opts(&[("round", "Round"), ("soft", "Soft"), ("square", "Square"), ("leaf", "Leaf")]), on_change: move |v: String| c.update(|p| p.bubble_shape = v) }
            }
            div { class: "settings-row",
                span { class: "settings-label", "Avatar" }
                SelectMenu { label: "Avatar", value: pr.avatar_shape.clone(), options: opts(&[("circle", "Round"), ("squircle", "Squircle"), ("square", "Square")]), on_change: move |v: String| c.update(|p| p.avatar_shape = v) }
            }
            div { class: "settings-row",
                span { class: "settings-label", "Message width" }
                SelectMenu { label: "Message width", value: pr.chat_width.clone(), options: opts(&[("narrow", "Narrow"), ("normal", "Normal"), ("wide", "Wide"), ("full", "Full width")]), on_change: move |v: String| c.update(|p| p.chat_width = v) }
            }
            div { class: "settings-row",
                span { class: "settings-label", "Font" }
                SelectMenu { label: "Font", value: pr.font_family.clone(), options: opts(&[("system", "System"), ("rounded", "Rounded"), ("serif", "Serif"), ("mono", "Monospace"), ("dyslexic", "Readable")]), on_change: move |v: String| c.update(|p| p.font_family = v) }
            }
            Toggle { label: "Reduce motion", desc: "Turn off animations and transitions", checked: pr.reduce_motion, on_change: move |v| c.update(|p| p.reduce_motion = v) }
        }
    }
}

#[component]
pub fn AppearanceWallpapersGlass(#[props(default)] on_reset_appearance: Option<EventHandler<()>>) -> Element {
    let c = ctx();
    let pr: Prefs = c.prefs.read().clone();
    let mut uploading = use_signal(|| false);
    let mut msg = use_signal(|| Option::<String>::None);
    let wp_color = pr.app_wallpaper.strip_prefix("color:")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "#60A5FA".to_string());
    let panel_transparency = 100u8.saturating_sub(pr.panel_opacity);
    let panel_transparency_pct = panel_transparency;
    let panel_blur_pct = ((pr.panel_blur as f32 / 32.0) * 100.0).round() as u8;
    let wp_dim_pct = ((pr.app_wallpaper_dim as f32 / 80.0) * 100.0).round() as u8;
    let wp_blur_pct = ((pr.app_wallpaper_blur as f32 / 24.0) * 100.0).round() as u8;
    rsx! {
        div { class: "settings-group",
            div { class: "settings-row column-layout",
                span { class: "settings-label", "Chat wallpaper" }
                div { class: "wallpaper-grid", role: "radiogroup", aria_label: "Chat wallpaper",
                    for (id, name) in WALLPAPER_NAMES.iter().copied() {
                        button {
                            key: "chat-wp-{id}",
                            r#type: "button",
                            role: "radio",
                            aria_checked: "{pr.wallpaper == id}",
                            class: if pr.wallpaper == id { "wallpaper-tile active" } else { "wallpaper-tile" },
                            onclick: move |_| c.update(|p| p.wallpaper = id.to_string()),
                            span { class: "wallpaper-sample wp-{id}" }
                            span { "{name}" }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-row column-layout",
                span { class: "settings-label", "App wallpaper" }
                div { class: "wallpaper-grid",
                    button { r#type: "button", class: if pr.app_wallpaper.is_empty() { "wallpaper-tile active" } else { "wallpaper-tile" }, onclick: move |_| c.update(|p| p.app_wallpaper.clear()),
                        span { class: "wallpaper-sample wp-none" } span { "None" }
                    }
                    for (id, name) in WALLPAPER_NAMES.iter().copied().filter(|(id, _)| *id != "none") {
                        button { key: "{id}", r#type: "button", class: if pr.app_wallpaper == id { "wallpaper-tile active" } else { "wallpaper-tile" },
                            onclick: move |_| c.update(|p| p.app_wallpaper = id.to_string()),
                            span { class: "wallpaper-sample wp-{id}" } span { "{name}" }
                        }
                    }
                }
            }
            div { class: "inline-form",
                input { r#type: "color", aria_label: "App background color", value: "{wp_color}", onchange: move |e| { let v = format!("color:{}", e.value().to_uppercase()); c.update(move |p| p.app_wallpaper = v) } }
                input { id: "app-wp-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                    onchange: move |_| {
                        uploading.set(true);
                        spawn(async move {
                            match platform::process_selected_wallpaper_webp("app-wp-file").await {
                                Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                    Ok(url) => c.update(move |p| p.app_wallpaper = url),
                                    Err(_) => msg.set(Some("Unable to upload photo".into())),
                                },
                                Err(e) if e == "too_large" => msg.set(Some("Photo larger than 2 MB".into())),
                                Err(e) if e == "unsupported" => msg.set(Some("Image format not supported".into())),
                                Err(_) => {}
                            }
                            uploading.set(false);
                        });
                    },
                }
                label { r#for: "app-wp-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } if uploading() { "Uploading" } else { "Upload your image" } }
            }
            if let Some(m) = msg() { div { class: "status-badge error", "{m}" } }
            div { class: "settings-row",
                span { class: "grow",
                    span { class: "settings-label", "Panel transparency: {panel_transparency}%" }
                    span { class: "settings-row-desc", "Drag to the right to clearly see the background image behind" }
                }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{panel_transparency}",
                    style: "--range-pct: {panel_transparency_pct}%;",
                    aria_label: "Panel transparency",
                    oninput: move |e| {
                        let t: u8 = e.value().parse().unwrap_or(0);
                        c.update(move |p| p.panel_opacity = 100u8.saturating_sub(t));
                    },
                }
            }
            Toggle {
                label: "Frosted panel blur",
                desc: "Create glass opacity for transparent frames",
                checked: pr.panel_glass,
                on_change: move |enabled: bool| {
                    c.update(move |p| {
                        p.panel_glass = enabled;
                        if enabled && p.panel_opacity == 100 {
                            p.panel_opacity = 60;
                        }
                    });
                },
            }
            if pr.panel_glass {
                div { class: "settings-row",
                    span { class: "grow",
                        span { class: "settings-label", "Glass blur level: {pr.panel_blur}px" }
                        span { class: "settings-row-desc", "Drag to the right to make the frame more opaque" }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "32",
                        value: "{pr.panel_blur}",
                        style: "--range-pct: {panel_blur_pct}%;",
                        aria_label: "Glass blur level",
                        oninput: move |e| { let v: u8 = e.value().parse().unwrap_or(12); c.update(move |p| p.panel_blur = v) },
                    }
                }
            }
            if !pr.app_wallpaper.is_empty() {
                div { class: "settings-row",
                    span { class: "settings-label", "Wallpaper dim: {pr.app_wallpaper_dim}%" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "80",
                        value: "{pr.app_wallpaper_dim}",
                        style: "--range-pct: {wp_dim_pct}%;",
                        aria_label: "Wallpaper dim",
                        oninput: move |e| { let v: u8 = e.value().parse().unwrap_or(0); c.update(move |p| p.app_wallpaper_dim = v) },
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Wallpaper blur: {pr.app_wallpaper_blur}px" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "24",
                        value: "{pr.app_wallpaper_blur}",
                        style: "--range-pct: {wp_blur_pct}%;",
                        aria_label: "Wallpaper blur",
                        oninput: move |e| { let v: u8 = e.value().parse().unwrap_or(0); c.update(move |p| p.app_wallpaper_blur = v) },
                    }
                }
            }
        }
        div { class: "settings-group",
            button {
                r#type: "button",
                class: "btn-secondary",
                style: "align-self: flex-start;",
                onclick: move |_| {
                    c.update(|p| {
                        let d = Prefs::default();
                        p.custom_accent = d.custom_accent;
                        p.bubble_self = d.bubble_self;
                        p.bubble_other = d.bubble_other;
                        p.bubble_shape = d.bubble_shape;
                        p.avatar_shape = d.avatar_shape;
                        p.chat_width = d.chat_width;
                        p.font_family = d.font_family;
                        p.wallpaper = d.wallpaper;
                        p.app_wallpaper = d.app_wallpaper;
                        p.app_wallpaper_dim = d.app_wallpaper_dim;
                        p.app_wallpaper_blur = d.app_wallpaper_blur;
                        p.panel_opacity = d.panel_opacity;
                        p.panel_glass = d.panel_glass;
                        p.panel_blur = d.panel_blur;
                        p.reduce_motion = d.reduce_motion;
                    });
                    if let Some(h) = on_reset_appearance {
                        h.call(());
                    }
                },
                "Reset default appearance"
            }
        }
    }
}

#[component]
pub fn AdvancedLookSettings(#[props(default)] on_reset_appearance: Option<EventHandler<()>>) -> Element {
    rsx! {
        AppearanceColorsShapes {}
        AppearanceWallpapersGlass { on_reset_appearance: on_reset_appearance }
    }
}

/// Spec 009: settings sync between devices and the avatar-motion budget for lists.
#[component]
pub fn SyncMotionSettings() -> Element {
    let ctx = use_context::<PrefsCtx>();
    let p = ctx.prefs.read().clone();
    let version = crate::services::settings_sync::known_version();
    rsx! {
        div { class: "settings-group",
            Toggle {
                label: "Sync settings across devices",
                desc: "Save your settings to use on another device",
                checked: p.sync_settings,
                on_change: move |v: bool| ctx.update(move |x| x.sync_settings = v),
            }
            if p.sync_settings && version > 0 {
                div { class: "settings-row-desc", "Sync version: {version}" }
            }
            div { class: "settings-row",
                span { class: "grow",
                    span { class: "settings-label", "List avatar animations" }
                    span { class: "settings-row-desc", "Enable motion for avatars in the list" }
                }
                SelectMenu {
                    label: "Avatar effects in the list",
                    options: vec![("always".into(), "Always".into()), ("hover".into(), "On hover".into()), ("off".into(), "Off".into())],
                    value: p.avatar_fx_lists.clone(),
                    on_change: move |v: String| ctx.update(move |x| x.avatar_fx_lists = v.clone()),
                }
            }
        }
    }
}
