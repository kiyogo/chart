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
        icons::{BarChartIcon, BellIcon, CheckIcon, CloseIcon, DownloadIcon, FileTextIcon, MicIcon, PlusIcon, RefreshIcon},
    },
    platform,
    services::storage_sync,
    storage::{
        availability_badge_label, format_bytes_human,
        media_opt::{self, OptimizedMediaPayload},
        recovery::{self, MediaFetchOutcome},
    },
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
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub blur: String,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaFileData {
    pub u: String,
    #[serde(default)]
    pub k: String,
    #[serde(default)]
    pub n: String,
    #[serde(default)]
    pub m: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub dur_ms: u64,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub blur: String,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
    #[serde(default)]
    pub waveform: Vec<u8>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EmbedAuthorItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EmbedMediaItem {
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EmbedFooterItem {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EmbedFieldItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DiscordEmbedItem {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub author: Option<EmbedAuthorItem>,
    #[serde(default)]
    pub footer: Option<EmbedFooterItem>,
    #[serde(default)]
    pub thumbnail: Option<EmbedMediaItem>,
    #[serde(default)]
    pub image: Option<EmbedMediaItem>,
    #[serde(default)]
    pub fields: Vec<EmbedFieldItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct WebhookComponentButton {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct WebhookEmbedData {
    #[serde(default)]
    pub webhook_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub embeds: Vec<DiscordEmbedItem>,
    #[serde(default)]
    pub components: Vec<WebhookComponentButton>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Rich {
    Poll(PollData),
    Vote { poll_id: String, option: usize },
    Note(NoteData),
    Reminder(ReminderData),
    Table(Vec<Vec<String>>),
    Image(ImageData),
    File(MediaFileData),
    Audio(MediaFileData),
    Video(MediaFileData),
    Invite(InviteData),
    Embed(WebhookEmbedData),
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
    if let Some(rest) = t.strip_prefix("file:") {
        return serde_json::from_str::<MediaFileData>(rest).ok().filter(|f| valid_image_url(&f.u)).map(Rich::File);
    }
    if let Some(rest) = t.strip_prefix("audio:") {
        return serde_json::from_str::<MediaFileData>(rest).ok().filter(|f| valid_image_url(&f.u)).map(Rich::Audio);
    }
    if let Some(rest) = t.strip_prefix("video:") {
        return serde_json::from_str::<MediaFileData>(rest).ok().filter(|f| valid_image_url(&f.u)).map(Rich::Video);
    }
    if let Some(rest) = t.strip_prefix("invite:") {
        return serde_json::from_str::<InviteData>(rest)
            .ok()
            .filter(|i| (i.kind == "group" || i.kind == "server") && i.code.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(Rich::Invite);
    }
    if let Some(rest) = t.strip_prefix("embed:") {
        return serde_json::from_str::<WebhookEmbedData>(rest).ok().map(Rich::Embed);
    }
    None
}

fn is_safe_ext_url(u: &str) -> bool {
    let l = u.trim().to_ascii_lowercase();
    (l.starts_with("https://") || l.starts_with("http://")) && !l.contains("javascript:") && !l.contains("data:")
}

pub fn build_rich_media_wire(
    url: String,
    key_b64: String,
    nonce_b64: String,
    scope: &str,
    payload: &OptimizedMediaPayload,
) -> String {
    let blur = payload
        .blur_data_url
        .as_deref()
        .filter(|s| s.starts_with("data:image/") && s.len() <= 1600)
        .unwrap_or("")
        .to_string();
    let w = payload.width.unwrap_or(0);
    let h = payload.height.unwrap_or(0);
    if payload.media_kind == "image" {
        let data = ImageData {
            u: url,
            k: key_b64,
            n: nonce_b64,
            m: payload.mime.clone(),
            name: payload.file_name.clone(),
            size: payload.bytes.len() as u64,
            sha: payload.sha256_plain.clone(),
            scope: scope.to_string(),
            blur,
            w,
            h,
        };
        return format!("img:{}", serde_json::to_string(&data).unwrap_or_default());
    }
    let prefix = match payload.media_kind.as_str() {
        "audio" => "audio:",
        "video" => "video:",
        _ => "file:",
    };
    let data = MediaFileData {
        u: url,
        k: key_b64,
        n: nonce_b64,
        m: payload.mime.clone(),
        name: payload.file_name.clone(),
        size: payload.bytes.len() as u64,
        sha: payload.sha256_plain.clone(),
        dur_ms: payload.duration_ms.unwrap_or(0),
        scope: scope.to_string(),
        blur,
        w,
        h,
        waveform: payload.waveform.clone(),
    };
    format!("{prefix}{}", serde_json::to_string(&data).unwrap_or_default())
}

fn valid_image_url(u: &str) -> bool {
    (u.starts_with("/api/attachments/") || u.starts_with("/uploads/media/")) && !u.contains("..") && !u.contains(char::is_whitespace)
}

/// Short preview text for lists / notifications.
pub fn rich_preview(r: &Rich) -> String {
    match r {
        Rich::Poll(p) => format!("Votes: {}", p.q),
        Rich::Vote { .. } => "Voted".into(),
        Rich::Note(n) => format!("Note: {}", n.t),
        Rich::Reminder(r) => format!("Appointment reminder: {}", r.t),
        Rich::Table(_) => "Spreadsheet".into(),
        Rich::Image(_) => "Image".into(),
        Rich::File(f) => format!("File: {}", if f.name.is_empty() { "Attached" } else { &f.name }),
        Rich::Audio(_) => "Voice messages".into(),
        Rich::Video(_) => "Video".into(),
        Rich::Invite(i) => format!("Invitation to join {}", i.name),
        Rich::Embed(w) => {
            if !w.content.trim().is_empty() {
                w.content.clone()
            } else if let Some(first) = w.embeds.first() {
                first
                    .title
                    .clone()
                    .or_else(|| first.description.clone())
                    .unwrap_or_else(|| format!("Webhook: {}", w.username))
            } else {
                format!("Webhook: {}", w.username)
            }
        }
    }
}

pub fn reminder_due(r: &ReminderData) -> Option<DateTime<Local>> {
    let naive = NaiveDateTime::parse_from_str(&r.at, "%Y-%m-%dT%H:%M").ok()?;
    Local.from_local_datetime(&naive).single()
}

/// Votes per poll: poll_id -> (voter -> option index). The latest vote wins.
pub type VoteTally = std::collections::HashMap<String, std::collections::HashMap<String, usize>>;

/// Image message (SharedWorker Priority DownloadManager -> `blob:` URL ONLY, 0 raw URLs in DOM,
/// local OPFS/IndexedDB cache first, transparent E2EE decrypt, and two-way peer recovery).
fn parse_blur_data_uri(raw: &str) -> Option<(&str, &str)> {
    if raw.len() > 1600 || !raw.starts_with("data:image/") {
        return None;
    }
    let rest = raw.strip_prefix("data:")?;
    let (mime, b64) = rest.split_once(";base64,")?;
    if !mime.starts_with("image/") || b64.is_empty() {
        return None;
    }
    Some((mime, b64))
}

#[component]
pub fn RichImage(data: ImageData) -> Element {
    let init_key = (data.u.clone(), data.k.clone(), data.n.clone());
    let init_cached = platform::get_cached_blob_url_sync(&data.u, &data.k, &data.n);
    let mut src = use_signal(move || init_cached);
    let mut loaded_key = use_signal(move || {
        if src.peek().is_some() { Some(init_key) } else { None }
    });
    let mut missing = use_signal(|| Option::<(String, String, String)>::None);
    let mut rec_status = use_signal(|| Option::<String>::None);
    let mut failed = use_signal(|| false);
    let mut open = use_signal(|| false);

    let key_now = (data.u.clone(), data.k.clone(), data.n.clone());
    let mime_now = data.m.clone();
    let scope_now = data.scope.clone();

    use_effect(use_reactive!(|key_now| {
        if loaded_key.peek().as_ref() == Some(&key_now) && src.peek().is_some() {
            return;
        }
        let (u, k, n) = key_now.clone();
        if let Some(hit) = platform::get_cached_blob_url_sync(&u, &k, &n) {
            loaded_key.set(Some(key_now));
            missing.set(None);
            failed.set(false);
            src.set(Some(hit));
            return;
        }
        src.set(None);
        missing.set(None);
        failed.set(false);
        let m_in = mime_now.clone();
        let sc_in = scope_now.clone();
        let target_key = key_now.clone();
        spawn(async move {
            let mime = if m_in.starts_with("image/") { m_in } else { "image/png".into() };
            // 1. Primary path for BOTH E2EE & non-E2EE images: SharedWorker Priority DownloadManager +
            //    deduplicated LRU `blobPool` (1 single `blob:` URL, 0 raw `/uploads/` or duplicate loads!).
            if let Some(burl) = platform::resolve_e2ee_blob_media_url(&u, &k, &n, &mime, "HIGH").await {
                loaded_key.set(Some(target_key));
                src.set(Some(burl));
                return;
            }
            // 2. Fallback when server copy expired / requires local IndexedDB or WebRTC P2P recovery.
            let scope = if sc_in.is_empty() {
                if k.is_empty() { "group" } else { "dm" }
            } else {
                sc_in.as_str()
            };
            match recovery::load_or_fetch_media(&u, &k, &n, &mime, scope, "original").await {
                MediaFetchOutcome::Ready { bytes, mime: m, .. } => {
                    let burl = platform::register_cached_blob_url(&u, &k, &n, &bytes, &m).await;
                    loaded_key.set(Some(target_key));
                    src.set(Some(burl));
                }
                MediaFetchOutcome::MissingNeedsRecovery { ref_id, availability_state, reason } => {
                    missing.set(Some((ref_id, availability_state, reason)));
                }
                MediaFetchOutcome::Error(_) => failed.set(true),
            }
        });
    }));

    let blur_parts = parse_blur_data_uri(&data.blur);
    let stage_style = if data.w > 0 && data.h > 0 {
        format!("aspect-ratio: {} / {};", data.w, data.h)
    } else {
        String::new()
    };

    rsx! {
        div { class: "rich-image",
            if let Some(s) = src() {
                button { r#type: "button", class: "rich-image-btn rich-image-stage loaded", style: "{stage_style}", aria_label: "See photos", onclick: move |_| open.set(true),
                    img { class: "rich-image-full", src: "{s}", alt: "Image", decoding: "async" }
                }
                if open() {
                    div {
                        class: "lightbox",
                        role: "dialog",
                        aria_modal: "true",
                        aria_label: "Photo",
                        tabindex: "-1",
                        onmounted: move |ev| {
                            spawn(async move {
                                let _ = ev.set_focus(true).await;
                            });
                        },
                        onkeydown: move |e| {
                            if e.key() == Key::Escape {
                                open.set(false);
                            }
                        },
                        onclick: move |_| open.set(false),
                        img { src: "{s}", alt: "Image", onclick: move |e| e.stop_propagation() }
                        div { class: "lightbox-actions", onclick: move |e| e.stop_propagation(),
                            a { href: "{s}", download: if data.name.is_empty() { "chatw-image" } else { data.name.as_str() }, class: "btn-secondary", DownloadIcon { size: 15 } "Download the original" }
                            button { r#type: "button", class: "icon-btn lightbox-close", aria_label: "Close", onclick: move |_| open.set(false), CloseIcon {} }
                        }
                    }
                }
            } else if let Some((rid, avail, reason)) = missing() {
                div { class: "rich-card media-recovery-card",
                    div { class: "rich-kind", "{availability_badge_label(&avail)}" }
                    div { class: "rich-body muted", "{reason}" }
                    if let Some(st) = rec_status() {
                        div { class: "form-hint", "{st}" }
                    }
                    if avail != "no_remaining_copies" && avail != "denied_by_peer" && avail != "deleted_for_everyone" {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| {
                                let id = rid.clone();
                                spawn(async move {
                                    rec_status.set(Some("Sending restore request".into()));
                                    if let Ok(res) = storage_sync::create_recovery_request(&id, Some("webrtc_p2p")).await {
                                        let st = res.get("status").and_then(|s| s.as_str()).unwrap_or("pending");
                                        if let Some(req_id) = res.get("request_id").and_then(|s| s.as_str()) {
                                            if st == "pending" {
                                                recovery::initiate_p2p_recovery_offer(req_id).await;
                                            }
                                        }
                                        rec_status.set(Some(availability_badge_label(st).to_string()));
                                    }
                                });
                            },
                            RefreshIcon { size: 14 } "Recover from friends"
                        }
                    }
                }
            } else if failed() {
                div { class: "rich-image-error", "Unable to download or decode images" }
            } else if let Some((bm, bb)) = blur_parts {
                div { class: "rich-image-stage loading", style: "{stage_style}",
                    canvas {
                        class: "rich-image-blur",
                        width: "16",
                        height: "16",
                        "data-blur-mime": "{bm}",
                        "data-blur-b64": "{bb}",
                        aria_hidden: "true",
                    }
                    span { class: "rich-image-blur-spinner", span { class: "spinner" } }
                }
            } else {
                div { class: "rich-image-loading", span { class: "spinner" } }
            }
        }
    }
}

/// File, Voice Note (Opus) and Video message renderer with Service Worker protected HTTP 206
/// Range streaming (non-E2EE & E2EE `CWCH2` chunked AEAD), automatic tab-bound grant revocation,
/// blurred poster preview, playback speed control, and clean watermark-free original download.
#[component]
pub fn RichMediaAttachment(data: MediaFileData, kind: String) -> Element {
    let mut data_url = use_signal(|| Option::<String>::None);
    let mut sw_grant_id = use_signal(|| Option::<String>::None);
    let mut avail_note = use_signal(|| Option::<(String, String, String)>::None);
    let mut rec_status = use_signal(|| Option::<String>::None);
    let mut playback_rate = use_signal(|| "1".to_string());
    let player_id = use_signal(|| format!("cw-media-{}", uuid::Uuid::new_v4().simple()));
    let d = data.clone();
    let knd = kind.clone();

    use_drop(move || {
        if let Some(gid) = sw_grant_id.peek().as_deref() {
            platform::revoke_sw_media_stream(gid);
        }
        if let Some(u) = data_url.peek().as_deref() {
            platform::revoke_object_url(u);
        }
    });

    use_effect(use_reactive!(|d, knd| {
        if let Some(prev_gid) = sw_grant_id.peek().as_deref() {
            platform::revoke_sw_media_stream(prev_gid);
        }
        sw_grant_id.set(None);
        if let Some(prev) = data_url.peek().as_deref() {
            platform::revoke_object_url(prev);
        }
        data_url.set(None);
        avail_note.set(None);
        spawn(async move {
            let fallback_mime = if !d.m.is_empty() {
                d.m.clone()
            } else if knd == "audio" {
                "audio/webm".into()
            } else if knd == "video" {
                "video/mp4".into()
            } else {
                "application/octet-stream".into()
            };

            // Video & Audio on Web: prefer Service Worker protected chunked HTTP 206 streaming
            // (`/stream/media/{grantId}`) — streams both non-E2EE Range and E2EE `CWCH2` chunks
            // on demand with strict per-tab 403 anti-direct-access enforcement!
            if (knd == "video" || knd == "audio")
                && let Some((gid, stream_url)) = platform::register_sw_media_stream(
                    &d.u,
                    &d.k,
                    &d.n,
                    &fallback_mime,
                    &d.name,
                )
                .await
            {
                sw_grant_id.set(Some(gid));
                data_url.set(Some(stream_url));
                return;
            }

            // Fallback when SW not controlling yet: non-E2EE streams direct HTTP Range
            if d.k.is_empty() && (knd == "video" || knd == "audio") {
                data_url.set(Some(platform::resolve_asset_url(&d.u)));
                return;
            }
            let scope = if d.scope.is_empty() {
                if d.k.is_empty() { "group" } else { "dm" }
            } else {
                d.scope.as_str()
            };
            match recovery::load_or_fetch_media(&d.u, &d.k, &d.n, &fallback_mime, scope, "original").await {
                MediaFetchOutcome::Ready { bytes, mime, .. } => {
                    data_url.set(Some(platform::media_bytes_to_object_url(&bytes, &mime).await));
                }
                MediaFetchOutcome::MissingNeedsRecovery { ref_id, availability_state, reason } => {
                    avail_note.set(Some((ref_id, availability_state, reason)));
                }
                MediaFetchOutcome::Error(_) => {
                    if d.k.is_empty() {
                        data_url.set(Some(platform::resolve_asset_url(&d.u)));
                    }
                }
            }
        });
    }));

    let fname = if data.name.is_empty() {
        match kind.as_str() {
            "audio" => "Voice messages".to_string(),
            "video" => "Video".to_string(),
            _ => "Attachments".to_string(),
        }
    } else {
        data.name.clone()
    };

    let pid = player_id();

    rsx! {
        div { class: "rich-card media-attachment-card",
            div { class: "rich-kind",
                if kind == "audio" { "Voice messages" } else if kind == "video" { "Video" } else { "Attachments" }
                if !data.k.is_empty() { " · E2EE" }
                if data.size > 0 { " · {format_bytes_human(data.size)}" }
            }
            div { class: "rich-title", "{fname}" }
            if let Some(u) = data_url() {
                if kind == "audio" {
                    {
                        let bars: Vec<u8> = if !data.waveform.is_empty() {
                            data.waveform.iter().copied().take(48).collect()
                        } else {
                            // Deterministic subtle waveform preview for legacy voice notes without embedded samples
                            let seed = data.sha.as_bytes().iter().fold(17u32, |a, &b| a.wrapping_mul(31).wrapping_add(b as u32));
                            (0..36u32).map(|i| (((seed.wrapping_mul(i + 7) ^ (i * 13)) % 24) + 6) as u8).collect()
                        };
                        let total_bars = bars.len().max(1);
                        rsx! {
                            div { class: "voice-waveform", role: "slider", aria_label: "Seek voice message",
                                for (idx, amp) in bars.into_iter().enumerate() {
                                    {
                                        let h_pct = ((amp as f32 / 31.0) * 100.0).clamp(16.0, 100.0);
                                        let el_id = pid.clone();
                                        let ratio = (idx as f64 + 0.5) / (total_bars as f64);
                                        rsx! {
                                            button {
                                                key: "{idx}",
                                                r#type: "button",
                                                class: "voice-wave-bar",
                                                style: "height: {h_pct:.0}%;",
                                                aria_label: "Seek voice position",
                                                onclick: move |_| {
                                                    let id_json = serde_json::to_string(&el_id).unwrap_or_default();
                                                    let _ = dioxus::document::eval(&format!(
                                                        "(() => {{ const el = document.getElementById({id_json}); if (el && Number.isFinite(el.duration) && el.duration > 0) {{ el.currentTime = el.duration * {ratio}; el.play().catch(()=>{{}}); }} }})()"
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    audio {
                        id: "{pid}",
                        controls: true,
                        preload: "metadata",
                        src: "{u}",
                        style: "width: 100%; max-width: 280px;",
                    }
                } else if kind == "video" {
                    div { class: "rich-video-stage",
                        video {
                            id: "{pid}",
                            controls: true,
                            preload: "metadata",
                            playsinline: true,
                            src: "{u}",
                            style: "width: 100%; max-width: 340px; border-radius: 8px; background: #000;",
                        }
                    }
                }
                div { style: "margin-top: 6px; display: flex; flex-wrap: wrap; align-items: center; gap: 6px;",
                    if u.starts_with("/stream/") {
                        {
                            let dl_url = u.clone();
                            let dl_name = fname.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    style: "display: inline-flex; gap: 6px;",
                                    onclick: move |_| {
                                        let u_json = serde_json::to_string(&dl_url).unwrap_or_default();
                                        let n_json = serde_json::to_string(&dl_name).unwrap_or_default();
                                        let _ = dioxus::document::eval(&format!(
                                            r#"(async () => {{
                                                try {{
                                                    const res = await fetch({u_json}, {{ credentials: 'include' }});
                                                    if (!res.ok) return;
                                                    const blob = await res.blob();
                                                    const ou = URL.createObjectURL(blob);
                                                    const a = document.createElement('a');
                                                    a.href = ou;
                                                    a.download = {n_json};
                                                    document.body.appendChild(a);
                                                    a.click();
                                                    setTimeout(() => {{ a.remove(); URL.revokeObjectURL(ou); }}, 15000);
                                                }} catch (_) {{}}
                                            }})();"#
                                        ));
                                    },
                                    DownloadIcon { size: 14 } "Download the original"
                                }
                            }
                        }
                    } else {
                        a { href: "{u}", download: "{fname}", class: "btn-secondary", style: "display: inline-flex; gap: 6px;",
                            DownloadIcon { size: 14 } "Download the original"
                        }
                    }
                    if kind == "video" || kind == "audio" {
                        select {
                            class: "settings-select",
                            aria_label: "Playback speed",
                            style: "width: auto; min-width: 66px; padding: 4px 8px; font-size: 12px;",
                            value: "{playback_rate}",
                            onchange: {
                                let el_id = pid.clone();
                                move |e| {
                                    let rate = e.value();
                                    playback_rate.set(rate.clone());
                                    if let Ok(r) = rate.parse::<f64>() {
                                        let id_json = serde_json::to_string(&el_id).unwrap_or_default();
                                        let _ = dioxus::document::eval(&format!(
                                            "(() => {{ const el = document.getElementById({id_json}); if (el) el.playbackRate = {r}; }})()"
                                        ));
                                    }
                                }
                            },
                            option { value: "0.75", "0.75x" }
                            option { value: "1", "1x" }
                            option { value: "1.25", "1.25x" }
                            option { value: "1.5", "1.5x" }
                            option { value: "2", "2x" }
                        }
                    }
                }
            } else if let Some((rid, avail, reason)) = avail_note() {
                div { class: "rich-body muted", "{availability_badge_label(&avail)} · {reason}" }
                if let Some(st) = rec_status() {
                    div { class: "form-hint", "{st}" }
                }
                if avail != "no_remaining_copies" && avail != "denied_by_peer" && avail != "deleted_for_everyone" {
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: move |_| {
                            let id = rid.clone();
                            spawn(async move {
                                if let Ok(res) = storage_sync::create_recovery_request(&id, Some("webrtc_p2p")).await {
                                    let st = res.get("status").and_then(|s| s.as_str()).unwrap_or("pending");
                                    if let Some(req_id) = res.get("request_id").and_then(|s| s.as_str()) {
                                        if st == "pending" {
                                            recovery::initiate_p2p_recovery_offer(req_id).await;
                                        }
                                    }
                                    rec_status.set(Some(availability_badge_label(st).to_string()));
                                }
                            });
                        },
                        RefreshIcon { size: 14 } "Restore files"
                    }
                }
            } else {
                span { class: "spinner" }
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
            let total_votes = votes.len();
            let total_denom = total_votes.max(1);
            let mine = votes.get(&my_id).copied();
            let foot = if total_votes == 0 {
                "There are no votes yet".to_string()
            } else {
                format!("{total_votes} votes")
            };
            rsx! {
                div { class: "rich-card poll-card",
                    div { class: "rich-kind", BarChartIcon { size: 14 } "Poll" }
                    div { class: "rich-title", "{p.q}" }
                    div { class: "poll-options",
                        for (i, opt) in p.o.iter().enumerate() {
                            {
                                let count = votes.values().filter(|v| **v == i).count();
                                let pct = if total_votes > 0 { count * 100 / total_denom } else { 0 };
                                let chosen = mine == Some(i);
                                let pid = p.id.clone();
                                rsx! {
                                    button {
                                        key: "{i}",
                                        r#type: "button",
                                        class: if chosen { "poll-option chosen" } else { "poll-option" },
                                        aria_pressed: "{chosen}",
                                        disabled: on_vote.is_none(),
                                        onclick: move |_| if let Some(h) = on_vote { h.call((pid.clone(), i)) },
                                        span { class: "poll-bar", style: "width: {pct}%;" }
                                        span { class: "poll-check", aria_hidden: "true",
                                            if chosen { CheckIcon { size: 11 } }
                                        }
                                        span { class: "poll-label", "{opt}" }
                                        span { class: "poll-stats",
                                            if total_votes > 0 {
                                                span { class: "poll-pct", "{pct}%" }
                                            }
                                            span { class: "poll-count", "{count}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "rich-foot", "{foot}" }
                }
            }
        }
        Rich::Note(n) => rsx! {
            div { class: "rich-card note-card",
                div { class: "rich-kind", FileTextIcon { size: 14 } "Notes" }
                div { class: "rich-title", "{n.t}" }
                if !n.b.is_empty() { div { class: "rich-body", "{n.b}" } }
            }
        },
        Rich::Reminder(r) => {
            let when = reminder_due(&r).map(|d| d.format("%H:%M, %d/%m/%Y").to_string()).unwrap_or_else(|| r.at.clone());
            let past = reminder_due(&r).map(|d| d < Local::now()).unwrap_or(false);
            rsx! {
                div { class: "rich-card reminder-card",
                    div { class: "rich-kind", BellIcon { size: 14 } "Appointment reminder" }
                    div { class: "rich-title", "{r.t}" }
                    div { class: if past { "rich-body muted" } else { "rich-body" }, "{when}" if past { " · has passed" } }
                }
            }
        }
        Rich::Table(rows) => rsx! {
            div { class: "rich-card table-card",
                div { class: "rich-kind", "Spreadsheet · {rows.len()} rows" }
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
        Rich::File(f) => rsx! { RichMediaAttachment { data: f, kind: "file".to_string() } },
        Rich::Audio(a) => rsx! { RichMediaAttachment { data: a, kind: "audio".to_string() } },
        Rich::Video(v) => rsx! { RichMediaAttachment { data: v, kind: "video".to_string() } },
        Rich::Invite(i) => {
            let kind_label = if i.kind == "server" { "Communities" } else { "Groups" };
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
                            div { class: "rich-kind", "Invitation to {kind_label}" }
                            div { class: "rich-title", "{i.name}" }
                            div { class: "rich-body muted", "{i.members} members" if !i.from.is_empty() { " · invited by {i.from}" } }
                        }
                    }
                    button { r#type: "button", class: "btn-primary btn-block", disabled: on_open_invite.is_none(),
                        onclick: move |_| if let Some(h) = on_open_invite { h.call((k.clone(), c.clone())) },
                        "Watch and participate"
                    }
                }
            }
        }
        Rich::Embed(w) => {
            rsx! {
                div { class: "webhook-msg-wrap",
                    div { class: "webhook-msg-author",
                        if let Some(av) = w.avatar_url.as_deref().filter(|u| is_safe_ext_url(u) || platform::safe_asset_path(u)) {
                            img { class: "webhook-msg-avatar", src: "{av}", alt: "{w.username}" }
                        }
                        span { class: "webhook-msg-name", "{w.username}" }
                        span { class: "webhook-badge", "WEBHOOK" }
                    }
                    if !w.content.trim().is_empty() {
                        div { class: "webhook-msg-text", "{w.content}" }
                    }
                    for (ei, emb) in w.embeds.iter().enumerate() {
                        {
                            let color_hex = emb.color.map(|c| format!("#{:06x}", c & 0xffffff)).unwrap_or_else(|| "var(--accent)".into());
                            let bar_style = format!("border-left-color: {color_hex};");
                            rsx! {
                                div { key: "{ei}", class: "discord-embed-card", style: "{bar_style}",
                                    div { class: "discord-embed-main",
                                        if let Some(author) = &emb.author {
                                            div { class: "discord-embed-author",
                                                if let Some(ic) = author.icon_url.as_deref().filter(|u| is_safe_ext_url(u)) {
                                                    img { class: "discord-embed-author-icon", src: "{ic}", alt: "{author.name}" }
                                                }
                                                if let Some(au) = author.url.as_deref().filter(|u| is_safe_ext_url(u)) {
                                                    a { href: "{au}", target: "_blank", rel: "noopener noreferrer", "{author.name}" }
                                                } else {
                                                    span { "{author.name}" }
                                                }
                                            }
                                        }
                                        if let Some(title) = emb.title.as_deref().filter(|s| !s.trim().is_empty()) {
                                            if let Some(u) = emb.url.as_deref().filter(|u| is_safe_ext_url(u)) {
                                                a { class: "discord-embed-title", href: "{u}", target: "_blank", rel: "noopener noreferrer", "{title}" }
                                            } else {
                                                div { class: "discord-embed-title", "{title}" }
                                            }
                                        }
                                        if let Some(desc) = emb.description.as_deref().filter(|s| !s.trim().is_empty()) {
                                            div { class: "discord-embed-desc", "{desc}" }
                                        }
                                        if !emb.fields.is_empty() {
                                            div { class: "discord-embed-fields",
                                                for (fi, f) in emb.fields.iter().enumerate() {
                                                    div {
                                                        key: "{fi}",
                                                        class: if f.inline { "discord-embed-field inline" } else { "discord-embed-field" },
                                                        div { class: "discord-embed-field-name", "{f.name}" }
                                                        div { class: "discord-embed-field-val", "{f.value}" }
                                                    }
                                                }
                                            }
                                        }
                                        if let Some(img) = emb.image.as_ref().filter(|m| is_safe_ext_url(&m.url)) {
                                            img { class: "discord-embed-image", src: "{img.url}", alt: "Embed image", loading: "lazy" }
                                        }
                                        if emb.footer.is_some() || emb.timestamp.is_some() {
                                            div { class: "discord-embed-footer",
                                                if let Some(ft) = &emb.footer {
                                                    if let Some(ic) = ft.icon_url.as_deref().filter(|u| is_safe_ext_url(u)) {
                                                        img { class: "discord-embed-footer-icon", src: "{ic}", alt: "" }
                                                    }
                                                    span { "{ft.text}" }
                                                }
                                                if let Some(ts) = emb.timestamp.as_deref().filter(|s| !s.is_empty()) {
                                                    span { class: "discord-embed-ts", " · {ts}" }
                                                }
                                            }
                                        }
                                    }
                                    if let Some(th) = emb.thumbnail.as_ref().filter(|m| is_safe_ext_url(&m.url)) {
                                        img { class: "discord-embed-thumb", src: "{th.url}", alt: "Thumbnail", loading: "lazy" }
                                    }
                                }
                            }
                        }
                    }
                    if !w.components.is_empty() {
                        div { class: "webhook-components-row",
                            for (bi, btn) in w.components.iter().enumerate() {
                                if let Some(u) = btn.url.as_deref().filter(|u| is_safe_ext_url(u)) {
                                    a { key: "{bi}", class: "btn-secondary webhook-comp-btn", href: "{u}", target: "_blank", rel: "noopener noreferrer", "{btn.label}" }
                                } else {
                                    span { key: "{bi}", class: "btn-secondary webhook-comp-btn", "{btn.label}" }
                                }
                            }
                        }
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
    #[props(default)] on_media: Option<EventHandler<OptimizedMediaPayload>>,
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
    let mut preserve_original = use_signal(|| false);
    let mut recording = use_signal(|| false);
    let img_id = format!("{input_id}-image");
    let img_id2 = img_id.clone();
    let doc_id = format!("{input_id}-file");
    let doc_id2 = doc_id.clone();
    rsx! {
        div { class: "composer-tools",
            if recording() {
                button {
                    r#type: "button",
                    class: "icon-btn active",
                    title: "Stop and send voice recording",
                    aria_label: "Stop recording",
                    onclick: move |_| {
                        recording.set(false);
                        spawn(async move {
                            if let Ok(payload) = media_opt::stop_voice_recording().await {
                                if let Some(h) = on_media {
                                    h.call(payload);
                                }
                            }
                        });
                    },
                    MicIcon { size: 24 }
                }
            }
            button {
                r#type: "button",
                class: if open() { "icon-btn active" } else { "icon-btn" },
                title: "Add",
                aria_label: "Add photos, videos, files, record audio, vote",
                aria_expanded: "{open()}",
                onclick: move |e| { e.stop_propagation(); open.set(!open()); },
                PlusIcon { size: 24 }
            }
            if open() {
                button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Close", onclick: move |_| open.set(false) }
                div { class: "account-menu tools-menu", role: "menu",
                    label { class: "menu-item", role: "menuitem",
                        input {
                            id: "{img_id}",
                            r#type: "file",
                            accept: "image/*,video/mp4,video/webm",
                            style: "display: none;",
                            onchange: move |_| {
                                open.set(false);
                                let fid = img_id2.clone();
                                let orig = preserve_original();
                                spawn(async move {
                                    if let Ok(payload) = media_opt::pick_and_optimize_from_input(&fid, orig).await {
                                        if let Some(h) = on_media {
                                            h.call(payload);
                                        } else {
                                            on_image.call((payload.bytes, payload.mime));
                                        }
                                    } else if let Ok((bytes, mime)) = platform::read_selected_file_raw(&fid).await {
                                        on_image.call((bytes, mime));
                                    }
                                });
                            },
                        }
                        "Photos and videos"
                    }
                    if on_media.is_some() {
                        label { class: "menu-item", role: "menuitem",
                            input {
                                id: "{doc_id}",
                                r#type: "file",
                                style: "display: none;",
                                onchange: move |_| {
                                    open.set(false);
                                    let fid = doc_id2.clone();
                                    spawn(async move {
                                        if let Ok(payload) = media_opt::pick_and_optimize_from_input(&fid, true).await {
                                            if let Some(h) = on_media {
                                                h.call(payload);
                                            }
                                        }
                                    });
                                },
                            }
                            "Attachments"
                        }
                        button {
                            r#type: "button",
                            class: "menu-item",
                            role: "menuitem",
                            onclick: move |_| {
                                open.set(false);
                                spawn(async move {
                                    if media_opt::start_voice_recording().await.is_ok() {
                                        recording.set(true);
                                    }
                                });
                            },
                            "Voice recording"
                        }
                    }
                    label { class: "menu-item", role: "menuitemcheckbox",
                        onclick: move |e| e.stop_propagation(),
                        input {
                            r#type: "checkbox",
                            checked: preserve_original(),
                            onchange: move |e| preserve_original.set(e.checked()),
                        }
                        "Keep the original"
                    }
                    if allow_poll { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("poll")); }, "Poll" } }
                    if allow_note { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("note")); }, "Notes" } }
                    if allow_reminder { button { r#type: "button", class: "menu-item", role: "menuitem", onclick: move |_| { open.set(false); dialog.set(Some("reminder")); }, "Appointment reminder" } }
                }
            }
            if let Some(kind) = dialog() {
                div { class: "modal-overlay", onclick: move |_| dialog.set(None),
                    div { class: "modal-dialog", role: "dialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                        onkeydown: move |e| if e.key() == Key::Escape { dialog.set(None) },
                        div { class: "modal-header",
                            h2 { class: "modal-title",
                                match kind { "poll" => "Create poll", "note" => "Create note", _ => "Create reminder" }
                            }
                            button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| dialog.set(None), CloseIcon {} }
                        }
                        div { class: "modal-body",
                            if kind == "poll" {
                                label { class: "form-field", span { class: "form-label", "Question" }
                                    input { maxlength: "200", value: "{q}", oninput: move |e| q.set(e.value()), placeholder: "Where to go for lunch?" }
                                }
                                div { class: "form-field",
                                    span { class: "form-label", "Choose" }
                                    for (i, o) in opts().iter().enumerate() {
                                        div { key: "{i}", class: "poll-edit-row",
                                            input { maxlength: "80", value: "{o}", placeholder: "Option {i + 1}",
                                                oninput: move |e| { let v = e.value(); opts.with_mut(|l| l[i] = v); } }
                                            if opts.read().len() > 2 {
                                                button { r#type: "button", class: "icon-btn", aria_label: "Delete selection", onclick: move |_| { opts.with_mut(|l| { l.remove(i); }); }, CloseIcon { size: 16 } }
                                            }
                                        }
                                    }
                                    if opts.read().len() < 10 {
                                        button { r#type: "button", class: "btn-ghost", style: "align-self: flex-start;", onclick: move |_| opts.with_mut(|l| l.push(String::new())), PlusIcon { size: 16 } "More options" }
                                    }
                                }
                            } else if kind == "note" {
                                label { class: "form-field", span { class: "form-label", "Title" } input { maxlength: "120", value: "{title}", oninput: move |e| title.set(e.value()) } }
                                label { class: "form-field", span { class: "form-label", "Content" } textarea { rows: "5", maxlength: "2000", value: "{body}", oninput: move |e| body.set(e.value()) } }
                            } else {
                                label { class: "form-field", span { class: "form-label", "Prompt content" } input { maxlength: "120", value: "{title}", oninput: move |e| title.set(e.value()), placeholder: "Meetings" } }
                                label { class: "form-field", span { class: "form-label", "Time" } input { r#type: "datetime-local", value: "{at}", oninput: move |e| at.set(e.value()) } }
                                div { class: "form-hint", "ChatW will notify everyone in the chat when it's time." }
                            }
                        }
                        div { class: "modal-footer",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| dialog.set(None), "Cancel" }
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
                                "Send"
                            }
                        }
                    }
                }
            }
        }
    }
}
