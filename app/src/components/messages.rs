//! Shared message list + composer used by DMs, group DMs and community channels.

use chrono::{DateTime, Datelike, Local, NaiveDate};
use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        emoji_picker::EmojiPicker,
        icons::{ClockIcon, CopyIcon, PinIcon, ReplyIcon, ShieldIcon, SmilePlusIcon},
        rich::{Rich, RichContent, VoteTally, parse_rich, rich_preview},
    },
    platform,
};

/// Render-ready message, independent of transport (E2EE DM, group, channel).
#[derive(Clone, PartialEq, Debug)]
pub struct ViewMessage {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_avatar: Option<String>,
    pub text: String,
    pub created_at: String,
    pub is_self: bool,
    pub is_error: bool,
    pub is_pending: bool,
    pub reply_text: Option<String>,
    /// (emoji, count, reacted_by_me)
    pub reactions: Vec<(String, usize, bool)>,
    pub is_pinned: bool,
    /// Posted by a bot (groups) — shows a BOT tag.
    pub is_bot: bool,
    /// Message from a group leader / deputy while "highlight admins" is on.
    pub highlighted: bool,
}

const GROUP_WINDOW_SECS: i64 = 5 * 60;

/// Default initial DOM render window for large message histories (10,000+ messages).
/// Older slices are mounted progressively on scroll-up or click with zero layout shift.
pub const MESSAGE_DOM_WINDOW_STEP: usize = 120;

/// Computes the slice range `(start_idx, end_idx)` of `total_len` messages to mount in the DOM
/// given the current `render_limit` and an optional `highlight_idx` that must remain visible.
pub fn compute_message_window_bounds(
    total_len: usize,
    render_limit: usize,
    highlight_idx: Option<usize>,
) -> (usize, usize) {
    if total_len == 0 {
        return (0, 0);
    }
    let effective_limit = render_limit.max(MESSAGE_DOM_WINDOW_STEP);
    let mut start = total_len.saturating_sub(effective_limit);
    if let Some(h_idx) = highlight_idx {
        if h_idx < start {
            start = h_idx.saturating_sub(20);
        }
    }
    (start, total_len)
}

/// Reactions offered when hovering the single react button.
const QUICK_REACTIONS: [&str; 8] = ["👍", "❤️", "😂", "😮", "😢", "🔥", "🎉", "🙏"];

fn parse_local(iso: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(iso).ok().map(|d| d.with_timezone(&Local))
}

const ENGLISH_MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

fn day_label(date: NaiveDate) -> String {
    let today = Local::now().date_naive();
    let month_name = ENGLISH_MONTHS[(date.month().saturating_sub(1) as usize).min(11)];
    if date.year() == today.year() {
        format!("{month_name} {}", date.day())
    } else {
        format!("{month_name} {}, {}", date.day(), date.year())
    }
}

/// A message body that is only an uploaded image path is rendered as a sticker/media.
pub fn sticker_url_of(text: &str) -> Option<String> {
    let t = text.trim();
    if t.contains(char::is_whitespace) {
        return None;
    }
    // Built-in sticker set shipped with the client.
    if t.starts_with("/stickers/") && t.ends_with(".svg") && !t.contains("..") {
        return Some(t.to_string());
    }
    let path = if let Some(idx) = t.find("/uploads/") {
        let prefix = &t[..idx];
        if !(prefix.is_empty() || prefix.starts_with("http://") || prefix.starts_with("https://")) {
            return None;
        }
        &t[idx..]
    } else {
        return None;
    };
    let lower = path.to_lowercase();
    let is_image = [".avif", ".webp", ".png", ".gif", ".jpg", ".jpeg"].iter().any(|ext| lower.ends_with(ext));
    if is_image { Some(t.to_string()) } else { None }
}

/// A search word for "Find similar": the OpenMoji name if known, else the file name.
pub fn sticker_search_term(url: &str) -> String {
    let file = url.rsplit('/').next().unwrap_or_default();
    let stem = file.split('.').next().unwrap_or_default();
    if url.starts_with("/stickers/openmoji/") {
        // Hexcodes are not searchable; the picker resolves names from its catalogue.
        if let Some(c) = crate::services::openmoji::cached() {
            if let Some(i) = c.items.iter().find(|i| i.0 == stem) {
                return i.1.split_whitespace().last().unwrap_or(&i.1).to_lowercase();
            }
        }
        return String::new();
    }
    stem.replace(['-', '_'], " ")
}

/// `gif:<url>` messages are rendered as large animated media.
pub fn gif_url_of(text: &str) -> Option<String> {
    let t = text.trim().strip_prefix("gif:")?;
    let ok = (t.starts_with("/uploads/media/") || t.starts_with("/stickers/")) && !t.contains("..") && !t.contains(char::is_whitespace);
    if ok { Some(t.to_string()) } else { None }
}

/// Short preview text for lists and notifications.
pub fn preview_text(text: &str) -> String {
    if let Some(r) = parse_rich(text) {
        return rich_preview(&r);
    }
    if gif_url_of(text).is_some() {
        return "Sent a GIF".to_string();
    }
    if sticker_url_of(text).is_some() {
        return "Sent a sticker".to_string();
    }
    segments(text)
        .into_iter()
        .map(|s| match s {
            Segment::Text(t) | Segment::Link(t) => t,
            Segment::Emoji(name, _) => format!(":{name}:"),
        })
        .collect()
}

enum Segment {
    Text(String),
    Link(String),
    /// Custom emoji token `<:name:/uploads/media/...>`
    Emoji(String, String),
}

/// Split custom emoji tokens out of a text run.
fn split_emoji(text: &str, out: &mut Vec<Segment>) {
    let mut rest = text;
    while let Some(start) = rest.find("<:") {
        let after = &rest[start + 2..];
        let parsed = after.find(':').and_then(|colon| {
            let name = &after[..colon];
            let tail = &after[colon + 1..];
            let end = tail.find('>')?;
            let url = &tail[..end];
            let valid_name = (2..=32).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let valid_url = url.starts_with("/uploads/media/") && !url.contains("..") && !url.contains(char::is_whitespace);
            (valid_name && valid_url).then(|| (name.to_string(), url.to_string(), 2 + colon + 1 + end + 1))
        });
        match parsed {
            Some((name, url, len)) => {
                if start > 0 {
                    out.push(Segment::Text(rest[..start].to_string()));
                }
                out.push(Segment::Emoji(name, url));
                rest = &rest[start + len..];
            }
            None => {
                out.push(Segment::Text(rest[..start + 2].to_string()));
                rest = &rest[start + 2..];
            }
        }
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest.to_string()));
    }
}

/// Split text into plain and http(s) link segments (text is escaped by Dioxus).
fn segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let next = ["https://", "http://"].iter().filter_map(|p| rest.find(p)).min();
        match next {
            Some(pos) => {
                buf.push_str(&rest[..pos]);
                let tail = &rest[pos..];
                let end = tail.find(|c: char| c.is_whitespace()).unwrap_or(tail.len());
                let mut link = &tail[..end];
                while link.ends_with(['.', ',', ')', '!', '?', ';', ':']) && link.len() > 8 {
                    link = &link[..link.len() - 1];
                }
                if !buf.is_empty() {
                    split_emoji(&std::mem::take(&mut buf), &mut out);
                }
                out.push(Segment::Link(link.to_string()));
                rest = &tail[link.len()..];
            }
            None => {
                buf.push_str(rest);
                rest = "";
            }
        }
    }
    if !buf.is_empty() {
        split_emoji(&buf, &mut out);
    }
    out
}

/// Text with @mentions highlighted.
#[component]
fn MentionText(text: String) -> Element {
    let mut parts: Vec<(bool, String)> = Vec::new();
    let mut buf = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '@' && buf.chars().last().map(|p| p.is_whitespace()).unwrap_or(true) && chars.peek().is_some_and(|n| n.is_alphanumeric() || *n == '_') {
            if !buf.is_empty() {
                parts.push((false, std::mem::take(&mut buf)));
            }
            let mut name = String::from("@");
            while let Some(&n) = chars.peek() {
                if n.is_alphanumeric() || n == '_' || n == '.' {
                    name.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            parts.push((true, name));
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        parts.push((false, buf));
    }
    rsx! {
        for (i, (is_mention, t)) in parts.into_iter().enumerate() {
            if is_mention { span { key: "{i}", class: "mention", "{t}" } } else { span { key: "{i}", "{t}" } }
        }
    }
}

#[component]
pub fn MessageList(
    messages: Vec<ViewMessage>,
    #[props(default)] show_sender_names: bool,
    #[props(default)] e2ee: bool,
    #[props(default)] intro_title: String,
    #[props(default)] intro_sub: String,
    #[props(default)] on_open_profile: Option<EventHandler<String>>,
    /// Click on an avatar / name with viewport coordinates (opens the profile popover).
    #[props(default)] on_profile_click: Option<EventHandler<(String, f64, f64)>>,
    #[props(default)] on_react: Option<EventHandler<(String, String)>>,
    #[props(default)] on_reply: Option<EventHandler<ViewMessage>>,
    #[props(default)] highlight_id: Option<String>,
    /// Current user id (poll votes) and username (mention highlight).
    #[props(default)] my_id: String,
    #[props(default)] my_username: String,
    /// Message after which "Đã xem" is shown.
    #[props(default)] seen_id: Option<String>,
    /// Double-click the empty area beside a message to reply.
    #[props(default)] double_click_reply: bool,
    #[props(default)] on_pin: Option<EventHandler<(String, bool)>>,
    /// Send a raw rich payload (e.g. a poll vote) into this conversation.
    #[props(default)] on_send_raw: Option<EventHandler<String>>,
    #[props(default)] on_open_invite: Option<EventHandler<(String, String)>>,
    /// Keep a copy of a message in "Chính mình" (end-to-end encrypted).
    #[props(default)] on_save_self: Option<EventHandler<String>>,
    /// Create an enterprise task directly from a channel message `(msg_id, preview)`.
    #[props(default)] on_create_task: Option<EventHandler<(String, String)>>,
    /// Older history pagination state (Spec 046: load newest first, scroll up for older).
    #[props(default)] has_more: bool,
    #[props(default)] loading_more: bool,
    #[props(default)] loading_initial: bool,
    #[props(default)] on_load_more: Option<EventHandler<()>>,
) -> Element {
    let mut full_picker_for = use_signal(|| Option::<String>::None);
    let mut sticker_menu_for = use_signal(|| Option::<String>::None);
    let mut plugin_actions = use_signal(Vec::<crate::services::plugin_api::DynamicMessageAction>::new);
    let mut loaded_installs_len = use_signal(|| usize::MAX);
    let installs_len = crate::components::plugin_hub::PLUGIN_INSTALLATIONS().len();
    if *loaded_installs_len.peek() != installs_len {
        loaded_installs_len.set(installs_len);
        spawn(async move {
            let acts = crate::services::plugin_api::host_get_message_actions(None).await;
            if *plugin_actions.peek() != acts {
                plugin_actions.set(acts);
            }
        });
    }
    let mut scroll_checking = use_signal(|| false);
    let mut dom_render_limit = use_signal(|| MESSAGE_DOM_WINDOW_STEP);
    let prefs_ctx = try_consume_context::<crate::prefs::PrefsCtx>();
    let fav_stickers = prefs_ctx.map(|c| c.prefs.read().fav_stickers.clone()).unwrap_or_default();
    // Poll votes are messages too: tally them and hide them from the list.
    let mut tally: VoteTally = VoteTally::new();
    for m in &messages {
        if let Some(Rich::Vote { poll_id, option }) = parse_rich(&m.text) {
            tally.entry(poll_id).or_default().insert(m.sender_id.clone(), option);
        }
    }
    let all_messages: Vec<ViewMessage> = messages.into_iter().filter(|m| !matches!(parse_rich(&m.text), Some(Rich::Vote { .. }))).collect();
    let highlight_idx = highlight_id
        .as_deref()
        .and_then(|hid| all_messages.iter().position(|m| m.id == hid));
    let (win_start, win_end) = compute_message_window_bounds(
        all_messages.len(),
        *dom_render_limit.read(),
        highlight_idx,
    );
    let hidden_older_in_memory = win_start;
    let messages: Vec<ViewMessage> = all_messages[win_start..win_end].to_vec();
    let mention_token = if my_username.is_empty() { String::new() } else { format!("@{}", my_username.to_lowercase()) };
    let count = messages.len();
    let parsed: Vec<Option<DateTime<Local>>> = messages.iter().map(|m| parse_local(&m.created_at)).collect();

    rsx! {
        div {
            id: "chat-messages-scroll",
            class: "chat-messages",
            role: "log",
            aria_live: "polite",
            onscroll: move |_| {
                if !loading_initial && (hidden_older_in_memory > 0 || has_more) && !loading_more && !*scroll_checking.peek() {
                    scroll_checking.set(true);
                    spawn(async move {
                        let mut ev = dioxus::document::eval(
                            "const el = document.getElementById('chat-messages-scroll'); const pinning = window.__chatwStickToBottom && window.__chatwStickToBottom.isPinning(); dioxus.send(!pinning && el ? el.scrollTop <= 180 : false);"
                        );
                        let near_top = ev.recv::<bool>().await.unwrap_or(false);
                        scroll_checking.set(false);
                        if near_top {
                            if hidden_older_in_memory > 0 {
                                platform::snapshot_chat_scroll_height().await;
                                let cur = *dom_render_limit.peek();
                                dom_render_limit.set(cur.saturating_add(MESSAGE_DOM_WINDOW_STEP));
                                platform::restore_chat_scroll_after_prepend();
                            } else if let Some(h) = on_load_more {
                                h.call(());
                            }
                        }
                    });
                }
            },
            div { class: "message-stream",
                div { class: "message-stream-start" }
                if loading_initial && messages.is_empty() {
                    div { class: "chat-loading-initial", role: "status", aria_label: "Loading messages",
                        span { class: "spinner" }
                    }
                } else if hidden_older_in_memory > 0 {
                    div { class: "load-older-bar",
                        button {
                            r#type: "button",
                            class: "btn-ghost load-older-btn",
                            onclick: move |_| {
                                spawn(async move {
                                    platform::snapshot_chat_scroll_height().await;
                                    let cur = *dom_render_limit.peek();
                                    dom_render_limit.set(cur.saturating_add(MESSAGE_DOM_WINDOW_STEP));
                                    platform::restore_chat_scroll_after_prepend();
                                });
                            },
                            "Show more previous messages ({hidden_older_in_memory})"
                        }
                    }
                } else if loading_more {
                    div { class: "load-older-bar",
                        span { class: "spinner" }
                        span { "Loading older messages" }
                    }
                } else if has_more {
                    if let Some(h) = on_load_more {
                        div { class: "load-older-bar",
                            button {
                                r#type: "button",
                                class: "btn-ghost load-older-btn",
                                onclick: move |_| h.call(()),
                                "Load older messages"
                            }
                        }
                    }
                } else {
                    div { class: "conversation-intro",
                        if !intro_title.is_empty() {
                            div { class: "conversation-intro-title", "{intro_title}" }
                        }
                        if !intro_sub.is_empty() {
                            div { class: "empty-chat-sub", "{intro_sub}" }
                        }
                    }
                    if e2ee {
                        div { class: "e2ee-notice",
                            ShieldIcon { size: 16 }
                            span { "Messages are end-to-end encrypted on your device. The server only relays ciphertext and cannot read your content." }
                        }
                    }
                }
                for (i, msg) in messages.iter().enumerate() {
                    {
                        let prev = if i > 0 { messages.get(i - 1) } else { None };
                        let next = messages.get(i + 1);
                        let t = parsed[i];
                        let day = t.map(|d| d.date_naive());
                        let prev_day = if i > 0 { parsed[i - 1].map(|d| d.date_naive()) } else { None };
                        let new_day = day.is_some() && (i == 0 || day != prev_day);
                        let close = |a: Option<DateTime<Local>>, b: Option<DateTime<Local>>| match (a, b) {
                            (Some(a), Some(b)) => (b - a).num_seconds().abs() <= GROUP_WINDOW_SECS && a.date_naive() == b.date_naive(),
                            _ => false,
                        };
                        let joins_prev = !new_day
                            && prev.map(|p| p.sender_id == msg.sender_id && !p.is_error).unwrap_or(false)
                            && close(if i > 0 { parsed[i - 1] } else { None }, t);
                        let joins_next = next.map(|n| n.sender_id == msg.sender_id && !n.is_error).unwrap_or(false)
                            && close(t, parsed.get(i + 1).copied().flatten())
                            && parsed.get(i + 1).copied().flatten().map(|d| Some(d.date_naive()) == day).unwrap_or(false);
                        let side = if msg.is_self { "sent" } else { "received" };
                        let mut row_cls = format!("message-row {side}");
                        if !joins_prev { row_cls.push_str(" group-start"); }
                        if !joins_next { row_cls.push_str(" group-end"); }
                        if msg.is_pending { row_cls.push_str(" pending"); }
                        if highlight_id.as_deref() == Some(msg.id.as_str()) { row_cls.push_str(" highlight"); }
                        if msg.highlighted { row_cls.push_str(" admin-highlight"); }
                        if !mention_token.is_empty() && !msg.is_self && msg.text.to_lowercase().contains(&mention_token) { row_cls.push_str(" mentions-me"); }
                        let rich = if msg.is_error { None } else { parse_rich(&msg.text) };
                        let time_str = t.map(|d| d.format("%H:%M").to_string()).unwrap_or_default();
                        let full_time = t.map(|d| d.format("%H:%M, %d/%m/%Y").to_string()).unwrap_or_default();
                        let gif = if msg.is_error { None } else { gif_url_of(&msg.text) };
                        let sticker = if msg.is_error || gif.is_some() { None } else { sticker_url_of(&msg.text) };
                        let show_name = show_sender_names && !msg.is_self && !joins_prev;
                        let show_avatar = !msg.is_self && !joins_next;
                        let sender_for_avatar = msg.sender_id.clone();
                        let sender_for_name = msg.sender_id.clone();
                        let m_reply = msg.clone();
                        let m_reply2 = msg.clone();
                        let pin_id = msg.id.clone();
                        let pinned_now = msg.is_pinned;
                        let is_seen = seen_id.as_deref() == Some(msg.id.as_str());
                        let tally_for = tally.clone();
                        let m_copy = msg.text.clone();
                        let mid = msg.id.clone();
                        rsx! {
                          div { key: "{msg.id}", style: "display: contents;",
                            if new_day {
                                if let Some(d) = day {
                                    div { class: "date-separator", span { "{day_label(d)}" } }
                                }
                            }
                            div { id: "msg-{msg.id}", class: "{row_cls}",
                                div { class: "message-avatar-slot",
                                    if show_avatar {
                                        button {
                                            r#type: "button",
                                            aria_label: "View profile {msg.sender_name}",
                                            onclick: move |evt: MouseEvent| {
                                                let c = evt.client_coordinates();
                                                if let Some(h) = on_profile_click { h.call((sender_for_avatar.clone(), c.x, c.y)) }
                                                else if let Some(h) = on_open_profile { h.call(sender_for_avatar.clone()) }
                                            },
                                            Avatar { name: msg.sender_name.clone(), avatar_url: msg.sender_avatar.clone(), size: 36 }
                                        }
                                    }
                                }
                                div { class: "message-content-wrapper",
                                    if show_name {
                                        button {
                                            r#type: "button",
                                            class: "message-sender-name",
                                            onclick: move |evt: MouseEvent| {
                                                let c = evt.client_coordinates();
                                                if let Some(h) = on_profile_click { h.call((sender_for_name.clone(), c.x, c.y)) }
                                                else if let Some(h) = on_open_profile { h.call(sender_for_name.clone()) }
                                            },
                                            "{msg.sender_name}"
                                            if msg.is_bot { span { class: "bot-tag", "Plugin" } }
                                        }
                                    }
                                    if let Some(r) = rich.clone() {
                                        div { class: "message-rich", title: "{full_time}",
                                            RichContent {
                                                rich: r,
                                                message_id: msg.id.clone(),
                                                my_id: my_id.clone(),
                                                tally: tally_for,
                                                on_vote: on_send_raw.map(|h| EventHandler::new(move |(pid, i): (String, usize)| h.call(format!("vote:{pid}:{i}")))),
                                                on_open_invite: on_open_invite,
                                            }
                                        }
                                        span { class: "message-time-standalone",
                                            if msg.is_pinned { span { class: "pin-mark", aria_label: "Pinned", "📌" } }
                                            "{time_str}"
                                        }
                                    } else if let Some(url) = gif {
                                        div { class: "message-media gif", title: "{full_time}",
                                            img { class: "gif-img", src: "{platform::resolve_asset_url(&url)}", alt: "GIF", loading: "lazy" }
                                        }
                                        span { class: "message-time-standalone", "{time_str}" }
                                    } else if let Some(url) = sticker {
                                        {
                                            let menu_open = sticker_menu_for().as_deref() == Some(msg.id.as_str());
                                            let mid = msg.id.clone();
                                            let is_fav = fav_stickers.contains(&url);
                                            let (u1, u2, u3) = (url.clone(), url.clone(), url.clone());
                                            rsx! {
                                                div { class: "message-media sticker-media", title: "{full_time}",
                                                    button { r#type: "button", class: "sticker-open", aria_label: "Sticker options", aria_haspopup: "menu", aria_expanded: "{menu_open}",
                                                        onclick: move |_| sticker_menu_for.set(if menu_open { None } else { Some(mid.clone()) }),
                                                        img { class: "sticker-img", src: "{platform::resolve_asset_url(&url)}", alt: "Sticker", loading: "lazy" }
                                                    }
                                                    if menu_open {
                                                        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Close", onclick: move |_| sticker_menu_for.set(None) }
                                                        div { class: "account-menu sticker-menu", role: "menu",
                                                            onkeydown: move |e| if e.key() == Key::Escape { sticker_menu_for.set(None) },
                                                            if let Some(ctx) = prefs_ctx {
                                                                button { r#type: "button", role: "menuitem", class: "account-menu-item",
                                                                    onclick: move |_| { sticker_menu_for.set(None); crate::components::sticker_picker::toggle_favorite(ctx, &u1) },
                                                                    if is_fav { "★ Remove from favorite Stickers" } else { "☆ Add to favorite Sticker" }
                                                                }
                                                            }
                                                            if let Some(h) = on_send_raw {
                                                                button { r#type: "button", role: "menuitem", class: "account-menu-item",
                                                                    onclick: move |_| { sticker_menu_for.set(None); h.call(u2.clone()) }, "Resend" }
                                                            }
                                                            button { r#type: "button", role: "menuitem", class: "account-menu-item",
                                                                onclick: move |_| {
                                                                    sticker_menu_for.set(None);
                                                                    let name = sticker_search_term(&u3);
                                                                    *crate::components::sticker_picker::PICKER_REQUEST.write() = Some(name);
                                                                }, "Find similar Stickers" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        span { class: "message-time-standalone", "{time_str}" }
                                    } else {
                                        div {
                                            class: if msg.is_error { "message-bubble error-bubble" } else { "message-bubble" },
                                            title: "{full_time}",
                                            if let Some(rt) = msg.reply_text.as_ref() {
                                                span { class: "reply-quote", "{rt}" }
                                            }
                                            span { class: "message-text",
                                                for (si, seg) in segments(&msg.text).into_iter().enumerate() {
                                                    match seg {
                                                        Segment::Text(t) => rsx! { span { key: "{si}", MentionText { text: t } } },
                                                        Segment::Link(l) => rsx! {
                                                            a { key: "{si}", href: "{l}", target: "_blank", rel: "noopener noreferrer nofollow", "{l}" }
                                                        },
                                                        Segment::Emoji(name, url) => rsx! {
                                                            img { key: "{si}", class: "inline-emoji", src: "{platform::resolve_asset_url(&url)}", alt: ":{name}:", title: ":{name}:", loading: "lazy" }
                                                        },
                                                    }
                                                }
                                            }
                                            if !time_str.is_empty() {
                                                span { class: "bubble-meta",
                                                    if msg.is_pinned { span { class: "pin-mark", aria_label: "Pinned", "📌" } }
                                                    "{time_str}"
                                                    if msg.is_self && msg.is_pending {
                                                        span { class: "pending-clock", title: "Sending", aria_label: "Sending", ClockIcon { size: 12 } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !msg.reactions.is_empty() {
                                        div { class: "message-reactions-row",
                                            for (emoji, n, mine) in msg.reactions.iter().cloned() {
                                                {
                                                    let mid_r = mid.clone();
                                                    let em = emoji.clone();
                                                    rsx! {
                                                        button {
                                                            key: "{emoji}",
                                                            r#type: "button",
                                                            class: if mine { "reaction-pill mine" } else { "reaction-pill" },
                                                            aria_label: "Expressions of {emoji}, {n} people",
                                                            disabled: on_react.is_none(),
                                                            onclick: move |_| if let Some(h) = on_react { h.call((mid_r.clone(), em.clone())) },
                                                            span { "{emoji}" }
                                                            if n > 1 { span { class: "reaction-count", "{n}" } }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !msg.is_error {
                                    div { class: "message-actions", role: "toolbar", aria_label: "Message manipulation",
                                        if on_react.is_some() {
                                            div { class: "react-trigger",
                                                button {
                                                    r#type: "button",
                                                    class: "message-action-btn",
                                                    title: "Drop expressions",
                                                    aria_label: "Drop expressions",
                                                    aria_haspopup: "true",
                                                    SmilePlusIcon {}
                                                }
                                                div { class: "react-tray", role: "group", aria_label: "Quick expressions",
                                                    for em in QUICK_REACTIONS {
                                                        {
                                                            let mid_q = mid.clone();
                                                            rsx! {
                                                                button {
                                                                    key: "{em}",
                                                                    r#type: "button",
                                                                    class: "react-option",
                                                                    title: "{em}",
                                                                    aria_label: "Drop emote {em}",
                                                                    onclick: move |_| if let Some(h) = on_react { h.call((mid_q.clone(), em.to_string())) },
                                                                    "{em}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "react-option more",
                                                        title: "Add expressions",
                                                        aria_label: "Add expressions",
                                                        onclick: { let m = mid.clone(); move |_| full_picker_for.set(Some(m.clone())) },
                                                        SmilePlusIcon { size: 16 }
                                                    }
                                                }
                                                if full_picker_for().as_deref() == Some(mid.as_str()) {
                                                    div { class: "react-picker",
                                                        EmojiPicker {
                                                            on_select: { let m = mid.clone(); move |e: String| {
                                                                full_picker_for.set(None);
                                                                if !e.starts_with("<:") { if let Some(h) = on_react { h.call((m.clone(), e)) } }
                                                            }},
                                                            on_close: move |_| full_picker_for.set(None),
                                                            align_right: msg.is_self,
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if on_pin.is_some() {
                                            button {
                                                r#type: "button",
                                                class: "message-action-btn",
                                                title: if pinned_now { "Unpin" } else { "Pin message" },
                                                aria_label: if pinned_now { "Unpin" } else { "Pin message" },
                                                onclick: move |_| if let Some(h) = on_pin { h.call((pin_id.clone(), !pinned_now)) },
                                                PinIcon {}
                                            }
                                        }
                                        if on_reply.is_some() {
                                            button {
                                                r#type: "button",
                                                class: "message-action-btn",
                                                title: "Reply",
                                                aria_label: "Reply",
                                                onclick: move |_| if let Some(h) = on_reply { h.call(m_reply.clone()) },
                                                ReplyIcon {}
                                            }
                                        }
                                        if let Some(h) = on_create_task {
                                            button {
                                                r#type: "button",
                                                class: "message-action-btn",
                                                title: "Create jobs",
                                                aria_label: "Create jobs",
                                                onclick: { let id = mid.clone(); let t = m_copy.clone(); move |_| h.call((id.clone(), t.clone())) },
                                                crate::components::icons::ListChecksIcon { size: 16 }
                                            }
                                        }
                                        if let Some(h) = on_save_self {
                                            button {
                                                r#type: "button",
                                                class: "message-action-btn",
                                                title: "Saved to Self",
                                                aria_label: "Saved to Self",
                                                onclick: { let t = m_copy.clone(); move |_| h.call(t.clone()) },
                                                crate::components::icons::DevicesIcon { size: 16 }
                                            }
                                        }
                                        for pact in plugin_actions() {
                                            {
                                                let iid = pact.installation_id.clone();
                                                let aid = pact.id.clone();
                                                let mid_p = mid.clone();
                                                let txt_p = m_copy.clone();
                                                let sender_p = msg.sender_name.clone();
                                                rsx! {
                                                    button {
                                                        key: "pact-{pact.installation_id}-{pact.id}",
                                                        r#type: "button",
                                                        class: "message-action-btn plugin-msg-action-btn",
                                                        title: "{pact.label} ({pact.plugin_name})",
                                                        aria_label: "{pact.label}",
                                                        "data-plugin-msg-action": "{pact.id}",
                                                        onclick: move |_| {
                                                            let iid2 = iid.clone();
                                                            let aid2 = aid.clone();
                                                            let payload = serde_json::json!({
                                                                "id": mid_p,
                                                                "content": txt_p,
                                                                "text": txt_p,
                                                                "senderName": sender_p,
                                                            });
                                                            spawn(async move {
                                                                let _ = crate::services::plugin_api::host_trigger_message_action(&iid2, &aid2, payload).await;
                                                            });
                                                        },
                                                        span { class: "plugin-msg-action-label", "{pact.label}" }
                                                    }
                                                }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "message-action-btn",
                                            title: "Copy",
                                            aria_label: "Copy content",
                                            onclick: move |_| platform::copy_to_clipboard(&m_copy),
                                            CopyIcon {}
                                        }
                                    }
                                }
                                if double_click_reply && on_reply.is_some() {
                                    div {
                                        class: "message-gutter",
                                        title: "Double click to reply",
                                        ondoubleclick: move |_| if let Some(h) = on_reply { h.call(m_reply2.clone()) },
                                    }
                                }
                            }
                            if is_seen {
                                div { class: "seen-indicator", "Seen" }
                            }
                          }
                        }
                    }
                }
                if count == 0 && intro_title.is_empty() {
                    div { class: "empty-chat-inner", "No messages yet" }
                }
            }
        }
    }
}

pub use crate::components::composer::{Composer, MentionCandidate};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windowed_message_slice_100000_messages_performance_and_highlight() {
        let total = 100_000usize;
        let t0 = std::time::Instant::now();
        let (s1, e1) = compute_message_window_bounds(total, MESSAGE_DOM_WINDOW_STEP, None);
        assert_eq!(e1, 100_000);
        assert_eq!(e1 - s1, MESSAGE_DOM_WINDOW_STEP);
        assert_eq!(s1, 99_880);

        // When jumping to an older message (#420), window expands smoothly to include #420 with 20-message context
        let (s2, e2) = compute_message_window_bounds(total, MESSAGE_DOM_WINDOW_STEP, Some(420));
        assert_eq!(s2, 400);
        assert_eq!(e2, 100_000);
        assert!(t0.elapsed().as_micros() < 1000, "window bound calculation must be O(1) sub-millisecond");
    }

    #[test]
    fn test_day_label_current_year_and_previous_year() {
        let cur_year = Local::now().date_naive().year();
        let d_cur = NaiveDate::from_ymd_opt(cur_year, 11, 20).unwrap();
        assert_eq!(day_label(d_cur), "November 20");
        let d_old = NaiveDate::from_ymd_opt(2025, 11, 22).unwrap();
        if cur_year != 2025 {
            assert_eq!(day_label(d_old), "November 22, 2025");
        }
    }
}
