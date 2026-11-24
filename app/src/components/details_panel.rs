//! Right-hand conversation details for a direct message: profile summary,
//! quick actions, in-conversation search and a collapsible security section.

use dioxus::prelude::*;

use crate::{
    components::{
        icons::{
            BellIcon, CheckIcon, ChevronDownIcon, ChevronRightIcon, CloseIcon, CopyIcon,
            DevicesIcon, FileTextIcon, ImageIcon, LinkIcon, MessageIcon, MicIcon, PaletteIcon,
            PhoneIcon, PinIcon, SearchIcon, SelfChatPrideAvatar, ShieldIcon, TrashIcon, UserIcon,
            VideoIcon,
        },
        messages::ViewMessage,
        profile_card::DecoratedAvatar,
        rich::{
            parse_rich, rich_preview, ImageData, LightboxMedia, Rich, GLOBAL_LIGHTBOX,
        },
    },
    i18n::t,
    models::{SessionItem, UserProfileResponse},
    platform,
    prefs::PrefsCtx,
    services::api,
    storage::format_bytes_human,
};

fn presence_label(p: &str) -> &'static str {
    match p {
        "online" => "Online",
        "idle" => "Away",
        "dnd" => "Do not disturb",
        "invisible" | "offline" => "Offline",
        _ => "Status unknown",
    }
}

/// Group a base64url fingerprint into blocks of 4 for easier out-of-band comparison.
pub fn format_fingerprint(fp: &str) -> String {
    fp.chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
pub fn DmDetailsPanel(
    user_id: String,
    username: String,
    avatar_url: Option<String>,
    fingerprint: Option<String>,
    my_fingerprint: Option<String>,
    #[props(default)] my_user_id: String,
    #[props(default)] on_key_verified: Option<EventHandler<String>>,
    messages: Vec<ViewMessage>,
    on_close: EventHandler<()>,
    on_focus_composer: EventHandler<()>,
    on_audio_call: EventHandler<()>,
    on_video_call: EventHandler<()>,
    on_view_profile: EventHandler<()>,
    on_jump_to: EventHandler<String>,
) -> Element {
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let mut security_open = use_signal(|| false);
    let mut copied = use_signal(|| false);
    let mut ack_tick = use_signal(|| 0u32);

    let effective_my_uid: Option<String> = if !my_user_id.trim().is_empty() {
        Some(my_user_id.clone())
    } else {
        crate::services::api::auth_me_cached_id()
    };

    let mut loaded_uid = use_signal(String::new);
    let mut auto_expanded_for = use_signal(String::new);
    if *loaded_uid.peek() != user_id {
        let uid = user_id.clone();
        loaded_uid.set(uid.clone());
        profile.set(None);
        security_open.set(false);
        spawn(async move {
            if let Ok(p) = api::get_user_profile(&uid).await {
                if *loaded_uid.peek() == uid {
                    profile.set(Some(p));
                }
            }
        });
    }
    if fingerprint.is_some() && *auto_expanded_for.peek() != user_id {
        let has_peer_alert = effective_my_uid.as_deref() != Some(user_id.as_str())
            && crate::crypto::e2ee::get_peer_key_change_alert(effective_my_uid.as_deref(), &user_id).is_some();
        if has_peer_alert {
            auto_expanded_for.set(user_id.clone());
            security_open.set(true);
        }
    }

    let p = profile();
    let display_name = p
        .as_ref()
        .and_then(|p| p.display_name.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| username.clone());
    let effects_on = use_context::<PrefsCtx>().prefs.read().profile_effects;
    let c1 = p.as_ref().and_then(|p| p.profile_color.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| "var(--accent)".into());
    let c2 = p.as_ref().and_then(|p| p.profile_accent.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| c1.clone());
    let effect = p.as_ref().and_then(|p| p.profile_effect.clone()).unwrap_or_else(|| "none".into());
    let custom_fx_url = p.as_ref().and_then(|p| p.custom_effect_url.clone()).filter(|u| !u.is_empty());
    let custom_fx_anim = p.as_ref().and_then(|p| p.custom_effect_anim.clone()).unwrap_or_else(|| "none".into());
    let decoration = p.as_ref().and_then(|p| p.avatar_decoration.clone());
    let banner_style = p
        .as_ref()
        .and_then(|p| p.custom_banner_url.clone())
        .filter(|u| !u.is_empty())
        .map(|u| format!("background-image: url('{}');", platform::resolve_asset_url(&u)))
        .unwrap_or_else(|| format!("background: linear-gradient(135deg, {c1}, {c2});"));
    let status_line = p.as_ref().map(|p| {
        let custom = match (p.status_emoji.as_deref(), p.status_text.as_deref()) {
            (Some(e), Some(t)) if !t.is_empty() => format!("{e} {t}"),
            (None, Some(t)) if !t.is_empty() => t.to_string(),
            (Some(e), _) if !e.is_empty() => e.to_string(),
            _ => String::new(),
        };
        // Presence is a self-declared preference, not live tracking: only show
        // explicit "do not disturb"/"idle" choices or a custom status.
        if custom.is_empty() {
            match p.presence.as_str() {
                "idle" | "dnd" => presence_label(&p.presence).to_string(),
                _ => String::new(),
            }
        } else {
            custom
        }
    }).filter(|s| !s.is_empty());
    let bio = p.as_ref().and_then(|p| p.bio.clone()).filter(|b| !b.trim().is_empty());

    rsx! {
        button {
            r#type: "button",
            class: "drawer-backdrop",
            aria_label: "Close the dashboard",
            tabindex: "-1",
            onclick: move |_| on_close.call(()),
        }
        aside { class: "details-panel", style: "--pc-1: {c1}; --pc-2: {c2};", aria_label: "Conversation info",
            if effects_on && effect != "none" && effect != "custom" {
                div { class: "profile-effect effect-{effect}", aria_hidden: "true",
                    for n in 0..14 { i { key: "{n}", style: "--i: {n};" } }
                }
            }
            if effects_on && effect == "custom" {
                if let Some(u) = custom_fx_url {
                    img { class: "custom-effect anim-{custom_fx_anim}", src: "{platform::resolve_asset_url(&u)}", alt: "", aria_hidden: "true" }
                }
            }
            button {
                r#type: "button",
                class: "icon-btn profile-close only-mobile",
                title: "Close",
                aria_label: "Close",
                onclick: move |_| on_close.call(()),
                CloseIcon {}
            }
            div { class: "details-scroll",
                div { class: "details-hero",
                    div { class: "details-banner", style: "{banner_style}" }
                    div { class: "details-avatar",
                        DecoratedAvatar { name: username.clone(), avatar_url: avatar_url.clone(), size: 80, decoration: decoration,
                            motion: if effects_on { p.as_ref().and_then(|p| p.avatar_motion.clone()) } else { None } }
                    }
                    div { class: "details-name", "{crate::services::profile_ext::nickname_of(&user_id).unwrap_or_else(|| display_name.clone())}" }
                    if crate::services::profile_ext::nickname_of(&user_id).is_some() {
                        div { class: "details-username", "{display_name}" }
                    }
                    div { class: "details-username", "@{username}" }
                    if let Some(st) = status_line {
                        div { class: "details-status", "{st}" }
                    }
                }
                div { class: "details-actions",
                    button { r#type: "button", class: "details-action", title: "Message", onclick: move |_| on_focus_composer.call(()),
                        MessageIcon {} span { "Message" }
                    }
                    button { r#type: "button", class: "details-action", title: "Voice call", onclick: move |_| on_audio_call.call(()),
                        PhoneIcon {} span { "Call" }
                    }
                    button { r#type: "button", class: "details-action", title: "Video call", onclick: move |_| on_video_call.call(()),
                        VideoIcon {} span { "Video" }
                    }
                    button { r#type: "button", class: "details-action", title: "View profile", onclick: move |_| on_view_profile.call(()),
                        UserIcon {} span { "Profile" }
                    }
                }

                if let Some(b) = bio {
                    div { class: "details-section",
                        div { class: "details-section-title", "Introduction" }
                        div { class: "details-bio", "{b}" }
                    }
                }

                ContactNoteEditor { key: "{user_id}", user_id: user_id.clone() }

                div { class: "details-section",
                    button {
                        r#type: "button",
                        class: "disclosure",
                        aria_expanded: "{security_open()}",
                        onclick: move |_| security_open.set(!security_open()),
                        span { class: "disclosure-icon", ShieldIcon { size: 16 } }
                        span { class: "disclosure-text",
                            span { class: "disclosure-title", "Security and encryption" }
                            span { class: "disclosure-sub",
                                if fingerprint.is_some() { "End-to-end encryption is on" } else { "Loading public key" }
                            }
                        }
                        if security_open() { ChevronDownIcon { size: 16 } } else { ChevronRightIcon { size: 16 } }
                    }
                    if security_open() {
                        div { class: "security-note",
                            "End-to-end encrypted direct messages protocol v2 (ECDH P-256 + HKDF Ratchet + AES-256-GCM + ECDSA P-256 signature). "
                            "Compare the security code below with {username} over the out-of-band channel to detect a key substitution attack."
                        }
                        {
                            let my_uid = effective_my_uid.clone();
                            let _ = ack_tick();
                            let alert_now = if my_uid.as_deref() == Some(user_id.as_str()) {
                                None
                            } else {
                                crate::crypto::e2ee::get_peer_key_change_alert(my_uid.as_deref(), &user_id)
                            };
                            let uid_ack = user_id.clone();
                            let my_uid_ack = my_uid.clone();
                            rsx! {
                                if let Some(alert) = alert_now {
                                    div { class: "status-badge error", style: "margin: 8px 0; display: flex; flex-direction: column; gap: 6px;",
                                        strong { "Identity Key Change Alert (TOFU)" }
                                        span { "{username}'s encryption key has changed since the previous session. Check the security code directly before sending sensitive information." }
                                        span { style: "font-family: monospace; font-size: 11px;", "Old: {format_fingerprint(&alert.previous_identity_fingerprint)}" }
                                        span { style: "font-family: monospace; font-size: 11px;", "New: {format_fingerprint(&alert.new_identity_fingerprint)}" }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            onclick: move |_| {
                                                crate::crypto::e2ee::acknowledge_peer_key_change(my_uid_ack.as_deref(), &uid_ack);
                                                ack_tick.set(ack_tick() + 1);
                                                if let Some(h) = on_key_verified {
                                                    h.call(uid_ack.clone());
                                                }
                                            },
                                            "New key verified"
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(fp) = fingerprint.clone() {
                            div { class: "details-section-title", "{username}'s lock fingerprint" }
                            div { class: "fingerprint", "{format_fingerprint(&fp)}" }
                            {
                                let qr_svg = qrcode::QrCode::new(format!("chatw-e2ee-v2:{user_id}:{fp}").as_bytes())
                                    .ok()
                                    .map(|code| {
                                        code.render::<qrcode::render::svg::Color>()
                                            .min_dimensions(132, 132)
                                            .dark_color(qrcode::render::svg::Color("#111827"))
                                            .light_color(qrcode::render::svg::Color("#ffffff"))
                                            .build()
                                    });
                                rsx! {
                                    if let Some(svg) = qr_svg {
                                        div {
                                            class: "e2ee-verify-qr",
                                            style: "margin: 8px 0; padding: 8px; background: #fff; border-radius: 10px; display: inline-block;",
                                            dangerous_inner_html: "{svg}",
                                        }
                                    }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| {
                                    platform::copy_to_clipboard(&fp);
                                    copied.set(true);
                                },
                                CopyIcon {}
                                if copied() { "Copied" } else { "Copy fingerprint" }
                            }
                        }
                        if let Some(mine) = my_fingerprint.clone() {
                            div { class: "details-section-title", "Fingerprint your lock" }
                            div { class: "fingerprint", "{format_fingerprint(&mine)}" }
                        }
                        div { class: "details-row",
                            span { class: "details-row-label", "User code" }
                        }
                        div { class: "fingerprint", "{user_id}" }
                        PeerRecoveryConvToggle { peer_id: user_id.clone() }
                    }
                }
            }
        }
    }
}

/// Per-conversation toggle allowing two-way missing DM file recovery via WebRTC/relay.
#[component]
fn PeerRecoveryConvToggle(peer_id: String) -> Element {
    let mut allow_rec = use_signal(|| true);
    let mut dm_backup = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let pid = peer_id.clone();
    use_effect(use_reactive!(|pid| {
        spawn(async move {
            if let Ok(pol) = crate::services::storage_sync::get_conv_policy(&pid).await {
                allow_rec.set(pol.allow_peer_recovery);
                dm_backup.set(pol.dm_encrypted_backup);
            }
        });
    }));

    rsx! {
        div { class: "details-recovery-box",
            label { class: "details-toggle-row",
                span { "Allows two-way file recovery" }
                input {
                    r#type: "checkbox",
                    checked: allow_rec(),
                    onchange: {
                        let p = peer_id.clone();
                        move |e: Event<FormData>| {
                            let val = e.checked();
                            allow_rec.set(val);
                            let peer = p.clone();
                            let bkup = dm_backup();
                            spawn(async move {
                                let pol = crate::services::storage_sync::ConvStoragePolicy {
                                    allow_peer_recovery: val,
                                    dm_encrypted_backup: bkup,
                                };
                                match crate::services::storage_sync::update_conv_policy(&peer, &pol).await {
                                    Ok(_) => status_msg.set(Some(if val { "Two-way recovery enabled".into() } else { "Disabled two-way recovery".into() })),
                                    Err(_) => status_msg.set(Some("Settings cannot be saved".into())),
                                }
                            });
                        }
                    },
                }
            }
            if let Some(m) = status_msg() {
                div { class: "form-hint", "{m}" }
            }
        }
    }
}

/// My private nickname and note for this person (only I can see them).
#[component]
fn ContactNoteEditor(user_id: String) -> Element {
    let start = crate::services::profile_ext::CONTACTS.read().get(&user_id).cloned().unwrap_or_default();
    let mut nickname = use_signal(|| start.0.clone().unwrap_or_default());
    let mut note = use_signal(|| start.1.clone().unwrap_or_default());
    let mut open = use_signal(|| false);
    let mut status = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);
    let uid = user_id.clone();
    let save = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let u = uid.clone();
        spawn(async move {
            match crate::services::profile_ext::save_note(&u, &nickname(), &note()).await {
                Ok(()) => status.set(Some((false, "Saved".into()))),
                Err(_e) => status.set(Some((true, "Unable to save".to_string()))),
            }
            busy.set(false);
        });
    };
    rsx! {
        div { class: "details-section",
            button { r#type: "button", class: "details-section-toggle", aria_expanded: "{open()}", onclick: move |_| open.set(!open()),
                span { class: "details-section-title", "Nicknames and private notes" }
                span { class: "member-sub", if open() { "Collapse" } else if nickname().is_empty() && note().is_empty() { "Add" } else { "Edit" } }
            }
            if !open() && !note().trim().is_empty() {
                div { class: "details-bio private-note", "{note}" }
            }
            if open() {
                p { class: "member-sub", "Only you see. Nickname replaces this person's name in your list." }
                input { r#type: "text", maxlength: "40", placeholder: "Nickname", aria_label: "Nickname", value: "{nickname}", oninput: move |e| nickname.set(e.value()) }
                textarea { rows: "3", maxlength: "500", placeholder: "Private notes", aria_label: "Private notes", value: "{note}", oninput: move |e| note.set(e.value()) }
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: save, "Save" }
                    if let Some((err, m)) = status() { span { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
                }
            }
        }
    }
}

// ============================================================================
// Spec 089: Right-Side Personal Vault & Info Sidebar for "Chính mình"
// (Modeled after Telegram Web K "Saved Messages" & Signal "Note to Self")
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfVaultTab {
    Photo,
    Documents,
    Link,
    Pinned,
}

#[derive(Clone, Debug, PartialEq)]
enum SelfMediaEntry {
    Image {
        msg_id: String,
        created_at: String,
        data: ImageData,
    },
    Video {
        msg_id: String,
        created_at: String,
        name: String,
        size: u64,
        dur_ms: u64,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum SelfDocKind {
    File,
    Audio,
    Note,
    Reminder,
}

#[derive(Clone, Debug, PartialEq)]
struct SelfDocEntry {
    msg_id: String,
    created_at: String,
    kind: SelfDocKind,
    title: String,
    subtitle: String,
}

#[derive(Clone, Debug, PartialEq)]
struct SelfLinkEntry {
    msg_id: String,
    created_at: String,
    url: String,
    host: String,
}

/// Extracts clean `http://` and `https://` URLs from a message's text or note/reminder content.
pub fn extract_message_links(text: &str) -> Vec<String> {
    let scan_buf: String;
    let body = match parse_rich(text) {
        Some(Rich::Note(n)) => {
            scan_buf = format!("{} {}", n.t, n.b);
            scan_buf.as_str()
        }
        Some(Rich::Reminder(r)) => {
            scan_buf = r.t;
            scan_buf.as_str()
        }
        Some(_) => return Vec::new(),
        None => text,
    };

    let mut out = Vec::new();
    for raw_tok in body.split_whitespace() {
        let tok = raw_tok.trim_start_matches(|c: char| matches!(c, '(' | '[' | '{' | '<' | '"' | '\''));
        let lower = tok.to_ascii_lowercase();
        if !lower.starts_with("https://") && !lower.starts_with("http://") {
            continue;
        }
        let mut cleaned = tok;
        while let Some(last) = cleaned.chars().last() {
            let strip = match last {
                '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'' | '>' | ']' | '}' => true,
                ')' => cleaned.matches(')').count() > cleaned.matches('(').count(),
                _ => false,
            };
            if strip {
                cleaned = &cleaned[..cleaned.len() - last.len_utf8()];
            } else {
                break;
            }
        }
        let host = extract_link_host(cleaned);
        if (host.contains('.') || host.eq_ignore_ascii_case("localhost"))
            && !out.iter().any(|u| u == cleaned)
        {
            out.push(cleaned.to_string());
        }
    }
    out
}

/// Extracts a clean display host from an HTTP(S) URL.
pub fn extract_link_host(url: &str) -> String {
    let without_scheme = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = host_port.split(':').next().unwrap_or(host_port);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

fn format_duration_short(dur_ms: u64) -> String {
    let total_secs = dur_ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

fn display_message_preview(m: &ViewMessage) -> String {
    match parse_rich(&m.text) {
        Some(r) => rich_preview(&r),
        None => m.text.clone(),
    }
}

#[component]
fn SelfMediaThumb(
    msg_id: String,
    data: ImageData,
    on_jump_to: EventHandler<String>,
) -> Element {
    let init_key = (data.u.clone(), data.k.clone(), data.n.clone());
    let init_cached = platform::get_cached_blob_url_sync(&data.u, &data.k, &data.n);
    let mut src = use_signal(move || init_cached);
    let mut loaded_key = use_signal(move || if src.peek().is_some() { Some(init_key) } else { None });

    let key_now = (data.u.clone(), data.k.clone(), data.n.clone());
    let mime_now = data.m.clone();
    use_effect(use_reactive!(|key_now| {
        if loaded_key.peek().as_ref() == Some(&key_now) && src.peek().is_some() {
            return;
        }
        let (u, k, n) = key_now.clone();
        if let Some(hit) = platform::get_cached_blob_url_sync(&u, &k, &n) {
            loaded_key.set(Some(key_now));
            src.set(Some(hit));
            return;
        }
        let mime = if mime_now.starts_with("image/") {
            mime_now.clone()
        } else {
            "image/png".into()
        };
        let target_key = key_now.clone();
        spawn(async move {
            if let Some(burl) = platform::resolve_e2ee_blob_media_url(&u, &k, &n, &mime, "LOW").await {
                loaded_key.set(Some(target_key));
                src.set(Some(burl));
            }
        });
    }));

    let blur_ok = data.blur.starts_with("data:image/") && data.blur.len() <= 1600;
    let display_name = if data.name.trim().is_empty() {
        "Image".to_string()
    } else {
        data.name.clone()
    };
    let w = data.w;
    let h = data.h;
    let size = data.size;
    let jump_id = msg_id.clone();

    rsx! {
        button {
            r#type: "button",
            class: "self-media-tile",
            title: "{display_name}",
            onclick: move |_| {
                if let Some(url) = src() {
                    *GLOBAL_LIGHTBOX.write() = Some(LightboxMedia {
                        src: url,
                        name: display_name.clone(),
                        width: w,
                        height: h,
                        size,
                        ai_restored: false,
                        original_src: None,
                    });
                } else {
                    on_jump_to.call(jump_id.clone());
                }
            },
            if let Some(url) = src() {
                img { src: "{url}", alt: "{display_name}", loading: "lazy" }
            } else if blur_ok {
                img { class: "self-media-blur", src: "{data.blur}", alt: "" }
            } else {
                span { class: "self-media-placeholder", ImageIcon { size: 20 } }
            }
        }
    }
}

/// Dedicated right-hand panel for "Chính mình" ("Note to self"):
/// Pride mascot hero, 4 quick actions, Telegram/Zalo-style Shared Storage tabs
/// (Photo, Documents, Link, Pinned), live in-chat search, synced devices, and E2EE lock details.
#[component]
pub fn SelfDetailsPanel(
    user_id: String,
    username: String,
    my_fingerprint: Option<String>,
    sessions: Vec<SessionItem>,
    messages: Vec<ViewMessage>,
    on_close: EventHandler<()>,
    on_focus_composer: EventHandler<()>,
    on_open_wallpaper: EventHandler<()>,
    on_clear_self: EventHandler<()>,
    on_rename_device: EventHandler<(String, String)>,
    on_jump_to: EventHandler<String>,
) -> Element {
    let mut active_tab = use_signal(|| SelfVaultTab::Photo);
    let mut security_open = use_signal(|| false);
    let mut copied_fp = use_signal(|| false);
    let mut copied_link = use_signal(|| Option::<String>::None);

    // Partition decrypted self-chat messages into real Photo/Video, Documents/Notes, Links & Pinned vaults.
    let mut media_items: Vec<SelfMediaEntry> = Vec::new();
    let mut doc_items: Vec<SelfDocEntry> = Vec::new();
    let mut link_items: Vec<SelfLinkEntry> = Vec::new();
    let mut pinned_items: Vec<ViewMessage> = Vec::new();

    for m in messages.iter().rev() {
        if m.is_error {
            continue;
        }
        if m.is_pinned {
            pinned_items.push(m.clone());
        }
        match parse_rich(&m.text) {
            Some(Rich::Image(img)) => {
                media_items.push(SelfMediaEntry::Image {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    data: img,
                });
            }
            Some(Rich::Video(v)) => {
                media_items.push(SelfMediaEntry::Video {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    name: if v.name.trim().is_empty() { "Video".into() } else { v.name },
                    size: v.size,
                    dur_ms: v.dur_ms,
                });
            }
            Some(Rich::File(f)) => {
                doc_items.push(SelfDocEntry {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    kind: SelfDocKind::File,
                    title: if f.name.trim().is_empty() { "Documents".into() } else { f.name },
                    subtitle: if f.size > 0 { format_bytes_human(f.size) } else { platform::format_local_time(&m.created_at) },
                });
            }
            Some(Rich::Audio(a)) => {
                doc_items.push(SelfDocEntry {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    kind: SelfDocKind::Audio,
                    title: if a.name.trim().is_empty() { "Voice messages".into() } else { a.name },
                    subtitle: if a.dur_ms > 0 {
                        format_duration_short(a.dur_ms)
                    } else if a.size > 0 {
                        format_bytes_human(a.size)
                    } else {
                        platform::format_local_time(&m.created_at)
                    },
                });
            }
            Some(Rich::Note(n)) => {
                doc_items.push(SelfDocEntry {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    kind: SelfDocKind::Note,
                    title: if n.t.trim().is_empty() { "Notes".into() } else { n.t.clone() },
                    subtitle: if n.b.trim().is_empty() {
                        platform::format_local_time(&m.created_at)
                    } else {
                        n.b.clone()
                    },
                });
                for url in extract_message_links(&m.text) {
                    link_items.push(SelfLinkEntry {
                        msg_id: m.id.clone(),
                        created_at: m.created_at.clone(),
                        host: extract_link_host(&url),
                        url,
                    });
                }
            }
            Some(Rich::Reminder(r)) => {
                doc_items.push(SelfDocEntry {
                    msg_id: m.id.clone(),
                    created_at: m.created_at.clone(),
                    kind: SelfDocKind::Reminder,
                    title: r.t.clone(),
                    subtitle: r.at.clone(),
                });
                for url in extract_message_links(&m.text) {
                    link_items.push(SelfLinkEntry {
                        msg_id: m.id.clone(),
                        created_at: m.created_at.clone(),
                        host: extract_link_host(&url),
                        url,
                    });
                }
            }
            Some(_) => {}
            None => {
                for url in extract_message_links(&m.text) {
                    link_items.push(SelfLinkEntry {
                        msg_id: m.id.clone(),
                        created_at: m.created_at.clone(),
                        host: extract_link_host(&url),
                        url,
                    });
                }
            }
        }
    }

    let media_count = media_items.len();
    let doc_count = doc_items.len();
    let link_count = link_items.len();
    let pin_count = pinned_items.len();
    let self_title = t("chat.self").to_string();
    let _ = username;

    rsx! {
        button {
            r#type: "button",
            class: "drawer-backdrop",
            aria_label: "Close",
            tabindex: "-1",
            onclick: move |_| on_close.call(()),
        }
        aside {
            class: "details-panel self-details-panel",
            "data-testid": "self-details-panel",
            aria_label: "Conversation info",
            button {
                r#type: "button",
                class: "icon-btn profile-close only-mobile",
                title: "Close",
                aria_label: "Close",
                onclick: move |_| on_close.call(()),
                CloseIcon {}
            }
            div { class: "details-scroll",
                // 1. Hero Header
                div { class: "details-hero self-details-hero",
                    div { class: "details-banner self-details-banner", aria_hidden: "true" }
                    div { class: "details-avatar self-details-avatar",
                        span { class: "self-chat-avatar pride-voxel self-details-pride-avatar", aria_hidden: "true",
                            SelfChatPrideAvatar {}
                        }
                    }
                    div { class: "details-name", "{self_title}" }
                    div { class: "details-status self-details-sub",
                        ShieldIcon { size: 13 }
                        span { "Only you" }
                    }
                }

                // 2. 4 Quick Actions (Notes, Search, Theme/Wallpaper, Clear all)
                div { class: "details-actions",
                    button {
                        r#type: "button",
                        class: "details-action",
                        title: "Create note",
                        onclick: move |_| on_focus_composer.call(()),
                        MessageIcon { size: 19 }
                        span { "Notes" }
                    }
                    button {
                        r#type: "button",
                        class: "details-action",
                        title: "Search in chat",
                        onclick: move |_| platform::focus_selector("#self-details-search-input"),
                        SearchIcon { size: 19 }
                        span { "Search" }
                    }
                    button {
                        r#type: "button",
                        class: "details-action",
                        "data-testid": "self-action-wallpaper",
                        title: "Chat wallpaper",
                        onclick: move |_| on_open_wallpaper.call(()),
                        PaletteIcon { size: 19 }
                        span { "Theme" }
                    }
                    button {
                        r#type: "button",
                        class: "details-action danger",
                        "data-testid": "self-action-clear",
                        title: "Delete all",
                        onclick: move |_| on_clear_self.call(()),
                        TrashIcon { size: 19 }
                        span { "Delete all" }
                    }
                }

                // 3. Shared Storage Vault (Photo / Documents / Link / Pinned)
                div { class: "details-section self-vault-section",
                    div { class: "details-section-title", "Storage" }
                    div { class: "self-vault-tabs", role: "tablist", aria_label: "Storage",
                        button {
                            r#type: "button",
                            role: "tab",
                            "data-vault-tab": "photo",
                            aria_selected: "{active_tab() == SelfVaultTab::Photo}",
                            class: if active_tab() == SelfVaultTab::Photo { "self-vault-tab active" } else { "self-vault-tab" },
                            onclick: move |_| active_tab.set(SelfVaultTab::Photo),
                            span { "Photo" }
                            if media_count > 0 { span { class: "self-vault-badge", "{media_count}" } }
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            "data-vault-tab": "documents",
                            aria_selected: "{active_tab() == SelfVaultTab::Documents}",
                            class: if active_tab() == SelfVaultTab::Documents { "self-vault-tab active" } else { "self-vault-tab" },
                            onclick: move |_| active_tab.set(SelfVaultTab::Documents),
                            span { "Documents" }
                            if doc_count > 0 { span { class: "self-vault-badge", "{doc_count}" } }
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            "data-vault-tab": "link",
                            aria_selected: "{active_tab() == SelfVaultTab::Link}",
                            class: if active_tab() == SelfVaultTab::Link { "self-vault-tab active" } else { "self-vault-tab" },
                            onclick: move |_| active_tab.set(SelfVaultTab::Link),
                            span { "Link" }
                            if link_count > 0 { span { class: "self-vault-badge", "{link_count}" } }
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            "data-vault-tab": "pinned",
                            aria_selected: "{active_tab() == SelfVaultTab::Pinned}",
                            class: if active_tab() == SelfVaultTab::Pinned { "self-vault-tab active" } else { "self-vault-tab" },
                            onclick: move |_| active_tab.set(SelfVaultTab::Pinned),
                            span { "Pinned" }
                            if pin_count > 0 { span { class: "self-vault-badge", "{pin_count}" } }
                        }
                    }

                    match active_tab() {
                        SelfVaultTab::Photo => rsx! {
                            if media_items.is_empty() {
                                div { class: "self-vault-empty",
                                    ImageIcon { size: 20 }
                                    span { "No results" }
                                }
                            } else {
                                div { class: "self-media-grid",
                                    for item in media_items.into_iter().take(24) {
                                        match item {
                                            SelfMediaEntry::Image { msg_id, data, .. } => rsx! {
                                                SelfMediaThumb {
                                                    key: "{msg_id}",
                                                    msg_id: msg_id,
                                                    data: data,
                                                    on_jump_to: on_jump_to,
                                                }
                                            },
                                            SelfMediaEntry::Video { msg_id, name, dur_ms, .. } => {
                                                let id_jump = msg_id.clone();
                                                let dur_lbl = if dur_ms > 0 { format_duration_short(dur_ms) } else { String::new() };
                                                rsx! {
                                                    button {
                                                        key: "{msg_id}",
                                                        r#type: "button",
                                                        class: "self-media-tile self-media-video",
                                                        title: "{name}",
                                                        onclick: move |_| on_jump_to.call(id_jump.clone()),
                                                        VideoIcon { size: 20 }
                                                        if !dur_lbl.is_empty() {
                                                            span { class: "self-media-dur", "{dur_lbl}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        SelfVaultTab::Documents => rsx! {
                            if doc_items.is_empty() {
                                div { class: "self-vault-empty",
                                    FileTextIcon { size: 20 }
                                    span { "No results" }
                                }
                            } else {
                                div { class: "self-vault-list",
                                    for doc in doc_items.into_iter().take(30) {
                                        {
                                            let id_jump = doc.msg_id.clone();
                                            let when = platform::format_local_time(&doc.created_at);
                                            rsx! {
                                                button {
                                                    key: "{doc.msg_id}",
                                                    r#type: "button",
                                                    class: "self-vault-row",
                                                    onclick: move |_| on_jump_to.call(id_jump.clone()),
                                                    span { class: "self-vault-row-icon",
                                                        match doc.kind {
                                                            SelfDocKind::Audio => rsx! { MicIcon { size: 16 } },
                                                            SelfDocKind::Reminder => rsx! { BellIcon { size: 16 } },
                                                            _ => rsx! { FileTextIcon { size: 16 } },
                                                        }
                                                    }
                                                    span { class: "self-vault-row-body",
                                                        span { class: "self-vault-item-title", "{doc.title}" }
                                                        span { class: "self-vault-item-sub", "{doc.subtitle} · {when}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        SelfVaultTab::Link => rsx! {
                            if link_items.is_empty() {
                                div { class: "self-vault-empty",
                                    LinkIcon { size: 20 }
                                    span { "No results" }
                                }
                            } else {
                                div { class: "self-vault-list",
                                    for lnk in link_items.into_iter().take(30) {
                                        {
                                            let id_jump = lnk.msg_id.clone();
                                            let url_copy = lnk.url.clone();
                                            let is_copied = copied_link().as_deref() == Some(lnk.url.as_str());
                                            let when = platform::format_local_time(&lnk.created_at);
                                            rsx! {
                                                div { key: "{lnk.msg_id}-{lnk.url}", class: "self-vault-row-wrap",
                                                    button {
                                                        r#type: "button",
                                                        class: "self-vault-row",
                                                        onclick: move |_| on_jump_to.call(id_jump.clone()),
                                                        span { class: "self-vault-row-icon", LinkIcon { size: 16 } }
                                                        span { class: "self-vault-row-body",
                                                            span { class: "self-vault-item-title", "{lnk.host}" }
                                                            span { class: "self-vault-item-sub", "{lnk.url}" }
                                                            span { class: "self-vault-item-meta", "{when}" }
                                                        }
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "icon-btn self-vault-copy-btn",
                                                        title: "Copy link",
                                                        aria_label: "Copy link",
                                                        onclick: move |_| {
                                                            platform::copy_to_clipboard(&url_copy);
                                                            copied_link.set(Some(url_copy.clone()));
                                                        },
                                                        if is_copied { CheckIcon { size: 14 } } else { CopyIcon { size: 14 } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        SelfVaultTab::Pinned => rsx! {
                            if pinned_items.is_empty() {
                                div { class: "self-vault-empty",
                                    PinIcon { size: 20 }
                                    span { "No results" }
                                }
                            } else {
                                div { class: "self-vault-list",
                                    for pm in pinned_items.into_iter().take(30) {
                                        {
                                            let id_jump = pm.id.clone();
                                            let preview = display_message_preview(&pm);
                                            let when = platform::format_local_time(&pm.created_at);
                                            rsx! {
                                                button {
                                                    key: "{pm.id}",
                                                    r#type: "button",
                                                    class: "self-vault-row",
                                                    onclick: move |_| on_jump_to.call(id_jump.clone()),
                                                    span { class: "self-vault-row-icon", PinIcon { size: 16 } }
                                                    span { class: "self-vault-row-body",
                                                        span { class: "self-vault-item-title", "{preview}" }
                                                        span { class: "self-vault-item-sub", "{when}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }

                // 4. Synced Devices ("Your device")
                if !sessions.is_empty() {
                    div { class: "details-section",
                        div { class: "details-section-title", "Your device" }
                        div { class: "self-device-list",
                            for sess in sessions {
                                {
                                    let label = sess
                                        .device_label
                                        .clone()
                                        .or(sess.device_name.clone())
                                        .unwrap_or_else(|| "Devices".into());
                                    let sid = sess.id.clone();
                                    let label_click = label.clone();
                                    rsx! {
                                        button {
                                            key: "{sess.id}",
                                            r#type: "button",
                                            class: if sess.is_current { "self-device-row current" } else { "self-device-row" },
                                            title: "Click to rename the device",
                                            onclick: move |_| on_rename_device.call((sid.clone(), label_click.clone())),
                                            span { class: "self-vault-row-icon", DevicesIcon { size: 15 } }
                                            span { class: "self-device-name", "{label}" }
                                            if sess.is_current {
                                                span { class: "self-device-badge", "this device" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 6. Security & Personal E2EE Lock Disclosure
                div { class: "details-section",
                    button {
                        r#type: "button",
                        class: "disclosure",
                        aria_expanded: "{security_open()}",
                        onclick: move |_| security_open.set(!security_open()),
                        span { class: "disclosure-icon", ShieldIcon { size: 16 } }
                        span { class: "disclosure-text",
                            span { class: "disclosure-title", "Security and encryption" }
                            span { class: "disclosure-sub", "End-to-end encryption is on" }
                        }
                        if security_open() { ChevronDownIcon { size: 16 } } else { ChevronRightIcon { size: 16 } }
                    }
                    if security_open() {
                        div { class: "security-note", "Direct messages are end-to-end encrypted" }
                        if let Some(mine) = my_fingerprint.clone() {
                            div { class: "details-section-title", "Fingerprint your lock" }
                            div { class: "fingerprint", "{format_fingerprint(&mine)}" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| {
                                    platform::copy_to_clipboard(&mine);
                                    copied_fp.set(true);
                                },
                                CopyIcon {}
                                if copied_fp() { "Copied" } else { "Copy fingerprint" }
                            }
                        }
                        div { class: "details-row",
                            span { class: "details-row-label", "User code" }
                        }
                        div { class: "fingerprint", "{user_id}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod spec089_self_vault_tests {
    use super::{extract_link_host, extract_message_links};

    #[test]
    fn extracts_clean_http_links_without_trailing_punctuation() {
        let links = extract_message_links(
            "Check https://github.com/kiyogo/chart, and (https://en.wikipedia.org/wiki/Rust_(programming_language))!",
        );
        assert_eq!(
            links,
            vec![
                "https://github.com/kiyogo/chart".to_string(),
                "https://en.wikipedia.org/wiki/Rust_(programming_language)".to_string(),
            ]
        );
        assert_eq!(extract_link_host("https://www.github.com/kiyogo/chart"), "github.com");
    }

    #[test]
    fn extracts_links_inside_rich_notes_and_skips_binary_rich_payloads() {
        let note_links = extract_message_links(r#"note:{"t":"Bookmarks","b":"See https://chatw.app/docs."}"#);
        assert_eq!(note_links, vec!["https://chatw.app/docs".to_string()]);
        assert!(extract_message_links(r#"img:{"u":"/uploads/media/a.png","name":"pic"}"#).is_empty());
    }
}
