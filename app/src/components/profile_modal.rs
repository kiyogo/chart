use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::CloseIcon},
    models::UserProfileResponse,
    services::api::{get_user_profile, update_my_profile},
};

#[component]
pub fn ProfileModal(
    user_id: String,
    is_self: bool,
    on_close: EventHandler<()>,
    #[props(default)] on_profile_updated: Option<EventHandler<UserProfileResponse>>,
) -> Element {
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let mut is_editing = use_signal(|| false);
    let mut edit_display_name = use_signal(String::new);
    let mut edit_bio = use_signal(String::new);
    let mut edit_status_text = use_signal(String::new);
    let mut edit_presence = use_signal(|| "online".to_string());
    let mut is_saving = use_signal(|| false);

    let uid = user_id.clone();
    use_effect(move || {
        let u = uid.clone();
        spawn(async move {
            if let Ok(p) = get_user_profile(&u).await {
                edit_display_name.set(p.display_name.clone().unwrap_or_default());
                edit_bio.set(p.bio.clone().unwrap_or_default());
                edit_status_text.set(p.status_text.clone().unwrap_or_default());
                edit_presence.set(p.presence.clone());
                profile.set(Some(p));
            }
        });
    });

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-dialog",
                style: "max-width: 420px; width: 100%; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 16px; overflow: hidden; box-shadow: 0 8px 32px rgba(0,0,0,0.2);",
                onclick: move |evt| evt.stop_propagation(),

                // Banner
                div {
                    class: "profile-banner",
                    style: "height: 100px; background: linear-gradient(135deg, var(--primary), var(--primary-hover)); position: relative;",
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        style: "position: absolute; top: 8px; right: 8px; background: rgba(0,0,0,0.3); color: #fff;",
                        aria_label: "Đóng",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                }

                // Profile Header Body
                div {
                    style: "padding: 0 20px 20px 20px; position: relative;",
                    if let Some(p) = profile() {
                        div {
                            style: "display: flex; justify-content: space-between; align-items: flex-end; margin-top: -36px; margin-bottom: 12px;",
                            div {
                                style: "border: 4px solid var(--bg-surface); border-radius: 50%; overflow: hidden; background: var(--bg-surface);",
                                Avatar {
                                    name: p.username.clone(),
                                    avatar_url: p.avatar_url.clone(),
                                    size: 72,
                                }
                            }
                            if is_self && !is_editing() {
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    style: "font-size: 13px; padding: 6px 12px; border-radius: 8px;",
                                    onclick: move |_| is_editing.set(true),
                                    "Chỉnh sửa hồ sơ"
                                }
                            }
                        }

                        if is_editing() {
                            // Edit Form
                            div {
                                style: "display: flex; flex-direction: column; gap: 12px;",
                                div {
                                    label { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", "Tên hiển thị" }
                                    input {
                                        r#type: "text",
                                        class: "chat-input",
                                        style: "width: 100%; border: 1px solid var(--border); border-radius: 8px; padding: 8px;",
                                        value: "{edit_display_name}",
                                        oninput: move |evt| edit_display_name.set(evt.value()),
                                    }
                                }
                                div {
                                    label { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", "Tiểu sử (Bio)" }
                                    textarea {
                                        class: "chat-input",
                                        style: "width: 100%; border: 1px solid var(--border); border-radius: 8px; padding: 8px; resize: none; height: 60px;",
                                        value: "{edit_bio}",
                                        oninput: move |evt| edit_bio.set(evt.value()),
                                    }
                                }
                                div {
                                    label { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", "Trạng thái" }
                                    input {
                                        r#type: "text",
                                        class: "chat-input",
                                        style: "width: 100%; border: 1px solid var(--border); border-radius: 8px; padding: 8px;",
                                        placeholder: "Bạn đang nghĩ gì?",
                                        value: "{edit_status_text}",
                                        oninput: move |evt| edit_status_text.set(evt.value()),
                                    }
                                }
                                div {
                                    label { style: "font-size: 12px; font-weight: 600; color: var(--text-secondary);", "Hiện diện" }
                                    select {
                                        style: "width: 100%; border: 1px solid var(--border); border-radius: 8px; padding: 8px; background: var(--bg-surface); color: var(--text);",
                                        value: "{edit_presence}",
                                        onchange: move |evt| edit_presence.set(evt.value()),
                                        option { value: "online", "Trực tuyến (Online)" }
                                        option { value: "idle", "Chờ (Idle)" }
                                        option { value: "dnd", "Không làm phiền (DND)" }
                                        option { value: "offline", "Ẩn danh (Invisible)" }
                                    }
                                }
                                div {
                                    style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px;",
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        onclick: move |_| is_editing.set(false),
                                        "Hủy"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        disabled: is_saving(),
                                        onclick: move |_| {
                                            is_saving.set(true);
                                            let dname = edit_display_name().trim().to_string();
                                            let bio = edit_bio().trim().to_string();
                                            let st = edit_status_text().trim().to_string();
                                            let pres = edit_presence();
                                            let on_updated = on_profile_updated;
                                            spawn(async move {
                                                if let Ok(updated) = update_my_profile(
                                                    if dname.is_empty() { None } else { Some(&dname) },
                                                    if bio.is_empty() { None } else { Some(&bio) },
                                                    if st.is_empty() { None } else { Some(&st) },
                                                    None,
                                                    Some(&pres),
                                                    None,
                                                ).await {
                                                    profile.set(Some(updated.clone()));
                                                    if let Some(h) = on_updated {
                                                        h.call(updated);
                                                    }
                                                }
                                                is_saving.set(false);
                                                is_editing.set(false);
                                            });
                                        },
                                        if is_saving() { "Đang lưu..." } else { "Lưu thay đổi" }
                                    }
                                }
                            }
                        } else {
                            // View Mode
                            div {
                                div {
                                    style: "font-size: 18px; font-weight: 700; color: var(--text);",
                                    "{p.display_name.clone().unwrap_or_else(|| p.username.clone())}"
                                }
                                div {
                                    style: "font-size: 13px; color: var(--text-secondary); margin-bottom: 8px;",
                                    "@{p.username}"
                                }

                                if let Some(ref st) = p.status_text {
                                    if !st.is_empty() {
                                        div {
                                            style: "display: inline-flex; align-items: center; gap: 6px; background: var(--bg-hover); padding: 4px 10px; border-radius: 12px; font-size: 12px; margin-bottom: 12px;",
                                            span { "💬" }
                                            span { "{st}" }
                                        }
                                    }
                                }

                                if let Some(ref bio) = p.bio {
                                    if !bio.is_empty() {
                                        div {
                                            style: "margin-top: 8px; margin-bottom: 12px;",
                                            div { style: "font-size: 11px; font-weight: 600; text-transform: uppercase; color: var(--text-secondary); margin-bottom: 4px;", "Giới thiệu" }
                                            div { style: "font-size: 13px; color: var(--text); white-space: pre-wrap;", "{bio}" }
                                        }
                                    }
                                }

                                div {
                                    style: "margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 8px;",
                                    div {
                                        style: "display: flex; justify-content: space-between; font-size: 12px;",
                                        span { style: "color: var(--text-secondary);", "Trạng thái:" }
                                        span {
                                            class: "presence-badge {p.presence}",
                                            style: "text-transform: capitalize; font-weight: 600;",
                                            "{p.presence}"
                                        }
                                    }
                                    div {
                                        style: "display: flex; justify-content: space-between; font-size: 12px;",
                                        span { style: "color: var(--text-secondary);", "ID tài khoản:" }
                                        span { style: "font-family: monospace; font-size: 11px; color: var(--text-secondary);", "{p.id}" }
                                    }
                                }
                            }
                        }
                    } else {
                        div { style: "text-align: center; padding: 24px; color: var(--text-secondary);", "Đang tải thông tin hồ sơ..." }
                    }
                }
            }
        }
    }
}
