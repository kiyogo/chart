//! Contacts Directory Hub ("Danh bạ") — Spec 167.
//!
//! Two-pane directory synchronized 100% with ChatW's theme tokens (Light, Ash,
//! Dark, Obsidian, custom accent, density, glass/scrim):
//!   - Left (`ContactsSidebarMenu`): search bar + quick actions + 4 category rows
//!     (Friends list, Groups and communities, Friend requests, Group and community invites).
//!   - Right (`ContactsMainPanel`): full dedicated workspace for the active category,
//!     including responsive 3-column invitation cards with "Withdraw request" / "Accept" / "Decline".

use std::collections::BTreeMap;

use chrono::{DateTime, Local};
use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        composer::fold,
        icons::{
            BackIcon, BlockIcon, CheckIcon, ChevronRightIcon, GroupIcon, MessageIcon, MoreIcon,
            PhoneIcon, PlusIcon, SearchIcon, ServerIcon, TrashIcon, UserIcon, UserPlusIcon,
            VideoIcon,
        },
        profile_card::{DecoratedAvatar, name_effect_class},
    },
    models::{
        DirectTargetInviteItem, DirectTargetInvitesResponse, FriendItem, GroupItem,
        PendingRequestItem, SentRequestItem, ServerItem, UserSearchResult,
    },
    services::profile_ext,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactsTab {
    FriendsList,
    GroupsCommunities,
    FriendRequests,
    TargetInvites,
}

/// Formats an ISO-8601 timestamp into compact `dd/MM` (e.g. `08/11`), matching the reference cards.
fn format_short_date(iso: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return dt.with_timezone(&Local).format("%d/%m").to_string();
    }
    if iso.len() >= 10 {
        let m = &iso[5..7];
        let d = &iso[8..10];
        return format!("{d}/{m}");
    }
    String::new()
}

fn friend_display_name(f: &FriendItem) -> String {
    profile_ext::nickname_of(&f.id)
        .or_else(|| f.display_name.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| f.username.clone())
}

fn alpha_bucket(name: &str) -> char {
    let folded = fold(name.trim());
    match folded.chars().next() {
        Some(c) if c.is_ascii_alphabetic() => c.to_ascii_uppercase(),
        _ => '#',
    }
}

/// Parse a pasted ChatW invite link or raw code into `(kind, code)`.
fn parse_invite_input(raw: &str) -> Option<(String, String)> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(idx) = s.find("/invite/") {
        let rest = &s[idx + 8..];
        let mut parts = rest.split('/').filter(|p| !p.is_empty());
        let k = parts.next()?;
        let code = parts.next()?.split(['?', '#']).next()?.trim();
        if code.is_empty() {
            return None;
        }
        let kind = if k == "s" || k == "server" { "server" } else { "group" };
        return Some((kind.to_string(), code.to_string()));
    }
    Some(("group".to_string(), s.to_string()))
}

// ============================================================================
// Crisp Vector Icons for Contacts Directory (matches app/src/components/icons.rs)
// ============================================================================

#[component]
fn ContactsListIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round",
            circle { cx: "9", cy: "7", r: "3.5" }
            path { d: "M3 19.5c0-3.2 2.7-5.5 6-5.5s6 2.3 6 5.5" }
            line { x1: "16", y1: "8", x2: "21", y2: "8" }
            line { x1: "16", y1: "12", x2: "21", y2: "12" }
            line { x1: "17.5", y1: "16", x2: "21", y2: "16" }
        }
    }
}

#[component]
fn ContactsGroupIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round",
            circle { cx: "12", cy: "7.5", r: "3" }
            path { d: "M7.5 19c0-2.6 2-4.5 4.5-4.5s4.5 1.9 4.5 4.5" }
            circle { cx: "5.5", cy: "10", r: "2.2" }
            path { d: "M2 18.5c0-1.9 1.4-3.3 3.3-3.5" }
            circle { cx: "18.5", cy: "10", r: "2.2" }
            path { d: "M22 18.5c0-1.9-1.4-3.3-3.3-3.5" }
        }
    }
}

#[component]
fn ContactsInviteUserIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round",
            circle { cx: "9.5", cy: "7.5", r: "3.5" }
            path { d: "M3.5 19.5c0-3.2 2.7-5.5 6-5.5s6 2.3 6 5.5" }
            path { d: "M16 9h5" }
            path { d: "M18.5 6.5L21 9l-2.5 2.5" }
        }
    }
}

#[component]
fn ContactsInviteGroupIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round",
            circle { cx: "9.5", cy: "8", r: "2.8" }
            path { d: "M5 19c0-2.4 2-4.2 4.5-4.2S14 16.6 14 19" }
            circle { cx: "4.5", cy: "11", r: "1.9" }
            path { d: "M2 18.5c0-1.6 1.1-2.8 2.6-3.1" }
            path { d: "M16 8.5h5" }
            path { d: "M18.5 6l2.5 2.5-2.5 2.5" }
        }
    }
}

#[component]
fn ContactsChatBubbleIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! {
        svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.75", stroke_linecap: "round", stroke_linejoin: "round",
            path { d: "M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" }
            line { x1: "8.5", y1: "10.5", x2: "15.5", y2: "10.5" }
            line { x1: "8.5", y1: "14", x2: "13", y2: "14" }
        }
    }
}

// ============================================================================
// Left Sidebar: Contacts Menu (Reference Image 1)
// ============================================================================

#[component]
pub fn ContactsSidebarMenu(
    active_tab: ContactsTab,
    friends: Vec<FriendItem>,
    groups: Vec<GroupItem>,
    servers: Vec<ServerItem>,
    pending_requests: Vec<PendingRequestItem>,
    sent_requests: Vec<SentRequestItem>,
    target_invites: DirectTargetInvitesResponse,
    friend_input: String,
    search_results: Vec<UserSearchResult>,
    on_select_tab: EventHandler<ContactsTab>,
    on_friend_input_change: EventHandler<String>,
    on_select_friend: EventHandler<FriendItem>,
    on_select_group: EventHandler<GroupItem>,
    on_select_server: EventHandler<ServerItem>,
    on_send_friend_request_by_id: EventHandler<String>,
    on_accept_friend_request: EventHandler<String>,
    on_open_create_group: EventHandler<()>,
) -> Element {
    let q = fold(friend_input.trim());
    let has_query = !q.is_empty();
    let friends_count = friends.len();
    let gc_count = groups.len() + servers.len();
    let req_badge = pending_requests.len();
    let invite_badge = target_invites.received.len();

    let matching_friends: Vec<FriendItem> = if has_query {
        friends
            .iter()
            .filter(|f| fold(&friend_display_name(f)).contains(&q) || fold(&f.username).contains(&q))
            .take(6)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let matching_groups: Vec<GroupItem> = if has_query {
        groups
            .iter()
            .filter(|g| fold(&g.name).contains(&q))
            .take(4)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let matching_servers: Vec<ServerItem> = if has_query {
        servers
            .iter()
            .filter(|s| fold(&s.name).contains(&q))
            .take(4)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let nav_items: [(ContactsTab, &'static str, usize, usize); 4] = [
        (ContactsTab::FriendsList, "Friends list", friends_count, 0),
        (ContactsTab::GroupsCommunities, "Groups and communities", gc_count, 0),
        (ContactsTab::FriendRequests, "Friend requests", pending_requests.len() + sent_requests.len(), req_badge),
        (ContactsTab::TargetInvites, "Group and community invites", target_invites.received.len() + target_invites.sent.len(), invite_badge),
    ];

    rsx! {
        div { class: "contacts-sidebar-search",
            div { class: "search-field",
                SearchIcon { size: 16 }
                input {
                    r#type: "search",
                    placeholder: "Search",
                    aria_label: "Search",
                    value: "{friend_input}",
                    oninput: move |e| on_friend_input_change.call(e.value()),
                }
            }
            button {
                r#type: "button",
                class: "icon-btn",
                title: "Add friend",
                aria_label: "Add friend",
                onmousedown: move |e| {
                    if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                        on_select_tab.call(ContactsTab::FriendRequests);
                    }
                },
                onclick: move |_| on_select_tab.call(ContactsTab::FriendRequests),
                UserPlusIcon { size: 18 }
            }
            button {
                r#type: "button",
                class: "icon-btn",
                title: "Create new group",
                aria_label: "Create new group",
                onclick: move |_| on_open_create_group.call(()),
                GroupIcon { size: 18 }
            }
        }

        div { class: "sidebar-scrollable contacts-sidebar-scroll",
            if has_query {
                if !matching_friends.is_empty() {
                    div { class: "sidebar-section-header", span { "Friends list" } }
                    for f in matching_friends {
                        {
                            let fc = f.clone();
                            let dname = friend_display_name(&f);
                            rsx! {
                                button {
                                    key: "csf-{f.id}",
                                    r#type: "button",
                                    class: "chat-item friend-row",
                                    onclick: move |_| on_select_friend.call(fc.clone()),
                                    Avatar { name: f.username.clone(), avatar_url: f.avatar_url.clone(), size: 36 }
                                    div { class: "chat-item-info",
                                        span { class: "chat-item-name", "data-no-i18n": "true", "{dname}" }
                                        span { class: "chat-item-preview", "data-no-i18n": "true", "@{f.username}" }
                                    }
                                }
                            }
                        }
                    }
                }
                if !matching_groups.is_empty() || !matching_servers.is_empty() {
                    div { class: "sidebar-section-header", span { "Groups and communities" } }
                    for g in matching_groups {
                        {
                            let gc = g.clone();
                            rsx! {
                                button {
                                    key: "csg-{g.id}",
                                    r#type: "button",
                                    class: "chat-item friend-row",
                                    onclick: move |_| on_select_group.call(gc.clone()),
                                    Avatar { name: g.name.clone(), avatar_url: g.avatar_url.clone(), size: 36 }
                                    div { class: "chat-item-info",
                                        span { class: "chat-item-name", "data-no-i18n": "true", "{g.name}" }
                                        span { class: "chat-item-preview", "{g.member_count} members" }
                                    }
                                }
                            }
                        }
                    }
                    for s in matching_servers {
                        {
                            let sc = s.clone();
                            rsx! {
                                button {
                                    key: "css-{s.id}",
                                    r#type: "button",
                                    class: "chat-item friend-row",
                                    onclick: move |_| on_select_server.call(sc.clone()),
                                    Avatar { name: s.name.clone(), avatar_url: s.icon_url.clone(), size: 36 }
                                    div { class: "chat-item-info",
                                        span { class: "chat-item-name", "data-no-i18n": "true", "{s.name}" }
                                        span { class: "chat-item-preview", "{s.member_count} members" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "sidebar-section-header", span { "Search results" } }
                if search_results.is_empty() {
                    div { class: "sidebar-empty", "User not found" }
                } else {
                    for user in search_results {
                        div { class: "request-card", key: "csr-{user.id}",
                            div { class: "request-user",
                                Avatar { name: user.username.clone(), avatar_url: user.avatar_url.clone(), size: 32 }
                                span { class: "chat-item-name", "data-no-i18n": "true", "{user.username}" }
                            }
                            if user.relationship == "friend" {
                                span { class: "btn-small-disabled", "Friends" }
                            } else if user.relationship == "pending_sent" {
                                span { class: "btn-small-disabled", "Sent" }
                            } else if user.relationship == "pending_received" {
                                button {
                                    r#type: "button",
                                    class: "btn-small-primary",
                                    onclick: {
                                        let uid = user.id.clone();
                                        let req_id = pending_requests.iter().find(|r| r.sender_id == uid).map(|r| r.id.clone());
                                        move |_| {
                                            if let Some(rid) = req_id.clone() {
                                                on_accept_friend_request.call(rid);
                                            }
                                        }
                                    },
                                    "Accept"
                                }
                            } else {
                                button {
                                    r#type: "button",
                                    class: "btn-small-primary",
                                    onclick: {
                                        let uid = user.id.clone();
                                        move |_| on_send_friend_request_by_id.call(uid.clone())
                                    },
                                    "Add friend"
                                }
                            }
                        }
                    }
                }
            } else {
                nav { class: "contacts-menu-list", aria_label: "Contacts",
                    for (tab, label, _total, badge) in nav_items {
                        button {
                            key: "{label}",
                            r#type: "button",
                            class: if active_tab == tab { "contacts-menu-item active" } else { "contacts-menu-item" },
                            aria_current: if active_tab == tab { "page" } else { "false" },
                            onmousedown: move |e| {
                                if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                                    crate::storage::msg_store::perf_mark_interaction("contacts:tab");
                                    on_select_tab.call(tab);
                                }
                            },
                            onclick: move |_| on_select_tab.call(tab),
                            span { class: "contacts-menu-icon",
                                match tab {
                                    ContactsTab::FriendsList => rsx! { ContactsListIcon { size: 20 } },
                                    ContactsTab::GroupsCommunities => rsx! { ContactsGroupIcon { size: 20 } },
                                    ContactsTab::FriendRequests => rsx! { ContactsInviteUserIcon { size: 20 } },
                                    ContactsTab::TargetInvites => rsx! { ContactsInviteGroupIcon { size: 20 } },
                                }
                            }
                            span { class: "contacts-menu-label", "{label}" }
                            if badge > 0 {
                                span { class: "chat-item-badge", "{badge}" }
                            }
                            span { class: "contacts-menu-chevron", ChevronRightIcon { size: 16 } }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Right Main Stage: Contacts Directory Workspace (Reference Image 2)
// ============================================================================

#[component]
pub fn ContactsMainPanel(
    active_tab: ContactsTab,
    friends: Vec<FriendItem>,
    groups: Vec<GroupItem>,
    servers: Vec<ServerItem>,
    pending_requests: Vec<PendingRequestItem>,
    sent_requests: Vec<SentRequestItem>,
    target_invites: DirectTargetInvitesResponse,
    blocked_ids: Vec<String>,
    friend_input: String,
    search_results: Vec<UserSearchResult>,
    add_friend_feedback: Option<(bool, String)>,
    is_adding_friend: bool,
    on_mobile_back: EventHandler<()>,
    on_select_tab: EventHandler<ContactsTab>,
    on_select_friend: EventHandler<FriendItem>,
    on_select_group: EventHandler<GroupItem>,
    on_select_server: EventHandler<ServerItem>,
    on_view_profile: EventHandler<String>,
    on_audio_call: EventHandler<String>,
    on_video_call: EventHandler<String>,
    on_remove_friend: EventHandler<String>,
    on_block: EventHandler<(String, bool)>,
    on_friend_input_change: EventHandler<String>,
    on_add_friend_submit: EventHandler<()>,
    on_send_friend_request_by_id: EventHandler<String>,
    on_accept_friend_request: EventHandler<String>,
    on_delete_friend_request: EventHandler<String>,
    on_accept_target_invite: EventHandler<DirectTargetInviteItem>,
    on_delete_target_invite: EventHandler<String>,
    on_open_invite_code: EventHandler<(String, String)>,
    on_open_create_group: EventHandler<()>,
    on_open_create_server: EventHandler<()>,
) -> Element {
    let mut friend_filter = use_signal(String::new);
    let mut friend_sort = use_signal(|| "az".to_string());
    let mut friend_menu_for = use_signal(|| Option::<String>::None);

    let mut gc_filter = use_signal(String::new);
    let mut gc_kind = use_signal(|| "all".to_string());
    let mut gc_sort = use_signal(|| "az".to_string());

    let mut invite_link_input = use_signal(String::new);

    let header_title = match active_tab {
        ContactsTab::FriendsList => "Friends list",
        ContactsTab::GroupsCommunities => "Groups and communities",
        ContactsTab::FriendRequests => "Friend requests",
        ContactsTab::TargetInvites => "Group and community invites",
    };

    rsx! {
        div { class: "contacts-stage",
            header { class: "contacts-header",
                button {
                    r#type: "button",
                    class: "icon-btn back-btn",
                    aria_label: "Back to contacts",
                    onclick: move |_| on_mobile_back.call(()),
                    BackIcon {}
                }
                span { class: "contacts-header-icon",
                    match active_tab {
                        ContactsTab::FriendsList => rsx! { ContactsListIcon { size: 20 } },
                        ContactsTab::GroupsCommunities => rsx! { ContactsGroupIcon { size: 20 } },
                        ContactsTab::FriendRequests => rsx! { ContactsInviteUserIcon { size: 20 } },
                        ContactsTab::TargetInvites => rsx! { ContactsInviteGroupIcon { size: 20 } },
                    }
                }
                h2 { class: "contacts-header-title", "{header_title}" }
                div { class: "contacts-header-actions",
                    match active_tab {
                        ContactsTab::FriendsList => rsx! {
                            button {
                                r#type: "button",
                                class: "btn-secondary contacts-header-btn",
                                onclick: move |_| on_select_tab.call(ContactsTab::FriendRequests),
                                UserPlusIcon { size: 16 }
                                span { "Add friend" }
                            }
                        },
                        ContactsTab::GroupsCommunities => rsx! {
                            button {
                                r#type: "button",
                                class: "btn-secondary contacts-header-btn",
                                onclick: move |_| on_open_create_group.call(()),
                                PlusIcon { size: 16 }
                                span { "Create new group" }
                            }
                            button {
                                r#type: "button",
                                class: "btn-secondary contacts-header-btn",
                                onclick: move |_| on_open_create_server.call(()),
                                ServerIcon { size: 16 }
                                span { "Create community" }
                            }
                        },
                        _ => rsx! {},
                    }
                }
            }

            div { class: "contacts-body",
                match active_tab {
                    // ========================================================
                    // 1. Friends list ("Danh sách bạn bè")
                    // ========================================================
                    ContactsTab::FriendsList => {
                        let fq = fold(friend_filter().trim());
                        let mut filtered: Vec<FriendItem> = friends
                            .iter()
                            .filter(|f| {
                                fq.is_empty()
                                    || fold(&friend_display_name(f)).contains(&fq)
                                    || fold(&f.username).contains(&fq)
                            })
                            .cloned()
                            .collect();
                        let sort_mode = friend_sort();
                        filtered.sort_by(|a, b| {
                            let na = fold(&friend_display_name(a));
                            let nb = fold(&friend_display_name(b));
                            if sort_mode == "za" { nb.cmp(&na) } else { na.cmp(&nb) }
                        });
                        let total_shown = filtered.len();
                        let mut map: BTreeMap<char, Vec<FriendItem>> = BTreeMap::new();
                        for f in filtered {
                            map.entry(alpha_bucket(&friend_display_name(&f))).or_default().push(f);
                        }
                        let grouped: Vec<(char, Vec<FriendItem>)> = if sort_mode == "za" {
                            map.into_iter().rev().collect()
                        } else {
                            map.into_iter().collect()
                        };
                        let open_menu_id = friend_menu_for();

                        rsx! {
                            div { class: "contacts-summary-bar",
                                span { class: "contacts-section-heading", "Friends ({total_shown})" }
                            }
                            div { class: "contacts-surface-card contacts-toolbar-card",
                                div { class: "contacts-toolbar",
                                    div { class: "search-field contacts-toolbar-search",
                                        SearchIcon { size: 16 }
                                        input {
                                            r#type: "search",
                                            placeholder: "Search friends",
                                            aria_label: "Search friends",
                                            value: "{friend_filter}",
                                            oninput: move |e| friend_filter.set(e.value()),
                                        }
                                    }
                                    select {
                                        class: "contacts-select",
                                        aria_label: "Sort friends",
                                        value: "{friend_sort}",
                                        onchange: move |e| friend_sort.set(e.value()),
                                        option { value: "az", "Sort: Name (A-Z)" }
                                        option { value: "za", "Sort: Name (Z-A)" }
                                    }
                                }

                                if grouped.is_empty() {
                                    div { class: "contacts-empty-box",
                                        ContactsListIcon { size: 36 }
                                        div { class: "contacts-empty-title",
                                            if friends.is_empty() { "No friends yet" } else { "User not found" }
                                        }
                                        if friends.is_empty() {
                                            button {
                                                r#type: "button",
                                                class: "btn-primary",
                                                onclick: move |_| on_select_tab.call(ContactsTab::FriendRequests),
                                                UserPlusIcon { size: 16 }
                                                span { "Add friend" }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "contacts-alpha-list",
                                        for (letter, bucket) in grouped {
                                            div { key: "alpha-{letter}", class: "contacts-alpha-group",
                                                div { class: "contacts-alpha-letter", "{letter}" }
                                                for friend in bucket {
                                                    {
                                                        let dname = friend_display_name(&friend);
                                                        let name_fx = name_effect_class(friend.name_effect.as_deref());
                                                        let is_blocked = blocked_ids.contains(&friend.id);
                                                        let is_menu_open = open_menu_for_id(&open_menu_id, &friend.id);
                                                        let f_open = friend.clone();
                                                        let f_msg = friend.clone();
                                                        let uid_call = friend.id.clone();
                                                        let uid_vcall = friend.id.clone();
                                                        let uid_menu = friend.id.clone();
                                                        let uid_prof = friend.id.clone();
                                                        let uid_rem = friend.id.clone();
                                                        let uid_blk = friend.id.clone();
                                                        let status_sub = match (friend.status_emoji.as_deref(), friend.status_text.as_deref()) {
                                                            (Some(e), Some(t)) if !t.trim().is_empty() => format!("{e} {t}"),
                                                            (None, Some(t)) if !t.trim().is_empty() => t.to_string(),
                                                            _ => format!("@{}", friend.username),
                                                        };
                                                        rsx! {
                                                            div { key: "fr-{friend.id}", class: "contacts-list-row",
                                                                button {
                                                                    r#type: "button",
                                                                    class: "contacts-list-row-main",
                                                                    onclick: move |_| on_select_friend.call(f_open.clone()),
                                                                    DecoratedAvatar {
                                                                        name: friend.username.clone(),
                                                                        avatar_url: friend.avatar_url.clone(),
                                                                        size: 44,
                                                                        decoration: friend.avatar_decoration.clone().filter(|d| d != "custom"),
                                                                        motion: friend.avatar_motion.clone(),
                                                                        in_list: true,
                                                                    }
                                                                    div { class: "contacts-list-row-info",
                                                                        div { class: "contacts-list-row-name {name_fx}", "data-no-i18n": "true", "{dname}" }
                                                                        div { class: "contacts-list-row-sub", "data-no-i18n": "true", "{status_sub}" }
                                                                    }
                                                                }
                                                                div { class: "contacts-list-row-actions",
                                                                    button {
                                                                        r#type: "button",
                                                                        class: "icon-btn",
                                                                        title: "Message",
                                                                        aria_label: "Message",
                                                                        onclick: move |_| on_select_friend.call(f_msg.clone()),
                                                                        MessageIcon { size: 18 }
                                                                    }
                                                                    if !is_blocked {
                                                                        button {
                                                                            r#type: "button",
                                                                            class: "icon-btn",
                                                                            title: "Voice call",
                                                                            aria_label: "Voice call",
                                                                            onclick: move |_| on_audio_call.call(uid_call.clone()),
                                                                            PhoneIcon { size: 18 }
                                                                        }
                                                                        button {
                                                                            r#type: "button",
                                                                            class: "icon-btn",
                                                                            title: "Video call",
                                                                            aria_label: "Video call",
                                                                            onclick: move |_| on_video_call.call(uid_vcall.clone()),
                                                                            VideoIcon { size: 18 }
                                                                        }
                                                                    }
                                                                    div { class: "contacts-row-more-wrap",
                                                                        button {
                                                                            r#type: "button",
                                                                            class: if is_menu_open { "icon-btn active" } else { "icon-btn" },
                                                                            title: "More options",
                                                                            aria_label: "More options",
                                                                            onclick: move |e| {
                                                                                e.stop_propagation();
                                                                                if friend_menu_for.peek().as_deref() == Some(uid_menu.as_str()) {
                                                                                    friend_menu_for.set(None);
                                                                                } else {
                                                                                    friend_menu_for.set(Some(uid_menu.clone()));
                                                                                }
                                                                            },
                                                                            MoreIcon { size: 18 }
                                                                        }
                                                                        if is_menu_open {
                                                                            button {
                                                                                r#type: "button",
                                                                                class: "menu-dismiss",
                                                                                tabindex: "-1",
                                                                                aria_label: "Close",
                                                                                onclick: move |_| friend_menu_for.set(None),
                                                                            }
                                                                            div { class: "cw-dropdown-menu contacts-row-menu", role: "menu",
                                                                                button {
                                                                                    r#type: "button",
                                                                                    role: "menuitem",
                                                                                    class: "cw-dropdown-item",
                                                                                    onclick: move |_| {
                                                                                        friend_menu_for.set(None);
                                                                                        on_view_profile.call(uid_prof.clone());
                                                                                    },
                                                                                    UserIcon { size: 16 }
                                                                                    span { "View profile" }
                                                                                }
                                                                                button {
                                                                                    r#type: "button",
                                                                                    role: "menuitem",
                                                                                    class: "cw-dropdown-item",
                                                                                    onclick: move |_| {
                                                                                        friend_menu_for.set(None);
                                                                                        on_block.call((uid_blk.clone(), !is_blocked));
                                                                                    },
                                                                                    BlockIcon { size: 16 }
                                                                                    span { if is_blocked { "Unblock" } else { "Block" } }
                                                                                }
                                                                                button {
                                                                                    r#type: "button",
                                                                                    role: "menuitem",
                                                                                    class: "cw-dropdown-item danger",
                                                                                    onclick: move |_| {
                                                                                        friend_menu_for.set(None);
                                                                                        on_remove_friend.call(uid_rem.clone());
                                                                                    },
                                                                                    TrashIcon { size: 16 }
                                                                                    span { "Remove friend" }
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
                            }
                        }
                    },

                    // ========================================================
                    // 2. Groups and communities ("Danh sách nhóm và cộng đồng")
                    // ========================================================
                    ContactsTab::GroupsCommunities => {
                        let q = fold(gc_filter().trim());
                        let kind = gc_kind();
                        let sort_mode = gc_sort();

                        #[derive(Clone)]
                        enum GcEntry {
                            Group(GroupItem),
                            Server(ServerItem),
                        }
                        impl GcEntry {
                            fn id(&self) -> &str {
                                match self { Self::Group(g) => &g.id, Self::Server(s) => &s.id }
                            }
                            fn name(&self) -> &str {
                                match self { Self::Group(g) => &g.name, Self::Server(s) => &s.name }
                            }
                            fn avatar(&self) -> Option<String> {
                                match self { Self::Group(g) => g.avatar_url.clone(), Self::Server(s) => s.icon_url.clone() }
                            }
                            fn members(&self) -> i64 {
                                match self { Self::Group(g) => g.member_count, Self::Server(s) => s.member_count }
                            }
                            fn is_server(&self) -> bool {
                                matches!(self, Self::Server(_))
                            }
                        }

                        let mut items: Vec<GcEntry> = Vec::new();
                        if kind == "all" || kind == "groups" {
                            for g in &groups {
                                if q.is_empty() || fold(&g.name).contains(&q) {
                                    items.push(GcEntry::Group(g.clone()));
                                }
                            }
                        }
                        if kind == "all" || kind == "communities" {
                            for s in &servers {
                                if q.is_empty() || fold(&s.name).contains(&q) {
                                    items.push(GcEntry::Server(s.clone()));
                                }
                            }
                        }
                        items.sort_by(|a, b| match sort_mode.as_str() {
                            "za" => fold(b.name()).cmp(&fold(a.name())),
                            "members" => b.members().cmp(&a.members()).then_with(|| fold(a.name()).cmp(&fold(b.name()))),
                            _ => fold(a.name()).cmp(&fold(b.name())),
                        });
                        let shown_count = items.len();

                        rsx! {
                            div { class: "contacts-summary-bar",
                                span { class: "contacts-section-heading", "Groups and communities ({shown_count})" }
                            }
                            div { class: "contacts-surface-card contacts-toolbar-card",
                                div { class: "contacts-toolbar",
                                    div { class: "search-field contacts-toolbar-search",
                                        SearchIcon { size: 16 }
                                        input {
                                            r#type: "search",
                                            placeholder: "Search groups and communities",
                                            aria_label: "Search groups and communities",
                                            value: "{gc_filter}",
                                            oninput: move |e| gc_filter.set(e.value()),
                                        }
                                    }
                                    div { class: "contacts-filter-pills", role: "tablist",
                                        for (val, label) in [("all", "All"), ("groups", "Groups"), ("communities", "Communities")] {
                                            button {
                                                key: "{val}",
                                                r#type: "button",
                                                role: "tab",
                                                aria_selected: "{gc_kind() == val}",
                                                class: if gc_kind() == val { "contacts-pill active" } else { "contacts-pill" },
                                                onclick: move |_| gc_kind.set(val.to_string()),
                                                "{label}"
                                            }
                                        }
                                    }
                                    select {
                                        class: "contacts-select",
                                        aria_label: "Sort groups and communities",
                                        value: "{gc_sort}",
                                        onchange: move |e| gc_sort.set(e.value()),
                                        option { value: "az", "Sort: Name (A-Z)" }
                                        option { value: "za", "Sort: Name (Z-A)" }
                                        option { value: "members", "Most members" }
                                    }
                                }

                                if items.is_empty() {
                                    div { class: "contacts-empty-box",
                                        ContactsGroupIcon { size: 36 }
                                        div { class: "contacts-empty-title", "No groups or communities yet" }
                                        div { class: "contacts-empty-actions",
                                            button {
                                                r#type: "button",
                                                class: "btn-primary",
                                                onclick: move |_| on_open_create_group.call(()),
                                                PlusIcon { size: 16 }
                                                span { "Create new group" }
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-secondary",
                                                onclick: move |_| on_open_create_server.call(()),
                                                ServerIcon { size: 16 }
                                                span { "Create community" }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "contacts-gc-list",
                                        for entry in items {
                                            {
                                                let e_key = format!("{}-{}", if entry.is_server() { "s" } else { "g" }, entry.id());
                                                let e_name = entry.name().to_string();
                                                let e_av = entry.avatar();
                                                let e_mem = entry.members();
                                                let is_srv = entry.is_server();
                                                let entry_click = entry.clone();
                                                rsx! {
                                                    button {
                                                        key: "{e_key}",
                                                        r#type: "button",
                                                        class: "contacts-list-row contacts-gc-row",
                                                        onclick: move |_| match &entry_click {
                                                            GcEntry::Group(g) => on_select_group.call(g.clone()),
                                                            GcEntry::Server(s) => on_select_server.call(s.clone()),
                                                        },
                                                        Avatar { name: e_name.clone(), avatar_url: e_av, size: 46 }
                                                        div { class: "contacts-list-row-info",
                                                            div { class: "contacts-gc-title-line",
                                                                span { class: "contacts-list-row-name", "data-no-i18n": "true", "{e_name}" }
                                                                span { class: if is_srv { "contacts-kind-tag community" } else { "contacts-kind-tag" },
                                                                    if is_srv { "Community" } else { "Group" }
                                                                }
                                                            }
                                                            div { class: "contacts-list-row-sub", "{e_mem} members" }
                                                        }
                                                        span { class: "contacts-gc-open",
                                                            span { "Open" }
                                                            ChevronRightIcon { size: 16 }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    // ========================================================
                    // 3. Friend requests ("Lời mời kết bạn") — Matches Image 2!
                    // ========================================================
                    ContactsTab::FriendRequests => {
                        let recv_count = pending_requests.len();
                        let sent_count = sent_requests.len();

                        rsx! {
                            // Inline Add-Friend bar
                            div { class: "contacts-surface-card contacts-add-box",
                                form {
                                    class: "contacts-add-form",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        on_add_friend_submit.call(());
                                    },
                                    div { class: "search-field contacts-toolbar-search",
                                        UserPlusIcon { size: 16 }
                                        input {
                                            r#type: "text",
                                            placeholder: "Find or enter the account name",
                                            aria_label: "Friend account name",
                                            value: "{friend_input}",
                                            oninput: move |e| on_friend_input_change.call(e.value()),
                                        }
                                    }
                                    button {
                                        r#type: "submit",
                                        class: "btn-primary contacts-add-submit",
                                        disabled: friend_input.trim().is_empty() || is_adding_friend,
                                        "Add friend"
                                    }
                                }
                                if let Some((is_err, msg)) = add_friend_feedback {
                                    div {
                                        class: if is_err { "status-badge error" } else { "status-badge success" },
                                        "{msg}"
                                    }
                                }
                                if !friend_input.trim().is_empty() {
                                    if search_results.is_empty() {
                                        div { class: "contacts-inline-empty", "User not found" }
                                    } else {
                                        div { class: "contacts-req-grid contacts-search-grid",
                                            for user in search_results {
                                                {
                                                    let uid_prof = user.id.clone();
                                                    let uid_add = user.id.clone();
                                                    let rec_req_id = pending_requests.iter().find(|r| r.sender_id == user.id).map(|r| r.id.clone());
                                                    let sent_req_id = sent_requests.iter().find(|r| r.receiver_id == user.id).map(|r| r.id.clone());
                                                    rsx! {
                                                        div { key: "sr-{user.id}", class: "contacts-req-card",
                                                            div { class: "contacts-req-card-top",
                                                                Avatar { name: user.username.clone(), avatar_url: user.avatar_url.clone(), size: 46 }
                                                                div { class: "contacts-req-card-meta",
                                                                    div { class: "contacts-req-card-name", "data-no-i18n": "true", "{user.username}" }
                                                                    div { class: "contacts-req-card-date", "data-no-i18n": "true", "@{user.username}" }
                                                                }
                                                                button {
                                                                    r#type: "button",
                                                                    class: "contacts-card-msg-btn",
                                                                    title: "View profile",
                                                                    aria_label: "View profile",
                                                                    onclick: move |_| on_view_profile.call(uid_prof.clone()),
                                                                    ContactsChatBubbleIcon { size: 18 }
                                                                }
                                                            }
                                                            div { class: "contacts-req-card-actions",
                                                                if user.relationship == "friend" {
                                                                    span { class: "btn-secondary contacts-card-full-btn disabled",
                                                                        CheckIcon { size: 15 }
                                                                        span { "Friends" }
                                                                    }
                                                                } else if user.relationship == "pending_sent" {
                                                                    if let Some(srid) = sent_req_id {
                                                                        button {
                                                                            r#type: "button",
                                                                            class: "btn-secondary contacts-card-full-btn",
                                                                            onclick: move |_| on_delete_friend_request.call(srid.clone()),
                                                                            "Withdraw request"
                                                                        }
                                                                    } else {
                                                                        span { class: "btn-secondary contacts-card-full-btn disabled", "Sent" }
                                                                    }
                                                                } else if user.relationship == "pending_received" {
                                                                    if let Some(rrid) = rec_req_id {
                                                                        {
                                                                            let r_dec = rrid.clone();
                                                                            let r_acc = rrid.clone();
                                                                            rsx! {
                                                                                button {
                                                                                    r#type: "button",
                                                                                    class: "btn-secondary contacts-card-half-btn",
                                                                                    onclick: move |_| on_delete_friend_request.call(r_dec.clone()),
                                                                                    "Decline"
                                                                                }
                                                                                button {
                                                                                    r#type: "button",
                                                                                    class: "btn-primary contacts-card-half-btn",
                                                                                    onclick: move |_| on_accept_friend_request.call(r_acc.clone()),
                                                                                    "Accept"
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    button {
                                                                        r#type: "button",
                                                                        class: "btn-primary contacts-card-full-btn",
                                                                        onclick: move |_| on_send_friend_request_by_id.call(uid_add.clone()),
                                                                        "Add friend"
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

                            // Received Friend Requests
                            section { class: "contacts-section",
                                h3 { class: "contacts-section-heading", "Received friend requests ({recv_count})" }
                                if pending_requests.is_empty() {
                                    div { class: "contacts-surface-card contacts-empty-inline",
                                        "No received friend requests"
                                    }
                                } else {
                                    div { class: "contacts-req-grid",
                                        for req in pending_requests {
                                            {
                                                let dname = req.sender_display_name
                                                    .clone()
                                                    .filter(|s| !s.trim().is_empty())
                                                    .unwrap_or_else(|| req.sender_username.clone());
                                                let date_str = format_short_date(&req.created_at);
                                                let rid_acc = req.id.clone();
                                                let rid_dec = req.id.clone();
                                                let uid_prof = req.sender_id.clone();
                                                rsx! {
                                                    div { key: "recv-{req.id}", class: "contacts-req-card",
                                                        div { class: "contacts-req-card-top",
                                                            Avatar { name: req.sender_username.clone(), avatar_url: req.sender_avatar_url.clone(), size: 46 }
                                                            div { class: "contacts-req-card-meta",
                                                                div { class: "contacts-req-card-name", "data-no-i18n": "true", "{dname}" }
                                                                div { class: "contacts-req-card-date", "{date_str}" }
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "contacts-card-msg-btn",
                                                                title: "View profile",
                                                                aria_label: "View profile",
                                                                onclick: move |_| on_view_profile.call(uid_prof.clone()),
                                                                ContactsChatBubbleIcon { size: 18 }
                                                            }
                                                        }
                                                        div { class: "contacts-req-card-actions",
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-secondary contacts-card-half-btn",
                                                                onclick: move |_| on_delete_friend_request.call(rid_dec.clone()),
                                                                "Decline"
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-primary contacts-card-half-btn",
                                                                onclick: move |_| on_accept_friend_request.call(rid_acc.clone()),
                                                                "Accept"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Sent Friend Requests (Exact Reference Image 2 card grid!)
                            section { class: "contacts-section",
                                h3 { class: "contacts-section-heading", "Sent friend requests ({sent_count})" }
                                if sent_requests.is_empty() {
                                    div { class: "contacts-surface-card contacts-empty-inline",
                                        "No sent friend requests"
                                    }
                                } else {
                                    div { class: "contacts-req-grid",
                                        for req in sent_requests {
                                            {
                                                let dname = req.receiver_display_name
                                                    .clone()
                                                    .filter(|s| !s.trim().is_empty())
                                                    .unwrap_or_else(|| req.receiver_username.clone());
                                                let date_str = format_short_date(&req.created_at);
                                                let rid_withdraw = req.id.clone();
                                                let uid_prof = req.receiver_id.clone();
                                                rsx! {
                                                    div { key: "sent-{req.id}", class: "contacts-req-card",
                                                        div { class: "contacts-req-card-top",
                                                            Avatar { name: req.receiver_username.clone(), avatar_url: req.receiver_avatar_url.clone(), size: 46 }
                                                            div { class: "contacts-req-card-meta",
                                                                div { class: "contacts-req-card-name", "data-no-i18n": "true", "{dname}" }
                                                                div { class: "contacts-req-card-date", "{date_str}" }
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "contacts-card-msg-btn",
                                                                title: "View profile",
                                                                aria_label: "View profile",
                                                                onclick: move |_| on_view_profile.call(uid_prof.clone()),
                                                                ContactsChatBubbleIcon { size: 18 }
                                                            }
                                                        }
                                                        div { class: "contacts-req-card-actions",
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-secondary contacts-card-full-btn",
                                                                onclick: move |_| on_delete_friend_request.call(rid_withdraw.clone()),
                                                                "Withdraw request"
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
                    },

                    // ========================================================
                    // 4. Group & Community Invites ("Lời mời vào nhóm và cộng đồng")
                    // ========================================================
                    ContactsTab::TargetInvites => {
                        let recv_len = target_invites.received.len();
                        let sent_len = target_invites.sent.len();

                        rsx! {
                            div { class: "contacts-surface-card contacts-add-box",
                                form {
                                    class: "contacts-add-form",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        if let Some((k, c)) = parse_invite_input(&invite_link_input()) {
                                            invite_link_input.set(String::new());
                                            on_open_invite_code.call((k, c));
                                        }
                                    },
                                    div { class: "search-field contacts-toolbar-search",
                                        ContactsInviteGroupIcon { size: 16 }
                                        input {
                                            r#type: "text",
                                            placeholder: "Paste invite link or code",
                                            aria_label: "Paste invite link or code",
                                            value: "{invite_link_input}",
                                            oninput: move |e| invite_link_input.set(e.value()),
                                        }
                                    }
                                    button {
                                        r#type: "submit",
                                        class: "btn-primary contacts-add-submit",
                                        disabled: invite_link_input().trim().is_empty(),
                                        "Join"
                                    }
                                }
                            }

                            section { class: "contacts-section",
                                h3 { class: "contacts-section-heading", "Received invitations ({recv_len})" }
                                if target_invites.received.is_empty() {
                                    div { class: "contacts-surface-card contacts-empty-inline",
                                        "No pending group or community invites"
                                    }
                                } else {
                                    div { class: "contacts-req-grid",
                                        for inv in target_invites.received {
                                            {
                                                let inviter = inv.peer_display_name
                                                    .clone()
                                                    .filter(|s| !s.trim().is_empty())
                                                    .unwrap_or_else(|| inv.peer_username.clone());
                                                let date_str = format_short_date(&inv.created_at);
                                                let is_srv = inv.kind == "server";
                                                let inv_acc = inv.clone();
                                                let inv_del = inv.id.clone();
                                                let inv_preview_k = inv.kind.clone();
                                                let inv_preview_c = inv.code.clone();
                                                rsx! {
                                                    div { key: "tinv-r-{inv.id}", class: "contacts-req-card",
                                                        div { class: "contacts-req-card-top",
                                                            Avatar { name: inv.target_name.clone(), avatar_url: inv.target_avatar_url.clone(), size: 46 }
                                                            div { class: "contacts-req-card-meta",
                                                                div { class: "contacts-gc-title-line",
                                                                    span { class: "contacts-req-card-name", "data-no-i18n": "true", "{inv.target_name}" }
                                                                    span { class: if is_srv { "contacts-kind-tag community" } else { "contacts-kind-tag" },
                                                                        if is_srv { "Community" } else { "Group" }
                                                                    }
                                                                }
                                                                div { class: "contacts-req-card-date",
                                                                    "{date_str} · {inv.member_count} members"
                                                                }
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "contacts-card-msg-btn",
                                                                title: "Open",
                                                                aria_label: "Open",
                                                                onclick: move |_| on_open_invite_code.call((inv_preview_k.clone(), inv_preview_c.clone())),
                                                                ContactsChatBubbleIcon { size: 18 }
                                                            }
                                                        }
                                                        div { class: "contacts-req-card-inviter", "Invited by {inviter}" }
                                                        div { class: "contacts-req-card-actions",
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-secondary contacts-card-half-btn",
                                                                onclick: move |_| on_delete_target_invite.call(inv_del.clone()),
                                                                "Decline"
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-primary contacts-card-half-btn",
                                                                onclick: move |_| on_accept_target_invite.call(inv_acc.clone()),
                                                                "Join"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            section { class: "contacts-section",
                                h3 { class: "contacts-section-heading", "Sent invitations ({sent_len})" }
                                if target_invites.sent.is_empty() {
                                    div { class: "contacts-surface-card contacts-empty-inline",
                                        "No sent invitations"
                                    }
                                } else {
                                    div { class: "contacts-req-grid",
                                        for inv in target_invites.sent {
                                            {
                                                let invitee = inv.peer_display_name
                                                    .clone()
                                                    .filter(|s| !s.trim().is_empty())
                                                    .unwrap_or_else(|| inv.peer_username.clone());
                                                let date_str = format_short_date(&inv.created_at);
                                                let is_srv = inv.kind == "server";
                                                let inv_del = inv.id.clone();
                                                let uid_prof = inv.peer_id.clone();
                                                rsx! {
                                                    div { key: "tinv-s-{inv.id}", class: "contacts-req-card",
                                                        div { class: "contacts-req-card-top",
                                                            Avatar { name: inv.target_name.clone(), avatar_url: inv.target_avatar_url.clone(), size: 46 }
                                                            div { class: "contacts-req-card-meta",
                                                                div { class: "contacts-gc-title-line",
                                                                    span { class: "contacts-req-card-name", "data-no-i18n": "true", "{inv.target_name}" }
                                                                    span { class: if is_srv { "contacts-kind-tag community" } else { "contacts-kind-tag" },
                                                                        if is_srv { "Community" } else { "Group" }
                                                                    }
                                                                }
                                                                div { class: "contacts-req-card-date", "{date_str} · @{invitee}" }
                                                            }
                                                            button {
                                                                r#type: "button",
                                                                class: "contacts-card-msg-btn",
                                                                title: "View profile",
                                                                aria_label: "View profile",
                                                                onclick: move |_| on_view_profile.call(uid_prof.clone()),
                                                                ContactsChatBubbleIcon { size: 18 }
                                                            }
                                                        }
                                                        div { class: "contacts-req-card-actions",
                                                            button {
                                                                r#type: "button",
                                                                class: "btn-secondary contacts-card-full-btn",
                                                                onclick: move |_| on_delete_target_invite.call(inv_del.clone()),
                                                                "Withdraw request"
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
                    },
                }
            }
        }
    }
}

fn open_menu_for_id(cur: &Option<String>, id: &str) -> bool {
    cur.as_deref() == Some(id)
}
