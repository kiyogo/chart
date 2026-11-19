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
};

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
    // Debounced remote search (directory always, people when 2+ characters).
    use_effect(move || {
        let q = query().trim().to_string();
        // peek: reading `seq` reactively here would re-run this effect forever.
        let my = *seq.peek() + 1;
        seq.set(my);
        spawn(async move {
            platform::sleep_ms(250).await;
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
                        placeholder: "Nhập tên, @tài_khoản, chủ đề",
                        aria_label: "Tìm kiếm mọi thứ",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
            }
            if let Some(msg) = status() {
                div { class: "chat-banner", role: "status", span { "{msg}" } button { r#type: "button", class: "btn-ghost", onclick: move |_| status.set(None), "Đóng" } }
            }
            div { class: "search-results",
                if show(Tab::Chats) && (!my_friends.is_empty() || !my_groups.is_empty() || !my_servers.is_empty()) {
                    section { class: "search-section",
                        h2 { class: "search-section-title", "Cuộc trò chuyện của bạn" }
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
                                                div { class: "result-sub", "@{f.username} · Bạn bè" }
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
                                                div { class: "result-sub", "Nhóm · {g.member_count} thành viên" }
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
                                                div { class: "result-sub", "Cộng đồng · {sv.member_count} thành viên" }
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
                        h2 { class: "search-section-title", "Mọi người" }
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
                                                        "friend" => "Bạn bè",
                                                        "pending_sent" => "Đã gửi lời mời",
                                                        "pending_received" => "Đang chờ bạn chấp nhận",
                                                        _ => "Người dùng ChatW",
                                                    }
                                                }
                                            }
                                            if let Some(f) = fr {
                                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_open_friend.call(f.clone()), "Nhắn tin" }
                                            } else if u.relationship == "none" {
                                                button { r#type: "button", class: "btn-primary", onclick: move |_| { on_add_friend.call(uid.clone()); people.with_mut(|l| for x in l.iter_mut() { if x.id == uid { x.relationship = "pending_sent".into(); } }); },
                                                    UserPlusIcon { size: 16 } "Kết bạn"
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
                    DirSection { title: "Cộng đồng công khai", icon: "community", entries: d.servers.clone(), empty: if q.is_empty() { "Chưa có cộng đồng công khai đã xác minh" } else { "Không có cộng đồng phù hợp" },
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
                                        status.set(Some(format!("Đã tham gia cộng đồng {name}")));
                                        on_joined.call(());
                                        dir.with_mut(|d| for x in d.servers.iter_mut() { if x.id.as_deref() == Some(id.as_str()) { x.joined = true; } });
                                    }
                                    Err(_) => status.set(Some("Không tham gia được cộng đồng này".into())),
                                }
                            });
                        },
                    }
                }
                if show(Tab::Groups) {
                    DirSection { title: "Nhóm công khai", icon: "group", entries: d.groups.clone(), empty: if q.is_empty() { "Chưa có nhóm công khai đã xác minh" } else { "Không có nhóm phù hợp" },
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
                                    Ok(v) if v.get("status").and_then(|s| s.as_str()) == Some("pending") => status.set(Some(format!("Đã gửi yêu cầu tham gia {name}. Trưởng hoặc phó nhóm sẽ duyệt."))),
                                    Ok(_) => {
                                        status.set(Some(format!("Đã tham gia nhóm {name}")));
                                        on_joined.call(());
                                        dir.with_mut(|d| for x in d.groups.iter_mut() { if x.id.as_deref() == Some(id.as_str()) { x.joined = true; } });
                                    }
                                    Err(e) => status.set(Some(match e.code.as_str() {
                                        "group_blocked" | "blocked_from_group" => "Bạn không thể tham gia nhóm này".to_string(),
                                        _ => "Không tham gia được nhóm này".to_string(),
                                    })),
                                }
                            });
                        },
                    }
                }
                if show(Tab::Bots) {
                    {
                        let mut plugin_entries = d.bots.clone();
                        for pm in crate::components::plugins_view::PLUGINS.read().iter() {
                            if q.is_empty() || fold(&pm.display_name).contains(&q) || fold(&pm.username).contains(&q) || fold(&pm.description).contains(&q) {
                                if !plugin_entries.iter().any(|x| x.username.as_deref() == Some(pm.username.as_str())) {
                                    plugin_entries.push(DirectoryEntry {
                                        id: Some(pm.id.clone()),
                                        username: Some(pm.username.clone()),
                                        display_name: Some(format!("{} {}", pm.icon, pm.display_name)),
                                        name: Some(pm.display_name.clone()),
                                        handle: None,
                                        description: Some(pm.description.clone()),
                                        category: None,
                                        avatar_url: None,
                                        banner_url: None,
                                        cover_url: None,
                                        member_count: 0,
                                        require_approval: false,
                                        joined: pm.enabled,
                                    });
                                }
                            }
                        }
                        rsx! {
                            DirSection { title: "Plugin công khai", icon: "plugin", entries: plugin_entries, empty: if q.is_empty() { "Chưa có plugin công khai đã xác minh" } else { "Không có plugin phù hợp" },
                                on_action: move |e: DirectoryEntry| {
                                    let id = e.id.clone().unwrap_or_default();
                                    let name = e.username.clone().unwrap_or_default();
                                    if id.starts_with("mod:") {
                                        on_open_bot.call(BotItem {
                                            id,
                                            owner_id: "local".into(),
                                            username: name,
                                            display_name: e.display_name.unwrap_or_default(),
                                            description: e.description,
                                            avatar_url: e.avatar_url,
                                            is_public: true,
                                            verification_status: "verified".into(),
                                            token_hint: None,
                                            token: None,
                                            last_text: None,
                                        });
                                    } else {
                                        spawn(async move {
                                            if let Ok(b) = social::bot_by_name(&name).await { on_open_bot.call(b); }
                                        });
                                    }
                                },
                            }
                        }
                    }
                }
                if show(Tab::Business) {
                    DirSection { title: "Trang Kinh doanh", icon: "business", entries: d.businesses.clone(), empty: if q.is_empty() { "Chưa có trang Kinh doanh nào" } else { "Không có trang phù hợp" },
                        base: base.clone(),
                        on_action: move |e: DirectoryEntry| {
                            let h = e.handle.clone().unwrap_or_default();
                            let b = origin.peek().clone();
                            platform::open_external_url(&format!("{b}/k/{}", urlencoding::encode(&h)));
                        },
                    }
                }
                if loading() { div { class: "search-loading", role: "status", "Đang tìm" } }
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
                                "community" | "group" => format!("{} thành viên{}", e.member_count, if e.require_approval { " · cần duyệt" } else { "" }),
                                "plugin" | "bot" => format!("@{}", e.username.clone().unwrap_or_default()),
                                _ => e.category.clone().unwrap_or_else(|| "Kinh doanh".into()),
                            };
                            let banner = e.banner_url.clone().or(e.cover_url.clone()).filter(|u| platform::safe_asset_path(u));
                            let action = match (icon.as_str(), e.joined) {
                                ("community" | "group", true) => "Mở",
                                ("community", false) => "Tham gia",
                                ("group", false) => if e.require_approval { "Xin tham gia" } else { "Tham gia" },
                                ("plugin" | "bot", _) => "Dùng Plugin",
                                _ => "Xem trang",
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
                                            div { class: "result-name", "{name}" span { class: "verified-tag", title: "Đã xác minh", "✓" } }
                                            div { class: "result-sub", "{sub}" }
                                            if let Some(dsc) = e.description.as_ref().filter(|d| !d.trim().is_empty()) {
                                                div { class: "result-desc", "{dsc}" }
                                            }
                                        }
                                        button { r#type: "button", class: if action == "Tham gia" || action == "Xin tham gia" { "btn-primary" } else { "btn-secondary" }, onclick: move |_| on_action.call(ec.clone()), "{action}" }
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
