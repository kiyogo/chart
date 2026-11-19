//! Spec 012 interface: profile sections in the owner's order (with templates,
//! showcase, achievements, guestbook, shared communities and mutual friends)
//! and the layout editor (drag and drop, presets, export / import, preview as).

use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, profile_card::badge_label},
    models::UserProfileResponse,
    platform,
    services::{library, profile_plus as pp},
};

/// Profile sections in layout order. `view_as`: "self" | "friend" | "stranger" (preview).
#[component]
pub fn ProfileSections(
    user_id: String,
    profile: UserProfileResponse,
    plus: pp::ProfilePlus,
    is_self: bool,
    #[props(default = "self".to_string())] view_as: String,
    #[props(default)] on_changed: Option<EventHandler<()>>,
) -> Element {
    let mut entry = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let preview_other = is_self && view_as != "self";
    let p = profile.clone();
    let vis_of = |key: &str| p.detail_visibility.as_ref().and_then(|m| m.get(key).cloned()).unwrap_or_else(|| if key == "location" { "friends".into() } else { "everyone".into() });
    // In "preview as" mode, apply the same rules the server applies for other people.
    let shown = |key: &str| -> bool {
        if !preview_other {
            return true;
        }
        match vis_of(key).as_str() {
            "everyone" => true,
            "friends" => view_as == "friend",
            _ => false,
        }
    };
    let uid = user_id.clone();
    rsx! {
        div { class: "pc-sections",
            for s in plus.sections.iter().filter(|s| s.visible || (is_self && view_as == "self")).cloned() {
                {
                    let hidden_note = !s.visible;
                    let cls = format!("pc-section {} {}", if s.size == "wide" { "wide" } else { "" }, if hidden_note { "is-hidden" } else { "" });
                    let body = match s.id.as_str() {
                        "about" => p.bio.clone().filter(|b| !b.trim().is_empty()).map(|b| rsx! { div { class: "profile-card-bio", "{b}" } }),
                        "details" => {
                            let d = p.details.clone();
                            let rows: Vec<(&str, String)> = [
                                ("occupation", "Công việc", d.occupation.clone()),
                                ("location", "Nơi sống", d.location.clone()),
                                ("website", "Website", d.website.clone()),
                                ("pronouns", "Danh xưng", d.pronouns.clone()),
                            ]
                            .into_iter()
                            .filter(|(k, _, v)| shown(k) && v.as_ref().is_some_and(|x| !x.is_empty()))
                            .map(|(_, l, v)| (l, v.unwrap_or_default()))
                            .collect();
                            let interests = if shown("interests") { d.interests.clone() } else { Vec::new() };
                            if rows.is_empty() && interests.is_empty() {
                                None
                            } else {
                                Some(rsx! {
                                    div { class: "profile-details",
                                        for (l, v) in rows {
                                            div { key: "{l}", class: "profile-detail", span { class: "pd-k", "{l}" }
                                                if l == "Website" && (v.starts_with("https://") || v.starts_with("http://")) {
                                                    button { r#type: "button", class: "link-btn", onclick: { let v = v.clone(); move |_| platform::open_external_url(&v) }, "{v}" }
                                                } else { span { "{v}" } }
                                            }
                                        }
                                    }
                                    if !interests.is_empty() {
                                        div { class: "interest-chips", for i in interests { span { key: "{i}", class: "interest-chip", "{i}" } } }
                                    }
                                })
                            }
                        }
                        "badges" => (!p.badges.is_empty()).then(|| rsx! {
                            div { class: "profile-badges", for b in p.badges.iter() { span { key: "{b}", class: "profile-badge", "{badge_label(b)}" } } }
                        }),
                        "showcase" => (!plus.showcase.is_empty()).then(|| rsx! {
                            div { class: "showcase-grid",
                                for it in plus.showcase.iter().cloned() {
                                    if it.kind == "link" {
                                        button { key: "{it.id}", r#type: "button", class: "showcase-link", onclick: { let u = it.url.clone(); move |_| platform::open_external_url(&u) },
                                            if it.pinned { span { class: "pin-dot", "★" } }
                                            "{it.title.clone().unwrap_or_else(|| it.url.clone())}"
                                        }
                                    } else {
                                        figure { key: "{it.id}", class: "showcase-item",
                                            img { src: "{platform::resolve_asset_url(&it.url)}", alt: "{it.title.clone().unwrap_or_default()}", loading: "lazy" }
                                            if let Some(t) = it.title.clone().filter(|t| !t.is_empty()) { figcaption { if it.pinned { "★ " } "{t}" } }
                                        }
                                    }
                                }
                            }
                        }),
                        "achievements" => {
                            let list: Vec<pp::Achievement> = plus.achievements.iter().filter(|a| !preview_other || a.unlocked).cloned().collect();
                            (!list.is_empty()).then(|| rsx! {
                                div { class: "achievement-grid",
                                    for a in list {
                                        {
                                            let meta = pp::ACHIEVEMENTS.iter().find(|x| x.0 == a.id).copied().unwrap_or(("", "Thành tích", ""));
                                            let secret = a.hidden && !a.unlocked;
                                            rsx! {
                                                div { key: "{a.id}", class: if a.unlocked { "achievement unlocked" } else { "achievement" }, title: "{meta.2}",
                                                    span { class: "achievement-medal", if a.unlocked { "🏅" } else if secret { "?" } else { "○" } }
                                                    span { class: "achievement-name", if secret { "Thành tích ẩn" } else { "{meta.1}" } }
                                                    if !a.unlocked && !secret {
                                                        span { class: "achievement-progress",
                                                            span { class: "achievement-bar", style: "width: {(a.progress as f64 / a.goal.max(1) as f64 * 100.0).round()}%;" }
                                                        }
                                                        span { class: "member-sub", "{a.progress}/{a.goal}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            })
                        }
                        "guestbook" => {
                            let can_sign = plus.can_sign_guestbook && !is_self;
                            if plus.guestbook_mode == "off" { None } else {
                                Some(rsx! {
                                    if can_sign {
                                        form { class: "guestbook-form",
                                            onsubmit: { let uid = uid.clone(); move |e: Event<FormData>| {
                                                e.prevent_default();
                                                let (u, b) = (uid.clone(), entry().trim().to_string());
                                                if b.is_empty() { return; }
                                                spawn(async move {
                                                    match pp::sign(&u, &b).await {
                                                        Ok(()) => { entry.set(String::new()); error.set(None); if let Some(h) = on_changed { h.call(()) } }
                                                        Err(e) => error.set(Some(match e.code.as_str() { "rate_limited" => "Bạn viết nhiều quá, thử lại sau".into(), "guestbook_closed" => "Sổ lưu bút đang đóng".into(), c => format!("Không gửi được ({c})") })),
                                                    }
                                                });
                                            } },
                                            input { maxlength: "300", aria_label: "Viết vào sổ lưu bút", placeholder: "Viết vài lời", value: "{entry}", oninput: move |e| entry.set(e.value()) }
                                            button { r#type: "submit", class: "btn-small-secondary", disabled: entry().trim().is_empty(), "Gửi" }
                                        }
                                        if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                                    }
                                    if plus.guestbook.is_empty() {
                                        div { class: "member-sub", "Chưa có lời nhắn" }
                                    }
                                    for g in plus.guestbook.iter().cloned() {
                                        div { key: "{g.id}", class: "guestbook-entry",
                                            Avatar { name: g.author.clone().unwrap_or_default(), avatar_url: g.avatar_url.clone(), size: 24 }
                                            div { class: "grow",
                                                div { class: "member-sub", "{g.author.clone().unwrap_or_default()} · {platform::format_local_time(&g.at)}" }
                                                div { "{g.body}" }
                                            }
                                            if g.can_delete && !preview_other {
                                                button { r#type: "button", class: "icon-btn small", aria_label: "Xóa lời nhắn",
                                                    onclick: { let id = g.id.clone(); move |_| { let id = id.clone(); spawn(async move { if pp::delete_entry(&id).await.is_ok() { if let Some(h) = on_changed { h.call(()) } } }); } },
                                                    "×"
                                                }
                                            }
                                        }
                                    }
                                })
                            }
                        }
                        "communities" => (!plus.shared_communities.is_empty()).then(|| rsx! {
                            div { class: "chip-row", for c in plus.shared_communities.iter().cloned() { span { key: "{c.id}", class: "interest-chip community-chip", "{c.name}" } } }
                        }),
                        "mutual" => (!plus.mutual_friends.is_empty()).then(|| rsx! {
                            div { class: "member-sub", "{plus.mutual_friends.len()} bạn chung: {plus.mutual_friends.iter().take(5).cloned().collect::<Vec<_>>().join(\", \")}" }
                        }),
                        "links" => {
                            let links: Vec<(String, String)> = p.social_links.as_object().map(|m| m.iter().filter_map(|(k, v)| v.as_str().filter(|u| u.starts_with("https://")).map(|u| (k.clone(), u.to_string()))).collect()).unwrap_or_default();
                            (!links.is_empty()).then(|| rsx! {
                                div { class: "chip-row", for (k, u) in links { button { key: "{k}", r#type: "button", class: "link-btn", onclick: move |_| platform::open_external_url(&u), "{k}" } } }
                            })
                        }
                        _ => None,
                    };
                    rsx! {
                        if let Some(b) = body {
                            div { key: "{s.id}", class: "{cls}", "data-section": "{s.id}",
                                div { class: "profile-card-label", "{pp::section_label(&s.id)}" if hidden_note { span { class: "member-sub", " · đang ẩn" } } }
                                {b}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Layout editor: template, drag-and-drop order, visibility, size, guestbook, presets, showcase.
#[component]
pub fn LayoutEditor(
    template: String,
    sections: Vec<pp::Section>,
    guestbook_mode: String,
    on_change: EventHandler<(String, Vec<pp::Section>, String)>,
    #[props(default)] showcase: Vec<pp::ShowItem>,
    #[props(default)] on_showcase_changed: Option<EventHandler<()>>,
) -> Element {
    let mut presets = use_signal(Vec::<pp::Preset>::new);
    let mut preset_name = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut add_kind = use_signal(|| "link".to_string());
    let mut add_url = use_signal(String::new);
    let mut add_title = use_signal(String::new);
    use_hook(move || {
        spawn(async move {
            if let Ok(p) = pp::presets().await {
                presets.set(p);
            }
        });
    });
    let (t0, s0, g0) = (template.clone(), sections.clone(), guestbook_mode.clone());
    let emit = move |t: String, s: Vec<pp::Section>, g: String| on_change.call((t, s, g));
    // Drag and drop runs in the page (native HTML5 events) and reports (from, to) here.
    let mut latest = use_signal(|| (t0.clone(), s0.clone(), g0.clone()));
    if *latest.peek() != (t0.clone(), s0.clone(), g0.clone()) {
        latest.set((t0.clone(), s0.clone(), g0.clone()));
    }
    use_hook(move || {
        spawn(async move {
            let mut ev = dioxus::document::eval(
                r#"
                if (!window.__cwSectionDnd) {
                    window.__cwSectionDnd = true;
                    let from = null;
                    const row = (e) => e.target.closest && e.target.closest('.section-row');
                    document.addEventListener('dragstart', (e) => { const r = row(e); if (!r) return; from = Number(r.dataset.idx); e.dataTransfer.effectAllowed = 'move'; try { e.dataTransfer.setData('text/plain', String(from)); } catch (_) {} });
                    document.addEventListener('dragover', (e) => { if (row(e) && from !== null) e.preventDefault(); });
                    document.addEventListener('drop', (e) => { const r = row(e); if (!r || from === null) return; e.preventDefault(); const to = Number(r.dataset.idx); if (window.__cwSectionSend && to !== from) window.__cwSectionSend([from, to]); from = null; });
                    document.addEventListener('dragend', () => { from = null; });
                }
                window.__cwSectionSend = (v) => dioxus.send(v);
                "#,
            );
            while let Ok((from, to)) = ev.recv::<(usize, usize)>().await {
                let (t, mut v, g) = latest.peek().clone();
                if from < v.len() && to < v.len() && from != to {
                    let it = v.remove(from);
                    v.insert(to, it);
                    on_change.call((t, v, g));
                }
            }
        });
    });
    let move_item = {
        let (t, s, g) = (t0.clone(), s0.clone(), g0.clone());
        move |from: usize, to: usize| {
            let mut v = s.clone();
            if from < v.len() && to < v.len() && from != to {
                let it = v.remove(from);
                v.insert(to, it);
                emit(t.clone(), v, g.clone());
            }
        }
    };
    let fav = crate::components::sticker_picker::PICKER_REQUEST.peek().is_some();
    let _ = fav;
    rsx! {
        div { class: "layout-editor",
            div { class: "settings-group-title", "Mẫu" }
            p { class: "member-sub", "Mẫu chỉ đổi thẻ hồ sơ, không đổi giao diện ứng dụng." }
            div { class: "template-grid", role: "radiogroup", aria_label: "Mẫu hồ sơ",
                for (id, name) in pp::TEMPLATES {
                    {
                        let (s, g) = (s0.clone(), g0.clone());
                        rsx! {
                            button { key: "{id}", r#type: "button", role: "radio", aria_checked: "{template == id}",
                                class: if template == id { "template-tile active" } else { "template-tile" },
                                onclick: move |_| emit(id.to_string(), s.clone(), g.clone()),
                                span { class: "template-swatch pc-tpl-{id}" }
                                span { "{name}" }
                            }
                        }
                    }
                }
            }
            div { class: "settings-group-title", "Các mục" }
            div { class: "section-list", role: "list",
                for (i, s) in sections.iter().cloned().enumerate() {
                    {
                        let (t, all, g) = (t0.clone(), s0.clone(), g0.clone());
                        let (t2, all2, g2) = (t0.clone(), s0.clone(), g0.clone());
                        let mv_up = move_item.clone();
                        let mv_down = move_item.clone();
                        let n = sections.len();
                        rsx! {
                            div { key: "{s.id}", role: "listitem", draggable: "true", "data-idx": "{i}",
                                class: "section-row",
                                span { class: "drag-handle", aria_hidden: "true", "⋮⋮" }
                                span { class: "grow", "{pp::section_label(&s.id)}" }
                                button { r#type: "button", class: "btn-small-secondary", aria_label: "Đưa {pp::section_label(&s.id)} lên", disabled: i == 0, onclick: move |_| mv_up(i, i.saturating_sub(1)), "↑" }
                                button { r#type: "button", class: "btn-small-secondary", aria_label: "Đưa {pp::section_label(&s.id)} xuống", disabled: i + 1 == n, onclick: move |_| mv_down(i, i + 1), "↓" }
                                select { aria_label: "Kích thước {pp::section_label(&s.id)}", value: "{s.size}",
                                    onchange: move |e| { let mut v = all.clone(); v[i].size = e.value(); emit(t.clone(), v, g.clone()); },
                                    option { value: "normal", selected: s.size == "normal", "Thường" }
                                    option { value: "wide", selected: s.size == "wide", "Rộng" }
                                }
                                label { class: "demo-check",
                                    input { r#type: "checkbox", checked: s.visible, aria_label: "Hiện {pp::section_label(&s.id)}",
                                        onchange: move |e| { let mut v = all2.clone(); v[i].visible = e.checked(); emit(t2.clone(), v, g2.clone()); } }
                                    "Hiện"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "inline-form",
                span { class: "member-sub", "Sổ lưu bút:" }
                select { aria_label: "Ai được viết sổ lưu bút", value: "{guestbook_mode}",
                    onchange: { let (t, s) = (t0.clone(), s0.clone()); move |e: Event<FormData>| emit(t.clone(), s.clone(), e.value()) },
                    option { value: "friends", selected: guestbook_mode == "friends", "Bạn bè" }
                    option { value: "everyone", selected: guestbook_mode == "everyone", "Mọi người" }
                    option { value: "off", selected: guestbook_mode == "off", "Tắt" }
                }
                button { r#type: "button", class: "btn-small-secondary", onclick: { let g = g0.clone(); move |_| emit("minimal".into(), pp::default_sections(), g.clone()) }, "Khôi phục mặc định" }
            }

            div { class: "settings-group-title", "Bản đã lưu" }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
            div { class: "inline-form",
                input { maxlength: "40", placeholder: "Tên bản bố cục", aria_label: "Tên bản bố cục", value: "{preset_name}", oninput: move |e| preset_name.set(e.value()) }
                button { r#type: "button", class: "btn-small-secondary", disabled: preset_name().trim().is_empty(),
                    onclick: { let (t, s) = (t0.clone(), s0.clone()); move |_| {
                        let (t, s, n) = (t.clone(), s.clone(), preset_name().trim().to_string());
                        spawn(async move {
                            match pp::save_preset(&n, &t, &s).await {
                                Ok(()) => { preset_name.set(String::new()); msg.set(Some((false, "Đã lưu bản bố cục".into()))); if let Ok(p) = pp::presets().await { presets.set(p); } }
                                Err(e) => msg.set(Some((true, if e.code == "preset_limit_reached" { "Đã đủ 10 bản".into() } else { format!("Không lưu được ({})", e.code) }))),
                            }
                        });
                    } },
                    "Lưu bản hiện tại"
                }
                button { r#type: "button", class: "btn-small-secondary",
                    onclick: { let (t, s) = (t0.clone(), s0.clone()); move |_| {
                        let text = pp::export_layout(&t, &s);
                        platform::copy_to_clipboard(&text);
                        msg.set(Some((false, "Đã sao chép bố cục dạng JSON (dán để chia sẻ hoặc nhập lại)".into())));
                    } },
                    "Xuất"
                }
                button { r#type: "button", class: "btn-small-secondary",
                    onclick: { let g = g0.clone(); move |_| {
                        let g = g.clone();
                        spawn(async move {
                            let mut ev = dioxus::document::eval("navigator.clipboard.readText().then((t) => dioxus.send(t)).catch(() => dioxus.send(''));");
                            let text: String = ev.recv().await.unwrap_or_default();
                            match pp::import_layout(&text) {
                                Some((t, s)) => { emit(t, s, g.clone()); msg.set(Some((false, "Đã nhập bố cục từ bộ nhớ tạm".into()))); }
                                None => msg.set(Some((true, "Bộ nhớ tạm không có bố cục ChatW hợp lệ".into()))),
                            }
                        });
                    } },
                    "Nhập"
                }
            }
            div { class: "preset-list",
                for p in presets() {
                    {
                        let (pid, pt, ps) = (p.id.clone(), p.template.clone(), p.sections.clone());
                        let g = g0.clone();
                        rsx! {
                            div { key: "{p.id}", class: "preset-row",
                                span { class: "grow", "{p.name}" span { class: "member-sub", " · {pp::template_name(&p.template)}" } }
                                button { r#type: "button", class: "btn-small-secondary", onclick: move |_| emit(pt.clone(), ps.clone(), g.clone()), "Áp dụng" }
                                button { r#type: "button", class: "btn-small-secondary danger",
                                    onclick: move |_| { let id = pid.clone(); spawn(async move { if pp::delete_preset(&id).await.is_ok() { presets.with_mut(|l| l.retain(|x| x.id != id)); } }); },
                                    "Xóa" }
                            }
                        }
                    }
                }
            }

            div { class: "settings-group-title", "Trưng bày" }
            div { class: "showcase-manage",
                for (i, it) in showcase.iter().cloned().enumerate() {
                    {
                        let all: Vec<String> = showcase.iter().map(|x| x.id.clone()).collect();
                        let pins: Vec<String> = showcase.iter().filter(|x| x.pinned).map(|x| x.id.clone()).collect();
                        let (a1, p1, a2, p2) = (all.clone(), pins.clone(), all.clone(), pins.clone());
                        let (id_pin, id_del) = (it.id.clone(), it.id.clone());
                        rsx! {
                            div { key: "{it.id}", class: "preset-row",
                                if it.kind == "link" { span { class: "showcase-kind", "🔗" } } else { img { class: "showcase-thumb", src: "{platform::resolve_asset_url(&it.url)}", alt: "" } }
                                span { class: "grow", "{it.title.clone().unwrap_or_else(|| it.kind.clone())}" }
                                button { r#type: "button", class: if it.pinned { "btn-small-secondary active" } else { "btn-small-secondary" }, aria_pressed: "{it.pinned}",
                                    onclick: move |_| {
                                        let mut p = p1.clone();
                                        if p.contains(&id_pin) { p.retain(|x| x != &id_pin) } else { p.push(id_pin.clone()) }
                                        let a = a1.clone();
                                        spawn(async move { if pp::order_showcase(&a, &p).await.is_ok() { if let Some(h) = on_showcase_changed { h.call(()) } } });
                                    },
                                    if it.pinned { "★ Đã ghim" } else { "☆ Ghim" }
                                }
                                button { r#type: "button", class: "btn-small-secondary", disabled: i == 0, aria_label: "Đưa lên",
                                    onclick: move |_| {
                                        let mut a = a2.clone(); a.swap(i, i - 1);
                                        let p = p2.clone();
                                        spawn(async move { if pp::order_showcase(&a, &p).await.is_ok() { if let Some(h) = on_showcase_changed { h.call(()) } } });
                                    }, "↑" }
                                button { r#type: "button", class: "btn-small-secondary danger", aria_label: "Bỏ khỏi khu trưng bày",
                                    onclick: move |_| { let id = id_del.clone(); spawn(async move { if pp::delete_showcase(&id).await.is_ok() { if let Some(h) = on_showcase_changed { h.call(()) } } }); },
                                    "Bỏ" }
                            }
                        }
                    }
                }
            }
            div { class: "inline-form",
                select { aria_label: "Loại mục trưng bày", value: "{add_kind}", onchange: move |e| add_kind.set(e.value()),
                    option { value: "link", "Liên kết nổi bật" }
                    option { value: "image", "Ảnh (tải lên)" }
                    option { value: "sticker", "Sticker yêu thích" }
                }
                input { maxlength: "40", placeholder: "Tiêu đề", aria_label: "Tiêu đề mục trưng bày", value: "{add_title}", oninput: move |e| add_title.set(e.value()) }
                if add_kind() == "link" {
                    input { maxlength: "300", placeholder: "https://", aria_label: "Địa chỉ liên kết", value: "{add_url}", oninput: move |e| add_url.set(e.value()) }
                    button { r#type: "button", class: "btn-small-secondary", disabled: showcase.len() >= 12 || add_url().trim().is_empty(),
                        onclick: move |_| {
                            let (u, t) = (add_url().trim().to_string(), add_title().trim().to_string());
                            spawn(async move {
                                match pp::add_showcase("link", &u, &t).await {
                                    Ok(()) => { add_url.set(String::new()); add_title.set(String::new()); if let Some(h) = on_showcase_changed { h.call(()) } }
                                    Err(e) => msg.set(Some((true, format!("Không thêm được ({})", e.code)))),
                                }
                            });
                        }, "Thêm" }
                } else if add_kind() == "image" {
                    input { id: "showcase-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/gif,image/avif", style: "display: none;",
                        onchange: move |_| {
                            let t = add_title().trim().to_string();
                            spawn(async move {
                                match platform::read_selected_file_raw("showcase-file").await {
                                    Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                        Ok(url) => match pp::add_showcase(if mime == "image/gif" { "gif" } else { "image" }, &url, &t).await {
                                            Ok(()) => { add_title.set(String::new()); if let Some(h) = on_showcase_changed { h.call(()) } }
                                            Err(e) => msg.set(Some((true, format!("Không thêm được ({})", e.code)))),
                                        },
                                        Err(e) => msg.set(Some((true, format!("Không tải ảnh lên được ({})", e.code)))),
                                    },
                                    Err(e) if e == "too_large" => msg.set(Some((true, "Ảnh lớn hơn 2 MB".into()))),
                                    Err(_) => {}
                                }
                            });
                        },
                    }
                    label { r#for: "showcase-file", class: "btn-small-secondary", role: "button", tabindex: "0", "Chọn ảnh" }
                } else {
                    {
                        let favs = try_consume_context::<crate::prefs::PrefsCtx>().map(|c| c.prefs.read().fav_stickers.clone()).unwrap_or_default();
                        rsx! {
                            if favs.is_empty() { span { class: "member-sub", "Chưa có sticker yêu thích (bấm ☆ trong bảng sticker)" } }
                            for u in favs.into_iter().take(12) {
                                button { key: "{u}", r#type: "button", class: "showcase-pick", aria_label: "Trưng bày sticker này", disabled: showcase.len() >= 12,
                                    onclick: { let u = u.clone(); move |_| {
                                        let (u, t) = (u.clone(), add_title().trim().to_string());
                                        spawn(async move { if pp::add_showcase("sticker", &u, &t).await.is_ok() { if let Some(h) = on_showcase_changed { h.call(()) } } });
                                    } },
                                    img { src: "{platform::resolve_asset_url(&u)}", alt: "" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
