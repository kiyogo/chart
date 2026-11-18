use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    components::{
        avatar::Avatar,
        icons::{
            ChevronDownIcon, ChevronLeftIcon, ChevronRightIcon, CompassIcon, FriendsIcon,
            HashIcon, LogoutIcon, MegaphoneIcon, MessageIcon, MicIcon, MicOffIcon, PhoneOffIcon,
            PlusIcon, SearchIcon, SettingsIcon, UserIcon, UserPlusIcon, VolumeIcon,
        },
        navigation::NavSection,
    },
    models::{
        ChannelItem, FriendItem, GroupItem, PendingRequestItem, ServerCategoryItem, ServerItem,
        UserSearchResult, VoiceStateItem,
    },
    platform,
};

fn server_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let a = parts[0].chars().next().unwrap_or('C');
        let b = parts[1].chars().next().unwrap_or(' ');
        format!("{a}{b}").to_uppercase()
    } else {
        name.chars().take(2).collect::<String>().to_uppercase()
    }
}

fn format_badge(count: usize) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    }
}

#[component]
pub fn NavigationSidebar(
    active_section: NavSection,
    current_username: String,
    current_avatar_url: Option<String>,
    hidden_mobile: bool,
    // Section switcher + account (replaces the old server rail)
    unread_dm_count: usize,
    pending_friend_count: usize,
    on_select_dm: EventHandler<()>,
    on_select_friends: EventHandler<()>,
    on_select_communities: EventHandler<()>,
    on_open_profile: EventHandler<()>,
    on_open_settings: EventHandler<()>,
    on_logout: EventHandler<()>,
    // Communities list
    servers: Vec<ServerItem>,
    on_select_server: EventHandler<ServerItem>,
    on_open_create_server: EventHandler<()>,
    // DM + Group DM
    friends: Vec<FriendItem>,
    active_friend_id: Option<String>,
    groups: Vec<GroupItem>,
    active_group_id: Option<String>,
    unread_counts: HashMap<String, usize>,
    last_previews: HashMap<String, String>,
    on_select_friend: EventHandler<FriendItem>,
    on_select_group: EventHandler<GroupItem>,
    on_open_create_group: EventHandler<()>,
    // Friends management
    friend_input: String,
    search_results: Vec<UserSearchResult>,
    pending_requests: Vec<PendingRequestItem>,
    add_friend_feedback: Option<(bool, String)>,
    is_adding_friend: bool,
    on_friend_input_change: EventHandler<String>,
    on_add_friend_submit: EventHandler<()>,
    on_send_friend_request_by_id: EventHandler<String>,
    on_accept_friend_request: EventHandler<String>,
    // Community Server
    selected_server: Option<ServerItem>,
    server_categories: Vec<ServerCategoryItem>,
    server_channels: Vec<ChannelItem>,
    selected_channel_id: Option<String>,
    active_voice_channel: Option<ChannelItem>,
    voice_participants: Vec<VoiceStateItem>,
    is_voice_muted: bool,
    on_select_channel: EventHandler<ChannelItem>,
    on_open_create_category: EventHandler<()>,
    on_open_create_channel: EventHandler<Option<String>>,
    on_open_server_invite: EventHandler<()>,
    on_toggle_voice_mute: EventHandler<()>,
    on_leave_voice: EventHandler<()>,
) -> Element {
    let mut collapsed_categories = use_signal(HashSet::<String>::new);
    let mut dm_query = use_signal(String::new);
    let mut dm_filter = use_signal(|| "all");
    let mut account_open = use_signal(|| false);
    let mut community_query = use_signal(String::new);

    let sidebar_cls = if hidden_mobile {
        "nav-sidebar hidden-mobile"
    } else {
        "nav-sidebar"
    };

    // Deduplicate groups defensively by ID
    let mut seen_groups = HashSet::new();
    let unique_groups: Vec<GroupItem> = groups
        .into_iter()
        .filter(|g| seen_groups.insert(g.id.clone()))
        .collect();

    let q = dm_query().trim().to_lowercase();
    let filter = dm_filter();
    let friends_shown: Vec<FriendItem> = friends
        .iter()
        .filter(|f| q.is_empty() || f.username.to_lowercase().contains(&q))
        .filter(|f| match filter {
            "unread" => unread_counts.get(&f.id).copied().unwrap_or(0) > 0,
            "groups" => false,
            _ => true,
        })
        .cloned()
        .collect();
    let groups_shown: Vec<GroupItem> = unique_groups
        .iter()
        .filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q))
        .filter(|_| filter == "all" || filter == "groups")
        .cloned()
        .collect();

    let in_dm = active_section == NavSection::DirectMessages;
    let in_friends = active_section == NavSection::Friends;
    let in_communities = matches!(active_section, NavSection::Communities | NavSection::Server(_));
    let cq = community_query().trim().to_lowercase();
    let mut seen_srv = HashSet::new();
    let servers_shown: Vec<ServerItem> = servers
        .into_iter()
        .filter(|s| seen_srv.insert(s.id.clone()))
        .filter(|s| cq.is_empty() || s.name.to_lowercase().contains(&cq))
        .collect();

    rsx! {
        aside { class: "{sidebar_cls}", aria_label: "Danh sách điều hướng",
            // App header: account menu + contextual create action
            div { class: "sidebar-app-header",
                button {
                    r#type: "button",
                    class: "account-btn",
                    aria_label: "Tài khoản của tôi",
                    aria_haspopup: "menu",
                    aria_expanded: "{account_open()}",
                    onclick: move |_| account_open.set(!account_open()),
                    Avatar { name: current_username.clone(), avatar_url: current_avatar_url.clone(), size: 36 }
                    span { class: "account-meta",
                        span { class: "account-name", "{current_username}" }
                        span { class: "account-sub", "Tài khoản của bạn" }
                    }
                    ChevronDownIcon { size: 14 }
                }
                if in_dm {
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Tạo nhóm mới",
                        aria_label: "Tạo nhóm mới",
                        onclick: move |_| on_open_create_group.call(()),
                        PlusIcon { size: 20 }
                    }
                } else if in_communities {
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Tạo hoặc tham gia cộng đồng",
                        aria_label: "Tạo hoặc tham gia cộng đồng",
                        onclick: move |_| on_open_create_server.call(()),
                        PlusIcon { size: 20 }
                    }
                }
                if account_open() {
                    button {
                        r#type: "button",
                        class: "menu-dismiss",
                        aria_label: "Đóng menu",
                        tabindex: "-1",
                        onclick: move |_| account_open.set(false),
                    }
                    div { class: "account-menu", role: "menu",
                        button { r#type: "button", class: "menu-item", role: "menuitem",
                            onclick: move |_| { account_open.set(false); on_open_profile.call(()); },
                            UserIcon {} span { "Hồ sơ của tôi" }
                        }
                        button { r#type: "button", class: "menu-item", role: "menuitem",
                            aria_label: "Cài đặt",
                            onclick: move |_| { account_open.set(false); on_open_settings.call(()); },
                            SettingsIcon {} span { "Cài đặt" }
                        }
                        button { r#type: "button", class: "menu-item danger", role: "menuitem",
                            onclick: move |_| { account_open.set(false); on_logout.call(()); },
                            LogoutIcon {} span { "Đăng xuất" }
                        }
                    }
                }
            }

            // Section switcher
            nav { class: "section-switch", role: "tablist", aria_label: "Chuyển mục",
                button {
                    r#type: "button",
                    role: "tab",
                    aria_selected: "{in_dm}",
                    class: if in_dm { "section-tab active" } else { "section-tab" },
                    onclick: move |_| on_select_dm.call(()),
                    MessageIcon { size: 17 }
                    span { "Tin nhắn" }
                    if unread_dm_count > 0 { span { class: "section-badge", "{format_badge(unread_dm_count)}" } }
                }
                button {
                    r#type: "button",
                    role: "tab",
                    aria_selected: "{in_friends}",
                    class: if in_friends { "section-tab active" } else { "section-tab" },
                    onclick: move |_| on_select_friends.call(()),
                    FriendsIcon {}
                    span { "Bạn bè" }
                    if pending_friend_count > 0 { span { class: "section-badge", "{format_badge(pending_friend_count)}" } }
                }
                button {
                    r#type: "button",
                    role: "tab",
                    aria_selected: "{in_communities}",
                    class: if in_communities { "section-tab active" } else { "section-tab" },
                    onclick: move |_| on_select_communities.call(()),
                    CompassIcon { size: 17 }
                    span { "Cộng đồng" }
                }
            }

            match active_section {
                NavSection::DirectMessages => rsx! {
                    div { class: "sidebar-search",
                        div { class: "search-field",
                            SearchIcon { size: 16 }
                            input {
                                r#type: "search",
                                placeholder: "Tìm cuộc trò chuyện",
                                aria_label: "Tìm cuộc trò chuyện",
                                value: "{dm_query}",
                                oninput: move |e| dm_query.set(e.value()),
                            }
                        }
                    }
                    div { class: "sidebar-filters", role: "tablist", aria_label: "Lọc cuộc trò chuyện",
                        for (val, label) in [("all", "Tất cả"), ("unread", "Chưa đọc"), ("direct", "Cá nhân"), ("groups", "Nhóm")] {
                            button {
                                key: "{val}",
                                r#type: "button",
                                role: "tab",
                                aria_selected: "{filter == val}",
                                class: if filter == val { "filter-chip active" } else { "filter-chip" },
                                onclick: move |_| dm_filter.set(val),
                                "{label}"
                            }
                        }
                    }

                    div { class: "sidebar-scrollable",
                        if filter != "groups" {
                        div { class: "sidebar-section-header",
                            span { "Tin nhắn trực tiếp" }
                        }
                        if friends_shown.is_empty() {
                            div { class: "sidebar-empty",
                                if friends.is_empty() { "Chưa có bạn bè. Mở mục Bạn bè để kết bạn." } else { "Không có cuộc trò chuyện phù hợp" }
                            }
                        } else {
                            for friend in friends_shown {
                                {
                                    let is_selected = active_friend_id.as_deref() == Some(friend.id.as_str());
                                    let unread = unread_counts.get(&friend.id).copied().unwrap_or(0);
                                    let preview = last_previews
                                        .get(&friend.id)
                                        .cloned()
                                        .unwrap_or_else(|| "Bắt đầu trò chuyện".to_string());
                                    let f_click = friend.clone();
                                    rsx! {
                                        button {
                                            key: "{friend.id}",
                                            r#type: "button",
                                            class: if is_selected { "chat-item selected" } else { "chat-item" },
                                            onclick: move |_| on_select_friend.call(f_click.clone()),
                                            Avatar {
                                                name: friend.username.clone(),
                                                avatar_url: friend.avatar_url.clone(),
                                                size: 44,
                                            }
                                            div { class: "chat-item-info",
                                                div { class: "chat-item-header",
                                                    span {
                                                        class: if unread > 0 { "chat-item-name unread" } else { "chat-item-name" },
                                                        "{friend.username}"
                                                    }
                                                    if unread > 0 {
                                                        span { class: "chat-item-badge", "{format_badge(unread)}" }
                                                    }
                                                }
                                                div {
                                                    class: if unread > 0 { "chat-item-preview unread" } else { "chat-item-preview" },
                                                    "{preview}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        }

                        // Separated Group DMs section
                        if filter == "all" || filter == "groups" {
                        div { class: "sidebar-section-header",
                            span { "Nhóm" }
                            button {
                                r#type: "button",
                                class: "sidebar-section-btn",
                                title: "Tạo nhóm DM",
                                aria_label: "Tạo nhóm DM",
                                onclick: move |_| on_open_create_group.call(()),
                                PlusIcon { size: 14 }
                            }
                        }
                        if groups_shown.is_empty() {
                            div { class: "sidebar-empty", "Chưa có nhóm" }
                        } else {
                            for grp in groups_shown {
                                {
                                    let is_selected = active_group_id.as_deref() == Some(grp.id.as_str());
                                    let g_click = grp.clone();
                                    rsx! {
                                        button {
                                            key: "{grp.id}",
                                            r#type: "button",
                                            class: if is_selected { "chat-item selected" } else { "chat-item" },
                                            onclick: move |_| on_select_group.call(g_click.clone()),
                                            Avatar {
                                                name: grp.name.clone(),
                                                avatar_url: grp.avatar_url.clone(),
                                                size: 44,
                                            }
                                            div { class: "chat-item-info",
                                                div { class: "chat-item-header",
                                                    span { class: "chat-item-name", "{grp.name}" }
                                                }
                                                div { class: "chat-item-preview", "{grp.member_count} thành viên" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        }
                    }
                },

                NavSection::Friends => rsx! {
                    div { class: "add-friend-container",
                        form {
                            class: "add-friend-form",
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                on_add_friend_submit.call(());
                            },
                            input {
                                r#type: "text",
                                placeholder: "Tìm hoặc nhập tên tài khoản",
                                aria_label: "Tên tài khoản kết bạn",
                                value: "{friend_input}",
                                oninput: move |evt| on_friend_input_change.call(evt.value()),
                            }
                            button {
                                r#type: "submit",
                                class: "btn-small-primary",
                                disabled: friend_input.trim().is_empty() || is_adding_friend,
                                "Kết bạn"
                            }
                        }
                        if let Some((is_err, msg)) = add_friend_feedback {
                            div {
                                class: if is_err { "status-badge error" } else { "status-badge success" },
                                "{msg}"
                            }
                        }
                    }
                    div { class: "sidebar-scrollable",
                        if !friend_input.trim().is_empty() {
                            div { class: "sidebar-section-header", span { "Kết quả tìm kiếm" } }
                            if search_results.is_empty() {
                                div { class: "empty-chat-inner", "Không tìm thấy người dùng" }
                            } else {
                                for user in search_results {
                                    div { class: "request-card", key: "{user.id}",
                                        div { class: "request-user",
                                            Avatar {
                                                name: user.username.clone(),
                                                avatar_url: user.avatar_url.clone(),
                                                size: 30,
                                            }
                                            span { class: "chat-item-name", "{user.username}" }
                                        }
                                        if user.relationship == "friend" {
                                            span { class: "btn-small-disabled", "Bạn bè" }
                                        } else if user.relationship == "pending_sent" {
                                            span { class: "btn-small-disabled", "Đã gửi" }
                                        } else if user.relationship == "pending_received" {
                                            button {
                                                r#type: "button",
                                                class: "btn-small-primary",
                                                onclick: {
                                                    let uid = user.id.clone();
                                                    let req_id = pending_requests
                                                        .iter()
                                                        .find(|r| r.sender_id == uid)
                                                        .map(|r| r.id.clone());
                                                    move |_| {
                                                        if let Some(rid) = req_id.clone() {
                                                            on_accept_friend_request.call(rid);
                                                        }
                                                    }
                                                },
                                                "Chấp nhận"
                                            }
                                        } else {
                                            button {
                                                r#type: "button",
                                                class: "btn-small-primary",
                                                onclick: {
                                                    let uid = user.id.clone();
                                                    move |_| on_send_friend_request_by_id.call(uid.clone())
                                                },
                                                "Kết bạn"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "sidebar-section-header",
                            span { "Lời mời kết bạn ({pending_requests.len()})" }
                        }
                        if pending_requests.is_empty() {
                            div { class: "sidebar-empty", "Không có lời mời nào" }
                        } else {
                            for req in pending_requests {
                                div { class: "request-card", key: "{req.id}",
                                    div { class: "request-user",
                                        Avatar {
                                            name: req.sender_username.clone(),
                                            avatar_url: req.sender_avatar_url.clone(),
                                            size: 30,
                                        }
                                        span { class: "chat-item-name", "{req.sender_username}" }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-small-primary",
                                        onclick: {
                                            let rid = req.id.clone();
                                            move |_| on_accept_friend_request.call(rid.clone())
                                        },
                                        "Chấp nhận"
                                    }
                                }
                            }
                        }

                        div { class: "sidebar-section-header",
                            span { "Tất cả bạn bè ({friends.len()})" }
                        }
                        for friend in friends {
                            {
                                let f_click = friend.clone();
                                rsx! {
                                    button {
                                        key: "fl-{friend.id}",
                                        r#type: "button",
                                        class: "chat-item",
                                        onclick: move |_| on_select_friend.call(f_click.clone()),
                                        Avatar {
                                            name: friend.username.clone(),
                                            avatar_url: friend.avatar_url.clone(),
                                            size: 32,
                                        }
                                        div { class: "chat-item-info",
                                            span { class: "chat-item-name", "{friend.username}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },

                NavSection::Server(_) => rsx! {
                    if let Some(srv) = selected_server {
                        div { class: "sidebar-top-header",
                            button {
                                r#type: "button",
                                class: "icon-btn",
                                title: "Tất cả cộng đồng",
                                aria_label: "Quay lại danh sách cộng đồng",
                                onclick: move |_| on_select_communities.call(()),
                                ChevronLeftIcon { size: 18 }
                            }
                            span { class: "sidebar-title", title: "{srv.name}", "{srv.name}" }
                            div { style: "display: flex; align-items: center; gap: 2px;",
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    title: "Tạo mã mời tham gia",
                                    aria_label: "Mời tham gia cộng đồng",
                                    onclick: move |_| on_open_server_invite.call(()),
                                    UserPlusIcon { size: 16 }
                                }
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    title: "Tạo danh mục mới",
                                    aria_label: "Tạo danh mục",
                                    onclick: move |_| on_open_create_category.call(()),
                                    PlusIcon { size: 16 }
                                }
                            }
                        }

                        div { class: "sidebar-scrollable",
                            // Channels without a category
                            {
                                let uncategorized: Vec<ChannelItem> = server_channels
                                    .iter()
                                    .filter(|c| c.category_id.is_none())
                                    .cloned()
                                    .collect();
                                rsx! {
                                    for ch in uncategorized {
                                        ChannelRowItem {
                                            channel: ch,
                                            is_selected: selected_channel_id.as_deref() == Some(""),
                                            selected_channel_id: selected_channel_id.clone(),
                                            active_voice_channel_id: active_voice_channel.as_ref().map(|v| v.id.clone()),
                                            voice_participants: voice_participants.clone(),
                                            on_select: on_select_channel,
                                        }
                                    }
                                }
                            }

                            // Vertical Collapsible Categories + Channels underneath
                            for cat in server_categories {
                                {
                                    let cat_id = cat.id.clone();
                                    let is_collapsed = collapsed_categories.read().contains(&cat_id);
                                    let cat_channels: Vec<ChannelItem> = server_channels
                                        .iter()
                                        .filter(|c| c.category_id.as_deref() == Some(cat.id.as_str()))
                                        .cloned()
                                        .collect();
                                    let cat_id_toggle = cat.id.clone();
                                    let cat_id_add = cat.id.clone();
                                    rsx! {
                                        div { class: "category-block", key: "{cat.id}",
                                            div { class: "category-row",
                                                button {
                                                    r#type: "button",
                                                    class: "category-row-left",
                                                    onclick: move |_| {
                                                        collapsed_categories.with_mut(|set| {
                                                            if set.contains(&cat_id_toggle) {
                                                                set.remove(&cat_id_toggle);
                                                            } else {
                                                                set.insert(cat_id_toggle.clone());
                                                            }
                                                        });
                                                    },
                                                    if is_collapsed {
                                                        ChevronRightIcon { size: 13 }
                                                    } else {
                                                        ChevronDownIcon { size: 13 }
                                                    }
                                                    span { class: "category-name", "{cat.name}" }
                                                }
                                                button {
                                                    r#type: "button",
                                                    class: "sidebar-section-btn",
                                                    title: "Tạo kênh trong {cat.name}",
                                                    aria_label: "Tạo kênh trong danh mục",
                                                    onclick: move |_| on_open_create_channel.call(Some(cat_id_add.clone())),
                                                    PlusIcon { size: 13 }
                                                }
                                            }
                                            if !is_collapsed {
                                                for ch in cat_channels {
                                                    ChannelRowItem {
                                                        key: "{ch.id}",
                                                        channel: ch,
                                                        is_selected: false,
                                                        selected_channel_id: selected_channel_id.clone(),
                                                        active_voice_channel_id: active_voice_channel.as_ref().map(|v| v.id.clone()),
                                                        voice_participants: voice_participants.clone(),
                                                        on_select: on_select_channel,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if server_channels.is_empty() {
                                div { class: "empty-chat-inner", "Chưa có kênh nào" }
                            }
                        }
                    } else {
                        div { class: "empty-chat-inner", "Chọn một cộng đồng trong danh sách" }
                    }
                },

                NavSection::Communities => rsx! {
                    div { class: "sidebar-search",
                        div { class: "search-field",
                            SearchIcon { size: 16 }
                            input {
                                r#type: "search",
                                placeholder: "Tìm cộng đồng",
                                aria_label: "Tìm cộng đồng",
                                value: "{community_query}",
                                oninput: move |e| community_query.set(e.value()),
                            }
                        }
                    }
                    div { class: "sidebar-scrollable",
                        div { class: "sidebar-section-header", span { "Cộng đồng của bạn" } }
                        if servers_shown.is_empty() {
                            div { class: "sidebar-empty",
                                if cq.is_empty() { "Bạn chưa tham gia cộng đồng nào." } else { "Không có cộng đồng phù hợp" }
                            }
                        } else {
                            for srv in servers_shown {
                                {
                                    let is_sel = selected_server.as_ref().map(|s| s.id == srv.id).unwrap_or(false);
                                    let s_click = srv.clone();
                                    let initials = server_initials(&srv.name);
                                    rsx! {
                                        button {
                                            key: "{srv.id}",
                                            r#type: "button",
                                            class: if is_sel { "chat-item community-item selected" } else { "chat-item community-item" },
                                            aria_label: "Cộng đồng {srv.name}",
                                            onclick: move |_| on_select_server.call(s_click.clone()),
                                            span { class: "community-icon",
                                                if let Some(icon) = srv.icon_url.as_ref().filter(|u| !u.is_empty()) {
                                                    img { src: "{platform::resolve_asset_url(icon)}", alt: "" }
                                                } else {
                                                    "{initials}"
                                                }
                                            }
                                            div { class: "chat-item-info",
                                                span { class: "chat-item-name", "{srv.name}" }
                                                span { class: "chat-item-preview",
                                                    "{srv.member_count} thành viên"
                                                    if srv.is_owner { " · Bạn là chủ sở hữu" }
                                                }
                                            }
                                            ChevronRightIcon { size: 16 }
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "create-community-btn",
                            onclick: move |_| on_open_create_server.call(()),
                            span { class: "community-icon add", PlusIcon { size: 18 } }
                            span { class: "chat-item-info",
                                span { class: "chat-item-name", "Tạo hoặc tham gia cộng đồng" }
                                span { class: "chat-item-preview", "Bằng tên mới hoặc mã mời" }
                            }
                        }
                    }
                },

                NavSection::Settings => rsx! {
                    div { class: "sidebar-top-header",
                        span { class: "sidebar-title", "Cài đặt" }
                    }
                    div { class: "sidebar-scrollable",
                        div { style: "padding: 12px 10px; font-size: 13px; color: var(--text-secondary);",
                            "Quản lý tài khoản, hồ sơ cá nhân, giao diện và khóa bảo mật E2EE tại trang cài đặt."
                        }
                    }
                },
            }

            // Active Voice Channel Status Bar (docked above user footer)
            if let Some(vc) = active_voice_channel {
                div { class: "voice-status-panel",
                    div { class: "voice-status-info",
                        span { class: "voice-status-label", "Đã kết nối thoại" }
                        span { class: "voice-status-channel", "{vc.name}" }
                    }
                    div { style: "display: flex; align-items: center; gap: 4px;",
                        button {
                            r#type: "button",
                            class: if is_voice_muted { "icon-btn btn-danger" } else { "icon-btn" },
                            title: if is_voice_muted { "Bật micro" } else { "Tắt micro" },
                            aria_label: "Chuyển trạng thái micro",
                            onclick: move |_| on_toggle_voice_mute.call(()),
                            if is_voice_muted { MicOffIcon { size: 16 } } else { MicIcon { size: 16 } }
                        }
                        button {
                            r#type: "button",
                            class: "icon-btn btn-danger",
                            title: "Ngắt kết nối thoại",
                            aria_label: "Ngắt kết nối thoại",
                            onclick: move |_| on_leave_voice.call(()),
                            PhoneOffIcon { size: 16 }
                        }
                    }
                }
            }

        }
    }
}

#[component]
fn ChannelRowItem(
    channel: ChannelItem,
    is_selected: bool,
    selected_channel_id: Option<String>,
    active_voice_channel_id: Option<String>,
    voice_participants: Vec<VoiceStateItem>,
    on_select: EventHandler<ChannelItem>,
) -> Element {
    let selected = is_selected || selected_channel_id.as_deref() == Some(channel.id.as_str());
    let is_voice = channel.channel_type == "voice";
    let is_announcement = channel.channel_type == "announcement";
    let is_active_voice = active_voice_channel_id.as_deref() == Some(channel.id.as_str());
    let ch_click = channel.clone();

    let participants_in_this: Vec<VoiceStateItem> = if is_voice {
        voice_participants
            .into_iter()
            .filter(|p| p.channel_id == channel.id)
            .collect()
    } else {
        Vec::new()
    };

    rsx! {
        div {
            button {
                r#type: "button",
                class: if selected || is_active_voice { "channel-row selected" } else { "channel-row" },
                onclick: move |_| on_select.call(ch_click.clone()),
                if is_voice {
                    VolumeIcon { size: 16 }
                } else if is_announcement {
                    MegaphoneIcon { size: 16 }
                } else {
                    HashIcon { size: 16 }
                }
                span { class: "channel-row-name", "{channel.name}" }
            }
            if is_voice && !participants_in_this.is_empty() {
                div { class: "voice-participants-list",
                    for p in participants_in_this {
                        div { class: "voice-participant-row", key: "{p.user_id}",
                            Avatar {
                                name: p.username.clone(),
                                avatar_url: p.avatar_url.clone(),
                                size: 20,
                            }
                            span { style: "flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{p.username}" }
                            if p.self_mute {
                                MicOffIcon { size: 12 }
                            }
                        }
                    }
                }
            }
        }
    }
}

