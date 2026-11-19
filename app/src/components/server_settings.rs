//! Community server settings page (matching App Settings full-page two-pane layout):
//! overview (banner, public listing, search-engine indexing, enterprise mode),
//! roles with permission bits, members, welcome & rules, expressions, audit log,
//! and enterprise settings.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        enterprise::settings::EnterpriseSettingsPanel,
        icons::{
            BackIcon, BookIcon, BuildingIcon, ChevronRightIcon, DownloadIcon, FileTextIcon,
            FriendsIcon, GlobeIcon, ImageIcon, PlusIcon, RefreshIcon, SearchIcon, ShieldIcon,
            TrashIcon,
        },
        navigation::conversations::LABEL_COLORS,
    },
    models::{ServerItem, ServerMemberItem, ServerSettings},
    platform,
    services::{
        api,
        enterprise::{ENT_PERMS_META, export_all_json_url},
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
    (MANAGE_EXPRESSIONS, "Quản lý biểu cảm", "Thêm, xóa emoji và sticker của cộng đồng"),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Roles,
    Members,
    Welcome,
    Expressions,
    Audit,
    Enterprise,
}

const NAV: [(&str, Tab, &str); 7] = [
    ("Cộng đồng", Tab::Overview, "Tổng quan"),
    ("Cộng đồng", Tab::Welcome, "Chào mừng và nội quy"),
    ("Cộng đồng", Tab::Expressions, "Emoji và Sticker"),
    ("Quản lý", Tab::Roles, "Vai trò và quyền"),
    ("Quản lý", Tab::Members, "Thành viên"),
    ("Quản lý", Tab::Audit, "Nhật ký kiểm duyệt"),
    ("Doanh nghiệp", Tab::Enterprise, "Doanh nghiệp"),
];

fn tab_label(t: Tab) -> &'static str {
    NAV.iter().find(|x| x.1 == t).map(|x| x.2).unwrap_or("Tổng quan")
}

fn tab_keywords(t: Tab) -> &'static str {
    match t {
        Tab::Overview => "tong quan cong khai tim kiem google doanh nghiep anh bia banner",
        Tab::Roles => "vai tro quyen admin quan tri vien permissions role",
        Tab::Members => "thanh vien member kick gan vai tro",
        Tab::Welcome => "chao mung noi quy welcome rules",
        Tab::Expressions => "emoji sticker bieu cam tuy chinh",
        Tab::Audit => "nhat ky kiem duyet audit log lich su",
        Tab::Enterprise => "doanh nghiep workspace phong ban nhan su cau hinh",
    }
}

fn tab_icon(t: Tab) -> Element {
    match t {
        Tab::Overview => rsx! { GlobeIcon { size: 18 } },
        Tab::Roles => rsx! { ShieldIcon { size: 18 } },
        Tab::Members => rsx! { FriendsIcon { size: 18 } },
        Tab::Welcome => rsx! { BookIcon { size: 18 } },
        Tab::Expressions => rsx! { ImageIcon { size: 18 } },
        Tab::Audit => rsx! { FileTextIcon { size: 18 } },
        Tab::Enterprise => rsx! { BuildingIcon { size: 18 } },
    }
}

#[component]
pub fn ServerSettingsView(
    server: ServerItem,
    my_id: String,
    on_close: EventHandler<()>,
    #[props(default)] on_enterprise_changed: Option<EventHandler<()>>,
) -> Element {
    let mut tab = use_signal(|| Tab::Overview);
    let mut nav_query = use_signal(String::new);
    let mut has_detail = use_signal(|| false);
    let mut roles = use_signal(Vec::<ServerRole>::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut settings = use_signal(|| Option::<ServerSettings>::None);
    let mut mine = use_signal(|| (0i64, 0i64, false));
    let mut editing = use_signal(|| Option::<String>::None);
    let mut confirm_disable_ent = use_signal(|| false);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    use_hook(|| platform::focus_selector(".server-settings-stage"));

    let sid = server.id.clone();
    let reload = {
        let sid = sid.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(p) = social::server_my_permissions_all(&sid).await {
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

    let (bits, ent_bits, is_owner) = mine();
    let can_roles = bits & MANAGE_ROLES != 0;
    let can_kick = bits & KICK_MEMBERS != 0;
    let is_ent = settings().as_ref().map(|s| s.is_enterprise).unwrap_or(server.is_enterprise);
    let can_ent_settings = is_owner || (ent_bits & ((1 << 16) | (1 << 15)) != 0);
    let show_enterprise_tab = is_ent && can_ent_settings;
    let editing_role = editing().and_then(|id| roles.read().iter().find(|r| r.id == id).cloned());

    let query_folded = crate::components::composer::fold(nav_query().trim());
    let filtered_nav: Vec<(&'static str, Tab, &'static str)> = NAV
        .iter()
        .copied()
        .filter(|(_, t, label)| {
            if *t == Tab::Enterprise && !show_enterprise_tab {
                return false;
            }
            query_folded.is_empty()
                || crate::components::composer::fold(label).contains(&query_folded)
                || tab_keywords(*t).contains(&query_folded)
        })
        .collect();

    let banner_bg = settings()
        .and_then(|s| s.banner_url)
        .filter(|b| platform::safe_asset_path(b))
        .map(|b| format!("background-image: url('{}');", platform::resolve_asset_url(&b)))
        .unwrap_or_default();

    rsx! {
        div {
            class: "main-stage server-settings-stage",
            tabindex: "-1",
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()); },
            header { class: "chat-header",
                button {
                    r#type: "button",
                    class: "icon-btn",
                    aria_label: "Quay lại cộng đồng",
                    onclick: move |_| {
                        spawn(async move {
                            if has_detail() && platform::get_viewport_width().await < 768 {
                                has_detail.set(false);
                            } else {
                                on_close.call(());
                            }
                        });
                    },
                    BackIcon {}
                }
                Avatar { name: server.name.clone(), avatar_url: server.icon_url.clone(), size: 28 }
                h2 { class: "chat-header-title", "Cài đặt cộng đồng · {server.name}" }
                div { class: "chat-header-spacer" }
                button {
                    r#type: "button",
                    class: "btn-ghost",
                    onclick: move |_| reload(),
                    RefreshIcon { size: 15 }
                    span { "Tải lại" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| on_close.call(()),
                    "Xong"
                }
            }

            div { class: if has_detail() { "settings-page has-detail" } else { "settings-page" },
                // Left Settings Nav
                nav { class: "settings-nav", aria_label: "Danh mục cài đặt cộng đồng",
                    div { class: "settings-search search-field",
                        SearchIcon { size: 16 }
                        input {
                            r#type: "search",
                            placeholder: "Tìm cài đặt",
                            aria_label: "Tìm cài đặt cộng đồng",
                            value: "{nav_query}",
                            oninput: move |e| nav_query.set(e.value()),
                            onkeydown: {
                                let first_match = filtered_nav.first().map(|x| x.1);
                                move |e| if e.key() == Key::Enter {
                                    if let Some(t) = first_match {
                                        tab.set(t);
                                        has_detail.set(true);
                                    }
                                }
                            },
                        }
                    }

                    if !nav_query().trim().is_empty() && filtered_nav.is_empty() {
                        div { class: "sidebar-empty", "Không tìm thấy cài đặt phù hợp" }
                    }

                    for (i, (group, t, label)) in filtered_nav.iter().copied().enumerate() {
                        if nav_query().trim().is_empty() && (i == 0 || filtered_nav[i - 1].0 != group) {
                            div { class: "settings-nav-title", "{group}" }
                        }

                        button {
                            r#type: "button",
                            class: if tab() == t { "settings-nav-item active" } else { "settings-nav-item" },
                            aria_current: if tab() == t { "page" } else { "false" },
                            onclick: move |_| { tab.set(t); has_detail.set(true); },
                            {tab_icon(t)}
                            span { "{label}" }
                            span { class: "chevron", ChevronRightIcon { size: 16 } }
                        }
                    }

                    div { class: "settings-nav-footer",
                        button {
                            r#type: "button",
                            class: "settings-nav-item",
                            onclick: move |_| on_close.call(()),
                            BackIcon { size: 18 }
                            span { "Quay lại cộng đồng" }
                        }
                    }
                }

                div {
                    class: "panel-resizer",
                    "data-resizer": "settings-nav",
                    title: "Kéo để thay đổi kích thước danh mục",
                }

                // Right Settings Body
                div { class: "settings-body",
                    div { class: "settings-content wide",
                        div {
                            h1 { class: "settings-page-title", "{tab_label(tab())}" }
                        }

                        if let Some((is_err, m)) = msg() {
                            div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" }
                        }

                        match tab() {
                            Tab::Enterprise => {
                                let reload_ent = reload.clone();
                                rsx! {
                                    if is_ent {
                                        EnterpriseSettingsPanel {
                                            server_id: sid.clone(),
                                            on_changed: move |_| {
                                                reload_ent();
                                                if let Some(h) = on_enterprise_changed { h.call(()); }
                                            },
                                        }
                                    } else {
                                        div { class: "settings-group",
                                            div { class: "settings-list",
                                                div { class: "settings-row",
                                                    span { class: "settings-label", "Chế độ doanh nghiệp hiện đang tắt." }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Overview => rsx! {
                                if let Some(st) = settings() {
                                    {
                                        let (s1, s2, s3, s4, s5) = (sid.clone(), sid.clone(), sid.clone(), sid.clone(), sid.clone());
                                        let mode_sub = if st.is_enterprise {
                                            if st.is_public { "Không gian doanh nghiệp · Công khai" } else { "Không gian doanh nghiệp" }
                                        } else if st.is_public {
                                            "Cộng đồng công khai"
                                        } else {
                                            "Cộng đồng riêng tư"
                                        };
                                        rsx! {
                                            div { class: "settings-profile-card",
                                                div { class: "settings-profile-banner", style: "{banner_bg}" }
                                                div { class: "settings-profile-main",
                                                    div { class: "settings-avatar-wrapper",
                                                        Avatar {
                                                            name: server.name.clone(),
                                                            avatar_url: server.icon_url.clone(),
                                                            size: 72,
                                                        }
                                                    }
                                                    div { class: "settings-profile-info",
                                                        div { class: "settings-profile-name", "{server.name}" }
                                                        div { class: "settings-profile-sub", "{mode_sub}" }
                                                    }
                                                    if is_owner {
                                                        div { class: "settings-profile-action",
                                                            input {
                                                                id: "srv-banner-file",
                                                                r#type: "file",
                                                                accept: "image/png,image/jpeg,image/webp,image/avif",
                                                                style: "display: none;",
                                                                onchange: move |_| {
                                                                    let s = s3.clone();
                                                                    spawn(async move {
                                                                        if let Ok((bytes, mime)) = platform::read_selected_file_raw("srv-banner-file").await {
                                                                            match crate::services::library::upload_media(bytes, &mime).await {
                                                                                Ok(url) => match social::server_settings_set(&s, json!({ "banner_url": url })).await {
                                                                                    Ok(n) => {
                                                                                        settings.set(Some(n));
                                                                                        msg.set(Some((false, "Đã cập nhật ảnh bìa cộng đồng".into())));
                                                                                    }
                                                                                    Err(_) => msg.set(Some((true, "Không lưu được ảnh bìa".into()))),
                                                                                },
                                                                                Err(_) => msg.set(Some((true, "Không tải ảnh bìa lên được".into()))),
                                                                            }
                                                                        }
                                                                    });
                                                                },
                                                            }
                                                            label {
                                                                r#for: "srv-banner-file",
                                                                class: "btn-avatar-upload",
                                                                role: "button",
                                                                tabindex: "0",
                                                                "Đổi ảnh bìa"
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            div { class: "settings-group",
                                                h2 { class: "settings-group-title", "Thiết lập cộng đồng" }
                                                div { class: "settings-list",
                                                    label { class: "settings-row toggle-row",
                                                        span { class: "grow",
                                                            span { class: "settings-label", "Cộng đồng công khai" }
                                                            span { class: "settings-row-desc",
                                                                match (st.is_public, st.verification_status.as_str()) {
                                                                    (false, _) => "Chỉ người có liên kết mời mới tham gia được",
                                                                    (true, "verified") => "Đã xác minh · hiện trong Tìm kiếm, ai cũng tham gia được",
                                                                    (true, "rejected") => "Bị từ chối xác minh",
                                                                    _ => "Đang chờ đội ngũ ChatW xác minh",
                                                                }
                                                            }
                                                        }
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "switch",
                                                            checked: st.is_public,
                                                            disabled: !is_owner,
                                                            onchange: move |e| {
                                                                let s = s1.clone();
                                                                let v = e.checked();
                                                                spawn(async move {
                                                                    match social::server_settings_set(&s, json!({ "is_public": v })).await {
                                                                        Ok(n) => settings.set(Some(n)),
                                                                        Err(_) => msg.set(Some((true, "Không lưu được".into()))),
                                                                    }
                                                                });
                                                            },
                                                        }
                                                    }
                                                    label { class: "settings-row toggle-row",
                                                        span { class: "grow",
                                                            span { class: "settings-label", "Cho phép Google và công cụ tìm kiếm hiển thị" }
                                                            span { class: "settings-row-desc", "Trang công khai của cộng đồng đã xác minh hiển thị tin nhắn được ghim và tin nhắn mới trong các kênh văn bản để công cụ tìm kiếm lập chỉ mục" }
                                                        }
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "switch",
                                                            checked: st.public_index,
                                                            disabled: !is_owner || !st.is_public,
                                                            onchange: move |e| {
                                                                let s = s2.clone();
                                                                let v = e.checked();
                                                                spawn(async move {
                                                                    match social::server_settings_set(&s, json!({ "public_index": v })).await {
                                                                        Ok(n) => settings.set(Some(n)),
                                                                        Err(_) => msg.set(Some((true, "Không lưu được".into()))),
                                                                    }
                                                                });
                                                            },
                                                        }
                                                    }
                                                    label { class: "settings-row toggle-row",
                                                        span { class: "grow",
                                                            span { class: "settings-label", "Chế độ doanh nghiệp" }
                                                            span { class: "ent-switch-desc",
                                                                if st.is_enterprise && st.retention_locked {
                                                                    "Đang bật khóa bảo toàn dữ liệu doanh nghiệp. Không thể tắt khi chính sách lưu trữ còn hiệu lực."
                                                                } else {
                                                                    "Bổ sung công cụ quản lý và làm việc dành cho doanh nghiệp."
                                                                }
                                                            }
                                                        }
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "switch",
                                                            checked: st.is_enterprise,
                                                            disabled: !is_owner || (st.is_enterprise && st.retention_locked),
                                                            onchange: move |e| {
                                                                let next = e.checked();
                                                                if !next {
                                                                    confirm_disable_ent.set(true);
                                                                    return;
                                                                }
                                                                let s = s4.clone();
                                                                spawn(async move {
                                                                    match social::server_settings_set(&s, json!({ "is_enterprise": true })).await {
                                                                        Ok(n) => {
                                                                            settings.set(Some(n));
                                                                            msg.set(Some((false, "Đã bật chế độ doanh nghiệp".into())));
                                                                            if let Some(h) = on_enterprise_changed { h.call(()); }
                                                                        }
                                                                        Err(err) => msg.set(Some((true, format!("Không bật được: {}", err.code)))),
                                                                    }
                                                                });
                                                            },
                                                        }
                                                    }
                                                }
                                            }

                                            if confirm_disable_ent() && st.is_enterprise {
                                                div { class: "ent-card",
                                                    p { class: "settings-row-desc", "Xác nhận tắt chế độ doanh nghiệp? Dữ liệu nhân sự, công việc và tài liệu vẫn được giữ nguyên an toàn và không bị xóa." }
                                                    div { class: "inline-form",
                                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_disable_ent.set(false), "Giữ lại" }
                                                        a { class: "btn-secondary", href: "{export_all_json_url(&s5)}", download: "enterprise-export.json", DownloadIcon { size: 14 } " Xuất dữ liệu JSON" }
                                                        button {
                                                            r#type: "button",
                                                            class: "btn-danger-outline",
                                                            onclick: {
                                                                let s = s5.clone();
                                                                move |_| {
                                                                    let s = s.clone();
                                                                    spawn(async move {
                                                                        match social::server_settings_set(&s, json!({ "is_enterprise": false })).await {
                                                                            Ok(n) => {
                                                                                confirm_disable_ent.set(false);
                                                                                settings.set(Some(n));
                                                                                msg.set(Some((false, "Đã tắt chế độ doanh nghiệp; dữ liệu cũ vẫn được bảo toàn".into())));
                                                                                if let Some(h) = on_enterprise_changed { h.call(()); }
                                                                            }
                                                                            Err(err) => {
                                                                                confirm_disable_ent.set(false);
                                                                                msg.set(Some((true, if err.code == "retention_locked" { "Không thể tắt vì đang bật khóa bảo toàn dữ liệu".into() } else { format!("Lỗi: {}", err.code) })));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            "Xác nhận tắt"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Roles => rsx! {
                                div { class: "role-layout",
                                    div { class: "role-list",
                                        for r in roles() {
                                            {
                                                let id = r.id.clone();
                                                rsx! {
                                                    button {
                                                        key: "{r.id}",
                                                        r#type: "button",
                                                        class: if editing().as_deref() == Some(r.id.as_str()) { "role-item active" } else { "role-item" },
                                                        onclick: move |_| editing.set(Some(id.clone())),
                                                        span { class: "label-dot", style: "--label: {r.color};" }
                                                        span { class: "grow", "{r.name}" }
                                                    }
                                                }
                                            }
                                        }
                                        if can_roles {
                                            button {
                                                r#type: "button",
                                                class: "btn-secondary",
                                                onclick: {
                                                    let s = sid.clone();
                                                    move |_| {
                                                        let s = s.clone();
                                                        spawn(async move {
                                                            let n = roles.read().len();
                                                            match social::server_role_create(&s, &format!("Vai trò {n}"), LABEL_COLORS[n % LABEL_COLORS.len()], VIEW_CHANNELS | SEND_MESSAGES).await {
                                                                Ok(r) => {
                                                                    editing.set(Some(r.id.clone()));
                                                                    roles.with_mut(|l| l.push(r));
                                                                }
                                                                Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                            }
                                                        });
                                                    }
                                                },
                                                PlusIcon { size: 16 }
                                                " Tạo vai trò"
                                            }
                                        }
                                    }
                                    if let Some(r) = editing_role {
                                        RoleEditor {
                                            key: "{r.id}",
                                            server_id: sid.clone(),
                                            role: r.clone(),
                                            my_bits: bits,
                                            is_enterprise: is_ent,
                                            my_ent_bits: ent_bits,
                                            can_edit: can_roles,
                                            on_saved: move |nr: ServerRole| {
                                                roles.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nr.id) { *x = nr.clone(); });
                                                msg.set(Some((false, "Đã lưu vai trò".into())));
                                            },
                                            on_deleted: move |id: String| {
                                                roles.with_mut(|l| l.retain(|x| x.id != id));
                                                editing.set(None);
                                            },
                                            on_error: move |code: String| msg.set(Some((true, role_error(&code).into()))),
                                        }
                                    } else {
                                        div { class: "settings-list",
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Chọn một vai trò bên trái để xem và chỉnh sửa quyền" }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Welcome => rsx! {
                                div { class: "settings-group",
                                    crate::components::community_ui::WelcomeEditor { server_id: server.id.clone() }
                                }
                            },
                            Tab::Expressions => rsx! {
                                div { class: "settings-group",
                                    crate::components::community_ui::ExpressionsManager { server_id: server.id.clone() }
                                }
                            },
                            Tab::Audit => rsx! {
                                div { class: "settings-group",
                                    crate::components::community_ui::AuditLogView { server_id: server.id.clone() }
                                }
                            },
                            Tab::Members => rsx! {
                                div { class: "settings-group",
                                    h2 { class: "settings-group-title", "Danh sách thành viên ({members.read().len()})" }
                                    div { class: "settings-list",
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
                                                            div { class: "member-name",
                                                                "{m.username}"
                                                                if is_srv_owner { span { class: "member-sub", " · chủ sở hữu" } }
                                                                if is_me { span { class: "member-sub", " · bạn" } }
                                                            }
                                                            div { class: "role-chips",
                                                                for r in my_roles {
                                                                    {
                                                                        let (s, u, rid) = (sid.clone(), m.user_id.clone(), r.id.clone());
                                                                        rsx! {
                                                                            span { key: "{r.id}", class: "label-pill", style: "--label: {r.color};",
                                                                                "{r.name}"
                                                                                if can_roles && r.permissions & !bits == 0 {
                                                                                    button {
                                                                                        r#type: "button",
                                                                                        class: "chip-x",
                                                                                        aria_label: "Gỡ vai trò {r.name}",
                                                                                        onclick: move |_| {
                                                                                            let (s, u, rid) = (s.clone(), u.clone(), rid.clone());
                                                                                            spawn(async move {
                                                                                                match social::server_member_role(&s, &u, &rid, false).await {
                                                                                                    Ok(_) => members.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.user_id == u) { x.roles.retain(|y| y != &rid); }),
                                                                                                    Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                                                                }
                                                                                            });
                                                                                        },
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
                                                            select {
                                                                aria_label: "Thêm vai trò cho {m.username}",
                                                                class: "role-select",
                                                                onchange: move |e| {
                                                                    let rid = e.value();
                                                                    if rid.is_empty() { return; }
                                                                    let (s, u) = (s_add.clone(), u_add.clone());
                                                                    spawn(async move {
                                                                        match social::server_member_role(&s, &u, &rid, true).await {
                                                                            Ok(_) => members.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.user_id == u) { if !x.roles.contains(&rid) { x.roles.push(rid.clone()); } }),
                                                                            Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                                        }
                                                                    });
                                                                },
                                                                option { value: "", "+ Vai trò" }
                                                                for r in assignable { option { key: "{r.id}", value: "{r.id}", "{r.name}" } }
                                                            }
                                                        }
                                                        if can_kick && !is_me && !is_srv_owner {
                                                            button {
                                                                r#type: "button",
                                                                class: "icon-btn danger",
                                                                title: "Mời ra khỏi cộng đồng",
                                                                aria_label: "Mời {m.username} ra khỏi cộng đồng",
                                                                onclick: move |_| {
                                                                    let (s, u) = (s_kick.clone(), u_kick.clone());
                                                                    spawn(async move {
                                                                        match social::server_kick(&s, &u).await {
                                                                            Ok(_) => members.with_mut(|l| l.retain(|x| x.user_id != u)),
                                                                            Err(e) => msg.set(Some((true, role_error(&e.code).into()))),
                                                                        }
                                                                    });
                                                                },
                                                                TrashIcon { size: 16 }
                                                            }
                                                        }
                                                    }
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
    }
}

#[component]
fn RoleEditor(
    server_id: String,
    role: ServerRole,
    my_bits: i64,
    is_enterprise: bool,
    my_ent_bits: i64,
    can_edit: bool,
    on_saved: EventHandler<ServerRole>,
    on_deleted: EventHandler<String>,
    on_error: EventHandler<String>,
) -> Element {
    let mut name = use_signal(|| role.name.clone());
    let mut color = use_signal(|| role.color.clone());
    let mut bits = use_signal(|| role.permissions);
    let mut ent_bits = use_signal(|| role.enterprise_permissions);
    let is_everyone = role.name == "@everyone";
    let (s_save, r_save) = (server_id.clone(), role.id.clone());
    let save = move |_| {
        let (s, r) = (s_save.clone(), r_save.clone());
        let body = if is_everyone {
            json!({ "permissions": bits(), "enterprise_permissions": ent_bits(), "color": color() })
        } else {
            json!({ "name": name().trim(), "color": color(), "permissions": bits(), "enterprise_permissions": ent_bits() })
        };
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
            div { class: "settings-group",
                div { class: "settings-list",
                    div { class: "settings-row",
                        span { class: "settings-label", "Tên vai trò" }
                        input {
                            r#type: "text",
                            maxlength: "32",
                            value: "{name}",
                            disabled: is_everyone || !can_edit,
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div { class: "settings-row",
                        span { class: "settings-label", "Màu hiển thị" }
                        input {
                            r#type: "color",
                            value: "{color}",
                            disabled: !can_edit,
                            oninput: move |e| color.set(e.value().to_uppercase()),
                        }
                    }
                }
            }

            div { class: "settings-group",
                h3 { class: "settings-group-title", ShieldIcon { size: 16 } " Quyền máy chủ" }
                div { class: "settings-list",
                    for (bit, label, desc) in PERMS {
                        {
                            let on = bits() & bit != 0;
                            let allowed = can_edit && my_bits & bit != 0;
                            rsx! {
                                label { key: "{bit}", class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "{label}" }
                                        span { class: "settings-row-desc", "{desc}" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: on,
                                        disabled: !allowed,
                                        onchange: move |e| bits.set(if e.checked() { bits() | bit } else { bits() & !bit }),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if is_enterprise {
                div { class: "settings-group",
                    h3 { class: "settings-group-title", BuildingIcon { size: 16 } " Quyền doanh nghiệp" }
                    div { class: "settings-list",
                        for (bit, label, desc) in ENT_PERMS_META {
                            {
                                let on = ent_bits() & bit != 0;
                                let allowed = can_edit && my_ent_bits & bit != 0;
                                rsx! {
                                    label { key: "ent-{bit}", class: "settings-row toggle-row",
                                        span { class: "grow",
                                            span { class: "settings-label", "{label}" }
                                            span { class: "settings-row-desc", "{desc}" }
                                        }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: on,
                                            disabled: !allowed,
                                            onchange: move |e| ent_bits.set(if e.checked() { ent_bits() | bit } else { ent_bits() & !bit }),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if can_edit {
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-primary", onclick: save, "Lưu vai trò" }
                    if !is_everyone {
                        button {
                            r#type: "button",
                            class: "btn-danger-outline",
                            onclick: move |_| {
                                let (s, r) = (s_del.clone(), r_del.clone());
                                spawn(async move {
                                    match social::server_role_delete(&s, &r).await {
                                        Ok(_) => on_deleted.call(r),
                                        Err(e) => on_error.call(e.code),
                                    }
                                });
                            },
                            TrashIcon { size: 16 }
                            " Xóa vai trò"
                        }
                    }
                }
            }
        }
    }
}
