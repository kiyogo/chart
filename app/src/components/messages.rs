//! Shared message list + composer used by DMs, group DMs and community channels.

use chrono::{DateTime, Datelike, Local, NaiveDate};
use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        emoji_picker::EmojiPicker,
        icons::{CloseIcon, EmojiIcon, SendIcon, ShieldIcon, StickerIcon},
        sticker_picker::StickerPicker,
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
}

const GROUP_WINDOW_SECS: i64 = 5 * 60;

fn parse_local(iso: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(iso).ok().map(|d| d.with_timezone(&Local))
}

fn day_label(date: NaiveDate) -> String {
    let today = Local::now().date_naive();
    if date == today {
        "Hôm nay".to_string()
    } else if Some(date) == today.pred_opt() {
        "Hôm qua".to_string()
    } else if date.year() == today.year() {
        format!("{} tháng {}", date.day(), date.month())
    } else {
        format!("{:02}/{:02}/{}", date.day(), date.month(), date.year())
    }
}

/// A message body that is only an uploaded image path is rendered as a sticker/media.
pub fn sticker_url_of(text: &str) -> Option<String> {
    let t = text.trim();
    if t.contains(char::is_whitespace) {
        return None;
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

enum Segment {
    Text(String),
    Link(String),
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
                    out.push(Segment::Text(std::mem::take(&mut buf)));
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
        out.push(Segment::Text(buf));
    }
    out
}

#[component]
pub fn MessageList(
    messages: Vec<ViewMessage>,
    #[props(default)] show_sender_names: bool,
    #[props(default)] e2ee: bool,
    #[props(default)] intro_title: String,
    #[props(default)] intro_sub: String,
    #[props(default)] on_open_profile: Option<EventHandler<String>>,
    #[props(default)] on_react: Option<EventHandler<(String, String)>>,
    #[props(default)] on_reply: Option<EventHandler<ViewMessage>>,
    #[props(default)] highlight_id: Option<String>,
) -> Element {
    let count = messages.len();
    let parsed: Vec<Option<DateTime<Local>>> = messages.iter().map(|m| parse_local(&m.created_at)).collect();

    rsx! {
        div { id: "chat-messages-scroll", class: "chat-messages", role: "log", aria_live: "polite",
            div { class: "message-stream",
                div { class: "message-stream-start" }
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
                        span { "Tin nhắn được mã hóa đầu cuối trên thiết bị. Máy chủ chỉ chuyển tiếp bản mã và không thể đọc nội dung." }
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
                        if highlight_id.as_deref() == Some(msg.id.as_str()) { row_cls.push_str(" highlight"); }
                        let time_str = t.map(|d| d.format("%H:%M").to_string()).unwrap_or_default();
                        let full_time = t.map(|d| d.format("%H:%M, %d/%m/%Y").to_string()).unwrap_or_default();
                        let sticker = if msg.is_error { None } else { sticker_url_of(&msg.text) };
                        let show_name = show_sender_names && !msg.is_self && !joins_prev;
                        let show_avatar = !msg.is_self && !joins_next;
                        let sender_for_avatar = msg.sender_id.clone();
                        let sender_for_name = msg.sender_id.clone();
                        let m_reply = msg.clone();
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
                                            aria_label: "Xem hồ sơ {msg.sender_name}",
                                            onclick: move |_| if let Some(h) = on_open_profile { h.call(sender_for_avatar.clone()) },
                                            Avatar { name: msg.sender_name.clone(), avatar_url: msg.sender_avatar.clone(), size: 32 }
                                        }
                                    }
                                }
                                div { class: "message-content-wrapper",
                                    if show_name {
                                        button {
                                            r#type: "button",
                                            class: "message-sender-name",
                                            onclick: move |_| if let Some(h) = on_open_profile { h.call(sender_for_name.clone()) },
                                            "{msg.sender_name}"
                                        }
                                    }
                                    if let Some(url) = sticker {
                                        div { class: "message-media", title: "{full_time}",
                                            img { class: "sticker-img", src: "{platform::resolve_asset_url(&url)}", alt: "Sticker", loading: "lazy" }
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
                                                        Segment::Text(t) => rsx! { span { key: "{si}", "{t}" } },
                                                        Segment::Link(l) => rsx! {
                                                            a { key: "{si}", href: "{l}", target: "_blank", rel: "noopener noreferrer nofollow", "{l}" }
                                                        },
                                                    }
                                                }
                                            }
                                            if !time_str.is_empty() {
                                                span { class: "bubble-meta",
                                                    "{time_str}"
                                                    if msg.is_self && msg.is_pending {
                                                        span { aria_label: "Đang gửi", "·" }
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
                                                            aria_label: "Biểu cảm {emoji}, {n} người",
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
                                    div { class: "message-actions", role: "toolbar", aria_label: "Thao tác tin nhắn",
                                        if on_react.is_some() {
                                            for em in ["👍", "❤️", "😂"] {
                                                {
                                                    let mid_q = mid.clone();
                                                    rsx! {
                                                        button {
                                                            key: "{em}",
                                                            r#type: "button",
                                                            class: "message-action-btn",
                                                            title: "Thả {em}",
                                                            aria_label: "Thả biểu cảm {em}",
                                                            onclick: move |_| if let Some(h) = on_react { h.call((mid_q.clone(), em.to_string())) },
                                                            "{em}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if on_reply.is_some() {
                                            button {
                                                r#type: "button",
                                                class: "message-action-btn",
                                                title: "Trả lời",
                                                aria_label: "Trả lời",
                                                onclick: move |_| if let Some(h) = on_reply { h.call(m_reply.clone()) },
                                                svg { width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                                    polyline { points: "9 17 4 12 9 7" }
                                                    path { d: "M20 18v-2a4 4 0 0 0-4-4H4" }
                                                }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "message-action-btn",
                                            title: "Sao chép",
                                            aria_label: "Sao chép nội dung",
                                            onclick: move |_| platform::copy_to_clipboard(&m_copy),
                                            svg { width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                                rect { x: "9", y: "9", width: "13", height: "13", rx: "2" }
                                                path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                                            }
                                        }
                                    }
                                }
                            }
                          }
                        }
                    }
                }
                if count == 0 && intro_title.is_empty() {
                    div { class: "empty-chat-inner", "Chưa có tin nhắn" }
                }
            }
        }
    }
}

#[component]
pub fn Composer(
    input_id: String,
    value: String,
    placeholder: String,
    on_input: EventHandler<String>,
    on_send: EventHandler<()>,
    #[props(default)] sending: bool,
    #[props(default)] reply_to: Option<String>,
    #[props(default)] on_cancel_reply: Option<EventHandler<()>>,
    #[props(default)] on_sticker: Option<EventHandler<String>>,
    #[props(default)] disabled: bool,
) -> Element {
    let mut emoji_open = use_signal(|| false);
    let mut sticker_open = use_signal(|| false);
    let can_send = !value.trim().is_empty() && !sending && !disabled;
    let id_for_emoji = input_id.clone();
    let value_for_emoji = value.clone();

    rsx! {
        div { class: "composer",
            div { class: "composer-inner",
                if emoji_open() {
                    EmojiPicker {
                        on_select: move |emoji: String| {
                            let cur = value_for_emoji.clone();
                            let id = id_for_emoji.clone();
                            spawn(async move {
                                let updated = platform::insert_text_in_input(&id, &cur, &emoji).await;
                                on_input.call(updated);
                            });
                        },
                        on_close: move |_| emoji_open.set(false),
                    }
                }
                if sticker_open() {
                    if let Some(on_st) = on_sticker {
                        div { class: "picker-popover align-right", style: "padding: 0;",
                            StickerPicker {
                                on_select: move |url: String| {
                                    sticker_open.set(false);
                                    on_st.call(url);
                                },
                                _on_close: Some(EventHandler::new(move |_| sticker_open.set(false))),
                            }
                        }
                    }
                }
                if let Some(rt) = reply_to.as_ref() {
                    div { class: "composer-context",
                        span { class: "composer-context-text", "Trả lời: {rt}" }
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            aria_label: "Hủy trả lời",
                            onclick: move |_| if let Some(h) = on_cancel_reply { h.call(()) },
                            CloseIcon { size: 16 }
                        }
                    }
                }
                div { class: "composer-box",
                    button {
                        r#type: "button",
                        class: if emoji_open() { "icon-btn active" } else { "icon-btn" },
                        title: "Emoji",
                        aria_label: "Chọn emoji",
                        aria_expanded: "{emoji_open()}",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            sticker_open.set(false);
                            emoji_open.set(!emoji_open());
                        },
                        EmojiIcon {}
                    }
                    textarea {
                        id: "{input_id}",
                        class: "composer-input",
                        rows: "1",
                        placeholder: "{placeholder}",
                        aria_label: "{placeholder}",
                        disabled: disabled,
                        value: "{value}",
                        oninput: move |evt| on_input.call(evt.value()),
                        onkeydown: move |evt| {
                            let key = evt.key();
                            if key == Key::Enter && !evt.modifiers().shift() && !evt.is_composing() {
                                evt.prevent_default();
                                on_send.call(());
                            } else if key == Key::Escape {
                                emoji_open.set(false);
                                sticker_open.set(false);
                            }
                        },
                    }
                    if on_sticker.is_some() {
                        button {
                            r#type: "button",
                            class: if sticker_open() { "icon-btn active" } else { "icon-btn" },
                            title: "Sticker",
                            aria_label: "Chọn sticker",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                emoji_open.set(false);
                                sticker_open.set(!sticker_open());
                            },
                            StickerIcon {}
                        }
                    }
                    button {
                        r#type: "button",
                        class: "send-btn",
                        title: "Gửi (Enter)",
                        aria_label: "Gửi",
                        disabled: !can_send,
                        onclick: move |_| on_send.call(()),
                        SendIcon {}
                    }
                }
                div { class: "composer-hint hide-mobile",
                    span { class: "kbd", "Enter" } " gửi · " span { class: "kbd", "Shift + Enter" } " xuống dòng"
                }
            }
        }
    }
}
