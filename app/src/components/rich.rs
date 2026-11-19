//! Rich message types carried as text (inside the E2EE envelope for direct messages):
//!
//! * `poll:{json}`      — question + options; votes are `vote:{poll_id}:{index}` messages
//! * `note:{json}`      — a titled note
//! * `reminder:{json}`  — reminder with a local time; the app notifies when due
//! * `table:<tsv>`      — spreadsheet data pasted as a table
//! * `img:{json}`       — image; E2EE images carry the AES key, group images a plain URL
//! * `invite:{json}`    — a rich invite card for a group / community

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::{
        avatar::Avatar,
        icons::{BellIcon, CheckIcon, CloseIcon, PlusIcon},
    },
    crypto::blob,
    platform,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PollData {
    pub id: String,
    pub q: String,
    pub o: Vec<String>,
    #[serde(default)]
    pub multi: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteData {
    pub t: String,
    #[serde(default)]
    pub b: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReminderData {
    pub id: String,
    pub t: String,
    /// Local datetime "YYYY-MM-DDTHH:MM"
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    pub u: String,
    #[serde(default)]
    pub k: String,
    #[serde(default)]
    pub n: String,
    #[serde(default)]
    pub m: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InviteData {
    pub kind: String,
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub members: i64,
    #[serde(default)]
    pub from: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Rich {
    Poll(PollData),
    Vote { poll_id: String, option: usize },
    Note(NoteData),
    Reminder(ReminderData),
    Table(Vec<Vec<String>>),
    Image(ImageData),
    Invite(InviteData),
}

pub fn parse_rich(text: &str) -> Option<Rich> {
    let t = text.trim_start();
    if let Some(rest) = t.strip_prefix("poll:") {
        return serde_json::from_str::<PollData>(rest).ok().filter(|p| p.o.len() >= 2 && p.o.len() <= 10).map(Rich::Poll);
    }
    if let Some(rest) = t.strip_prefix("vote:") {
        let mut it = rest.trim().splitn(2, ':');
        let id = it.next()?.to_string();
        let option = it.next()?.trim().parse().ok()?;
        return Some(Rich::Vote { poll_id: id, option });
    }
    if let Some(rest) = t.strip_prefix("note:") {
        return serde_json::from_str::<NoteData>(rest).ok().map(Rich::Note);
    }
    if let Some(rest) = t.strip_prefix("reminder:") {
        return serde_json::from_str::<ReminderData>(rest).ok().map(Rich::Reminder);
    }
    if let Some(rest) = t.strip_prefix("table:") {
        let rows: Vec<Vec<String>> = rest
            .split('\n')
            .filter(|r| !r.is_empty())
            .take(200)
            .map(|r| r.split('\t').take(30).map(str::to_string).collect())
            .collect();
        return (!rows.is_empty()).then_some(Rich::Table(rows));
    }
    if let Some(rest) = t.strip_prefix("img:") {
        return serde_json::from_str::<ImageData>(rest).ok().filter(|i| valid_image_url(&i.u)).map(Rich::Image);
    }
    if let Some(rest) = t.strip_prefix("invite:") {
        return serde_json::from_str::<InviteData>(rest)
            .ok()
            .filter(|i| (i.kind == "group" || i.kind == "server") && i.code.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(Rich::Invite);
    }
    None
}

fn valid_image_url(u: &str) -> bool {
    (u.starts_with("/api/attachments/") || u.starts_with("/uploads/media/")) && !u.contains("..") && !u.contains(char::is_whitespace)
}

/// Short preview text for lists / notifications.
pub fn rich_preview(r: &Rich) -> String {
    match r {
        Rich::Poll(p) => format!("Bình chọn: {}", p.q),
        Rich::Vote { .. } => "Đã bình chọn".into(),
        Rich::Note(n) => format!("Ghi chú: {}", n.t),
        Rich::Reminder(r) => format!("Nhắc hẹn: {}", r.t),
        Rich::Table(_) => "Bảng tính".into(),
        Rich::Image(_) => "Hình ảnh".into(),
        Rich::Invite(i) => format!("Lời mời tham gia {}", i.name),
    }
}

pub fn reminder_due(r: &ReminderData) -> Option<DateTime<Local>> {
    let naive = NaiveDateTime::parse_from_str(&r.at, "%Y-%m-%dT%H:%M").ok()?;
    Local.from_local_datetime(&naive).single()
}

/// Votes per poll: poll_id -> (voter -> option index). The latest vote wins.
pub type VoteTally = std::collections::HashMap<String, std::collections::HashMap<String, usize>>;

/// Image message (decrypts client-side when a key is present).
#[component]
pub fn RichImage(data: ImageData) -> Element {
    let mut src = use_signal(|| Option::<String>::None);
    let mut failed = use_signal(|| false);
    let mut open = use_signal(|| false);
    let d = data.clone();
    use_effect(use_reactive!(|d| {
        spawn(async move {
            if d.k.is_empty() {
                src.set(Some(platform::resolve_asset_url(&d.u)));
                return;
            }
            match platform::fetch_bytes(&d.u).await.and_then(|c| blob::open(&c, &d.k, &d.n)) {
                Ok(bytes) => {
                    let mime = if d.m.starts_with("image/") { d.m.clone() } else { "image/png".into() };
                    src.set(Some(platform::image_bytes_to_data_url(&bytes, &mime).await));
                }
                Err(_) => failed.set(true),
            }
        });
    }));
    rsx! {
        div { class: "rich-image",
            if let Some(s) = src() {
                button { r#type: "button", class: "rich-image-btn", aria_label: "Xem ảnh", onclick: move |_| open.set(true),
                    img { src: "{s}", alt: "Hình ảnh", loading: "lazy" }
                }
                if open() {
                    div { class: "lightbox", role: "dialog", aria_label: "Ảnh", onclick: move |_| open.set(false),
                        img { src: "{s}", alt: "Hình ảnh" }
                        button { r#type: "button", class: "icon-btn lightbox-close", aria_label: "Đóng", CloseIcon {} }
                    }
                }
            } else if failed() {
                div { class: "rich-image-error", "Không giải mã được ảnh" }
            } else {
                div { class: "rich-image-loading", span { class: "spinner" } }
            }
        }
    }
}

#[component]
pub fn RichContent(
    rich: Rich,
    message_id: String,
    #[props(default)] my_id: String,
    #[props(default)] tally: VoteTally,
    #[props(default)] on_vote: Option<EventHandler<(String, usize)>>,
    #[props(default)] on_open_invite: Option<EventHandler<(String, String)>>,
) -> Element {
    match rich {
        Rich::Poll(p) => {
            let votes = tally.get(&p.id).cloned().unwrap_or_default();
            let total = votes.len().max(1);
            let mine = votes.get(&my_id).copied();
            rsx! {
                div { class: "rich-card poll-card",
                    div { class: "rich-kind", "Bình chọn" }
                    div { class: "rich-title", "{p.q}" }
                    div { class: "poll-options",
                        for (i, opt) in p.o.iter().enumerate() {
                            {
                                let count = votes.values().filter(|v| **v == i).count();
                                let pct = count * 100 / total;
                                let pid = p.id.clone();
                                rsx! {
                                    button {
                                        key: "{i}",
                                        r#type: "button",
                                        class: if mine == Some(i) { "poll-option chosen" } else { "poll-option" },
                                        disabled: on_vote.is_none(),
                                        onclick: move |_| if let Some(h) = on_vote { h.call((pid.clone(), i)) },
                                        span { class: "poll-bar", style: "width: {pct}%;" }
                                        span { class: "poll-label", if mine == Some(i) { CheckIcon { size: 14 } } "{opt}" }
                                        span { class: "poll-count", "{count}" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "rich-foot", "{votes.len()} lượt bình chọn" }
                }
            }
        }
        Rich::Note(n) => rsx! {
            div { class: "rich-card note-card",
                div { class: "rich-kind", "Ghi chú" }
                div { class: "rich-title", "{n.t}" }
                if !n.b.is_empty() { div { class: "rich-body", "{n.b}" } }
            }
        },
        Rich::Reminder(r) => {
            let when = reminder_due(&r).map(|d| d.format("%H:%M, %d/%m/%Y").to_string()).unwrap_or_else(|| r.at.clone());
            let past = reminder_due(&r).map(|d| d < Local::now()).unwrap_or(false);
            rsx! {
                div { class: "rich-card reminder-card",
                    div { class: "rich-kind", BellIcon { size: 14 } "Nhắc hẹn" }
                    div { class: "rich-title", "{r.t}" }
                    div { class: if past { "rich-body muted" } else { "rich-body" }, "{when}" if past { " · đã qua" } }
                }
            }
        }
        Rich::Table(rows) => rsx! {
            div { class: "rich-card table-card",
                div { class: "rich-kind", "Bảng tính · {rows.len()} hàng" }
                div { class: "table-scroll",
                    table {
                        for (ri, row) in rows.iter().enumerate() {
                            tr { key: "{ri}",
                                for (ci, cell) in row.iter().enumerate() {
                                    if ri == 0 { th { key: "{ci}", "{cell}" } } else { td { key: "{ci}", "{cell}" } }
                                }
                            }
                        }
                    }
                }
            }
        },
        Rich::Image(d) => rsx! { RichImage { data: d } },
        Rich::Invite(i) => {
            let kind_label = if i.kind == "server" { "Cộng đồng" } else { "Nhóm" };
            // Card fields come from another user: only same-origin asset paths are rendered.
            let banner = i.banner.as_ref().filter(|b| platform::safe_asset_path(b)).map(|b| format!("background-image: url('{}');", platform::resolve_asset_url(b))).unwrap_or_default();
            let avatar = i.avatar.clone().filter(|a| platform::safe_asset_path(a));
            let (k, c) = (i.kind.clone(), i.code.clone());
            rsx! {
                div { class: "rich-card invite-card",
                    div { class: "invite-banner", style: "{banner}" }
                    div { class: "invite-body",
                        Avatar { name: i.name.clone(), avatar_url: avatar, size: 48 }
                        div { class: "invite-text",
                            div { class: "rich-kind", "Lời mời vào {kind_label}" }
                            div { class: "rich-title", "{i.name}" }
                            div { class: "rich-body muted", "{i.members} thành viên" if !i.from.is_empty() { " · mời bởi {i.from}" } }
                        }
                    }
                    button { r#type: "button", class: "btn-primary btn-block", disabled: on_open_invite.is_none(),
                        onclick: move |_| if let Some(h) = on_open_invite { h.call((k.clone(), c.clone())) },
                        "Xem và tham gia"
                    }
                }
            }
        }
        Rich::Vote { .. } => rsx! {},
    }
}

fn now_local_input() -> String {
    (Local::now() + chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M").to_string()
}

/// "+" menu in the composer: image, poll, note, reminder.
#[component]
pub fn ComposerTools(
    input_id: String,
    on_raw: EventHandler<String>,
    on_image: EventHandler<(Vec<u8>, String)>,
    #[props(default = true)] allow_poll: bool,
    #[props(default = true)] allow_note: bool,
    #[props(default = true)] allow_reminder: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let mut dialog = use_signal(|| Option::<&'static str>::None);
    let mut q = use_signal(String::new);
    let mut opts = use_signal(|| vec![String::new(), String::new()]);
    let mut title = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut at = use_signal(now_local_input);
    let file_id = format!("{input_id}-image");
    let file_id2 = file_id.clone();
    rsx! {
        div { class: "composer-tools",
            button {
                r#type: "button",
                class: if open() { "icon-btn active" } else { "icon-btn" },
                title: "Thêm",
                aria_label: "Thêm ảnh, bình chọn, ghi chú, nhắc hẹn",
                aria_expanded: "{open()}",
                onclick: move |e| { e.stop_propagation(); open.set(!open()); },
                PlusIcon { size: 20 }
            }
            if open() {
                button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Đóng", onclick: move |_| open.set(false) }
                div { class: "account-menu tools-menu", role: "menu",
                    label { class: "menu-item", role: "menuitem",
                        input {
                            id: "{file_id}",
                            r#type: "file",
                            accept: "image/png,image/jpeg,image/webp,image/gif",
                            style: "display: none;",
                            onchange: move |_| {
                                open.set(false);
                                let fid = file_id2.clone();
                                spawn(async move {
                                    if let Ok((bytes, mime)) = platform::read_selected_file_raw(&fid).await {
                                        on_image.call((bytes, mime));
                                    }
                                });
                            },
                        }
                        "Ảnh"
                    }
                    if allow_poll { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("poll")); }, "Bình chọn" } }
                    if allow_note { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("note")); }, "Ghi chú" } }
                    if allow_reminder { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("reminder")); }, "Nhắc hẹn" } }
                }
            }
            if let Some(kind) = dialog() {
                div { class: "modal-overlay", onclick: move |_| dialog.set(None),
                    div { class: "modal-dialog", role: "dialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                        onkeydown: move |e| if e.key() == Key::Escape { dialog.set(None) },
                        div { class: "modal-header",
                            h2 { class: "modal-title",
                                match kind { "poll" => "Tạo bình chọn", "note" => "Tạo ghi chú", _ => "Tạo nhắc hẹn" }
                            }
                            button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| dialog.set(None), CloseIcon {} }
                        }
                        div { class: "modal-body",
                            if kind == "poll" {
                                label { class: "form-field", span { class: "form-label", "Câu hỏi" }
                                    input { maxlength: "200", value: "{q}", oninput: move |e| q.set(e.value()), placeholder: "Đi ăn trưa ở đâu?" }
                                }
                                div { class: "form-field",
                                    span { class: "form-label", "Lựa chọn" }
                                    for (i, o) in opts().iter().enumerate() {
                                        div { key: "{i}", class: "poll-edit-row",
                                            input { maxlength: "80", value: "{o}", placeholder: "Lựa chọn {i + 1}",
                                                oninput: move |e| { let v = e.value(); opts.with_mut(|l| l[i] = v); } }
                                            if opts.read().len() > 2 {
                                                button { r#type: "button", class: "icon-btn", aria_label: "Xóa lựa chọn", onclick: move |_| { opts.with_mut(|l| { l.remove(i); }); }, CloseIcon { size: 16 } }
                                            }
                                        }
                                    }
                                    if opts.read().len() < 10 {
                                        button { r#type: "button", class: "btn-ghost", style: "align-self: flex-start;", onclick: move |_| opts.with_mut(|l| l.push(String::new())), PlusIcon { size: 16 } "Thêm lựa chọn" }
                                    }
                                }
                            } else if kind == "note" {
                                label { class: "form-field", span { class: "form-label", "Tiêu đề" } input { maxlength: "120", value: "{title}", oninput: move |e| title.set(e.value()) } }
                                label { class: "form-field", span { class: "form-label", "Nội dung" } textarea { rows: "5", maxlength: "2000", value: "{body}", oninput: move |e| body.set(e.value()) } }
                            } else {
                                label { class: "form-field", span { class: "form-label", "Nội dung nhắc" } input { maxlength: "120", value: "{title}", oninput: move |e| title.set(e.value()), placeholder: "Họp nhóm" } }
                                label { class: "form-field", span { class: "form-label", "Thời gian" } input { r#type: "datetime-local", value: "{at}", oninput: move |e| at.set(e.value()) } }
                                div { class: "form-hint", "ChatW sẽ thông báo cho mọi người trong cuộc trò chuyện khi đến giờ." }
                            }
                        }
                        div { class: "modal-footer",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| dialog.set(None), "Hủy" }
                            button { r#type: "button", class: "btn-primary",
                                onclick: move |_| {
                                    let payload = match kind {
                                        "poll" => {
                                            let options: Vec<String> = opts().into_iter().map(|o| o.trim().to_string()).filter(|o| !o.is_empty()).collect();
                                            if q().trim().is_empty() || options.len() < 2 { return; }
                                            let p = PollData { id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(), q: q().trim().to_string(), o: options, multi: false };
                                            format!("poll:{}", serde_json::to_string(&p).unwrap_or_default())
                                        }
                                        "note" => {
                                            if title().trim().is_empty() { return; }
                                            format!("note:{}", serde_json::to_string(&NoteData { t: title().trim().into(), b: body().trim().into() }).unwrap_or_default())
                                        }
                                        _ => {
                                            if title().trim().is_empty() || at().is_empty() { return; }
                                            let r = ReminderData { id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(), t: title().trim().into(), at: at() };
                                            format!("reminder:{}", serde_json::to_string(&r).unwrap_or_default())
                                        }
                                    };
                                    on_raw.call(payload);
                                    q.set(String::new());
                                    opts.set(vec![String::new(), String::new()]);
                                    title.set(String::new());
                                    body.set(String::new());
                                    at.set(now_local_input());
                                    dialog.set(None);
                                },
                                "Gửi"
                            }
                        }
                    }
                }
            }
        }
    }
}
