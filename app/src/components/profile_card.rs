//! ChatW profile card: themed colors, avatar decoration and an optional animated
//! effect over the banner. Shared by the profile modal, popover and editor preview.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        avatar_motion::{AvatarMotion, MotionLayers, legacy_motion},
    },
    models::UserProfileResponse,
    platform,
};

pub const DECORATIONS: [(&str, &str); 6] = [
    ("none", "Không"),
    ("halo", "Vầng sáng"),
    ("crown", "Vương miện"),
    ("sprout", "Mầm xanh"),
    ("stars", "Chòm sao"),
    ("cat", "Tai mèo"),
];

pub const EFFECTS: [(&str, &str); 7] = [
    ("none", "Không"),
    ("sparkle", "Lấp lánh"),
    ("snow", "Tuyết rơi"),
    ("aurora", "Cực quang"),
    ("bubbles", "Bong bóng"),
    ("confetti", "Pháo giấy"),
    ("hearts", "Trái tim"),
];

pub const AVATAR_EFFECTS: [(&str, &str); 6] = [
    ("none", "Không"),
    ("spin", "Xoay tròn"),
    ("pulse", "Nhịp đập"),
    ("float", "Lơ lửng"),
    ("wobble", "Lắc lư"),
    ("rainbow", "Viền cầu vồng"),
];

pub const NAME_EFFECTS: [(&str, &str); 16] = [
    ("none", "Không"),
    ("gradient", "Chuyển màu"),
    ("shimmer", "Ánh kim"),
    ("rainbow", "Cầu vồng"),
    ("neon", "Neon"),
    ("wave", "Gợn sóng"),
    ("glow", "Phát sáng"),
    ("sparkle", "Lấp lánh"),
    ("glitch", "Nhiễu sóng"),
    ("pulse", "Nhịp đập"),
    ("typing", "Gõ chữ"),
    ("reveal", "Hiện dần"),
    ("cycle", "Đổi màu"),
    ("outline", "Viền chữ"),
    ("holo", "Hologram"),
    ("breathe", "Hơi thở"),
];

pub const NAMEPLATES: [(&str, &str); 7] = [
    ("none", "Không"),
    ("aurora", "Cực quang"),
    ("sunset", "Hoàng hôn"),
    ("ocean", "Đại dương"),
    ("galaxy", "Thiên hà"),
    ("sakura", "Hoa anh đào"),
    ("forest", "Rừng xanh"),
];

pub const EFFECT_ANIMS: [(&str, &str); 6] = [
    ("none", "Đứng yên"),
    ("float", "Bồng bềnh"),
    ("spin", "Xoay"),
    ("pulse", "Phập phồng"),
    ("drift", "Trôi ngang"),
    ("sway", "Đung đưa"),
];

/// CSS class + inline style for a nameplate background (lists, cards).
pub fn nameplate_style(nameplate: Option<&str>, custom_url: Option<&str>) -> (String, String) {
    match nameplate.unwrap_or("none") {
        "none" | "" => (String::new(), String::new()),
        "custom" => match custom_url.filter(|u| !u.is_empty()) {
            Some(u) => ("has-nameplate np-custom".into(), format!("--np-image: url('{}');", platform::resolve_asset_url(u))),
            None => (String::new(), String::new()),
        },
        other => (format!("has-nameplate np-{other}"), String::new()),
    }
}

pub fn name_effect_class(effect: Option<&str>) -> String {
    match effect.unwrap_or("none") {
        "none" | "" => String::new(),
        e => format!("name-fx name-fx-{e}"),
    }
}

/// Preset (primary, accent) pairs for the profile theme.
pub const THEME_PRESETS: [(&str, &str, &str); 8] = [
    ("#2764E7", "#7C3AED", "Đại dương"),
    ("#0EA5A4", "#22C55E", "Rừng nhiệt đới"),
    ("#F97316", "#EF4444", "Hoàng hôn"),
    ("#DB2777", "#F59E0B", "Hoa anh đào"),
    ("#6D28D9", "#0EA5E9", "Thiên hà"),
    ("#334155", "#64748B", "Đá xám"),
    ("#B45309", "#65A30D", "Mùa thu"),
    ("#0F766E", "#1D4ED8", "Biển sâu"),
];

pub fn badge_label(code: &str) -> &str {
    match code {
        "staff" => "Đội ngũ ChatW",
        "early" => "Người dùng sớm",
        "verified" => "Đã xác minh",
        "developer" => "Nhà phát triển",
        "moderator" => "Điều phối viên",
        other => other,
    }
}

fn member_since_label(iso: &str) -> Option<String> {
    let dt = chrono::DateTime::parse_from_rfc3339(iso).ok()?;
    let local: chrono::DateTime<chrono::Local> = dt.into();
    use chrono::Datelike;
    Some(format!("Tháng {}, {}", local.month(), local.year()))
}

fn presence_text(p: &str) -> Option<&'static str> {
    match p {
        "idle" => Some("Đang vắng"),
        "dnd" => Some("Không làm phiền"),
        _ => None,
    }
}

/// Avatar wrapped with its decoration and (spec 009) avatar motion.
#[component]
pub fn DecoratedAvatar(
    name: String,
    avatar_url: Option<String>,
    size: u32,
    #[props(default)] decoration: Option<String>,
    #[props(default)] avatar_effect: Option<String>,
    #[props(default)] custom_decoration_url: Option<String>,
    #[props(default)] motion: Option<AvatarMotion>,
    /// Lists follow the "avatar motion in lists" budget (always / hover / off).
    #[props(default)] in_list: bool,
) -> Element {
    let deco = decoration.unwrap_or_else(|| "none".to_string());
    let budget = try_consume_context::<crate::prefs::PrefsCtx>().map(|c| c.prefs.read().avatar_fx_lists.clone()).unwrap_or_else(|| "hover".into());
    let motion = motion.filter(|m| m.is_valid()).or_else(|| legacy_motion(avatar_effect.as_deref()));
    let motion = if in_list && budget == "off" { None } else { motion };
    let (mcls, mstyle) = motion.as_ref().map(|m| m.render_attrs(in_list && budget == "hover")).unwrap_or_default();
    let kind = mcls.split(' ').find_map(|c| c.strip_prefix("avm-k-")).unwrap_or_default().to_string();
    let mut on = use_signal(|| false);
    let click = mcls.contains("avm-t-click");
    rsx! {
        span { class: if on() { "deco-wrap deco-{deco} {mcls} avm-on" } else { "deco-wrap deco-{deco} {mcls}" }, style: "--deco-size: {size}px; {mstyle}",
            onclick: move |_| if click { on.set(!on()) },
            span { class: "avatar-motion", Avatar { name, avatar_url, size } }
            MotionLayers { kind }
            if deco == "custom" {
                if let Some(u) = custom_decoration_url.as_ref().filter(|u| !u.is_empty()) {
                    img { class: "deco-custom", src: "{platform::resolve_asset_url(u)}", alt: "", aria_hidden: "true" }
                }
            }
            if deco == "crown" { span { class: "deco-badge deco-badge-top", aria_hidden: "true", "👑" } }
            if deco == "sprout" { span { class: "deco-badge deco-badge-side", aria_hidden: "true", "🌱" } }
            if deco == "stars" {
                span { class: "deco-star s1", aria_hidden: "true", "✦" }
                span { class: "deco-star s2", aria_hidden: "true", "✦" }
                span { class: "deco-star s3", aria_hidden: "true", "✧" }
            }
        }
    }
}

#[component]
pub fn ProfileCard(
    profile: UserProfileResponse,
    #[props(default)] compact: bool,
    #[props(default = true)] effects_enabled: bool,
    /// Spec 012: layout, template, showcase, achievements, guestbook.
    #[props(default)] plus: Option<crate::services::profile_plus::ProfilePlus>,
    #[props(default)] is_self: bool,
    #[props(default = "self".to_string())] view_as: String,
    #[props(default)] on_plus_changed: Option<EventHandler<()>>,
    #[props(default)] children: Element,
) -> Element {
    let p = profile;
    let tpl = plus.as_ref().map(|x| format!(" pc-tpl-{}", x.template)).unwrap_or_default();
    let c1 = p.profile_color.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| "var(--accent)".to_string());
    let c2 = p.profile_accent.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| c1.clone());
    let banner_img = p
        .custom_banner_url
        .as_ref()
        .filter(|u| !u.is_empty())
        .map(|u| format!("background-image: url('{}');", platform::resolve_asset_url(u)))
        .unwrap_or_default();
    let effect = p.profile_effect.clone().unwrap_or_else(|| "none".to_string());
    let name = p.display_name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| p.username.clone());
    let status = match (p.status_emoji.as_deref(), p.status_text.as_deref()) {
        (Some(e), Some(t)) if !t.is_empty() => Some(format!("{e} {t}")),
        (_, Some(t)) if !t.is_empty() => Some(t.to_string()),
        (Some(e), _) if !e.is_empty() => Some(e.to_string()),
        _ => presence_text(&p.presence).map(str::to_string),
    };
    let since = p.member_since.as_deref().and_then(member_since_label);
    let avatar_size = if compact { 76 } else { 92 };
    let cls = if compact { "profile-card compact" } else { "profile-card" };

    rsx! {
        div { class: "{cls}{tpl}", style: "--pc-1: {c1}; --pc-2: {c2};",
            div { class: "profile-card-banner", style: "{banner_img}" }
            if effects_enabled && effect != "none" && effect != "custom" {
                div { class: "profile-effect effect-{effect}", aria_hidden: "true",
                    for n in 0..14 { i { key: "{n}", style: "--i: {n};" } }
                }
            }
            if effects_enabled && effect == "custom" {
                if let Some(u) = p.custom_effect_url.as_ref().filter(|u| !u.is_empty()) {
                    img { class: "custom-effect anim-{p.custom_effect_anim.clone().unwrap_or_else(|| \"none\".into())}", src: "{platform::resolve_asset_url(u)}", alt: "", aria_hidden: "true" }
                }
            }
            div { class: "profile-card-body",
                div { class: "profile-card-avatar",
                    DecoratedAvatar {
                        name: p.username.clone(),
                        avatar_url: p.avatar_url.clone(),
                        size: avatar_size,
                        decoration: p.avatar_decoration.clone(),
                        avatar_effect: if effects_enabled { p.avatar_effect.clone() } else { None },
                        custom_decoration_url: p.custom_decoration_url.clone(),
                        motion: if effects_enabled { p.avatar_motion.clone() } else { None },
                    }
                }
                {
                    let (np_cls, np_style) = nameplate_style(p.nameplate.as_deref(), p.custom_nameplate_url.as_deref());
                    let fx = name_effect_class(p.name_effect.as_deref());
                    rsx! {
                        div { class: "profile-card-names {np_cls}", style: "{np_style}",
                            div { class: "profile-card-name {fx}", "{name}" }
                            if let Some(pr) = p.details.pronouns.as_ref().filter(|x| !x.is_empty()) {
                                div { class: "profile-card-pronouns", "{pr}" }
                            }
                            div { class: "profile-card-username", "@{p.username}" }
                        }
                    }
                }
                if plus.is_none() && !p.badges.is_empty() {
                    div { class: "profile-badges",
                        for b in p.badges.iter() {
                            span { key: "{b}", class: "profile-badge", "{badge_label(b)}" }
                        }
                    }
                }
                if let Some(st) = status {
                    div { class: "profile-card-status", "{st}" }
                }
                if let Some(pl) = plus.clone() {
                    crate::components::profile_plus_ui::ProfileSections {
                        user_id: p.id.clone(), profile: p.clone(), plus: pl, is_self, view_as: view_as.clone(), on_changed: on_plus_changed,
                    }
                    if let Some(s) = since.clone() {
                        div { class: "profile-card-section", div { class: "profile-card-label", "Thành viên ChatW từ" } div { class: "profile-card-value", "{s}" } }
                    }
                } else {
                    div { class: "profile-card-sections",
                        if let Some(bio) = p.bio.as_ref().filter(|b| !b.trim().is_empty()) {
                            div { class: "profile-card-section",
                                div { class: "profile-card-label", "Giới thiệu" }
                                div { class: "profile-card-bio", "{bio}" }
                            }
                        }
                        if p.details.has_any() {
                            div { class: "profile-card-section",
                                div { class: "profile-card-label", "Thông tin" }
                                div { class: "profile-details",
                                    if let Some(v) = p.details.occupation.as_ref().filter(|x| !x.is_empty()) { div { class: "profile-detail", span { class: "pd-k", "Công việc" } span { "{v}" } } }
                                    if let Some(v) = p.details.location.as_ref().filter(|x| !x.is_empty()) { div { class: "profile-detail", span { class: "pd-k", "Nơi sống" } span { "{v}" } } }
                                    if let Some(v) = p.details.website.as_ref().filter(|x| x.starts_with("https://") || x.starts_with("http://")) {
                                        div { class: "profile-detail", span { class: "pd-k", "Website" }
                                            button { r#type: "button", class: "link-btn", onclick: { let v = v.clone(); move |_| platform::open_external_url(&v) }, "{v}" }
                                        }
                                    }
                                }
                                if !p.details.interests.is_empty() {
                                    div { class: "interest-chips",
                                        for i in p.details.interests.iter() { span { key: "{i}", class: "interest-chip", "{i}" } }
                                    }
                                }
                            }
                        }
                        if let Some(note) = p.note.as_ref().filter(|n| !n.trim().is_empty()) {
                            div { class: "profile-card-section private-note",
                                div { class: "profile-card-label", "Ghi chú riêng của bạn" }
                                div { class: "profile-card-bio", "{note}" }
                            }
                        }
                        if let Some(s) = since {
                            div { class: "profile-card-section",
                                div { class: "profile-card-label", "Thành viên ChatW từ" }
                                div { class: "profile-card-value", "{s}" }
                            }
                        }
                    }
                }
                {children}
            }
        }
    }
}
