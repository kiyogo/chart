use dioxus::prelude::*;

use crate::{
    models::StickerPackItem,
    platform,
    services::stickers::stickers_list_packs,
};

#[component]
pub fn StickerPicker(
    on_select: EventHandler<String>,
    #[props(default)] _on_close: Option<EventHandler<()>>,
) -> Element {
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut active_pack_id = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(list) = stickers_list_packs().await {
                if let Some(first) = list.first() {
                    active_pack_id.set(Some(first.id.clone()));
                }
                packs.set(list);
            }
            is_loading.set(false);
        });
    });

    let active_pack = packs()
        .into_iter()
        .find(|p| Some(&p.id) == active_pack_id().as_ref());

    rsx! {
        div {
            class: "sticker-picker-popover",
            style: "width: 300px; height: 320px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-lg); box-shadow: var(--shadow-md); display: flex; flex-direction: column; overflow: hidden;",
            onclick: move |evt| evt.stop_propagation(),

            // Pack Tabs
            div {
                style: "display: flex; gap: 4px; padding: 8px; border-bottom: 1px solid var(--border); overflow-x: auto; background: var(--bg-secondary);",
                for p in packs() {
                    {
                        let pid = p.id.clone();
                        let is_active = active_pack_id().as_deref() == Some(pid.as_str());
                        rsx! {
                            button {
                                key: "{p.id}",
                                r#type: "button",
                                class: if is_active { "btn-small-primary" } else { "btn-small-disabled" },
                                style: "cursor: pointer;",
                                onclick: move |_| active_pack_id.set(Some(pid.clone())),
                                "{p.name}"
                            }
                        }
                    }
                }
            }

            // Stickers Grid
            div {
                style: "flex: 1; padding: 12px; overflow-y: auto; display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; align-content: start;",
                if is_loading() {
                    div { style: "grid-column: span 4; text-align: center; color: var(--text-secondary); padding: 20px;", "Đang tải sticker" }
                } else if let Some(pack) = active_pack {
                    if pack.stickers.is_empty() {
                        div { style: "grid-column: span 4; text-align: center; color: var(--text-secondary); padding: 20px;", "Chưa có sticker trong bộ này" }
                    } else {
                        for st in pack.stickers {
                            {
                                let url = st.image_url.clone();
                                let resolved = platform::resolve_asset_url(&url);
                                let title_str = st.name.clone().unwrap_or_else(|| "Sticker".into());
                                rsx! {
                                    button {
                                        key: "{st.id}",
                                        r#type: "button",
                                        style: "background: var(--bg-secondary); border: 1px solid var(--border); padding: 6px; border-radius: var(--radius-md); cursor: pointer; display: flex; align-items: center; justify-content: center; aspect-ratio: 1;",
                                        title: "{title_str}",
                                        onclick: move |_| on_select.call(url.clone()),
                                        img {
                                            src: "{resolved}",
                                            alt: "{title_str}",
                                            style: "width: 100%; height: 100%; object-fit: contain;",
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { style: "grid-column: span 4; text-align: center; color: var(--text-secondary); padding: 20px;", "Chưa có bộ sticker nào" }
                }
            }
        }
    }
}
