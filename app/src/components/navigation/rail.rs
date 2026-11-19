//! ChatW left navigation column. Always shows the app sections (messages, friends,
//! communities, quick search); communities and group chats are only pinned here
//! when the user enables it in Settings. On phones it becomes a bottom tab bar.

use dioxus::prelude::*;

use crate::i18n::t;

use crate::{
    components::{
        avatar::Avatar,
        plugins_view::{ACTIVE_LEFT_PLUGIN, ACTIVE_PLUGIN_PAGE, EDITING_PLUGIN_ID, PluginGlyph, enabled_left_apps, enabled_rail_pages},
        icons::{CompassIcon, FriendsIcon, LogoIcon, LogoutIcon, MessageIcon, PluginIcon, SearchIcon, SettingsIcon, UserIcon},
        navigation::NavSection,
    },
    models::{GroupItem, ServerItem},
    platform,
};

fn badge(n: usize) -> String {
    if n > 99 { "99+".into() } else { n.to_string() }
}

fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [a, b, ..] => format!("{}{}", a.chars().next().unwrap_or('C'), b.chars().next().unwrap_or(' ')).to_uppercase(),
        _ => name.chars().take(2).collect::<String>().to_uppercase(),
    }
}

#[component]
pub fn AppRail(
    active_section: NavSection,
    hidden_mobile: bool,
    unread_dm_count: usize,
    pending_friend_count: usize,
    current_username: String,
    current_avatar_url: Option<String>,
    pinned_servers: Vec<ServerItem>,
    pinned_groups: Vec<GroupItem>,
    active_group_id: Option<String>,
    on_select_dm: EventHandler<()>,
    on_select_friends: EventHandler<()>,
    on_select_communities: EventHandler<()>,
    on_quick_switch: EventHandler<()>,
    on_select_server: EventHandler<ServerItem>,
    on_select_group: EventHandler<GroupItem>,
    on_open_profile: EventHandler<()>,
    on_open_settings: EventHandler<()>,
    on_logout: EventHandler<()>,
    #[props(default)] on_select_search: Option<EventHandler<()>>,
) -> Element {
    let mut menu_open = use_signal(|| false);
    let active_mod_page = ACTIVE_PLUGIN_PAGE.read().clone();
    let active_left_plugin = ACTIVE_LEFT_PLUGIN.read().clone();
    let in_mod = active_mod_page.is_some();
    let in_left = !in_mod && active_left_plugin.is_some();
    let in_search = !in_mod && active_section == NavSection::Search;
    let in_dm = !in_mod && !in_left && active_section == NavSection::DirectMessages;
    let in_friends = !in_mod && !in_left && active_section == NavSection::Friends;
    let in_comm = !in_mod && !in_left && matches!(active_section, NavSection::Communities | NavSection::Server(_));
    let in_settings = !in_mod && active_section == NavSection::Settings;
    let left_apps = enabled_left_apps();
    let custom_pages = enabled_rail_pages();
    let switch_to_dm = in_settings || active_section == NavSection::Search;
    let mut cls = String::from("app-rail");
    if hidden_mobile { cls.push_str(" hidden-mobile"); }

    rsx! {
        nav { class: "{cls}", aria_label: "Điều hướng chính",
            div { class: "rail-brand", title: "chatw", aria_hidden: "true",
                LogoIcon { size: 22 }
            }
            div { class: "rail-nav",
                button {
                    r#type: "button",
                    class: if in_dm { "rail-item active" } else { "rail-item" },
                    "data-tip": "{t(\"nav.messages\")}",
                    aria_label: "{t(\"nav.messages\")}",
                    aria_current: if in_dm { "page" } else { "false" },
                    onclick: move |_| { *ACTIVE_PLUGIN_PAGE.write() = None; *ACTIVE_LEFT_PLUGIN.write() = None; on_select_dm.call(()); },
                    span { class: "rail-icon", MessageIcon { size: 20 }
                        if unread_dm_count > 0 { span { class: "rail-count", "{badge(unread_dm_count)}" } }
                    }
                }
                button {
                    r#type: "button",
                    class: if in_friends { "rail-item active" } else { "rail-item" },
                    "data-tip": "{t(\"nav.friends\")}",
                    aria_label: "{t(\"nav.friends\")}",
                    aria_current: if in_friends { "page" } else { "false" },
                    onclick: move |_| { *ACTIVE_PLUGIN_PAGE.write() = None; *ACTIVE_LEFT_PLUGIN.write() = None; on_select_friends.call(()); },
                    span { class: "rail-icon", FriendsIcon {}
                        if pending_friend_count > 0 { span { class: "rail-count", "{badge(pending_friend_count)}" } }
                    }
                }
                button {
                    r#type: "button",
                    class: if in_comm { "rail-item active" } else { "rail-item" },
                    "data-tip": "{t(\"nav.communities\")}",
                    aria_label: "{t(\"nav.communities\")}",
                    aria_current: if in_comm { "page" } else { "false" },
                    onclick: move |_| { *ACTIVE_PLUGIN_PAGE.write() = None; *ACTIVE_LEFT_PLUGIN.write() = None; on_select_communities.call(()); },
                    span { class: "rail-icon", CompassIcon { size: 20 } }
                }
                button {
                    r#type: "button",
                    class: if in_search { "rail-item active" } else { "rail-item" },
                    "data-tip": "{t(\"nav.search\")}",
                    aria_label: "{t(\"nav.search\")}",
                    aria_current: if in_search { "page" } else { "false" },
                    onclick: move |_| {
                        *ACTIVE_PLUGIN_PAGE.write() = None;
                        *ACTIVE_LEFT_PLUGIN.write() = None;
                        match on_select_search { Some(h) => h.call(()), None => on_quick_switch.call(()) }
                    },
                    span { class: "rail-icon", SearchIcon { size: 20 } }
                }
                button {
                    r#type: "button",
                    class: if active_mod_page.as_deref() == Some("studio") { "rail-item active" } else { "rail-item" },
                    "data-tip": "Plugin",
                    aria_label: "Plugin",
                    aria_current: if active_mod_page.as_deref() == Some("studio") { "page" } else { "false" },
                    onclick: move |_| {
                        *ACTIVE_LEFT_PLUGIN.write() = None;
                        *EDITING_PLUGIN_ID.write() = None;
                        *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                    },
                    span { class: "rail-icon", PluginIcon { size: 20 } }
                }
                for (pid, picon, plogo, ptitle) in left_apps {
                    {
                        let is_act = active_left_plugin.as_deref() == Some(pid.as_str()) && !in_mod;
                        let is_logo = !plogo.trim().is_empty() && !plogo.trim().starts_with("svg:");
                        let g_size = if is_logo { 40 } else { 20 };
                        let cls = match (is_logo, is_act) {
                            (true, true) => "rail-item has-logo active",
                            (true, false) => "rail-item has-logo",
                            (false, true) => "rail-item active",
                            (false, false) => "rail-item",
                        };
                        let pid2 = pid.clone();
                        rsx! {
                            button {
                                key: "left-{pid}",
                                r#type: "button",
                                class: "{cls}",
                                "data-tip": "{ptitle}",
                                aria_label: "{ptitle}",
                                aria_current: if is_act { "page" } else { "false" },
                                onclick: move |_| {
                                    *ACTIVE_PLUGIN_PAGE.write() = None;
                                    if is_act {
                                        *ACTIVE_LEFT_PLUGIN.write() = None;
                                    } else {
                                        *ACTIVE_LEFT_PLUGIN.write() = Some(pid2.clone());
                                        if switch_to_dm {
                                            on_select_dm.call(());
                                        }
                                    }
                                },
                                span { class: "rail-icon", PluginGlyph { icon: picon, logo_url: plogo, size: g_size } }
                            }
                        }
                    }
                }
                for (pid, picon, plogo, ptitle) in custom_pages {
                    {
                        let is_act = active_mod_page.as_deref() == Some(pid.as_str());
                        let is_logo = !plogo.trim().is_empty() && !plogo.trim().starts_with("svg:");
                        let g_size = if is_logo { 40 } else { 20 };
                        let cls = match (is_logo, is_act) {
                            (true, true) => "rail-item has-logo active",
                            (true, false) => "rail-item has-logo",
                            (false, true) => "rail-item active",
                            (false, false) => "rail-item",
                        };
                        let pid2 = pid.clone();
                        rsx! {
                            button {
                                key: "mod-{pid}",
                                r#type: "button",
                                class: "{cls}",
                                "data-tip": "{ptitle}",
                                aria_label: "{ptitle}",
                                onclick: move |_| {
                                    *ACTIVE_LEFT_PLUGIN.write() = None;
                                    *ACTIVE_PLUGIN_PAGE.write() = Some(pid2.clone());
                                },
                                span { class: "rail-icon", PluginGlyph { icon: picon, logo_url: plogo, size: g_size } }
                            }
                        }
                    }
                }
            }

            if !pinned_servers.is_empty() || !pinned_groups.is_empty() {
                div { class: "rail-pins", aria_label: "Đã ghim",
                    for srv in pinned_servers {
                        {
                            let active = matches!(&active_section, NavSection::Server(id) if id == &srv.id);
                            let s = srv.clone();
                            rsx! {
                                button {
                                    key: "s-{srv.id}",
                                    r#type: "button",
                                    class: if active { "rail-pin community active" } else { "rail-pin community" },
                                    "data-tip": "{srv.name}",
                                    aria_label: "Cộng đồng {srv.name}",
                                    onclick: move |_| on_select_server.call(s.clone()),
                                    if let Some(icon) = srv.icon_url.as_ref().filter(|u| !u.is_empty()) {
                                        img { src: "{platform::resolve_asset_url(icon)}", alt: "" }
                                    } else {
                                        "{initials(&srv.name)}"
                                    }
                                }
                            }
                        }
                    }
                    for grp in pinned_groups {
                        {
                            let active = active_group_id.as_deref() == Some(grp.id.as_str());
                            let g = grp.clone();
                            rsx! {
                                button {
                                    key: "g-{grp.id}",
                                    r#type: "button",
                                    class: if active { "rail-pin group active" } else { "rail-pin group" },
                                    "data-tip": "{grp.name}",
                                    aria_label: "Nhóm {grp.name}",
                                    onclick: move |_| on_select_group.call(g.clone()),
                                    Avatar { name: grp.name.clone(), avatar_url: grp.avatar_url.clone(), size: 40 }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "rail-spacer" }

            div { class: "rail-bottom",
                button {
                    r#type: "button",
                    class: if in_settings { "rail-item compact active" } else { "rail-item compact" },
                    "data-tip": "{t(\"nav.settings\")}",
                    aria_label: "{t(\"nav.settings\")}",
                    onclick: move |_| { *ACTIVE_PLUGIN_PAGE.write() = None; *ACTIVE_LEFT_PLUGIN.write() = None; on_open_settings.call(()); },
                    span { class: "rail-icon", SettingsIcon {} }
                }
                button {
                    r#type: "button",
                    class: "rail-account",
                    title: "Tài khoản: {current_username}",
                    aria_label: "Tài khoản của tôi",
                    aria_haspopup: "menu",
                    aria_expanded: "{menu_open()}",
                    onclick: move |_| menu_open.set(!menu_open()),
                    Avatar { name: current_username.clone(), avatar_url: current_avatar_url.clone(), size: 38 }
                }
                if menu_open() {
                    button { r#type: "button", class: "menu-dismiss", aria_label: "Đóng menu", tabindex: "-1", onclick: move |_| menu_open.set(false) }
                    div { class: "account-menu rail-menu", role: "menu",
                        div { class: "menu-header", "{current_username}" }
                        button { r#type: "button", class: "menu-item", role: "menuitem",
                            onclick: move |_| { menu_open.set(false); on_open_profile.call(()); },
                            UserIcon {} span { "Hồ sơ của tôi" }
                        }
                        button { r#type: "button", class: "menu-item", role: "menuitem", aria_label: "Mở cài đặt",
                            onclick: move |_| { menu_open.set(false); on_open_settings.call(()); },
                            SettingsIcon {} span { "Cài đặt" }
                        }
                        crate::components::accounts::AccountSwitcher { on_close: move |_| menu_open.set(false) }
                        button { r#type: "button", class: "menu-item danger", role: "menuitem",
                            onclick: move |_| { menu_open.set(false); on_logout.call(()); },
                            LogoutIcon {} span { "Đăng xuất" }
                        }
                    }
                }
            }
        }
    }
}
