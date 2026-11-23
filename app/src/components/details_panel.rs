//! Right-hand conversation details for a direct message: profile summary,
//! quick actions, in-conversation search and a collapsible security section.

use dioxus::prelude::*;

use crate::{
    components::{
        icons::{
            ChevronDownIcon, ChevronRightIcon, CloseIcon, CopyIcon, MessageIcon, PhoneIcon,
            SearchIcon, ShieldIcon, UserIcon, VideoIcon,
        },
        messages::ViewMessage,
        profile_card::DecoratedAvatar,
    },
    prefs::PrefsCtx,
    models::UserProfileResponse,
    platform,
    services::api,
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
    let mut search = use_signal(String::new);
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

    let q = search().trim().to_lowercase();
    let hits: Vec<ViewMessage> = if q.is_empty() {
        Vec::new()
    } else {
        messages
            .iter()
            .rev()
            .filter(|m| !m.is_error && m.text.to_lowercase().contains(&q))
            .take(50)
            .cloned()
            .collect()
    };

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
                    div { class: "details-section-title", "Search in chat" }
                    div { class: "search-field",
                        SearchIcon { size: 16 }
                        input {
                            r#type: "search",
                            placeholder: "Find downloaded messages",
                            aria_label: "Find messages in chats",
                            value: "{search}",
                            oninput: move |e| search.set(e.value()),
                        }
                    }
                    if !q.is_empty() {
                        if hits.is_empty() {
                            div { class: "sidebar-empty", "There are no matching messages" }
                        } else {
                            div { class: "details-search-results",
                                for h in hits {
                                    {
                                        let id = h.id.clone();
                                        let who = if h.is_self { "You".to_string() } else { h.sender_name.clone() };
                                        let when = platform::format_local_time(&h.created_at);
                                        rsx! {
                                            button {
                                                key: "{h.id}",
                                                r#type: "button",
                                                class: "details-search-hit",
                                                onclick: move |_| on_jump_to.call(id.clone()),
                                                span { class: "details-search-hit-meta", "{who} · {when}" }
                                                span { class: "details-search-hit-text", "{h.text}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "form-hint", "Search runs on the device, in the decoded messages of this session." }
                    }
                }

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
