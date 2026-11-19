use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::CloseIcon},
    models::FriendItem,
    services::groups::groups_create,
};

#[component]
pub fn CreateGroupModal(
    friends: Vec<FriendItem>,
    on_close: EventHandler<()>,
    on_created: EventHandler<String>,
) -> Element {
    let mut group_name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut selected_friends = use_signal(std::collections::HashSet::<String>::new);
    let mut is_submitting = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let handle_toggle_friend = use_callback(move |id: String| {
        selected_friends.with_mut(|set| {
            if set.contains(&id) {
                set.remove(&id);
            } else {
                set.insert(id);
            }
        });
    });

    let handle_submit = move |_| {
        let name = group_name();
        if name.trim().is_empty() {
            error_msg.set(Some("Vui lòng nhập tên nhóm".to_string()));
            return;
        }

        let desc = if description().trim().is_empty() {
            None
        } else {
            Some(description().trim().to_string())
        };

        let member_ids: Vec<String> = selected_friends().into_iter().collect();

        is_submitting.set(true);
        error_msg.set(None);

        spawn(async move {
            match groups_create(&name, desc, None, member_ids).await {
                Ok(grp) => {
                    is_submitting.set(false);
                    on_created.call(grp.id);
                }
                Err(err) => {
                    is_submitting.set(false);
                    error_msg.set(Some(format!("Lỗi tạo nhóm: {err}")));
                }
            }
        });
    };

    rsx! {
        div { class: "settings-modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "settings-modal",
                style: "max-width: 480px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "settings-modal-header",
                    h2 { "Tạo nhóm mới" }
                    button {
                        class: "settings-close-btn",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                }
                div { class: "settings-modal-content",
                    if let Some(err) = error_msg() {
                        div { class: "form-error", style: "margin-bottom: 12px;", "{err}" }
                    }

                    div { class: "settings-section",
                        label { class: "settings-label", "Tên nhóm *" }
                        input {
                            class: "search-input",
                            style: "width: 100%; box-sizing: border-box; margin-top: 6px;",
                            r#type: "text",
                            placeholder: "Nhập tên nhóm",
                            value: "{group_name}",
                            oninput: move |e| group_name.set(e.value()),
                        }
                    }

                    div { class: "settings-section", style: "margin-top: 12px;",
                        label { class: "settings-label", "Mô tả nhóm" }
                        input {
                            class: "search-input",
                            style: "width: 100%; box-sizing: border-box; margin-top: 6px;",
                            r#type: "text",
                            placeholder: "Mô tả mục đích nhóm",
                            value: "{description}",
                            oninput: move |e| description.set(e.value()),
                        }
                    }

                    div { class: "settings-section", style: "margin-top: 16px;",
                        label { class: "settings-label", "Thêm bạn bè vào nhóm" }
                        div {
                            style: "max-height: 200px; overflow-y: auto; margin-top: 8px; background: var(--bg-surface-hover); border-radius: 8px;",
                            if friends.is_empty() {
                                div { style: "padding: 16px; text-align: center; color: var(--text-secondary);", "Chưa có bạn bè nào để thêm." }
                            } else {
                                for f in friends.iter() {
                                    {
                                        let fid = f.id.clone();
                                        let is_checked = selected_friends().contains(&fid);
                                        rsx! {
                                            div {
                                                key: "{f.id}",
                                                class: "partner-info-card",
                                                style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; margin: 4px; cursor: pointer; border-radius: 6px;",
                                                onclick: {
                                                    let fid = fid.clone();
                                                    move |_| handle_toggle_friend.call(fid.clone())
                                                },
                                                div { style: "display: flex; align-items: center; gap: 10px;",
                                                    Avatar {
                                                        user_id: f.id.clone(),
                                                        username: f.username.clone(),
                                                        avatar_url: f.avatar_url.clone(),
                                                        size: 32,
                                                    }
                                                    span { style: "font-weight: 500;", "{f.username}" }
                                                }
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_checked,
                                                    onclick: move |e| e.stop_propagation(),
                                                    onchange: {
                                                        let fid = fid.clone();
                                                        move |_| handle_toggle_friend.call(fid.clone())
                                                    },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 20px;",
                        button {
                            class: "partner-action-btn",
                            onclick: move |_| on_close.call(()),
                            "Hủy"
                        }
                        button {
                            class: "add-friend-submit-btn",
                            disabled: is_submitting(),
                            onclick: handle_submit,
                            if is_submitting() { "Đang tạo" } else { "Tạo nhóm" }
                        }
                    }
                }
            }
        }
    }
}
