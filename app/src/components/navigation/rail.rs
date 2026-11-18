use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{FriendsIcon, LogoIcon, LogoutIcon, PlusIcon, SettingsIcon},
        navigation::NavSection,
    },
    models::ServerItem,
    platform,
};

fn server_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let a = parts[0].chars().next().unwrap_or('S');
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
pub fn NavigationRail(
    active_section: NavSection,
    servers: Vec<ServerItem>,
    unread_dm_count: usize,
    pending_friend_count: usize,
    current_username: String,
    current_avatar_url: Option<String>,
    hidden_mobile: bool,
    on_select_dm: EventHandler<()>,
    on_select_friends: EventHandler<()>,
    on_select_server: EventHandler<ServerItem>,
    on_open_create_server: EventHandler<()>,
    on_open_settings: EventHandler<()>,
    on_open_profile: EventHandler<()>,
    on_logout: EventHandler<()>,
) -> Element {
    let rail_cls = if hidden_mobile {
        "nav-rail hidden-mobile"
    } else {
        "nav-rail"
    };

    // Deduplicate servers defensively by ID in case of concurrent refreshes
    let mut seen = std::collections::HashSet::new();
    let unique_servers: Vec<ServerItem> = servers
        .into_iter()
        .filter(|s| seen.insert(s.id.clone()))
        .collect();

    rsx! {
        nav { class: "{rail_cls}", aria_label: "Điều hướng chính",
            // Direct Messages
            div {
                class: if active_section == NavSection::DirectMessages { "rail-slot active" } else { "rail-slot" },
                span { class: "rail-pill" }
                button {
                    r#type: "button",
                    class: "rail-btn",
                    title: "Tin nhắn riêng",
                    aria_label: "Tin nhắn riêng",
                    onclick: move |_| on_select_dm.call(()),
                    LogoIcon { size: 22 }
                    if unread_dm_count > 0 {
                        span { class: "rail-badge", "{format_badge(unread_dm_count)}" }
                    }
                }
            }

            // Friends
            div {
                class: if active_section == NavSection::Friends { "rail-slot active" } else { "rail-slot" },
                span { class: "rail-pill" }
                button {
                    r#type: "button",
                    class: "rail-btn",
                    title: "Bạn bè",
                    aria_label: "Bạn bè",
                    onclick: move |_| on_select_friends.call(()),
                    FriendsIcon {}
                    if pending_friend_count > 0 {
                        span { class: "rail-badge", "{format_badge(pending_friend_count)}" }
                    }
                }
            }

            div { class: "rail-divider" }

            // Community Servers Logo Rail
            div { class: "rail-scroll",
                for srv in unique_servers {
                    {
                        let is_selected = matches!(&active_section, NavSection::Server(id) if id == &srv.id);
                        let srv_click = srv.clone();
                        let initials = server_initials(&srv.name);
                        rsx! {
                            div {
                                key: "{srv.id}",
                                class: if is_selected { "rail-slot active" } else { "rail-slot" },
                                span { class: "rail-pill" }
                                button {
                                    r#type: "button",
                                    class: "rail-btn",
                                    title: "{srv.name}",
                                    aria_label: "Cộng đồng {srv.name}",
                                    onclick: move |_| on_select_server.call(srv_click.clone()),
                                    if let Some(icon) = srv.icon_url.as_ref().filter(|u| !u.is_empty()) {
                                        img { src: "{platform::resolve_asset_url(icon)}", alt: "{srv.name}" }
                                    } else {
                                        span { "{initials}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Add / Join Community
                div { class: "rail-slot",
                    button {
                        r#type: "button",
                        class: "rail-btn rail-action",
                        title: "Tạo hoặc tham gia cộng đồng",
                        aria_label: "Thêm cộng đồng",
                        onclick: move |_| on_open_create_server.call(()),
                        PlusIcon { size: 20 }
                    }
                }
            }

            // Bottom User / Settings / Logout
            div { class: "rail-footer",
                button {
                    r#type: "button",
                    class: "icon-btn",
                    title: "Hồ sơ của tôi ({current_username})",
                    aria_label: "Hồ sơ cá nhân",
                    onclick: move |_| on_open_profile.call(()),
                    Avatar {
                        name: current_username.clone(),
                        avatar_url: current_avatar_url.clone(),
                        size: 32,
                    }
                }
                button {
                    r#type: "button",
                    class: if active_section == NavSection::Settings { "icon-btn active" } else { "icon-btn" },
                    title: "Cài đặt",
                    aria_label: "Cài đặt",
                    onclick: move |_| on_open_settings.call(()),
                    SettingsIcon {}
                }
                button {
                    r#type: "button",
                    class: "icon-btn btn-danger",
                    title: "Đăng xuất",
                    aria_label: "Đăng xuất",
                    onclick: move |_| on_logout.call(()),
                    LogoutIcon {}
                }
            }
        }
    }
}

