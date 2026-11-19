//! Message composer shared by direct messages, groups and channels.
//!
//! Suggestions while typing:
//! * `@name`  → member / friend mentions
//! * `:smile` → emoji by English name
//! * `/short` → quick replies (Kinh doanh)
//! * short messages → matching stickers (e.g. "cảm ơn" → "Thank you")
//!
//! Pasting spreadsheet cells (tab separated) asks, or follows the saved choice:
//! send as an image, as a table message, or insert as plain text.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        emoji_data::CATEGORIES,
        icons::{CloseIcon, EmojiIcon, GifIcon, SendIcon, StickerIcon},
        media_picker::{MediaPicker, MediaTab},
        rich::ComposerTools,
    },
    models::StickerItem,
    platform,
    prefs::PrefsCtx,
    services::{openmoji, stickers::stickers_list_packs},
};

#[derive(Clone, Debug, PartialEq)]
pub struct MentionCandidate {
    pub username: String,
    pub display: String,
    pub avatar_url: Option<String>,
}

/// Every sticker the user can send, loaded once per session for suggestions.
static STICKERS: GlobalSignal<Option<Vec<StickerItem>>> = Signal::global(|| None);
/// The built-in OpenMoji catalogue, for "pizza" / "mèo" style suggestions.
static OPENMOJI: GlobalSignal<Option<std::rc::Rc<openmoji::Catalog>>> = Signal::global(|| None);

/// Extra trigger words for the built-in stickers (names stay English).
const STICKER_TRIGGERS: &[(&str, &[&str])] = &[
    ("hello", &["hello", "hi", "hey", "chao", "xin chao", "alo", "chao ban", "chao moi nguoi", "hi ban"]),
    ("wave", &["bye", "tam biet", "bai bai", "hello", "hi", "chao"]),
    ("love", &["love", "yeu", "thuong", "iu", "<3", "yeu qua", "thich qua", "love you", "yeu ban"]),
    ("love it", &["love", "yeu", "thuong", "iu", "<3", "yeu qua", "thich qua"]),
    ("heartbeat", &["love", "yeu", "thuong", "nho", "nho ban"]),
    ("lol", &["haha", "hihi", "hehe", "lol", "buon cuoi", "kkk", "=))", ":))", "cuoi", "hahaha"]),
    ("ok", &["ok", "oke", "okay", "duoc", "dong y", "yes", "u", "uk", "dc", "chuan", "okela"]),
    ("thumbs up", &["ok", "oke", "okay", "duoc", "dong y", "yes", "u", "uk", "dc", "chuan"]),
    ("wow", &["wow", "oa", "troi oi", "omg", "woa", "dinh", "qua dinh", "ghe"]),
    ("sad", &["buon", "sad", ":(", "chan", "that vong", "buon qua"]),
    ("cry", &["khoc", "cry", "huhu", "hu hu", "tui than", "khoc that"]),
    ("sleepy", &["buon ngu", "sleepy", "ngu ngon", "good night", "chuc ngu ngon", "gn", "zzz", "di ngu"]),
    ("sleep", &["ngu", "ngu ngon", "good night", "chuc ngu ngon", "gn", "zzz"]),
    ("party", &["party", "tiec", "sinh nhat", "happy birthday", "hpbd", "chuc mung sinh nhat"]),
    ("celebrate", &["chuc mung", "congrats", "yay", "mung qua", "tuyet voi"]),
    ("thanks", &["cam on", "thanks", "thank you", "tks", "thx", "ty", "cam on nhe", "cam on ban", "cam on nhieu"]),
    ("thank you", &["cam on", "thanks", "thank you", "tks", "thx", "ty", "cam on nhe", "cam on ban"]),
    ("hmm", &["hmm", "suy nghi", "de xem", "hmmm", "?", "sao ta", "that khong"]),
    ("angry", &["tuc", "gian", "angry", "buc", "buc minh", "hmph", "tuc qua"]),
    ("cool", &["cool", "ngau", "chat", "xin", "dinh cao", "pro"]),
    ("shy", &["ngai", "xau ho", "shy", "e then", "hihi ngai"]),
    ("yay", &["yay", "vui qua", "hoan ho", "tuyet", "tuyet voi", "yeah"]),
    ("sorry", &["xin loi", "sorry", "loi cua minh", "sr", "xl"]),
    ("bounce", &["vui", "yay", "hay qua", "vui qua"]),
    ("dizzy", &["chong mat", "met qua", "dizzy", "roi qua"]),
    ("typing", &["doi", "cho", "doi xiu", "wait", "dang go"]),
    ("rocket", &["nhanh", "go", "di thoi", "lets go", "let's go", "len"]),
];

/// Fold Vietnamese diacritics and case so "Cảm ơn" matches "cam on".
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => 'a',
            'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
            'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
            'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' => 'o',
            'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
            'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
            'đ' => 'd',
            other => other,
        })
        .collect()
}

fn words_match(text: &str, trigger: &str) -> bool {
    if text == trigger {
        return true;
    }
    // Whole-word containment for short messages ("ok nhé" → "ok").
    let padded = format!(" {text} ");
    padded.contains(&format!(" {trigger} "))
}

pub fn suggest_stickers(text: &str, all: &[StickerItem]) -> Vec<StickerItem> {
    let t = fold(text.trim());
    let t = t.trim_end_matches(['!', '.', '~']).trim().to_string();
    if t.is_empty() || t.chars().count() > 40 || t.starts_with('/') || t.starts_with(':') || t.starts_with('@') {
        return Vec::new();
    }
    let mut out: Vec<StickerItem> = Vec::new();
    for st in all {
        let name = fold(&st.name);
        let mut hit = !name.is_empty() && words_match(&t, &name);
        if !hit {
            if let Some((_, triggers)) = STICKER_TRIGGERS.iter().find(|(n, _)| *n == name) {
                hit = triggers.iter().any(|tr| words_match(&t, tr));
            }
        }
        if !hit {
            // Name words like "cat love" or "bear thanks" match their emotion word.
            hit = name.split_whitespace().last().is_some_and(|last| {
                STICKER_TRIGGERS.iter().any(|(n, triggers)| (*n == last || n.ends_with(last)) && triggers.iter().any(|tr| words_match(&t, tr)))
            });
        }
        if hit && !out.iter().any(|o| o.file_url == st.file_url) {
            out.push(st.clone());
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
enum Suggest {
    Mention(MentionCandidate),
    Emoji { emoji: String, name: String },
    Quick { shortcut: String, text: String },
}

/// The token being typed at the end of the input: (start byte index, token).
fn trailing_token(value: &str) -> Option<(usize, &str)> {
    let start = value.rfind(char::is_whitespace).map(|i| i + value[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)).unwrap_or(0);
    let tok = &value[start..];
    (!tok.is_empty()).then_some((start, tok))
}

fn build_suggestions(value: &str, mentions: &[MentionCandidate], quick: &[(String, String)], emoji_on: bool) -> Vec<Suggest> {
    let Some((start, tok)) = trailing_token(value) else {
        return Vec::new();
    };
    if let Some(q) = tok.strip_prefix('@') {
        let q = fold(q);
        return mentions
            .iter()
            .filter(|m| q.is_empty() || fold(&m.username).contains(&q) || fold(&m.display).contains(&q))
            .take(6)
            .cloned()
            .map(Suggest::Mention)
            .collect();
    }
    if start == 0 {
        if let Some(q) = tok.strip_prefix('/') {
            if !quick.is_empty() {
                let q = q.to_lowercase();
                return quick
                    .iter()
                    .filter(|(s, _)| s.to_lowercase().starts_with(&q))
                    .take(6)
                    .map(|(s, t)| Suggest::Quick { shortcut: s.clone(), text: t.clone() })
                    .collect();
            }
        }
    }
    if emoji_on {
        if let Some(q) = tok.strip_prefix(':') {
            if q.chars().count() >= 2 && q.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let q = q.to_lowercase();
                let mut out = Vec::new();
                for cat in CATEGORIES {
                    for (em, kw) in cat.items {
                        let name: String = kw.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                        if kw.split_whitespace().any(|w| w.starts_with(&q)) {
                            out.push(Suggest::Emoji { emoji: em.to_string(), name });
                            if out.len() >= 8 {
                                return out;
                            }
                        }
                    }
                }
                return out;
            }
        }
    }
    Vec::new()
}

fn apply_suggestion(value: &str, s: &Suggest) -> String {
    let start = trailing_token(value).map(|(i, _)| i).unwrap_or(value.len());
    match s {
        Suggest::Mention(m) => format!("{}@{} ", &value[..start], m.username),
        Suggest::Emoji { emoji, .. } => format!("{}{} ", &value[..start], emoji),
        Suggest::Quick { text, .. } => text.clone(),
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
struct PasteEvent {
    kind: String,
    #[serde(default)]
    b64: String,
    #[serde(default)]
    mime: String,
    #[serde(default)]
    text: String,
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
    /// Shown instead of the input when sending is not possible (blocked, no permission).
    #[props(default)] disabled_reason: Option<String>,
    #[props(default)] mentions: Vec<MentionCandidate>,
    #[props(default)] quick_replies: Vec<(String, String)>,
    /// Raw rich payloads (polls, notes, reminders, tables).
    #[props(default)] on_raw: Option<EventHandler<String>>,
    /// Images (picked or pasted): bytes + mime.
    #[props(default)] on_image: Option<EventHandler<(Vec<u8>, String)>>,
    #[props(default = true)] allow_poll: bool,
    #[props(default = true)] allow_note: bool,
    #[props(default = true)] allow_reminder: bool,
) -> Element {
    let prefs_ctx = try_use_context::<PrefsCtx>();
    let prefs = prefs_ctx.map(|c| c.prefs.read().clone()).unwrap_or_default();
    let mut picker = use_signal(|| Option::<MediaTab>::None);
    // A sticker's "Find similar" opens the sticker picker with its name as the search.
    use_effect(move || {
        if crate::components::sticker_picker::PICKER_REQUEST.read().is_some() {
            picker.set(Some(MediaTab::Sticker));
        }
    });
    let mut sel = use_signal(|| 0usize);
    let mut dismissed_for = use_signal(String::new);
    let mut pending_table = use_signal(|| Option::<String>::None);
    let mut remember_choice = use_signal(|| false);
    let mut notice = use_signal(|| Option::<String>::None);

    let can_send = !value.trim().is_empty() && !sending && !disabled;
    let id_for_emoji = input_id.clone();
    let value_for_emoji = value.clone();

    // Load stickers once for suggestions.
    let wants_stickers = prefs.sticker_suggest && on_sticker.is_some();
    use_effect(move || {
        if wants_stickers && STICKERS.peek().is_none() {
            *STICKERS.write() = Some(Vec::new());
            spawn(async move {
                if let Ok(packs) = stickers_list_packs().await {
                    *STICKERS.write() = Some(packs.into_iter().flat_map(|p| p.stickers).collect());
                }
            });
            spawn(async move {
                if let Ok(c) = openmoji::load().await {
                    *OPENMOJI.write() = Some(c);
                }
            });
        }
    });

    // Paste watcher: images and spreadsheet cells.
    let paste_id = input_id.clone();
    // Latest input value for the paste handler, without re-rendering on write.
    let mut paste_value = use_hook(|| CopyValue::new(String::new()));
    paste_value.set(value.clone());
    use_future(move || {
        let id = paste_id.clone();
        async move {
            let mut ev = platform::paste_watch(&id, true);
            while let Ok(p) = ev.recv::<PasteEvent>().await {
                match p.kind.as_str() {
                    "image" => {
                        if let Some(h) = on_image {
                            use base64::Engine;
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(p.b64.as_bytes()) {
                                h.call((bytes, p.mime.clone()));
                            }
                        } else {
                            notice.set(Some("Cuộc trò chuyện này chưa hỗ trợ gửi ảnh".into()));
                        }
                    }
                    "table" => {
                        let mode = prefs_ctx.map(|c| c.prefs.peek().paste_mode.clone()).unwrap_or_else(|| "ask".into());
                        match mode.as_str() {
                            "image" if on_image.is_some() => send_table_image(p.text.clone(), on_image, notice),
                            "table" if on_raw.is_some() => {
                                if let Some(h) = on_raw {
                                    h.call(format!("table:{}", p.text));
                                }
                            }
                            "text" => {
                                let cur = paste_value.peek().clone();
                                let id2 = id.clone();
                                let t = p.text.clone();
                                spawn(async move {
                                    let updated = platform::insert_text_in_input(&id2, &cur, &t).await;
                                    on_input.call(updated);
                                });
                            }
                            _ => pending_table.set(Some(p.text.clone())),
                        }
                    }
                    "error" => notice.set(Some("Ảnh lớn hơn 8 MB nên không dán được".into())),
                    _ => {}
                }
            }
        }
    });

    let suggestions: Vec<Suggest> = if disabled || dismissed_for() == value {
        Vec::new()
    } else {
        let mentions_on = prefs.mention_suggest;
        build_suggestions(&value, if mentions_on { &mentions } else { &[] }, &quick_replies, prefs.emoji_suggest)
    };
    let sticker_hits: Vec<StickerItem> = if suggestions.is_empty() && wants_stickers && !disabled && dismissed_for() != value {
        let mut hits = STICKERS.read().as_ref().map(|all| suggest_stickers(&value, all)).unwrap_or_default();
        if hits.len() < 10 {
            if let Some(c) = OPENMOJI.read().as_ref() {
                hits.extend(c.suggest(&value, 10 - hits.len()));
            }
        }
        hits
    } else {
        Vec::new()
    };
    let n_sugg = suggestions.len();
    let cur_sel = if n_sugg == 0 { 0 } else { sel().min(n_sugg - 1) };
    let sugg_for_keys = suggestions.clone();
    let value_for_keys = value.clone();
    let enter_sends = prefs.enter_to_send;

    rsx! {
        div { class: "composer",
            div { class: "composer-inner",
                if let Some(t) = picker() {
                    MediaPicker {
                        key: "{t as u8}",
                        initial_tab: t,
                        on_emoji: move |emoji: String| {
                            let cur = value_for_emoji.clone();
                            let id = id_for_emoji.clone();
                            spawn(async move {
                                let updated = platform::insert_text_in_input(&id, &cur, &emoji).await;
                                on_input.call(updated);
                            });
                        },
                        on_sticker: on_sticker.map(|h| EventHandler::new(move |url: String| { picker.set(None); h.call(url); })),
                        on_gif: on_sticker.map(|h| EventHandler::new(move |url: String| { picker.set(None); h.call(format!("gif:{url}")); })),
                        on_close: move |_| picker.set(None),
                    }
                }
                if let Some(msg) = notice() {
                    div { class: "composer-context composer-notice", role: "status",
                        span { class: "composer-context-text", "{msg}" }
                        button { r#type: "button", class: "icon-btn", aria_label: "Đóng thông báo", onclick: move |_| notice.set(None), CloseIcon { size: 16 } }
                    }
                }
                if let Some(tsv) = pending_table() {
                    {
                        let rows = tsv.lines().count();
                        let cols = tsv.lines().map(|l| l.split('\t').count()).max().unwrap_or(0);
                        let t_img = tsv.clone();
                        let t_tab = tsv.clone();
                        let t_txt = tsv.clone();
                        let id_txt = input_id.clone();
                        let cur_txt = value.clone();
                        let remember = move |mode: &str| {
                            if remember_choice() {
                                if let Some(c) = prefs_ctx {
                                    let m = mode.to_string();
                                    c.update(move |p| p.paste_mode = m);
                                }
                            }
                        };
                        rsx! {
                            div { class: "paste-choice", role: "dialog", aria_label: "Dán bảng tính",
                                div { class: "paste-choice-title", "Dán bảng tính {rows} dòng × {cols} cột" }
                                div { class: "paste-choice-sub", "Chọn cách gửi nội dung vừa dán" }
                                div { class: "paste-choice-actions",
                                    if on_image.is_some() {
                                        button { r#type: "button", class: "btn-secondary",
                                            onclick: move |_| { remember("image"); pending_table.set(None); send_table_image(t_img.clone(), on_image, notice); },
                                            "Dạng ảnh"
                                        }
                                    }
                                    if on_raw.is_some() {
                                        button { r#type: "button", class: "btn-secondary",
                                            onclick: move |_| { remember("table"); pending_table.set(None); if let Some(h) = on_raw { h.call(format!("table:{t_tab}")); } },
                                            "Dạng bảng"
                                        }
                                    }
                                    button { r#type: "button", class: "btn-secondary",
                                        onclick: move |_| {
                                            remember("text");
                                            pending_table.set(None);
                                            let id = id_txt.clone();
                                            let cur = cur_txt.clone();
                                            let t = t_txt.clone();
                                            spawn(async move {
                                                let updated = platform::insert_text_in_input(&id, &cur, &t).await;
                                                on_input.call(updated);
                                            });
                                        },
                                        "Dạng chữ"
                                    }
                                    button { r#type: "button", class: "btn-ghost", onclick: move |_| pending_table.set(None), "Hủy" }
                                }
                                label { class: "paste-choice-remember",
                                    input { r#type: "checkbox", checked: remember_choice(), onchange: move |e| remember_choice.set(e.checked()) }
                                    span { "Ghi nhớ lựa chọn" }
                                }
                            }
                        }
                    }
                }
                if !suggestions.is_empty() {
                    div { class: "suggest-list", role: "listbox", aria_label: "Gợi ý",
                        for (i, sg) in suggestions.iter().cloned().enumerate() {
                            {
                                let v = value.clone();
                                let sg2 = sg.clone();
                                let active = i == cur_sel;
                                rsx! {
                                    button {
                                        key: "{i}",
                                        r#type: "button",
                                        role: "option",
                                        aria_selected: "{active}",
                                        class: if active { "suggest-item active" } else { "suggest-item" },
                                        onmousedown: move |e| e.prevent_default(),
                                        onclick: move |_| { on_input.call(apply_suggestion(&v, &sg2)); sel.set(0); platform::focus_chat_input(); },
                                        match sg {
                                            Suggest::Mention(m) => rsx! {
                                                Avatar { name: m.username.clone(), avatar_url: m.avatar_url.clone(), size: 24 }
                                                span { class: "suggest-main", "{m.display}" }
                                                span { class: "suggest-sub", "@{m.username}" }
                                            },
                                            Suggest::Emoji { emoji, name } => rsx! {
                                                span { class: "suggest-emoji", "{emoji}" }
                                                span { class: "suggest-main", ":{name}" }
                                            },
                                            Suggest::Quick { shortcut, text } => rsx! {
                                                span { class: "suggest-main", "/{shortcut}" }
                                                span { class: "suggest-sub", "{text}" }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "suggest-hint", "↑ ↓ chọn · Enter hoặc Tab để chèn · Esc để ẩn" }
                    }
                }
                if !sticker_hits.is_empty() {
                    div { class: "sticker-suggest", aria_label: "Sticker gợi ý",
                        for st in sticker_hits {
                            {
                                let url = st.file_url.clone();
                                rsx! {
                                    button {
                                        key: "{st.id}",
                                        r#type: "button",
                                        class: "sticker-suggest-cell",
                                        title: "{st.name}",
                                        aria_label: "Gửi sticker {st.name}",
                                        onclick: move |_| {
                                            if let Some(h) = on_sticker {
                                                on_input.call(String::new());
                                                h.call(url.clone());
                                            }
                                        },
                                        img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{st.name}", loading: "lazy" }
                                    }
                                }
                            }
                        }
                        button { r#type: "button", class: "icon-btn sticker-suggest-close", aria_label: "Ẩn gợi ý sticker", title: "Ẩn gợi ý",
                            onclick: { let v = value.clone(); move |_| dismissed_for.set(v.clone()) },
                            CloseIcon { size: 14 }
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
                if let Some(reason) = disabled_reason.as_ref() {
                    div { class: "composer-disabled", role: "status", "{reason}" }
                } else {
                    {
                        let mod_btns = crate::components::plugins_view::enabled_composer_buttons();
                        if !mod_btns.is_empty() && input_id != "bot-chat-input" {
                            rsx! {
                                div { class: "plugin-composer-bar",
                                    for (blabel, bcontent, pname) in mod_btns {
                                        {
                                            let cur_v = value.clone();
                                            let bc = bcontent.clone();
                                            rsx! {
                                                button {
                                                    key: "{pname}-{blabel}",
                                                    r#type: "button",
                                                    class: "plugin-cmd-chip",
                                                    title: "Plugin: {pname}",
                                                    onclick: move |_| {
                                                        let out = crate::components::plugins_view::try_run_plugin_command(&bc, "Bạn")
                                                            .unwrap_or_else(|| if cur_v.is_empty() { bc.clone() } else { format!("{cur_v} {bc}") });
                                                        on_input.call(out);
                                                        platform::focus_chat_input();
                                                    },
                                                    "{blabel}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                    div { class: "composer-box",
                        if on_raw.is_some() || on_image.is_some() {
                            ComposerTools {
                                input_id: input_id.clone(),
                                on_raw: move |raw: String| if let Some(h) = on_raw { h.call(raw) },
                                on_image: move |img: (Vec<u8>, String)| {
                                    if let Some(h) = on_image { h.call(img) } else { notice.set(Some("Cuộc trò chuyện này chưa hỗ trợ gửi ảnh".into())) }
                                },
                                allow_poll: allow_poll && on_raw.is_some(),
                                allow_note: allow_note && on_raw.is_some(),
                                allow_reminder: allow_reminder && on_raw.is_some(),
                            }
                        }
                        button {
                            r#type: "button",
                            class: if picker() == Some(MediaTab::Emoji) { "icon-btn active" } else { "icon-btn" },
                            title: "Emoji",
                            aria_label: "Chọn emoji",
                            aria_expanded: "{picker().is_some()}",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                picker.set(if picker() == Some(MediaTab::Emoji) { None } else { Some(MediaTab::Emoji) });
                            },
                            EmojiIcon {}
                        }
                        textarea {
                            id: "{input_id}",
                            class: "composer-input",
                            rows: "1",
                            placeholder: "{placeholder}",
                            aria_label: "{placeholder}",
                            aria_autocomplete: "list",
                            disabled: disabled,
                            value: "{value}",
                            oninput: move |evt| { sel.set(0); on_input.call(evt.value()) },
                            onkeydown: move |evt| {
                                let key = evt.key();
                                if !sugg_for_keys.is_empty() {
                                    match key {
                                        Key::ArrowDown => { evt.prevent_default(); sel.set((cur_sel + 1) % n_sugg); return; }
                                        Key::ArrowUp => { evt.prevent_default(); sel.set((cur_sel + n_sugg - 1) % n_sugg); return; }
                                        Key::Tab | Key::Enter if !evt.modifiers().shift() => {
                                            evt.prevent_default();
                                            on_input.call(apply_suggestion(&value_for_keys, &sugg_for_keys[cur_sel]));
                                            sel.set(0);
                                            return;
                                        }
                                        Key::Escape => { evt.prevent_default(); dismissed_for.set(value_for_keys.clone()); return; }
                                        _ => {}
                                    }
                                }
                                let send_combo = if enter_sends { !evt.modifiers().shift() } else { evt.modifiers().ctrl() || evt.modifiers().meta() };
                                if key == Key::Enter && send_combo && !evt.is_composing() {
                                    evt.prevent_default();
                                    on_send.call(());
                                } else if key == Key::Escape {
                                    picker.set(None);
                                    pending_table.set(None);
                                }
                            },
                        }
                        if on_sticker.is_some() {
                            button {
                                r#type: "button",
                                class: if picker() == Some(MediaTab::Gif) { "icon-btn active" } else { "icon-btn" },
                                title: "GIF",
                                aria_label: "Chọn GIF",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    picker.set(if picker() == Some(MediaTab::Gif) { None } else { Some(MediaTab::Gif) });
                                },
                                GifIcon {}
                            }
                            button {
                                r#type: "button",
                                class: if picker() == Some(MediaTab::Sticker) { "icon-btn active" } else { "icon-btn" },
                                title: "Sticker",
                                aria_label: "Chọn sticker",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    picker.set(if picker() == Some(MediaTab::Sticker) { None } else { Some(MediaTab::Sticker) });
                                },
                                StickerIcon {}
                            }
                        }
                        button {
                            r#type: "button",
                            class: "send-btn",
                            title: "Gửi",
                            aria_label: "Gửi",
                            disabled: !can_send,
                            onclick: move |_| on_send.call(()),
                            SendIcon {}
                        }
                    }
                    div { class: "composer-hint hide-mobile",
                        if enter_sends {
                            span { class: "kbd", "Enter" } " gửi · " span { class: "kbd", "Shift + Enter" } " xuống dòng · " span { class: "kbd", "@" } " nhắc tên · " span { class: "kbd", ":" } " emoji"
                        } else {
                            span { class: "kbd", "Ctrl + Enter" } " gửi · " span { class: "kbd", "Enter" } " xuống dòng · " span { class: "kbd", "@" } " nhắc tên"
                        }
                    }
                }
            }
        }
    }
}

fn send_table_image(tsv: String, on_image: Option<EventHandler<(Vec<u8>, String)>>, mut notice: Signal<Option<String>>) {
    let Some(h) = on_image else {
        return;
    };
    spawn(async move {
        match platform::render_table_png(&tsv).await {
            Ok(bytes) => h.call((bytes, "image/png".to_string())),
            Err(_) => notice.set(Some("Không tạo được ảnh từ bảng tính".into())),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(name: &str) -> StickerItem {
        StickerItem { id: name.into(), pack_id: "p".into(), name: name.into(), file_url: format!("/stickers/{name}.svg"), is_animated: false, created_at: String::new() }
    }

    #[test]
    fn folds_vietnamese() {
        assert_eq!(fold("Cảm Ơn Nhé"), "cam on nhe");
        assert_eq!(fold("Đẹp"), "dep");
    }

    #[test]
    fn sticker_suggestions_match_triggers() {
        let all = vec![st("Thank you"), st("Hello"), st("Sad")];
        let hits = suggest_stickers("Cảm ơn", &all);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Thank you");
        assert!(suggest_stickers("một câu rất dài không liên quan gì đến sticker cả đâu nhé bạn ơi", &all).is_empty());
        assert_eq!(suggest_stickers("hello!", &all)[0].name, "Hello");
    }

    #[test]
    fn mention_and_quick_suggestions() {
        let m = vec![MentionCandidate { username: "ngson".into(), display: "Sơn".into(), avatar_url: None }];
        let s = build_suggestions("chào @ng", &m, &[], true);
        assert_eq!(s.len(), 1);
        assert_eq!(apply_suggestion("chào @ng", &s[0]), "chào @ngson ");
        let q = vec![("gia".to_string(), "Giá sản phẩm là 100k".to_string())];
        let s = build_suggestions("/gi", &[], &q, true);
        assert_eq!(apply_suggestion("/gi", &s[0]), "Giá sản phẩm là 100k");
        assert!(build_suggestions("a /gi", &[], &q, true).is_empty());
    }

    #[test]
    fn emoji_suggestions() {
        let s = build_suggestions("hi :grin", &[], &[], true);
        assert!(!s.is_empty());
        assert!(build_suggestions("hi :grin", &[], &[], false).is_empty());
    }
}
