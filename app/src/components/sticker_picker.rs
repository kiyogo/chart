use dioxus::prelude::*;

use crate::{models::StickerPackItem, platform, services::stickers::stickers_list_packs};

/// Sticker picker body (rendered inside a `.picker-popover`). Lists the user's
/// real sticker packs from `/api/stickers`; selecting sends the sticker URL.
#[component]
pub fn StickerPicker(
    on_select: EventHandler<String>,
    #[props(default)] _on_close: Option<EventHandler<()>>,
) -> Element {
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut active_pack_id = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);
    let mut load_error = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            match stickers_list_packs().await {
                Ok(list) => {
                    if let Some(first) = list.first() {
                        active_pack_id.set(Some(first.id.clone()));
                    }
                    packs.set(list);
                }
                Err(_) => load_error.set(true),
            }
            is_loading.set(false);
        });
    });

    let active_pack = packs().into_iter().find(|p| Some(&p.id) == active_pack_id().as_ref());

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100%; min-height: 0;",
            onclick: move |evt| evt.stop_propagation(),
            onkeydown: move |evt| if evt.key() == Key::Escape { if let Some(h) = _on_close { h.call(()) } },
            div { class: "picker-header",
                span { class: "details-section-title", "Sticker" }
            }
            if !packs.read().is_empty() {
                div { class: "picker-tabs", role: "tablist",
                    for p in packs() {
                        {
                            let pid = p.id.clone();
                            let is_active = active_pack_id().as_deref() == Some(pid.as_str());
                            rsx! {
                                button {
                                    key: "{p.id}",
                                    r#type: "button",
                                    class: if is_active { "picker-tab text active" } else { "picker-tab text" },
                                    onclick: move |_| active_pack_id.set(Some(pid.clone())),
                                    "{p.name}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "picker-body",
                if is_loading() {
                    div { class: "picker-empty", "Đang tải sticker" }
                } else if load_error() {
                    div { class: "picker-empty", "Không tải được danh sách sticker" }
                } else if let Some(pack) = active_pack {
                    if pack.stickers.is_empty() {
                        div { class: "picker-empty", "Bộ sticker này chưa có sticker nào" }
                    } else {
                        div { class: "sticker-grid",
                            for st in pack.stickers {
                                {
                                    let url = st.file_url.clone();
                                    let resolved = platform::resolve_asset_url(&url);
                                    let title_str = if st.name.is_empty() { "Sticker".to_string() } else { st.name.clone() };
                                    rsx! {
                                        button {
                                            key: "{st.id}",
                                            r#type: "button",
                                            class: "sticker-cell",
                                            title: "{title_str}",
                                            aria_label: "Gửi sticker {title_str}",
                                            onclick: move |_| on_select.call(url.clone()),
                                            img { src: "{resolved}", alt: "{title_str}", loading: "lazy" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "picker-empty", "Bạn chưa có bộ sticker nào" }
                }
            }
        }
    }
}
