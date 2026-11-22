use std::rc::Rc;

use dioxus::prelude::*;

use crate::{
    prefs::PrefsCtx,
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

/// "Find similar" from a sticker in a message: open the sticker picker with this search.
pub static PICKER_REQUEST: GlobalSignal<Option<String>> = Signal::global(|| None);

const FAV_TAB: &str = "__favorites";
const RECENT_TAB: &str = "__recent";

/// Add or remove a sticker from Favorite Stickers (synced with my other devices).
pub fn toggle_favorite(ctx: PrefsCtx, url: &str) {
    let url = url.to_string();
    ctx.update(move |p| {
        if p.fav_stickers.contains(&url) {
            p.fav_stickers.retain(|u| u != &url);
        } else {
            p.fav_stickers.insert(0, url.clone());
        }
    });
}

/// Remember a sent sticker in Recently Used.
pub fn push_recent(ctx: PrefsCtx, url: &str) {
    let url = url.to_string();
    ctx.update(move |p| {
        p.recent_stickers.retain(|u| u != &url);
        p.recent_stickers.insert(0, url.clone());
        p.recent_stickers.truncate(24);
    });
}

/// English name for a sticker URL from my packs or the OpenMoji catalogue.
pub fn sticker_name(url: &str, packs: &[StickerPackItem], catalog: Option<&Catalog>) -> String {
    if let Some(s) = packs.iter().flat_map(|p| p.stickers.iter()).find(|s| s.file_url == url) {
        return s.name.clone();
    }
    if let (Some(c), Some(hex)) = (catalog, url.strip_prefix("/stickers/openmoji/").and_then(|h| h.strip_suffix(".svg"))) {
        if let Some(i) = c.items.iter().find(|i| i.0 == hex) {
            return i.1.clone();
        }
    }
    "Sticker".to_string()
}

#[component]
fn StickerCell(st: StickerItem, on_select: EventHandler<String>, #[props(default)] fav: bool, #[props(default)] on_fav: Option<EventHandler<String>>) -> Element {
    let url = st.file_url.clone();
    let url2 = st.file_url.clone();
    let title = if st.name.is_empty() { "Sticker".to_string() } else { st.name.clone() };
    rsx! {
        div { class: "sticker-cell-wrap",
            button { r#type: "button", class: "sticker-cell", title: "{title}", aria_label: "Send sticker {title}",
                onclick: move |_| on_select.call(url.clone()),
                img { src: "{platform::resolve_asset_url(&st.file_url)}", alt: "{title}", loading: "lazy", decoding: "async" }
            }
            if let Some(h) = on_fav {
                button { r#type: "button", class: if fav { "sticker-fav on" } else { "sticker-fav" },
                    title: if fav { "Remove from Favorite Stickers" } else { "Add to Favorite Stickers" },
                    aria_label: if fav { "Remove {title} from Favorite Stickers" } else { "Add {title} to Favorite Stickers" },
                    aria_pressed: "{fav}",
                    onclick: move |e| { e.stop_propagation(); h.call(url2.clone()) },
                    if fav { "★" } else { "☆" }
                }
            }
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
    let mut query = use_signal(|| PICKER_REQUEST.write().take().unwrap_or_default());
    let mut catalog = use_signal(|| Option::<Rc<Catalog>>::None);
    let mut om_failed = use_signal(|| false);
    let mut om_cat = use_signal(|| 0usize);
    let prefs_ctx = use_context::<PrefsCtx>();
    let favs = prefs_ctx.prefs.read().fav_stickers.clone();
    let recents = prefs_ctx.prefs.read().recent_stickers.clone();
    let on_select = EventHandler::new(move |url: String| {
        push_recent(prefs_ctx, &url);
        on_select.call(url);
    });
    let on_fav = EventHandler::new(move |url: String| toggle_favorite(prefs_ctx, &url));

    let favs_start = favs.clone();
    use_effect(move || {
        let favs_start = favs_start.clone();
        spawn(async move {
            match stickers_list_packs().await {
                Ok(list) => {
                    if active_pack_id.peek().is_none() {
                        let start = if !favs_start.is_empty() { FAV_TAB.to_string() } else { list.first().map(|p| p.id.clone()).unwrap_or_else(|| openmoji::PACK_ID.to_string()) };
                        active_pack_id.set(Some(start));
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

    let community: Option<StickerPackItem> = crate::services::community::ACTIVE_EXPRESSIONS.read().as_ref().and_then(|(sid, l)| {
        let st: Vec<StickerItem> = l.iter().filter(|x| x.kind == "sticker").map(|x| StickerItem {
            id: x.id.clone(), pack_id: format!("community-{sid}"), name: x.name.clone(), file_url: x.file_url.clone(), is_animated: x.is_animated, created_at: String::new(),
        }).collect();
        (!st.is_empty()).then(|| StickerPackItem { id: format!("community-{sid}"), name: "Community".into(), description: None, author_id: None, cover_url: None, stickers: st, created_at: String::new() })
    });
    if let Some(c) = community.clone() {
        if !packs.peek().iter().any(|p| p.id == c.id) {
            packs.with_mut(|l| l.insert(0, c));
        }
    }
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
                for (tid, tlabel) in [(FAV_TAB, "★ Favorites"), (RECENT_TAB, "Recently Used")] {
                    button { key: "{tid}", r#type: "button", role: "tab",
                        aria_selected: "{active.as_deref() == Some(tid)}",
                        class: if active.as_deref() == Some(tid) { "picker-tab text active" } else { "picker-tab text" },
                        onclick: move |_| { query.set(String::new()); active_pack_id.set(Some(tid.to_string())) },
                        "{tlabel}"
                    }
                }
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
                                        StickerCell { key: "{st.id}", st: st.clone(), on_select, fav: favs.contains(&st.file_url), on_fav }
                                    }
                                }
                            }
                        }
                    }
                } else if active.as_deref() == Some(FAV_TAB) || active.as_deref() == Some(RECENT_TAB) {
                    {
                        let is_fav = active.as_deref() == Some(FAV_TAB);
                        let list = if is_fav { favs.clone() } else { recents.clone() };
                        let all = packs();
                        let cat = catalog();
                        rsx! {
                            div { class: "picker-section-title", if is_fav { "Favorite Stickers" } else { "Recently Used" } }
                            if list.is_empty() {
                                div { class: "picker-empty",
                                    if is_fav { "No favorite stickers yet. Press ☆ on a sticker to add it." } else { "The sticker you sent will appear here." }
                                }
                            } else {
                                div { class: "sticker-grid",
                                    for u in list {
                                        StickerCell { key: "{u}", st: StickerItem { id: u.clone(), pack_id: String::new(), name: sticker_name(&u, &all, cat.as_deref()), file_url: u.clone(), is_animated: false, created_at: String::new() },
                                            on_select, fav: favs.contains(&u), on_fav }
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
                                        StickerCell { key: "{it.0}", st: it.as_sticker(), on_select, fav: favs.contains(&it.url()), on_fav }
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
                        div { class: "picker-empty", "Loading stickers" }
                    }
                } else if is_loading() {
                    div { class: "picker-empty", "Loading stickers" }
                } else if load_error() {
                    div { class: "picker-empty", "Unable to load sticker list" }
                } else if let Some(pack) = active_pack {
                    if pack.stickers.is_empty() {
                        div { class: "picker-empty", "This sticker set does not have any stickers yet" }
                    } else {
                        div { class: "sticker-grid",
                            for st in pack.stickers {
                                StickerCell { key: "{st.id}", st: st.clone(), on_select, fav: favs.contains(&st.file_url), on_fav }
                            }
                        }
                    }
                } else {
                    div { class: "picker-empty", "You don't have any sticker sets yet" }
                }
            }
        }
    }
}
