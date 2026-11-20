//! Dedicated Notifications full page (`NavSection::Notifications`).
//! Shows real unread direct messages and incoming friend requests.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BellIcon, FriendsIcon, MessageIcon},
    },
    models::{FriendItem, PendingRequestItem},
};

#[component]
pub fn NotificationsView(
    friends: Vec<FriendItem>,
    unread_counts: HashMap<String, usize>,
    pending_requests: Vec<PendingRequestItem>,
    on_open_friend: EventHandler<FriendItem>,
    on_open_friends_tab: EventHandler<()>,
    on_open_messages: EventHandler<()>,
) -> Element {
    let mut filter = use_signal(|| "all".to_string());

    let unread_friends: Vec<(FriendItem, usize)> = friends
        .iter()
        .filter_map(|f| unread_counts.get(&f.id).copied().filter(|&c| c > 0).map(|c| (f.clone(), c)))
        .collect();

    let total_unread_msgs: usize = unread_friends.iter().map(|(_, c)| *c).sum();
    let total_pending = pending_requests.len();
    let show_msgs = filter() == "all" || filter() == "messages";
    let show_reqs = filter() == "all" || filter() == "requests";
    let visible_empty = (!show_msgs || unread_friends.is_empty()) && (!show_reqs || pending_requests.is_empty());

    rsx! {
        div { class: "notifications-page",
            header { class: "notifications-header",
                div { class: "notifications-title-row",
                    h1 { class: "notifications-title", "Thông báo" }
                    if total_unread_msgs + total_pending > 0 {
                        span { class: "notifications-count-pill", "{total_unread_msgs + total_pending} mới" }
                    }
                }
                div { class: "notifications-tabs", role: "tablist", aria_label: "Lọc thông báo",
                    button {
                        r#type: "button",
                        role: "tab",
                        class: if filter() == "all" { "download-tab active" } else { "download-tab" },
                        onclick: move |_| filter.set("all".to_string()),
                        "Tất cả"
                    }
                    button {
                        r#type: "button",
                        role: "tab",
                        class: if filter() == "messages" { "download-tab active" } else { "download-tab" },
                        onclick: move |_| filter.set("messages".to_string()),
                        "Tin nhắn ({total_unread_msgs})"
                    }
                    button {
                        r#type: "button",
                        role: "tab",
                        class: if filter() == "requests" { "download-tab active" } else { "download-tab" },
                        onclick: move |_| filter.set("requests".to_string()),
                        "Kết bạn ({total_pending})"
                    }
                }
            }

            div { class: "notifications-scroll",
                div { class: "notifications-inner",
                    if show_reqs && !pending_requests.is_empty() {
                        section { class: "notifications-section",
                            div { class: "notifications-section-head",
                                h2 { "Lời mời kết bạn" }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| on_open_friends_tab.call(()),
                                    "Mở danh sách bạn bè"
                                }
                            }
                            div { class: "notifications-list",
                                for req in pending_requests.clone() {
                                    button {
                                        key: "req-{req.id}",
                                        r#type: "button",
                                        class: "notification-row",
                                        onclick: move |_| on_open_friends_tab.call(()),
                                        Avatar { name: req.sender_username.clone(), avatar_url: req.sender_avatar_url.clone(), size: 42 }
                                        div { class: "notification-row-main",
                                            strong { "{req.sender_username}" }
                                            span { "Đã gửi lời mời kết bạn cho bạn" }
                                        }
                                        span { class: "notification-row-tag",
                                            FriendsIcon { size: 15 }
                                            span { "Xem" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if show_msgs && !unread_friends.is_empty() {
                        section { class: "notifications-section",
                            div { class: "notifications-section-head",
                                h2 { "Tin nhắn chưa đọc" }
                            }
                            div { class: "notifications-list",
                                for (friend, count) in unread_friends.clone() {
                                    {
                                        let fr = friend.clone();
                                        let label = friend.display_name.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| friend.username.clone());
                                        rsx! {
                                            button {
                                                key: "dm-{friend.id}",
                                                r#type: "button",
                                                class: "notification-row",
                                                onclick: move |_| on_open_friend.call(fr.clone()),
                                                Avatar { name: friend.username.clone(), avatar_url: friend.avatar_url.clone(), size: 42 }
                                                div { class: "notification-row-main",
                                                    strong { "{label}" }
                                                    span { "{count} tin nhắn mới đang chờ bạn đọc" }
                                                }
                                                span { class: "notification-row-badge", "{count}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if visible_empty {
                        div { class: "notifications-empty",
                            div { class: "notifications-empty-icon", BellIcon { size: 28 } }
                            h2 { "Không có thông báo mới" }
                            p { "Mọi tin nhắn chưa đọc và lời mời kết bạn của bạn sẽ hiển thị tập trung tại đây." }
                            div { class: "notifications-empty-actions",
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: move |_| on_open_messages.call(()),
                                    MessageIcon { size: 16 }
                                    span { "Về tin nhắn" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| on_open_friends_tab.call(()),
                                    FriendsIcon { size: 16 }
                                    span { "Tìm bạn bè" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
