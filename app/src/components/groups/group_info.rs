use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::CloseIcon},
    models::{GroupItem, GroupMemberItem},
    services::groups::{
        groups_delete, groups_get_members, groups_remove_member, groups_update_member_role,
    },
};

#[component]
pub fn GroupInfoPanel(
    group: GroupItem,
    current_user_id: String,
    on_close: EventHandler<()>,
    on_open_invite: EventHandler<()>,
    on_open_add_member: EventHandler<()>,
    on_group_deleted: EventHandler<()>,
    on_group_left: EventHandler<()>,
) -> Element {
    let mut members = use_signal(Vec::<GroupMemberItem>::new);
    let error_msg = use_signal(|| Option::<String>::None);
    let group_id = group.id.clone();
    let current_role = group.role.clone().unwrap_or_else(|| "member".to_string());
    let is_owner = current_role == "owner";
    let is_admin = current_role == "admin" || is_owner;

    let fetch_members = {
        let gid = group_id.clone();
        move || {
            let gid = gid.clone();
            spawn(async move {
                if let Ok(list) = groups_get_members(&gid).await {
                    members.set(list);
                }
            });
        }
    };

    use_effect({
        let fetch = fetch_members.clone();
        move || {
            fetch();
        }
    });

    let gid_for_kick = group_id.clone();
    let fetch_for_kick = fetch_members.clone();
    let handle_kick = use_callback(move |target_uid: String| {
        let gid = gid_for_kick.clone();
        let fetch = fetch_for_kick.clone();
        spawn(async move {
            if groups_remove_member(&gid, &target_uid).await.is_ok() {
                fetch();
            }
        });
    });

    let gid_for_role = group_id.clone();
    let fetch_for_role = fetch_members.clone();
    let handle_change_role = use_callback(move |(target_uid, new_role): (String, String)| {
        let gid = gid_for_role.clone();
        let fetch = fetch_for_role.clone();
        spawn(async move {
            if groups_update_member_role(&gid, &target_uid, &new_role).await.is_ok() {
                fetch();
            }
        });
    });

    let handle_leave = {
        let gid = group_id.clone();
        let my_id = current_user_id.clone();
        move |_| {
            let gid = gid.clone();
            let my_id = my_id.clone();
            spawn(async move {
                if groups_remove_member(&gid, &my_id).await.is_ok() {
                    on_group_left.call(());
                }
            });
        }
    };

    let handle_delete = {
        let gid = group_id.clone();
        move |_| {
            let gid = gid.clone();
            spawn(async move {
                if groups_delete(&gid).await.is_ok() {
                    on_group_deleted.call(());
                }
            });
        }
    };

    rsx! {
        aside { class: "partner-info-sidebar",
            div { class: "partner-info-header",
                h3 { "Thông tin nhóm" }
                button {
                    class: "partner-info-close-btn",
                    onclick: move |_| on_close.call(()),
                    CloseIcon {}
                }
            }
            div { class: "partner-info-content",
                if let Some(err) = error_msg() {
                    div { class: "form-error", style: "margin: 8px 16px;", "{err}" }
                }

                div { class: "partner-info-profile",
                    Avatar {
                        user_id: group.id.clone(),
                        username: group.name.clone(),
                        avatar_url: group.avatar_url.clone(),
                        size: 72,
                    }
                    h4 { class: "partner-info-name", "{group.name}" }
                    if let Some(ref desc) = group.description {
                        p { class: "partner-info-meta", style: "margin-top: 4px; font-size: 13px;", "{desc}" }
                    }
                    div {
                        style: "margin-top: 6px; font-size: 12px; color: var(--text-secondary);",
                        "Vai trò của bạn: ",
                        strong { style: "color: var(--primary-color);", "{current_role.to_uppercase()}" }
                    }
                }

                if is_admin {
                    div { style: "display: flex; gap: 8px; padding: 0 16px; margin-bottom: 12px;",
                        button {
                            class: "partner-action-btn",
                            style: "flex: 1; font-size: 13px;",
                            onclick: move |_| on_open_invite.call(()),
                            "Mời liên kết"
                        }
                        button {
                            class: "partner-action-btn",
                            style: "flex: 1; font-size: 13px;",
                            onclick: move |_| on_open_add_member.call(()),
                            "+ Thêm người"
                        }
                    }
                }

                div { class: "partner-info-section",
                    h5 { class: "partner-info-section-title", "Thành viên ({members().len()})" }
                    div { class: "partner-info-card", style: "padding: 0;",
                        for m in members().iter() {
                            {
                                let m_id = m.user_id.clone();
                                let m_role = m.role.clone();
                                let is_self = m_id == current_user_id;
                                rsx! {
                                    div {
                                        key: "{m.user_id}",
                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 12px;",
                                        div { style: "display: flex; align-items: center; gap: 8px;",
                                            Avatar {
                                                user_id: m.user_id.clone(),
                                                username: m.username.clone(),
                                                avatar_url: m.avatar_url.clone(),
                                                size: 28,
                                            }
                                            div {
                                                div { style: "font-size: 13px; font-weight: 500;",
                                                    "{m.username}"
                                                    if is_self {
                                                        span { style: "color: var(--text-secondary); font-size: 11px; margin-left: 4px;", "(bạn)" }
                                                    }
                                                }
                                                div { style: "font-size: 11px; color: var(--text-secondary);",
                                                    match m.role.as_str() {
                                                        "owner" => "Trưởng nhóm",
                                                        "admin" => "Quản trị viên",
                                                        _ => "Thành viên",
                                                    }
                                                }
                                            }
                                        }

                                        if is_owner && !is_self {
                                            div { style: "display: flex; gap: 4px;",
                                                if m_role == "member" {
                                                    button {
                                                        class: "partner-action-btn",
                                                        style: "padding: 2px 6px; font-size: 11px;",
                                                        onclick: {
                                                            let m_id = m_id.clone();
                                                            move |_| handle_change_role.call((m_id.clone(), "admin".to_string()))
                                                        },
                                                        "+ Admin"
                                                    }
                                                } else if m_role == "admin" {
                                                    button {
                                                        class: "partner-action-btn",
                                                        style: "padding: 2px 6px; font-size: 11px;",
                                                        onclick: {
                                                            let m_id = m_id.clone();
                                                            move |_| handle_change_role.call((m_id.clone(), "member".to_string()))
                                                        },
                                                        "- Admin"
                                                    }
                                                }
                                                button {
                                                    class: "partner-action-btn",
                                                    style: "padding: 2px 6px; font-size: 11px; color: #EF4444;",
                                                    onclick: {
                                                        let m_id = m_id.clone();
                                                        move |_| handle_kick.call(m_id.clone())
                                                    },
                                                    "Xóa"
                                                }
                                            }
                                        } else if is_admin && !is_self && m_role == "member" {
                                            button {
                                                class: "partner-action-btn",
                                                style: "padding: 2px 6px; font-size: 11px; color: #EF4444;",
                                                onclick: {
                                                    let m_id = m_id.clone();
                                                    move |_| handle_kick.call(m_id.clone())
                                                },
                                                "Xóa"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "padding: 16px; margin-top: auto;",
                    if !is_owner {
                        button {
                            class: "partner-action-btn",
                            style: "width: 100%; color: #EF4444; background: var(--bg-error);",
                            onclick: handle_leave,
                            "Rời khỏi nhóm"
                        }
                    } else {
                        button {
                            class: "partner-action-btn",
                            style: "width: 100%; color: #EF4444; background: var(--bg-error);",
                            onclick: handle_delete,
                            "Giải tán nhóm"
                        }
                    }
                }
            }
        }
    }
}
