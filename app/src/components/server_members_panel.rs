//! Spec 017: Discord-style Server Member List sidebar + top header Search & Toggle bar
//! Shared by both Community servers and Enterprise workspaces/channels.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{CloseIcon, HashIcon, PanelRightIcon, SearchIcon},
        navigation::sidebar::format_display_label,
        profile_card::{name_effect_class, nameplate_style},
    },
    models::{ServerMemberItem, ServerMessageSearchHit},
    services::{
        api,
        enterprise::{self, SearchResultItem, module_label},
        social::ServerRole,
    },
};

pub fn member_display_name(m: &ServerMemberItem) -> String {
    crate::services::profile_ext::nickname_of(&m.user_id)
        .or_else(|| m.nickname.clone().filter(|s| !s.trim().is_empty()))
        .or_else(|| m.display_name.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| m.username.clone())
}

fn usable_role_color(color: &str) -> Option<String> {
    let c = color.trim();
    if c.is_empty() || c.eq_ignore_ascii_case("#000000") || c.eq_ignore_ascii_case("#99aab5") {
        None
    } else {
        Some(c.to_string())
    }
}

fn dot_class(presence: Option<&str>) -> &'static str {
    match presence.unwrap_or("offline") {
        "online" => "discord-status-dot online",
        "idle" => "discord-status-dot idle",
        "dnd" => "discord-status-dot dnd",
        _ => "discord-status-dot offline",
    }
}

struct MemberGroup {
    key: String,
    label: String,
    is_offline: bool,
    members: Vec<(ServerMemberItem, Option<String>)>,
}

fn build_member_groups(
    members: &[ServerMemberItem],
    roles: &[ServerRole],
    owner_id: &str,
) -> Vec<MemberGroup> {
    let mut custom_roles: Vec<&ServerRole> = roles
        .iter()
        .filter(|r| r.name != "@everyone")
        .collect();
    custom_roles.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.name.cmp(&b.name)));

    let mut role_buckets: Vec<(&ServerRole, Vec<(ServerMemberItem, Option<String>)>)> =
        custom_roles.iter().map(|r| (*r, Vec::new())).collect();
    let mut owner_bucket: Vec<(ServerMemberItem, Option<String>)> = Vec::new();
    let mut plain_bucket: Vec<(ServerMemberItem, Option<String>)> = Vec::new();
    let mut offline_bucket: Vec<(ServerMemberItem, Option<String>)> = Vec::new();

    for m in members {
        let pres = m.presence.as_deref().unwrap_or("offline");
        let is_off = matches!(pres, "offline" | "invisible");

        let matched_role = custom_roles.iter().find(|r| m.roles.contains(&r.id)).copied();
        let name_color = matched_role.and_then(|r| usable_role_color(&r.color));

        if is_off {
            offline_bucket.push((m.clone(), name_color));
            continue;
        }

        if let Some(r) = matched_role {
            if let Some((_, list)) = role_buckets.iter_mut().find(|(rb, _)| rb.id == r.id) {
                list.push((m.clone(), name_color));
            }
        } else if m.user_id == owner_id {
            owner_bucket.push((m.clone(), Some("#7C3AED".to_string())));
        } else {
            plain_bucket.push((m.clone(), None));
        }
    }

    let sort_bucket = |list: &mut Vec<(ServerMemberItem, Option<String>)>| {
        list.sort_by(|(a, _), (b, _)| {
            member_display_name(a)
                .to_lowercase()
                .cmp(&member_display_name(b).to_lowercase())
        });
    };

    let mut out = Vec::new();

    if !owner_bucket.is_empty() {
        sort_bucket(&mut owner_bucket);
        out.push(MemberGroup {
            key: "__owner__".into(),
            label: format!("Chủ sở hữu {}", owner_bucket.len()),
            is_offline: false,
            members: owner_bucket,
        });
    }

    for (r, mut list) in role_buckets {
        if list.is_empty() {
            continue;
        }
        sort_bucket(&mut list);
        out.push(MemberGroup {
            key: format!("role:{}", r.id),
            label: format!("{} {}", r.name, list.len()),
            is_offline: false,
            members: list,
        });
    }

    if !plain_bucket.is_empty() {
        sort_bucket(&mut plain_bucket);
        out.push(MemberGroup {
            key: "__members__".into(),
            label: format!("Thành viên {}", plain_bucket.len()),
            is_offline: false,
            members: plain_bucket,
        });
    }

    if !offline_bucket.is_empty() {
        sort_bucket(&mut offline_bucket);
        out.push(MemberGroup {
            key: "__offline__".into(),
            label: format!("Ngoại tuyến {}", offline_bucket.len()),
            is_offline: true,
            members: offline_bucket,
        });
    }

    out
}

/// Top-right Discord-style tools in server channel & enterprise headers:
/// `[PanelRight toggle] [Tìm kiếm {server_name}]` with live dropdown hits.
#[component]
pub fn ServerHeaderTools(
    server_id: String,
    server_name: String,
    is_enterprise: bool,
    members: Vec<ServerMemberItem>,
    members_open: bool,
    on_toggle_members: EventHandler<()>,
    on_open_profile: EventHandler<String>,
    on_jump_channel_msg: EventHandler<(String, String)>,
    #[props(default)] on_open_ent_module: Option<EventHandler<String>>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut msg_hits = use_signal(Vec::<ServerMessageSearchHit>::new);
    let mut ent_hits = use_signal(Vec::<SearchResultItem>::new);
    let mut busy = use_signal(|| false);
    let mut req_seq = use_signal(|| 0u64);
    let mut focused = use_signal(|| false);

    let srv_watch = server_id.clone();
    use_effect(use_reactive!(|srv_watch| {
        let _ = srv_watch;
        query.set(String::new());
        msg_hits.set(Vec::new());
        ent_hits.set(Vec::new());
        focused.set(false);
    }));

    let q_trim = query().trim().to_string();
    let q_low = q_trim.to_lowercase();
    let matched_members: Vec<ServerMemberItem> = if q_low.is_empty() {
        Vec::new()
    } else {
        members
            .iter()
            .filter(|m| {
                let dn = member_display_name(m).to_lowercase();
                dn.contains(&q_low) || m.username.to_lowercase().contains(&q_low)
            })
            .take(5)
            .cloned()
            .collect()
    };

    let no_members = matched_members.is_empty();
    let show_popover = focused() && !q_trim.is_empty();

    rsx! {
        div { class: "header-server-tools",
            button {
                r#type: "button",
                class: if members_open { "icon-btn active" } else { "icon-btn" },
                title: if members_open { "Ẩn danh sách thành viên" } else { "Hiện danh sách thành viên" },
                aria_label: "Danh sách thành viên",
                aria_pressed: "{members_open}",
                onclick: move |_| on_toggle_members.call(()),
                PanelRightIcon { size: 20 }
            }

            div { class: "header-server-search-wrap",
                if show_popover {
                    button {
                        r#type: "button",
                        class: "header-search-backdrop",
                        tabindex: "-1",
                        aria_label: "Đóng tìm kiếm",
                        onclick: move |_| focused.set(false),
                    }
                }
                div { class: "header-server-search",
                    input {
                        r#type: "search",
                        class: "header-server-search-input",
                        placeholder: "Tìm kiếm {server_name}",
                        aria_label: "Tìm kiếm trong {server_name}",
                        value: "{query}",
                        onfocus: move |_| focused.set(true),
                        onkeydown: move |e| {
                            if e.key() == Key::Escape {
                                focused.set(false);
                            }
                        },
                        oninput: move |e| {
                            let v = e.value();
                            query.set(v.clone());
                            focused.set(true);
                            let t = v.trim().to_string();
                            let next = *req_seq.peek() + 1;
                            req_seq.set(next);
                            if t.is_empty() {
                                msg_hits.set(Vec::new());
                                ent_hits.set(Vec::new());
                                busy.set(false);
                                return;
                            }
                            busy.set(true);
                            let sid = server_id.clone();
                            spawn(async move {
                                crate::platform::sleep_ms(160).await;
                                if *req_seq.peek() != next {
                                    return;
                                }
                                if let Ok(mh) = api::servers_search_messages(&sid, &t).await {
                                    if *req_seq.peek() == next {
                                        msg_hits.set(mh);
                                    }
                                }
                                if is_enterprise {
                                    if let Ok(eh) = enterprise::search(&sid, &t).await {
                                        if *req_seq.peek() == next {
                                            ent_hits.set(eh);
                                        }
                                    }
                                } else {
                                    ent_hits.set(Vec::new());
                                }
                                if *req_seq.peek() == next {
                                    busy.set(false);
                                }
                            });
                        },
                    }
                    if !q_trim.is_empty() {
                        button {
                            r#type: "button",
                            class: "header-search-clear",
                            aria_label: "Xóa từ khóa",
                            onclick: move |_| {
                                query.set(String::new());
                                msg_hits.set(Vec::new());
                                ent_hits.set(Vec::new());
                                focused.set(false);
                            },
                            CloseIcon { size: 13 }
                        }
                    } else {
                        span { class: "header-search-icon", aria_hidden: "true", SearchIcon { size: 15 } }
                    }
                }

                if show_popover {
                    div { class: "header-search-popover", role: "listbox", aria_label: "Kết quả tìm kiếm máy chủ",
                        if !matched_members.is_empty() {
                            div { class: "header-search-group", "Thành viên" }
                            for m in matched_members {
                                {
                                    let uid = m.user_id.clone();
                                    let dn = member_display_name(&m);
                                    rsx! {
                                        button {
                                            key: "sm-{m.user_id}",
                                            r#type: "button",
                                            class: "header-search-hit",
                                            onclick: move |_| {
                                                focused.set(false);
                                                on_open_profile.call(uid.clone());
                                            },
                                            Avatar { name: m.username.clone(), avatar_url: m.avatar_url.clone(), size: 24 }
                                            div { class: "header-search-hit-body",
                                                span { class: "header-search-hit-title", "{dn}" }
                                                span { class: "header-search-hit-sub", "@{m.username}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !msg_hits().is_empty() {
                            div { class: "header-search-group", "Tin nhắn kênh" }
                            for h in msg_hits().into_iter().take(8) {
                                {
                                    let cid = h.channel_id.clone();
                                    let mid = h.message_id.clone();
                                    rsx! {
                                        button {
                                            key: "sh-{h.message_id}",
                                            r#type: "button",
                                            class: "header-search-hit",
                                            onclick: move |_| {
                                                focused.set(false);
                                                on_jump_channel_msg.call((cid.clone(), mid.clone()));
                                            },
                                            HashIcon { size: 15 }
                                            div { class: "header-search-hit-body",
                                                span { class: "header-search-hit-title", "{h.content}" }
                                                span { class: "header-search-hit-sub", "#{format_display_label(&h.channel_name)} · {h.sender_name}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_enterprise && !ent_hits().is_empty() {
                            div { class: "header-search-group", "Không gian doanh nghiệp" }
                            for eh in ent_hits().into_iter().take(6) {
                                {
                                    let mod_k = eh.module.clone();
                                    rsx! {
                                        button {
                                            key: "eh-{eh.kind}-{eh.id}",
                                            r#type: "button",
                                            class: "header-search-hit",
                                            onclick: move |_| {
                                                focused.set(false);
                                                if let Some(h) = on_open_ent_module {
                                                    h.call(mod_k.clone());
                                                }
                                            },
                                            div { class: "header-search-hit-body",
                                                span { class: "header-search-hit-title", "{eh.title}" }
                                                span { class: "header-search-hit-sub", "{module_label(&eh.module)} · {eh.subtitle}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !busy() && no_members && msg_hits().is_empty() && ent_hits().is_empty() {
                            div { class: "header-search-empty", "Không tìm thấy kết quả phù hợp" }
                        } else if busy() && msg_hits().is_empty() {
                            div { class: "header-search-empty", "Đang tìm kiếm" }
                        }
                    }
                }
            }
        }
    }
}

/// Right-hand Discord-style member roster panel grouped by roles & online/offline status.
#[component]
pub fn ServerMembersPanel(
    owner_id: String,
    members: Vec<ServerMemberItem>,
    roles: Vec<ServerRole>,
    on_profile_click: EventHandler<(String, f64, f64)>,
    on_close: EventHandler<()>,
) -> Element {
    let groups = build_member_groups(&members, &roles, &owner_id);

    rsx! {
        button {
            r#type: "button",
            class: "drawer-backdrop",
            aria_label: "Đóng danh sách thành viên",
            tabindex: "-1",
            onclick: move |_| on_close.call(()),
        }
        aside { class: "details-panel server-members-panel", aria_label: "Danh sách thành viên máy chủ",
            div { class: "server-members-scroll",
                if members.is_empty() {
                    div { class: "sidebar-empty", "Chưa có thành viên nào" }
                } else {
                    for g in groups {
                        div { key: "{g.key}", class: if g.is_offline { "discord-role-group offline" } else { "discord-role-group" },
                            div { class: "discord-role-header", "{g.label}" }
                            for (m, name_color) in g.members {
                                {
                                    let shown = member_display_name(&m);
                                    let (np_cls, np_style) = nameplate_style(m.nameplate.as_deref(), m.custom_nameplate_url.as_deref());
                                    let fx_cls = name_effect_class(m.name_effect.as_deref());
                                    let status_line = match (m.status_emoji.as_deref(), m.status_text.as_deref()) {
                                        (Some(e), Some(t)) => format!("{e} {t}").trim().to_string(),
                                        (Some(e), None) => e.trim().to_string(),
                                        (None, Some(t)) => t.trim().to_string(),
                                        (None, None) => String::new(),
                                    };
                                    let dot = dot_class(m.presence.as_deref());
                                    let uid = m.user_id.clone();
                                    let color_style = name_color.map(|c| format!("color: {c};")).unwrap_or_default();
                                    let is_owner_row = m.user_id == owner_id;
                                    let row_cls = if np_cls.is_empty() {
                                        "discord-member-row".to_string()
                                    } else {
                                        format!("discord-member-row {np_cls}")
                                    };
                                    rsx! {
                                        button {
                                            key: "{m.user_id}",
                                            r#type: "button",
                                            class: "{row_cls}",
                                            style: "{np_style}",
                                            onclick: move |evt: MouseEvent| {
                                                let c = evt.client_coordinates();
                                                on_profile_click.call((uid.clone(), c.x, c.y));
                                            },
                                            span { class: "discord-member-avatar",
                                                Avatar { name: m.username.clone(), avatar_url: m.avatar_url.clone(), size: 32 }
                                                span { class: "{dot}", aria_hidden: "true" }
                                            }
                                            div { class: "discord-member-meta",
                                                div { class: "discord-member-name-row",
                                                    span {
                                                        class: "discord-member-name {fx_cls}",
                                                        style: "{color_style}",
                                                        "{shown}"
                                                    }
                                                    if is_owner_row {
                                                        span { class: "discord-owner-crown", title: "Chủ sở hữu", aria_label: "Chủ sở hữu", "👑" }
                                                    }
                                                }
                                                if !status_line.is_empty() {
                                                    div { class: "discord-member-status", "{status_line}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
