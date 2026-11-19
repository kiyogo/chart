//! Demo Gallery: try avatar motions, name effects, nameplates, profile effects
//! and chat wallpapers live, in desktop / mobile / reduced-motion previews,
//! then apply them to the real profile or preferences. Every "Áp dụng" saves
//! through the same API the profile editor uses.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar_motion::{AvatarMotion, AvatarStudio, MOTIONS, MotionAvatar},
        chat_extras::WALLPAPER_NAMES,
        profile_card::{EFFECTS, NAME_EFFECTS, NAMEPLATES, name_effect_class, nameplate_style},
    },
    models::UserProfileResponse,
    prefs::PrefsCtx,
    services::{api, profile_ext},
};

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Avatar,
    Names,
    Plates,
    Effects,
    Wallpapers,
}

#[component]
pub fn DemoGallery(my_id: String, my_name: String, my_avatar: Option<String>) -> Element {
    let prefs_ctx = use_context::<PrefsCtx>();
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let mut trial = use_signal(|| Option::<AvatarMotion>::None);
    let mut section = use_signal(|| Section::Avatar);
    let mut mode = use_signal(|| "desktop");
    let mut reduced = use_signal(|| false);
    let mut run_all = use_signal(|| false);
    let mut status = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);

    let uid = my_id.clone();
    use_effect(move || {
        let u = uid.clone();
        spawn(async move {
            if let Ok(p) = api::get_user_profile(&u).await {
                trial.set(p.avatar_motion.clone());
                profile.set(Some(p));
            }
        });
    });

    let mut apply_field = move |key: &'static str, value: String, label: String| {
        if busy() {
            return;
        }
        busy.set(true);
        status.set(None);
        spawn(async move {
            let mut fields = serde_json::Map::new();
            fields.insert(key.to_string(), json!(value));
            match api::update_my_profile_fields(serde_json::Value::Object(fields)).await {
                Ok(mut p) => {
                    if let Some(old) = profile.peek().as_ref() {
                        p.avatar_motion = old.avatar_motion.clone();
                        p.details = old.details.clone();
                    }
                    profile.set(Some(p));
                    status.set(Some((false, format!("Đã áp dụng {label} cho hồ sơ"))));
                }
                Err(e) => status.set(Some((true, format!("Không áp dụng được ({})", e.code)))),
            }
            busy.set(false);
        });
    };

    let apply_motion = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        status.set(None);
        let m = trial();
        spawn(async move {
            match profile_ext::set_motion(m.as_ref()).await {
                Ok(_) => {
                    profile.with_mut(|p| {
                        if let Some(p) = p {
                            p.avatar_motion = m.clone();
                        }
                    });
                    status.set(Some((false, "Đã áp dụng hiệu ứng avatar cho hồ sơ".into())));
                }
                Err(e) => status.set(Some((true, format!("Không áp dụng được ({})", e.code)))),
            }
            busy.set(false);
        });
    };

    let p = profile();
    let name = p.as_ref().and_then(|p| p.display_name.clone()).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| my_name.clone());
    let saved_motion = p.as_ref().and_then(|p| p.avatar_motion.clone());
    let cur_name_fx = p.as_ref().and_then(|p| p.name_effect.clone()).unwrap_or_else(|| "none".into());
    let cur_plate = p.as_ref().and_then(|p| p.nameplate.clone()).unwrap_or_else(|| "none".into());
    let cur_effect = p.as_ref().and_then(|p| p.profile_effect.clone()).unwrap_or_else(|| "none".into());
    let c1 = p.as_ref().and_then(|p| p.profile_color.clone()).unwrap_or_else(|| "var(--accent)".into());
    let c2 = p.as_ref().and_then(|p| p.profile_accent.clone()).unwrap_or_else(|| "#A78BFA".into());
    let cur_wp = prefs_ctx.prefs.read().wallpaper.clone();
    let stage_cls = format!("demo-stage {}{}", mode(), if reduced() { " demo-reduced" } else { "" });

    rsx! {
        div { class: "demo-gallery",
            div { class: "demo-toolbar",
                div { class: "segmented", role: "tablist", aria_label: "Nhóm demo",
                    for (s, label) in [(Section::Avatar, "Avatar"), (Section::Names, "Hiệu ứng tên"), (Section::Plates, "Bảng tên"), (Section::Effects, "Hiệu ứng hồ sơ"), (Section::Wallpapers, "Hình nền chat")] {
                        button { key: "{label}", r#type: "button", role: "tab", aria_selected: "{section() == s}", class: if section() == s { "active" } else { "" },
                            onclick: move |_| { section.set(s); status.set(None); }, "{label}" }
                    }
                }
                div { class: "segmented", role: "radiogroup", aria_label: "Chế độ xem trước",
                    for (m, label) in [("desktop", "Desktop"), ("mobile", "Mobile"), ("compact", "Gọn")] {
                        button { key: "{m}", r#type: "button", role: "radio", aria_checked: "{mode() == m}", class: if mode() == m { "active" } else { "" },
                            onclick: move |_| mode.set(m), "{label}" }
                    }
                }
                label { class: "demo-check",
                    input { r#type: "checkbox", checked: reduced(), onchange: move |e| reduced.set(e.checked()) }
                    "Xem như khi giảm chuyển động"
                }
            }
            if let Some((err, msg)) = status() {
                div { class: if err { "status-badge error" } else { "status-badge success" }, role: "status", "{msg}" }
            }
            div { class: "{stage_cls}", style: "--pc-1: {c1}; --pc-2: {c2};",
                match section() {
                    Section::Avatar => rsx! {
                        p { class: "member-sub", "Thử 40 hiệu ứng, chỉnh thông số rồi bấm Áp dụng. Người khác thấy hiệu ứng này trên hồ sơ và danh sách của họ." }
                        AvatarStudio { name: my_name.clone(), avatar_url: my_avatar.clone(), value: trial(), on_change: move |m| trial.set(m) }
                        div { class: "demo-actions",
                            button { r#type: "button", class: "btn-primary", disabled: busy() || trial() == saved_motion, onclick: apply_motion, "Áp dụng cho hồ sơ" }
                            button { r#type: "button", class: "btn-secondary", disabled: trial() == saved_motion, onclick: move |_| trial.set(saved_motion.clone()), "Khôi phục hiệu ứng đang dùng" }
                        }
                        div { class: "settings-group-title", "Xem tất cả cùng lúc" }
                        label { class: "demo-check",
                            input { r#type: "checkbox", checked: run_all(), onchange: move |e| run_all.set(e.checked()) }
                            "Chạy cả 40 hiệu ứng (tốn CPU hơn, chỉ nên bật để so sánh)"
                        }
                        div { class: "demo-motion-wall",
                            for (id, label, _, _) in MOTIONS {
                                {
                                    let mut m = AvatarMotion::of(id);
                                    m.trigger = if run_all() { "always".into() } else { "hover".into() };
                                    let pick = AvatarMotion::of(id);
                                    rsx! {
                                        button { key: "{id}-{run_all()}", r#type: "button", class: "demo-motion-cell", title: "Thử {label}",
                                            onclick: move |_| trial.set(Some(pick.clone())),
                                            MotionAvatar { name: my_name.clone(), avatar_url: my_avatar.clone(), size: 56, motion: m }
                                            span { "{label}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Section::Names => rsx! {
                        div { class: "demo-list",
                            for (id, label) in NAME_EFFECTS {
                                {
                                    let n = name.clone();
                                    rsx! {
                                        div { key: "{id}", class: if cur_name_fx == id { "demo-row current" } else { "demo-row" },
                                            span { class: "demo-name {name_effect_class(Some(id))}", "{n}" }
                                            span { class: "member-sub grow", "{label}" }
                                            if cur_name_fx == id {
                                                span { class: "demo-badge", "Đang dùng" }
                                            } else {
                                                button { r#type: "button", class: "btn-small-secondary", disabled: busy(), onclick: move |_| apply_field("name_effect", id.to_string(), format!("hiệu ứng tên {label}")), "Áp dụng" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Section::Plates => rsx! {
                        div { class: "demo-list",
                            for (id, label) in NAMEPLATES {
                                {
                                    let (cls, style) = nameplate_style(Some(id), None);
                                    let n = name.clone();
                                    rsx! {
                                        div { key: "{id}", class: if cur_plate == id { "demo-row current" } else { "demo-row" },
                                            div { class: "demo-plate chat-item {cls}", style: "{style}",
                                                span { class: "chat-item-name {name_effect_class(Some(&cur_name_fx))}", "{n}" }
                                            }
                                            span { class: "member-sub grow", "{label}" }
                                            if cur_plate == id {
                                                span { class: "demo-badge", "Đang dùng" }
                                            } else {
                                                button { r#type: "button", class: "btn-small-secondary", disabled: busy(), onclick: move |_| apply_field("nameplate", id.to_string(), format!("bảng tên {label}")), "Áp dụng" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Section::Effects => rsx! {
                        div { class: "demo-effect-grid",
                            for (id, label) in EFFECTS {
                                div { key: "{id}", class: if cur_effect == id { "demo-effect current" } else { "demo-effect" },
                                    div { class: "demo-effect-banner profile-card-banner",
                                        if id != "none" {
                                            div { class: "profile-effect effect-{id}", aria_hidden: "true",
                                                for n in 0..14 { i { key: "{n}", style: "--i: {n};" } }
                                            }
                                        }
                                    }
                                    div { class: "demo-effect-foot",
                                        span { "{label}" }
                                        if cur_effect == id {
                                            span { class: "demo-badge", "Đang dùng" }
                                        } else {
                                            button { r#type: "button", class: "btn-small-secondary", disabled: busy(), onclick: move |_| apply_field("profile_effect", id.to_string(), format!("hiệu ứng {label}")), "Áp dụng" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Section::Wallpapers => rsx! {
                        p { class: "member-sub", "Hình nền mặc định cho các đoạn chat chưa có hình nền chung. Lưu trong cài đặt và đồng bộ với thiết bị khác khi bật đồng bộ." }
                        div { class: "demo-effect-grid",
                            for (id, label) in WALLPAPER_NAMES {
                                div { key: "{id}", class: if cur_wp == id { "demo-effect current" } else { "demo-effect" },
                                    div { class: "demo-wallpaper wallpaper-sample wp-{id}",
                                        div { class: "demo-bubble other", "Chào bạn!" }
                                        div { class: "demo-bubble mine", "Hình nền này đẹp đó" }
                                    }
                                    div { class: "demo-effect-foot",
                                        span { "{label}" }
                                        if cur_wp == id {
                                            span { class: "demo-badge", "Đang dùng" }
                                        } else {
                                            button { r#type: "button", class: "btn-small-secondary",
                                                onclick: move |_| { prefs_ctx.update(move |p| p.wallpaper = id.to_string()); status.set(Some((false, format!("Đã đặt hình nền {label}")))); }, "Áp dụng" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
