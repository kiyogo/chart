//! Rich message types carried as text (inside the E2EE envelope for direct messages):
//!
//! * `poll:{json}`      — question + options; votes are `vote:{poll_id}:{index}` messages
//! * `note:{json}`      — a titled note
//! * `reminder:{json}`  — reminder with a local time; the app notifies when due
//! * `table:<tsv>`      — spreadsheet data pasted as a table
//! * `img:{json}`       — image; E2EE images carry the AES key, group images a plain URL
//! * `invite:{json}`    — a rich invite card for a group / community

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::{
    components::{
        avatar::Avatar,
        icons::{
            BarChartIcon, BellIcon, CheckIcon, CloseIcon, DownloadIcon, DownloadMinimalIcon,
            EnhanceToggleIcon, FileTextIcon, FocusIcon, MicIcon, PlusIcon, RefreshIcon, RotateCwIcon,
            ZoomInIcon, ZoomOutIcon,
        },
    },
    platform,
    services::storage_sync,
    storage::{
        availability_badge_label, format_bytes_human,
        media_opt::{self, OptimizedMediaPayload},
        recovery::{self, MediaFetchOutcome},
    },
};

static SESSION_START_MS: OnceLock<i64> = OnceLock::new();

/// Returns true when a received message arrived live during the current session (not historical messages loaded on a new device).
pub fn is_message_live_incoming(created_at: &str) -> bool {
    let boot_ms = *SESSION_START_MS.get_or_init(|| Utc::now().timestamp_millis());
    DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.timestamp_millis() >= boot_ms - 4_000)
        .unwrap_or(false)
}

/// Viewport-root Lightbox payload so full-screen image previews mount outside
/// `.chat-column` / `.message-row` / `.rich-image` for exact viewport centering (Spec 078).
#[derive(Clone, Debug, PartialEq)]
pub struct LightboxMedia {
    pub src: String,
    pub name: String,
    pub media_id: String,
    pub content_hash: String,
    pub orig_profile_json: String,
    pub ai_restored: bool,
    pub original_src: Option<String>,
    pub is_sender_original: bool,
    /// True when opened from a new-device/history image that hasn't been enhanced locally yet.
    pub auto_enhance_on_open: bool,
    /// True when this image is eligible for on-device Real-ESRGAN 4× enhancement (non-GIF, non-sender-original).
    pub can_ai_enhance: bool,
    /// Wire/display width in pixels (used for deterministic Frame-0 Lightbox sizing).
    pub width: u32,
    /// Wire/display height in pixels (used for deterministic Frame-0 Lightbox sizing).
    pub height: u32,
    /// Pre-compression original width if recorded in attachment metadata (`0` if unknown).
    pub orig_width: u32,
    /// Pre-compression original height if recorded in attachment metadata (`0` if unknown).
    pub orig_height: u32,
}

pub static GLOBAL_LIGHTBOX: GlobalSignal<Option<LightboxMedia>> = GlobalSignal::new(|| None);
/// Broadcast `(media_id, enhanced_blob_url)` when Lightbox finishes on-demand Real-ESRGAN 4× so chat bubbles sync immediately.
pub static GLOBAL_AI_UPDATED: GlobalSignal<Option<(String, String)>> = GlobalSignal::new(|| None);
/// Spec 115: Broadcast user-facing media validation notices (e.g. blocked `.svg` upload) to global toast banner.
pub static GLOBAL_MEDIA_NOTICE: GlobalSignal<Option<String>> = GlobalSignal::new(|| None);

/// Instant Sender Original Preview item shown in the message stream the millisecond a file is chosen,
/// before AVIF compression, E2EE encryption, and S3/CAS upload complete (Spec 110).
#[derive(Clone, Debug, PartialEq)]
pub struct PendingSenderMedia {
    pub handle_id: String,
    pub preview_url: String,
    pub file_name: String,
    pub media_kind: String,
    pub width: u32,
    pub height: u32,
    pub status_label: String,
    pub failed: bool,
}

pub static PENDING_SENDER_MEDIA: GlobalSignal<Vec<PendingSenderMedia>> = GlobalSignal::new(Vec::new);

pub fn push_pending_media(item: PendingSenderMedia) {
    PENDING_SENDER_MEDIA.write().push(item);
}

pub fn update_pending_media_status(preview_url: &str, status_label: &str, failed: bool) {
    let mut list = PENDING_SENDER_MEDIA.write();
    if let Some(it) = list.iter_mut().find(|x| x.preview_url == preview_url || x.handle_id == preview_url) {
        it.status_label = status_label.to_string();
        it.failed = failed;
    }
}

pub fn remove_pending_media(preview_url: &str) {
    let mut list = PENDING_SENDER_MEDIA.write();
    list.retain(|x| x.preview_url != preview_url && x.handle_id != preview_url);
}

#[component]
pub fn PendingSenderMediaList() -> Element {
    let items = PENDING_SENDER_MEDIA();
    if items.is_empty() {
        return rsx! {};
    }
    rsx! {
        for item in items {
            {
                let key_id = item.handle_id.clone();
                let rem_id = item.handle_id.clone();
                let (box_style, stage_style) = if let Some((bw, bh)) = compute_chat_image_box(item.width, item.height) {
                    (
                        format!("width: min({bw}px, 100%);"),
                        format!("aspect-ratio: {bw} / {bh}; position: relative;"),
                    )
                } else {
                    (String::new(), "position: relative;".to_string())
                };
                rsx! {
                    div { key: "{key_id}", class: "message-row self pending-sender-media-row", "data-pending-media": "true",
                        div { class: "message-content",
                            if item.media_kind == "image" && !item.preview_url.is_empty() {
                                div { class: "rich-image", style: "{box_style}",
                                    div { class: "rich-image-stage loaded", style: "{stage_style}",
                                        img {
                                            class: "rich-image-full",
                                            src: "{item.preview_url}",
                                            alt: "{item.file_name}",
                                            "data-sender-original": "true",
                                        }
                                        div {
                                            class: if item.failed { "status-badge error" } else { "chip-privacy" },
                                            style: "position: absolute; bottom: 8px; right: 8px; backdrop-filter: blur(8px);",
                                            "{item.status_label}"
                                            if item.failed {
                                                button {
                                                    r#type: "button",
                                                    class: "btn-secondary",
                                                    style: "margin-left: 6px; padding: 2px 8px; font-size: 11px;",
                                                    onclick: move |_| {
                                                        media_opt::release_pending_handle(&rem_id, true);
                                                        remove_pending_media(&rem_id);
                                                    },
                                                    "Đóng"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "rich-card",
                                    div { class: "rich-title", "{item.file_name}" }
                                    div { class: if item.failed { "rich-body error" } else { "rich-body muted" }, "{item.status_label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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
    #[serde(default)]
    pub orig_size: u64,
    #[serde(default)]
    pub orig_w: u32,
    #[serde(default)]
    pub orig_h: u32,
    /// Compact OriginalQualityProfile JSON extracted on the sender's untouched original image
    /// so the receiver's 2-stage Large CNN + closed-loop calibration restores sharpness/color ~100%.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub qp: String,
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
    let orig_w = payload.target_width.unwrap_or(w);
    let orig_h = payload.target_height.unwrap_or(h);
    let orig_size = payload.original_size as u64;
    let wire_size = if !payload.bytes.is_empty() {
        payload.bytes.len() as u64
    } else {
        payload.optimized_size as u64
    };
    if payload.media_kind == "image" {
        let data = ImageData {
            u: url,
            k: key_b64,
            n: nonce_b64,
            m: payload.mime.clone(),
            name: crate::storage::media_opt::strip_file_extension(&payload.file_name),
            size: wire_size,
            sha: payload.sha256_plain.clone(),
            scope: scope.to_string(),
            blur,
            w,
            h,
            orig_size,
            orig_w,
            orig_h,
            qp: payload.orig_profile_json.clone().unwrap_or_default(),
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
        name: crate::storage::media_opt::strip_file_extension(&payload.file_name),
        size: wire_size,
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
pub fn RichImage(data: ImageData, #[props(default)] is_live_incoming: bool) -> Element {
    let init_cached = platform::get_cached_blob_url_sync(&data.u, &data.k, &data.n);
    let cached_raw = init_cached.clone();
    let src = use_signal(move || init_cached);
    let raw_src = use_signal(move || cached_raw);
    let loaded_key = use_signal(|| Option::<(String, String, String)>::None);
    let missing = use_signal(|| Option::<(String, String, String)>::None);
    let mut rec_status = use_signal(|| Option::<String>::None);
    let failed = use_signal(|| false);
    let ai_enhanced = use_signal(|| false);
    let ai_loading = use_signal(|| false);
    let ai_revealing = use_signal(|| false);
    let ai_progress = use_signal(|| 0u32);
    let ai_cache_hit = use_signal(|| false);
    let needs_on_demand_ai = use_signal(|| false);
    let is_sender_original = use_signal(|| false);

    let key_now = (data.u.clone(), data.k.clone(), data.n.clone());
    let mime_now = data.m.clone();
    let scope_now = data.scope.clone();
    let sha_now = data.sha.clone();
    let qp_now = data.qp.clone();
    let name_now = data.name.clone();
    let wire_w_now = data.w;
    let wire_h_now = data.h;

    let target_w = if data.orig_w > 0 { data.orig_w } else { data.w };
    let target_h = if data.orig_h > 0 { data.orig_h } else { data.h };

    // Sync immediately when Lightbox finishes enhancing this history image on-demand
    let media_u_sync = data.u.clone();
    use_effect(move || {
        if let Some((mid, enh_url)) = GLOBAL_AI_UPDATED() {
            if mid == media_u_sync && !enh_url.is_empty() {
                let mut src = src;
                let mut ai_enhanced = ai_enhanced;
                let mut needs_on_demand_ai = needs_on_demand_ai;
                let mut ai_loading = ai_loading;
                src.set(Some(enh_url));
                ai_enhanced.set(true);
                needs_on_demand_ai.set(false);
                ai_loading.set(false);
            }
        }
    });

    let run_ai_pipeline = move |base_url: String,
                                media_id: String,
                                content_hash: String,
                                mime_type: String,
                                orig_qp: String,
                                auto_run: bool,
                                key_b64_reg: String,
                                nonce_b64_reg: String,
                                fname_reg: String,
                                wire_w_reg: u32,
                                wire_h_reg: u32| {
        if mime_type == "image/gif" {
            return;
        }
        let mut ai_loading = ai_loading;
        let mut ai_revealing = ai_revealing;
        let mut ai_progress = ai_progress;
        let mut ai_enhanced = ai_enhanced;
        let mut ai_cache_hit = ai_cache_hit;
        let mut needs_on_demand_ai = needs_on_demand_ai;
        let mut is_sender_original = is_sender_original;
        let mut src = src;

        spawn(async move {
            let url_json = serde_json::to_string(&base_url).unwrap_or_default();
            let mid_json = serde_json::to_string(&media_id).unwrap_or_default();
            let hash_json = serde_json::to_string(&content_hash).unwrap_or_default();
            let qp_json = serde_json::to_string(&orig_qp).unwrap_or_else(|_| "\"\"".to_string());
            let key_json = serde_json::to_string(&key_b64_reg).unwrap_or_else(|_| "\"\"".to_string());
            let nonce_json = serde_json::to_string(&nonce_b64_reg).unwrap_or_else(|_| "\"\"".to_string());
            let fname_json = serde_json::to_string(&fname_reg).unwrap_or_else(|_| "\"image\"".to_string());
            let mime_json = serde_json::to_string(&mime_type).unwrap_or_else(|_| "\"image/avif\"".to_string());
            let auto_flag = if auto_run { "true" } else { "false" };
            let js = format!(
                r#"
                (async () => {{
                    // 0. Register this loaded image (even if blurry/un-enhanced!) in local cross-chat PDQ/BLAKE3 dedup index
                    // so copying/pasting it into another chat NEVER creates a duplicate file on the server!
                    const regDedup = (isSenderOrigLocal) => {{
                        if (window.__chatwDedupLocal && window.__chatwDedupLocal.registerLoadedImageFromBlobUrl) {{
                            window.__chatwDedupLocal.registerLoadedImageFromBlobUrl({url_json}, {{
                                url: {mid_json},
                                keyB64: {key_json},
                                nonceB64: {nonce_json},
                                mime: {mime_json},
                                fileName: {fname_json},
                                sha256: {hash_json},
                                width: {wire_w_reg},
                                height: {wire_h_reg},
                                origW: {target_w},
                                origH: {target_h},
                                qp: {qp_json},
                                isAiRecovery: Boolean({qp_json}) || ({target_w} > {wire_w_reg}),
                                isSenderOrigLocal: Boolean(isSenderOrigLocal),
                            }}).catch(() => {{}});
                        }}
                    }};
                    // 1. Sender keeps 100% untouched local original -> show immediately with zero AI overlay
                    if (window.__chatwMediaOpt && window.__chatwMediaOpt.getSenderOriginalFromCache) {{
                        const orig = await window.__chatwMediaOpt.getSenderOriginalFromCache({mid_json});
                        if (orig && orig.found && orig.url) {{
                            regDedup(true);
                            dioxus.send({{ type: 'sender_orig', url: orig.url }});
                            return;
                        }}
                    }}
                    regDedup(false);
                    if (!window.__chatwAiRestorer || !window.__chatwAiRestorer.isAiEnabled()) {{
                        dioxus.send({{ type: 'noop' }});
                        return;
                    }}
                    // 2. Check persistent IndexedDB AI Output Cache first (< 3ms)
                    const cached = await window.__chatwAiRestorer.restoreImageFromUrl({url_json}, {{
                        mediaId: {mid_json},
                        contentHash: {hash_json},
                        targetWidth: {target_w},
                        targetHeight: {target_h},
                        origProfileJson: {qp_json},
                        cacheOnly: true,
                    }});
                    if (cached && cached.enhanced && cached.url) {{
                        dioxus.send({{ type: 'done', url: cached.url, cacheHit: true }});
                        return;
                    }}
                    // 3. New device / history image not cached yet -> show sleek corner SVG icon until user clicks!
                    if (!{auto_flag}) {{
                        dioxus.send({{ type: 'needs_on_demand' }});
                        return;
                    }}
                    // 4. Live incoming image (or user tapped corner icon) -> blurred + AI shimmer + live progress!
                    dioxus.send({{ type: 'start', pct: 3 }});
                    const res = await window.__chatwAiRestorer.restoreImageFromUrl({url_json}, {{
                        mediaId: {mid_json},
                        contentHash: {hash_json},
                        targetWidth: {target_w},
                        targetHeight: {target_h},
                        origProfileJson: {qp_json},
                        forceRun: true,
                        onProgress: (pct) => {{
                            try {{ dioxus.send({{ type: 'progress', pct: Number(pct) || 0 }}); }} catch (_) {{}}
                        }},
                    }});
                    if (res && res.enhanced && res.url) {{
                        dioxus.send({{ type: 'done', url: res.url, cacheHit: Boolean(res.cacheHit) }});
                    }} else {{
                        dioxus.send({{ type: 'failed' }});
                    }}
                }})();
                "#
            );
            let mut eval = dioxus::document::eval(&js);
            while let Ok(v) = eval.recv::<serde_json::Value>().await {
                let ev_type = v.get("type").and_then(|s| s.as_str()).unwrap_or("");
                match ev_type {
                    "sender_orig" => {
                        if let Some(orig_url) = v.get("url").and_then(|s| s.as_str()).filter(|s| !s.is_empty()) {
                            is_sender_original.set(true);
                            ai_enhanced.set(false);
                            needs_on_demand_ai.set(false);
                            ai_loading.set(false);
                            src.set(Some(orig_url.to_string()));
                        }
                        break;
                    }
                    "needs_on_demand" => {
                        ai_loading.set(false);
                        needs_on_demand_ai.set(true);
                        break;
                    }
                    "start" => {
                        needs_on_demand_ai.set(false);
                        ai_revealing.set(false);
                        ai_loading.set(true);
                        ai_progress.set(v.get("pct").and_then(|n| n.as_u64()).unwrap_or(3) as u32);
                    }
                    "progress" => {
                        if let Some(p) = v.get("pct").and_then(|n| n.as_u64()) {
                            let p32 = (p as u32).min(100);
                            if p32 != *ai_progress.peek() {
                                ai_progress.set(p32);
                            }
                        }
                    }
                    "done" => {
                        let was_loading = *ai_loading.peek();
                        ai_progress.set(100);
                        needs_on_demand_ai.set(false);
                        if let Some(new_url) = v.get("url").and_then(|s| s.as_str()).filter(|s| !s.is_empty()) {
                            ai_enhanced.set(true);
                            ai_cache_hit.set(v.get("cacheHit").and_then(|b| b.as_bool()).unwrap_or(false));
                            src.set(Some(new_url.to_string()));
                        }
                        if was_loading {
                            ai_revealing.set(true);
                            ai_loading.set(false);
                            let mut rev_sig = ai_revealing;
                            spawn(async move {
                                let mut ev = dioxus::document::eval("await new Promise((r) => setTimeout(r, 350)); dioxus.send(1);");
                                let _ = ev.recv::<i32>().await;
                                rev_sig.set(false);
                            });
                        } else {
                            ai_loading.set(false);
                            ai_revealing.set(false);
                        }
                        break;
                    }
                    _ => {
                        ai_loading.set(false);
                        ai_revealing.set(false);
                        break;
                    }
                }
            }
        });
    };

    use_effect(use_reactive!(|key_now| {
        if loaded_key.peek().as_ref() == Some(&key_now) && src.peek().is_some() {
            return;
        }
        let mut ai_enhanced = ai_enhanced;
        let mut ai_loading = ai_loading;
        let mut ai_revealing = ai_revealing;
        let mut ai_progress = ai_progress;
        let mut ai_cache_hit = ai_cache_hit;
        let mut needs_on_demand_ai = needs_on_demand_ai;
        let mut is_sender_original = is_sender_original;
        let mut raw_src = raw_src;
        let mut src = src;
        let mut loaded_key = loaded_key;
        let mut missing = missing;
        let mut failed = failed;
        let (u, k, n) = key_now.clone();
        ai_enhanced.set(false);
        ai_loading.set(false);
        ai_revealing.set(false);
        ai_progress.set(0);
        ai_cache_hit.set(false);
        needs_on_demand_ai.set(false);
        is_sender_original.set(false);
        if let Some(hit) = platform::get_cached_blob_url_sync(&u, &k, &n) {
            loaded_key.set(Some(key_now));
            missing.set(None);
            failed.set(false);
            raw_src.set(Some(hit.clone()));
            src.set(Some(hit.clone()));
            run_ai_pipeline(hit, u, sha_now.clone(), mime_now.clone(), qp_now.clone(), is_live_incoming, k, n, name_now.clone(), wire_w_now, wire_h_now);
            return;
        }
        src.set(None);
        raw_src.set(None);
        missing.set(None);
        failed.set(false);
        let m_in = mime_now.clone();
        let sc_in = scope_now.clone();
        let sh_in = sha_now.clone();
        let qp_in = qp_now.clone();
        let fname_in = name_now.clone();
        let wire_w_in = wire_w_now;
        let wire_h_in = wire_h_now;
        let target_key = key_now.clone();
        spawn(async move {
            let mime = if m_in.starts_with("image/") { m_in } else { "image/avif".into() };
            if let Some(burl) = platform::resolve_e2ee_blob_media_url(&u, &k, &n, &mime, "HIGH").await {
                loaded_key.set(Some(target_key));
                raw_src.set(Some(burl.clone()));
                src.set(Some(burl.clone()));
                run_ai_pipeline(burl, u, sh_in, mime, qp_in, is_live_incoming, k, n, fname_in, wire_w_in, wire_h_in);
                return;
            }
            let scope = if sc_in.is_empty() {
                if k.is_empty() { "group" } else { "dm" }
            } else {
                sc_in.as_str()
            };
            match recovery::load_or_fetch_media(&u, &k, &n, &mime, scope, "original").await {
                MediaFetchOutcome::Ready { bytes, mime: m, .. } => {
                    let burl = platform::register_cached_blob_url(&u, &k, &n, &bytes, &m).await;
                    loaded_key.set(Some(target_key));
                    raw_src.set(Some(burl.clone()));
                    src.set(Some(burl.clone()));
                    run_ai_pipeline(burl, u, sh_in, m, qp_in, is_live_incoming, k, n, fname_in, wire_w_in, wire_h_in);
                }
                MediaFetchOutcome::MissingNeedsRecovery { ref_id, availability_state, reason } => {
                    missing.set(Some((ref_id, availability_state, reason)));
                }
                MediaFetchOutcome::Error(_) => failed.set(true),
            }
        });
    }));

    let blur_parts = parse_blur_data_uri(&data.blur);
    let (box_style, stage_style) = if let Some((bw, bh)) = compute_chat_image_box(data.w, data.h) {
        (
            format!("width: min({bw}px, 100%);"),
            format!("aspect-ratio: {bw} / {bh};"),
        )
    } else if data.w > 0 && data.h > 0 {
        (String::new(), format!("aspect-ratio: {} / {};", data.w, data.h))
    } else {
        (String::new(), String::new())
    };

    rsx! {
        div { class: "rich-image", style: "{box_style}",
            if let Some(s) = src() {
                {
                    let lb_src = s.clone();
                    let lb_raw_src = raw_src().unwrap_or_else(|| s.clone());
                    let lb_name = crate::storage::media_opt::strip_file_extension(&data.name);
                    let lb_mid = data.u.clone();
                    let lb_sha = data.sha.clone();
                    let lb_qp = data.qp.clone();
                    let lb_w = data.w;
                    let lb_h = data.h;
                    let lb_orig_w = data.orig_w;
                    let lb_orig_h = data.orig_h;
                    let is_ai = ai_enhanced();
                    let is_orig = is_sender_original();
                    let can_ai = !is_orig && data.m != "image/gif";
                    let is_loading = ai_loading();
                    let is_revealing = ai_revealing();
                    let pct_now = ai_progress();
                    let show_corner_icon = needs_on_demand_ai() && !is_ai && !is_orig && !is_loading && !is_revealing;

                    let corner_raw = lb_raw_src.clone();
                    let corner_mid = data.u.clone();
                    let corner_sha = data.sha.clone();
                    let corner_mime = data.m.clone();
                    let corner_qp = data.qp.clone();
                    let corner_k = data.k.clone();
                    let corner_n = data.n.clone();
                    let corner_name = data.name.clone();
                    let corner_w = data.w;
                    let corner_h = data.h;
                    let dom_ref_id = crate::storage::extract_ref_id(&data.u).unwrap_or_default();

                    rsx! {
                        button {
                            r#type: "button",
                            class: "rich-image-btn rich-image-stage loaded",
                            style: "{stage_style}",
                            aria_label: "See photos",
                            onclick: move |_| {
                                *GLOBAL_LIGHTBOX.write() = Some(LightboxMedia {
                                    src: lb_src.clone(),
                                    name: lb_name.clone(),
                                    media_id: lb_mid.clone(),
                                    content_hash: lb_sha.clone(),
                                    orig_profile_json: lb_qp.clone(),
                                    ai_restored: is_ai,
                                    original_src: Some(lb_raw_src.clone()),
                                    is_sender_original: is_orig,
                                    auto_enhance_on_open: show_corner_icon,
                                    can_ai_enhance: can_ai,
                                    width: lb_w,
                                    height: lb_h,
                                    orig_width: lb_orig_w,
                                    orig_height: lb_orig_h,
                                });
                            },
                            img {
                                class: if is_loading { "rich-image-full is-ai-generating" } else { "rich-image-full" },
                                src: "{s}",
                                alt: "Image",
                                decoding: "async",
                                "data-media-ref": "{dom_ref_id}",
                                "data-ai-restored": if is_ai { "true" } else { "false" },
                                "data-ai-cache-hit": if ai_cache_hit() { "true" } else { "false" },
                                "data-sender-original": if is_orig { "true" } else { "false" },
                                "data-ai-pending": if show_corner_icon { "true" } else { "false" },
                            }
                            if is_loading || is_revealing {
                                div {
                                    class: if is_revealing && !is_loading { "ai-gen-overlay is-revealing" } else { "ai-gen-overlay" },
                                    aria_hidden: "true",
                                    img {
                                        class: "ai-gen-blur-img",
                                        src: "{lb_raw_src}",
                                        alt: "",
                                        draggable: "false",
                                    }
                                    div { class: "ai-gen-frost" }
                                    div { class: "ai-gen-aurora" }
                                    div { class: "ai-gen-dots-base" }
                                    div { class: "ai-gen-dots-wave" }
                                    span { class: "ai-gen-pct", "{pct_now}%" }
                                }
                            } else if show_corner_icon {
                                span {
                                    class: "rich-image-enhance-corner",
                                    title: "Làm nét ảnh",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        run_ai_pipeline(
                                            corner_raw.clone(),
                                            corner_mid.clone(),
                                            corner_sha.clone(),
                                            corner_mime.clone(),
                                            corner_qp.clone(),
                                            true,
                                            corner_k.clone(),
                                            corner_n.clone(),
                                            corner_name.clone(),
                                            corner_w,
                                            corner_h,
                                        );
                                    },
                                    FocusIcon { size: 18 }
                                }
                            }
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
    #[props(default)] is_live_incoming: bool,
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
        Rich::Image(d) => rsx! { RichImage { data: d, is_live_incoming } },
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
                            accept: "image/png,image/jpeg,image/webp,image/avif,image/gif,video/mp4,video/webm",
                            style: "display: none;",
                            onchange: move |_| {
                                open.set(false);
                                let fid = img_id2.clone();
                                let orig = preserve_original();
                                spawn(async move {
                                    // Phase 1 (< 20ms): Render crisp sender local original immediately before compression/E2EE/upload!
                                    match media_opt::pick_instant_preview(&fid).await {
                                        Ok(pick) => {
                                            let hid = pick.handle_id.clone();
                                            push_pending_media(PendingSenderMedia {
                                                handle_id: hid.clone(),
                                                preview_url: pick.instant_preview_url.clone(),
                                                file_name: pick.file_name.clone(),
                                                media_kind: pick.media_kind.clone(),
                                                width: pick.width.unwrap_or(0),
                                                height: pick.height.unwrap_or(0),
                                                status_label: "Đang tối ưu".into(),
                                                failed: false,
                                            });
                                            platform::scroll_chat_to_bottom(false);
                                            match media_opt::process_pending_handle(&hid, orig).await {
                                                Ok(payload) => {
                                                    media_opt::release_pending_handle(&hid, false);
                                                    if let Some(h) = on_media {
                                                        h.call(payload);
                                                    } else {
                                                        remove_pending_media(&hid);
                                                        on_image.call((payload.bytes, payload.mime));
                                                    }
                                                }
                                                Err(e) if e.contains("svg_not_allowed") => {
                                                    media_opt::release_pending_handle(&hid, true);
                                                    remove_pending_media(&hid);
                                                    *GLOBAL_MEDIA_NOTICE.write() = Some("Không hỗ trợ gửi tệp SVG".into());
                                                }
                                                Err(_) => {
                                                    update_pending_media_status(&hid, "Xử lý thất bại", true);
                                                }
                                            }
                                        }
                                        Err(e) if e.contains("svg_not_allowed") => {
                                            *GLOBAL_MEDIA_NOTICE.write() = Some("Không hỗ trợ gửi tệp SVG".into());
                                        }
                                        Err(_) => {
                                            match media_opt::pick_and_optimize_from_input(&fid, orig).await {
                                                Ok(payload) => {
                                                    if let Some(h) = on_media {
                                                        h.call(payload);
                                                    } else {
                                                        on_image.call((payload.bytes, payload.mime));
                                                    }
                                                }
                                                Err(e) if e.contains("svg_not_allowed") => {
                                                    *GLOBAL_MEDIA_NOTICE.write() = Some("Không hỗ trợ gửi tệp SVG".into());
                                                }
                                                Err(_) => match platform::read_selected_file_raw(&fid).await {
                                                    Ok((bytes, mime)) => on_image.call((bytes, mime)),
                                                    Err(e) if e.contains("svg_not_allowed") => {
                                                        *GLOBAL_MEDIA_NOTICE.write() = Some("Không hỗ trợ gửi tệp SVG".into());
                                                    }
                                                    Err(_) => {}
                                                },
                                            }
                                        }
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
                                        match media_opt::pick_and_optimize_from_input(&fid, true).await {
                                            Ok(payload) => {
                                                if let Some(h) = on_media {
                                                    h.call(payload);
                                                }
                                            }
                                            Err(e) if e.contains("svg_not_allowed") => {
                                                *GLOBAL_MEDIA_NOTICE.write() = Some("Không hỗ trợ gửi tệp SVG".into());
                                            }
                                            Err(_) => {}
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

/// Pure pan limit calculation for Spec 080 corner-clamped lightbox panning.
/// Ensures zoomed images stop flush at the viewport edges/corners and never disappear off-screen.
pub fn lightbox_pan_limits(
    zoom: f64,
    rotation_deg: i32,
    base_w: f64,
    base_h: f64,
    vw: f64,
    vh: f64,
) -> (f64, f64) {
    if base_w <= 0.0 || base_h <= 0.0 || vw <= 0.0 || vh <= 0.0 || zoom <= 1.001 {
        return (0.0, 0.0);
    }
    let quarter = ((rotation_deg % 360) + 360) % 360;
    let (w0, h0) = if quarter == 90 || quarter == 270 {
        (base_h, base_w)
    } else {
        (base_w, base_h)
    };
    let max_x = ((w0 * zoom - vw) * 0.5).max(0.0);
    let max_y = ((h0 * zoom - vh) * 0.5).max(0.0);
    (max_x, max_y)
}

/// Hard-clamps `(px, py)` so image edges never pull inside the viewport (max is the exact corner).
pub fn clamp_lightbox_pan(
    px: f64,
    py: f64,
    zoom: f64,
    rotation_deg: i32,
    base_w: f64,
    base_h: f64,
    vw: f64,
    vh: f64,
) -> (f64, f64) {
    let (mx, my) = lightbox_pan_limits(zoom, rotation_deg, base_w, base_h, vw, vh);
    (px.clamp(-mx, mx), py.clamp(-my, my))
}

/// Synchronously reads `(window.innerWidth, window.innerHeight)` on WASM so Frame-0 Lightbox
/// sizing has zero async delay and zero first-frame layout flash (Spec 112).
fn read_viewport_size() -> (f64, f64) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            let vw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1440.0);
            let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(900.0);
            if vw > 100.0 && vh > 100.0 {
                return (vw, vh);
            }
        }
    }
    (1440.0, 900.0)
}

/// Computes the exact final post-enhancement pixel dimensions `(final_w, final_h)` upfront
/// before Real-ESRGAN 4× starts so Lightbox renders at the final display size from Frame 0
/// with 0px resize jump across open -> enhance -> 4K completion -> toggle original (Spec 112).
pub fn compute_lightbox_target_dimensions(
    src_w: u32,
    src_h: u32,
    orig_w: u32,
    orig_h: u32,
    is_ai_eligible: bool,
) -> Option<(f64, f64)> {
    let (base_w, base_h) = if src_w > 0 && src_h > 0 {
        (src_w as f64, src_h as f64)
    } else if orig_w > 0 && orig_h > 0 {
        (orig_w as f64, orig_h as f64)
    } else {
        return None;
    };
    let aspect = (base_w / base_h).max(0.05);
    let (nat_w, nat_h) = if orig_w > 0 && orig_h > 0 {
        (orig_w as f64, orig_h as f64)
    } else {
        (base_w, base_h)
    };
    if !is_ai_eligible {
        return Some((nat_w, nat_h));
    }
    // Mirror `ai_restorer.js` exact Real-ESRGAN 4× output geometry:
    // input longest edge capped at 1080px -> 4× -> output longest edge capped at 3840px.
    let max_in = base_w.max(base_h);
    let (proc_w, proc_h) = if max_in > 1080.0 {
        let r = 1080.0 / max_in;
        ((base_w * r).round().max(32.0), (base_h * r).round().max(32.0))
    } else {
        (base_w, base_h)
    };
    let mut out_w = proc_w * 4.0;
    let out_h = proc_h * 4.0;
    let max_out = out_w.max(out_h);
    if max_out > 3840.0 {
        let r = 3840.0 / max_out;
        out_w = (out_w * r).round();
    }
    let final_w = out_w.max(nat_w);
    let final_h = (final_w / aspect).round().max(1.0);
    Some((final_w, final_h))
}

/// Fits final natural dimensions `(final_w, final_h)` deterministically inside the Lightbox stage
/// (`min(92vw, 1440px) × 86vh`) without artificially upscaling native sender originals beyond 1:1.
pub fn compute_lightbox_fit_box(final_w: f64, final_h: f64, vw: f64, vh: f64) -> (f64, f64) {
    let max_w = (vw * 0.92).min(1440.0).max(1.0);
    let max_h = (vh * 0.86).max(1.0);
    if final_w <= 0.0 || final_h <= 0.0 {
        return (max_w.round(), max_h.round());
    }
    let scale = (max_w / final_w).min(max_h / final_h).min(1.0);
    ((final_w * scale).round().max(1.0), (final_h * scale).round().max(1.0))
}

/// Computes stable chat-bubble display dimensions inside `352px × 352px` (`22rem × 22rem`) so
/// image bubbles never reflow between blur-placeholder, wire AVIF, AI enhancement, and 4K swap.
pub fn compute_chat_image_box(w: u32, h: u32) -> Option<(u32, u32)> {
    if w == 0 || h == 0 {
        return None;
    }
    let max_edge = 352.0_f64;
    let wf = w as f64;
    let hf = h as f64;
    let scale = (max_edge / wf).min(max_edge / hf).min(1.0);
    let bw = (wf * scale).round().clamp(48.0, max_edge) as u32;
    let bh = (hf * scale).round().clamp(48.0, max_edge) as u32;
    Some((bw, bh))
}

async fn measure_lightbox_bounds() -> Option<(f64, f64, f64, f64)> {
    let mut ev = dioxus::document::eval(
        r#"
        const img = document.querySelector('.lightbox-img');
        if (img && img.offsetWidth > 0 && img.offsetHeight > 0) {
            dioxus.send([img.offsetWidth, img.offsetHeight, window.innerWidth, window.innerHeight]);
        } else {
            dioxus.send(null);
        }
        "#,
    );
    ev.recv::<Option<[f64; 4]>>()
        .await
        .ok()
        .flatten()
        .map(|a| (a[0], a[1], a[2], a[3]))
}

async fn read_lightbox_natural_size() -> Option<(u32, u32)> {
    let mut ev = dioxus::document::eval(
        r#"
        const img = document.querySelector('.lightbox-img');
        if (img && img.naturalWidth > 0 && img.naturalHeight > 0) {
            dioxus.send([img.naturalWidth, img.naturalHeight]);
        } else {
            dioxus.send(null);
        }
        "#,
    );
    ev.recv::<Option<[u32; 2]>>()
        .await
        .ok()
        .flatten()
        .map(|a| (a[0], a[1]))
}

/// Pure-Rust True-Physics Mass-Spring-Damper & Pointer Momentum Engine for Lightbox (Spec 113).
/// Implements Apple `CASpringAnimation` / PhotoSwipe v5 Hooke's law dynamics:
///   F = -k * (x - target) - c * v,   c = 2 * zeta * sqrt(k * m)
/// with 250Hz sub-stepped integration, recent pointer velocity tracking for flick momentum carry,
/// zoomed interior exponential glide, boundary rebound spring, and seamless mid-flight grab.
#[derive(Clone, Debug)]
pub struct LightboxSpringEngine {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub tx: f64,
    pub ty: f64,
    pub zoom: f64,
    pub vz: f64,
    pub tzoom: f64,
    pub rot: f64,
    pub vrot: f64,
    pub trot: f64,
    pub base_w: f64,
    pub base_h: f64,
    pub vw: f64,
    pub vh: f64,
    pub dragging: bool,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub drag_origin_x: f64,
    pub drag_origin_y: f64,
    pub samples: Vec<(f64, f64, f64)>,
    pub generation: u64,
    pub raf_running: bool,
    pub last_frame_ms: f64,
}

impl Default for LightboxSpringEngine {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            tx: 0.0,
            ty: 0.0,
            zoom: 1.0,
            vz: 0.0,
            tzoom: 1.0,
            rot: 0.0,
            vrot: 0.0,
            trot: 0.0,
            base_w: 0.0,
            base_h: 0.0,
            vw: 0.0,
            vh: 0.0,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_origin_x: 0.0,
            drag_origin_y: 0.0,
            samples: Vec::with_capacity(12),
            generation: 0,
            raf_running: false,
            last_frame_ms: 0.0,
        }
    }
}

impl LightboxSpringEngine {
    // Pan spring: fluid near-critical Apple CASpringAnimation feel (k=400, zeta=0.92).
    // Settles 220px..400px displacements cleanly to exact 0.0px in ~320..360ms while carrying real flick momentum!
    const PAN_STIFFNESS: f64 = 400.0;
    const PAN_DAMPING_RATIO: f64 = 0.92;
    // Zoom / rotation spring: snappy, near-critically damped (~240ms settle, zero wobble).
    const ZOOM_STIFFNESS: f64 = 440.0;
    const ZOOM_DAMPING_RATIO: f64 = 0.94;
    // Interior zoomed glide friction (iOS UIScrollView-style exponential velocity decay).
    const GLIDE_DECAY: f64 = 5.4;
    const MAX_RELEASE_SPEED: f64 = 3800.0;
    const MAX_SUB_DT: f64 = 0.004;

    pub fn reset_immediate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.raf_running = false;
        self.dragging = false;
        self.x = 0.0;
        self.y = 0.0;
        self.vx = 0.0;
        self.vy = 0.0;
        self.tx = 0.0;
        self.ty = 0.0;
        self.zoom = 1.0;
        self.vz = 0.0;
        self.tzoom = 1.0;
        self.rot = 0.0;
        self.vrot = 0.0;
        self.trot = 0.0;
        self.samples.clear();
        self.last_frame_ms = 0.0;
    }

    pub fn set_bounds(&mut self, base_w: f64, base_h: f64, vw: f64, vh: f64) {
        self.base_w = base_w;
        self.base_h = base_h;
        self.vw = vw;
        self.vh = vh;
    }

    /// Catches the image seamlessly right at its live mid-air coordinates (zero teleport!).
    pub fn begin_drag(&mut self, client_x: f64, client_y: f64, now_ms: f64) {
        self.generation = self.generation.wrapping_add(1);
        self.raf_running = false;
        self.dragging = true;
        self.vx = 0.0;
        self.vy = 0.0;
        self.drag_start_x = client_x;
        self.drag_start_y = client_y;
        self.drag_origin_x = self.x;
        self.drag_origin_y = self.y;
        self.samples.clear();
        self.samples.push((now_ms, client_x, client_y));
    }

    /// Tracks pointer 1:1 with zero latency and records velocity samples for authentic release momentum.
    pub fn move_drag(&mut self, client_x: f64, client_y: f64, now_ms: f64) -> bool {
        if !self.dragging {
            return false;
        }
        let dx = client_x - self.drag_start_x;
        let dy = client_y - self.drag_start_y;
        self.x = self.drag_origin_x + dx;
        self.y = self.drag_origin_y + dy;
        self.tx = self.x;
        self.ty = self.y;
        self.samples.push((now_ms, client_x, client_y));
        // Keep only recent samples within the last 100ms
        let cutoff = now_ms - 100.0;
        if self.samples.len() > 12 {
            let drop_n = self.samples.len() - 12;
            self.samples.drain(0..drop_n);
        }
        while self.samples.len() > 2 && self.samples[0].0 < cutoff {
            self.samples.remove(0);
        }
        dx.abs() > 3.0 || dy.abs() > 3.0
    }

    /// Computes physical pointer release velocity (px/s) and engages spring return / momentum glide.
    pub fn end_drag(&mut self, now_ms: f64) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        let (mut vx, mut vy) = (0.0_f64, 0.0_f64);
        if let Some(&(t_last, x_last, y_last)) = self.samples.last() {
            // If pointer paused > 65ms before release, hand momentum naturally decayed to 0.
            if now_ms - t_last <= 65.0 && self.samples.len() >= 2 {
                // Pick the earliest sample within the last 90ms that spans >= 6ms for low-noise velocity.
                let mut chosen = self.samples[0];
                for &s in &self.samples {
                    if t_last - s.0 <= 90.0 {
                        chosen = s;
                        break;
                    }
                }
                let dt_ms = t_last - chosen.0;
                if dt_ms >= 4.0 {
                    vx = ((x_last - chosen.1) / (dt_ms / 1000.0))
                        .clamp(-Self::MAX_RELEASE_SPEED, Self::MAX_RELEASE_SPEED);
                    vy = ((y_last - chosen.2) / (dt_ms / 1000.0))
                        .clamp(-Self::MAX_RELEASE_SPEED, Self::MAX_RELEASE_SPEED);
                }
            }
        }
        self.samples.clear();
        self.vx = vx;
        self.vy = vy;
        let rot_i = self.trot.round() as i32;
        let (cx, cy) = clamp_lightbox_pan(
            self.x,
            self.y,
            self.tzoom,
            rot_i,
            self.base_w,
            self.base_h,
            self.vw,
            self.vh,
        );
        self.tx = cx;
        self.ty = cy;
    }

    /// Sets new spring targets for zoom / rotate / reset / resize clamp smoothly.
    pub fn set_targets(&mut self, next_zoom: f64, next_rot: f64, force_center: bool) {
        self.tzoom = next_zoom;
        self.trot = next_rot;
        if force_center {
            self.tx = 0.0;
            self.ty = 0.0;
        } else {
            let rot_i = next_rot.round() as i32;
            let (cx, cy) = clamp_lightbox_pan(
                self.x,
                self.y,
                next_zoom,
                rot_i,
                self.base_w,
                self.base_h,
                self.vw,
                self.vh,
            );
            self.tx = cx;
            self.ty = cy;
        }
    }

    pub fn snap_to_targets(&mut self) {
        self.x = self.tx;
        self.y = self.ty;
        self.vx = 0.0;
        self.vy = 0.0;
        self.zoom = self.tzoom;
        self.vz = 0.0;
        self.rot = ((self.trot.round() as i32 % 360) + 360) as f64 % 360.0;
        self.trot = self.rot;
        self.vrot = 0.0;
    }

    /// Advances the physical mass-spring-damper simulation by `dt_sec`.
    /// Returns `true` while motion is still active, `false` once settled cleanly on target.
    pub fn step(&mut self, dt_sec: f64) -> bool {
        let total_dt = dt_sec.clamp(0.0005, 0.15);
        let sub_steps = (total_dt / Self::MAX_SUB_DT).ceil().max(1.0) as usize;
        let h = total_dt / (sub_steps as f64);

        let pan_c = 2.0 * Self::PAN_DAMPING_RATIO * Self::PAN_STIFFNESS.sqrt();
        let z_c = 2.0 * Self::ZOOM_DAMPING_RATIO * Self::ZOOM_STIFFNESS.sqrt();
        let rot_i = self.trot.round() as i32;
        let (mx, my) = lightbox_pan_limits(
            self.tzoom,
            rot_i,
            self.base_w,
            self.base_h,
            self.vw,
            self.vh,
        );

        for _ in 0..sub_steps {
            // Zoom & Rotation spring
            let az = -Self::ZOOM_STIFFNESS * (self.zoom - self.tzoom) - z_c * self.vz;
            self.vz += az * h;
            self.zoom += self.vz * h;

            let ar = -Self::ZOOM_STIFFNESS * (self.rot - self.trot) - z_c * self.vrot;
            self.vrot += ar * h;
            self.rot += self.vrot * h;

            // Axis X: interior zoomed inertia glide OR boundary/return Hooke's law spring
            if mx > 0.5 && self.x.abs() <= mx && (self.x - self.tx).abs() <= 1.0 && self.vx.abs() > 8.0 {
                self.vx *= (-Self::GLIDE_DECAY * h).exp();
                self.x += self.vx * h;
                self.tx = self.x.clamp(-mx, mx);
            } else {
                self.tx = self.tx.clamp(-mx, mx);
                let ax = -Self::PAN_STIFFNESS * (self.x - self.tx) - pan_c * self.vx;
                self.vx += ax * h;
                self.x += self.vx * h;
            }

            // Axis Y: interior zoomed inertia glide OR boundary/return Hooke's law spring
            if my > 0.5 && self.y.abs() <= my && (self.y - self.ty).abs() <= 1.0 && self.vy.abs() > 8.0 {
                self.vy *= (-Self::GLIDE_DECAY * h).exp();
                self.y += self.vy * h;
                self.ty = self.y.clamp(-my, my);
            } else {
                self.ty = self.ty.clamp(-my, my);
                let ay = -Self::PAN_STIFFNESS * (self.y - self.ty) - pan_c * self.vy;
                self.vy += ay * h;
                self.y += self.vy * h;
            }
        }

        let settled = (self.x - self.tx).abs() < 0.65
            && (self.y - self.ty).abs() < 0.65
            && self.vx.abs() < 22.0
            && self.vy.abs() < 22.0
            && (self.zoom - self.tzoom).abs() < 0.0008
            && self.vz.abs() < 0.015
            && (self.rot - self.trot).abs() < 0.08
            && self.vrot.abs() < 2.5;

        if settled {
            self.snap_to_targets();
            false
        } else {
            true
        }
    }

    pub fn format_transform(&self) -> String {
        let px = if self.x.abs() < 0.05 { 0.0 } else { self.x };
        let py = if self.y.abs() < 0.05 { 0.0 } else { self.y };
        let rot_norm = if (self.rot - self.rot.round()).abs() < 0.05 {
            format!("{}deg", self.rot.round() as i32)
        } else {
            format!("{:.1}deg", self.rot)
        };
        format!(
            "translate3d({px:.1}px, {py:.1}px, 0) scale({:.3}) rotate({rot_norm})",
            self.zoom
        )
    }
}

#[cfg(target_arch = "wasm32")]
fn now_perf_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or_else(|| js_sys::Date::now())
}

#[cfg(not(target_arch = "wasm32"))]
fn now_perf_ms() -> f64 {
    0.0
}

#[cfg(target_arch = "wasm32")]
fn prefers_reduced_motion() -> bool {
    let Some(win) = web_sys::window() else { return false; };
    if let Some(doc) = win.document() {
        if let Some(root) = doc.document_element() {
            if root.get_attribute("data-reduce-motion").as_deref() == Some("true") {
                return true;
            }
        }
    }
    false
}

#[cfg(target_arch = "wasm32")]
fn apply_lightbox_transform_dom(tf: &str) {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return; };
    for sel in [".lightbox-img", ".ai-gen-overlay.lightbox-ai-gen"] {
        if let Ok(Some(el)) = doc.query_selector(sel) {
            if let Ok(he) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = he.style().set_property("transform", tf);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_lightbox_transform_dom(_tf: &str) {}

#[cfg(target_arch = "wasm32")]
fn kick_lightbox_spring_raf(engine: std::rc::Rc<std::cell::RefCell<LightboxSpringEngine>>) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::{closure::Closure, JsCast};

    if prefers_reduced_motion() {
        let tf = {
            let mut st = engine.borrow_mut();
            st.snap_to_targets();
            st.format_transform()
        };
        apply_lightbox_transform_dom(&tf);
        return;
    }

    let (gen_id, already_running) = {
        let mut st = engine.borrow_mut();
        if st.raf_running {
            (st.generation, true)
        } else {
            st.raf_running = true;
            st.last_frame_ms = now_perf_ms();
            (st.generation, false)
        }
    };
    if already_running {
        return;
    }

    let cb_slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let cb_slot_clone = cb_slot.clone();
    let eng_clone = engine.clone();

    *cb_slot.borrow_mut() = Some(Closure::wrap(Box::new(move |_ts_ms: f64| {
        let now_ms = now_perf_ms();
        let (keep_going, tf) = {
            let mut st = eng_clone.borrow_mut();
            if st.generation != gen_id || st.dragging {
                st.raf_running = false;
                drop(st);
                let _ = cb_slot_clone.borrow_mut().take();
                return;
            }
            let prev = if st.last_frame_ms > 0.0 { st.last_frame_ms } else { now_ms - 16.667 };
            st.last_frame_ms = now_ms;
            let dt = ((now_ms - prev) / 1000.0).clamp(0.001, 0.150);
            let active = st.step(dt);
            if !active {
                st.raf_running = false;
            }
            (active, st.format_transform())
        };
        apply_lightbox_transform_dom(&tf);
        if keep_going {
            if let Some(win) = web_sys::window() {
                if let Some(cb) = cb_slot_clone.borrow().as_ref() {
                    if win.request_animation_frame(cb.as_ref().unchecked_ref()).is_ok() {
                        return;
                    }
                }
            }
            eng_clone.borrow_mut().raf_running = false;
        }
        let _ = cb_slot_clone.borrow_mut().take();
    }) as Box<dyn FnMut(f64)>));

    if let Some(win) = web_sys::window() {
        if let Some(cb) = cb_slot.borrow().as_ref() {
            if win.request_animation_frame(cb.as_ref().unchecked_ref()).is_err() {
                engine.borrow_mut().raf_running = false;
                let _ = cb_slot.borrow_mut().take();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn kick_lightbox_spring_raf(engine: std::rc::Rc<std::cell::RefCell<LightboxSpringEngine>>) {
    engine.borrow_mut().snap_to_targets();
}

/// Root-mounted Telegram Web K style global image lightbox viewer (Specs 078, 080, 101, 112 & 113).
/// Computes exact final post-enhancement viewport fit dimensions synchronously on Frame 0 (0px jump)
/// and drives unrounded 0px-radius photos with a 60/120/240Hz Pure-Rust mass-spring-damper physics engine.
#[component]
pub fn GlobalImageLightbox() -> Element {
    let _ = SESSION_START_MS.get_or_init(|| Utc::now().timestamp_millis());
    let media_opt = GLOBAL_LIGHTBOX();
    let mut zoom = use_signal(|| 1.0_f64);
    let mut rotation = use_signal(|| 0_i32);
    let mut dragging = use_signal(|| false);
    let mut did_drag = use_signal(|| false);
    let mut show_original = use_signal(|| false);
    let mut upscaling_ai = use_signal(|| false);
    let mut lb_revealing = use_signal(|| false);
    let mut lb_progress = use_signal(|| 0u32);
    let mut active_key = use_signal(String::new);
    let mut viewport = use_signal(read_viewport_size);
    let mut fallback_nat = use_signal(|| (0u32, 0u32));
    // Live layout (base_w, base_h, vw, vh) used for corner-clamped panning
    let mut bounds = use_signal(|| (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64));
    // Zero-VDOM-rerender 60/120/240Hz Pure-Rust physics controller (stored in a Copy Signal handle, accessed only via `.peek()`)
    let physics = use_signal(|| std::rc::Rc::new(std::cell::RefCell::new(LightboxSpringEngine::default())));

    let cur_key = media_opt
        .as_ref()
        .map(|m| format!("{}::{}", m.media_id, m.original_src.as_deref().unwrap_or(&m.src)))
        .unwrap_or_default();
    let lightbox_open = !cur_key.is_empty();
    if *active_key.peek() != cur_key {
        let auto_enh = media_opt
            .as_ref()
            .map(|m| m.auto_enhance_on_open && !m.ai_restored && !m.is_sender_original && m.can_ai_enhance)
            .unwrap_or(false);
        active_key.set(cur_key.clone());
        viewport.set(read_viewport_size());
        fallback_nat.set((0, 0));
        show_original.set(false);
        upscaling_ai.set(false);
        lb_revealing.set(false);
        lb_progress.set(if auto_enh { 3 } else { 0 });
        zoom.set(1.0);
        rotation.set(0);
        dragging.set(false);
        did_drag.set(false);
        bounds.set((0.0, 0.0, 0.0, 0.0));
        physics.peek().borrow_mut().reset_immediate();
    }

    let run_lb_enhance = move |m: LightboxMedia| {
        let Some(raw_u) = m.original_src.clone() else { return; };
        if *upscaling_ai.peek() {
            return;
        }
        let mut upscaling_ai = upscaling_ai;
        let mut lb_revealing = lb_revealing;
        let mut lb_progress = lb_progress;
        let mut show_original = show_original;
        let key_at_start = active_key.peek().clone();
        lb_revealing.set(false);
        upscaling_ai.set(true);
        lb_progress.set(3);
        spawn(async move {
            let url_json = serde_json::to_string(&raw_u).unwrap_or_default();
            let mid_json = serde_json::to_string(&if m.media_id.is_empty() { m.name.clone() } else { m.media_id.clone() }).unwrap_or_default();
            let hash_json = serde_json::to_string(&m.content_hash).unwrap_or_else(|_| "\"v1\"".to_string());
            let qp_json = serde_json::to_string(&m.orig_profile_json).unwrap_or_else(|_| "\"\"".to_string());
            let js = format!(
                r#"
                (async () => {{
                    if (!window.__chatwAiRestorer) {{ dioxus.send({{ type: 'failed' }}); return; }}
                    // Let Lightbox Frame-0/Frame-1 paint smoothly at 60/120fps before kicking off GPU AI work!
                    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
                    const res = await window.__chatwAiRestorer.restoreImageFromUrl({url_json}, {{
                        mediaId: {mid_json},
                        contentHash: {hash_json},
                        origProfileJson: {qp_json},
                        force4x: true,
                        forceRun: true,
                        onProgress: (pct) => {{
                            try {{ dioxus.send({{ type: 'progress', pct: Number(pct) || 0 }}); }} catch (_) {{}}
                        }},
                    }});
                    if (res && res.enhanced && res.url) {{
                        dioxus.send({{ type: 'done', url: res.url }});
                    }} else {{
                        dioxus.send({{ type: 'failed' }});
                    }}
                }})();
                "#
            );
            let mut eval = dioxus::document::eval(&js);
            while let Ok(v) = eval.recv::<serde_json::Value>().await {
                match v.get("type").and_then(|s| s.as_str()).unwrap_or("") {
                    "progress" => {
                        if let Some(p) = v.get("pct").and_then(|n| n.as_u64()) {
                            let p32 = (p as u32).min(100);
                            if p32 != *lb_progress.peek() {
                                lb_progress.set(p32);
                            }
                        }
                    }
                    "done" => {
                        if let Some(new_url) = v.get("url").and_then(|s| s.as_str()).filter(|s| !s.is_empty()) {
                            lb_progress.set(100);
                            show_original.set(false);
                            if !m.media_id.is_empty() {
                                *GLOBAL_AI_UPDATED.write() = Some((m.media_id.clone(), new_url.to_string()));
                            }
                            let still_same = GLOBAL_LIGHTBOX
                                .peek()
                                .as_ref()
                                .map(|cur| cur.media_id == m.media_id && cur.original_src == m.original_src)
                                .unwrap_or(false);
                            if still_same {
                                *GLOBAL_LIGHTBOX.write() = Some(LightboxMedia {
                                    src: new_url.to_string(),
                                    ai_restored: true,
                                    auto_enhance_on_open: false,
                                    ..m.clone()
                                });
                                lb_revealing.set(true);
                                upscaling_ai.set(false);
                                let mut rev_sig = lb_revealing;
                                let active_key_ref = active_key;
                                let expected_key = key_at_start.clone();
                                spawn(async move {
                                    let mut ev = dioxus::document::eval(
                                        "await new Promise((r) => setTimeout(r, 350)); dioxus.send(1);",
                                    );
                                    let _ = ev.recv::<i32>().await;
                                    if *active_key_ref.peek() == expected_key {
                                        rev_sig.set(false);
                                    }
                                });
                            } else {
                                upscaling_ai.set(false);
                            }
                        } else {
                            upscaling_ai.set(false);
                        }
                        break;
                    }
                    _ => {
                        upscaling_ai.set(false);
                        lb_revealing.set(false);
                        break;
                    }
                }
            }
            if *upscaling_ai.peek() {
                upscaling_ai.set(false);
            }
        });
    };

    // Scenario B (New Device / History): when user taps an unenhanced image to view it, automatically enhance inside Lightbox!
    use_effect(use_reactive!(|cur_key| {
        if cur_key.is_empty() {
            return;
        }
        if let Some(m) = GLOBAL_LIGHTBOX.peek().clone() {
            if m.auto_enhance_on_open && !m.ai_restored && !m.is_sender_original && m.can_ai_enhance {
                run_lb_enhance(m);
            }
        }
    }));

    // Single rAF-throttled viewport resize listener while Lightbox is open (never re-created on `src` swap!).
    use_effect(use_reactive!(|lightbox_open| {
        if !lightbox_open {
            let _ = dioxus::document::eval(
                r#"
                if (window.__chatwLbResize) {
                    window.removeEventListener('resize', window.__chatwLbResize);
                    window.__chatwLbResize = null;
                }
                "#,
            );
            return;
        }
        spawn(async move {
            let mut ev = dioxus::document::eval(
                r#"
                let raf = 0;
                const onResize = () => {
                    if (raf) return;
                    raf = requestAnimationFrame(() => {
                        raf = 0;
                        try { dioxus.send([window.innerWidth, window.innerHeight]); } catch (_) {}
                    });
                };
                if (window.__chatwLbResize) {
                    window.removeEventListener('resize', window.__chatwLbResize);
                }
                window.__chatwLbResize = onResize;
                window.addEventListener('resize', onResize, { passive: true });
                "#,
            );
            while let Ok(arr) = ev.recv::<[f64; 2]>().await {
                let (vw, vh) = (arr[0], arr[1]);
                if vw > 100.0 && vh > 100.0 {
                    viewport.set((vw, vh));
                }
            }
        });
    }));

    let Some(media) = media_opt else {
        return rsx! {};
    };

    let can_toggle_ai = media.can_ai_enhance && !media.is_sender_original && media.original_src.is_some();
    let (vw, vh) = viewport();
    let (fb_w, fb_h) = fallback_nat();
    let eff_w = if media.width > 0 { media.width } else { fb_w };
    let eff_h = if media.height > 0 { media.height } else { fb_h };
    let fit_box = compute_lightbox_target_dimensions(eff_w, eff_h, media.orig_width, media.orig_height, can_toggle_ai)
        .map(|(fw, fh)| compute_lightbox_fit_box(fw, fh, vw, vh));

    if let Some((fw, fh)) = fit_box {
        physics.peek().borrow_mut().set_bounds(fw, fh, vw, vh);
    } else {
        let (bw, bh, v_w, v_h) = *bounds.peek();
        if bw > 0.0 && bh > 0.0 {
            physics.peek().borrow_mut().set_bounds(bw, bh, v_w, v_h);
        }
    }

    let mut set_zoom_clamped = move |next: f64| {
        // Spec 080: Min zoom locked at 100% (1.0x) so photo never shrinks too tiny; max 400% (4.0x).
        let clamped = (next.clamp(1.0, 4.0) * 100.0).round() / 100.0;
        zoom.set(clamped);
        let eng = physics.peek().clone();
        let next_rot = eng.borrow().trot;
        eng.borrow_mut().set_targets(clamped, next_rot, false);
        kick_lightbox_spring_raf(eng);
        if fit_box.is_none() {
            spawn(async move {
                if let Some((bw, bh, v_w, v_h)) = measure_lightbox_bounds().await {
                    bounds.set((bw, bh, v_w, v_h));
                    let eng2 = physics.peek().clone();
                    {
                        let mut st = eng2.borrow_mut();
                        st.set_bounds(bw, bh, v_w, v_h);
                        let z_now = st.tzoom;
                        let r_now = st.trot;
                        st.set_targets(z_now, r_now, false);
                    }
                    kick_lightbox_spring_raf(eng2);
                }
            });
        }
    };

    let mut apply_rotate = move || {
        let next_rot_i = (*rotation.peek() + 90) % 360;
        rotation.set(next_rot_i);
        let eng = physics.peek().clone();
        {
            let mut st = eng.borrow_mut();
            let z_now = st.tzoom;
            // Always rotate clockwise smoothly (`270 -> 360`, normalized to `0` on settle)
            let next_rot_f = st.trot + 90.0;
            st.set_targets(z_now, next_rot_f, false);
        }
        kick_lightbox_spring_raf(eng);
    };

    let mut reset_view = move || {
        zoom.set(1.0);
        rotation.set(0);
        dragging.set(false);
        did_drag.set(false);
        let eng = physics.peek().clone();
        eng.borrow_mut().set_targets(1.0, 0.0, true);
        kick_lightbox_spring_raf(eng);
    };

    let mut start_drag = move |cx: f64, cy: f64| {
        physics.peek().borrow_mut().begin_drag(cx, cy, now_perf_ms());
        dragging.set(true);
        did_drag.set(false);
    };

    let mut handle_drag_move = move |cx: f64, cy: f64| {
        let (moved_far, tf) = {
            let eng = physics.peek();
            let mut st = eng.borrow_mut();
            if !st.dragging {
                return;
            }
            let far = st.move_drag(cx, cy, now_perf_ms());
            (far, st.format_transform())
        };
        apply_lightbox_transform_dom(&tf);
        if moved_far && !*did_drag.peek() {
            did_drag.set(true);
        }
    };

    let mut finish_drag_and_snap = move || {
        if *dragging.peek() {
            dragging.set(false);
            let eng = physics.peek().clone();
            eng.borrow_mut().end_drag(now_perf_ms());
            kick_lightbox_spring_raf(eng);
        }
    };

    let close_lightbox = move || {
        physics.peek().borrow_mut().reset_immediate();
        *GLOBAL_LIGHTBOX.write() = None;
    };

    let z = zoom();
    let is_dragging = dragging();
    let pct = (z * 100.0).round() as i32;
    let stage_cls = if is_dragging {
        "lightbox-stage is-panning"
    } else {
        "lightbox-stage can-pan"
    };
    let frame_style = if let Some((fw, fh)) = fit_box {
        format!("width: {fw:.0}px; height: {fh:.0}px;")
    } else {
        String::new()
    };
    let img_style = format!("transform: {};", physics.peek().borrow().format_transform());
    let clean_media_name = crate::storage::media_opt::strip_file_extension(&media.name);
    let dl_name = if clean_media_name.is_empty() {
        "chatw-image".to_string()
    } else {
        clean_media_name.clone()
    };
    let ai_toggle_active = media.ai_restored && !show_original();
    let media_for_toggle = media.clone();
    // Frame-0 active enhancing flag: eliminates the 1-2 frame raw blurry flash before `use_effect` starts!
    let is_lb_enhancing = upscaling_ai() || (media.auto_enhance_on_open && !media.ai_restored && can_toggle_ai);
    let is_lb_revealing = lb_revealing() && !is_lb_enhancing;
    let lb_pct_now = lb_progress().max(if is_lb_enhancing { 3 } else { 0 });
    let has_known_dims = media.width > 0 && media.height > 0;

    rsx! {
        div {
            class: "lightbox",
            role: "dialog",
            aria_modal: "true",
            aria_label: "Photo",
            tabindex: "-1",
            onmounted: move |ev| {
                spawn(async move {
                    let _ = ev.set_focus(true).await;
                    if !has_known_dims {
                        if let Some((bw, bh, v_w, v_h)) = measure_lightbox_bounds().await {
                            bounds.set((bw, bh, v_w, v_h));
                            physics.peek().borrow_mut().set_bounds(bw, bh, v_w, v_h);
                        }
                    }
                });
            },
            onkeydown: move |e| {
                match e.key() {
                    Key::Escape => {
                        e.prevent_default();
                        close_lightbox();
                    }
                    Key::Character(ref ch) if ch == "+" || ch == "=" => {
                        e.prevent_default();
                        set_zoom_clamped(zoom() + 0.25);
                    }
                    Key::Character(ref ch) if ch == "-" || ch == "_" => {
                        e.prevent_default();
                        set_zoom_clamped(zoom() - 0.25);
                    }
                    Key::Character(ref ch) if ch == "0" => {
                        e.prevent_default();
                        reset_view();
                    }
                    Key::Character(ref ch) if ch == "r" || ch == "R" => {
                        e.prevent_default();
                        apply_rotate();
                    }
                    _ => {}
                }
            },
            onwheel: move |e| {
                e.prevent_default();
                e.stop_propagation();
                let dy = e.delta().strip_units().y;
                if dy.abs() > 0.01 {
                    let factor = if dy < 0.0 { 1.15 } else { 1.0 / 1.15 };
                    set_zoom_clamped(zoom() * factor);
                }
            },

            // Pure symmetric 100vw x 100vh viewport-centered image stage
            div {
                class: "{stage_cls}",
                onclick: move |_| {
                    if did_drag() {
                        did_drag.set(false);
                        return;
                    }
                    close_lightbox();
                },
                onmousedown: move |e| {
                    let c = e.client_coordinates();
                    start_drag(c.x, c.y);
                },
                onmousemove: move |e| {
                    let c = e.client_coordinates();
                    handle_drag_move(c.x, c.y);
                },
                onmouseup: move |_| finish_drag_and_snap(),
                onmouseleave: move |_| finish_drag_and_snap(),

                {
                    let cur_display_src = if show_original() {
                        media.original_src.as_deref().unwrap_or(&media.src)
                    } else {
                        &media.src
                    };
                    let overlay_cls = if is_lb_revealing {
                        "ai-gen-overlay lightbox-ai-gen is-revealing"
                    } else {
                        "ai-gen-overlay lightbox-ai-gen"
                    };
                    rsx! {
                        div {
                            class: "lightbox-img-frame",
                            style: "{frame_style}",
                            onclick: move |e| e.stop_propagation(),
                            img {
                                class: if is_lb_enhancing { "lightbox-img is-ai-generating" } else { "lightbox-img" },
                                style: "{img_style}",
                                src: "{cur_display_src}",
                                alt: "Image",
                                draggable: "false",
                                "data-media-ref": "{crate::storage::extract_ref_id(&media.media_id).unwrap_or_default()}",
                                onload: move |_| {
                                    if !has_known_dims && *fallback_nat.peek() == (0, 0) {
                                        spawn(async move {
                                            if let Some((nw, nh)) = read_lightbox_natural_size().await {
                                                fallback_nat.set((nw, nh));
                                            }
                                            if let Some((bw, bh, v_w, v_h)) = measure_lightbox_bounds().await {
                                                bounds.set((bw, bh, v_w, v_h));
                                                physics.peek().borrow_mut().set_bounds(bw, bh, v_w, v_h);
                                            }
                                        });
                                    }
                                },
                                ondoubleclick: move |e| {
                                    e.stop_propagation();
                                    if zoom() > 1.05 {
                                        reset_view();
                                    } else {
                                        set_zoom_clamped(2.0);
                                    }
                                },
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    e.stop_propagation();
                                    let c = e.client_coordinates();
                                    start_drag(c.x, c.y);
                                },
                            }
                            if is_lb_enhancing || is_lb_revealing {
                                div {
                                    class: "{overlay_cls}",
                                    style: "{img_style}",
                                    aria_hidden: "true",
                                    img {
                                        class: "ai-gen-blur-img",
                                        src: "{media.original_src.as_deref().unwrap_or(&media.src)}",
                                        alt: "",
                                        draggable: "false",
                                    }
                                    div { class: "ai-gen-frost" }
                                    div { class: "ai-gen-aurora" }
                                    div { class: "ai-gen-dots-base" }
                                    div { class: "ai-gen-dots-wave" }
                                    span { class: "ai-gen-pct", "{lb_pct_now}%" }
                                }
                            }
                        }
                    }
                }
            }

            // Single borderless Telegram-style topbar: filename ONLY on the left, clean SVG icon controls on the right
            div { class: "lightbox-topbar",
                div { class: "lightbox-meta", onclick: move |e| e.stop_propagation(),
                    if !clean_media_name.is_empty() {
                        span { class: "lightbox-filename", title: "{clean_media_name}", "{clean_media_name}" }
                    }
                }
                div { class: "lightbox-actions", role: "toolbar", aria_label: "Photo", onclick: move |e| e.stop_propagation(),
                    if can_toggle_ai {
                        button {
                            r#type: "button",
                            class: if ai_toggle_active {
                                "lightbox-tool-btn lightbox-enhance-toggle is-active"
                            } else {
                                "lightbox-tool-btn lightbox-enhance-toggle"
                            },
                            disabled: is_lb_enhancing,
                            aria_label: "Chuyển đổi ảnh làm nét",
                            title: if !media.ai_restored {
                                "Làm nét ảnh"
                            } else if show_original() {
                                "Chuyển sang ảnh đã làm nét"
                            } else {
                                "Chuyển sang ảnh trước khi làm nét"
                            },
                            onclick: move |_| {
                                if media_for_toggle.ai_restored {
                                    show_original.set(!show_original());
                                } else {
                                    run_lb_enhance(media_for_toggle.clone());
                                }
                            },
                            EnhanceToggleIcon { size: 18 }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "lightbox-tool-btn lightbox-zoom-btn",
                        aria_label: "Zoom out",
                        title: "Zoom out",
                        disabled: z <= 1.01,
                        onclick: move |_| set_zoom_clamped(zoom() - 0.25),
                        ZoomOutIcon { size: 18 }
                    }
                    button {
                        r#type: "button",
                        class: "lightbox-zoom-readout",
                        aria_label: "Reset zoom",
                        title: "Reset zoom",
                        onclick: move |_| reset_view(),
                        "{pct}%"
                    }
                    button {
                        r#type: "button",
                        class: "lightbox-tool-btn lightbox-zoom-btn",
                        aria_label: "Zoom in",
                        title: "Zoom in",
                        disabled: z >= 3.99,
                        onclick: move |_| set_zoom_clamped(zoom() + 0.25),
                        ZoomInIcon { size: 18 }
                    }
                    button {
                        r#type: "button",
                        class: "lightbox-tool-btn lightbox-rotate",
                        aria_label: "Rotate image",
                        title: "Rotate image",
                        onclick: move |_| apply_rotate(),
                        RotateCwIcon { size: 18 }
                    }
                    a {
                        href: "{media.src}",
                        download: "{dl_name}",
                        class: "lightbox-tool-btn lightbox-download",
                        aria_label: "Download original",
                        title: "Download original",
                        DownloadMinimalIcon { size: 18 }
                    }
                    button {
                        r#type: "button",
                        class: "lightbox-tool-btn lightbox-close",
                        aria_label: "Close",
                        title: "Close",
                        onclick: move |_| close_lightbox(),
                        CloseIcon { size: 18 }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod spec080_lightbox_clamp_tests {
    use super::*;

    #[test]
    fn pan_locks_at_center_when_image_fits_viewport() {
        assert_eq!(clamp_lightbox_pan(300.0, -250.0, 1.0, 0, 960.0, 540.0, 1600.0, 960.0), (0.0, 0.0));
        // 150% of 960x540 = 1440x810, still fits inside 1600x960 -> must stay centered
        assert_eq!(clamp_lightbox_pan(500.0, 500.0, 1.5, 0, 960.0, 540.0, 1600.0, 960.0), (0.0, 0.0));
    }

    #[test]
    fn pan_clamps_to_exact_image_corners_when_zoomed() {
        // 200% of 960x540 = 1920x1080 in 1600x960 -> max pan = (160, 60)
        assert_eq!(clamp_lightbox_pan(2000.0, -2000.0, 2.0, 0, 960.0, 540.0, 1600.0, 960.0), (160.0, -60.0));
        assert_eq!(clamp_lightbox_pan(-2000.0, 2000.0, 2.0, 0, 960.0, 540.0, 1600.0, 960.0), (-160.0, 60.0));
    }

    #[test]
    fn pan_swaps_axes_on_90_degree_rotation() {
        // Rotated 90deg at 200%: effective 1080x1920 in 1600x960 -> max_x = 0, max_y = (1920 - 960)/2 = 480
        assert_eq!(clamp_lightbox_pan(500.0, 1000.0, 2.0, 90, 960.0, 540.0, 1600.0, 960.0), (0.0, 480.0));
    }

    #[test]
    fn spec112_precomputes_exact_final_4x_frame_from_frame_zero() {
        // 836x471 wire AVIF -> predicted 3344x1884 -> fits 1440x811 on 1600x960 and 1214x684 on 1320x860!
        let target = compute_lightbox_target_dimensions(836, 471, 1672, 941, true).unwrap();
        assert_eq!(target, (3344.0, 1884.0));
        assert_eq!(compute_lightbox_fit_box(target.0, target.1, 1600.0, 960.0), (1440.0, 811.0));
        assert_eq!(compute_lightbox_fit_box(target.0, target.1, 1320.0, 860.0), (1214.0, 684.0));
        // Exact same fit box when already restored to 3344x1884 (0.0px jump!)
        assert_eq!(compute_lightbox_fit_box(3344.0, 1884.0, 1600.0, 960.0), (1440.0, 811.0));
        assert_eq!(compute_lightbox_fit_box(3344.0, 1884.0, 1320.0, 860.0), (1214.0, 684.0));
    }

    #[test]
    fn spec112_preserves_native_size_for_sender_originals_spec078_and_spec101() {
        let s78 = compute_lightbox_target_dimensions(960, 540, 960, 540, false).unwrap();
        assert_eq!(compute_lightbox_fit_box(s78.0, s78.1, 1600.0, 960.0), (960.0, 540.0));
        let s101 = compute_lightbox_target_dimensions(800, 600, 800, 600, false).unwrap();
        assert_eq!(compute_lightbox_fit_box(s101.0, s101.1, 1280.0, 800.0), (800.0, 600.0));
    }

    #[test]
    fn spec112_chat_bubble_box_locks_landscape_and_portrait_geometry() {
        assert_eq!(compute_chat_image_box(836, 471), Some((352, 198)));
        assert_eq!(compute_chat_image_box(3344, 1884), Some((352, 198)));
        assert_eq!(compute_chat_image_box(800, 1600), Some((176, 352)));
        assert_eq!(compute_chat_image_box(200, 120), Some((200, 120)));
    }

    #[test]
    fn spec113_spring_carries_flick_momentum_and_settles_smoothly() {
        let mut eng = LightboxSpringEngine::default();
        eng.set_bounds(1200.0, 675.0, 1600.0, 960.0);
        // Fast flick +200px in 80ms -> release velocity = +2500 px/s
        eng.begin_drag(500.0, 400.0, 1000.0);
        eng.move_drag(600.0, 440.0, 1040.0);
        eng.move_drag(700.0, 480.0, 1080.0);
        eng.end_drag(1085.0);
        assert!(eng.vx > 2000.0, "expected real flick velocity, got {}", eng.vx);
        assert_eq!((eng.tx, eng.ty), (0.0, 0.0));

        // First 16ms frame MUST carry inertia further right (> 200px) instead of braking dead!
        assert!(eng.step(0.016));
        assert!(eng.x > 215.0, "expected physical inertia arc > 215px, got {}", eng.x);

        // Must smoothly return and settle cleanly at exact (0, 0) within 600ms!
        let mut t = 0.016;
        while eng.step(0.016) && t < 1.0 {
            t += 0.016;
        }
        assert!(t < 0.60, "spring took too long to settle: {t:.3}s");
        assert_eq!((eng.x, eng.y), (0.0, 0.0));
        assert_eq!(eng.format_transform(), "translate3d(0.0px, 0.0px, 0) scale(1.000) rotate(0deg)");
    }

    #[test]
    fn spec113_stationary_release_settles_under_420ms_for_spec101() {
        let mut eng = LightboxSpringEngine::default();
        eng.set_bounds(800.0, 600.0, 1280.0, 800.0);
        eng.begin_drag(400.0, 300.0, 1000.0);
        eng.move_drag(620.0, 440.0, 1050.0);
        // Held stationary for 100ms before release -> velocity decays to 0
        eng.end_drag(1150.0);
        assert_eq!((eng.vx, eng.vy), (0.0, 0.0));

        let mut t = 0.0;
        while eng.step(0.016) && t < 1.0 {
            t += 0.016;
        }
        assert!(t < 0.42, "stationary 220px release should settle < 420ms, took {t:.3}s");
        assert_eq!((eng.x, eng.y), (0.0, 0.0));
    }

    #[test]
    fn spec113_mid_flight_grab_catches_without_teleport() {
        let mut eng = LightboxSpringEngine::default();
        eng.set_bounds(1200.0, 675.0, 1600.0, 960.0);
        eng.begin_drag(500.0, 400.0, 1000.0);
        eng.move_drag(800.0, 400.0, 1050.0);
        eng.end_drag(1150.0);
        for _ in 0..5 {
            eng.step(0.016);
        }
        let mid_x = eng.x;
        assert!(mid_x > 40.0 && mid_x < 280.0, "expected mid-flight x, got {mid_x}");
        eng.begin_drag(600.0, 400.0, 1250.0);
        assert!((eng.x - mid_x).abs() < 1e-6);
        assert_eq!(eng.vx, 0.0);
    }
}
