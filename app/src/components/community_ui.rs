//! Spec 013 interface: welcome screen + rules, my profile in a community,
//! events with RSVP, forum channels, and the community settings pages
//! (welcome editor, emoji / stickers, moderation audit log).

use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::CloseIcon},
    platform,
    services::{community as cm, library},
};

fn seen_key(server: &str) -> String {
    format!("chatw_welcome_seen_{server}")
}

/// Should the welcome screen open for this community now?
pub async fn should_show_welcome(server: &str) -> Option<cm::Welcome> {
    let w = cm::welcome(server).await.ok()?;
    if !w.has_content() {
        return None;
    }
    if w.must_accept() || platform::storage_get(&seen_key(server)).is_none() { Some(w) } else { None }
}

#[component]
pub fn WelcomeDialog(server_id: String, welcome: cm::Welcome, on_close: EventHandler<()>, #[props(default)] on_open_channel: Option<EventHandler<String>>) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut agreed = use_signal(|| false);
    use_hook(|| platform::focus_selector(".welcome-dialog"));
    let must = welcome.must_accept();
    let sid = server_id.clone();
    let close = move |_| {
        platform::storage_set(&seen_key(&sid), "1");
        on_close.call(());
    };
    let sid2 = server_id.clone();
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-content welcome-dialog", role: "dialog", aria_modal: "true", aria_labelledby: "welcome-title", tabindex: "-1",
                onkeydown: { let c = close.clone(); move |e: Event<KeyboardData>| if e.key() == Key::Escape && !must { c(()) } },
                div { class: "welcome-hero",
                    div { class: "welcome-kicker", "Chào mừng bạn đến" }
                    h2 { id: "welcome-title", class: "welcome-name", "{welcome.name}" }
                }
                if let Some(m) = welcome.welcome_message.clone().filter(|m| !m.trim().is_empty()) {
                    p { class: "welcome-message", "{m}" }
                }
                if !welcome.channels.is_empty() {
                    div { class: "settings-group-title", "Bắt đầu từ đây" }
                    div { class: "welcome-channels",
                        for c in welcome.channels.iter().cloned() {
                            button { key: "{c.id}", r#type: "button", class: "welcome-channel",
                                onclick: { let (id, c2) = (c.id.clone(), close.clone()); move |_| { c2(()); if let Some(h) = on_open_channel { h.call(id.clone()) } } },
                                span { class: "member-sub", if c.kind == "voice" { "🔊" } else if c.kind == "forum" { "🗂" } else if c.kind == "event" { "📅" } else { "#" } }
                                span { "{c.name}" }
                            }
                        }
                    }
                }
                if let Some(r) = welcome.rules.clone().filter(|r| !r.trim().is_empty()) {
                    div { class: "settings-group-title", "Nội quy" }
                    div { class: "welcome-rules", "{r}" }
                    if must {
                        label { class: "demo-check", input { r#type: "checkbox", checked: agreed(), onchange: move |e| agreed.set(e.checked()) } "Tôi đã đọc và đồng ý nội quy" }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                div { class: "modal-actions",
                    if must {
                        button { r#type: "button", class: "btn-primary", disabled: !agreed() || busy(),
                            onclick: { let c = close.clone(); move |_| {
                                busy.set(true);
                                let (s, c) = (sid2.clone(), c.clone());
                                spawn(async move {
                                    match cm::accept_rules(&s).await {
                                        Ok(()) => c(()),
                                        Err(e) => error.set(Some(cm::error_text(&e.code))),
                                    }
                                    busy.set(false);
                                });
                            } },
                            "Đồng ý và vào cộng đồng"
                        }
                    } else {
                        button { r#type: "button", class: "btn-primary", onclick: { let c = close.clone(); move |_| c(()) }, "Vào cộng đồng" }
                    }
                }
            }
        }
    }
}

/// How I appear in one community (nickname, avatar, short bio); the global profile is untouched.
#[component]
pub fn CommunityProfileDialog(server_id: String, server_name: String, my_name: String, on_close: EventHandler<()>) -> Element {
    let mut nick = use_signal(String::new);
    let mut avatar = use_signal(|| Option::<String>::None);
    let mut bio = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let sid = server_id.clone();
    use_hook(move || {
        let s = sid.clone();
        spawn(async move {
            if let Ok((n, a, b)) = cm::my_profile(&s).await {
                nick.set(n);
                avatar.set(a);
                bio.set(b);
            }
        });
        platform::focus_selector("[aria-labelledby='cprof-title']");
    });
    let sid2 = server_id.clone();
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-content", role: "dialog", aria_modal: "true", aria_labelledby: "cprof-title", tabindex: "-1", onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| if e.key() == Key::Escape { on_close.call(()) },
                div { class: "modal-header",
                    h3 { id: "cprof-title", class: "modal-title", "Hồ sơ của bạn trong {server_name}" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon { size: 18 } }
                }
                p { class: "modal-sub", "Chỉ đổi cách bạn xuất hiện trong cộng đồng này. Hồ sơ chung của bạn giữ nguyên." }
                div { class: "inline-form",
                    Avatar { name: if nick().is_empty() { my_name.clone() } else { nick() }, avatar_url: avatar(), size: 56 }
                    input { id: "cprof-avatar", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/gif,image/avif", style: "display: none;",
                        onchange: move |_| {
                            spawn(async move {
                                match platform::read_selected_file_raw("cprof-avatar").await {
                                    Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                        Ok(u) => avatar.set(Some(u)),
                                        Err(e) => msg.set(Some((true, format!("Không tải ảnh lên được ({})", e.code)))),
                                    },
                                    Err(e) if e == "too_large" => msg.set(Some((true, "Ảnh lớn hơn 2 MB".into()))),
                                    Err(_) => {}
                                }
                            });
                        },
                    }
                    label { r#for: "cprof-avatar", class: "btn-secondary", role: "button", tabindex: "0", "Ảnh riêng cho cộng đồng" }
                    if avatar().is_some() { button { r#type: "button", class: "btn-small-secondary", onclick: move |_| avatar.set(None), "Dùng ảnh chung" } }
                }
                label { class: "form-field", span { class: "form-label", "Biệt danh trong cộng đồng" }
                    input { maxlength: "32", placeholder: "{my_name}", value: "{nick}", oninput: move |e| nick.set(e.value()) } }
                label { class: "form-field", span { class: "form-label", "Giới thiệu ngắn ({bio().chars().count()}/190)" }
                    textarea { rows: "3", maxlength: "190", value: "{bio}", oninput: move |e| bio.set(e.value()) } }
                if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Đóng" }
                    button { r#type: "button", class: "btn-primary",
                        onclick: move |_| {
                            let s = sid2.clone();
                            spawn(async move {
                                msg.set(Some(match cm::save_my_profile(&s, &nick(), avatar().as_deref(), &bio()).await {
                                    Ok(()) => (false, "Đã lưu hồ sơ cộng đồng".into()),
                                    Err(e) => (true, cm::error_text(&e.code)),
                                }));
                            });
                        }, "Lưu" }
                }
            }
        }
    }
}

/// Events of a community: list, RSVP, create / delete (with Manage Channels).
#[component]
pub fn EventsPanel(server_id: String, can_manage: bool, #[props(default)] on_close: Option<EventHandler<()>>) -> Element {
    let mut list = use_signal(Vec::<cm::Event>::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut creating = use_signal(|| false);
    let mut title = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut place = use_signal(String::new);
    let mut start = use_signal(String::new);
    let mut end = use_signal(String::new);
    let sid = server_id.clone();
    let reload = use_callback(move |_: ()| {
        let s = sid.clone();
        spawn(async move {
            match cm::events(&s).await {
                Ok(l) => list.set(l),
                Err(e) => msg.set(Some((true, cm::error_text(&e.code)))),
            }
        });
    });
    use_hook(move || reload.call(()));
    // datetime-local values are local time; convert with the browser's offset.
    let to_iso = |v: String| async move {
        if v.is_empty() {
            return None;
        }
        let mut ev = dioxus::document::eval(&format!("dioxus.send(new Date({}).toISOString())", serde_json::to_string(&v).unwrap_or_default()));
        ev.recv::<String>().await.ok()
    };
    let sid_c = server_id.clone();
    rsx! {
        div { class: "events-panel",
            div { class: "studio-panel-head",
                span { class: "settings-group-title", "Sự kiện sắp tới" }
                div { class: "inline-form",
                    if can_manage { button { r#type: "button", class: "btn-small-secondary", onclick: move |_| creating.set(!creating()), if creating() { "Đóng" } else { "+ Tạo sự kiện" } } }
                    if let Some(h) = on_close { button { r#type: "button", class: "icon-btn", aria_label: "Đóng sự kiện", onclick: move |_| h.call(()), CloseIcon { size: 16 } } }
                }
            }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
            if creating() {
                div { class: "event-form",
                    input { maxlength: "100", placeholder: "Tên sự kiện", aria_label: "Tên sự kiện", value: "{title}", oninput: move |e| title.set(e.value()) }
                    textarea { rows: "2", maxlength: "1000", placeholder: "Mô tả", aria_label: "Mô tả sự kiện", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                    input { maxlength: "120", placeholder: "Địa điểm hoặc kênh", aria_label: "Địa điểm", value: "{place}", oninput: move |e| place.set(e.value()) }
                    div { class: "inline-form",
                        label { class: "studio-inline", "Bắt đầu" input { r#type: "datetime-local", aria_label: "Thời gian bắt đầu", value: "{start}", oninput: move |e| start.set(e.value()) } }
                        label { class: "studio-inline", "Kết thúc" input { r#type: "datetime-local", aria_label: "Thời gian kết thúc", value: "{end}", oninput: move |e| end.set(e.value()) } }
                    }
                    button { r#type: "button", class: "btn-primary", disabled: title().trim().is_empty() || start().is_empty(),
                        onclick: move |_| {
                            let s = sid_c.clone();
                            spawn(async move {
                                let st = to_iso(start()).await.unwrap_or_default();
                                let en = to_iso(end()).await;
                                match cm::create_event(&s, title().trim(), desc().trim(), place().trim(), &st, en.as_deref()).await {
                                    Ok(()) => { creating.set(false); title.set(String::new()); desc.set(String::new()); place.set(String::new()); msg.set(Some((false, "Đã tạo sự kiện".into()))); reload.call(()); }
                                    Err(e) => msg.set(Some((true, cm::error_text(&e.code)))),
                                }
                            });
                        }, "Tạo" }
                }
            }
            if list.read().is_empty() {
                div { class: "sidebar-empty", "Chưa có sự kiện sắp tới" }
            }
            for ev in list() {
                {
                    let (s1, s2, s3, s4) = (server_id.clone(), server_id.clone(), server_id.clone(), server_id.clone());
                    let (e1, e2, e3, e4) = (ev.id.clone(), ev.id.clone(), ev.id.clone(), ev.id.clone());
                    let mine = ev.mine.clone().unwrap_or_default();
                    rsx! {
                        div { key: "{ev.id}", class: "event-card",
                            div { class: "event-when", "{platform::format_local_time(&ev.starts_at)}" }
                            div { class: "settings-label", "{ev.title}" }
                            if let Some(l) = ev.location.clone().filter(|l| !l.is_empty()) { div { class: "member-sub", "📍 {l}" } }
                            if let Some(d) = ev.description.clone().filter(|d| !d.is_empty()) { p { class: "event-desc", "{d}" } }
                            div { class: "member-sub", "{ev.going} người tham gia · {ev.interested} người quan tâm" }
                            div { class: "inline-form",
                                button { r#type: "button", class: if mine == "going" { "btn-small-secondary active" } else { "btn-small-secondary" }, aria_pressed: "{mine == \"going\"}",
                                    onclick: { let m = mine.clone(); move |_| { let (s, e, st) = (s1.clone(), e1.clone(), if m == "going" { "none" } else { "going" }); spawn(async move { let _ = cm::rsvp(&s, &e, st).await; reload.call(()); }); } },
                                    "✓ Tham gia" }
                                button { r#type: "button", class: if mine == "interested" { "btn-small-secondary active" } else { "btn-small-secondary" }, aria_pressed: "{mine == \"interested\"}",
                                    onclick: { let m = mine.clone(); move |_| { let (s, e, st) = (s2.clone(), e2.clone(), if m == "interested" { "none" } else { "interested" }); spawn(async move { let _ = cm::rsvp(&s, &e, st).await; reload.call(()); }); } },
                                    "☆ Quan tâm" }
                                if can_manage {
                                    button { r#type: "button", class: "btn-small-secondary danger",
                                        onclick: move |_| { let (s, e) = (s3.clone(), e3.clone()); spawn(async move { if cm::delete_event(&s, &e).await.is_ok() { reload.call(()); } }); },
                                        "Xóa" }
                                }
                            }
                            { let _ = (&s4, &e4); rsx! {} }
                        }
                    }
                }
            }
        }
    }
}

/// Forum channel: topics with replies, pin / lock for moderators.
#[component]
pub fn ForumView(channel_id: String, can_manage: bool) -> Element {
    let mut threads = use_signal(Vec::<cm::Thread>::new);
    let mut open = use_signal(|| Option::<cm::Thread>::None);
    let mut posts = use_signal(Vec::<cm::Post>::new);
    let mut new_title = use_signal(String::new);
    let mut new_body = use_signal(String::new);
    let mut reply = use_signal(String::new);
    let mut msg = use_signal(|| Option::<String>::None);
    let ch = channel_id.clone();
    let reload = use_callback(move |_: ()| {
        let c = ch.clone();
        spawn(async move {
            if let Ok(l) = cm::threads(&c).await {
                threads.set(l);
            }
        });
    });
    let ch2 = channel_id.clone();
    let load_posts = use_callback(move |t: String| {
        let c = ch2.clone();
        spawn(async move {
            if let Ok(p) = cm::posts(&c, &t).await {
                posts.set(p);
            }
        });
    });
    let ch_key = channel_id.clone();
    use_effect(use_reactive!(|ch_key| {
        let _ = ch_key;
        open.set(None);
        reload.call(());
    }));
    let (ch3, ch4, ch5) = (channel_id.clone(), channel_id.clone(), channel_id.clone());
    rsx! {
        div { class: "forum-view",
            if let Some(m) = msg() { div { class: "status-badge error", "{m}" } }
            if let Some(t) = open() {
                div { class: "forum-thread",
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-small-secondary", onclick: move |_| { open.set(None); reload.call(()); }, "← Tất cả chủ đề" }
                        h3 { class: "grow forum-title", if t.pinned { "📌 " } "{t.title}" if t.locked { " 🔒" } }
                        if can_manage {
                            button { r#type: "button", class: "btn-small-secondary",
                                onclick: { let (c, t) = (ch3.clone(), t.clone()); move |_| { let (c, mut t) = (c.clone(), t.clone()); spawn(async move { if cm::update_thread(&c, &t.id, Some(!t.pinned), None).await.is_ok() { t.pinned = !t.pinned; open.set(Some(t)); } }); } },
                                if t.pinned { "Bỏ ghim" } else { "Ghim" } }
                            button { r#type: "button", class: "btn-small-secondary",
                                onclick: { let (c, t) = (ch4.clone(), t.clone()); move |_| { let (c, mut t) = (c.clone(), t.clone()); spawn(async move { if cm::update_thread(&c, &t.id, None, Some(!t.locked)).await.is_ok() { t.locked = !t.locked; open.set(Some(t)); } }); } },
                                if t.locked { "Mở khóa" } else { "Khóa" } }
                        }
                    }
                    div { class: "forum-posts",
                        for (i, p) in posts().into_iter().enumerate() {
                            div { key: "{p.id}", class: if i == 0 { "forum-post first" } else { "forum-post" },
                                Avatar { name: p.sender_name.clone().unwrap_or_default(), avatar_url: p.sender_avatar_url.clone(), size: 32 }
                                div { class: "grow",
                                    div { class: "member-sub", "{p.sender_name.clone().unwrap_or_default()} · {platform::format_local_time(&p.created_at)}" }
                                    div { class: "forum-post-body", "{p.content}" }
                                }
                            }
                        }
                    }
                    if !t.locked || can_manage {
                        form { class: "guestbook-form",
                            onsubmit: { let (c, tid) = (ch5.clone(), t.id.clone()); move |e: Event<FormData>| {
                                e.prevent_default();
                                let (c, tid, body) = (c.clone(), tid.clone(), reply().trim().to_string());
                                if body.is_empty() { return; }
                                spawn(async move {
                                    match cm::reply(&c, &tid, &body).await {
                                        Ok(()) => { reply.set(String::new()); msg.set(None); load_posts.call(tid); }
                                        Err(e) => msg.set(Some(cm::error_text(&e.code))),
                                    }
                                });
                            } },
                            input { maxlength: "4000", placeholder: "Trả lời chủ đề", aria_label: "Trả lời chủ đề", value: "{reply}", oninput: move |e| reply.set(e.value()) }
                            button { r#type: "submit", class: "btn-primary", disabled: reply().trim().is_empty(), "Gửi" }
                        }
                    } else {
                        div { class: "member-sub", "Chủ đề đã khóa." }
                    }
                }
            } else {
                div { class: "forum-new",
                    input { maxlength: "120", placeholder: "Tiêu đề chủ đề mới", aria_label: "Tiêu đề chủ đề", value: "{new_title}", oninput: move |e| new_title.set(e.value()) }
                    textarea { rows: "3", maxlength: "4000", placeholder: "Nội dung", aria_label: "Nội dung chủ đề", value: "{new_body}", oninput: move |e| new_body.set(e.value()) }
                    button { r#type: "button", class: "btn-primary", disabled: new_title().trim().is_empty() || new_body().trim().is_empty(),
                        onclick: { let c = channel_id.clone(); move |_| {
                            let (c, t, b) = (c.clone(), new_title().trim().to_string(), new_body().trim().to_string());
                            spawn(async move {
                                match cm::create_thread(&c, &t, &b).await {
                                    Ok(_) => { new_title.set(String::new()); new_body.set(String::new()); msg.set(None); reload.call(()); }
                                    Err(e) => msg.set(Some(cm::error_text(&e.code))),
                                }
                            });
                        } },
                        "Đăng chủ đề" }
                }
                if threads.read().is_empty() { div { class: "sidebar-empty", "Chưa có chủ đề nào. Hãy mở chủ đề đầu tiên." } }
                div { class: "forum-list",
                    for t in threads() {
                        button { key: "{t.id}", r#type: "button", class: "forum-row",
                            onclick: { let t = t.clone(); move |_| { load_posts.call(t.id.clone()); open.set(Some(t.clone())); } },
                            span { class: "grow forum-title", if t.pinned { "📌 " } "{t.title}" if t.locked { " 🔒" } }
                            span { class: "member-sub", "{t.author.clone().unwrap_or_default()} · {t.replies} bài · {platform::format_local_time(&t.last_activity)}" }
                        }
                    }
                }
            }
        }
    }
}

/// Server settings › Chào mừng & nội quy.
#[component]
pub fn WelcomeEditor(server_id: String) -> Element {
    let mut message = use_signal(String::new);
    let mut rules = use_signal(String::new);
    let mut require = use_signal(|| false);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let sid = server_id.clone();
    use_hook(move || {
        let s = sid.clone();
        spawn(async move {
            if let Ok(w) = cm::welcome(&s).await {
                message.set(w.welcome_message.unwrap_or_default());
                rules.set(w.rules.unwrap_or_default());
                require.set(w.require_rules);
            }
        });
    });
    rsx! {
        div { class: "settings-section",
            label { class: "form-field", span { class: "form-label", "Lời chào ({message().chars().count()}/500)" }
                textarea { rows: "3", maxlength: "500", value: "{message}", oninput: move |e| message.set(e.value()) } }
            label { class: "form-field", span { class: "form-label", "Nội quy ({rules().chars().count()}/2000)" }
                textarea { rows: "6", maxlength: "2000", placeholder: "1. Tôn trọng mọi người\n2. Không spam", value: "{rules}", oninput: move |e| rules.set(e.value()) } }
            label { class: "demo-check", input { r#type: "checkbox", checked: require(), onchange: move |e| require.set(e.checked()) } "Thành viên phải đồng ý nội quy trước khi gửi tin nhắn" }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
            button { r#type: "button", class: "btn-primary",
                onclick: move |_| {
                    let s = server_id.clone();
                    spawn(async move {
                        msg.set(Some(match cm::save_welcome(&s, &message(), &rules(), require()).await {
                            Ok(()) => (false, "Đã lưu màn hình chào mừng".into()),
                            Err(e) => (true, cm::error_text(&e.code)),
                        }));
                    });
                }, "Lưu" }
        }
    }
}

/// Server settings › Emoji & Sticker of the community.
#[component]
pub fn ExpressionsManager(server_id: String) -> Element {
    let mut list = use_signal(Vec::<cm::Expression>::new);
    let mut name = use_signal(String::new);
    let mut kind = use_signal(|| "emoji".to_string());
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let sid = server_id.clone();
    let reload = use_callback(move |_: ()| {
        let s = sid.clone();
        spawn(async move {
            if let Ok(l) = cm::expressions(&s).await {
                list.set(l);
            }
            cm::activate(&s).await;
        });
    });
    use_hook(move || reload.call(()));
    let sid_up = server_id.clone();
    rsx! {
        div { class: "settings-section",
            p { class: "settings-row-desc", "Emoji và sticker riêng của cộng đồng (tên tiếng Anh, chữ thường không dấu). Thành viên dùng được trong các kênh của cộng đồng này. Cần quyền Manage Expressions." }
            div { class: "inline-form",
                select { aria_label: "Loại", value: "{kind}", onchange: move |e| kind.set(e.value()),
                    option { value: "emoji", "Custom Emoji" }
                    option { value: "sticker", "Sticker" }
                }
                input { maxlength: "32", placeholder: "name (vd: party_cat)", aria_label: "Tên emoji hoặc sticker", value: "{name}", oninput: move |e| name.set(e.value()) }
                input { id: "expr-file", r#type: "file", accept: "image/png,image/gif,image/webp,image/avif", style: "display: none;",
                    onchange: move |_| {
                        let (s, k, n) = (sid_up.clone(), kind(), name().trim().to_lowercase());
                        spawn(async move {
                            match platform::read_selected_file_raw("expr-file").await {
                                Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                    Ok(u) => match cm::add_expression(&s, &k, &n, &u, mime == "image/gif").await {
                                        Ok(()) => { name.set(String::new()); msg.set(Some((false, "Đã thêm".into()))); reload.call(()); }
                                        Err(e) => msg.set(Some((true, cm::error_text(&e.code)))),
                                    },
                                    Err(e) => msg.set(Some((true, format!("Không tải lên được ({})", e.code)))),
                                },
                                Err(e) if e == "too_large" => msg.set(Some((true, "Ảnh lớn hơn 2 MB".into()))),
                                Err(_) => {}
                            }
                        });
                    },
                }
                label { r#for: "expr-file", class: if name().trim().len() < 2 { "btn-secondary disabled" } else { "btn-secondary" }, role: "button", tabindex: "0", aria_disabled: "{name().trim().len() < 2}", "Tải ảnh lên" }
            }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, "{m}" } }
            div { class: "inv-grid compact",
                for x in list() {
                    {
                        let (s, id) = (server_id.clone(), x.id.clone());
                        rsx! {
                            div { key: "{x.id}", class: "inv-card",
                                img { src: "{platform::resolve_asset_url(&x.file_url)}", alt: "{x.name}", loading: "lazy" }
                                div { class: "settings-label", if x.kind == "emoji" { ":{x.name}:" } else { "{x.name}" } }
                                button { r#type: "button", class: "btn-small-secondary danger",
                                    onclick: move |_| { let (s, id) = (s.clone(), id.clone()); spawn(async move { if cm::delete_expression(&s, &id).await.is_ok() { reload.call(()); } }); },
                                    "Xóa" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Server settings › Nhật ký kiểm duyệt.
#[component]
pub fn AuditLogView(server_id: String) -> Element {
    let mut list = use_signal(Vec::<cm::AuditEntry>::new);
    let mut err = use_signal(|| Option::<String>::None);
    use_hook(move || {
        spawn(async move {
            match cm::audit_log(&server_id).await {
                Ok(l) => list.set(l),
                Err(e) => err.set(Some(cm::error_text(&e.code))),
            }
        });
    });
    rsx! {
        div { class: "settings-section",
            if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
            if list.read().is_empty() && err().is_none() { div { class: "sidebar-empty", "Chưa có thay đổi nào được ghi" } }
            div { class: "security-log",
                for (i, a) in list().into_iter().enumerate() {
                    div { key: "{i}", class: if a.action.contains("deleted") || a.action.contains("kicked") { "security-event warn" } else { "security-event" },
                        span { class: "settings-label", "{cm::audit_label(&a.action)}" if let Some(t) = a.target.clone().filter(|t| !t.is_empty()) { span { class: "member-sub", " · {t}" } } }
                        span { class: "member-sub", "{a.actor.clone().unwrap_or_else(|| \"?\".into())} · {platform::format_local_time(&a.at)}" }
                    }
                }
            }
        }
    }
}
