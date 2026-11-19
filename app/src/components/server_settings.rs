//! Community server settings (Discord style): overview (public listing after
//! verification, search-engine indexing, cover), roles with permission bits,
//! and members (assign roles, kick). The server enforces every permission.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{CloseIcon, GlobeIcon, PlusIcon, ShieldIcon, TrashIcon},
        navigation::conversations::LABEL_COLORS,
    },
    models::{ServerItem, ServerMemberItem, ServerSettings},
    platform,
    services::{
        api,
        social::{self, ServerRole},
    },
};

pub const VIEW_CHANNELS: i64 = 1 << 0;
pub const SEND_MESSAGES: i64 = 1 << 1;
pub const MANAGE_CHANNELS: i64 = 1 << 2;
pub const MANAGE_ROLES: i64 = 1 << 3;
pub const MANAGE_SERVER: i64 = 1 << 4;
pub const CREATE_INVITES: i64 = 1 << 5;
pub const ADMINISTRATOR: i64 = 1 << 6;
pub const KICK_MEMBERS: i64 = 1 << 7;
pub const MENTION_EVERYONE: i64 = 1 << 8;
pub const CONNECT_VOICE: i64 = 1 << 9;
pub const MANAGE_EXPRESSIONS: i64 = 1 << 10;

pub const PERMS: [(i64, &str, &str); 11] = [
    (VIEW_CHANNELS, "Xem kênh", "Thấy các kênh và đọc tin nhắn"),
    (SEND_MESSAGES, "Gửi tin nhắn", "Nhắn tin trong các kênh văn bản"),
    (MENTION_EVERYONE, "Nhắc @everyone và @here", "Gửi tin nhắn có @everyone hoặc @here"),
    (CONNECT_VOICE, "Vào kênh thoại", "Tham gia kênh thoại và gọi trong cộng đồng"),
    (CREATE_INVITES, "Tạo lời mời", "Tạo liên kết và thẻ mời bạn bè"),
    (MANAGE_CHANNELS, "Quản lý kênh", "Tạo, sửa, xóa kênh và danh mục"),
    (KICK_MEMBERS, "Mời thành viên ra", "Xóa thành viên khỏi cộng đồng"),
    (MANAGE_ROLES, "Quản lý vai trò", "Tạo vai trò và gán vai trò thấp hơn quyền của mình"),
    (MANAGE_SERVER, "Quản lý cộng đồng", "Đổi cài đặt chung của cộng đồng"),
    (MANAGE_EXPRESSIONS, "Manage Expressions", "Thêm, xóa emoji và sticker của cộng đồng"),
    (ADMINISTRATOR, "Quản trị viên", "Toàn quyền. Chỉ cấp cho người thật sự tin tưởng"),
];

fn role_error(code: &str) -> &'static str {
    match code {
        "cannot_grant_missing_permissions" => "Bạn không thể cấp hoặc thu hồi quyền mà chính bạn không có",
        "insufficient_permissions" => "Bạn không có quyền làm việc này",
        "cannot_delete_everyone" => "Không thể xóa vai trò @everyone",
        "cannot_rename_everyone" => "Không thể đổi tên vai trò @everyone",
        "invalid_role_name" => "Tên vai trò từ 1 đến 32 ký tự",
        "cannot_kick_owner" => "Không thể mời chủ sở hữu ra",
        "cannot_kick_administrator" => "Chỉ chủ sở hữu mới mời được quản trị viên ra",
        _ => "Không thực hiện được",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Roles,
    Members,
    Welcome,
    Expressions,
    Audit,
}

#[component]
pub fn ServerSettingsDialog(server: ServerItem, my_id: String, on_close: EventHandler<()>) -> Element {
    let mut tab = use_signal(|| Tab::Roles);
    let mut roles = use_signal(Vec::<ServerRole>::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut settings = use_signal(|| Option::<ServerSettings>::None);
    let mut mine = use_signal(|| (0i64, false));
    let mut editing = use_signal(|| Option::<String>::None);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    use_hook(|| platform::focus_selector(".manage-dialog"));
    let sid = server.id.clone();
    let reload = {
        let sid = sid.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(p) = social::server_my_permissions(&sid).await {
                    mine.set(p);
                }
                if let Ok(r) = social::server_roles(&sid).await {
                    roles.set(r);
                }
                if let Ok(m) = api::servers_get_members(&sid).await {
                    members.set(m);
                }
                if let Ok(s) = social::server_settings(&sid).await {
                    settings.set(Some(s));
                }
            });
        }
    };
    {
        let r = reload.clone();
        use_hook(move || r());
    }
    let (bits, is_owner) = mine();
    let can_roles = bits & MANAGE_ROLES != 0;
    let can_kick = bits & KICK_MEMBERS != 0;
    let editing_role = editing().and_then(|id| roles.read().iter().find(|r| r.id == id).cloned());

    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content wide manage-dialog", role: "dialog", aria_modal: "true", aria_labelledby: "srv-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    Avatar { name: server.name.clone(), avatar_url: server.icon_url.clone(), size: 40 }
                    div { class: "modal-head-text",
                        h3 { id: "srv-title", class: "modal-title", "Cài đặt cộng đồng" }
                        div { class: "modal-sub", "{server.name}" if is_owner { " · bạn là chủ sở hữu" } }
                    }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                div { class: "sidebar-filters", role: "tablist",
                    for (tb, label) in [(Tab::Overview, "Tổng quan"), (Tab::Roles, "Vai trò và quyền"), (Tab::Members, "Thành viên"), (Tab::Welcome, "Chào mừng và nội quy"), (Tab::Expressions, "Emoji và Sticker"), (Tab::Audit, "Nhật ký kiểm duyệt")] {
                        button { key: "{label}", r#type: "button", role: "tab", aria_selected: "{tab() == tb}", class: if tab() == tb { "filter-chip active" } else { "filter-chip" }, onclick: move |_| tab.set(tb), "{label}" }
                    }
                }
                if let Some((is_err, m)) = msg() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" } }
                div { class: "manage-body",
                    match tab() {
                        Tab::Overview => rsx! {
                            if let Some(st) = settings() {
                                {
                                    let (s1, s2, s3) = (sid.clone(), sid.clone(), sid.clone());
                                    rsx! {
                                        label { class: "settings-row toggle-row",
                                            span { class: "grow",
                                                span { class: "settings-label", GlobeIcon { size: 16 } " Cộng đồng công khai" }
                                                span { class: "settings-row-desc",
                                                    match (st.is_public, st.verification_status.as_str()) {
                                                        (false, _) => "Chỉ người có liên kết mời mới tham gia được",
                                                        (true, "verified") => "Đã xác minh · hiện trong Tìm kiếm, ai cũng tham gia được",
                                                        (true, "rejected") => "Bị từ chối xác minh",
                                                        _ => "Đang chờ đội ngũ ChatW xác minh",
                                                    }
                                                }
                                            }
                                            input { r#type: "checkbox", class: "switch", checked: st.is_public, disabled: !is_owner,
                                                onchange: move |e| { let s = s1.clone(); let v = e.checked(); spawn(async move {
                                                    match social::server_settings_set(&s, json!({ "is_public": v })).await { Ok(n) => settings.set(Some(n)), Err(_) => msg.set(Some((true, "Không lưu được".into()))) }
                                                }); },
                                            }
                                        }
                                        label { class: "settings-row toggle-row",
                                            span { class: "grow",
                                                span { class: "settings-label", "Cho phép Google và công cụ tìm kiếm hiển thị" }
                                                span { class: "settings-row-desc", "Trang công khai của cộng đồng đã xác minh hiển thị tin nhắn được ghim (nổi bật) và tin nhắn mới trong các kênh văn bản để công cụ tìm kiếm lập chỉ mục" }
                                            }
                                            input { r#type: "checkbox", class: "switch", checked: st.public_index, disabled: !is_owner || !st.is_public,
                                                onchange: move |e| { let s = s2.clone(); let v = e.checked(); spawn(async move {
                                                    match social::server_settings_set(&s, json!({ "public_index": v })).await { Ok(n) => settings.set(Some(n)), Err(_) => msg.set(Some((true, "Không lưu được".into()))) }
                                                }); },
                                            }
                                        }
                                        div { class: "inline-form",
                                            input { id: "srv-banner-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                                                onchange: move |_| { let s = s3.clone(); spawn(async move {
                                                    if let Ok((bytes, mime)) = platform::read_selected_file_raw("srv-banner-file").await {
                                                        match crate::services::library::upload_media(bytes, &mime).await {
                                                            Ok(url) => if let Ok(n) = social::server_settings_set(&s, json!({ "banner_url": url })).await { settings.set(Some(n)); },
                                                            Err(_) => msg.set(Some((true, "Không tải ảnh bìa lên được".into()))),
                                                        }
                                                    }
                                                }); },
                                            }
                                            if is_owner { label { r#for: "srv-banner-file", class: "btn-secondary", role: "button", tabindex: "0", "Ảnh bìa cộng đồng" } }
                                            if let Some(b) = st.banner_url.clone().filter(|b| platform::safe_asset_path(b)) {
                                                span { class: "banner-preview", style: "background-image: url('{platform::resolve_asset_url(&b)}');" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Tab::Roles => rsx! {
                            p { class: "settings-group-desc", "Vai trò quyết định thành viên được làm gì. Quyền của một người là tổng quyền của @everyone và mọi vai trò họ có." }
                            div { class: "role-layout",
                                div { class: "role-list",
                                    for r in roles() {
                                        {
                                            let id = r.id.clone();
                                            let count = members.read().iter().filter(|m| m.roles.contains(&r.id)).count();
                                            rsx! {
                                                button { key: "{r.id}", r#type: "button", class: if editing().as_deref() == Some(r.id.as_str()) { "role-item active" } else { "role-item" },
                                                    onclick: move |_| editing.set(Some(id.clone())),
                                                    span { class: "label-dot", style: "--label: {r.color};" }
                                                    span { class: "grow", "{r.name}" }
                                                }
                                            }
                                        }
                                    }
                                    if can_roles {
                                        button { r#type: "button", class: "btn-secondary",
                                            onclick: { let s = sid.clone(); move |_| { let s = s.clone(); spawn(async move {
                                                let n = roles.read().len();
                                                match social::server_role_create(&s, &format!("Vai trò {n}"), LABEL_COLORS[n % LABEL_COLORS.len()], VIEW_CHANNELS | SEND_MESSAGES).await {
                                                    Ok(r) => { editing.set(Some(r.id.clone())); roles.with_mut(|l| l.push(r)); }
                                                    Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                }
                                            }); } },
                                            PlusIcon { size: 16 } "Tạo vai trò"
                                        }
                                    }
                                }
                                if let Some(r) = editing_role {
                                    RoleEditor { key: "{r.id}", server_id: sid.clone(), role: r.clone(), my_bits: bits, can_edit: can_roles,
                                        on_saved: move |nr: ServerRole| { roles.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nr.id) { *x = nr.clone(); }); msg.set(Some((false, "Đã lưu vai trò".into()))); },
                                        on_deleted: move |id: String| { roles.with_mut(|l| l.retain(|x| x.id != id)); editing.set(None); },
                                        on_error: move |code: String| msg.set(Some((true, role_error(&code).into()))),
                                    }
                                } else {
                                    div { class: "sidebar-empty", "Chọn một vai trò để xem và sửa quyền" }
                                }
                            }
                        },
                        Tab::Welcome => rsx! { crate::components::community_ui::WelcomeEditor { server_id: server.id.clone() } },
                        Tab::Expressions => rsx! { crate::components::community_ui::ExpressionsManager { server_id: server.id.clone() } },
                        Tab::Audit => rsx! { crate::components::community_ui::AuditLogView { server_id: server.id.clone() } },
                        Tab::Members => rsx! {
                            for m in members() {
                                {
                                    let is_me = m.user_id == my_id;
                                    let is_srv_owner = m.user_id == server.owner_id;
                                    let my_roles: Vec<ServerRole> = roles.read().iter().filter(|r| m.roles.contains(&r.id)).cloned().collect();
                                    let assignable: Vec<ServerRole> = roles.read().iter().filter(|r| r.name != "@everyone" && !m.roles.contains(&r.id) && r.permissions & !bits == 0).cloned().collect();
                                    let (s_add, s_kick) = (sid.clone(), sid.clone());
                                    let (u_add, u_kick) = (m.user_id.clone(), m.user_id.clone());
                                    rsx! {
                                        div { key: "{m.user_id}", class: "member-row",
                                            Avatar { name: m.username.clone(), avatar_url: m.avatar_url.clone(), size: 36 }
                                            div { class: "grow",
                                                div { class: "member-name", "{m.username}" if is_srv_owner { span { class: "member-sub", " · chủ sở hữu" } } if is_me { span { class: "member-sub", " (bạn)" } } }
                                                div { class: "role-chips",
                                                    for r in my_roles {
                                                        {
                                                            let (s, u, rid) = (sid.clone(), m.user_id.clone(), r.id.clone());
                                                            rsx! {
                                                                span { key: "{r.id}", class: "label-pill", style: "--label: {r.color};",
                                                                    "{r.name}"
                                                                    if can_roles && r.permissions & !bits == 0 {
                                                                        button { r#type: "button", class: "chip-x", aria_label: "Gỡ vai trò {r.name}",
                                                                            onclick: move |_| { let (s, u, rid) = (s.clone(), u.clone(), rid.clone()); spawn(async move {
                                                                                match social::server_member_role(&s, &u, &rid, false).await {
                                                                                    Ok(_) => members.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.user_id == u) { x.roles.retain(|y| y != &rid); }),
                                                                                    Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                                                }
                                                                            }); },
                                                                            "×"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if can_roles && !assignable.is_empty() {
                                                select { aria_label: "Thêm vai trò cho {m.username}", class: "role-select",
                                                    onchange: move |e| { let rid = e.value(); if rid.is_empty() { return; } let (s, u) = (s_add.clone(), u_add.clone()); spawn(async move {
                                                        match social::server_member_role(&s, &u, &rid, true).await {
                                                            Ok(_) => members.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.user_id == u) { if !x.roles.contains(&rid) { x.roles.push(rid.clone()); } }),
                                                            Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                        }
                                                    }); },
                                                    option { value: "", "+ Vai trò" }
                                                    for r in assignable { option { key: "{r.id}", value: "{r.id}", "{r.name}" } }
                                                }
                                            }
                                            if can_kick && !is_me && !is_srv_owner {
                                                button { r#type: "button", class: "icon-btn danger", title: "Mời ra khỏi cộng đồng", aria_label: "Mời {m.username} ra khỏi cộng đồng",
                                                    onclick: move |_| { let (s, u) = (s_kick.clone(), u_kick.clone()); spawn(async move {
                                                        match social::server_kick(&s, &u).await {
                                                            Ok(_) => members.with_mut(|l| l.retain(|x| x.user_id != u)),
                                                            Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                        }
                                                    }); },
                                                    TrashIcon { size: 16 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-ghost", onclick: move |_| reload(), "Tải lại" }
                    button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Xong" }
                }
            }
        }
    }
}

#[component]
fn RoleEditor(server_id: String, role: ServerRole, my_bits: i64, can_edit: bool, on_saved: EventHandler<ServerRole>, on_deleted: EventHandler<String>, on_error: EventHandler<String>) -> Element {
    let mut name = use_signal(|| role.name.clone());
    let mut color = use_signal(|| role.color.clone());
    let mut bits = use_signal(|| role.permissions);
    let is_everyone = role.name == "@everyone";
    let (s_save, r_save) = (server_id.clone(), role.id.clone());
    let save = move |_| {
        let (s, r) = (s_save.clone(), r_save.clone());
        let body = if is_everyone { json!({ "permissions": bits(), "color": color() }) } else { json!({ "name": name().trim(), "color": color(), "permissions": bits() }) };
        spawn(async move {
            match social::server_role_update(&s, &r, body).await {
                Ok(nr) => on_saved.call(nr),
                Err(e) => on_error.call(e.code),
            }
        });
    };
    let (s_del, r_del) = (server_id.clone(), role.id.clone());
    rsx! {
        div { class: "role-editor",
            div { class: "form-grid",
                label { class: "form-field", span { "Tên vai trò" }
                    input { r#type: "text", maxlength: "32", value: "{name}", disabled: is_everyone || !can_edit, oninput: move |e| name.set(e.value()) }
                }
                label { class: "form-field", span { "Màu" }
                    input { r#type: "color", value: "{color}", disabled: !can_edit, oninput: move |e| color.set(e.value().to_uppercase()) }
                }
            }
            div { class: "settings-group-title", ShieldIcon { size: 16 } " Quyền" }
            for (bit, label, desc) in PERMS {
                {
                    let on = bits() & bit != 0;
                    let allowed = can_edit && my_bits & bit != 0;
                    rsx! {
                        label { key: "{bit}", class: "settings-row toggle-row",
                            span { class: "grow", span { class: "settings-label", "{label}" } span { class: "settings-row-desc", "{desc}" } }
                            input { r#type: "checkbox", class: "switch", checked: on, disabled: !allowed,
                                onchange: move |e| bits.set(if e.checked() { bits() | bit } else { bits() & !bit }) }
                        }
                    }
                }
            }
            if can_edit {
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-primary", onclick: save, "Lưu vai trò" }
                    if !is_everyone {
                        button { r#type: "button", class: "btn-danger-outline",
                            onclick: move |_| { let (s, r) = (s_del.clone(), r_del.clone()); spawn(async move {
                                match social::server_role_delete(&s, &r).await { Ok(_) => on_deleted.call(r), Err(e) => on_error.call(e.code) }
                            }); },
                            TrashIcon { size: 16 } "Xóa vai trò"
                        }
                    }
                }
            }
        }
    }
}
