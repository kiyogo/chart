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
        icons::{
            BellOffIcon, CheckIcon, ChevronDownIcon, CloseIcon, DevicesIcon, HashIcon, ImageIcon,
            MoreIcon, PinIcon, PluginIcon, PlusIcon, SearchIcon, SendIcon, TagIcon, TrashIcon,
        },
        messages::preview_text,
        profile_card::{name_effect_class, nameplate_style},
    },
    i18n::t,
    models::{BotItem, ChannelItem, ChatLabel, ConvPref, FriendItem, GroupItem},
    services::{api, feed, library, profile_ext, social},
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
    #[props(default)] my_username: String,
    #[props(default)] my_avatar_url: Option<String>,
    #[props(default)] my_status_text: Option<String>,
    #[props(default)] my_status_emoji: Option<String>,
    #[props(default)] on_my_status_updated: Option<EventHandler<(Option<String>, Option<String>)>>,
    #[props(default)] on_note_reply: Option<EventHandler<(FriendItem, String)>>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| "all".to_string());
    let _ = (on_create_group, on_create_self);
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
            name: crate::services::profile_ext::nickname_of(&f.id)
                .or_else(|| f.display_name.clone().filter(|n| !n.trim().is_empty()))
                .unwrap_or_else(|| f.username.clone()),
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
    let _ = (&mut filter, &mut label_menu, ChevronDownIcon);
    rows.retain(|r| q.is_empty() || crate::components::composer::fold(&r.name).contains(&q));
    // Pinned first (most recently pinned on top), then "Chính mình", then the rest.
    rows.sort_by(|a, b| match (&a.pinned_at, &b.pinned_at) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.order.cmp(&b.order),
    });
    let pinned: Vec<Row> = rows.iter().filter(|r| r.pinned_at.is_some()).cloned().collect();
    let rest: Vec<Row> = rows.iter().filter(|r| r.pinned_at.is_none()).cloned().collect();

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
                        ConvOpen::SelfChat => rsx! { span { class: "self-chat-avatar", aria_hidden: "true", DevicesIcon { size: 24 } } },
                        ConvOpen::Channel(_) => rsx! { span { class: "self-chat-avatar channel", aria_hidden: "true", HashIcon { size: 22 } } },
                        ConvOpen::Dm(f) => rsx! {
                            crate::components::profile_card::DecoratedAvatar {
                                name: f.username.clone(), avatar_url: r.avatar.clone(), size: 52,
                                decoration: f.avatar_decoration.clone().filter(|d| d != "custom"),
                                motion: f.avatar_motion.clone(), in_list: true,
                            }
                        },
                        ConvOpen::Bot(_) => rsx! {
                            div { class: "chat-item-avatar",
                                Avatar { name: r.name.clone(), avatar_url: r.avatar.clone(), size: 52 }
                            }
                        },
                        _ => rsx! { Avatar { name: r.name.clone(), avatar_url: r.avatar.clone(), size: 52 } },
                    }
                    div { class: "chat-item-info",
                        div { class: "chat-item-header",
                            span { class: if r.unread > 0 { "chat-item-name unread {r.name_fx}" } else { "chat-item-name {r.name_fx}" }, "{r.name}" }
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
                        on_pref_label: move |v| { on_pref.call(v) },
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
        div { class: "sidebar-scrollable",
            if q.is_empty() && f == "all" {
                StoriesNotesBar {
                    strip_id: "conv-notes-strip".to_string(),
                    my_username: my_username.clone(),
                    my_avatar_url: my_avatar_url.clone(),
                    my_status_text: my_status_text.clone(),
                    my_status_emoji: my_status_emoji.clone(),
                    friends: friends.clone(),
                    on_my_status_updated: on_my_status_updated,
                    on_open_friend: {
                        let on_open = on_open;
                        move |fr: FriendItem| on_open.call(ConvOpen::Dm(fr))
                    },
                    on_note_reply: on_note_reply,
                }
            }
            if !pinned.is_empty() {
                div { class: "sidebar-section-header", span { "{t(\"chat.pinned\")}" } }
                for r in pinned { {render_row(r)} }
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

const QUICK_NOTE_EMOJIS: [&str; 8] = ["💭", "🔥", "☕", "🎧", "🚀", "😄", "🎉", "✨"];

/// Messenger-style horizontally scrollable Notes / Stories bar ("Tạo tin").
#[component]
pub fn StoriesNotesBar(
    strip_id: String,
    my_username: String,
    my_avatar_url: Option<String>,
    my_status_text: Option<String>,
    my_status_emoji: Option<String>,
    friends: Vec<FriendItem>,
    #[props(default)] on_my_status_updated: Option<EventHandler<(Option<String>, Option<String>)>>,
    on_open_friend: EventHandler<FriendItem>,
    #[props(default)] on_note_reply: Option<EventHandler<(FriendItem, String)>>,
) -> Element {
    let mut edit_my_note = use_signal(|| false);
    let mut note_emoji = use_signal(|| my_status_emoji.clone().unwrap_or_default());
    let mut note_text = use_signal(|| my_status_text.clone().unwrap_or_default());
    let mut note_expiry = use_signal(|| 1440i64);
    let mut note_saving = use_signal(|| false);
    let mut note_err = use_signal(|| Option::<String>::None);
    let mut viewing_friend = use_signal(|| Option::<FriendItem>::None);
    let mut reply_draft = use_signal(String::new);

    let mut local_status_text = use_signal(|| my_status_text.clone());
    let mut local_status_emoji = use_signal(|| my_status_emoji.clone());
    let mut active_tab = use_signal(|| "thought");
    let mut story_draft = use_signal(String::new);
    let mut story_media_url = use_signal(|| Option::<String>::None);
    let mut story_uploading = use_signal(|| false);
    let mut story_posting = use_signal(|| false);
    let mut story_success = use_signal(|| false);
    let mut active_stories = use_signal(Vec::<feed::FeedPostItem>::new);
    let mut viewing_story = use_signal(|| Option::<feed::FeedPostItem>::None);

    let reload_stories = use_callback(move |_: ()| {
        spawn(async move {
            if let Ok(list) = feed::list_feed("all").await {
                active_stories.set(list);
            }
        });
    });

    use_effect(move || {
        reload_stories.call(());
    });

    use_effect(use_reactive!(|my_status_text, my_status_emoji| {
        local_status_text.set(my_status_text);
        local_status_emoji.set(my_status_emoji);
    }));

    let drag_js_id = strip_id.clone();
    use_effect(move || {
        let js = format!(
            r#"
            (function() {{
                const el = document.getElementById({id:?});
                if (!el || el.__notesDragReady) return;
                el.__notesDragReady = true;
                let down = false, startX = 0, startScroll = 0, moved = false;
                el.addEventListener('dragstart', (e) => e.preventDefault());
                el.addEventListener('pointerdown', (e) => {{
                    if (e.button !== 0) return;
                    down = true; moved = false;
                    startX = e.clientX; startScroll = el.scrollLeft;
                }});
                window.addEventListener('pointermove', (e) => {{
                    if (!down) return;
                    const dx = e.clientX - startX;
                    if (Math.abs(dx) > 5) {{
                        if (!moved) {{
                            moved = true;
                            el.classList.add('dragging');
                        }}
                        el.scrollLeft = startScroll - dx;
                    }}
                }});
                window.addEventListener('pointerup', () => {{
                    if (!down) return;
                    down = false;
                    el.classList.remove('dragging');
                    if (moved) {{
                        el.dataset.justDragged = '1';
                        setTimeout(() => {{ delete el.dataset.justDragged; }}, 60);
                    }}
                }});
                el.addEventListener('wheel', (e) => {{
                    if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {{
                        el.scrollLeft += e.deltaY;
                        e.preventDefault();
                    }}
                }}, {{ passive: false }});
                el.addEventListener('click', (e) => {{
                    if (el.dataset.justDragged === '1') {{
                        e.preventDefault();
                        e.stopPropagation();
                    }}
                }}, true);
            }})();
            "#,
            id = drag_js_id
        );
        let _ = dioxus::document::eval(&js);
    });

    let mut ordered_friends = friends.clone();
    ordered_friends.sort_by_key(|f| {
        let has_note = f.status_text.as_deref().is_some_and(|s| !s.trim().is_empty())
            || f.status_emoji.as_deref().is_some_and(|s| !s.trim().is_empty());
        let is_online = f.presence.as_deref() == Some("online");
        (!has_note, !is_online, f.username.to_lowercase())
    });

    let cur_em = local_status_emoji().as_deref().map(str::trim).unwrap_or("").to_string();
    let cur_tx = local_status_text().as_deref().map(str::trim).unwrap_or("").to_string();
    let my_has_note = !cur_em.is_empty() || !cur_tx.is_empty();
    let my_bubble_label = if my_has_note {
        format!("{cur_em} {cur_tx}").trim().to_string()
    } else {
        "Chia sẻ suy nghĩ".to_string()
    };

    let stories_snapshot = active_stories();
    let my_latest_story = stories_snapshot
        .iter()
        .find(|p| p.author_username == my_username)
        .cloned();
    let my_has_story = my_latest_story.is_some();

    let init_em = cur_em.clone();
    let init_tx = cur_tx.clone();
    let init_em2 = cur_em.clone();
    let init_tx2 = cur_tx.clone();
    let init_em3 = cur_em.clone();
    let init_tx3 = cur_tx.clone();
    let my_story_open = my_latest_story.clone();

    rsx! {
        div { class: "notes-strip-wrap",
            div { id: "{strip_id}", class: "notes-strip", role: "list", aria_label: "Tin và ghi chú",
                // Current user tile ("Tạo tin")
                div {
                    class: "note-item my-note-tile",
                    role: "listitem",
                    button {
                        r#type: "button",
                        class: "note-bubble-btn",
                        aria_label: "Chia sẻ suy nghĩ",
                        title: "Chia sẻ suy nghĩ",
                        onclick: move |e| {
                            e.stop_propagation();
                            note_emoji.set(init_em.clone());
                            note_text.set(init_tx.clone());
                            note_err.set(None);
                            active_tab.set("thought");
                            story_success.set(false);
                            edit_my_note.set(true);
                        },
                        span {
                            class: if my_has_note { "note-bubble mine" } else { "note-bubble placeholder" },
                            if my_has_note {
                                "{my_bubble_label}"
                            } else {
                                "Chia sẻ"
                                br {}
                                "suy nghĩ"
                            }
                        }
                    }
                    div { class: "note-avatar-tile",
                        div { class: if my_has_story { "note-avatar-wrap has-story-ring" } else { "note-avatar-wrap" },
                            button {
                                r#type: "button",
                                class: "note-avatar-circle-btn",
                                aria_label: if my_has_story { "Xem tin của bạn" } else { "Tạo tin mới" },
                                title: if my_has_story { "Xem tin của bạn" } else { "Tạo tin" },
                                onclick: move |e| {
                                    e.stop_propagation();
                                    if let Some(st) = my_story_open.clone() {
                                        viewing_story.set(Some(st));
                                    } else {
                                        note_emoji.set(init_em3.clone());
                                        note_text.set(init_tx3.clone());
                                        note_err.set(None);
                                        active_tab.set("story");
                                        story_success.set(false);
                                        edit_my_note.set(true);
                                    }
                                },
                                Avatar {
                                    name: if my_username.is_empty() { "Tôi".to_string() } else { my_username.clone() },
                                    avatar_url: my_avatar_url.clone(),
                                    size: 52,
                                }
                            }
                            button {
                                r#type: "button",
                                class: "note-plus-badge",
                                aria_label: "Tạo tin mới",
                                title: "Tạo tin",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    note_emoji.set(init_em2.clone());
                                    note_text.set(init_tx2.clone());
                                    note_err.set(None);
                                    active_tab.set("story");
                                    story_success.set(false);
                                    edit_my_note.set(true);
                                },
                                PlusIcon { size: 12 }
                            }
                        }
                        span { class: "note-name", "Tạo tin" }
                    }
                }

                // Friends tiles
                for fr in ordered_friends {
                    {
                        let shown = crate::services::profile_ext::nickname_of(&fr.id)
                            .or_else(|| fr.display_name.clone().filter(|n| !n.trim().is_empty()))
                            .unwrap_or_else(|| fr.username.clone());
                        let fem = fr.status_emoji.as_deref().map(str::trim).unwrap_or("");
                        let ftx = fr.status_text.as_deref().map(str::trim).unwrap_or("");
                        let has_note = !fem.is_empty() || !ftx.is_empty();
                        let friend_story = stories_snapshot
                            .iter()
                            .find(|p| p.author_id == fr.id || p.author_username == fr.username)
                            .cloned();
                        let has_story = friend_story.is_some();
                        let bubble_str = format!("{fem} {ftx}").trim().to_string();
                        let online = fr.presence.as_deref() == Some("online");
                        let fr_click = fr.clone();
                        let st_click = friend_story.clone();
                        rsx! {
                            button {
                                key: "note-{fr.id}",
                                r#type: "button",
                                class: "note-item",
                                role: "listitem",
                                onclick: move |_| {
                                    if has_note {
                                        reply_draft.set(String::new());
                                        viewing_friend.set(Some(fr_click.clone()));
                                    } else if let Some(st) = st_click.clone() {
                                        reply_draft.set(String::new());
                                        viewing_story.set(Some(st));
                                    } else {
                                        on_open_friend.call(fr_click.clone());
                                    }
                                },
                                if has_note {
                                    span { class: "note-bubble", "{bubble_str}" }
                                }
                                span { class: if has_story { "note-avatar-wrap has-story-ring" } else { "note-avatar-wrap" },
                                    crate::components::profile_card::DecoratedAvatar {
                                        name: fr.username.clone(),
                                        avatar_url: fr.avatar_url.clone(),
                                        size: 52,
                                        decoration: fr.avatar_decoration.clone().filter(|d| d != "custom"),
                                        motion: fr.avatar_motion.clone(),
                                        in_list: true,
                                    }
                                    if online {
                                        span { class: "note-online-dot", title: "Đang hoạt động" }
                                    }
                                }
                                span { class: "note-name", "{shown}" }
                            }
                        }
                    }
                }
            }
        }

        if edit_my_note() {
            div { class: "modal-overlay", onclick: move |_| edit_my_note.set(false),
                div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Ghi chú và tin của bạn", onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h3 { class: "modal-title",
                            if active_tab() == "thought" { "Chia sẻ suy nghĩ" } else { "Tạo tin ảnh" }
                        }
                        button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| edit_my_note.set(false), CloseIcon { size: 18 } }
                    }
                    if active_tab() == "thought" {
                        div { class: "note-emoji-row",
                            for em in QUICK_NOTE_EMOJIS {
                                {
                                    let active = note_emoji().trim() == em;
                                    rsx! {
                                        button {
                                            key: "{em}",
                                            r#type: "button",
                                            class: if active { "note-emoji-chip active" } else { "note-emoji-chip" },
                                            onclick: move |_| note_emoji.set(em.to_string()),
                                            "{em}"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "note-input-row",
                            input {
                                class: "note-emoji-input",
                                r#type: "text",
                                maxlength: "8",
                                placeholder: "😊",
                                aria_label: "Emoji",
                                value: "{note_emoji}",
                                oninput: move |e| note_emoji.set(e.value()),
                            }
                            input {
                                r#type: "text",
                                maxlength: "120",
                                placeholder: "Chia sẻ suy nghĩ với bạn bè",
                                aria_label: "Nội dung ghi chú",
                                value: "{note_text}",
                                oninput: move |e| note_text.set(e.value()),
                            }
                        }
                        div { class: "inline-form",
                            span { class: "member-sub", "Hiển thị trong" }
                            select {
                                value: "{note_expiry}",
                                onchange: move |e| note_expiry.set(e.value().parse().unwrap_or(1440)),
                                option { value: "1440", "24 giờ" }
                                option { value: "240", "4 giờ" }
                                option { value: "60", "1 giờ" }
                                option { value: "0", "Không tự xóa" }
                            }
                        }
                        if let Some(e) = note_err() {
                            div { class: "status-badge error", "{e}" }
                        }
                        div { class: "modal-actions",
                            if my_has_note {
                                button {
                                    r#type: "button",
                                    class: "btn-ghost",
                                    disabled: note_saving(),
                                    onclick: move |_| {
                                        note_saving.set(true);
                                        note_err.set(None);
                                        spawn(async move {
                                            let r1 = api::update_my_profile(None, None, Some(""), Some(""), None, None).await;
                                            let r2 = profile_ext::update_me(json!({ "status_expires_in_minutes": 0 })).await;
                                            note_saving.set(false);
                                            if r1.is_ok() && r2.is_ok() {
                                                local_status_text.set(None);
                                                local_status_emoji.set(None);
                                                if let Some(h) = on_my_status_updated { h.call((None, None)); }
                                                edit_my_note.set(false);
                                            } else {
                                                note_err.set(Some("Không xóa được ghi chú".into()));
                                            }
                                        });
                                    },
                                    "Xóa tin"
                                }
                            }
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| edit_my_note.set(false), "Hủy" }
                            button {
                                r#type: "button",
                                class: "btn-primary",
                                disabled: note_saving() || (note_text().trim().is_empty() && note_emoji().trim().is_empty()),
                                onclick: move |_| {
                                    let t = note_text().trim().to_string();
                                    let em = note_emoji().trim().to_string();
                                    let exp = note_expiry();
                                    note_saving.set(true);
                                    note_err.set(None);
                                    spawn(async move {
                                        let r1 = api::update_my_profile(None, None, Some(&t), Some(&em), None, None).await;
                                        let r2 = profile_ext::update_me(json!({ "status_expires_in_minutes": exp })).await;
                                        note_saving.set(false);
                                        if r1.is_ok() && r2.is_ok() {
                                            let nt = (!t.is_empty()).then_some(t);
                                            let ne = (!em.is_empty()).then_some(em);
                                            local_status_text.set(nt.clone());
                                            local_status_emoji.set(ne.clone());
                                            if let Some(h) = on_my_status_updated {
                                                h.call((nt, ne));
                                            }
                                            edit_my_note.set(false);
                                        } else {
                                            note_err.set(Some("Không lưu được ghi chú".into()));
                                        }
                                    });
                                },
                                "Chia sẻ"
                            }
                        }
                    } else {
                        // Tab "story"
                        div { style: "display: flex; flex-direction: column; gap: var(--space-3);",
                            if let Some(murl) = story_media_url() {
                                div { style: "position: relative; max-height: 180px; overflow: hidden; border-radius: var(--radius-lg);",
                                    img { src: "{crate::platform::resolve_asset_url(&murl)}", alt: "Ảnh tin", style: "width: 100%; object-fit: cover;" }
                                    button {
                                        r#type: "button",
                                        class: "icon-btn",
                                        style: "position: absolute; top: 6px; right: 6px; background: rgba(0,0,0,0.6); color: #fff;",
                                        aria_label: "Xóa ảnh",
                                        onclick: move |_| story_media_url.set(None),
                                        CloseIcon { size: 14 }
                                    }
                                }
                            } else {
                                div { class: "inline-form",
                                    input {
                                        id: "story-media-upload",
                                        r#type: "file",
                                        accept: "image/png,image/jpeg,image/webp,image/gif,image/avif",
                                        style: "display: none;",
                                        onchange: move |_| {
                                            story_uploading.set(true);
                                            spawn(async move {
                                                if let Ok((bytes, mime)) = crate::platform::read_selected_file_raw("story-media-upload").await {
                                                    match library::upload_media(bytes, &mime).await {
                                                        Ok(url) => { story_media_url.set(Some(url)); note_err.set(None); }
                                                        Err(_) => note_err.set(Some("Không tải ảnh lên được".into())),
                                                    }
                                                }
                                                story_uploading.set(false);
                                            });
                                        },
                                    }
                                    label {
                                        r#for: "story-media-upload",
                                        class: "btn-secondary",
                                        style: "display: inline-flex; align-items: center; gap: 6px; cursor: pointer; width: 100%; justify-content: center;",
                                        ImageIcon { size: 18 }
                                        if story_uploading() { "Đang tải ảnh lên" } else { "Chọn ảnh cho tin mới" }
                                    }
                                }
                            }
                            input {
                                r#type: "text",
                                maxlength: "200",
                                placeholder: "Thêm chú thích cho tin",
                                aria_label: "Chú thích tin",
                                value: "{story_draft}",
                                oninput: move |e| story_draft.set(e.value()),
                            }
                            if story_success() {
                                div { class: "status-badge success", "Tin của bạn đã được đăng thành công" }
                            }
                            if let Some(e) = note_err() {
                                div { class: "status-badge error", "{e}" }
                            }
                            div { class: "modal-actions",
                                if let Some(my_st) = my_latest_story.clone() {
                                    button {
                                        r#type: "button",
                                        class: "btn-ghost",
                                        onclick: move |_| {
                                            edit_my_note.set(false);
                                            reply_draft.set(String::new());
                                            viewing_story.set(Some(my_st.clone()));
                                        },
                                        "Xem tin của bạn"
                                    }
                                }
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| edit_my_note.set(false), "Hủy" }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    disabled: story_posting() || story_uploading() || (story_draft().trim().is_empty() && story_media_url().is_none()),
                                    onclick: move |_| {
                                        let txt = story_draft().trim().to_string();
                                        let mu = story_media_url();
                                        story_posting.set(true);
                                        note_err.set(None);
                                        spawn(async move {
                                            match feed::create_post(&txt, mu.as_deref(), "friends", None, None).await {
                                                Ok(_) => {
                                                    story_draft.set(String::new());
                                                    story_media_url.set(None);
                                                    story_success.set(true);
                                                    reload_stories.call(());
                                                    edit_my_note.set(false);
                                                }
                                                Err(e) => note_err.set(Some(format!("Đăng tin thất bại: {}", e.code))),
                                            }
                                            story_posting.set(false);
                                        });
                                    },
                                    if story_posting() { "Đang đăng" } else { "Đăng tin" }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(vf) = viewing_friend() {
            {
                let shown = crate::services::profile_ext::nickname_of(&vf.id)
                    .or_else(|| vf.display_name.clone().filter(|n| !n.trim().is_empty()))
                    .unwrap_or_else(|| vf.username.clone());
                let full_note = format!(
                    "{} {}",
                    vf.status_emoji.as_deref().unwrap_or("").trim(),
                    vf.status_text.as_deref().unwrap_or("").trim()
                )
                .trim()
                .to_string();
                let vf_open = vf.clone();
                let vf_reply = vf.clone();
                let friend_story_opt = active_stories()
                    .into_iter()
                    .find(|p| p.author_id == vf.id || p.author_username == vf.username);
                rsx! {
                    div { class: "modal-overlay", onclick: move |_| viewing_friend.set(None),
                        div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Ghi chú của {shown}", onclick: move |e| e.stop_propagation(),
                            div { class: "modal-header",
                                div { style: "display: flex; align-items: center; gap: 10px;",
                                    Avatar { name: vf.username.clone(), avatar_url: vf.avatar_url.clone(), size: 36 }
                                    div {
                                        div { class: "modal-title", style: "font-size: 1rem;", "{shown}" }
                                        div { class: "member-sub", "@{vf.username}" }
                                    }
                                }
                                button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| viewing_friend.set(None), CloseIcon { size: 18 } }
                            }
                            div { class: "note-view-card", "{full_note}" }
                            div { class: "note-input-row",
                                input {
                                    r#type: "text",
                                    placeholder: "Trả lời ghi chú của {shown}",
                                    value: "{reply_draft}",
                                    oninput: move |e| reply_draft.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    disabled: reply_draft().trim().is_empty(),
                                    onclick: move |_| {
                                        let txt = format!("Ghi chú \"{full_note}\": {}", reply_draft().trim());
                                        viewing_friend.set(None);
                                        if let Some(h) = on_note_reply {
                                            h.call((vf_reply.clone(), txt));
                                        } else {
                                            on_open_friend.call(vf_reply.clone());
                                        }
                                    },
                                    SendIcon { size: 16 }
                                }
                            }
                            div { class: "modal-actions",
                                if let Some(fst) = friend_story_opt {
                                    button {
                                        r#type: "button",
                                        class: "btn-ghost",
                                        onclick: move |_| {
                                            viewing_friend.set(None);
                                            reply_draft.set(String::new());
                                            viewing_story.set(Some(fst.clone()));
                                        },
                                        "Xem tin ảnh"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        viewing_friend.set(None);
                                        on_open_friend.call(vf_open.clone());
                                    },
                                    "Mở cuộc trò chuyện"
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(st) = viewing_story() {
            {
                let st_author_name = st.author_display_name.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| st.author_username.clone());
                let is_my_story = st.author_username == my_username;
                let st_id_del = st.id.clone();
                let st_friend = friends.iter().find(|f| f.id == st.author_id || f.username == st.author_username).cloned();
                rsx! {
                    div { class: "modal-overlay", onclick: move |_| viewing_story.set(None),
                        div { class: "modal-dialog story-viewer-modal", role: "dialog", aria_modal: "true", aria_label: "Tin của {st_author_name}", onclick: move |e| e.stop_propagation(),
                            div { class: "modal-header",
                                div { style: "display: flex; align-items: center; gap: 10px;",
                                    Avatar { name: st.author_username.clone(), avatar_url: st.author_avatar_url.clone(), size: 36 }
                                    div {
                                        div { class: "modal-title", style: "font-size: 1rem;", "{st_author_name}" }
                                        div { class: "member-sub", "@{st.author_username}" }
                                    }
                                }
                                button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| viewing_story.set(None), CloseIcon { size: 18 } }
                            }
                            if let Some(murl) = st.media_url.as_ref().filter(|s| !s.trim().is_empty()) {
                                div { class: "story-viewer-media",
                                    img { src: "{crate::platform::resolve_asset_url(murl)}", alt: "Ảnh tin" }
                                }
                            }
                            if !st.content.trim().is_empty() {
                                div { class: "story-viewer-caption", "{st.content}" }
                            }
                            if !is_my_story {
                                if let Some(fr_target) = st_friend {
                                    div { class: "note-input-row",
                                        input {
                                            r#type: "text",
                                            placeholder: "Trả lời tin của {st_author_name}",
                                            value: "{reply_draft}",
                                            oninput: move |e| reply_draft.set(e.value()),
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-primary",
                                            disabled: reply_draft().trim().is_empty(),
                                            onclick: move |_| {
                                                let txt = format!("Trả lời tin: {}", reply_draft().trim());
                                                viewing_story.set(None);
                                                if let Some(h) = on_note_reply {
                                                    h.call((fr_target.clone(), txt));
                                                } else {
                                                    on_open_friend.call(fr_target.clone());
                                                }
                                            },
                                            SendIcon { size: 16 }
                                        }
                                    }
                                }
                            }
                            div { class: "modal-actions",
                                if is_my_story {
                                    button {
                                        r#type: "button",
                                        class: "btn-ghost danger",
                                        onclick: move |_| {
                                            let id_del = st_id_del.clone();
                                            spawn(async move {
                                                if feed::delete_post(&id_del).await.is_ok() {
                                                    active_stories.with_mut(|list| list.retain(|x| x.id != id_del));
                                                    viewing_story.set(None);
                                                }
                                            });
                                        },
                                        "Xóa tin này"
                                    }
                                }
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| viewing_story.set(None), "Đóng" }
                            }
                        }
                    }
                }
            }
        }
    }
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
    on_pref_label: EventHandler<(String, String, serde_json::Value)>,
    on_manage_labels: EventHandler<()>,
    on_block: EventHandler<(String, bool)>,
    on_mark_read: EventHandler<String>,
    on_delete_self: EventHandler<()>,
) -> Element {
    let mut show_labels = use_signal(|| false);
    let mut current_label_ids = use_signal(|| label_ids.clone());
    let ct = conv_type.clone();
    let id = conv_id.clone();
    let pref = move |fields: serde_json::Value| on_pref.call((ct.clone(), id.clone(), fields));
    let pref_pin = pref.clone();
    let pref_mute = pref.clone();
    let pref_comm = pref.clone();
    let pref_out = pref.clone();
    let ct_label = conv_type.clone();
    let id_label = conv_id.clone();
    let pref_label = move |fields: serde_json::Value| on_pref_label.call((ct_label.clone(), id_label.clone(), fields));
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
                            let on = current_label_ids.read().contains(&l.id);
                            let mut next = current_label_ids.read().clone();
                            if on { next.retain(|x| x != &l.id); } else { next.push(l.id.clone()); }
                            let next_val = next.clone();
                            let p = pref_label.clone();
                            rsx! {
                                button { key: "{l.id}", r#type: "button", role: "menuitemcheckbox", aria_checked: "{on}", class: "account-menu-item",
                                    onclick: move |_| {
                                        current_label_ids.set(next_val.clone());
                                        p(json!({ "label_ids": next_val.clone() }));
                                    },
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
                div { class: "menu-note", PluginIcon { size: 14 } " Trò chuyện với plugin" }
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
