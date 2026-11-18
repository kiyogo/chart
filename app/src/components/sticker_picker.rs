use dioxus::prelude::*;

use crate::{
    models::StickerPackItem,
    services::stickers::stickers_list_packs,
};

#[component]
pub fn StickerPicker(
    on_select: EventHandler<String>,
    #[props(default)] _on_close: Option<EventHandler<()>>,
) -> Element {
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut selected_pack_idx = use_signal(|| 0usize);

    use_effect(move || {
        spawn(async move {
            if let Ok(list) = stickers_list_packs().await {
                packs.set(list);
            }
        });
    });

    rsx! {
        div {
            class: "sticker-popup",
            style: "background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 4px 16px rgba(0,0,0,0.15); padding: 8px; width: 280px; max-height: 240px; display: flex; flex-direction: column; gap: 8px; z-index: 100;",
            role: "dialog",
            aria_label: "Chọn sticker",
            onclick: move |evt| evt.stop_propagation(),

            // Stickers Grid
            div {
                class: "sticker-grid",
                style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; overflow-y: auto; max-height: 180px; padding: 4px;",
                if packs.read().is_empty() {
                    div { style: "grid-column: 1 / -1; text-align: center; color: var(--text-secondary); font-size: 12px; padding: 12px;", "Đang tải sticker..." }
                } else if let Some(pack) = packs().get(selected_pack_idx()) {
                    for sticker in pack.stickers.iter() {
                        {
                            let file_url = sticker.file_url.clone();
                            let name = sticker.name.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: "sticker-btn",
                                    style: "background: var(--bg-hover); border: 1px solid var(--border); border-radius: 8px; padding: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 22px; transition: transform 0.1s;",
                                    title: "{name}",
                                    onclick: move |_| on_select.call(file_url.clone()),
                                    "{sticker.file_url}"
                                }
                            }
                        }
                    }
                }
            }

            // Pack Tabs
            if packs.read().len() > 1 {
                div {
                    class: "sticker-pack-tabs",
                    style: "display: flex; gap: 4px; border-top: 1px solid var(--border); padding-top: 6px;",
                    for (idx, p) in packs().iter().enumerate() {
                        {
                            let is_active = idx == selected_pack_idx();
                            let name = p.name.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: if is_active { "pack-tab active" } else { "pack-tab" },
                                    style: if is_active {
                                        "padding: 2px 8px; font-size: 11px; border-radius: 10px; background: var(--primary); color: #fff; border: none; cursor: pointer;"
                                    } else {
                                        "padding: 2px 8px; font-size: 11px; border-radius: 10px; background: var(--bg-hover); color: var(--text-secondary); border: none; cursor: pointer;"
                                    },
                                    onclick: move |_| selected_pack_idx.set(idx),
                                    "{name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
