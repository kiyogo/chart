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
        "owner" => "Group leader",
        "admin" => "Group deputy",
        _ => "Member",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Perms,
    Join,
    Members,
    Public,
    Plugins,
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
                    msg.set(Some("Saved".into()));
                }
                Err(e) => msg.set(Some(match e.code.as_str() {
                    "insufficient_permissions" | "only_owner" => "Only the group leader or deputy group leader can change the settings".into(),
                    _ => "Unable to save".into(),
                })),
            }
        });
    };

    let perm_rows: [(&str, &str, bool); 7] = [
        ("perm_send", "Send a message", st().perm_send),
        ("perm_poll", "Create a new vote", st().perm_poll),
        ("perm_note", "Create new notes", st().perm_note),
        ("perm_reminder", "Create a new appointment reminder", st().perm_reminder),
        ("perm_pin", "Pin message", st().perm_pin),
        ("perm_rename", "Rename the group", st().perm_rename),
        ("perm_avatar", "Change group avatar photo", st().perm_avatar),
    ];

    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content wide manage-dialog", role: "dialog", aria_modal: "true", aria_labelledby: "gm-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    Avatar { name: group.name.clone(), avatar_url: group.avatar_url.clone(), size: 40 }
                    div { class: "modal-head-text",
                        h3 { id: "gm-title", class: "modal-title", "Manage group" }
                        div { class: "modal-sub", "{group.name} · you are {role_label(&my_role)}" }
                    }
                    button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                div { class: "sidebar-filters", role: "tablist",
                    for (tb, label) in [(Tab::Perms, "Membership rights"), (Tab::Join, "Join"), (Tab::Members, "Team leader & deputy"), (Tab::Public, "Public & blocked"), (Tab::Plugins, "Plugin & Webhook")] {
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
                            p { class: "settings-group-desc", "Allow group members. The team leader and deputy team leader always get everything done." }
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
                                    "Mark messages from team leader/deputy"
                                    span { class: "settings-row-desc", "Messages from group leaders and deputy leaders are highlighted" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().highlight_admins, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "highlight_admins": e.checked() })) },
                                }
                            }
                        },
                        Tab::Join => rsx! {
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    "Approve new members"
                                    span { class: "settings-row-desc", "People joining by link must wait for approval from the group leader or deputy" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().require_approval, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "require_approval": e.checked() })) },
                                }
                            }
                            label { class: "toggle-row settings-row",
                                span { class: "grow",
                                    "Allows using link to join group"
                                    span { class: "settings-row-desc", "Turn off to disable any existing invite links" }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().allow_link_join, disabled: !is_admin,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "allow_link_join": e.checked() })) },
                                }
                            }
                            if st().allow_link_join {
                                div { class: "invite-link-box",
                                    LinkIcon { size: 16 }
                                    code { class: "invite-link-text", if let Some(l) = link() { "{l}" } else { "Creating link" } }
                                    if let Some(l) = link() {
                                        {
                                            let l1 = l.clone();
                                            let l2 = l.clone();
                                            let name = group.name.clone();
                                            rsx! {
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| { platform::copy_to_clipboard(&l1); msg.set(Some("Link copied".into())); }, CopyIcon {} "Copy" }
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::share_link(&name, &l2), ShareIcon { size: 16 } "Share" }
                                            }
                                        }
                                    }
                                    if is_admin {
                                        button { r#type: "button", class: "btn-ghost", title: "The old link will no longer work",
                                            onclick: { let g = gid.clone(); move |_| { let g = g.clone(); spawn(async move {
                                                if let Ok(code) = social::group_invite_regenerate(&g).await {
                                                    link.set(Some(format!("{}/invite/g/{}", platform::app_origin().await, code)));
                                                    msg.set(Some("Link changed. The old link is no longer available.".into()));
                                                }
                                            }); } },
                                            RefreshIcon { size: 16 } "Change link"
                                        }
                                    }
                                }
                            }
                            div { class: "settings-group-title", "Request to join" }
                            if requests.read().is_empty() {
                                div { class: "sidebar-empty", "There are no requirements" }
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
                                                }); }, CheckIcon { size: 16 } "Browse" }
                                                button { r#type: "button", class: "btn-ghost", onclick: move |_| { let (g, u) = (g2.clone(), u2.clone()); spawn(async move {
                                                    if social::group_decide_request(&g, &u, false).await.is_ok() { requests.with_mut(|l| l.retain(|x| x.user_id != u)); }
                                                }); }, "Refuse" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Tab::Members => rsx! {
                            p { class: "settings-group-desc", "The group leader has full authority and appoints group deputies. Group deputy manages members, approves new people and pins messages." }
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
                                                div { class: "member-name", "{m.username}" if is_me { span { class: "member-sub", " · you" } } }
                                                div { class: "member-sub", if role == "owner" { CrownIcon { size: 12 } } " {role_label(&role)}" }
                                            }
                                            if is_owner && !is_me && role != "owner" {
                                                button { r#type: "button", class: "btn-ghost",
                                                    onclick: move |_| { let (g, u) = (g1.clone(), u1.clone()); let next = if role == "admin" { "member" } else { "admin" }; spawn(async move {
                                                        match groups::groups_update_member_role(&g, &u, next).await {
                                                            Ok(_) => { on_members_changed.call(()); msg.set(Some(if next == "admin" { "Appointed group deputy".into() } else { "Removed group deputy rights".into() })); }
                                                            Err(_) => msg.set(Some("Roles cannot be changed".into())),
                                                        }
                                                    }); },
                                                    if m.role == "admin" { "Remove team leader" } else { "Appoint group deputy" }
                                                }
                                                button { r#type: "button", class: "btn-ghost", title: "You will become the group's deputy",
                                                    onclick: move |_| { let (g, u) = (g2.clone(), u2.clone()); spawn(async move {
                                                        match groups::groups_update_member_role(&g, &u, "owner").await {
                                                            Ok(_) => { on_members_changed.call(()); msg.set(Some("Team leader position has been transferred".into())); }
                                                            Err(_) => msg.set(Some("Rights cannot be transferred".into())),
                                                        }
                                                    }); },
                                                    CrownIcon { size: 14 } "Change group leader"
                                                }
                                            }
                                            if is_admin && !is_me && m.role == "member" {
                                                button { r#type: "button", class: "icon-btn danger", title: "Remove from group", aria_label: "Remove {m.username} from the group",
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
                                    GlobeIcon { size: 16 } " Public group"
                                    span { class: "settings-row-desc",
                                        match (st().is_public, st().verification_status.as_str()) {
                                            (false, _) => "Only people with the invite link can join",
                                            (true, "verified") => "Verified · shown in Search for everyone",
                                            (true, "rejected") => "Verification denied · not showing up in Search",
                                            _ => "Waiting for ChatW team to verify before appearing in Search",
                                        }
                                    }
                                }
                                input { r#type: "checkbox", class: "switch", checked: st().is_public, disabled: !is_owner,
                                    onchange: { let s2 = save.clone(); move |e: Event<FormData>| s2(json!({ "is_public": e.checked() })) },
                                }
                            }
                            div { class: "settings-group-title", "Group cover photo" }
                            div { class: "inline-form",
                                input { id: "group-banner-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                                    onchange: { let s2 = save.clone(); move |_| { let s3 = s2.clone(); spawn(async move {
                                        match platform::read_selected_file_raw("group-banner-file").await {
                                            Ok((bytes, mime)) => match crate::services::library::upload_media(bytes, &mime).await {
                                                Ok(url) => s3(json!({ "banner_url": url })),
                                                Err(_) => msg.set(Some("Unable to upload cover photo".into())),
                                            },
                                            Err(e) if e == "no_file" => {}
                                            Err(e) if e == "svg_not_allowed" => msg.set(Some("SVG images are not supported".into())),
                                            Err(e) if e == "too_large" => msg.set(Some("Photo larger than 8 MB".into())),
                                            Err(_) => msg.set(Some("Unable to process cover photo".into())),
                                        }
                                    }); } },
                                }
                                if is_admin {
                                    label { r#for: "group-banner-file", class: "btn-secondary", role: "button", tabindex: "0", "Download cover photo" }
                                }
                                if let Some(b) = st().banner_url.filter(|b| platform::safe_asset_path(b)) {
                                    span { class: "banner-preview", style: "background-image: url('{platform::resolve_asset_url(&b)}');" }
                                }
                            }
                            div { class: "danger-zone",
                                div { class: "settings-group-title", ShieldIcon { size: 16 } " Block groups" }
                                p { class: "settings-group-desc", "Leave the group and no one can add you to this group again, even through the invite link." }
                                button { r#type: "button", class: "btn-danger", onclick: move |_| confirm_block.set(true), BlockIcon { size: 16 } "Block this group" }
                            }
                        },
                        Tab::Plugins => rsx! {
                            crate::components::plugin_hub::ScopedPluginAndWebhookTab {
                                scope_type: "group".to_string(),
                                scope_id: gid.clone(),
                                channels: Vec::new(),
                                can_manage: is_admin,
                            }
                        },
                    }
                }
            }
        }
        if confirm_block() {
            div { class: "modal-overlay", onclick: move |_| confirm_block.set(false),
                div { class: "modal-content compact", role: "alertdialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                    h3 { class: "modal-title", "Block group {group.name}?" }
                    p { class: "modal-sub", if is_owner { "You are the team leader: please transfer team leader rights before blocking." } else { "You will leave the group immediately and cannot be added again." } }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_block.set(false), "Cancel" }
                        if !is_owner {
                            button { r#type: "button", class: "btn-danger",
                                onclick: { let g = gid.clone(); move |_| { let g = g.clone(); confirm_block.set(false); spawn(async move {
                                    match social::group_block(&g).await {
                                        Ok(_) => on_blocked.call(()),
                                        Err(_) => msg.set(Some("The group cannot be blocked".into())),
                                    }
                                }); } },
                                BlockIcon { size: 16 } "Block groups"
                            }
                        }
                    }
                }
            }
        }
    }
}
