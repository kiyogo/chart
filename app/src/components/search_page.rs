//! Full-page search: one bar that searches everything. Your own
//! conversations, friends, groups and communities first; then people on
//! ChatW, and verified public communities, groups, bots and Kinh doanh pages.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        composer::fold,
        icons::{CompassIcon, GroupIcon, PluginIcon, SearchIcon, StoreIcon, UserPlusIcon},
    },
    models::{BotItem, DirectoryEntry, DirectoryResults, FriendItem, GroupItem, ServerItem, UserSearchResult},
    platform,
    services::{api, social},
    storage::msg_store,
};
use chatw_msg_store::{ScopeKind, SearchCoverageState, SearchFilters, SearchHit};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    All,
    Chats,
    People,
    Communities,
    Groups,
    Bots,
    Business,
}

#[component]
pub fn SearchPage(
    friends: Vec<FriendItem>,
    groups: Vec<GroupItem>,
    servers: Vec<ServerItem>,
    on_open_friend: EventHandler<FriendItem>,
    on_open_group: EventHandler<GroupItem>,
    on_open_server: EventHandler<ServerItem>,
    on_open_bot: EventHandler<BotItem>,
    /// Something was joined: refresh lists.
    on_joined: EventHandler<()>,
    on_add_friend: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let tab = use_signal(|| Tab::All);
    let mut dir = use_signal(DirectoryResults::default);
    let mut people = use_signal(Vec::<UserSearchResult>::new);
    let mut msg_hits = use_signal(Vec::<SearchHit>::new);
    let mut msg_coverage = use_signal(|| SearchCoverageState::NotDownloaded);
    let mut msg_latency_ms = use_signal(|| 0.0f64);
    let mut loading = use_signal(|| false);
    let mut seq = use_signal(|| 0u64);
    let mut status = use_signal(|| Option::<String>::None);
    let mut origin = use_signal(String::new);

    use_effect(move || {
        let _ = dioxus::document::eval("setTimeout(() => document.getElementById('global-search-input')?.focus(), 50);");
        spawn(async move {
            origin.set(platform::app_origin().await);
        });
    });
    // Debounced local encrypted Vietnamese message search + remote directory/people search.
    use_effect(move || {
        let q = query().trim().to_string();
        // peek: reading `seq` reactively here would re-run this effect forever.
        let my = *seq.peek() + 1;
        seq.set(my);
        spawn(async move {
            if q.chars().count() >= 2 {
                let uid = api::auth_me_cached_id().unwrap_or_default();
                let (resp, lat) = msg_store::search_local_messages(&uid, &q, SearchFilters::default(), 20).await;
                if *seq.peek() == my {
                    msg_hits.set(resp.hits);
                    msg_coverage.set(resp.coverage);
                    msg_latency_ms.set(lat);
                }
            } else {
                msg_hits.set(Vec::new());
            }
            platform::sleep_ms(220).await;
            if *seq.peek() != my {
                return;
            }
            loading.set(true);
            if let Ok(d) = social::directory_search(&q).await {
                if *seq.peek() == my {
                    dir.set(d);
                }
            }
            if q.chars().count() >= 2 {
                if let Ok(p) = api::users_search(&q).await {
                    if *seq.peek() == my {
                        people.set(p);
                    }
                }
            } else {
                people.set(Vec::new());
            }
            loading.set(false);
        });
    });

    let q = fold(query().trim());
    let cur = tab();
    let show = |t: Tab| cur == Tab::All || cur == t;
    let my_friends: Vec<FriendItem> = if q.is_empty() {
        Vec::new()
    } else {
        friends
            .iter()
            .filter(|f| fold(&f.username).contains(&q) || f.display_name.as_deref().is_some_and(|d| fold(d).contains(&q)))
            .cloned()
            .collect()
    };
    let my_groups: Vec<GroupItem> = if q.is_empty() { Vec::new() } else { groups.iter().filter(|g| fold(&g.name).contains(&q)).cloned().collect() };
    let my_servers: Vec<ServerItem> = if q.is_empty() { Vec::new() } else { servers.iter().filter(|s| fold(&s.name).contains(&q)).cloned().collect() };
    let d = dir();
    let base = origin();

    rsx! {
        div { class: "search-page",
            div { class: "search-hero",
                div { class: "search-field big",
                    SearchIcon { size: 20 }
                    input {
                        id: "global-search-input",
                        r#type: "search",
                        placeholder: "Enter name, @account, subject",
                        aria_label: "Search everything",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
            }
            if let Some(msg) = status() {
                div { class: "chat-banner", role: "status", span { "{msg}" } button { r#type: "button", class: "btn-ghost", onclick: move |_| status.set(None), "Close" } }
            }
            div { class: "search-results",
                if !q.is_empty() && !msg_hits.read().is_empty() {
                    section { class: "search-section",
                        h2 { class: "search-section-title",
                            "Tin nhắn cục bộ ({msg_hits.read().len()} · {msg_latency_ms():.1} ms)"
                        }
                        if msg_coverage() != SearchCoverageState::IndexedComplete {
                            div { class: "settings-sublabel", "Đang tìm trên phần lịch sử đã lập chỉ mục cục bộ" }
                        }
                        div { class: "result-grid",
                            for hit in msg_hits() {
                                {
                                    let h_scope = hit.scope_type;
                                    let h_sid = hit.scope_id.clone();
                                    let fr_match = friends.iter().find(|f| f.id == h_sid).cloned();
                                    let grp_match = groups.iter().find(|g| g.id == h_sid).cloned();
                                    let title = match h_scope {
                                        ScopeKind::Dm => fr_match
                                            .as_ref()
                                            .and_then(|f| f.display_name.clone().filter(|s| !s.is_empty()).or_else(|| Some(f.username.clone())))
                                            .unwrap_or_else(|| "Tin nhắn cá nhân".into()),
                                        ScopeKind::Group => grp_match
                                            .as_ref()
                                            .map(|g| g.name.clone())
                                            .unwrap_or_else(|| "Nhóm".into()),
                                        ScopeKind::Channel => "Kênh".into(),
                                    };
                                    rsx! {
                                        button {
                                            key: "mh_{hit.message_id}",
                                            r#type: "button",
                                            class: "result-card",
                                            onclick: move |_| {
                                                if let Some(f) = fr_match.clone() {
                                                    on_open_friend.call(f);
                                                } else if let Some(g) = grp_match.clone() {
                                                    on_open_group.call(g);
                                                }
                                            },
                                            div { class: "result-info",
                                                div { class: "result-name", "{title}" }
                                                div { class: "result-sub", "{hit.snippet}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if show(Tab::Chats) && (!my_friends.is_empty() || !my_groups.is_empty() || !my_servers.is_empty()) {
                    section { class: "search-section",
                        h2 { class: "search-section-title", "Your conversations" }
                        div { class: "result-grid",
                            for f in my_friends {
                                {
                                    let fc = f.clone();
                                    let name = f.display_name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| f.username.clone());
                                    rsx! {
                                        button { key: "f{f.id}", r#type: "button", class: "result-card", onclick: move |_| on_open_friend.call(fc.clone()),
                                            Avatar { name: f.username.clone(), avatar_url: f.avatar_url.clone(), size: 44 }
                                            div { class: "result-info",
                                                div { class: "result-name", "{name}" }
                                                div { class: "result-sub", "@{f.username} · Friends" }
                                            }
                                        }
                                    }
                                }
                            }
                            for g in my_groups {
                                {
                                    let gc = g.clone();
                                    rsx! {
                                        button { key: "g{g.id}", r#type: "button", class: "result-card", onclick: move |_| on_open_group.call(gc.clone()),
                                            Avatar { name: g.name.clone(), avatar_url: g.avatar_url.clone(), size: 44 }
                                            div { class: "result-info",
                                                div { class: "result-name", "{g.name}" }
                                                div { class: "result-sub", "Group · {g.member_count} members" }
                                            }
                                        }
                                    }
                                }
                            }
                            for sv in my_servers {
                                {
                                    let sc = sv.clone();
                                    rsx! {
                                        button { key: "s{sv.id}", r#type: "button", class: "result-card", onclick: move |_| on_open_server.call(sc.clone()),
                                            Avatar { name: sv.name.clone(), avatar_url: sv.icon_url.clone(), size: 44 }
                                            div { class: "result-info",
                                                div { class: "result-name", "{sv.name}" }
                                                div { class: "result-sub", "Community · {sv.member_count} members" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if show(Tab::People) && !people.read().is_empty() {
                    section { class: "search-section",
                        h2 { class: "search-section-title", "Everyone" }
                        div { class: "result-grid",
                            for u in people() {
                                {
                                    let uid = u.id.clone();
                                    let fr = friends.iter().find(|f| f.id == u.id).cloned();
                                    rsx! {
                                        div { key: "p{u.id}", class: "result-card",
                                            Avatar { name: u.username.clone(), avatar_url: u.avatar_url.clone(), size: 44 }
                                            div { class: "result-info",
                                                div { class: "result-name", "{u.username}" }
                                                div { class: "result-sub",
                                                    match u.relationship.as_str() {
                                                        "friend" => "Friends",
                                                        "pending_sent" => "Invitation sent",
                                                        "pending_received" => "Waiting for your approval",
                                                        _ => "ChatW users",
                                                    }
                                                }
                                            }
                                            if let Some(f) = fr {
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_open_friend.call(f.clone()), "Message" }
                                            } else if u.relationship == "none" {
                                                button { r#type: "button", class: "btn-primary", onclick: move |_| { on_add_friend.call(uid.clone()); people.with_mut(|l| for x in l.iter_mut() { if x.id == uid { x.relationship = "pending_sent".into(); } }); },
                                                    UserPlusIcon { size: 16 } "Add friend"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if show(Tab::Communities) {
                    DirSection { title: "Public community", icon: "community", entries: d.servers.clone(), empty: if q.is_empty() { "There is no verified public community yet" } else { "There is no suitable community" },
                        on_action: move |e: DirectoryEntry| {
                            let id = e.id.clone().unwrap_or_default();
                            let joined = e.joined;
                            let name = e.name.clone().unwrap_or_default();
                            spawn(async move {
                                if joined {
                                    if let Some(sv) = api::servers_list().await.ok().and_then(|l| l.into_iter().find(|s| s.id == id)) { on_open_server.call(sv); }
                                    return;
                                }
                                match social::server_join_public(&id).await {
                                    Ok(_) => {
                                        status.set(Some(format!("Joined community {name}")));
                                        on_joined.call(());
                                        dir.with_mut(|d| for x in d.servers.iter_mut() { if x.id.as_deref() == Some(id.as_str()) { x.joined = true; } });
                                    }
                                    Err(_) => status.set(Some("Can't join this community".into())),
                                }
                            });
                        },
                    }
                }
                if show(Tab::Groups) {
                    DirSection { title: "Public group", icon: "group", entries: d.groups.clone(), empty: if q.is_empty() { "There is no verified public group yet" } else { "There is no suitable group" },
                        on_action: move |e: DirectoryEntry| {
                            let id = e.id.clone().unwrap_or_default();
                            let joined = e.joined;
                            let name = e.name.clone().unwrap_or_default();
                            spawn(async move {
                                if joined {
                                    if let Some(g) = crate::services::groups::groups_list().await.ok().and_then(|l| l.into_iter().find(|g| g.id == id)) { on_open_group.call(g); }
                                    return;
                                }
                                match social::directory_join_group(&id).await {
                                    Ok(v) if v.get("status").and_then(|s| s.as_str()) == Some("pending") => status.set(Some(format!("Request to join {name} has been sent. The group leader or deputy will approve."))),
                                    Ok(_) => {
                                        status.set(Some(format!("Joined group {name}")));
                                        on_joined.call(());
                                        dir.with_mut(|d| for x in d.groups.iter_mut() { if x.id.as_deref() == Some(id.as_str()) { x.joined = true; } });
                                    }
                                    Err(e) => status.set(Some(match e.code.as_str() {
                                        "group_blocked" | "blocked_from_group" => "You cannot join this group".to_string(),
                                        _ => "Cannot join this group".to_string(),
                                    })),
                                }
                            });
                        },
                    }
                }
                if show(Tab::Bots) {
                    {
                        let plugin_entries = d.bots.clone();
                        rsx! {
                            DirSection { title: "Public bots", icon: "plugin", entries: plugin_entries, empty: if q.is_empty() { "There are no verified public bots yet" } else { "There are no suitable bots" },
                                on_action: move |e: DirectoryEntry| {
                                    let name = e.username.clone().unwrap_or_default();
                                    spawn(async move {
                                        if let Ok(b) = social::bot_by_name(&name).await { on_open_bot.call(b); }
                                    });
                                },
                            }
                        }
                    }
                }
                if show(Tab::Business) {
                    DirSection { title: "Business Page", icon: "business", entries: d.businesses.clone(), empty: if q.is_empty() { "There are no Business pages yet" } else { "There are no matching pages" },
                        base: base.clone(),
                        on_action: move |e: DirectoryEntry| {
                            let h = e.handle.clone().unwrap_or_default();
                            let b = origin.peek().clone();
                            platform::open_external_url(&format!("{b}/k/{}", urlencoding::encode(&h)));
                        },
                    }
                }
                if loading() { div { class: "search-loading", role: "status", "Looking" } }
            }
        }
    }
}

#[component]
fn DirSection(title: String, icon: String, entries: Vec<DirectoryEntry>, empty: String, on_action: EventHandler<DirectoryEntry>, #[props(default)] base: String) -> Element {
    let _ = base;
    rsx! {
        section { class: "search-section",
            h2 { class: "search-section-title",
                match icon.as_str() {
                    "community" => rsx! { CompassIcon { size: 18 } },
                    "group" => rsx! { GroupIcon {} },
                    "plugin" | "bot" => rsx! { PluginIcon { size: 18 } },
                    _ => rsx! { StoreIcon { size: 18 } },
                }
                "{title}"
            }
            if entries.is_empty() {
                div { class: "sidebar-empty", "{empty}" }
            } else {
                div { class: "result-grid",
                    for (i, e) in entries.into_iter().enumerate() {
                        {
                            let name = e.display_name.clone().or(e.name.clone()).unwrap_or_default();
                            let sub = match icon.as_str() {
                                "community" | "group" => format!("{} members{}", e.member_count, if e.require_approval { " · need approval" } else { "" }),
                                "plugin" | "bot" => format!("@{}", e.username.clone().unwrap_or_default()),
                                _ => e.category.clone().unwrap_or_else(|| "Business".into()),
                            };
                            let banner = e.banner_url.clone().or(e.cover_url.clone()).filter(|u| platform::safe_asset_path(u));
                            let action = match (icon.as_str(), e.joined) {
                                ("community" | "group", true) => "Open",
                                ("community", false) => "Join",
                                ("group", false) => if e.require_approval { "Request to join" } else { "Join" },
                                ("plugin" | "bot", _) => "Use Plugins",
                                _ => "View page",
                            };
                            let ec = e.clone();
                            rsx! {
                                div { key: "{i}", class: "result-card dir-card",
                                    if let Some(b) = banner {
                                        div { class: "dir-banner", style: "background-image: url('{platform::resolve_asset_url(&b)}');" }
                                    }
                                    div { class: "dir-body",
                                        Avatar { name: name.clone(), avatar_url: e.avatar_url.clone(), size: 44 }
                                        div { class: "result-info",
                                            div { class: "result-name", "{name}" span { class: "verified-tag", title: "Verified", "✓" } }
                                            div { class: "result-sub", "{sub}" }
                                            if let Some(dsc) = e.description.as_ref().filter(|d| !d.trim().is_empty()) {
                                                div { class: "result-desc", "{dsc}" }
                                            }
                                        }
                                        button { r#type: "button", class: if action == "Join" || action == "Request to join" { "btn-primary" } else { "btn-secondary" }, onclick: move |_| on_action.call(ec.clone()), "{action}" }
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
