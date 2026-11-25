use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{CameraIcon, CheckIcon, CloseIcon, EditIcon, GlobeIcon, SettingsIcon, TrashIcon},
    },
    models::{GroupItem, GroupMemberItem},
    platform,
    services::groups::{
        groups_delete, groups_get_members, groups_get_settings, groups_remove_member,
        groups_update, groups_update_member_role, groups_update_settings, groups_upload_avatar,
        groups_upload_banner,
    },
};

#[component]
pub fn GroupInfoPanel(
    group: GroupItem,
    current_user_id: String,
    on_close: EventHandler<()>,
    on_open_invite: EventHandler<()>,
    on_open_add_member: EventHandler<()>,
    #[props(default)]
    on_group_updated: Option<EventHandler<GroupItem>>,
    on_group_deleted: EventHandler<()>,
    on_group_left: EventHandler<()>,
) -> Element {
    let mut cur_group = use_signal(|| group.clone());
    let mut members = use_signal(Vec::<GroupMemberItem>::new);
    let mut active_tab = use_signal(|| "members".to_string()); // "members" | "settings"
    let mut search_query = use_signal(String::new);
    let mut is_editing = use_signal(|| false);
    let mut edit_name = use_signal(|| group.name.clone());
    let mut edit_desc = use_signal(|| group.description.clone().unwrap_or_default());
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut is_uploading = use_signal(|| false);
    let mut settings = use_signal(|| Option::<serde_json::Value>::None);

    let group_id = group.id.clone();
    let current_role = cur_group().role.clone().unwrap_or_else(|| "member".to_string());
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

    let fetch_settings = {
        let gid = group_id.clone();
        move || {
            let gid = gid.clone();
            spawn(async move {
                if let Ok(st) = groups_get_settings(&gid).await {
                    settings.set(Some(st));
                }
            });
        }
    };

    use_effect({
        let fetch_m = fetch_members.clone();
        let fetch_s = fetch_settings.clone();
        move || {
            fetch_m();
            fetch_s();
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

    // Save edited name and description
    let handle_save_info = {
        let gid = group_id.clone();
        let on_updated = on_group_updated.clone();
        move |_| {
            let gid = gid.clone();
            let name = edit_name().trim().to_string();
            let desc = edit_desc().trim().to_string();
            if name.is_empty() {
                error_msg.set(Some("Tên nhóm không được để trống".into()));
                return;
            }
            let on_up = on_updated.clone();
            spawn(async move {
                match groups_update(&gid, Some(name.clone()), Some(desc.clone()), None, None).await {
                    Ok(up) => {
                        cur_group.set(up.clone());
                        is_editing.set(false);
                        status_msg.set(Some("Đã lưu thông tin nhóm".into()));
                        error_msg.set(None);
                        if let Some(ref cb) = on_up {
                            cb.call(up);
                        }
                    }
                    Err(_) => {
                        error_msg.set(Some("Không thể cập nhật thông tin nhóm".into()));
                    }
                }
            });
        }
    };

    // Upload avatar
    let handle_upload_avatar = {
        let gid = group_id.clone();
        let on_updated = on_group_updated.clone();
        move |_| {
            let gid = gid.clone();
            let on_up = on_updated.clone();
            spawn(async move {
                is_uploading.set(true);
                match platform::read_selected_file_raw("group-avatar-upload-file").await {
                    Ok((bytes, _mime)) => {
                        match groups_upload_avatar(&gid, bytes).await {
                            Ok(url) => {
                                let mut g = cur_group();
                                g.avatar_url = Some(url);
                                cur_group.set(g.clone());
                                status_msg.set(Some("Đã cập nhật ảnh đại diện nhóm".into()));
                                error_msg.set(None);
                                if let Some(ref cb) = on_up {
                                    cb.call(g);
                                }
                            }
                            Err(_) => {
                                error_msg.set(Some("Không thể tải lên ảnh đại diện nhóm".into()));
                            }
                        }
                    }
                    Err(e) if e == "too_large" => {
                        error_msg.set(Some("Ảnh đại diện vượt quá dung lượng 5 MB".into()));
                    }
                    Err(_) => {}
                }
                is_uploading.set(false);
            });
        }
    };

    // Upload banner cover
    let handle_upload_banner = {
        let gid = group_id.clone();
        let on_updated = on_group_updated.clone();
        move |_| {
            let gid = gid.clone();
            let on_up = on_updated.clone();
            spawn(async move {
                is_uploading.set(true);
                match platform::read_selected_file_raw("group-cover-upload-file").await {
                    Ok((bytes, _mime)) => {
                        match groups_upload_banner(&gid, bytes).await {
                            Ok(url) => {
                                let mut g = cur_group();
                                g.banner_url = Some(url);
                                cur_group.set(g.clone());
                                status_msg.set(Some("Đã cập nhật ảnh bìa nhóm".into()));
                                error_msg.set(None);
                                if let Some(ref cb) = on_up {
                                    cb.call(g);
                                }
                            }
                            Err(_) => {
                                error_msg.set(Some("Không thể tải lên ảnh bìa nhóm".into()));
                            }
                        }
                    }
                    Err(e) if e == "too_large" => {
                        error_msg.set(Some("Ảnh bìa vượt quá dung lượng 8 MB".into()));
                    }
                    Err(_) => {}
                }
                is_uploading.set(false);
            });
        }
    };

    // Remove banner cover
    let handle_remove_banner = {
        let gid = group_id.clone();
        let on_updated = on_group_updated.clone();
        move |_| {
            let gid = gid.clone();
            let on_up = on_updated.clone();
            spawn(async move {
                match groups_update(&gid, None, None, None, Some("".into())).await {
                    Ok(up) => {
                        cur_group.set(up.clone());
                        status_msg.set(Some("Đã xóa ảnh bìa nhóm".into()));
                        error_msg.set(None);
                        if let Some(ref cb) = on_up {
                            cb.call(up);
                        }
                    }
                    Err(_) => {
                        error_msg.set(Some("Không thể xóa ảnh bìa".into()));
                    }
                }
            });
        }
    };

    // Toggle settings helper
    let gid_for_setting = group_id.clone();
    let fetch_for_setting = fetch_settings.clone();
    let toggle_setting = use_callback(move |(key, val): (&'static str, bool)| {
        let gid = gid_for_setting.clone();
        let fetch = fetch_for_setting.clone();
        spawn(async move {
            if groups_update_settings(&gid, json!({ key: val })).await.is_ok() {
                fetch();
            }
        });
    });

    // Filter members based on search
    let filtered_members: Vec<GroupMemberItem> = {
        let q = search_query().trim().to_lowercase();
        if q.is_empty() {
            members()
        } else {
            members()
                .into_iter()
                .filter(|m| m.username.to_lowercase().contains(&q))
                .collect()
        }
    };

    let banner_bg_style = if let Some(ref b) = cur_group().banner_url.filter(|s| platform::safe_asset_path(s)) {
        format!("background-image: url('{}'); background-size: cover; background-position: center;", platform::resolve_asset_url(b))
    } else {
        "background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 70%, #1e1b4b), color-mix(in srgb, #6366f1 60%, #0f172a));".to_string()
    };

    rsx! {
        aside { class: "partner-info-sidebar group-info-sidebar",
            div { class: "partner-info-header",
                h3 { "Group information" }
                button {
                    class: "partner-info-close-btn",
                    onclick: move |_| on_close.call(()),
                    CloseIcon {}
                }
            }
            div { class: "partner-info-content group-info-scroll",
                if let Some(err) = error_msg() {
                    div { class: "form-error", style: "margin: 8px 16px;", "{err}" }
                }
                if let Some(msg) = status_msg() {
                    div { class: "form-success", style: "margin: 8px 16px; color: #10B981; font-size: 13px;", "{msg}" }
                }

                // 1. Group Cover Photo Banner
                div {
                    class: "group-cover-banner",
                    style: "{banner_bg_style}",
                    if is_admin {
                        input {
                            id: "group-cover-upload-file",
                            r#type: "file",
                            accept: "image/png,image/jpeg,image/webp,image/avif,image/gif",
                            style: "display: none;",
                            onchange: handle_upload_banner,
                        }
                        div { class: "group-cover-actions",
                            label {
                                r#for: "group-cover-upload-file",
                                class: "group-cover-btn",
                                title: "Đổi ảnh bìa nhóm",
                                CameraIcon { size: 14 }
                                span { "Đổi ảnh bìa" }
                            }
                            if cur_group().banner_url.is_some() {
                                button {
                                    r#type: "button",
                                    class: "group-cover-btn btn-remove",
                                    title: "Xóa ảnh bìa",
                                    onclick: handle_remove_banner,
                                    TrashIcon { size: 14 }
                                }
                            }
                        }
                    }
                }

                // 2. Group Profile Info with Avatar Overlay
                div { class: "partner-info-profile group-profile-card",
                    div { class: "group-avatar-wrapper",
                        Avatar {
                            user_id: cur_group().id.clone(),
                            username: cur_group().name.clone(),
                            avatar_url: cur_group().avatar_url.clone(),
                            size: 76,
                        }
                        if is_admin {
                            input {
                                id: "group-avatar-upload-file",
                                r#type: "file",
                                accept: "image/png,image/jpeg,image/webp,image/avif",
                                style: "display: none;",
                                onchange: handle_upload_avatar,
                            }
                            label {
                                r#for: "group-avatar-upload-file",
                                class: "group-avatar-edit-badge",
                                title: "Đổi ảnh đại diện nhóm",
                                CameraIcon { size: 15 }
                            }
                        }
                    }

                    if is_uploading() {
                        div { style: "font-size: 12px; color: var(--accent); margin-top: 4px;", "Đang tải ảnh lên" }
                    }

                    // Name and Description (view vs edit mode)
                    if is_editing() {
                        div { class: "group-edit-form",
                            input {
                                r#type: "text",
                                class: "form-input group-name-input",
                                placeholder: "Tên nhóm",
                                value: "{edit_name()}",
                                oninput: move |e: FormEvent| edit_name.set(e.value()),
                            }
                            textarea {
                                class: "form-input group-desc-input",
                                placeholder: "Mô tả nhóm",
                                rows: "2",
                                value: "{edit_desc()}",
                                oninput: move |e: FormEvent| edit_desc.set(e.value()),
                            }
                            div { class: "group-edit-actions",
                                button {
                                    r#type: "button",
                                    class: "partner-action-btn",
                                    onclick: move |_| is_editing.set(false),
                                    "Hủy"
                                }
                                button {
                                    r#type: "button",
                                    class: "partner-action-btn btn-primary-accent",
                                    onclick: handle_save_info,
                                    CheckIcon { size: 14 }
                                    span { "Lưu" }
                                }
                            }
                        }
                    } else {
                        div { class: "group-name-row",
                            h4 { class: "partner-info-name", "{cur_group().name}" }
                            if is_admin {
                                button {
                                    r#type: "button",
                                    class: "group-edit-trigger-btn",
                                    title: "Chỉnh sửa tên và mô tả nhóm",
                                    onclick: move |_| {
                                        edit_name.set(cur_group().name.clone());
                                        edit_desc.set(cur_group().description.clone().unwrap_or_default());
                                        is_editing.set(true);
                                    },
                                    EditIcon { size: 14 }
                                }
                            }
                        }
                        if let Some(ref desc) = cur_group().description.filter(|d| !d.trim().is_empty()) {
                            p { class: "partner-info-meta group-desc-text", "{desc}" }
                        }
                    }

                    div { class: "group-role-badge-row",
                        "Vai trò của bạn: ",
                        strong { class: "group-role-chip", "{current_role.to_uppercase()}" }
                    }
                }

                // 3. Quick Action Buttons
                if is_admin {
                    div { class: "group-quick-actions",
                        button {
                            class: "partner-action-btn",
                            onclick: move |_| on_open_invite.call(()),
                            "Mời liên kết"
                        }
                        button {
                            class: "partner-action-btn",
                            onclick: move |_| on_open_add_member.call(()),
                            "+ Thêm người"
                        }
                    }
                }

                // 4. Group Tabs (Members & Settings)
                div { class: "group-nav-tabs",
                    button {
                        r#type: "button",
                        class: if active_tab() == "members" { "group-tab-btn active" } else { "group-tab-btn" },
                        onclick: move |_| active_tab.set("members".to_string()),
                        "Thành viên ({members().len()})"
                    }
                    if is_admin {
                        button {
                            r#type: "button",
                            class: if active_tab() == "settings" { "group-tab-btn active" } else { "group-tab-btn" },
                            onclick: move |_| active_tab.set("settings".to_string()),
                            SettingsIcon { size: 15 }
                            span { "Cài đặt nhóm" }
                        }
                    }
                }

                // 5. Tab Content
                if active_tab() == "settings" && is_admin {
                    // Group Settings & Permissions
                    div { class: "group-settings-panel",
                        if let Some(ref st) = settings() {
                            div { class: "settings-group-container",
                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Phê duyệt thành viên mới" }
                                        span { class: "setting-desc", "Yêu cầu trưởng nhóm hoặc quản trị viên duyệt trước khi vào nhóm" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("require_approval").and_then(|v| v.as_bool()).unwrap_or(false),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("require_approval", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Tham gia bằng liên kết" }
                                        span { class: "setting-desc", "Cho phép người khác tham gia nhóm thông qua liên kết mời" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("allow_link_join").and_then(|v| v.as_bool()).unwrap_or(true),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("allow_link_join", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Chỉ quản trị viên gửi tin (Thông báo)" }
                                        span { class: "setting-desc", "Chỉ trưởng nhóm và quản trị viên mới có thể gửi tin nhắn" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: !st.get("perm_send").and_then(|v| v.as_bool()).unwrap_or(true),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("perm_send", !e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Cho phép tạo bình chọn" }
                                        span { class: "setting-desc", "Tất cả thành viên có thể tạo cuộc thăm dò ý kiến" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("perm_poll").and_then(|v| v.as_bool()).unwrap_or(true),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("perm_poll", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Cho phép ghim tin nhắn" }
                                        span { class: "setting-desc", "Tất cả thành viên có quyền ghim tin nhắn quan trọng" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("perm_pin").and_then(|v| v.as_bool()).unwrap_or(false),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("perm_pin", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Cho phép đổi tên nhóm" }
                                        span { class: "setting-desc", "Tất cả thành viên có thể đổi tên và chủ đề của nhóm" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("perm_rename").and_then(|v| v.as_bool()).unwrap_or(false),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("perm_rename", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Cho phép đổi avatar nhóm" }
                                        span { class: "setting-desc", "Tất cả thành viên có thể đổi ảnh đại diện nhóm" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("perm_avatar").and_then(|v| v.as_bool()).unwrap_or(false),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("perm_avatar", e.checked())),
                                    }
                                }

                                label { class: "toggle-row group-setting-row",
                                    span { class: "setting-text-block",
                                        span { class: "setting-label", "Làm nổi bật quản trị viên" }
                                        span { class: "setting-desc", "Gắn nhãn quản trị viên cạnh tên trong tin nhắn" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: st.get("highlight_admins").and_then(|v| v.as_bool()).unwrap_or(true),
                                        onchange: move |e: Event<FormData>| toggle_setting.call(("highlight_admins", e.checked())),
                                    }
                                }

                                if is_owner {
                                    label { class: "toggle-row group-setting-row",
                                        span { class: "setting-text-block",
                                            span { class: "setting-label", GlobeIcon { size: 14 } " Nhóm công khai" }
                                            span { class: "setting-desc", "Cho phép tìm kiếm và khám phá nhóm trong danh mục công cộng" }
                                        }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: st.get("is_public").and_then(|v| v.as_bool()).unwrap_or(false),
                                            onchange: move |e: Event<FormData>| toggle_setting.call(("is_public", e.checked())),
                                        }
                                    }
                                }
                            }
                        } else {
                            div { style: "padding: 16px; color: var(--text-secondary); text-align: center;", "Đang tải cài đặt" }
                        }
                    }
                } else {
                    // Members Tab
                    div { class: "partner-info-section",
                        // Search filter
                        div { class: "group-search-box",
                            input {
                                r#type: "text",
                                class: "group-search-input",
                                placeholder: "Tìm kiếm thành viên",
                                value: "{search_query()}",
                                oninput: move |e: FormEvent| search_query.set(e.value()),
                            }
                        }

                        div { class: "partner-info-card", style: "padding: 0;",
                            for m in filtered_members.iter() {
                                {
                                    let m_id = m.user_id.clone();
                                    let m_role = m.role.clone();
                                    let is_self = m_id == current_user_id;
                                    rsx! {
                                        div {
                                            key: "{m.user_id}",
                                            class: "group-member-row",
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
                                                            span { style: "color: var(--text-secondary); font-size: 11px; margin-left: 4px;", "· bạn" }
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
                }

                // 6. Danger Zone (Leave / Disband)
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
