//! Simple-group management (Zalo style): Trưởng nhóm / Phó nhóm roles, what
//! members may do, approval of new members, invite links, highlighting messages
//! from leaders, public listing (after verification) and blocking a group.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BlockIcon, CheckIcon, CloseIcon, CopyIcon, CrownIcon, GlobeIcon, LinkIcon, RefreshIcon, ShareIcon, ShieldIcon},
    },
    models::{GroupItem, GroupMemberItem, GroupSettings, JoinRequestItem},
    platform,
    services::{groups, social},
};

pub fn role_label(role: &str) -> &'static str {
    match role {
        "owner" => "Trưởng nhóm",
        "admin" => "Phó nhóm",
        _ => "Thành viên",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Perms,
    Join,
    Members,
    Public,
}

#[component]
pub fn GroupManageDialog(
    group: GroupItem,
    my_id: String,
    settings: Option<GroupSettings>,
    members: Vec<GroupMemberItem>,
    on_close: EventHandler<()>,
    on_settings: EventHandler<GroupSettings>,
    on_members_changed: EventHandler<()>,
    on_blocked: EventHandler<()>,
) -> Element {
    let mut tab = use_signal(|| Tab::Perms);
    let mut st = use_signal(|| settings.clone().unwrap_or_default());
    let mut requests = use_signal(Vec::<JoinRequestItem>::new);
    let mut link = use_signal(|| Option::<String>::None);
    let mut msg = use_signal(|| Option::<String>::None);
    let mut confirm_block = use_signal(|| false);
    use_hook(|| platform::focus_selector(".manage-dialog"));
    let gid = group.id.clone();
    let my_role = st().my_role.clone().unwrap_or_else(|| group.role.clone().unwrap_or_else(|| "member".into()));
    let is_admin = matches!(my_role.as_str(), "owner" | "admin");
    let is_owner = my_role == "owner";

    {
        let gid = gid.clone();
        use_effect(move || {
            let gid = gid.clone();
            spawn(async move {
                if let Ok(s) = social::group_settings(&gid).await {
                    st.set(s);
                }
                if let Ok(r) = social::group_join_requests(&gid).await {
                    requests.set(r);
                }
                if let Ok(code) = social::group_invite_link(&gid).await {
                    let url = format!("{}/invite/g/{}", platform::app_origin().await, code);
                    link.set(Some(url));
                }
            });
        });
    }

    let gid_set = gid.clone();
    let save = move |fields: serde_json::Value| {
        let g = gid_set.clone();
        spawn(async move {
            match social::group_settings_set(&g, fields).await {
                Ok(s) => {
                    st.set(s.clone());
                    on_settings.call(s);
                    msg.set(Some("Đã lưu".into()));
                }
                Err(e) => msg.set(Some(match e.code.as_str() {
                    "insufficient_permissions" | "only_owner" => "Chỉ trưởng hoặc phó nhóm được đổi cài đặt".into(),
                    _ => "Không lưu được".into(),
                })),
            }
        });
    };

    let perm_rows: [(&str, &str, bool); 7] = [
        ("perm_send", "Gửi tin nhắn", st().perm_send),
        ("perm_poll", "Tạo mới bình chọn", st().perm_poll),
        ("perm_note", "Tạo mới ghi chú", st().perm_note),
        ("perm_reminder", "Tạo mới nhắc hẹn", st().perm_reminder),
        ("perm_pin", "Ghim tin nhắn", st().perm_pin),
        ("perm_rename", "Đổi tên nhóm", st().perm_rename),
        ("perm_avatar", "Đổi ảnh đại diện nhóm", st().perm_avatar),
    ];

    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content wide manage-dialog", role: "dialog", aria_modal: "true", aria_labelledby: "gm-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    Avatar { name: group.name.clone(), avatar_url: group.avatar_url.clone(), size: 40 }
                    div { class: "modal-head-text",
                        h3 { id: "gm-title", class: "modal-title", "Quản lý nhóm" }
                        div { class: "modal-sub", "{group.name} · bạn là {role_label(&my_role)}" }
                    }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                div { class: "sidebar-filters", role: "tablist",
                    for (tb, label) in [(Tab::Perms, "Quyền thành viên"), (Tab::Join, "Tham gia"), (Tab::Members, "Trưởng & phó nhóm"), (Tab::Public, "Công khai & chặn")] {
                        button { key: "{label}", r#type: "button", role: "tab", aria_selected: "{tab() == tb}", class: if tab() == tb { "filter-chip active" } else { "filter-chip" }, onclick: move |_| tab.set(tb),
                            "{label}"
                            if tb == Tab::Join && !requests.read().is_empty() { span { class: "chat-item-badge", "{requests.read().len()}" } }
                        }
                    }
                }
                if let Some(m) = msg() { div { class: "status-badge success", role: "status", "{m}" } }
                div { class: "manage-body",
                    match tab() {
                        Tab::Perms => rsx! {
                            p { class: "settings-group-desc", "Cho phép các thành viên trong nhóm. Trưởng nhóm và phó nhóm luôn làm được mọi việc." }
                            for (key, label, on) in perm_rows {
                                {
                                    let s2 = save.clone();
                                    rsx! {
                                        label { key: "{key}", class: "toggle-row settings-row",
                                            span { class: "grow", "{label}" }
                                            input { r#type: "checkbox", class: "switch", checked: on, disabled: !is_admin,
                                                onchange: move |e| {
                                                    let mut m = serde_json::Map::new();
                                                    m.insert(key.to_string(), serde_json::Value::Bool(e.checked()));
                                                    s2(serde_json::Value::Object(m))
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    "Đánh dấu tin nhắn từ trưởng / phó nhóm"
                                    span { class: "settings-row-desc", "Tin nhắn của trưởng, phó nhóm được làm nổi bật" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().highlight_admins, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "highlight_admins": e.checked() })) },
                                }
                            }
                        },
                        Tab::Join => rsx! {
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    "Phê duyệt thành viên mới"
                                    span { class: "settings-row-desc", "Người tham gia bằng liên kết phải chờ trưởng hoặc phó nhóm duyệt" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().require_approval, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "require_approval": e.checked() })) },
                                }
                            }
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    "Cho phép dùng liên kết tham gia nhóm"
                                    span { class: "settings-row-desc", "Tắt để vô hiệu hóa mọi liên kết mời hiện có" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().allow_link_join, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "allow_link_join": e.checked() })) },
                                }
                            }
                            if st().allow_link_join {
                                div { class: "invite-link-box",
                                    LinkIcon { size: 16 }
                                    code { class: "invite-link-text", if let Some(l) = link() { "{l}" } else { "Đang tạo liên kết" } }
                                    if let Some(l) = link() {
                                        {
                                            let l1 = l.clone();
                                            let l2 = l.clone();
                                            let name = group.name.clone();
                                            rsx! {
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| { platform::copy_to_clipboard(&l1); msg.set(Some("Đã sao chép liên kết".into())); }, CopyIcon {} "Sao chép" }
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::share_link(&name, &l2), ShareIcon { size: 16 } "Chia sẻ" }
                                            }
                                        }
                                    }
                                    if is_admin {
                                        button { r#type: "button", class: "btn-ghost", title: "Liên kết cũ sẽ ngừng hoạt động",
                                            onclick: { let g = gid.clone(); move |_| { let g = g.clone(); spawn(async move {
                                                if let Ok(code) = social::group_invite_regenerate(&g).await {
                                                    link.set(Some(format!("{}/invite/g/{}", platform::app_origin().await, code)));
                                                    msg.set(Some("Đã đổi liên kết. Liên kết cũ không còn dùng được.".into()));
                                                }
                                            }); } },
                                            RefreshIcon { size: 16 } "Đổi liên kết"
                                        }
                                    }
                                }
                            }
                            div { class: "settings-group-title", "Yêu cầu tham gia" }
                            if requests.read().is_empty() {
                                div { class: "sidebar-empty", "Không có yêu cầu nào" }
                            }
                            for r in requests() {
                                {
                                    let (g1, g2) = (gid.clone(), gid.clone());
                                    let (u1, u2) = (r.user_id.clone(), r.user_id.clone());
                                    rsx! {
                                        div { key: "{r.user_id}", class: "member-row",
                                            Avatar { name: r.username.clone(), avatar_url: r.avatar_url.clone(), size: 36 }
                                            div { class: "grow",
                                                div { class: "member-name", "{r.username}" }
                                                if let Some(n) = r.note.as_ref() { div { class: "member-sub", "{n}" } }
                                            }
                                            if is_admin {
                                                button { r#type: "button", class: "btn-primary", onclick: move |_| { let (g, u) = (g1.clone(), u1.clone()); spawn(async move {
                                                    if social::group_decide_request(&g, &u, true).await.is_ok() { requests.with_mut(|l| l.retain(|x| x.user_id != u)); on_members_changed.call(()); }
                                                }); }, CheckIcon { size: 16 } "Duyệt" }
                                                button { r#type: "button", class: "btn-ghost", onclick: move |_| { let (g, u) = (g2.clone(), u2.clone()); spawn(async move {
                                                    if social::group_decide_request(&g, &u, false).await.is_ok() { requests.with_mut(|l| l.retain(|x| x.user_id != u)); }
                                                }); }, "Từ chối" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Tab::Members => rsx! {
                            p { class: "settings-group-desc", "Trưởng nhóm có toàn quyền và bổ nhiệm phó nhóm. Phó nhóm quản lý thành viên, duyệt người mới và ghim tin nhắn." }
                            for m in members.iter().cloned() {
                                {
                                    let (g1, g2, g3) = (gid.clone(), gid.clone(), gid.clone());
                                    let (u1, u2, u3) = (m.user_id.clone(), m.user_id.clone(), m.user_id.clone());
                                    let is_me = m.user_id == my_id;
                                    let role = m.role.clone();
                                    rsx! {
                                        div { key: "{m.user_id}", class: "member-row",
                                            Avatar { name: m.username.clone(), avatar_url: m.avatar_url.clone(), size: 36 }
                                            div { class: "grow",
                                                div { class: "member-name", "{m.username}" if is_me { span { class: "member-sub", " · bạn" } } }
                                                div { class: "member-sub", if role == "owner" { CrownIcon { size: 12 } } " {role_label(&role)}" }
                                            }
                                            if is_owner && !is_me && role != "owner" {
                                                button { r#type: "button", class: "btn-ghost",
                                                    onclick: move |_| { let (g, u) = (g1.clone(), u1.clone()); let next = if role == "admin" { "member" } else { "admin" }; spawn(async move {
                                                        match groups::groups_update_member_role(&g, &u, next).await {
                                                            Ok(_) => { on_members_changed.call(()); msg.set(Some(if next == "admin" { "Đã bổ nhiệm phó nhóm".into() } else { "Đã gỡ quyền phó nhóm".into() })); }
                                                            Err(_) => msg.set(Some("Không đổi được vai trò".into())),
                                                        }
                                                    }); },
                                                    if m.role == "admin" { "Gỡ phó nhóm" } else { "Bổ nhiệm phó nhóm" }
                                                }
                                                button { r#type: "button", class: "btn-ghost", title: "Bạn sẽ trở thành phó nhóm",
                                                    onclick: move |_| { let (g, u) = (g2.clone(), u2.clone()); spawn(async move {
                                                        match groups::groups_update_member_role(&g, &u, "owner").await {
                                                            Ok(_) => { on_members_changed.call(()); msg.set(Some("Đã chuyển quyền trưởng nhóm".into())); }
                                                            Err(_) => msg.set(Some("Không chuyển được quyền".into())),
                                                        }
                                                    }); },
                                                    CrownIcon { size: 14 } "Chuyển trưởng nhóm"
                                                }
                                            }
                                            if is_admin && !is_me && m.role == "member" {
                                                button { r#type: "button", class: "icon-btn danger", title: "Xóa khỏi nhóm", aria_label: "Xóa {m.username} khỏi nhóm",
                                                    onclick: move |_| { let (g, u) = (g3.clone(), u3.clone()); spawn(async move {
                                                        if groups::groups_remove_member(&g, &u).await.is_ok() { on_members_changed.call(()); }
                                                    }); },
                                                    CloseIcon { size: 16 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Tab::Public => rsx! {
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    GlobeIcon { size: 16 } " Nhóm công khai"
                                    span { class: "settings-row-desc",
                                        match (st().is_public, st().verification_status.as_str()) {
                                            (false, _) => "Chỉ người có liên kết mời mới tham gia được",
                                            (true, "verified") => "Đã xác minh · hiện trong Tìm kiếm cho mọi người",
                                            (true, "rejected") => "Bị từ chối xác minh · chưa hiện trong Tìm kiếm",
                                            _ => "Đang chờ đội ngũ ChatW xác minh trước khi hiện trong Tìm kiếm",
                                        }
                                    }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().is_public, disabled: !is_owner,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "is_public": e.checked() })) },
                                }
                            }
                            div { class: "settings-group-title", "Ảnh bìa nhóm" }
                            div { class: "inline-form",
                                input { id: "group-banner-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                                    onchange: { let s2 = save.clone(); move |_| { let s3 = s2.clone(); spawn(async move {
                                        match platform::read_selected_file_raw("group-banner-file").await {
                                            Ok((bytes, mime)) => match crate::services::library::upload_media(bytes, &mime).await {
                                                Ok(url) => s3(json!({ "banner_url": url })),
                                                Err(_) => msg.set(Some("Không tải ảnh bìa lên được".into())),
                                            },
                                            Err(e) if e == "too_large" => msg.set(Some("Ảnh lớn hơn 2 MB".into())),
                                            Err(_) => {}
                                        }
                                    }); } },
                                }
                                if is_admin {
                                    label { r#for: "group-banner-file", class: "btn-secondary", role: "button", tabindex: "0", "Tải ảnh bìa" }
                                }
                                if let Some(b) = st().banner_url.filter(|b| platform::safe_asset_path(b)) {
                                    span { class: "banner-preview", style: "background-image: url('{platform::resolve_asset_url(&b)}');" }
                                }
                            }
                            div { class: "danger-zone",
                                div { class: "settings-group-title", ShieldIcon { size: 16 } " Chặn nhóm" }
                                p { class: "settings-group-desc", "Rời nhóm và không ai có thể thêm bạn vào nhóm này nữa, kể cả qua liên kết mời." }
                                button { r#type: "button", class: "btn-danger", onclick: move |_| confirm_block.set(true), BlockIcon { size: 16 } "Chặn nhóm này" }
                            }
                        },
                    }
                }
            }
        }
        if confirm_block() {
            div { class: "modal-overlay", onclick: move |_| confirm_block.set(false),
                div { class: "modal-content compact", role: "alertdialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                    h3 { class: "modal-title", "Chặn nhóm {group.name}?" }
                    p { class: "modal-sub", if is_owner { "Bạn là trưởng nhóm: hãy chuyển quyền trưởng nhóm trước khi chặn." } else { "Bạn sẽ rời nhóm ngay và không thể bị thêm lại." } }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_block.set(false), "Hủy" }
                        if !is_owner {
                            button { r#type: "button", class: "btn-danger",
                                onclick: { let g = gid.clone(); move |_| { let g = g.clone(); confirm_block.set(false); spawn(async move {
                                    match social::group_block(&g).await {
                                        Ok(_) => on_blocked.call(()),
                                        Err(_) => msg.set(Some("Không chặn được nhóm".into())),
                                    }
                                }); } },
                                BlockIcon { size: 16 } "Chặn nhóm"
                            }
                        }
                    }
                }
            }
        }
    }
}
