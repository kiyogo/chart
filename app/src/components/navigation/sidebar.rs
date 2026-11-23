use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    components::{
        avatar::Avatar,
        enterprise::module_icon,
        icons::{
            ChevronDownIcon, ChevronLeftIcon, ChevronRightIcon, DevicesIcon, GroupIcon, HashIcon,
            MegaphoneIcon, MicIcon, MicOffIcon, PhoneOffIcon, PlusIcon, SearchIcon,
            UserPlusIcon, VolumeIcon,
        },
        navigation::NavSection,
        skeleton::{SkeletonChannels, SkeletonConversations},
    },
    models::{
        ChannelItem, FriendItem, GroupItem, PendingRequestItem, ServerCategoryItem, ServerItem,
        UserSearchResult, VoiceStateItem,
    },
    services::enterprise::{EnterpriseConfig, module_label},
};

/// Normalizes channel/category names so hyphens become spaces and only the
/// first letter is capitalized (while keeping acronyms like AI).
pub fn format_display_label(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '-' | '_' | '—' | '–' => ' ',
            _ => c,
        })
        .collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    let mut out = Vec::with_capacity(words.len());
    for (idx, word) in words.iter().enumerate() {
        if word.eq_ignore_ascii_case("ai") {
            out.push("AI".to_string());
        } else if idx == 0 {
            let lower = word.to_lowercase();
            let mut chars = lower.chars();
            if let Some(first) = chars.next() {
                let mut s = first.to_uppercase().to_string();
                s.push_str(chars.as_str());
                out.push(s);
            }
        } else {
            out.push(word.to_lowercase());
        }
    }
    out.join(" ")
}

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
    on_select_communities: EventHandler<()>,
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
    /// Replaces the built-in direct message list (unified conversation list).
    #[props(default)] dm_content: Option<Element>,
    /// Groups moved to Communities, listed above the servers.
    #[props(default)] community_groups: Vec<GroupItem>,
    #[props(default)] on_open_server_settings: Option<EventHandler<()>>,
    #[props(default)] is_server_settings_open: bool,
    /// Enterprise workspace state for the active server (optional & backwards-compatible).
    #[props(default)] enterprise_config: Option<EnterpriseConfig>,
    #[props(default)] active_ent_module: Option<String>,
    #[props(default)] on_select_ent_module: Option<EventHandler<String>>,
    /// Whether "Chính mình" self-chat currently exists (Spec 046).
    #[props(default = true)] self_chat_enabled: bool,
    /// Restore "Chính mình" self-chat from the Messages "+" dropdown menu.
    #[props(default)] on_create_self: Option<EventHandler<()>>,
    #[props(default = false)] loading: bool,
) -> Element {
    let mut collapsed_categories = use_signal(HashSet::<String>::new);
    let mut dm_query = use_signal(String::new);
    let dm_filter = use_signal(|| "all");
    let mut community_query = use_signal(String::new);
    let mut dm_plus_menu = use_signal(|| false);

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
            _ => true,
        })
        .cloned()
        .collect();
    let groups_shown: Vec<GroupItem> = unique_groups
        .iter()
        .filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q))
        .filter(|_| filter == "all")
        .cloned()
        .collect();

    let in_dm = active_section == NavSection::DirectMessages;
    let in_server = matches!(active_section, NavSection::Server(_));
    let in_communities = matches!(active_section, NavSection::Communities);
    let cq = community_query().trim().to_lowercase();
    let mut seen_srv = HashSet::new();
    let servers_shown: Vec<ServerItem> = servers
        .into_iter()
        .filter(|s| seen_srv.insert(s.id.clone()))
        .filter(|s| cq.is_empty() || s.name.to_lowercase().contains(&cq))
        .collect();
    let no_friends = friends.is_empty() && pending_requests.is_empty();
    let no_servers = servers_shown.is_empty() && community_groups.is_empty();

    rsx! {
        aside { class: "{sidebar_cls}", aria_label: "Navigation list",
            // Hide the "Cộng đồng" + "+" top bar when inside a specific server (Spec 046)
            if !in_server {
                div { class: "sidebar-top-header",
                    span { class: "sidebar-title",
                        match &active_section {
                            NavSection::DirectMessages => "Messages",
                            NavSection::Friends => "Friends",
                            NavSection::Communities | NavSection::Server(_) => "Communities",
                            NavSection::Meet => "Meetings",
                            NavSection::Feed => "Feed",
                            NavSection::VicoAi => "Vico AI",
                            NavSection::Search => "Search",
                            NavSection::Notifications => "Notifications",
                            NavSection::Download => "Download app",
                            NavSection::Settings => "Settings",
                        }
                    }
                    if in_dm {
                        div { class: "sidebar-plus-wrap",
                            button {
                                r#type: "button",
                                class: if dm_plus_menu() { "icon-btn active" } else { "icon-btn" },
                                title: "Create new",
                                aria_label: "Create new",
                                aria_haspopup: "menu",
                                aria_expanded: "{dm_plus_menu()}",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    dm_plus_menu.set(!dm_plus_menu());
                                },
                                PlusIcon { size: 20 }
                            }
                            if dm_plus_menu() {
                                button {
                                    r#type: "button",
                                    class: "menu-dismiss",
                                    tabindex: "-1",
                                    aria_label: "Close",
                                    onclick: move |_| dm_plus_menu.set(false),
                                }
                                div {
                                    class: "cw-dropdown-menu sidebar-plus-menu",
                                    role: "menu",
                                    onkeydown: move |e| if e.key() == Key::Escape { dm_plus_menu.set(false) },
                                    button {
                                        r#type: "button",
                                        role: "menuitem",
                                        class: "cw-dropdown-item",
                                        onclick: move |_| {
                                            dm_plus_menu.set(false);
                                            on_open_create_group.call(());
                                        },
                                        GroupIcon { size: 16 }
                                        span { "Create new group" }
                                    }
                                    if !self_chat_enabled {
                                        button {
                                            r#type: "button",
                                            role: "menuitem",
                                            class: "cw-dropdown-item",
                                            onclick: move |_| {
                                                dm_plus_menu.set(false);
                                                if let Some(h) = on_create_self {
                                                    h.call(());
                                                }
                                            },
                                            DevicesIcon { size: 16 }
                                            span { "Recreating \"Yourself\"" }
                                        }
                                    }
                                }
                            }
                        }
                    } else if in_communities {
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            title: "Create or join community",
                            aria_label: "Create or join community",
                            onclick: move |_| on_open_create_server.call(()),
                            PlusIcon { size: 20 }
                        }
                    }
                }
            }

            match active_section {
                NavSection::DirectMessages if dm_content.is_some() => rsx! { {dm_content} },
                NavSection::Search | NavSection::Feed | NavSection::VicoAi | NavSection::Notifications | NavSection::Download => rsx! {},
                NavSection::DirectMessages => rsx! {
                    div { class: "sidebar-search",
                        div { class: "search-field",
                            SearchIcon { size: 16 }
                            input {
                                r#type: "search",
                                placeholder: "Search conversations",
                                aria_label: "Search conversations",
                                value: "{dm_query}",
                                oninput: move |e| dm_query.set(e.value()),
                            }
                        }
                    }

                    div { class: "sidebar-scrollable",
                        div { class: "sidebar-section-header",
                            span { "Direct messages" }
                        }
                        if friends_shown.is_empty() {
                            div { class: "sidebar-empty",
                                if friends.is_empty() { "No friends yet. Open the Friends section to make friends." } else { "No appropriate conversation" }
                            }
                        } else {
                            for friend in friends_shown {
                                {
                                    let is_selected = active_friend_id.as_deref() == Some(friend.id.as_str());
                                    let unread = unread_counts.get(&friend.id).copied().unwrap_or(0);
                                    let preview = last_previews
                                        .get(&friend.id)
                                        .map(|t| crate::components::messages::preview_text(t))
                                        .unwrap_or_else(|| "Start conversation".to_string());
                                    let f_click = friend.clone();
                                    let (np_cls, np_style) = crate::components::profile_card::nameplate_style(friend.nameplate.as_deref(), friend.custom_nameplate_url.as_deref());
                                    let name_fx = crate::components::profile_card::name_effect_class(friend.name_effect.as_deref());
                                    let shown_name = friend.display_name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| friend.username.clone());
                                    rsx! {
                                        button {
                                            key: "{friend.id}",
                                            r#type: "button",
                                            class: if is_selected { "chat-item selected {np_cls}" } else { "chat-item {np_cls}" },
                                            style: "{np_style}",
                                            onclick: move |_| on_select_friend.call(f_click.clone()),
                                            crate::components::profile_card::DecoratedAvatar {
                                                name: friend.username.clone(),
                                                avatar_url: friend.avatar_url.clone(),
                                                size: 44,
                                                decoration: friend.avatar_decoration.clone().filter(|d| d != "custom"),
                                                motion: friend.avatar_motion.clone(),
                                                in_list: true,
                                            }
                                            div { class: "chat-item-info",
                                                div { class: "chat-item-header",
                                                    span {
                                                        class: if unread > 0 { "chat-item-name unread {name_fx}" } else { "chat-item-name {name_fx}" },
                                                        "{shown_name}"
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


                        // Separated Group DMs section
                        if filter == "all" {
                        div { class: "sidebar-section-header",
                            span { "Groups" }
                            button {
                                r#type: "button",
                                class: "sidebar-section-btn",
                                title: "Create a DM group",
                                aria_label: "Create a DM group",
                                onclick: move |_| on_open_create_group.call(()),
                                PlusIcon { size: 14 }
                            }
                        }
                        if groups_shown.is_empty() {
                            div { class: "sidebar-empty", "No groups yet" }
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
                                                div { class: "chat-item-preview", "{grp.member_count} members" }
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
                                placeholder: "Find or enter the account name",
                                aria_label: "Friend account name",
                                value: "{friend_input}",
                                oninput: move |evt| on_friend_input_change.call(evt.value()),
                            }
                            button {
                                r#type: "submit",
                                class: "btn-small-primary",
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
                    }
                    div { class: "sidebar-scrollable friends-scroll",
                        if !friend_input.trim().is_empty() {
                            div { class: "sidebar-section-header", span { "Search results" } }
                            if search_results.is_empty() {
                                div { class: "empty-chat-inner", "User not found" }
                            } else {
                                for user in search_results {
                                    div { class: "request-card", key: "{user.id}",
                                        div { class: "request-user",
                                            Avatar {
                                                name: user.username.clone(),
                                                avatar_url: user.avatar_url.clone(),
                                                size: 32,
                                            }
                                            span { class: "chat-item-name", "{user.username}" }
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
                        }

                        if !pending_requests.is_empty() {
                            for req in pending_requests {
                                div { class: "request-card", key: "{req.id}",
                                    div { class: "request-user",
                                        Avatar {
                                            name: req.sender_username.clone(),
                                            avatar_url: req.sender_avatar_url.clone(),
                                            size: 32,
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
                                        "Accept"
                                    }
                                }
                            }
                        }

                        for friend in friends {
                            {
                                let f_click = friend.clone();
                                rsx! {
                                    button {
                                        key: "fl-{friend.id}",
                                        r#type: "button",
                                        class: "chat-item friend-row",
                                        onclick: move |_| on_select_friend.call(f_click.clone()),
                                        Avatar {
                                            name: friend.username.clone(),
                                            avatar_url: friend.avatar_url.clone(),
                                            size: 36,
                                        }
                                        div { class: "chat-item-info",
                                            span { class: "chat-item-name", "{friend.username}" }
                                        }
                                    }
                                }
                            }
                        }
                        if loading && no_friends {
                            SkeletonConversations { count: 5 }
                        }
                    }
                },

                NavSection::Server(_) => rsx! {
                    if let Some(srv) = selected_server {
                        div { class: "sidebar-server-header",
                            button {
                                r#type: "button",
                                class: "icon-btn",
                                title: "All communities",
                                aria_label: "Return to the community list",
                                onclick: move |_| on_select_communities.call(()),
                                ChevronLeftIcon { size: 18 }
                            }
                            span { class: "sidebar-server-name", title: "{srv.name}", "{srv.name}" }
                            div { style: "display: flex; align-items: center; gap: 2px;",
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    title: "Invite friends using invitation cards or links",
                                    aria_label: "Please join the community",
                                    onclick: move |_| on_open_server_invite.call(()),
                                    UserPlusIcon { size: 16 }
                                }
                                if let Some(h) = on_open_server_settings {
                                    button {
                                        r#type: "button",
                                        class: if is_server_settings_open { "icon-btn active" } else { "icon-btn" },
                                        title: "Community settings",
                                        aria_label: "Community settings",
                                        onclick: move |_| h.call(()),
                                        crate::components::icons::SettingsIcon {}
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    title: "Create a new category",
                                    aria_label: "Create categories",
                                    onclick: move |_| on_open_create_category.call(()),
                                    PlusIcon { size: 16 }
                                }
                            }
                        }

                        div { class: "sidebar-scrollable",
                            if srv.is_enterprise {
                                if let (Some(cfg), Some(on_mod)) = (&enterprise_config, on_select_ent_module) {
                                    {
                                        let is_ws_collapsed = collapsed_categories.read().contains("__ent_ws__");
                                        let visible_mods: Vec<String> = if cfg.module_order.is_empty() {
                                            cfg.enabled_modules.iter().filter(|k| cfg.can_see_module(k)).cloned().collect()
                                        } else {
                                            cfg.module_order.iter().filter(|k| cfg.can_see_module(k)).cloned().collect()
                                        };
                                        rsx! {
                                            div { class: "category-block",
                                                div { class: "category-row",
                                                    button {
                                                        r#type: "button",
                                                        class: "category-row-left",
                                                        onclick: move |_| {
                                                            collapsed_categories.with_mut(|s| {
                                                                if s.contains("__ent_ws__") { s.remove("__ent_ws__"); } else { s.insert("__ent_ws__".into()); }
                                                            });
                                                        },
                                                        if is_ws_collapsed { ChevronRightIcon { size: 13 } } else { ChevronDownIcon { size: 13 } }
                                                        span { class: "category-name", "Work space" }
                                                    }
                                                }
                                                if !is_ws_collapsed {
                                                    for mod_key in visible_mods {
                                                        {
                                                            let is_active = active_ent_module.as_deref() == Some(mod_key.as_str());
                                                            let mk = mod_key.clone();
                                                            rsx! {
                                                                button {
                                                                    key: "ent-nav-{mod_key}",
                                                                    r#type: "button",
                                                                    class: if is_active { "channel-row selected" } else { "channel-row" },
                                                                    onclick: move |_| on_mod.call(mk.clone()),
                                                                    {module_icon(&mod_key)}
                                                                    span { class: "channel-row-name", "{module_label(&mod_key)}" }
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
                                                    span { class: "category-name", "{format_display_label(&cat.name)}" }
                                                }
                                                button {
                                                    r#type: "button",
                                                    class: "sidebar-section-btn",
                                                    title: "Create channel in {format_display_label(&cat.name)}",
                                                    aria_label: "Create channels in categories",
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
                                if loading {
                                    SkeletonChannels { count: 5 }
                                } else {
                                    div { class: "empty-chat-inner", "There are no channels yet" }
                                }
                            }
                        }
                    } else {
                        div { class: "empty-chat-inner", "Select a community from the list" }
                    }
                },

                NavSection::Communities => rsx! {
                    div { class: "sidebar-search",
                        div { class: "search-field",
                            SearchIcon { size: 16 }
                            input {
                                r#type: "search",
                                placeholder: "Find community",
                                aria_label: "Find community",
                                value: "{community_query}",
                                oninput: move |e| community_query.set(e.value()),
                            }
                        }
                    }
                    div { class: "sidebar-scrollable",
                        div { class: "sidebar-section-header", span { "Your community" } }
                        if servers_shown.is_empty() {
                            div { class: "sidebar-empty",
                                if cq.is_empty() { "You have not joined any community yet." } else { "There is no suitable community" }
                            }
                        } else {
                            for srv in servers_shown {
                                {
                                    let is_sel = selected_server.as_ref().map(|s| s.id == srv.id).unwrap_or(false);
                                    let s_click = srv.clone();
                                    let _ = server_initials(&srv.name);
                                    rsx! {
                                        button {
                                            key: "{srv.id}",
                                            r#type: "button",
                                            class: if is_sel { "chat-item community-item selected" } else { "chat-item community-item" },
                                            aria_label: "Community {srv.name}",
                                            onclick: move |_| on_select_server.call(s_click.clone()),
                                            Avatar { name: srv.name.clone(), avatar_url: srv.icon_url.clone(), size: 52 }
                                            div { class: "chat-item-info",
                                                span { class: "chat-item-name", "{srv.name}" }
                                                span { class: "chat-item-preview",
                                                    "{srv.member_count} members"
                                                    if srv.is_owner { " · You are the owner" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if !community_groups.is_empty() {
                            div { class: "sidebar-section-header", span { "Groups" } }
                            for grp in community_groups.iter().cloned() {
                                {
                                    let is_sel = active_group_id.as_deref() == Some(grp.id.as_str());
                                    let g_click = grp.clone();
                                    rsx! {
                                        button {
                                            key: "{grp.id}",
                                            r#type: "button",
                                            class: if is_sel { "chat-item community-item selected" } else { "chat-item community-item" },
                                            onclick: move |_| on_select_group.call(g_click.clone()),
                                            Avatar { name: grp.name.clone(), avatar_url: grp.avatar_url.clone(), size: 52 }
                                            div { class: "chat-item-info",
                                                span { class: "chat-item-name", "{grp.name}" }
                                                span { class: "chat-item-preview", "{grp.member_count} members · Group" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if loading && no_servers {
                            SkeletonConversations { count: 4 }
                        }
                        button {
                            r#type: "button",
                            class: "chat-item create-community-btn",
                            onclick: move |_| on_open_create_server.call(()),
                            span { class: "community-icon add", PlusIcon { size: 18 } }
                            span { class: "chat-item-info",
                                span { class: "chat-item-name", "Create or join community" }
                                span { class: "chat-item-preview", "By new name or invite link" }
                            }
                        }
                    }
                },

                NavSection::Settings | NavSection::Meet => rsx! {
                    div { class: "sidebar-top-header",
                        span { class: "sidebar-title", "Settings" }
                    }
                    div { class: "sidebar-scrollable",
                        div { style: "padding: 12px 10px; font-size: 13px; color: var(--text-secondary);",
                            "Manage your account, personal profile, interface and E2EE security key at the settings page."
                        }
                    }
                },
            }

            // Active Voice Channel Status Bar (docked above user footer)
            if let Some(vc) = active_voice_channel {
                div { class: "voice-status-panel",
                    div { class: "voice-status-info",
                        span { class: "voice-status-label", "Voice connected" }
                        span { class: "voice-status-channel", "{vc.name}" }
                    }
                    div { style: "display: flex; align-items: center; gap: 4px;",
                        button {
                            r#type: "button",
                            class: if is_voice_muted { "icon-btn danger" } else { "icon-btn" },
                            title: if is_voice_muted { "Turn on the microphone" } else { "Turn off the microphone" },
                            aria_label: "Switch microphone status",
                            onclick: move |_| on_toggle_voice_mute.call(()),
                            if is_voice_muted { MicOffIcon { size: 16 } } else { MicIcon { size: 16 } }
                        }
                        button {
                            r#type: "button",
                            class: "icon-btn danger",
                            title: "Disconnect the voice connection",
                            aria_label: "Disconnect the voice connection",
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
                span { class: "channel-row-name", "{format_display_label(&channel.name)}" }
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

#[cfg(test)]
mod tests {
    use super::format_display_label;

    #[test]
    fn cleans_hyphens_and_enforces_sentence_case() {
        assert_eq!(format_display_label("internal-announcement"), "Internal announcement");
        assert_eq!(format_display_label("executive board"), "Executive board");
        assert_eq!(format_display_label("Enterprise Space"), "Enterprise space");
        assert_eq!(format_display_label("Online Meeting Room"), "Online meeting room");
        assert_eq!(format_display_label("general"), "General");
        assert_eq!(format_display_label("Vico AI"), "Vico AI");
    }
}

