//! Invitations as links and cards (never bare codes): preview a group or
//! community from a link / card and join it, and invite friends by sending a
//! rich card (avatar, cover, member count) inside the end-to-end encrypted DM.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        composer::fold,
        icons::{CheckIcon, CloseIcon, CopyIcon, SearchIcon, ShareIcon},
        rich::InviteData,
    },
    models::{FriendItem, InvitePreview},
    platform,
    services::{api, dm, groups, social},
};

pub fn invite_url(origin: &str, kind: &str, code: &str) -> String {
    let k = if kind == "server" { "s" } else { "g" };
    format!("{origin}/invite/{k}/{code}")
}

#[component]
pub fn InviteDialog(kind: String, code: String, on_close: EventHandler<()>, on_joined: EventHandler<(String, String)>) -> Element {
    let mut preview = use_signal(|| Option::<InvitePreview>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut busy = use_signal(|| false);
    let mut pending = use_signal(|| false);
    use_hook(|| platform::focus_selector(".invite-dialog"));
    let (k0, c0) = (kind.clone(), code.clone());
    use_effect(move || {
        let (k, c) = (k0.clone(), c0.clone());
        spawn(async move {
            match social::invite_preview(&k, &c).await {
                Ok(p) => preview.set(Some(p)),
                Err(e) => error.set(Some(match e.code.as_str() {
                    "invite_not_found" | "not_found" => "Liên kết mời không tồn tại hoặc đã hết hạn".into(),
                    "invite_expired" => "Liên kết mời đã hết hạn".into(),
                    "link_join_disabled" => "Nhóm đã tắt tham gia bằng liên kết".into(),
                    _ => "Không mở được liên kết mời".into(),
                })),
            }
        });
    });
    let (k1, c1) = (kind.clone(), code.clone());
    let join = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let (k, c) = (k1.clone(), c1.clone());
        spawn(async move {
            if k == "server" {
                match api::servers_join(&c).await {
                    Ok(srv) => on_joined.call(("server".into(), srv.id)),
                    Err(_) => error.set(Some("Không tham gia được cộng đồng".into())),
                }
            } else {
                match groups::groups_join_by_invite(&c).await {
                    Ok(g) => on_joined.call(("group".into(), g.id)),
                    Err(e) if e.code == "join_request_pending" => pending.set(true),
                    Err(e) => error.set(Some(match e.code.as_str() {
                        "group_blocked" | "blocked_from_group" => "Bạn không thể tham gia nhóm này".into(),
                        "link_join_disabled" => "Nhóm đã tắt tham gia bằng liên kết".into(),
                        _ => "Không tham gia được nhóm".into(),
                    })),
                }
            }
            busy.set(false);
        });
    };
    let what = if kind == "server" { "cộng đồng" } else { "nhóm" };
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content invite-dialog", role: "dialog", aria_modal: "true", aria_label: "Lời mời tham gia", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                button { r#type: "button", class: "icon-btn invite-close", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                if let Some(p) = preview() {
                    {
                        let banner = p.banner_url.clone().filter(|b| platform::safe_asset_path(b));
                        rsx! {
                            div { class: "invite-banner", style: if let Some(b) = banner.as_ref() { format!("background-image: url('{}');", platform::resolve_asset_url(b)) } else { String::new() } }
                            div { class: "invite-body",
                                div { class: "invite-avatar", Avatar { name: p.name.clone(), avatar_url: p.avatar_url.clone(), size: 72 } }
                                div { class: "invite-kicker", "Bạn được mời tham gia {what}" }
                                h3 { class: "invite-name", "{p.name}" }
                                div { class: "invite-meta", "{p.member_count} thành viên" if p.require_approval { " · cần trưởng nhóm duyệt" } }
                                if let Some(d) = p.description.as_ref().filter(|d| !d.trim().is_empty()) { p { class: "invite-desc", "{d}" } }
                                if pending() {
                                    div { class: "status-badge success", "Đã gửi yêu cầu tham gia. Bạn sẽ vào nhóm khi được duyệt." }
                                } else if p.is_member {
                                    button { r#type: "button", class: "btn-primary wide", onclick: { let (k, id) = (kind.clone(), p.id.clone()); move |_| on_joined.call((k.clone(), id.clone())) }, CheckIcon { size: 16 } "Bạn đã ở trong {what} · Mở" }
                                } else {
                                    button { r#type: "button", class: "btn-primary wide", disabled: busy(), onclick: join,
                                        if p.require_approval { "Gửi yêu cầu tham gia" } else { "Tham gia {what}" }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(e) = error() {
                    div { class: "invite-body", h3 { class: "invite-name", "Không mở được lời mời" } p { class: "invite-desc", "{e}" } }
                } else {
                    div { class: "invite-body", p { class: "invite-desc", role: "status", "Đang tải lời mời" } }
                }
                if let (Some(e), Some(_)) = (error(), preview()) { div { class: "status-badge error", "{e}" } }
            }
        }
    }
}

/// Invite friends to a group or community with a card message (not a bare link).
#[component]
pub fn InviteFriendsDialog(
    kind: String,
    target_id: String,
    name: String,
    avatar_url: Option<String>,
    member_count: i64,
    friends: Vec<FriendItem>,
    my_id: String,
    my_name: String,
    on_close: EventHandler<()>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut picked = use_signal(Vec::<String>::new);
    let mut sent = use_signal(Vec::<String>::new);
    let mut code = use_signal(|| Option::<String>::None);
    let mut banner = use_signal(|| Option::<String>::None);
    let mut origin = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut busy = use_signal(|| false);
    use_hook(|| platform::focus_selector("[aria-labelledby='invf-title']"));
    let (k0, t0) = (kind.clone(), target_id.clone());
    use_effect(move || {
        let (k, t) = (k0.clone(), t0.clone());
        spawn(async move {
            origin.set(platform::app_origin().await);
            let c = if k == "server" {
                api::servers_create_invite(&t).await.ok()
            } else {
                match social::group_invite_link(&t).await {
                    Ok(c) => Some(c),
                    Err(e) => {
                        if e.code == "link_join_disabled" {
                            error.set(Some("Nhóm đang tắt tham gia bằng liên kết. Bật lại trong Quản lý nhóm › Tham gia.".into()));
                        }
                        None
                    }
                }
            };
            if let Some(c) = c.as_ref() {
                if let Ok(p) = social::invite_preview(&k, c).await {
                    banner.set(p.banner_url);
                }
            }
            code.set(c);
        });
    });
    let q = fold(query().trim());
    let shown: Vec<FriendItem> = friends.iter().filter(|f| q.is_empty() || fold(&f.username).contains(&q)).cloned().collect();
    let link = code().map(|c| invite_url(&origin(), &kind, &c));
    let (k1, n1, a1) = (kind.clone(), name.clone(), avatar_url.clone());
    let send = move |_| {
        let Some(c) = code() else {
            return;
        };
        if picked.read().is_empty() || busy() {
            return;
        }
        busy.set(true);
        let card = InviteData {
            kind: k1.clone(),
            code: c,
            name: n1.clone(),
            avatar: a1.clone(),
            banner: banner(),
            members: member_count,
            from: my_name.clone(),
        };
        let text = format!("invite:{}", serde_json::to_string(&card).unwrap_or_default());
        let me = my_id.clone();
        let targets = picked();
        spawn(async move {
            for fid in targets {
                match dm::send_encrypted_dm(&me, &fid, &text, None).await {
                    Ok(_) => sent.with_mut(|s| s.push(fid.clone())),
                    Err(_) => error.set(Some("Một số lời mời không gửi được".into())),
                }
            }
            picked.set(Vec::new());
            busy.set(false);
        });
    };
    let what = if kind == "server" { "cộng đồng" } else { "nhóm" };
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content", role: "dialog", aria_modal: "true", aria_labelledby: "invf-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    h3 { id: "invf-title", class: "modal-title", "Mời bạn bè vào {name}" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                p { class: "modal-sub", "Bạn bè nhận một thẻ mời có ảnh {what}, tên và số thành viên trong tin nhắn mã hóa đầu cuối." }
                if let Some(l) = link.clone() {
                    div { class: "invite-link-box",
                        code { class: "invite-link-text", "{l}" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = l.clone(); move |_| platform::copy_to_clipboard(&l) }, CopyIcon {} "Sao chép" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = l.clone(); let n = name.clone(); move |_| platform::share_link(&n, &l) }, ShareIcon { size: 16 } "Chia sẻ" }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                div { class: "search-field",
                    SearchIcon { size: 16 }
                    input { r#type: "search", placeholder: "Tìm bạn bè", aria_label: "Tìm bạn bè", value: "{query}", oninput: move |e| query.set(e.value()) }
                }
                div { class: "pick-list",
                    if shown.is_empty() { div { class: "sidebar-empty", "Không có bạn bè phù hợp" } }
                    for f in shown {
                        {
                            let id = f.id.clone();
                            let on = picked.read().contains(&f.id);
                            let done = sent.read().contains(&f.id);
                            rsx! {
                                label { key: "{f.id}", class: if on { "pick-row active" } else { "pick-row" },
                                    input { r#type: "checkbox", checked: on, disabled: done,
                                        onchange: move |e| picked.with_mut(|p| { p.retain(|x| x != &id); if e.checked() { p.push(id.clone()); } }),
                                    }
                                    Avatar { name: f.username.clone(), avatar_url: f.avatar_url.clone(), size: 32 }
                                    span { class: "grow", "{f.username}" }
                                    if done { span { class: "member-sub", CheckIcon { size: 14 } " Đã gửi" } }
                                }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Xong" }
                    button { r#type: "button", class: "btn-primary", disabled: picked.read().is_empty() || code().is_none() || busy(), onclick: send,
                        "Gửi thẻ mời ({picked.read().len()})"
                    }
                }
            }
        }
    }
}
