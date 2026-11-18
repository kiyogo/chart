use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::CloseIcon},
    models::UserProfileResponse,
    platform,
    services::api::{get_user_profile, update_my_profile, users_upload_avatar, users_upload_banner},
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
    let mut upload_msg = use_signal(|| Option::<(bool, String)>::None);

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

    let banner_style = match profile()
        .as_ref()
        .and_then(|p| p.custom_banner_url.as_ref())
        .filter(|u| !u.is_empty())
    {
        Some(url) => {
            let resolved = platform::resolve_asset_url(url);
            format!(
                "height: 112px; background: var(--primary) url('{resolved}') center/cover no-repeat; position: relative;"
            )
        }
        None => "height: 112px; background-color: var(--primary); position: relative;".to_string(),
    };

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-dialog",
                style: "max-width: 440px; overflow: hidden;",
                onclick: move |evt| evt.stop_propagation(),

                // Profile Banner
                div {
                    class: "profile-banner",
                    style: "{banner_style}",
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        style: "position: absolute; top: 10px; right: 10px; background: rgba(15,23,42,0.45); color: #fff;",
                        aria_label: "Đóng hồ sơ",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                    if is_self {
                        label {
                            style: "position: absolute; bottom: 10px; right: 10px; background: rgba(15,23,42,0.55); color: #fff; font-size: 12px; font-weight: 500; padding: 4px 10px; border-radius: 6px; cursor: pointer;",
                            input {
                                id: "banner-file-input",
                                r#type: "file",
                                accept: "image/*",
                                style: "display: none;",
                                onchange: move |_| {
                                    upload_msg.set(None);
                                    spawn(async move {
                                        match platform::process_selected_banner_bytes().await {
                                            Ok(bytes) => match users_upload_banner(bytes).await {
                                                Ok(new_url) => {
                                                    profile.with_mut(|p| {
                                                        if let Some(pr) = p {
                                                            pr.custom_banner_url = Some(new_url.clone());
                                                        }
                                                    });
                                                    upload_msg.set(Some((false, "Đã cập nhật ảnh bìa".into())));
                                                }
                                                Err(_) => upload_msg.set(Some((true, "Không thể tải lên ảnh bìa".into()))),
                                            },
                                            Err(e) if e == "no_file" => {}
                                            Err(_) => upload_msg.set(Some((true, "Không thể xử lý ảnh bìa".into()))),
                                        }
                                    });
                                },
                            }
                            "Đổi ảnh bìa"
                        }
                    }
                }

                // Body
                div { style: "padding: 0 20px 20px 20px; position: relative;",
                    if let Some(p) = profile() {
                        div { style: "display: flex; justify-content: space-between; align-items: flex-end; margin-top: -38px; margin-bottom: 14px;",
                            div { style: "border: 4px solid var(--bg-surface); border-radius: 50%; overflow: hidden; background: var(--bg-surface);",
                                Avatar {
                                    name: p.username.clone(),
                                    avatar_url: p.avatar_url.clone(),
                                    size: 76,
                                }
                            }
                            div { style: "display: flex; gap: 6px;",
                                if is_self {
                                    label {
                                        class: "btn-secondary",
                                        style: "cursor: pointer;",
                                        input {
                                            id: "avatar-file-input",
                                            r#type: "file",
                                            accept: "image/*",
                                            style: "display: none;",
                                            onchange: move |_| {
                                                let on_upd = on_profile_updated;
                                                upload_msg.set(None);
                                                spawn(async move {
                                                    match platform::process_selected_avatar_bytes().await {
                                                        Ok(bytes) => match users_upload_avatar(bytes).await {
                                                            Ok(resp) => {
                                                                profile.with_mut(|opt| {
                                                                    if let Some(pr) = opt {
                                                                        pr.avatar_url = Some(resp.avatar_url.clone());
                                                                        if let Some(h) = on_upd {
                                                                            h.call(pr.clone());
                                                                        }
                                                                    }
                                                                });
                                                                upload_msg.set(Some((false, "Đã cập nhật ảnh đại diện".into())));
                                                            }
                                                            Err(_) => upload_msg.set(Some((true, "Không thể cập nhật ảnh đại diện".into()))),
                                                        },
                                                        Err(e) if e == "no_file" => {}
                                                        Err(_) => upload_msg.set(Some((true, "Không thể xử lý ảnh đại diện".into()))),
                                                    }
                                                });
                                            },
                                        }
                                        "Đổi ảnh"
                                    }
                                }
                                if is_self && !is_editing() {
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        onclick: move |_| is_editing.set(true),
                                        "Chỉnh sửa"
                                    }
                                }
                            }
                        }

                        if let Some((is_err, msg)) = upload_msg() {
                            div {
                                class: if is_err { "status-badge error" } else { "status-badge success" },
                                style: "margin-bottom: 12px;",
                                "{msg}"
                            }
                        }

                        if is_editing() {
                            div { style: "display: flex; flex-direction: column; gap: 12px;",
                                div { class: "form-field",
                                    label { class: "form-label", "Tên hiển thị" }
                                    input {
                                        r#type: "text",
                                        value: "{edit_display_name}",
                                        oninput: move |evt| edit_display_name.set(evt.value()),
                                    }
                                }
                                div { class: "form-field",
                                    label { class: "form-label", "Tiểu sử" }
                                    textarea {
                                        style: "resize: none; height: 64px;",
                                        value: "{edit_bio}",
                                        oninput: move |evt| edit_bio.set(evt.value()),
                                    }
                                }
                                div { class: "form-field",
                                    label { class: "form-label", "Trạng thái tùy chỉnh" }
                                    input {
                                        r#type: "text",
                                        placeholder: "Bạn đang làm gì?",
                                        value: "{edit_status_text}",
                                        oninput: move |evt| edit_status_text.set(evt.value()),
                                    }
                                }
                                div { class: "form-field",
                                    label { class: "form-label", "Trạng thái hiện diện" }
                                    select {
                                        value: "{edit_presence}",
                                        onchange: move |evt| edit_presence.set(evt.value()),
                                        option { value: "online", "Trực tuyến (Online)" }
                                        option { value: "idle", "Tạm vắng (Idle)" }
                                        option { value: "dnd", "Không làm phiền (DND)" }
                                        option { value: "offline", "Ẩn danh (Invisible)" }
                                    }
                                }
                                div { style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;",
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
                                        if is_saving() { "Đang lưu" } else { "Lưu thay đổi" }
                                    }
                                }
                            }
                        } else {
                            div {
                                div { style: "font-size: 18px; font-weight: 700; color: var(--text-primary);",
                                    "{p.display_name.clone().unwrap_or_else(|| p.username.clone())}"
                                }
                                div { style: "font-size: 13px; color: var(--text-secondary); margin-bottom: 10px;",
                                    "@{p.username}"
                                }

                                if let Some(ref st) = p.status_text {
                                    if !st.is_empty() {
                                        div {
                                            style: "display: inline-flex; align-items: center; gap: 6px; background: var(--bg-secondary); padding: 4px 10px; border-radius: 9999px; font-size: 12px; margin-bottom: 12px;",
                                            span { "{st}" }
                                        }
                                    }
                                }

                                if !p.badges.is_empty() {
                                    div { style: "display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 12px;",
                                        for b in p.badges.iter() {
                                            span { class: "member-role-badge", "{b}" }
                                        }
                                    }
                                }

                                if let Some(ref bio) = p.bio {
                                    if !bio.is_empty() {
                                        div { style: "margin-bottom: 14px; padding: 10px 12px; background: var(--bg-secondary); border-radius: var(--radius-md);",
                                            div { class: "form-label", style: "margin-bottom: 4px;", "Giới thiệu" }
                                            div { style: "font-size: 13px; color: var(--text-primary); white-space: pre-wrap;", "{bio}" }
                                        }
                                    }
                                }

                                div { style: "padding-top: 12px; display: flex; flex-direction: column; gap: 8px; font-size: 12.5px;",
                                    div { style: "display: flex; justify-content: space-between;",
                                        span { style: "color: var(--text-secondary);", "Hiện diện" }
                                        span { style: "font-weight: 600; text-transform: capitalize;", "{p.presence}" }
                                    }
                                    div { style: "display: flex; justify-content: space-between; gap: 12px;",
                                        span { style: "color: var(--text-secondary);", "Mã định danh" }
                                        span { style: "font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);", "{p.id}" }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "empty-chat-inner", "Đang tải hồ sơ" }
                    }
                }
            }
        }
    }
}
