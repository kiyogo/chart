use dioxus::prelude::*;

use crate::{
    components::{
        emoji_data::{CATEGORIES, SKIN_TONES, with_skin_tone},
        icons::{ClockIcon, SearchIcon},
    },
    platform,
};

const RECENT_KEY: &str = "chatw_recent_emoji";
const TONE_KEY: &str = "chatw_skin_tone";
const MAX_RECENT: usize = 32;

fn load_recent() -> Vec<String> {
    platform::storage_get(RECENT_KEY)
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

fn push_recent(emoji: &str) -> Vec<String> {
    let mut list = load_recent();
    list.retain(|e| e != emoji);
    list.insert(0, emoji.to_string());
    list.truncate(MAX_RECENT);
    if let Ok(json) = serde_json::to_string(&list) {
        platform::storage_set(RECENT_KEY, &json);
    }
    list
}

/// Strip Vietnamese diacritics so "cười" matches keyword "cuoi".
fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => 'e',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => 'u',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            other => other,
        })
        .collect()
}

fn keyword_label(keywords: &str) -> String {
    keywords.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

/// Full emoji picker: search (EN/VI, diacritic-insensitive), categories, recents,
/// skin tones, and arrow-key navigation across the grid.
#[component]
pub fn EmojiPicker(
    on_select: EventHandler<String>,
    #[props(default)] on_close: Option<EventHandler<()>>,
    #[props(default)] align_right: bool,
) -> Element {
    let mut query = use_signal(String::new);
    let mut recent = use_signal(load_recent);
    let mut tone = use_signal(|| platform::storage_get(TONE_KEY).unwrap_or_default());
    let mut hovered = use_signal(|| Option::<(String, String)>::None);
    let mut active_cat = use_signal(|| if load_recent().is_empty() { "smileys" } else { "recent" });

    use_effect(move || {
        let _ = dioxus::document::eval(
            "setTimeout(() => { const el = document.getElementById('emoji-search'); if (el) el.focus(); }, 0);",
        );
    });

    let mut choose = move |emoji: String| {
        recent.set(push_recent(&emoji));
        on_select.call(emoji);
    };

    let q = fold(query().trim());
    let tone_now = tone();

    let results: Vec<(&'static str, &'static str)> = if q.is_empty() {
        Vec::new()
    } else {
        CATEGORIES
            .iter()
            .flat_map(|c| c.items.iter().copied())
            .filter(|(_, kw)| q.split_whitespace().all(|part| kw.split_whitespace().any(|w| w.starts_with(part))))
            .take(120)
            .collect()
    };

    let cls = if align_right { "picker-popover align-right" } else { "picker-popover" };

    rsx! {
        div {
            class: "{cls}",
            role: "dialog",
            aria_label: "Chọn emoji",
            onclick: move |evt| evt.stop_propagation(),
            onkeydown: move |evt| {
                match evt.key() {
                    Key::Escape => {
                        evt.prevent_default();
                        if let Some(h) = on_close { h.call(()); }
                    }
                    Key::ArrowRight | Key::ArrowLeft | Key::ArrowDown | Key::ArrowUp => {
                        let dir = match evt.key() {
                            Key::ArrowRight => "right",
                            Key::ArrowLeft => "left",
                            Key::ArrowDown => "down",
                            _ => "up",
                        };
                        evt.prevent_default();
                        let _ = dioxus::document::eval(&format!(r#"
                            (function() {{
                                const btns = [...document.querySelectorAll('.picker-popover .emoji-btn')];
                                if (!btns.length) return;
                                let i = btns.indexOf(document.activeElement);
                                if (i < 0) {{ btns[0].focus(); return; }}
                                const top = btns[i].offsetTop;
                                const cols = btns.filter(b => b.offsetTop === top && b.parentElement === btns[i].parentElement).length || 8;
                                const d = '{dir}';
                                if (d === 'right') i = Math.min(btns.length - 1, i + 1);
                                if (d === 'left') i = Math.max(0, i - 1);
                                if (d === 'down') i = Math.min(btns.length - 1, i + cols);
                                if (d === 'up') i = Math.max(0, i - cols);
                                btns[i].focus();
                                btns[i].scrollIntoView({{ block: 'nearest' }});
                            }})();
                        "#));
                    }
                    _ => {}
                }
            },

            div { class: "picker-header",
                div { class: "search-field",
                    SearchIcon { size: 16 }
                    input {
                        id: "emoji-search",
                        r#type: "search",
                        placeholder: "Tìm emoji (ví dụ: cười, tim, ok)",
                        aria_label: "Tìm emoji",
                        autocomplete: "off",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                e.prevent_default();
                                let qf = fold(query().trim());
                                if !qf.is_empty() {
                                    let first = CATEGORIES
                                        .iter()
                                        .flat_map(|c| c.items.iter())
                                        .find(|(_, kw)| qf.split_whitespace().all(|p| kw.split_whitespace().any(|w| w.starts_with(p))))
                                        .map(|(e, _)| with_skin_tone(e, &tone()));
                                    if let Some(em) = first { choose(em); }
                                }
                            }
                        },
                    }
                }
            }

            if q.is_empty() {
                div { class: "picker-tabs", role: "tablist",
                    button {
                        r#type: "button",
                        class: if active_cat() == "recent" { "picker-tab active" } else { "picker-tab" },
                        title: "Gần đây",
                        aria_label: "Gần đây",
                        onclick: move |_| {
                            active_cat.set("recent");
                            let _ = dioxus::document::eval("const el = document.getElementById('emoji-cat-recent'); if (el) el.scrollIntoView({block: 'start'});");
                        },
                        ClockIcon { size: 18 }
                    }
                    for cat in CATEGORIES.iter() {
                        button {
                            key: "{cat.id}",
                            r#type: "button",
                            class: if active_cat() == cat.id { "picker-tab active" } else { "picker-tab" },
                            title: "{cat.label}",
                            aria_label: "{cat.label}",
                            onclick: move |_| {
                                active_cat.set(cat.id);
                                let _ = dioxus::document::eval(&format!(
                                    "const el = document.getElementById('emoji-cat-{}'); if (el) el.scrollIntoView({{block: 'start'}});",
                                    cat.id
                                ));
                            },
                            "{cat.icon}"
                        }
                    }
                }
            }

            div { class: "picker-body",
                if !q.is_empty() {
                    if results.is_empty() {
                        div { class: "picker-empty", "Không tìm thấy emoji phù hợp" }
                    } else {
                        div { class: "picker-section-title", "Kết quả" }
                        div { class: "emoji-grid",
                            for (em, kw) in results {
                                {
                                    let shown = with_skin_tone(em, &tone_now);
                                    let pick = shown.clone();
                                    let hover = (shown.clone(), keyword_label(kw));
                                    rsx! {
                                        button {
                                            key: "s-{em}",
                                            r#type: "button",
                                            class: "emoji-btn",
                                            aria_label: "{kw}",
                                            onmouseenter: move |_| hovered.set(Some(hover.clone())),
                                            onclick: move |_| choose(pick.clone()),
                                            "{shown}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { id: "emoji-cat-recent",
                        div { class: "picker-section-title", "Gần đây" }
                        if recent.read().is_empty() {
                            div { class: "sidebar-empty", "Emoji bạn dùng sẽ xuất hiện ở đây" }
                        } else {
                            div { class: "emoji-grid",
                                for em in recent() {
                                    {
                                        let pick = em.clone();
                                        rsx! {
                                            button {
                                                key: "r-{em}",
                                                r#type: "button",
                                                class: "emoji-btn",
                                                aria_label: "{em}",
                                                onclick: move |_| choose(pick.clone()),
                                                "{em}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    for cat in CATEGORIES.iter() {
                        div { key: "{cat.id}", id: "emoji-cat-{cat.id}",
                            div { class: "picker-section-title", "{cat.label}" }
                            div { class: "emoji-grid",
                                for (em, kw) in cat.items.iter().copied() {
                                    {
                                        let shown = with_skin_tone(em, &tone_now);
                                        let pick = shown.clone();
                                        let hover = (shown.clone(), keyword_label(kw));
                                        rsx! {
                                            button {
                                                key: "{em}",
                                                r#type: "button",
                                                class: "emoji-btn",
                                                aria_label: "{kw}",
                                                onmouseenter: move |_| hovered.set(Some(hover.clone())),
                                                onclick: move |_| choose(pick.clone()),
                                                "{shown}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "picker-footer",
                if let Some((em, label)) = hovered() {
                    span { class: "picker-footer-emoji", "{em}" }
                    span { class: "truncate", "{label}" }
                } else {
                    span { class: "muted", "Mũi tên để di chuyển, Enter để chọn" }
                }
                div { class: "skin-tones", role: "radiogroup", aria_label: "Màu da",
                    for (modifier, sample) in SKIN_TONES {
                        button {
                            key: "{sample}",
                            r#type: "button",
                            class: if tone() == modifier { "skin-tone-btn active" } else { "skin-tone-btn" },
                            aria_label: "Màu da {sample}",
                            onclick: move |_| {
                                tone.set(modifier.to_string());
                                platform::storage_set(TONE_KEY, modifier);
                            },
                            "{sample}"
                        }
                    }
                }
            }
        }
    }
}
