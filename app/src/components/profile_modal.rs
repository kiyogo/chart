//! Full profile view and the profile editor (with live preview).

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        icons::{CloseIcon, MessageIcon},
        draw_studio::DrawStudio,
        profile_card::{
            DECORATIONS, DecoratedAvatar, EFFECTS, EFFECT_ANIMS, NAME_EFFECTS, NAMEPLATES, ProfileCard,
            THEME_PRESETS, name_effect_class, nameplate_style,
        },
    },
    models::UserProfileResponse,
    platform,
    services::{
        api::{get_user_profile, update_my_profile_fields, users_upload_avatar, users_upload_banner},
        library,
    },
};

#[derive(Clone, Copy, PartialEq)]
enum EditTab {
    Info,
    Motion,
    Layout,
    Style,
    Effects,
    Create,
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
    let mut studio_target = use_signal(|| Option::<String>::None);
    // Spec 009: minutes until the custom status clears (-1 = keep the current timer).
    let mut status_expiry = use_signal(|| -1i64);
    let mut interests_text = use_signal(String::new);
    // Spec 010: the full ChatW Studio (layers, animation) for a target.
    let mut full_studio = use_signal(|| Option::<String>::None);
    // Spec 012: layout / template / showcase / guestbook, plus the draft layout while editing.
    let mut plus = use_signal(|| Option::<crate::services::profile_plus::ProfilePlus>::None);
    let mut draft_layout = use_signal(|| Option::<(String, Vec<crate::services::profile_plus::Section>, String)>::None);
    let mut view_as = use_signal(|| "self".to_string());
    let mut preview_mobile = use_signal(|| false);
    let uid_plus = user_id.clone();
    let reload_plus = use_callback(move |_: ()| {
        let u = uid_plus.clone();
        spawn(async move {
            if let Ok(x) = crate::services::profile_plus::get(&u).await {
                plus.set(Some(x));
            }
        });
    });
    use_hook(move || reload_plus.call(()));

    let uid = user_id.clone();
    use_effect(move || {
        let u = uid.clone();
        spawn(async move {
            match get_user_profile(&u).await {
                Ok(p) => {
                    if start_editing && is_self {
                        interests_text.set(p.details.interests.join(", "));
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
                "avatar_effect": d.avatar_effect.clone().unwrap_or_else(|| "none".into()),
                "name_effect": d.name_effect.clone().unwrap_or_else(|| "none".into()),
                "nameplate": d.nameplate.clone().unwrap_or_else(|| "none".into()),
                "custom_decoration_url": d.custom_decoration_url.clone().unwrap_or_default(),
                "custom_nameplate_url": d.custom_nameplate_url.clone().unwrap_or_default(),
                "custom_effect_url": d.custom_effect_url.clone().unwrap_or_default(),
                "custom_effect_anim": d.custom_effect_anim.clone().unwrap_or_else(|| "none".into()),
            });
            let interests: Vec<String> = interests_text().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).take(12).collect();
            let mut ext = json!({
                "avatar_motion": d.avatar_motion.clone(),
                "pronouns": d.details.pronouns.clone().unwrap_or_default(),
                "location": d.details.location.clone().unwrap_or_default(),
                "website": d.details.website.clone().unwrap_or_default(),
                "occupation": d.details.occupation.clone().unwrap_or_default(),
                "interests": interests,
                "detail_visibility": d.detail_visibility.clone().unwrap_or_default(),
            });
            if status_expiry() >= 0 {
                ext["status_expires_in_minutes"] = json!(if d.status_text.as_deref().unwrap_or_default().trim().is_empty() { 0 } else { status_expiry() });
            }
            if let Some((t, sec, g)) = draft_layout() {
                if let Err(_e) = crate::services::profile_plus::save_layout(&t, &sec, &g).await {
                    notice.set(Some((true, "Unable to save layout".to_string())));
                }
            }
            let result = match update_my_profile_fields(fields).await {
                Ok(mut updated) => match crate::services::profile_ext::update_me(ext).await {
                    Ok(v) => {
                        crate::services::profile_ext::merge_into(&mut updated, &v);
                        Ok(updated)
                    }
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            };
            match result {
                Ok(updated) => {
                    status_expiry.set(-1);
                    draft_layout.set(None);
                    reload_plus.call(());
                    profile.set(Some(updated.clone()));
                    draft.set(None);
                    if let Some(h) = on_profile_updated {
                        h.call(updated);
                    }
                }
                Err(e) => notice.set(Some((true, match e.code.as_str() {
                    "invalid_website" => "Website must start with https:// or http://".to_string(),
                    "too_many_interests" => "Up to 12 preferences".to_string(),
                    _ => "Unable to save profile".to_string(),
                }))),
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
            aria_label: "User profile",
            onclick: move |_| on_close.call(()),
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            div { class: "{dialog_cls}", onclick: move |evt| evt.stop_propagation(),
                if load_error() {
                    div { class: "modal-body",
                        div { class: "status-badge error", "Unable to download profile" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Close" }
                    }
                } else if let Some(d) = draft() {
                    div { class: "profile-editor",
                        div { class: "profile-editor-form",
                            div { class: "modal-header",
                                h2 { class: "modal-title", "Edit profile" }
                                button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
                            }
                            div { class: "segmented", style: "margin: 0 var(--space-5);", role: "tablist",
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Info { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Info), "Information" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Motion { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Motion), "Avatar Studio" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Layout { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Layout), "Layout" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Style { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Style), "Decoration" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Effects { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Effects), "Effects" }
                                button { r#type: "button", role: "tab", class: if tab() == EditTab::Create { "active" } else { "" }, onclick: move |_| tab.set(EditTab::Create), "Self-created" }
                            }
                            div { class: "modal-body",
                                if tab() == EditTab::Info {
                                    div { class: "form-field",
                                        span { class: "form-label", "Photo" }
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
                                                                        notice.set(Some((false, "Updated profile photo".into())));
                                                                    }
                                                                    Err(_) => notice.set(Some((true, "Unable to load profile picture".into()))),
                                                                },
                                                                Err(e) if e == "no_file" => {}
                                                                Err(e) if e == "too_large" => notice.set(Some((true, "Animated cover photos must be less than 8 MB".into()))),
                                                                Err(_) => notice.set(Some((true, "Unable to process images".into()))),
                                                            }
                                                        });
                                                    },
                                                }
                                                "Change avatar"
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
                                                            // GIF / WebP covers are uploaded untouched so they keep their animation.
                                                            let animated = platform::read_animated_image("profile-banner-input", 8 * 1024 * 1024).await;
                                                            let processed = match animated {
                                                                Ok(Some(bytes)) => Ok(bytes),
                                                                Ok(None) => platform::process_banner_from_input("profile-banner-input").await,
                                                                Err(e) => Err(e),
                                                            };
                                                            match processed {
                                                                Ok(bytes) => match users_upload_banner(bytes).await {
                                                                    Ok(url) => {
                                                                        draft.with_mut(|d| if let Some(p) = d { p.custom_banner_url = Some(url.clone()); });
                                                                        profile.with_mut(|d| if let Some(p) = d { p.custom_banner_url = Some(url.clone()); });
                                                                        notice.set(Some((false, "Updated cover photo".into())));
                                                                    }
                                                                    Err(_) => notice.set(Some((true, "Unable to upload banner image".into()))),
                                                                },
                                                                Err(e) if e == "no_file" => {}
                                                                Err(_) => notice.set(Some((true, "Unable to process images".into()))),
                                                            }
                                                        });
                                                    },
                                                }
                                                "Change banner"
                                            }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Display name" }
                                        input {
                                            maxlength: "64",
                                            placeholder: "{d.username}",
                                            value: "{d.display_name.clone().unwrap_or_default()}",
                                            oninput: move |e| { let v = e.value(); edit(&|p| p.display_name = Some(v.clone())); },
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Custom status" }
                                        div { style: "display: flex; gap: var(--space-2);",
                                            input {
                                                style: "width: 4rem; text-align: center;",
                                                maxlength: "8",
                                                aria_label: "Status emojis",
                                                placeholder: "😊",
                                                value: "{d.status_emoji.clone().unwrap_or_default()}",
                                                oninput: move |e| { let v = e.value(); edit(&|p| p.status_emoji = Some(v.clone())); },
                                            }
                                            input {
                                                maxlength: "128",
                                                aria_label: "Status content",
                                                placeholder: "What are you doing?",
                                                value: "{d.status_text.clone().unwrap_or_default()}",
                                                oninput: move |e| { let v = e.value(); edit(&|p| p.status_text = Some(v.clone())); },
                                            }
                                        }
                                        div { class: "inline-form", style: "margin-top: var(--space-2);",
                                            span { class: "member-sub", "Manually delete the following status" }
                                            select { aria_label: "Manually delete the following status",
                                                onchange: move |e| status_expiry.set(e.value().parse().unwrap_or(-1)),
                                                option { value: "-1", selected: status_expiry() == -1, if d.status_expires_at.is_some() { "Keep the current timer" } else { "Do not delete yourself" } }
                                                option { value: "0", selected: status_expiry() == 0, "Never" }
                                                option { value: "30", selected: status_expiry() == 30, "30 minutes" }
                                                option { value: "60", selected: status_expiry() == 60, "1 hour" }
                                                option { value: "240", selected: status_expiry() == 240, "4 hours" }
                                                option { value: "{platform::minutes_until_midnight()}", "Ends today" }
                                                option { value: "10080", selected: status_expiry() == 10080, "7 days" }
                                            }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Mode" }
                                        select {
                                            value: "{d.presence}",
                                            onchange: move |e| { let v = e.value(); edit(&|p| p.presence = v.clone()); },
                                            option { value: "online", "Ready" }
                                            option { value: "idle", "Away" }
                                            option { value: "dnd", "Do not disturb" }
                                            option { value: "offline", "Hide" }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { class: "form-label", "Introduction" }
                                        textarea {
                                            rows: "4",
                                            maxlength: "256",
                                            placeholder: "A few lines about you",
                                            value: "{d.bio.clone().unwrap_or_default()}",
                                            oninput: move |e| { let v = e.value(); edit(&|p| p.bio = Some(v.clone())); },
                                        }
                                    }
                                    div { class: "settings-group-title", "More information" }
                                    p { class: "member-sub", "Each item has its own viewing permissions. The server only sends the item to people you allow." }
                                    for (key, label, ph, max) in [("pronouns", "Title", "she/her, he/she", 40), ("occupation", "Work", "Design, student", 64), ("location", "Place to live", "Hanoi", 64), ("website", "Website", "https://", 200)] {
                                        {
                                            let cur = match key { "pronouns" => d.details.pronouns.clone(), "occupation" => d.details.occupation.clone(), "location" => d.details.location.clone(), _ => d.details.website.clone() }.unwrap_or_default();
                                            let vis = d.detail_visibility.as_ref().and_then(|m| m.get(key).cloned()).unwrap_or_else(|| if key == "location" { "friends".into() } else { "everyone".into() });
                                            rsx! {
                                                div { key: "{key}", class: "detail-row",
                                                    label { class: "form-field grow",
                                                        span { class: "form-label", "{label}" }
                                                        input { maxlength: "{max}", placeholder: "{ph}", value: "{cur}",
                                                            oninput: move |e| { let v = e.value(); edit(&|p| { let t = Some(v.clone()); match key { "pronouns" => p.details.pronouns = t, "occupation" => p.details.occupation = t, "location" => p.details.location = t, _ => p.details.website = t } }); },
                                                        }
                                                    }
                                                    VisibilitySelect { field: key.to_string(), value: vis, on_change: move |v: String| edit(&|p| { p.detail_visibility.get_or_insert_with(Default::default).insert(key.to_string(), v.clone()); }) }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "detail-row",
                                        label { class: "form-field grow",
                                            span { class: "form-label", "Hobbies" }
                                            input { maxlength: "400", placeholder: "Music, Anime, Coding", value: "{interests_text}", oninput: move |e| interests_text.set(e.value()) }
                                        }
                                        VisibilitySelect { field: "interests".to_string(), value: d.detail_visibility.as_ref().and_then(|m| m.get("interests").cloned()).unwrap_or_else(|| "everyone".into()),
                                            on_change: move |v: String| edit(&|p| { p.detail_visibility.get_or_insert_with(Default::default).insert("interests".to_string(), v.clone()); }) }
                                    }
                                } else if tab() == EditTab::Layout {
                                    {
                                        let cur = draft_layout().or_else(|| plus().map(|x| (x.template, x.sections, x.guestbook_mode))).unwrap_or_else(|| ("minimal".into(), crate::services::profile_plus::default_sections(), "friends".into()));
                                        rsx! {
                                            crate::components::profile_plus_ui::LayoutEditor {
                                                template: cur.0, sections: cur.1, guestbook_mode: cur.2,
                                                showcase: plus().map(|x| x.showcase).unwrap_or_default(),
                                                on_change: move |v| draft_layout.set(Some(v)),
                                                on_showcase_changed: move |_| reload_plus.call(()),
                                            }
                                        }
                                    }
                                } else if tab() == EditTab::Motion {
                                    p { class: "member-sub", "Choose one of 40 effects, adjust speed, intensity, color, rotation direction and activation method. The effect appears on people's profiles, chat lists, and friends lists." }
                                    crate::components::avatar_motion::AvatarStudio {
                                        name: d.username.clone(),
                                        avatar_url: d.avatar_url.clone(),
                                        value: d.avatar_motion.clone(),
                                        on_change: move |m: Option<crate::components::avatar_motion::AvatarMotion>| edit(&|p| p.avatar_motion = m.clone()),
                                    }
                                } else if tab() == EditTab::Effects {
                                    div { class: "form-field",
                                        span { class: "form-label", "Avatar effect" }
                                        div { class: "inline-form",
                                            span { class: "member-sub", "40 effects with adjustable speed, color and activation are in Avatar Studio." }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| tab.set(EditTab::Motion), "Open Avatar Studio" }
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Name effect" }
                                        div { class: "option-tiles",
                                            for (val, label) in NAME_EFFECTS {
                                                button {
                                                    key: "{val}",
                                                    r#type: "button",
                                                    class: if d.name_effect.as_deref().unwrap_or("none") == val { "option-tile active" } else { "option-tile" },
                                                    onclick: move |_| edit(&|p| p.name_effect = Some(val.into())),
                                                    span { class: "name-sample {name_effect_class(Some(val))}", "{d.display_name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| d.username.clone())}" }
                                                    span { "{label}" }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Name plate" }
                                        span { class: "form-hint", "Shows up after your name in your friends' chat lists and on your profile." }
                                        div { class: "option-tiles wide",
                                            for (val, label) in NAMEPLATES {
                                                {
                                                    let (cls, style) = nameplate_style(Some(val), None);
                                                    rsx! {
                                                        button {
                                                            key: "{val}",
                                                            r#type: "button",
                                                            class: if d.nameplate.as_deref().unwrap_or("none") == val { "option-tile active" } else { "option-tile" },
                                                            onclick: move |_| edit(&|p| p.nameplate = Some(val.into())),
                                                            span { class: "nameplate-sample {cls}", style: "{style}", "{d.username}" }
                                                            span { "{label}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if tab() == EditTab::Create {
                                    if let Some(t) = studio_target() {
                                        DrawStudio {
                                            key: "{t}",
                                            target: t.clone(),
                                            on_close: move |_| studio_target.set(None),
                                            on_saved: { let t = t.clone(); move |url: String| {
                                                let u = url.clone();
                                                match t.as_str() {
                                                    "nameplate" => edit(&|p| { p.nameplate = Some("custom".into()); p.custom_nameplate_url = Some(u.clone()); }),
                                                    "effect" => edit(&|p| { p.profile_effect = Some("custom".into()); p.custom_effect_url = Some(u.clone()); if p.custom_effect_anim.as_deref().unwrap_or("none") == "none" { p.custom_effect_anim = Some("float".into()); } }),
                                                    _ => edit(&|p| { p.avatar_decoration = Some("custom".into()); p.custom_decoration_url = Some(u.clone()); }),
                                                }
                                                studio_target.set(None);
                                                notice.set(Some((false, "Drawing saved. Click \"Save changes\" to apply.".into())));
                                            }},
                                        }
                                    } else {
                                        div { class: "form-hint", "Upload your photo up to 2 MB or draw right in ChatW." }
                                        div { class: "inline-form",
                                            span { class: "member-sub", "Need multiple layers or animations? ChatW Studio exports animated GIFs and applies them directly to your profile." }
                                            for (t, l) in [("decoration", "Studio: decoration"), ("nameplate", "Studio: nameplate"), ("effect", "Studio: effects")] {
                                                button { key: "{t}", r#type: "button", class: "btn-secondary", onclick: move |_| full_studio.set(Some(t.to_string())), "{l}" }
                                            }
                                        }
                                        if let Some(t) = full_studio() {
                                            crate::components::studio::Studio {
                                                my_id: user_id.clone(),
                                                target: t,
                                                on_close: move |_| full_studio.set(None),
                                                on_used: move |(t, u): (String, String)| {
                                                    match t.as_str() {
                                                        "nameplate" => edit(&|p| { p.nameplate = Some("custom".into()); p.custom_nameplate_url = Some(u.clone()); }),
                                                        "effect" => edit(&|p| { p.profile_effect = Some("custom".into()); p.custom_effect_url = Some(u.clone()); }),
                                                        "decoration" => edit(&|p| { p.avatar_decoration = Some("custom".into()); p.custom_decoration_url = Some(u.clone()); }),
                                                        _ => {}
                                                    }
                                                },
                                            }
                                        }
                                        for (target, title, current) in [
                                            ("decoration", "Representative photo frame", d.custom_decoration_url.clone()),
                                            ("nameplate", "Name plate", d.custom_nameplate_url.clone()),
                                            ("effect", "Profile effects", d.custom_effect_url.clone()),
                                        ] {
                                            div { key: "{target}", class: "create-card",
                                                div { class: "create-preview create-{target}",
                                                    if let Some(u) = current.as_ref().filter(|u| !u.is_empty()) {
                                                        img { src: "{platform::resolve_asset_url(u)}", alt: "{title}" }
                                                    } else {
                                                        span { class: "muted", "Not yet" }
                                                    }
                                                }
                                                div { class: "create-body",
                                                    div { class: "settings-label", "{title}" }
                                                    div { class: "create-actions",
                                                        label { class: "btn-avatar-upload",
                                                            input {
                                                                id: "custom-{target}-input",
                                                                r#type: "file",
                                                                accept: "image/png,image/gif,image/webp,image/avif,image/jpeg",
                                                                style: "display: none;",
                                                                onchange: move |_| {
                                                                    notice.set(None);
                                                                    spawn(async move {
                                                                        let res: Result<String, String> = async {
                                                                            let (bytes, mime) = platform::read_selected_file_raw(&format!("custom-{target}-input")).await?;
                                                                            library::upload_media(bytes, &mime).await.map_err(|_e| "Upload failed".to_string())
                                                                        }.await;
                                                                        match res {
                                                                            Ok(u) => {
                                                                                match target {
                                                                                    "nameplate" => edit(&|p| { p.nameplate = Some("custom".into()); p.custom_nameplate_url = Some(u.clone()); }),
                                                                                    "effect" => edit(&|p| { p.profile_effect = Some("custom".into()); p.custom_effect_url = Some(u.clone()); }),
                                                                                    _ => edit(&|p| { p.avatar_decoration = Some("custom".into()); p.custom_decoration_url = Some(u.clone()); }),
                                                                                }
                                                                                notice.set(Some((false, "Uploaded. Click \"Save changes\" to apply.".into())));
                                                                            }
                                                                            Err(e) if e == "no_file" => {}
                                                                            Err(e) if e == "too_large" => notice.set(Some((true, "File larger than 2 MB".into()))),
                                                                            Err(e) => notice.set(Some((true, e))),
                                                                        }
                                                                    });
                                                                },
                                                            }
                                                            "Upload image"
                                                        }
                                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| studio_target.set(Some(target.to_string())), "Drawing in app" }
                                                        if current.as_ref().is_some_and(|u| !u.is_empty()) {
                                                            button { r#type: "button", class: "btn-ghost",
                                                                onclick: move |_| match target {
                                                                    "nameplate" => edit(&|p| { p.custom_nameplate_url = Some(String::new()); if p.nameplate.as_deref() == Some("custom") { p.nameplate = Some("none".into()); } }),
                                                                    "effect" => edit(&|p| { p.custom_effect_url = Some(String::new()); if p.profile_effect.as_deref() == Some("custom") { p.profile_effect = Some("none".into()); } }),
                                                                    _ => edit(&|p| { p.custom_decoration_url = Some(String::new()); if p.avatar_decoration.as_deref() == Some("custom") { p.avatar_decoration = Some("none".into()); } }),
                                                                },
                                                                "Stop using"
                                                            }
                                                        }
                                                    }
                                                    if target == "effect" {
                                                        label { class: "form-field",
                                                            span { class: "form-label", "Movement" }
                                                            select {
                                                                value: "{d.custom_effect_anim.clone().unwrap_or_else(|| \"none\".into())}",
                                                                onchange: move |e| { let v = e.value(); edit(&|p| p.custom_effect_anim = Some(v.clone())); },
                                                                for (val, label) in EFFECT_ANIMS { option { key: "{val}", value: "{val}", "{label}" } }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "form-field",
                                        span { class: "form-label", "Profile theme color" }
                                        div { class: "theme-swatches",
                                            for (c1, c2, label) in THEME_PRESETS {
                                                button {
                                                    key: "{label}",
                                                    r#type: "button",
                                                    title: "{label}",
                                                    aria_label: "Topic {label}",
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
                                                "Main color"
                                            }
                                            label { class: "color-field",
                                                input {
                                                    r#type: "color",
                                                    value: "{d.profile_accent.clone().unwrap_or_else(|| \"#7C3AED\".into())}",
                                                    oninput: move |e| { let v = e.value(); edit(&|p| p.profile_accent = Some(v.clone())); },
                                                }
                                                "Secondary color"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: move |_| edit(&|p| { p.profile_color = None; p.profile_accent = None; }),
                                                "Use default colors"
                                            }
                                        }
                                    }
                                    div { class: "form-field",
                                        span { class: "form-label", "Representative photo frame" }
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
                                        span { class: "form-label", "Profile effects" }
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
                                        span { class: "form-hint", "The effect turns off automatically when the operating system turns on motion reduction. Viewer can be turned off in Settings." }
                                    }
                                }
                                if let Some((is_err, msg)) = notice() {
                                    div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
                                }
                            }
                            div { class: "modal-footer",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| draft.set(None), "Cancel" }
                                button { r#type: "button", class: "btn-primary", disabled: saving(), onclick: save,
                                    if saving() { span { class: "spinner" } } else { "Save changes" }
                                }
                            }
                        }
                        div { class: if preview_mobile() { "profile-editor-preview mobile" } else { "profile-editor-preview" },
                            div { class: "preview-toolbar",
                                span { class: "form-label", "Preview" }
                                select { aria_label: "View profile as", value: "{view_as}", onchange: move |e| view_as.set(e.value()),
                                    option { value: "self", "As account holder" }
                                    option { value: "friend", "Like friends" }
                                    option { value: "stranger", "Like a stranger" }
                                }
                                button { r#type: "button", class: "btn-small-secondary", aria_pressed: "{preview_mobile()}", onclick: move |_| preview_mobile.set(!preview_mobile()),
                                    if preview_mobile() { "Desktop" } else { "Mobile" } }
                            }
                            {
                                let pl = plus().map(|mut x| { if let Some((t, sec, g)) = draft_layout() { x.template = t; x.sections = sec; x.guestbook_mode = g; } x });
                                rsx! { ProfileCard { profile: d.clone(), effects_enabled: effects_enabled, plus: pl, is_self: true, view_as: view_as() } }
                            }
                        }
                    }
                } else if let Some(p) = profile() {
                    div { class: "profile-modal-view",
                        button { r#type: "button", class: "icon-btn profile-close", aria_label: "Close profile", onclick: move |_| on_close.call(()), CloseIcon {} }
                        ProfileCard { profile: p.clone(), effects_enabled: effects_enabled, plus: plus(), is_self: is_self, on_plus_changed: move |_| reload_plus.call(()),
                            div { class: "profile-card-actions",
                                if is_self {
                                    button { r#type: "button", class: "btn-primary btn-block", onclick: move |_| draft.set(profile()), "Edit profile" }
                                } else if let Some(h) = on_message {
                                    button {
                                        r#type: "button",
                                        class: "btn-primary btn-block",
                                        onclick: { let id = p.id.clone(); move |_| h.call(id.clone()) },
                                        MessageIcon {}
                                        span { "Message" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "modal-body", div { class: "empty-chat-inner", "Loading profile" } }
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
        button { r#type: "button", class: "menu-dismiss", aria_label: "Close profile", tabindex: "-1", onclick: move |_| on_close.call(()) }
        div {
            class: "profile-popover",
            tabindex: "-1",
            role: "dialog",
            aria_label: "Profile",
            style: "--pop-x: {left}px; --pop-y: {top}px;",
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            if let Some(p) = profile() {
                ProfileCard { profile: p.clone(), compact: true, effects_enabled: effects_enabled,
                    div { class: "profile-card-actions",
                        if !is_self {
                            if let Some(h) = on_message {
                                button { r#type: "button", class: "btn-primary", onclick: { let id = p.id.clone(); move |_| h.call(id.clone()) }, MessageIcon {} span { "Message" } }
                            }
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: { let id = p.id.clone(); move |_| on_view_full.call(id.clone()) }, "See full profile" }
                    }
                }
            } else {
                div { class: "empty-chat-inner", "Loading profile" }
            }
        }
    }
}

/// Who may see one profile detail (enforced by the server).
#[component]
fn VisibilitySelect(field: String, value: String, on_change: EventHandler<String>) -> Element {
    rsx! {
        select { class: "vis-select", aria_label: "Who can see {field}", value: "{value}", onchange: move |e| on_change.call(e.value()),
            option { value: "everyone", selected: value == "everyone", "Everyone" }
            option { value: "friends", selected: value == "friends", "Friends" }
            option { value: "nobody", selected: value == "nobody", "Only me" }
        }
    }
}
