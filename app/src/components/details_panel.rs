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
        "online" => "Đang hoạt động",
        "idle" => "Đang vắng",
        "dnd" => "Không làm phiền",
        "invisible" | "offline" => "Ngoại tuyến",
        _ => "Không rõ trạng thái",
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

    let uid = user_id.clone();
    use_effect(use_reactive!(|uid| {
        profile.set(None);
        spawn(async move {
            if let Ok(p) = api::get_user_profile(&uid).await {
                profile.set(Some(p));
            }
        });
    }));

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
            aria_label: "Đóng bảng thông tin",
            tabindex: "-1",
            onclick: move |_| on_close.call(()),
        }
        aside { class: "details-panel", aria_label: "Thông tin cuộc trò chuyện",
            header { class: "details-header",
                h3 { class: "details-title", "Thông tin" }
                button {
                    r#type: "button",
                    class: "icon-btn",
                    title: "Đóng",
                    aria_label: "Đóng",
                    onclick: move |_| on_close.call(()),
                    CloseIcon {}
                }
            }
            div { class: "details-scroll",
                div { class: "details-hero",
                    div { class: "details-banner", style: "{banner_style}",
                        if effects_on && effect != "none" {
                            div { class: "profile-effect effect-{effect}", aria_hidden: "true",
                                for n in 0..14 { i { key: "{n}", style: "--i: {n};" } }
                            }
                        }
                    }
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
                    button { r#type: "button", class: "details-action", title: "Nhắn tin", onclick: move |_| on_focus_composer.call(()),
                        MessageIcon {} span { "Nhắn tin" }
                    }
                    button { r#type: "button", class: "details-action", title: "Gọi thoại", onclick: move |_| on_audio_call.call(()),
                        PhoneIcon {} span { "Gọi" }
                    }
                    button { r#type: "button", class: "details-action", title: "Gọi video", onclick: move |_| on_video_call.call(()),
                        VideoIcon {} span { "Video" }
                    }
                    button { r#type: "button", class: "details-action", title: "Xem hồ sơ", onclick: move |_| on_view_profile.call(()),
                        UserIcon {} span { "Hồ sơ" }
                    }
                }

                if let Some(b) = bio {
                    div { class: "details-section",
                        div { class: "details-section-title", "Giới thiệu" }
                        div { class: "details-bio", "{b}" }
                    }
                }

                ContactNoteEditor { key: "{user_id}", user_id: user_id.clone() }

                div { class: "details-section",
                    div { class: "details-section-title", "Tìm trong cuộc trò chuyện" }
                    div { class: "search-field",
                        SearchIcon { size: 16 }
                        input {
                            r#type: "search",
                            placeholder: "Tìm tin nhắn đã tải",
                            aria_label: "Tìm tin nhắn trong cuộc trò chuyện",
                            value: "{search}",
                            oninput: move |e| search.set(e.value()),
                        }
                    }
                    if !q.is_empty() {
                        if hits.is_empty() {
                            div { class: "sidebar-empty", "Không có tin nhắn khớp" }
                        } else {
                            div { class: "details-search-results",
                                for h in hits {
                                    {
                                        let id = h.id.clone();
                                        let who = if h.is_self { "Bạn".to_string() } else { h.sender_name.clone() };
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
                        div { class: "form-hint", "Tìm kiếm chạy trên thiết bị, trong các tin nhắn đã giải mã của phiên này." }
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
                            span { class: "disclosure-title", "Bảo mật và mã hóa" }
                            span { class: "disclosure-sub",
                                if fingerprint.is_some() { "Mã hóa đầu cuối đang bật" } else { "Đang tải khóa công khai" }
                            }
                        }
                        if security_open() { ChevronDownIcon { size: 16 } } else { ChevronRightIcon { size: 16 } }
                    }
                    if security_open() {
                        div { class: "security-note",
                            "Tin nhắn trực tiếp được mã hóa bằng ECDH P-256 + AES-256-GCM và ký bằng ECDSA P-256. "
                            "Để xác minh danh tính, hãy so sánh vân tay khóa dưới đây với {username} qua một kênh khác."
                        }
                        if let Some(fp) = fingerprint.clone() {
                            div { class: "details-section-title", "Vân tay khóa của {username}" }
                            div { class: "fingerprint", "{format_fingerprint(&fp)}" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| {
                                    platform::copy_to_clipboard(&fp);
                                    copied.set(true);
                                },
                                CopyIcon {}
                                if copied() { "Đã sao chép" } else { "Sao chép vân tay" }
                            }
                        }
                        if let Some(mine) = my_fingerprint.clone() {
                            div { class: "details-section-title", "Vân tay khóa của bạn" }
                            div { class: "fingerprint", "{format_fingerprint(&mine)}" }
                        }
                        div { class: "details-row",
                            span { class: "details-row-label", "Mã người dùng" }
                        }
                        div { class: "fingerprint", "{user_id}" }
                    }
                }
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
                Ok(()) => status.set(Some((false, "Đã lưu".into()))),
                Err(_e) => status.set(Some((true, "Không lưu được".to_string()))),
            }
            busy.set(false);
        });
    };
    rsx! {
        div { class: "details-section",
            button { r#type: "button", class: "details-section-toggle", aria_expanded: "{open()}", onclick: move |_| open.set(!open()),
                span { class: "details-section-title", "Biệt danh và ghi chú riêng" }
                span { class: "member-sub", if open() { "Thu gọn" } else if nickname().is_empty() && note().is_empty() { "Thêm" } else { "Sửa" } }
            }
            if !open() && !note().trim().is_empty() {
                div { class: "details-bio private-note", "{note}" }
            }
            if open() {
                p { class: "member-sub", "Chỉ bạn thấy. Biệt danh thay tên người này trong danh sách của bạn." }
                input { r#type: "text", maxlength: "40", placeholder: "Biệt danh", aria_label: "Biệt danh", value: "{nickname}", oninput: move |e| nickname.set(e.value()) }
                textarea { rows: "3", maxlength: "500", placeholder: "Ghi chú riêng", aria_label: "Ghi chú riêng", value: "{note}", oninput: move |e| note.set(e.value()) }
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: save, "Lưu" }
                    if let Some((err, m)) = status() { span { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
                }
            }
        }
    }
}
