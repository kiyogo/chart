//! Spec 012: personal inventory — everything the account owns or has
//! equipped, gathered from the real libraries (sticker packs, custom emoji,
//! GIFs, drawings, layout presets, achievements, favourites, equipped looks).

use dioxus::prelude::*;

use crate::{
    components::composer::fold,
    models::{LibraryItem, StickerPackItem, UserProfileResponse},
    platform,
    prefs::PrefsCtx,
    services::{api, drawings, library, profile_plus as pp, stickers},
};

#[derive(Clone, Copy, PartialEq)]
enum Cat {
    All,
    Equipped,
    Stickers,
    Emoji,
    Gifs,
    Drawings,
    Layouts,
    Achievements,
}

const CATS: [(Cat, &str); 8] = [
    (Cat::All, "All"),
    (Cat::Equipped, "Equipping"),
    (Cat::Stickers, "Sticker Packs"),
    (Cat::Emoji, "Custom Emoji"),
    (Cat::Gifs, "GIF Collections"),
    (Cat::Drawings, "Drawings"),
    (Cat::Layouts, "Profile Templates"),
    (Cat::Achievements, "Achievements"),
];

const SEEN_KEY: &str = "chatw_seen_achievements";

#[component]
pub fn Inventory(my_id: String) -> Element {
    let prefs = use_context::<PrefsCtx>();
    let mut cat = use_signal(|| Cat::All);
    let mut query = use_signal(String::new);
    let mut packs = use_signal(Vec::<StickerPackItem>::new);
    let mut emojis = use_signal(Vec::<LibraryItem>::new);
    let mut gifs = use_signal(Vec::<LibraryItem>::new);
    let mut projects = use_signal(Vec::<drawings::ProjectItem>::new);
    let mut presets = use_signal(Vec::<pp::Preset>::new);
    let mut plus = use_signal(|| Option::<pp::ProfilePlus>::None);
    let mut profile = use_signal(|| Option::<UserProfileResponse>::None);
    let mut studio = use_signal(|| Option::<String>::None);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let me = my_id.clone();
    use_hook(move || {
        spawn(async move {
            if let Ok(l) = stickers::stickers_list_packs().await {
                packs.set(l.into_iter().filter(|p| p.author_id.as_deref() == Some(me.as_str())).collect());
            }
            if let Ok(l) = library::emojis_list().await {
                emojis.set(l);
            }
            if let Ok(l) = library::gifs_list().await {
                gifs.set(l);
            }
            if let Ok(l) = drawings::list().await {
                projects.set(l);
            }
            if let Ok(l) = pp::presets().await {
                presets.set(l);
            }
            if let Ok(p) = api::get_user_profile(&me).await {
                profile.set(Some(p));
            }
            if let Ok(x) = pp::get(&me).await {
                // Tell the user about achievements unlocked since the last visit.
                let seen: Vec<String> = platform::storage_get(SEEN_KEY).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
                let now: Vec<String> = x.achievements.iter().filter(|a| a.unlocked).map(|a| a.id.clone()).collect();
                let new: Vec<&str> = now.iter().filter(|a| !seen.contains(a)).filter_map(|a| pp::ACHIEVEMENTS.iter().find(|m| m.0 == a).map(|m| m.1)).collect();
                if !new.is_empty() && !seen.is_empty() {
                    msg.set(Some((false, format!("Just unlocked: {}", new.join(", ")))));
                }
                platform::storage_set(SEEN_KEY, &serde_json::to_string(&now).unwrap_or_default());
                plus.set(Some(x));
            }
        });
    });

    let q = fold(query().trim());
    let hit = |name: &str| q.is_empty() || fold(name).contains(&q);
    let _ = (&mut cat, CATS);
    let show = |_c: Cat| true;
    let favs = prefs.prefs.read().fav_stickers.clone();
    let fav_gifs = prefs.prefs.read().fav_gifs.clone();
    let p = profile();

    rsx! {
        div { class: "inventory",
            div { class: "search-field",
                crate::components::icons::SearchIcon { size: 16 }
                input { r#type: "search", placeholder: "Search in inventory", aria_label: "Search in item inventory", value: "{query}", oninput: move |e| query.set(e.value()) }
            }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" } }

            if show(Cat::Equipped) {
                if let Some(pr) = p.clone() {
                    div { class: "settings-group-title", "Equipping" }
                    div { class: "inv-grid",
                        InvCard { title: "Avatar Movement", sub: pr.avatar_motion.as_ref().and_then(|m| m.meta()).map(|m| m.1.to_string()).unwrap_or_else(|| "None".into()) }
                        InvCard { title: "Avatar decoration", sub: pr.avatar_decoration.clone().unwrap_or_else(|| "None".into()), img: pr.custom_decoration_url.clone().filter(|_| pr.avatar_decoration.as_deref() == Some("custom")) }
                        InvCard { title: "Name effect", sub: pr.name_effect.clone().unwrap_or_else(|| "None".into()) }
                        InvCard { title: "Name plate", sub: pr.nameplate.clone().unwrap_or_else(|| "None".into()), img: pr.custom_nameplate_url.clone().filter(|_| pr.nameplate.as_deref() == Some("custom")) }
                        InvCard { title: "Profile effects", sub: pr.profile_effect.clone().unwrap_or_else(|| "None".into()), img: pr.custom_effect_url.clone().filter(|_| pr.profile_effect.as_deref() == Some("custom")) }
                        InvCard { title: "Profile layout", sub: plus().map(|x| pp::template_name(&x.template).to_string()).unwrap_or_else(|| "Minimalist".into()) }
                        for b in pr.badges.iter() { InvCard { key: "{b}", title: "Badge", sub: crate::components::profile_card::badge_label(b).to_string() } }
                    }
                }
            }
            if show(Cat::Stickers) {
                div { class: "settings-group-title", "Favorite Sticker & Sticker Set" }
                div { class: "inv-grid",
                    for pk in packs().into_iter().filter(|x| hit(&x.name)) {
                        InvCard { key: "{pk.id}", title: pk.name.clone(), sub: format!("{} sticker", pk.stickers.len()), img: pk.cover_url.clone().or(pk.stickers.first().map(|s| s.file_url.clone())) }
                    }
                    for u in favs.iter().cloned().filter(|u| hit(u)) {
                        InvCard { key: "{u}", title: "Favorite stickers", sub: String::new(), img: Some(u.clone()) }
                    }
                }
                if packs.read().is_empty() && favs.is_empty() { div { class: "sidebar-empty", "Blank" } }
            }
            if show(Cat::Emoji) {
                div { class: "settings-group-title", "Custom emojis" }
                div { class: "inv-grid compact",
                    for e in emojis().into_iter().filter(|x| hit(&x.name)) { InvCard { key: "{e.id}", title: format!(":{}:", e.name), sub: String::new(), img: Some(e.file_url.clone()) } }
                }
            }
            if show(Cat::Gifs) {
                div { class: "settings-group-title", "GIF collection" }
                div { class: "inv-grid compact",
                    for g in gifs().into_iter().filter(|x| hit(&x.name)) { InvCard { key: "{g.id}", title: g.name.clone(), sub: String::new(), img: Some(g.file_url.clone()) } }
                    for u in fav_gifs.iter().cloned() { InvCard { key: "f{u}", title: "Favorite GIFs", sub: String::new(), img: Some(u.clone()) } }
                }
            }
            if show(Cat::Drawings) {
                div { class: "settings-group-title", "Studio Drawing" }
                div { class: "inv-grid",
                    for pj in projects().into_iter().filter(|x| hit(&x.name)) {
                        {
                            let id = pj.id.clone();
                            rsx! {
                                div { key: "{pj.id}", class: "inv-card",
                                    if pj.has_thumb { img { src: "{drawings::thumb_url(&pj.id)}", alt: "", loading: "lazy" } }
                                    div { class: "settings-label", "{pj.name}" }
                                    div { class: "member-sub", "{pj.layers} layer · {pj.frames} frame" }
                                    button { r#type: "button", class: "btn-small-secondary", onclick: move |_| studio.set(Some(id.clone())), "Open in Studio" }
                                }
                            }
                        }
                    }
                }
            }
            if show(Cat::Layouts) {
                div { class: "settings-group-title", "Resume layout template" }
                div { class: "inv-grid",
                    for pr in presets().into_iter().filter(|x| hit(&x.name)) {
                        {
                            let (t, s) = (pr.template.clone(), pr.sections.clone());
                            let mode = plus().map(|x| x.guestbook_mode).unwrap_or_else(|| "friends".into());
                            rsx! {
                                div { key: "{pr.id}", class: "inv-card",
                                    span { class: "template-swatch pc-tpl-{pr.template}" }
                                    div { class: "settings-label", "{pr.name}" }
                                    div { class: "member-sub", "{pp::template_name(&pr.template)}" }
                                    button { r#type: "button", class: "btn-small-secondary",
                                        onclick: move |_| {
                                            let (t, s, m) = (t.clone(), s.clone(), mode.clone());
                                            spawn(async move {
                                                msg.set(Some(match pp::save_layout(&t, &s, &m).await {
                                                    Ok(()) => (false, "The layout has been applied to the profile".into()),
                                                    Err(_) => (true, "Layout not applicable".into()),
                                                }));
                                            });
                                        }, "Apply" }
                                }
                            }
                        }
                    }
                }
                if presets.read().is_empty() { div { class: "sidebar-empty", "Blank" } }
            }
            if show(Cat::Achievements) {
                if let Some(x) = plus() {
                    div { class: "settings-group-title", "Achievements" }
                    div { class: "achievement-grid wide",
                        for a in x.achievements.iter().cloned() {
                            {
                                let meta = pp::ACHIEVEMENTS.iter().find(|m| m.0 == a.id).copied().unwrap_or(("", "Achievements", ""));
                                let secret = a.hidden && !a.unlocked;
                                rsx! {
                                    div { key: "{a.id}", class: if a.unlocked { "achievement unlocked" } else { "achievement" },
                                        span { class: "achievement-medal", if a.unlocked { "🏅" } else if secret { "?" } else { "○" } }
                                        span { class: "achievement-name", if secret { "Hidden achievements" } else { "{meta.1}" } }
                                        span { class: "member-sub", if secret { "Secret until unlocked" } else { "{meta.2}" } }
                                        if a.unlocked {
                                            span { class: "member-sub", "Unlock {a.unlocked_at.as_deref().map(platform::format_local_time).unwrap_or_default()}" }
                                        } else if !secret {
                                            span { class: "achievement-progress", span { class: "achievement-bar", style: "width: {(a.progress as f64 / a.goal.max(1) as f64 * 100.0).round()}%;" } }
                                            span { class: "member-sub", "{a.progress}/{a.goal}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(id) = studio() {
                crate::components::studio::Studio { my_id: my_id.clone(), open_project: Some(id), on_close: move |_| studio.set(None) }
            }
        }
    }
}

#[component]
fn InvCard(title: String, sub: String, #[props(default)] img: Option<String>) -> Element {
    rsx! {
        div { class: "inv-card",
            if let Some(u) = img.filter(|u| u.starts_with('/')) { img { src: "{platform::resolve_asset_url(&u)}", alt: "", loading: "lazy" } }
            div { class: "settings-label", "{title}" }
            if !sub.is_empty() { div { class: "member-sub", "{sub}" } }
        }
    }
}
