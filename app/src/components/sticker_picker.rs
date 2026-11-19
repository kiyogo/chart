use std::rc::Rc;

use dioxus::prelude::*;

use crate::{
    components::{composer::fold, icons::SearchIcon},
    models::{StickerItem, StickerPackItem},
    platform,
    services::{
        openmoji::{self, Catalog},
        stickers::stickers_list_packs,
    },
};

/// Most search results rendered at once (OpenMoji alone has about 2,000 stickers).
const SEARCH_LIMIT: usize = 120;

#[component]
fn StickerCell(st: StickerItem, on_select: EventHandler<String>) -> Element {
    let url = st.file_url.clone();
    let title = if st.name.is_empty() { "Sticker".to_string() } else { st.name.clone() };
    rsx! {
        button { r#type: "button", class: "sticker-cell", title: "{title}", aria_label: "Send sticker {title}",
            onclick: move |_| on_select.call(url.clone()),
            img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{title}", loading: "lazy", decoding: "async" }
        }
    }
}

/// Sticker picker body (rendered inside a `.picker-popover`). Lists the user's
/// sticker packs from `/api/stickers` plus the built-in OpenMoji set;
/// selecting sends the sticker URL.
#[component]
pub fn StickerPicker(
    on_select: EventHandler<String>,
    #[props(default)] _on_close: Option<EventHandler<()>>,
) -> Element {
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut active_pack_id = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);
    let mut load_error = use_signal(|| false);
    let mut query = use_signal(String::new);
    let mut catalog = use_signal(|| Option::<Rc<Catalog>>::None);
    let mut om_failed = use_signal(|| false);
    let mut om_cat = use_signal(|| 0usize);

    use_effect(move || {
        spawn(async move {
            match stickers_list_packs().await {
                Ok(list) => {
                    if active_pack_id.peek().is_none() {
                        active_pack_id.set(Some(list.first().map(|p| p.id.clone()).unwrap_or_else(|| openmoji::PACK_ID.to_string())));
                    }
                    packs.set(list);
                }
                Err(_) => load_error.set(true),
            }
            is_loading.set(false);
        });
        spawn(async move {
            match openmoji::load().await {
                Ok(c) => catalog.set(Some(c)),
                Err(_) => om_failed.set(true),
            }
        });
    });

    let active = active_pack_id();
    let om_active = active.as_deref() == Some(openmoji::PACK_ID);
    let active_pack = packs().into_iter().find(|p| Some(&p.id) == active.as_ref());
    let q = fold(query().trim());

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100%; min-height: 0;",
            onclick: move |evt| evt.stop_propagation(),
            onkeydown: move |evt| if evt.key() == Key::Escape { if let Some(h) = _on_close { h.call(()) } },
            div { class: "picker-header",
                div { class: "search-field",
                    SearchIcon { size: 16 }
                    input {
                        r#type: "search",
                        placeholder: "Search stickers",
                        aria_label: "Search stickers",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
            }
            div { class: "picker-tabs", role: "tablist",
                for p in packs() {
                    {
                        let pid = p.id.clone();
                        let is_active = active.as_deref() == Some(pid.as_str());
                        rsx! {
                            button {
                                key: "{p.id}",
                                r#type: "button",
                                role: "tab",
                                aria_selected: "{is_active}",
                                class: if is_active { "picker-tab text active" } else { "picker-tab text" },
                                onclick: move |_| { query.set(String::new()); active_pack_id.set(Some(pid.clone())) },
                                "{p.name}"
                            }
                        }
                    }
                }
                if !om_failed() {
                    button {
                        r#type: "button",
                        role: "tab",
                        aria_selected: "{om_active}",
                        class: if om_active { "picker-tab text active" } else { "picker-tab text" },
                        onclick: move |_| { query.set(String::new()); active_pack_id.set(Some(openmoji::PACK_ID.to_string())) },
                        "OpenMoji"
                    }
                }
            }
            if om_active && q.is_empty() {
                if let Some(c) = catalog() {
                    div { class: "picker-subtabs", role: "tablist", aria_label: "OpenMoji categories",
                        for (i, cat) in c.categories.iter().enumerate() {
                            button { key: "{cat.0}", r#type: "button", role: "tab",
                                aria_selected: "{om_cat() == i}",
                                class: if om_cat() == i { "picker-subtab active" } else { "picker-subtab" },
                                title: "{cat.1}", aria_label: "{cat.1}",
                                onclick: move |_| om_cat.set(i),
                                img { src: "{platform::resolve_asset_url(&openmoji::url(&cat.2))}", alt: "", loading: "lazy" }
                            }
                        }
                    }
                }
            }
            div { class: "picker-body",
                if !q.is_empty() {
                    {
                        let mut hits: Vec<StickerItem> = packs().into_iter().flat_map(|p| p.stickers).filter(|s| fold(&s.name).contains(&q)).take(SEARCH_LIMIT).collect();
                        if let Some(c) = catalog() {
                            let room = SEARCH_LIMIT.saturating_sub(hits.len());
                            hits.extend(c.search(&q, room).iter().map(|i| i.as_sticker()));
                        }
                        rsx! {
                            if hits.is_empty() {
                                div { class: "picker-empty", "No stickers found" }
                            } else {
                                div { class: "sticker-grid",
                                    for st in hits {
                                        StickerCell { key: "{st.id}", st: st.clone(), on_select }
                                    }
                                }
                            }
                        }
                    }
                } else if om_active {
                    if let Some(c) = catalog() {
                        {
                            let idx = om_cat().min(c.categories.len().saturating_sub(1));
                            let label = c.categories.get(idx).map(|k| k.1.clone()).unwrap_or_default();
                            rsx! {
                                div { class: "picker-section-title", "{label}" }
                                div { class: "sticker-grid openmoji-grid",
                                    for it in c.items.iter().filter(|i| i.2 == idx) {
                                        StickerCell { key: "{it.0}", st: it.as_sticker(), on_select }
                                    }
                                }
                                div { class: "picker-credit",
                                    "Emoji by "
                                    button { r#type: "button", class: "link-btn", onclick: move |_| platform::open_external_url(openmoji::HOME_URL), "OpenMoji" }
                                    " · "
                                    button { r#type: "button", class: "link-btn", onclick: move |_| platform::open_external_url(openmoji::LICENSE_URL), "CC BY-SA 4.0" }
                                }
                            }
                        }
                    } else {
                        div { class: "picker-empty", "Đang tải sticker" }
                    }
                } else if is_loading() {
                    div { class: "picker-empty", "Đang tải sticker" }
                } else if load_error() {
                    div { class: "picker-empty", "Không tải được danh sách sticker" }
                } else if let Some(pack) = active_pack {
                    if pack.stickers.is_empty() {
                        div { class: "picker-empty", "Bộ sticker này chưa có sticker nào" }
                    } else {
                        div { class: "sticker-grid",
                            for st in pack.stickers {
                                StickerCell { key: "{st.id}", st: st.clone(), on_select }
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
