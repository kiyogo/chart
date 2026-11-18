//! Unified conversation list for the Messages section: "Chính mình" (self
//! chat), pinned conversations, direct messages, simple groups, channels popped
//! out of communities and bot chats. Each row has a menu for pinning, labels,
//! muting, moving a group to Communities, blocking and more.

use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BellOffIcon, BotIcon, CheckIcon, ChevronDownIcon, DevicesIcon, HashIcon, MoreIcon, PinIcon, PlusIcon, SearchIcon, TagIcon, TrashIcon},
        messages::preview_text,
        profile_card::{name_effect_class, nameplate_style},
    },
    i18n::t,
    models::{BotItem, ChannelItem, ChatLabel, ConvPref, FriendItem, GroupItem},
    services::social,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ConvOpen {
    SelfChat,
    Dm(FriendItem),
    Group(GroupItem),
    Channel(ChannelItem),
    Bot(BotItem),
}

/// A channel popped out of a community into Messages.
#[derive(Clone, Debug, PartialEq)]
pub struct PoppedChannel {
    pub channel: ChannelItem,
    pub server_name: String,
}

pub const LABEL_COLORS: [&str; 10] = ["#E5484D", "#F76B15", "#FFC53D", "#30A46C", "#12A594", "#0090FF", "#3E63DD", "#8E4EC6", "#D6409F", "#8B8D98"];

#[derive(Clone, PartialEq)]
struct Row {
    key: String,
    conv_type: &'static str,
    conv_id: String,
    name: String,
    avatar: Option<String>,
    preview: String,
    unread: usize,
    pinned_at: Option<String>,
    labels: Vec<ChatLabel>,
    muted: bool,
    open: ConvOpen,
    np: (String, String),
    name_fx: String,
    blocked: bool,
    in_communities: bool,
    order: usize,
}

#[component]
pub fn ConversationList(
    my_id: String,
    friends: Vec<FriendItem>,
    groups: Vec<GroupItem>,
    bots: Vec<BotItem>,
    popouts: Vec<PoppedChannel>,
    prefs: Vec<ConvPref>,
    labels: Vec<ChatLabel>,
    unread_counts: HashMap<String, usize>,
    last_previews: HashMap<String, String>,
    /// Self chat exists ("Chính mình").
    self_chat: bool,
    active_key: Option<String>,
    blocked: Vec<String>,
    on_open: EventHandler<ConvOpen>,
    /// (conv_type, conv_id, fields) for `/api/conversations/prefs`.
    on_pref: EventHandler<(String, String, serde_json::Value)>,
    on_create_group: EventHandler<()>,
    on_create_self: EventHandler<()>,
    on_delete_self: EventHandler<()>,
    on_labels_changed: EventHandler<Vec<ChatLabel>>,
    on_block: EventHandler<(String, bool)>,
    on_mark_read: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| "all".to_string());
    let mut menu_for = use_signal(|| Option::<String>::None);
    let mut label_menu = use_signal(|| false);
    let mut manage_labels = use_signal(|| false);
    let mut confirm_delete_self = use_signal(|| false);

    let pref_of = |t: &str, id: &str| prefs.iter().find(|p| p.conv_type == t && p.conv_id == id).cloned();
    let label_of = |ids: &[String]| -> Vec<ChatLabel> { labels.iter().filter(|l| ids.contains(&l.id)).cloned().collect() };

    let mut rows: Vec<Row> = Vec::new();
    if self_chat {
        let p = pref_of("self", &my_id);
        rows.push(Row {
            key: "self".into(),
            conv_type: "self",
            conv_id: my_id.clone(),
            name: t("chat.self").to_string(),
            avatar: None,
            preview: last_previews.get(&my_id).map(|s| preview_text(&strip_device_prefix(s))).unwrap_or_else(|| "Ghi chú, tệp và liên kết giữa các thiết bị của bạn".to_string()),
            unread: 0,
            pinned_at: p.as_ref().and_then(|p| p.pinned_at.clone()),
            labels: p.as_ref().map(|p| label_of(&p.label_ids)).unwrap_or_default(),
            muted: false,
            open: ConvOpen::SelfChat,
            np: (String::new(), String::new()),
            name_fx: String::new(),
            blocked: false,
            in_communities: false,
            order: 0,
        });
    }
    for (i, f) in friends.iter().enumerate() {
        let p = pref_of("dm", &f.id);
        rows.push(Row {
            key: format!("dm:{}", f.id),
            conv_type: "dm",
            conv_id: f.id.clone(),
            name: f.display_name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| f.username.clone()),
            avatar: f.avatar_url.clone(),
            preview: last_previews.get(&f.id).map(|s| preview_text(s)).unwrap_or_else(|| "Bắt đầu trò chuyện".to_string()),
            unread: unread_counts.get(&f.id).copied().unwrap_or(0),
            pinned_at: p.as_ref().and_then(|p| p.pinned_at.clone()),
            labels: p.as_ref().map(|p| label_of(&p.label_ids)).unwrap_or_default(),
            muted: p.as_ref().is_some_and(|p| p.muted),
            open: ConvOpen::Dm(f.clone()),
            np: nameplate_style(f.nameplate.as_deref(), f.custom_nameplate_url.as_deref()),
            name_fx: name_effect_class(f.name_effect.as_deref()),
            blocked: blocked.contains(&f.id),
            in_communities: false,
            order: 10 + i,
        });
    }
    for (i, g) in groups.iter().enumerate() {
        let p = pref_of("group", &g.id);
        let in_comm = p.as_ref().is_some_and(|p| p.in_communities);
        if in_comm {
            continue;
        }
        rows.push(Row {
            key: format!("group:{}", g.id),
            conv_type: "group",
            conv_id: g.id.clone(),
            name: g.name.clone(),
            avatar: g.avatar_url.clone(),
            preview: last_previews.get(&g.id).map(|s| preview_text(s)).unwrap_or_else(|| format!("{} thành viên", g.member_count)),
            unread: unread_counts.get(&g.id).copied().unwrap_or(0),
            pinned_at: p.as_ref().and_then(|p| p.pinned_at.clone()),
            labels: p.as_ref().map(|p| label_of(&p.label_ids)).unwrap_or_default(),
            muted: p.as_ref().is_some_and(|p| p.muted),
            open: ConvOpen::Group(g.clone()),
            np: (String::new(), String::new()),
            name_fx: String::new(),
            blocked: false,
            in_communities: in_comm,
            order: 5000 + i,
        });
    }
    for (i, pc) in popouts.iter().enumerate() {
        let p = pref_of("channel", &pc.channel.id);
        rows.push(Row {
            key: format!("channel:{}", pc.channel.id),
            conv_type: "channel",
            conv_id: pc.channel.id.clone(),
            name: format!("#{}", pc.channel.name),
            avatar: None,
            preview: pc.server_name.clone(),
            unread: unread_counts.get(&pc.channel.id).copied().unwrap_or(0),
            pinned_at: p.as_ref().and_then(|p| p.pinned_at.clone()),
            labels: p.as_ref().map(|p| label_of(&p.label_ids)).unwrap_or_default(),
            muted: p.as_ref().is_some_and(|p| p.muted),
            open: ConvOpen::Channel(pc.channel.clone()),
            np: (String::new(), String::new()),
            name_fx: String::new(),
            blocked: false,
            in_communities: false,
            order: 8000 + i,
        });
    }
    for (i, b) in bots.iter().enumerate() {
        let p = pref_of("bot", &b.id);
        rows.push(Row {
            key: format!("bot:{}", b.id),
            conv_type: "bot",
            conv_id: b.id.clone(),
            name: b.display_name.clone(),
            avatar: b.avatar_url.clone(),
            preview: b.last_text.clone().map(|s| preview_text(&s)).unwrap_or_else(|| format!("@{}", b.username)),
            unread: unread_counts.get(&b.id).copied().unwrap_or(0),
            pinned_at: p.as_ref().and_then(|p| p.pinned_at.clone()),
            labels: p.as_ref().map(|p| label_of(&p.label_ids)).unwrap_or_default(),
            muted: p.as_ref().is_some_and(|p| p.muted),
            open: ConvOpen::Bot(b.clone()),
            np: (String::new(), String::new()),
            name_fx: String::new(),
            blocked: false,
            in_communities: false,
            order: 9000 + i,
        });
    }

    let q = crate::components::composer::fold(query().trim());
    let f = filter();
    rows.retain(|r| q.is_empty() || crate::components::composer::fold(&r.name).contains(&q));
    rows.retain(|r| match f.as_str() {
        "all" => true,
        "unread" => r.unread > 0,
        id => r.labels.iter().any(|l| l.id == id),
    });
    // Pinned first (most recently pinned on top), then "Chính mình", then the rest.
    rows.sort_by(|a, b| match (&a.pinned_at, &b.pinned_at) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.order.cmp(&b.order),
    });
    let pinned: Vec<Row> = rows.iter().filter(|r| r.pinned_at.is_some()).cloned().collect();
    let rest: Vec<Row> = rows.iter().filter(|r| r.pinned_at.is_none()).cloned().collect();
    let active_label = labels.iter().find(|l| l.id == f).cloned();
    let label_counts: HashMap<String, usize> = labels
        .iter()
        .map(|l| (l.id.clone(), prefs.iter().filter(|p| p.label_ids.contains(&l.id)).count()))
        .collect();

    let labels_rows = labels.clone();
    let render_row = move |r: Row| {
        let selected = active_key.as_deref() == Some(r.key.as_str());
        let open = r.open.clone();
        let key_menu = r.key.clone();
        let key_ctx = r.key.clone();
        let menu_open = menu_for().as_deref() == Some(r.key.as_str());
        let (np_cls, np_style) = r.np.clone();
        let cls = format!(
            "chat-item {}{}{}",
            np_cls,
            if selected { " selected" } else { "" },
            if r.blocked { " is-blocked" } else { "" }
        );
        let r2 = r.clone();
        rsx! {
            div { key: "{r.key}", class: "conv-row",
                oncontextmenu: move |e| { e.prevent_default(); menu_for.set(Some(key_ctx.clone())); },
                button {
                    r#type: "button",
                    class: "{cls}",
                    style: "{np_style}",
                    "data-conv": "{r.key}",
                    onclick: move |_| on_open.call(open.clone()),
                    match &r.open {
                        ConvOpen::SelfChat => rsx! { span { class: "self-chat-avatar", aria_hidden: "true", DevicesIcon { size: 22 } } },
                        ConvOpen::Channel(_) => rsx! { span { class: "self-chat-avatar channel", aria_hidden: "true", HashIcon { size: 20 } } },
                        _ => rsx! { Avatar { name: r.name.clone(), avatar_url: r.avatar.clone(), size: 44 } },
                    }
                    div { class: "chat-item-info",
                        div { class: "chat-item-header",
                            span { class: if r.unread > 0 { "chat-item-name unread {r.name_fx}" } else { "chat-item-name {r.name_fx}" }, "{r.name}" }
                            if matches!(r.open, ConvOpen::Bot(_)) { span { class: "bot-tag", "BOT" } }
                            if r.muted { span { class: "chat-item-icon", title: "Đã tắt thông báo", BellOffIcon { size: 13 } } }
                            if r.pinned_at.is_some() { span { class: "chat-item-icon", title: "Đã ghim", PinIcon { size: 13 } } }
                            if r.unread > 0 {
                                span { class: if r.muted { "chat-item-badge muted" } else { "chat-item-badge" }, "{badge(r.unread)}" }
                            }
                        }
                        div { class: if r.unread > 0 { "chat-item-preview unread" } else { "chat-item-preview" },
                            if r.blocked { "Đã chặn" } else { "{r.preview}" }
                        }
                        if !r.labels.is_empty() {
                            div { class: "chat-item-labels",
                                for l in r.labels.iter() {
                                    span { key: "{l.id}", class: "label-pill", style: "--label: {l.color};", "{l.name}" }
                                }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    class: if menu_open { "conv-more active" } else { "conv-more" },
                    title: "Tùy chọn",
                    aria_label: "Tùy chọn cho {r.name}",
                    aria_expanded: "{menu_open}",
                    onclick: move |e| { e.stop_propagation(); menu_for.set(if menu_for().as_deref() == Some(key_menu.as_str()) { None } else { Some(key_menu.clone()) }); },
                    MoreIcon { size: 16 }
                }
                if menu_open {
                    ConvMenu {
                        row_key: r2.key.clone(),
                        conv_type: r2.conv_type.to_string(),
                        conv_id: r2.conv_id.clone(),
                        pinned: r2.pinned_at.is_some(),
                        muted: r2.muted,
                        unread: r2.unread,
                        blocked: r2.blocked,
                        in_communities: r2.in_communities,
                        label_ids: r2.labels.iter().map(|l| l.id.clone()).collect::<Vec<_>>(),
                        labels: labels_rows.clone(),
                        on_close: move |_| menu_for.set(None),
                        on_pref: move |v| { menu_for.set(None); on_pref.call(v) },
                        on_manage_labels: move |_| { menu_for.set(None); manage_labels.set(true); },
                        on_block: move |v| { menu_for.set(None); on_block.call(v) },
                        on_mark_read: move |id| { menu_for.set(None); on_mark_read.call(id) },
                        on_delete_self: move |_| { menu_for.set(None); confirm_delete_self.set(true); },
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "sidebar-search",
            div { class: "search-field",
                SearchIcon { size: 16 }
                input {
                    r#type: "search",
                    placeholder: "{t(\"chat.search_conv\")}",
                    aria_label: "{t(\"chat.search_conv\")}",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }
        }
        div { class: "sidebar-filters", role: "tablist", aria_label: "Lọc cuộc trò chuyện",
            for (val, label) in [("all", t("chat.all")), ("unread", t("chat.unread"))] {
                button {
                    key: "{val}",
                    r#type: "button",
                    role: "tab",
                    aria_selected: "{f == val}",
                    class: if f == val { "filter-chip active" } else { "filter-chip" },
                    onclick: move |_| filter.set(val.to_string()),
                    "{label}"
                }
            }
            div { class: "label-filter",
                button {
                    r#type: "button",
                    class: if active_label.is_some() { "filter-chip active" } else { "filter-chip" },
                    aria_haspopup: "menu",
                    aria_expanded: "{label_menu()}",
                    title: "Lọc theo thẻ phân loại",
                    onclick: move |e| { e.stop_propagation(); label_menu.set(!label_menu()); },
                    if let Some(l) = active_label.as_ref() {
                        span { class: "label-dot", style: "--label: {l.color};" }
                        "{l.name}"
                    } else {
                        TagIcon { size: 14 }
                        "Phân loại"
                    }
                    ChevronDownIcon { size: 14 }
                }
                if label_menu() {
                    button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Đóng", onclick: move |_| label_menu.set(false) }
                    div { class: "account-menu label-menu", role: "menu",
                        if labels.is_empty() {
                            div { class: "menu-note", "Chưa có thẻ phân loại nào" }
                        }
                        for l in labels.iter().cloned() {
                            {
                                let id = l.id.clone();
                                let count = label_counts.get(&l.id).copied().unwrap_or(0);
                                rsx! {
                                    button { key: "{l.id}", r#type: "button", role: "menuitemradio", aria_checked: "{f == l.id}", class: "account-menu-item",
                                        onclick: move |_| { filter.set(id.clone()); label_menu.set(false); },
                                        span { class: "label-dot", style: "--label: {l.color};" }
                                        span { class: "grow", "{l.name}" }
                                        span { class: "menu-count", "{count}" }
                                    }
                                }
                            }
                        }
                        if active_label.is_some() {
                            button { r#type: "button", class: "account-menu-item", onclick: move |_| { filter.set("all".into()); label_menu.set(false); }, "Bỏ lọc" }
                        }
                        button { r#type: "button", class: "account-menu-item", onclick: move |_| { label_menu.set(false); manage_labels.set(true); },
                            TagIcon { size: 16 } "Quản lý thẻ phân loại"
                        }
                    }
                }
            }
        }
        div { class: "sidebar-scrollable",
            if !self_chat && q.is_empty() && f == "all" {
                button { r#type: "button", class: "self-chat-create", onclick: move |_| on_create_self.call(()),
                    span { class: "self-chat-avatar small", DevicesIcon { size: 16 } }
                    span { "Tạo lại \"{t(\"chat.self\")}\" để lưu ghi chú giữa các thiết bị" }
                }
            }
            if !pinned.is_empty() {
                div { class: "sidebar-section-header", span { "{t(\"chat.pinned\")}" } }
                for r in pinned { {render_row(r)} }
            }
            div { class: "sidebar-section-header",
                span { "{t(\"nav.messages\")}" }
                button {
                    r#type: "button",
                    class: "sidebar-section-btn",
                    title: "Tạo nhóm",
                    aria_label: "Tạo nhóm",
                    onclick: move |_| on_create_group.call(()),
                    PlusIcon { size: 14 }
                }
            }
            if rest.is_empty() && rows.is_empty() {
                div { class: "sidebar-empty",
                    if friends.is_empty() && groups.is_empty() { "Chưa có cuộc trò chuyện. Mở mục Bạn bè để kết bạn." } else { "Không có cuộc trò chuyện phù hợp" }
                }
            }
            for r in rest { {render_row(r)} }
        }
        if manage_labels() {
            LabelManager {
                labels: labels.clone(),
                on_close: move |_| manage_labels.set(false),
                on_changed: move |l: Vec<ChatLabel>| on_labels_changed.call(l),
            }
        }
        if confirm_delete_self() {
            div { class: "modal-overlay", onclick: move |_| confirm_delete_self.set(false),
                div { class: "modal-content compact", role: "alertdialog", aria_modal: "true", aria_labelledby: "del-self-title", onclick: move |e| e.stop_propagation(),
                    h3 { id: "del-self-title", class: "modal-title", "Xóa \"{t(\"chat.self\")}\"?" }
                    p { class: "modal-sub", "Toàn bộ ghi chú trong cuộc trò chuyện với chính mình sẽ bị xóa vĩnh viễn trên mọi thiết bị. Bạn có thể tạo lại sau, nhưng nội dung cũ không khôi phục được." }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_delete_self.set(false), "{t(\"act.cancel\")}" }
                        button { r#type: "button", class: "btn-danger", onclick: move |_| { confirm_delete_self.set(false); on_delete_self.call(()); }, TrashIcon { size: 16 } "Xóa vĩnh viễn" }
                    }
                }
            }
        }
    }
}

fn badge(n: usize) -> String {
    if n > 99 { "99+".into() } else { n.to_string() }
}

/// Self-chat messages carry the sending device label: "\u{2060}Linux 1\u{2060}text".
pub fn split_device_prefix(text: &str) -> (Option<String>, String) {
    if let Some(rest) = text.strip_prefix('\u{2060}') {
        if let Some(end) = rest.find('\u{2060}') {
            let label = rest[..end].trim();
            // "session-id|label": a 36-char id plus a label of up to 40 characters.
            if !label.is_empty() && label.chars().count() <= 120 {
                return (Some(label.to_string()), rest[end + '\u{2060}'.len_utf8()..].to_string());
            }
        }
    }
    (None, text.to_string())
}

pub fn strip_device_prefix(text: &str) -> String {
    split_device_prefix(text).1
}

#[component]
fn ConvMenu(
    row_key: String,
    conv_type: String,
    conv_id: String,
    pinned: bool,
    muted: bool,
    unread: usize,
    blocked: bool,
    in_communities: bool,
    label_ids: Vec<String>,
    labels: Vec<ChatLabel>,
    on_close: EventHandler<()>,
    on_pref: EventHandler<(String, String, serde_json::Value)>,
    on_manage_labels: EventHandler<()>,
    on_block: EventHandler<(String, bool)>,
    on_mark_read: EventHandler<String>,
    on_delete_self: EventHandler<()>,
) -> Element {
    let mut show_labels = use_signal(|| false);
    let ct = conv_type.clone();
    let id = conv_id.clone();
    let pref = move |fields: serde_json::Value| on_pref.call((ct.clone(), id.clone(), fields));
    let pref_pin = pref.clone();
    let pref_mute = pref.clone();
    let pref_comm = pref.clone();
    let pref_out = pref.clone();
    let pref_label = pref.clone();
    let id_read = conv_id.clone();
    let id_block = conv_id.clone();
    let _ = row_key;
    rsx! {
        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Đóng", onclick: move |_| on_close.call(()) }
        div { class: "account-menu conv-menu", role: "menu",
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| pref_pin(json!({ "pinned": !pinned })),
                PinIcon { size: 16 } if pinned { "Bỏ ghim" } else { "Ghim lên đầu" }
            }
            if unread > 0 {
                button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| on_mark_read.call(id_read.clone()),
                    CheckIcon { size: 16 } "Đánh dấu đã đọc"
                }
            }
            button { r#type: "button", role: "menuitem", class: "account-menu-item", aria_expanded: "{show_labels()}", onclick: move |_| show_labels.set(!show_labels()),
                TagIcon { size: 16 } span { class: "grow", "Thẻ phân loại" } ChevronDownIcon { size: 14 }
            }
            if show_labels() {
                div { class: "menu-sub",
                    if labels.is_empty() { div { class: "menu-note", "Chưa có thẻ nào" } }
                    for l in labels.iter().cloned() {
                        {
                            let on = label_ids.contains(&l.id);
                            let mut next = label_ids.clone();
                            if on { next.retain(|x| x != &l.id); } else { next.push(l.id.clone()); }
                            let p = pref_label.clone();
                            rsx! {
                                button { key: "{l.id}", r#type: "button", role: "menuitemcheckbox", aria_checked: "{on}", class: "account-menu-item",
                                    onclick: move |_| p(json!({ "label_ids": next.clone() })),
                                    span { class: "label-dot", style: "--label: {l.color};" }
                                    span { class: "grow", "{l.name}" }
                                    if on { CheckIcon { size: 14 } }
                                }
                            }
                        }
                    }
                    button { r#type: "button", class: "account-menu-item", onclick: move |_| on_manage_labels.call(()), PlusIcon { size: 14 } "Tạo / sửa thẻ" }
                }
            }
            if conv_type != "self" {
                button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| pref_mute(json!({ "muted": !muted })),
                    BellOffIcon { size: 16 } if muted { "Bật lại thông báo" } else { "Tắt thông báo" }
                }
            }
            if conv_type == "group" {
                button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| pref_comm(json!({ "in_communities": !in_communities })),
                    HashIcon { size: 16 } if in_communities { "Đưa về Tin nhắn" } else { "Chuyển sang Cộng đồng" }
                }
            }
            if conv_type == "channel" {
                button { r#type: "button", role: "menuitem", class: "account-menu-item", onclick: move |_| pref_out(json!({ "in_messages": false })),
                    HashIcon { size: 16 } "Bỏ khỏi Tin nhắn"
                }
            }
            if conv_type == "dm" {
                button { r#type: "button", role: "menuitem", class: "account-menu-item danger", onclick: move |_| on_block.call((id_block.clone(), !blocked)),
                    crate::components::icons::BlockIcon { size: 16 } if blocked { "Bỏ chặn" } else { "Chặn" }
                }
            }
            if conv_type == "self" {
                button { r#type: "button", role: "menuitem", class: "account-menu-item danger", onclick: move |_| on_delete_self.call(()),
                    TrashIcon { size: 16 } "Xóa cuộc trò chuyện này"
                }
            }
            if conv_type == "bot" {
                div { class: "menu-note", BotIcon { size: 14 } " Trò chuyện với bot không mã hóa đầu cuối" }
            }
        }
    }
}

/// Create, rename, recolour and delete chat labels.
#[component]
pub fn LabelManager(labels: Vec<ChatLabel>, on_close: EventHandler<()>, on_changed: EventHandler<Vec<ChatLabel>>) -> Element {
    let mut list = use_signal(|| labels.clone());
    let mut name = use_signal(String::new);
    let mut color = use_signal(|| LABEL_COLORS[0].to_string());
    let mut error = use_signal(|| Option::<String>::None);
    let mut busy = use_signal(|| false);
    use_hook(|| crate::platform::focus_selector("[aria-labelledby='labels-title']"));
    let mut add = move || {
        let n = name().trim().to_string();
        if n.is_empty() || busy() {
            return;
        }
        busy.set(true);
        let c = color();
        spawn(async move {
            match social::label_create(&n, &c).await {
                Ok(l) => {
                    list.with_mut(|v| v.push(l));
                    name.set(String::new());
                    error.set(None);
                    on_changed.call(list());
                }
                Err(e) => error.set(Some(match e.code.as_str() {
                    "label_limit_reached" => "Đã đạt số thẻ tối đa".into(),
                    "invalid_label" => "Tên hoặc màu thẻ không hợp lệ".into(),
                    _ => "Không tạo được thẻ".into(),
                })),
            }
            busy.set(false);
        });
    };
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content", role: "dialog", aria_modal: "true", aria_labelledby: "labels-title", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    h3 { id: "labels-title", class: "modal-title", "Thẻ phân loại" }
                }
                p { class: "modal-sub", "Gắn thẻ cho cuộc trò chuyện, nhóm và kênh để lọc nhanh. Thẻ chỉ mình bạn thấy và đồng bộ giữa các thiết bị." }
                form { class: "label-create",
                    onsubmit: move |e| { e.prevent_default(); add(); },
                    input { r#type: "text", maxlength: "24", placeholder: "Tên thẻ, ví dụ: Khách hàng", aria_label: "Tên thẻ", value: "{name}", oninput: move |e| name.set(e.value()) }
                    div { class: "color-swatches", role: "radiogroup", aria_label: "Màu thẻ",
                        for c in LABEL_COLORS {
                            button { key: "{c}", r#type: "button", role: "radio", aria_checked: "{color() == c}", aria_label: "Màu {c}",
                                class: if color() == c { "swatch active" } else { "swatch" }, style: "--swatch: {c};",
                                onclick: move |_| color.set(c.to_string()),
                            }
                        }
                    }
                    button { r#type: "submit", class: "btn-primary", disabled: name().trim().is_empty() || busy(), PlusIcon { size: 16 } "Thêm" }
                }
                if let Some(err) = error() { div { class: "status-badge error", "{err}" } }
                div { class: "label-list",
                    if list.read().is_empty() { div { class: "sidebar-empty", "Chưa có thẻ nào" } }
                    for l in list() {
                        LabelRow { key: "{l.id}", label: l.clone(),
                            on_saved: move |nl: ChatLabel| { list.with_mut(|v| if let Some(x) = v.iter_mut().find(|x| x.id == nl.id) { *x = nl.clone(); }); on_changed.call(list()); },
                            on_deleted: move |id: String| { list.with_mut(|v| v.retain(|x| x.id != id)); on_changed.call(list()); },
                        }
                    }
                }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Xong" }
                }
            }
        }
    }
}

#[component]
fn LabelRow(label: ChatLabel, on_saved: EventHandler<ChatLabel>, on_deleted: EventHandler<String>) -> Element {
    let mut name = use_signal(|| label.name.clone());
    let mut color = use_signal(|| label.color.clone());
    let id = label.id.clone();
    let id_del = label.id.clone();
    let save = move |c: String| {
        let n = name().trim().to_string();
        if n.is_empty() {
            return;
        }
        let id = id.clone();
        spawn(async move {
            if let Ok(l) = social::label_update(&id, &n, &c).await {
                on_saved.call(l);
            }
        });
    };
    let save_blur = save.clone();
    rsx! {
        div { class: "label-row",
            span { class: "label-dot big", style: "--label: {color};" }
            input { r#type: "text", maxlength: "24", aria_label: "Tên thẻ", value: "{name}", oninput: move |e| name.set(e.value()), onchange: move |_| save_blur(color()) }
            div { class: "color-swatches small",
                for c in LABEL_COLORS {
                    {
                        let s = save.clone();
                        rsx! {
                            button { key: "{c}", r#type: "button", aria_label: "Màu {c}", class: if color() == c { "swatch active" } else { "swatch" }, style: "--swatch: {c};",
                                onclick: move |_| { color.set(c.to_string()); s(c.to_string()); },
                            }
                        }
                    }
                }
            }
            button { r#type: "button", class: "icon-btn", title: "Xóa thẻ", aria_label: "Xóa thẻ {label.name}",
                onclick: move |_| {
                    let id = id_del.clone();
                    spawn(async move {
                        if social::label_delete(&id).await.is_ok() {
                            on_deleted.call(id);
                        }
                    });
                },
                TrashIcon { size: 16 }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_prefix_roundtrip() {
        let (label, text) = split_device_prefix("\u{2060}Linux 1\u{2060}hello");
        assert_eq!(label.as_deref(), Some("Linux 1"));
        assert_eq!(text, "hello");
        assert_eq!(split_device_prefix("plain").0, None);
        assert_eq!(strip_device_prefix("\u{2060}Windows 2\u{2060}poll:{}"), "poll:{}");
        let long = "\u{2060}9ab7036d-cc5a-4770-b402-99aa00bb11cc|Laptop làm việc của tôi\u{2060}note";
        assert_eq!(split_device_prefix(long).1, "note");
    }
}
