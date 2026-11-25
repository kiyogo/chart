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
                    "invite_not_found" | "not_found" => "The invite link does not exist or has expired".into(),
                    "invite_expired" => "The invite link has expired".into(),
                    "link_join_disabled" => "The group has disabled join by link".into(),
                    _ => "Cannot open invite link".into(),
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
                    Err(_) => error.set(Some("Unable to join the community".into())),
                }
            } else {
                match groups::groups_join_by_invite(&c).await {
                    Ok(g) => on_joined.call(("group".into(), g.id)),
                    Err(e) if e.code == "join_request_pending" => pending.set(true),
                    Err(e) => error.set(Some(match e.code.as_str() {
                        "group_blocked" | "blocked_from_group" => "You cannot join this group".into(),
                        "link_join_disabled" => "The group has disabled join by link".into(),
                        _ => "Unable to join the group".into(),
                    })),
                }
            }
            busy.set(false);
        });
    };
    let what = if kind == "server" { "community" } else { "group" };
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content invite-dialog", role: "dialog", aria_modal: "true", aria_label: "Invitation to participate", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                button { r#type: "button", class: "icon-btn invite-close", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                if let Some(p) = preview() {
                    {
                        let banner = p.banner_url.clone().filter(|b| platform::safe_asset_path(b));
                        rsx! {
                            div { class: "invite-banner", style: if let Some(b) = banner.as_ref() { format!("background-image: url('{}');", platform::resolve_asset_url(b)) } else { String::new() } }
                            div { class: "invite-body",
                                div { class: "invite-avatar", Avatar { name: p.name.clone(), avatar_url: p.avatar_url.clone(), size: 72 } }
                                div { class: "invite-kicker", "You are invited to join {what}" }
                                h3 { class: "invite-name", "{p.name}" }
                                div { class: "invite-meta", "{p.member_count} members" if p.require_approval { " · Need group leader approval" } }
                                if let Some(d) = p.description.as_ref().filter(|d| !d.trim().is_empty()) { p { class: "invite-desc", "{d}" } }
                                if pending() {
                                    div { class: "status-badge success", "Request to join has been sent. You will join the group when approved." }
                                } else if p.is_member {
                                    button { r#type: "button", class: "btn-primary wide", onclick: { let (k, id) = (kind.clone(), p.id.clone()); move |_| on_joined.call((k.clone(), id.clone())) }, CheckIcon { size: 16 } "You are in {what} · Open" }
                                } else {
                                    button { r#type: "button", class: "btn-primary wide", disabled: busy(), onclick: join,
                                        if p.require_approval { "Submit a request to join" } else { "Tham gia {what}" }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(e) = error() {
                    div { class: "invite-body", h3 { class: "invite-name", "Unable to open invitation" } p { class: "invite-desc", "{e}" } }
                } else {
                    div { class: "invite-body", p { class: "invite-desc", role: "status", "Loading invitations" } }
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
                            error.set(Some("The group is turning off join by link. Turn it back on in Manage Groups › Join.".into()));
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
    let (k1, t1, n1, a1) = (kind.clone(), target_id.clone(), name.clone(), avatar_url.clone());
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
            code: c.clone(),
            name: n1.clone(),
            avatar: a1.clone(),
            banner: banner(),
            members: member_count,
            from: my_name.clone(),
        };
        let text = format!("invite:{}", serde_json::to_string(&card).unwrap_or_default());
        let me = my_id.clone();
        let targets = picked();
        let k_inv = k1.clone();
        let t_inv = t1.clone();
        let c_inv = c.clone();
        spawn(async move {
            for fid in targets {
                let _ = api::target_invites_create(&k_inv, &t_inv, &c_inv, &fid).await;
                match dm::send_encrypted_dm(&me, &fid, &text, None).await {
                    Ok(_) => sent.with_mut(|s| s.push(fid.clone())),
                    Err(_) => error.set(Some("Some invitations could not be sent".into())),
                }
            }
            picked.set(Vec::new());
            busy.set(false);
        });
    };
    let what = if kind == "server" { "community" } else { "group" };
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content", role: "dialog", aria_modal: "true", aria_labelledby: "invf-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    h3 { id: "invf-title", class: "modal-title", "Invite friends to {name}" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                p { class: "modal-sub", "Friends receive an invitation card with {what}'s photo, name, and member number in an end-to-end encrypted message." }
                if let Some(l) = link.clone() {
                    div { class: "invite-link-box",
                        code { class: "invite-link-text", "{l}" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = l.clone(); move |_| platform::copy_to_clipboard(&l) }, CopyIcon {} "Copy" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = l.clone(); let n = name.clone(); move |_| platform::share_link(&n, &l) }, ShareIcon { size: 16 } "Share" }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                div { class: "search-field",
                    SearchIcon { size: 16 }
                    input { r#type: "search", placeholder: "Find friends", aria_label: "Find friends", value: "{query}", oninput: move |e| query.set(e.value()) }
                }
                div { class: "pick-list",
                    if shown.is_empty() { div { class: "sidebar-empty", "No suitable friends" } }
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
                                    if done { span { class: "member-sub", CheckIcon { size: 14 } " Sent" } }
                                }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Done" }
                    button { r#type: "button", class: "btn-primary", disabled: picked.read().is_empty() || code().is_none() || busy(), onclick: send,
                        "Send invitation cards"
                    }
                }
            }
        }
    }
}
