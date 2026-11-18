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
    let q = query().trim().to_lowercase();
    let mine_shown: Vec<LibraryItem> = mine().into_iter().filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q)).collect();
    let builtin_shown: Vec<(&str, &str)> = BUILTIN_GIFS.iter().copied().filter(|(n, _)| q.is_empty() || n.to_lowercase().contains(&q)).collect();
    rsx! {
        div { class: "picker-header",
            div { class: "search-field",
                SearchIcon { size: 16 }
                input { r#type: "search", placeholder: "Search GIFs", aria_label: "Search GIFs", value: "{query}", oninput: move |e| query.set(e.value()) }
            }
        }
        div { class: "picker-body",
            div { class: "picker-section-title", "My GIFs" }
            if loading() {
                div { class: "picker-empty", "Loading" }
            } else if mine_shown.is_empty() {
                div { class: "sidebar-empty", "Add your own GIFs in Settings › Thư viện." }
            } else {
                div { class: "gif-grid",
                    for g in mine_shown {
                        {
                            let url = g.file_url.clone();
                            rsx! {
                                button { key: "{g.id}", r#type: "button", class: "gif-cell", title: "{g.name}", aria_label: "Send GIF {g.name}",
                                    onclick: move |_| on_select.call(url.clone()),
                                    img { src: "{platform::resolve_asset_url(&g.file_url)}", alt: "{g.name}", loading: "lazy" }
                                    span { class: "gif-name", "{g.name}" }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "picker-section-title", "ChatW Motion" }
            div { class: "gif-grid",
                for (name, url) in builtin_shown {
                    button { key: "{url}", r#type: "button", class: "gif-cell builtin", title: "{name}", aria_label: "Send GIF {name}",
                        onclick: move |_| on_select.call(url.to_string()),
                        img { src: "{url}", alt: "{name}", loading: "lazy" }
                        span { class: "gif-name", "{name}" }
                    }
                }
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
