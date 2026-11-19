//! Tabbed media picker: Emoji · Sticker · GIF (names are always English).

use dioxus::prelude::*;

use crate::{
    components::{emoji_picker::EmojiPicker, icons::SearchIcon, sticker_picker::StickerPicker},
    models::LibraryItem,
    platform,
    services::library,
};

/// Built-in animated reactions shipped with the client (usable as GIFs).
pub const BUILTIN_GIFS: [(&str, &str); 10] = [
    ("Wave", "/stickers/anim/wave.svg"),
    ("Heartbeat", "/stickers/anim/heartbeat.svg"),
    ("Bounce", "/stickers/anim/bounce.svg"),
    ("Dizzy", "/stickers/anim/dizzy.svg"),
    ("Celebrate", "/stickers/anim/celebrate.svg"),
    ("Typing", "/stickers/anim/typing.svg"),
    ("Cry", "/stickers/anim/cry.svg"),
    ("Angry", "/stickers/anim/angry.svg"),
    ("Sleep", "/stickers/anim/sleep.svg"),
    ("Rocket", "/stickers/anim/rocket.svg"),
];

#[derive(Clone, Copy, PartialEq)]
pub enum MediaTab {
    Emoji,
    Sticker,
    Gif,
}

#[component]
fn GifPane(on_select: EventHandler<String>) -> Element {
    let mut mine = use_signal(Vec::<LibraryItem>::new);
    let mut loading = use_signal(|| true);
    let mut query = use_signal(String::new);
    use_effect(move || {
        spawn(async move {
            if let Ok(list) = library::gifs_list().await {
                mine.set(list);
            }
            loading.set(false);
        });
    });
    let prefs_ctx = try_consume_context::<crate::prefs::PrefsCtx>();
    let fav_gifs: Vec<String> = prefs_ctx.map(|c| c.prefs.read().fav_gifs.clone()).unwrap_or_default();
    let toggle = move |url: String| {
        if let Some(c) = prefs_ctx {
            c.update(move |p| {
                if p.fav_gifs.contains(&url) {
                    p.fav_gifs.retain(|u| u != &url);
                } else {
                    p.fav_gifs.insert(0, url.clone());
                }
            });
        }
    };
    let q = query().trim().to_lowercase();
    let mine_shown: Vec<LibraryItem> = mine().into_iter().filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q)).collect();
    let builtin_shown: Vec<(&str, &str)> = BUILTIN_GIFS.iter().copied().filter(|(n, _)| q.is_empty() || n.to_lowercase().contains(&q)).collect();
    let fx_shown: Vec<(&str, &str)> = crate::components::fx_gifs::FX_GIFS.iter().copied().filter(|(n, _)| q.is_empty() || n.to_lowercase().contains(&q)).collect();
    rsx! {
        div { class: "picker-header",
            div { class: "search-field",
                SearchIcon { size: 16 }
                input { r#type: "search", placeholder: "Search GIFs", aria_label: "Search GIFs", value: "{query}", oninput: move |e| query.set(e.value()) }
            }
        }
        div { class: "picker-body",
            if q.is_empty() && !fav_gifs.is_empty() {
                div { class: "picker-section-title", "★ Favorite GIFs" }
                div { class: "gif-grid",
                    for u in fav_gifs.clone() {
                        {
                            let (send, un) = (u.clone(), u.clone());
                            rsx! {
                                div { key: "fav-{u}", class: "sticker-cell-wrap",
                                    button { r#type: "button", class: "gif-cell", title: "Send GIF", aria_label: "Send favorite GIF",
                                        onclick: move |_| on_select.call(send.clone()),
                                        img { src: "{platform::resolve_asset_url(&u)}", alt: "Favorite GIF", loading: "lazy" }
                                    }
                                    button { r#type: "button", class: "sticker-fav on", aria_pressed: "true", title: "Remove from Favorite GIFs", aria_label: "Remove from Favorite GIFs",
                                        onclick: move |e| { e.stop_propagation(); toggle(un.clone()) }, "★" }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "picker-section-title", "My GIFs" }
            if loading() {
                div { class: "picker-empty", "Loading" }
            } else if mine_shown.is_empty() {
                div { class: "sidebar-empty", "Add your own GIFs in Settings › Thư viện." }
            } else {
                div { class: "gif-grid",
                    for g in mine_shown {
                        {
                            let fav = fav_gifs.contains(&g.file_url);
                            rsx! {
                                GifCell { key: "{g.id}", name: g.name.clone(), url: g.file_url.clone(), builtin: false, fav, on_select, on_fav: move |u: String| toggle(u) }
                            }
                        }
                    }
                }
            }
            if !fx_shown.is_empty() {
                div { class: "picker-section-title", "Reactions" }
                div { class: "gif-grid",
                    for (name, url) in fx_shown {
                        GifCell { key: "{url}", name: name.to_string(), url: url.to_string(), builtin: true, fav: fav_gifs.iter().any(|f| f == url), on_select, on_fav: move |u: String| toggle(u) }
                    }
                }
            }
            div { class: "picker-section-title", "ChatW Motion" }
            div { class: "gif-grid",
                for (name, url) in builtin_shown {
                    GifCell { key: "{url}", name: name.to_string(), url: url.to_string(), builtin: true, fav: fav_gifs.iter().any(|f| f == url), on_select, on_fav: move |u: String| toggle(u) }
                }
            }
        }
    }
}

/// One GIF with a ☆ Favorite GIFs toggle.
#[component]
fn GifCell(name: String, url: String, builtin: bool, fav: bool, on_select: EventHandler<String>, on_fav: EventHandler<String>) -> Element {
    let (send, f) = (url.clone(), url.clone());
    rsx! {
        div { class: "sticker-cell-wrap",
            button { r#type: "button", class: if builtin { "gif-cell builtin" } else { "gif-cell" }, title: "{name}", aria_label: "Send GIF {name}",
                onclick: move |_| on_select.call(send.clone()),
                img { src: "{platform::resolve_asset_url(&url)}", alt: "{name}", loading: "lazy" }
                span { class: "gif-name", "{name}" }
            }
            button { r#type: "button", class: if fav { "sticker-fav on" } else { "sticker-fav" }, aria_pressed: "{fav}",
                title: if fav { "Remove from Favorite GIFs" } else { "Add to Favorite GIFs" },
                aria_label: if fav { "Remove {name} from Favorite GIFs" } else { "Add {name} to Favorite GIFs" },
                onclick: move |e| { e.stop_propagation(); on_fav.call(f.clone()) },
                if fav { "★" } else { "☆" }
            }
        }
    }
}

#[component]
pub fn MediaPicker(
    initial_tab: MediaTab,
    on_emoji: EventHandler<String>,
    #[props(default)] on_sticker: Option<EventHandler<String>>,
    #[props(default)] on_gif: Option<EventHandler<String>>,
    on_close: EventHandler<()>,
) -> Element {
    let mut tab = use_signal(|| initial_tab);
    let mut custom = use_signal(Vec::<(String, String)>::new);
    use_effect(move || {
        spawn(async move {
            if let Ok(list) = library::emojis_list().await {
                custom.set(list.into_iter().map(|e| (e.name, e.file_url)).collect());
            }
        });
    });
    rsx! {
        div {
            class: "picker-popover media-picker",
            role: "dialog",
            aria_label: "Emoji, stickers and GIFs",
            onclick: move |e| e.stop_propagation(),
            onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
            div { class: "media-tabs", role: "tablist",
                if on_gif.is_some() {
                    button { r#type: "button", role: "tab", aria_selected: "{tab() == MediaTab::Gif}", class: if tab() == MediaTab::Gif { "media-tab active" } else { "media-tab" }, onclick: move |_| tab.set(MediaTab::Gif), "GIF" }
                }
                if on_sticker.is_some() {
                    button { r#type: "button", role: "tab", aria_selected: "{tab() == MediaTab::Sticker}", class: if tab() == MediaTab::Sticker { "media-tab active" } else { "media-tab" }, onclick: move |_| tab.set(MediaTab::Sticker), "Sticker" }
                }
                button { r#type: "button", role: "tab", aria_selected: "{tab() == MediaTab::Emoji}", class: if tab() == MediaTab::Emoji { "media-tab active" } else { "media-tab" }, onclick: move |_| tab.set(MediaTab::Emoji), "Emoji" }
            }
            match tab() {
                MediaTab::Emoji => rsx! { EmojiPicker { embedded: true, custom_emojis: custom(), on_select: move |e| on_emoji.call(e), on_close: move |_| on_close.call(()) } },
                MediaTab::Sticker => rsx! {
                    StickerPicker { on_select: move |u: String| if let Some(h) = on_sticker { h.call(u) }, _on_close: Some(EventHandler::new(move |_| on_close.call(()))) }
                },
                MediaTab::Gif => rsx! { GifPane { on_select: move |u: String| if let Some(h) = on_gif { h.call(u) } } },
            }
        }
    }
}
