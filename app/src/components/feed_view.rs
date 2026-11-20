//! Spec 016 & Spec 031: Social Feed ("Mới") with compact top composer bar,
//! Facebook-style vertical Story Cards carousel, and full post threads backed by `/api/feed`.

use std::collections::HashMap;

use chrono::{DateTime, Local, Utc};
use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::{avatar_bg_color, avatar_initial, Avatar},
        icons::{
            BackIcon, ChevronLeftIcon, ChevronRightIcon, CloseIcon, FeedThreadsIcon, HeartIcon,
            ImageIcon, MessageIcon, PlusIcon, RepeatIcon, SendIcon, TrashIcon,
        },
        profile_card::name_effect_class,
    },
    models::FriendItem,
    platform,
    services::{
        api,
        feed::{self, FeedPostItem},
        library, profile_ext,
    },
};

const QUICK_NOTE_EMOJIS: [&str; 8] = ["💭", "🔥", "☕", "🎧", "🚀", "😄", "🎉", "✨"];

const STORY_GRADIENTS: [&str; 6] = [
    "linear-gradient(150deg, #3B82F6 0%, #6366F1 100%)",
    "linear-gradient(150deg, #EC4899 0%, #8B5CF6 100%)",
    "linear-gradient(150deg, #10B981 0%, #0EA5E9 100%)",
    "linear-gradient(150deg, #F59E0B 0%, #EF4444 100%)",
    "linear-gradient(150deg, #6366F1 0%, #14B8A6 100%)",
    "linear-gradient(150deg, #F43F5E 0%, #FB923C 100%)",
];

fn story_gradient_for(seed: &str) -> &'static str {
    let mut h: usize = 0;
    for b in seed.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as usize);
    }
    STORY_GRADIENTS[h % STORY_GRADIENTS.len()]
}

fn format_relative(iso: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let mins = Utc::now().signed_duration_since(dt.with_timezone(&Utc)).num_minutes();
    if mins < 1 {
        "Vừa xong".to_string()
    } else if mins < 60 {
        format!("{mins} phút")
    } else if mins < 24 * 60 {
        format!("{} giờ", mins / 60)
    } else if mins < 7 * 24 * 60 {
        format!("{} ngày", mins / (24 * 60))
    } else {
        dt.with_timezone(&Local).format("%d/%m/%Y").to_string()
    }
}

fn vis_label(v: &str) -> &'static str {
    match v {
        "public" => "Công khai",
        "self" => "Chỉ mình tôi",
        _ => "Bạn bè",
    }
}

#[component]
pub fn FeedView(
    my_id: String,
    my_username: String,
    my_avatar_url: Option<String>,
    my_status_text: Option<String>,
    my_status_emoji: Option<String>,
    friends: Vec<FriendItem>,
    refresh_tick: u64,
    on_back: EventHandler<()>,
    on_my_status_updated: EventHandler<(Option<String>, Option<String>)>,
    on_open_friend: EventHandler<FriendItem>,
    on_open_profile: EventHandler<String>,
    on_note_reply: EventHandler<(FriendItem, String)>,
) -> Element {
    let filter = || "all".to_string();
    let mut posts = use_signal(Vec::<FeedPostItem>::new);
    let mut loading = use_signal(|| true);
    let mut composer_open = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut media_url = use_signal(|| Option::<String>::None);
    let mut visibility = use_signal(|| "friends".to_string());
    let mut posting = use_signal(|| false);
    let mut uploading_img = use_signal(|| false);
    let mut err = use_signal(|| Option::<String>::None);

    // Story & Thought modals state
    let mut create_story_open = use_signal(|| false);
    let mut story_draft = use_signal(String::new);
    let mut story_media_url = use_signal(|| Option::<String>::None);
    let mut story_uploading = use_signal(|| false);
    let mut story_posting = use_signal(|| false);
    let mut story_err = use_signal(|| Option::<String>::None);

    let mut thought_open = use_signal(|| false);
    let mut local_status_text = use_signal(|| my_status_text.clone());
    let mut local_status_emoji = use_signal(|| my_status_emoji.clone());
    let mut note_emoji = use_signal(|| my_status_emoji.clone().unwrap_or_default());
    let mut note_text = use_signal(|| my_status_text.clone().unwrap_or_default());
    let mut note_expiry = use_signal(|| 1440i64);
    let mut note_saving = use_signal(|| false);
    let mut note_err = use_signal(|| Option::<String>::None);

    let mut viewing_story = use_signal(|| Option::<FeedPostItem>::None);
    let mut viewing_friend_note = use_signal(|| Option::<FriendItem>::None);
    let mut story_reply_draft = use_signal(String::new);

    let mut open_threads = use_signal(HashMap::<String, Vec<FeedPostItem>>::new);
    let mut reply_drafts = use_signal(HashMap::<String, String>::new);
    let mut repost_target = use_signal(|| Option::<FeedPostItem>::None);
    let mut repost_comment = use_signal(String::new);

    let load_posts = use_callback(move |flt: String| {
        loading.set(true);
        spawn(async move {
            match feed::list_feed(&flt).await {
                Ok(list) => {
                    posts.set(list);
                    err.set(None);
                }
                Err(e) => err.set(Some(format!("Không tải được bảng tin: {}", e.code))),
            }
            loading.set(false);
        });
    });

    {
        use_effect(use_reactive!(|(refresh_tick,)| {
            let _ = refresh_tick;
            load_posts.call(filter());
        }));
    }

    {
        use_effect(use_reactive!(|my_status_text, my_status_emoji| {
            local_status_text.set(my_status_text);
            local_status_emoji.set(my_status_emoji);
        }));
    }

    // Wire smooth horizontal scroll, drag-to-scroll & prev/next arrow state for #feed-story-rail
    use_effect(move || {
        let js = r#"
        (function() {
            const wrap = document.getElementById('feed-story-carousel');
            const rail = document.getElementById('feed-story-rail');
            if (!wrap || !rail) return;
            function updateArrows() {
                const maxScroll = rail.scrollWidth - rail.clientWidth;
                wrap.dataset.canPrev = rail.scrollLeft > 8 ? '1' : '0';
                wrap.dataset.canNext = maxScroll - rail.scrollLeft > 8 ? '1' : '0';
            }
            updateArrows();
            if (rail.__storyCarouselReady) return;
            rail.__storyCarouselReady = true;
            rail.addEventListener('scroll', updateArrows, { passive: true });
            window.addEventListener('resize', updateArrows, { passive: true });
            if (typeof ResizeObserver !== 'undefined') {
                new ResizeObserver(updateArrows).observe(rail);
            }
            let down = false, startX = 0, startScroll = 0, moved = false;
            rail.addEventListener('dragstart', (e) => e.preventDefault());
            rail.addEventListener('pointerdown', (e) => {
                if (e.button !== 0) return;
                down = true; moved = false;
                startX = e.clientX; startScroll = rail.scrollLeft;
            });
            window.addEventListener('pointermove', (e) => {
                if (!down) return;
                const dx = e.clientX - startX;
                if (Math.abs(dx) > 6) {
                    if (!moved) { moved = true; rail.classList.add('dragging'); }
                    rail.scrollLeft = startScroll - dx;
                }
            });
            window.addEventListener('pointerup', () => {
                if (!down) return;
                down = false;
                rail.classList.remove('dragging');
                if (moved) {
                    rail.dataset.justDragged = '1';
                    setTimeout(() => { delete rail.dataset.justDragged; }, 60);
                }
            });
            rail.addEventListener('click', (e) => {
                if (rail.dataset.justDragged === '1') {
                    e.preventDefault();
                    e.stopPropagation();
                }
            }, true);
        })();
        "#;
        let _ = dioxus::document::eval(js);
    });

    // Derived story data from real posts & friends
    let posts_snapshot = posts();
    let my_latest_story = posts_snapshot
        .iter()
        .find(|p| p.author_id == my_id || p.author_username == my_username)
        .cloned();
    let cur_em = local_status_emoji().as_deref().map(str::trim).unwrap_or("").to_string();
    let cur_tx = local_status_text().as_deref().map(str::trim).unwrap_or("").to_string();
    let my_has_note = !cur_em.is_empty() || !cur_tx.is_empty();
    let my_note_display = format!("{cur_em} {cur_tx}").trim().to_string();

    let mut ordered_friends = friends.clone();
    ordered_friends.sort_by_key(|f| {
        let has_story = posts_snapshot
            .iter()
            .any(|p| p.author_id == f.id || p.author_username == f.username);
        let has_note = f.status_text.as_deref().is_some_and(|s| !s.trim().is_empty())
            || f.status_emoji.as_deref().is_some_and(|s| !s.trim().is_empty());
        let is_online = f.presence.as_deref() == Some("online");
        (!has_story, !has_note, !is_online, f.username.to_lowercase())
    });

    let prompt_name = my_username.trim();
    let compose_prompt = if prompt_name.is_empty() {
        "Bạn đang nghĩ gì thế?".to_string()
    } else {
        format!("{prompt_name} ơi, bạn đang nghĩ gì thế?")
    };

    let show_expanded = composer_open() || !draft().is_empty() || media_url().is_some();
    let my_bg = avatar_bg_color(&my_username);
    let my_init = avatar_initial(&my_username);
    let init_em_btn = cur_em.clone();
    let init_tx_btn = cur_tx.clone();
    let init_em_card = cur_em.clone();
    let init_tx_card = cur_tx.clone();

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column feed-column",
                header { class: "chat-header only-mobile",
                    button {
                        r#type: "button",
                        class: "icon-btn back-btn",
                        aria_label: "Quay lại",
                        onclick: move |_| on_back.call(()),
                        BackIcon {}
                    }
                }

                div { class: "feed-scroll",
                    div { class: "feed-inner",
                        // 1. Compact Facebook-style Composer Bar at top
                        div { class: "feed-card feed-compose-bar",
                            div { class: "feed-compose-bar-row",
                                Avatar { name: my_username.clone(), avatar_url: my_avatar_url.clone(), size: 40 }
                                button {
                                    r#type: "button",
                                    class: "feed-compose-pill",
                                    aria_label: "Viết bài mới",
                                    onclick: move |_| {
                                        composer_open.set(true);
                                        let _ = dioxus::document::eval("setTimeout(() => { const el = document.getElementById('feed-compose-textarea'); if (el) el.focus(); }, 30);");
                                    },
                                    "{compose_prompt}"
                                }
                                div { class: "feed-compose-quick-actions",
                                    // Story quick button (coral red video/story icon)
                                    button {
                                        r#type: "button",
                                        class: "feed-quick-btn quick-story",
                                        title: "Tạo tin ảnh",
                                        aria_label: "Tạo tin ảnh",
                                        onclick: move |_| {
                                            story_err.set(None);
                                            create_story_open.set(true);
                                        },
                                        svg {
                                            width: "22", height: "22", view_box: "0 0 24 24",
                                            fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                            stroke_linecap: "round", stroke_linejoin: "round",
                                            rect { x: "2", y: "6", width: "14", height: "12", rx: "2.5", fill: "currentColor", fill_opacity: "0.14" }
                                            path { d: "M16 10l5-3v10l-5-3z", fill: "currentColor" }
                                            circle { cx: "9", cy: "12", r: "2", fill: "currentColor", stroke: "none" }
                                        }
                                    }
                                    // Photo upload quick button (green gallery icon)
                                    input {
                                        id: "feed-media-upload",
                                        r#type: "file",
                                        accept: "image/png,image/jpeg,image/webp,image/gif,image/avif",
                                        style: "display: none;",
                                        onchange: move |_| {
                                            composer_open.set(true);
                                            uploading_img.set(true);
                                            spawn(async move {
                                                if let Ok((bytes, mime)) = platform::read_selected_file_raw("feed-media-upload").await {
                                                    match library::upload_media(bytes, &mime).await {
                                                        Ok(url) => { media_url.set(Some(url)); err.set(None); }
                                                        Err(_) => err.set(Some("Không tải ảnh lên được".into())),
                                                    }
                                                }
                                                uploading_img.set(false);
                                            });
                                        },
                                    }
                                    label {
                                        r#for: "feed-media-upload",
                                        class: "feed-quick-btn quick-photo",
                                        title: "Thêm ảnh",
                                        aria_label: "Thêm ảnh",
                                        svg {
                                            width: "22", height: "22", view_box: "0 0 24 24",
                                            fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                            stroke_linecap: "round", stroke_linejoin: "round",
                                            rect { x: "3", y: "4", width: "18", height: "16", rx: "3", fill: "currentColor", fill_opacity: "0.14" }
                                            circle { cx: "8.5", cy: "9.5", r: "1.6", fill: "currentColor", stroke: "none" }
                                            path { d: "M21 16l-5-5L6 20" }
                                        }
                                    }
                                    // Thought / Moment quick button (rose pink reel/moment icon)
                                    button {
                                        r#type: "button",
                                        class: "feed-quick-btn quick-moment",
                                        title: "Chia sẻ suy nghĩ",
                                        aria_label: "Chia sẻ suy nghĩ",
                                        onclick: move |_| {
                                            note_emoji.set(init_em_btn.clone());
                                            note_text.set(init_tx_btn.clone());
                                            note_err.set(None);
                                            thought_open.set(true);
                                        },
                                        svg {
                                            width: "22", height: "22", view_box: "0 0 24 24",
                                            fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                            stroke_linecap: "round", stroke_linejoin: "round",
                                            rect { x: "3", y: "4", width: "18", height: "16", rx: "3.5", fill: "currentColor", fill_opacity: "0.14" }
                                            line { x1: "3", y1: "9", x2: "21", y2: "9" }
                                            line { x1: "9", y1: "4", x2: "7.5", y2: "9" }
                                            line { x1: "15.5", y1: "4", x2: "14", y2: "9" }
                                            polygon { points: "10.5,12 10.5,17 15,14.5", fill: "currentColor", stroke: "none" }
                                        }
                                    }
                                }
                            }

                            if show_expanded {
                                div { class: "feed-compose-expanded",
                                    textarea {
                                        id: "feed-compose-textarea",
                                        class: "feed-compose-input",
                                        rows: "3",
                                        maxlength: "1000",
                                        placeholder: "{compose_prompt}",
                                        aria_label: "Nội dung bài viết",
                                        value: "{draft}",
                                        oninput: move |e| draft.set(e.value()),
                                    }
                                    if let Some(murl) = media_url() {
                                        div { class: "feed-media-preview",
                                            img { src: "{platform::resolve_asset_url(&murl)}", alt: "Ảnh đính kèm" }
                                            button {
                                                r#type: "button",
                                                class: "icon-btn feed-media-rm",
                                                aria_label: "Xóa ảnh",
                                                onclick: move |_| media_url.set(None),
                                                CloseIcon { size: 14 }
                                            }
                                        }
                                    }
                                    div { class: "feed-compose-foot",
                                        div { class: "feed-compose-tools",
                                            label {
                                                r#for: "feed-media-upload",
                                                class: "icon-btn",
                                                title: "Thêm ảnh",
                                                aria_label: "Thêm ảnh bài viết",
                                                ImageIcon { size: 18 }
                                            }
                                            select {
                                                class: "feed-vis-select",
                                                value: "{visibility}",
                                                onchange: move |e| visibility.set(e.value()),
                                                option { value: "friends", "Bạn bè" }
                                                option { value: "public", "Công khai" }
                                                option { value: "self", "Chỉ mình tôi" }
                                            }
                                        }
                                        div { style: "display: flex; gap: 8px; align-items: center;",
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: move |_| {
                                                    draft.set(String::new());
                                                    media_url.set(None);
                                                    composer_open.set(false);
                                                },
                                                "Hủy"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-primary",
                                                disabled: posting() || uploading_img() || (draft().trim().is_empty() && media_url().is_none()),
                                                onclick: move |_| {
                                                    let txt = draft().trim().to_string();
                                                    let mu = media_url();
                                                    let vis = visibility();
                                                    if txt.is_empty() && mu.is_none() {
                                                        return;
                                                    }
                                                    posting.set(true);
                                                    spawn(async move {
                                                        match feed::create_post(&txt, mu.as_deref(), &vis, None, None).await {
                                                            Ok(_) => {
                                                                draft.set(String::new());
                                                                media_url.set(None);
                                                                composer_open.set(false);
                                                                err.set(None);
                                                                load_posts.call(filter());
                                                            }
                                                            Err(e) => err.set(Some(format!("Đăng bài thất bại: {}", e.code))),
                                                        }
                                                        posting.set(false);
                                                    });
                                                },
                                                if uploading_img() { "Đang tải" } else { "Đăng" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 2. Vertical Facebook-style Story Cards Carousel
                        div { id: "feed-story-carousel", class: "feed-story-carousel",
                            div { id: "feed-story-rail", class: "feed-story-rail", role: "list", aria_label: "Tin nổi bật",
                                // First Card: "Tạo tin"
                                button {
                                    r#type: "button",
                                    class: "feed-story-card create-card",
                                    role: "listitem",
                                    aria_label: "Tạo tin",
                                    onclick: move |_| {
                                        story_err.set(None);
                                        create_story_open.set(true);
                                    },
                                    div { class: "feed-story-create-media",
                                        if let Some(ref url) = my_avatar_url.as_ref().filter(|u| !u.trim().is_empty()) {
                                            img { src: "{platform::resolve_asset_url(url)}", alt: "Tạo tin" }
                                        } else {
                                            div {
                                                class: "feed-story-create-fallback",
                                                style: "background: linear-gradient(155deg, {my_bg} 0%, color-mix(in srgb, {my_bg} 68%, #0F172A) 100%);",
                                                span { "{my_init}" }
                                            }
                                        }
                                    }
                                    div { class: "feed-story-create-footer",
                                        span { class: "feed-story-create-plus", PlusIcon { size: 20 } }
                                        span { class: "feed-story-create-label", "Tạo tin" }
                                    }
                                }

                                // Second Card: "Tin của bạn" (if user has an active story or note)
                                if let Some(my_st) = my_latest_story.clone() {
                                    {
                                        let my_st_open = my_st.clone();
                                        let my_bg_grad = story_gradient_for(&my_id);
                                        let st_caption = my_st.content.trim().to_string();
                                        rsx! {
                                            button {
                                                r#type: "button",
                                                class: "feed-story-card story-card",
                                                role: "listitem",
                                                aria_label: "Tin của bạn",
                                                onclick: move |_| {
                                                    story_reply_draft.set(String::new());
                                                    viewing_story.set(Some(my_st_open.clone()));
                                                },
                                                if let Some(ref murl) = my_st.media_url.as_ref().filter(|s| !s.trim().is_empty()) {
                                                    img { class: "feed-story-bg-img", src: "{platform::resolve_asset_url(murl)}", alt: "Tin của bạn", loading: "lazy" }
                                                } else {
                                                    div { class: "feed-story-text-bg", style: "background: {my_bg_grad};",
                                                        span { class: "feed-story-text-preview", "{st_caption}" }
                                                    }
                                                }
                                                div { class: "feed-story-scrim" }
                                                span { class: "feed-story-avatar-badge has-story",
                                                    Avatar { name: my_username.clone(), avatar_url: my_avatar_url.clone(), size: 36 }
                                                }
                                                span { class: "feed-story-name", "Tin của bạn" }
                                            }
                                        }
                                    }
                                } else if my_has_note {
                                    {
                                        let my_bg_grad = story_gradient_for(&my_id);
                                        let note_preview = my_note_display.clone();
                                        let em_c = init_em_card.clone();
                                        let tx_c = init_tx_card.clone();
                                        rsx! {
                                            button {
                                                r#type: "button",
                                                class: "feed-story-card story-card",
                                                role: "listitem",
                                                aria_label: "Tin của bạn",
                                                onclick: move |_| {
                                                    note_emoji.set(em_c.clone());
                                                    note_text.set(tx_c.clone());
                                                    note_err.set(None);
                                                    thought_open.set(true);
                                                },
                                                div { class: "feed-story-text-bg", style: "background: {my_bg_grad};",
                                                    span { class: "feed-story-text-preview", "{note_preview}" }
                                                }
                                                div { class: "feed-story-scrim" }
                                                span { class: "feed-story-avatar-badge has-story",
                                                    Avatar { name: my_username.clone(), avatar_url: my_avatar_url.clone(), size: 36 }
                                                }
                                                span { class: "feed-story-name", "Tin của bạn" }
                                            }
                                        }
                                    }
                                }

                                // Friend Story Cards
                                for fr in ordered_friends.iter().take(16) {
                                    {
                                        let shown = profile_ext::nickname_of(&fr.id)
                                            .or_else(|| fr.display_name.clone().filter(|n| !n.trim().is_empty()))
                                            .unwrap_or_else(|| fr.username.clone());
                                        let fem = fr.status_emoji.as_deref().map(str::trim).unwrap_or("");
                                        let ftx = fr.status_text.as_deref().map(str::trim).unwrap_or("");
                                        let has_note = !fem.is_empty() || !ftx.is_empty();
                                        let note_str = format!("{fem} {ftx}").trim().to_string();
                                        let friend_story = posts_snapshot
                                            .iter()
                                            .find(|p| p.author_id == fr.id || p.author_username == fr.username)
                                            .cloned();
                                        let has_story = friend_story.is_some();
                                        let online = fr.presence.as_deref() == Some("online");
                                        let grad = story_gradient_for(&fr.id);
                                        let fr_bg = avatar_bg_color(&fr.username);
                                        let fr_init = avatar_initial(&fr.username);
                                        let fr_click = fr.clone();
                                        let st_click = friend_story.clone();
                                        let badge_cls = if has_story || has_note {
                                            "feed-story-avatar-badge has-story"
                                        } else {
                                            "feed-story-avatar-badge"
                                        };
                                        rsx! {
                                            button {
                                                key: "story-card-{fr.id}",
                                                r#type: "button",
                                                class: "feed-story-card story-card",
                                                role: "listitem",
                                                aria_label: "Tin của {shown}",
                                                onclick: move |_| {
                                                    story_reply_draft.set(String::new());
                                                    if let Some(st) = st_click.clone() {
                                                        viewing_story.set(Some(st));
                                                    } else if has_note {
                                                        viewing_friend_note.set(Some(fr_click.clone()));
                                                    } else {
                                                        on_open_friend.call(fr_click.clone());
                                                    }
                                                },
                                                if let Some(ref st) = friend_story {
                                                    if let Some(ref murl) = st.media_url.as_ref().filter(|s| !s.trim().is_empty()) {
                                                        img { class: "feed-story-bg-img", src: "{platform::resolve_asset_url(murl)}", alt: "{shown}", loading: "lazy" }
                                                    } else {
                                                        div { class: "feed-story-text-bg", style: "background: {grad};",
                                                            span { class: "feed-story-text-preview", "{st.content}" }
                                                        }
                                                    }
                                                } else if has_note {
                                                    div { class: "feed-story-text-bg", style: "background: {grad};",
                                                        span { class: "feed-story-text-preview", "{note_str}" }
                                                    }
                                                } else if let Some(ref aurl) = fr.avatar_url.as_ref().filter(|u| !u.trim().is_empty()) {
                                                    img { class: "feed-story-bg-img", src: "{platform::resolve_asset_url(aurl)}", alt: "{shown}", loading: "lazy" }
                                                } else {
                                                    div {
                                                        class: "feed-story-text-bg soft-fallback",
                                                        style: "background: linear-gradient(155deg, {fr_bg} 0%, color-mix(in srgb, {fr_bg} 62%, #0F172A) 100%);",
                                                        span { class: "feed-story-fallback-init", "{fr_init}" }
                                                    }
                                                }
                                                div { class: "feed-story-scrim" }
                                                span { class: "{badge_cls}",
                                                    Avatar { name: fr.username.clone(), avatar_url: fr.avatar_url.clone(), size: 36 }
                                                    if online {
                                                        span { class: "feed-story-online-dot", title: "Đang hoạt động" }
                                                    }
                                                }
                                                span { class: "feed-story-name", "{shown}" }
                                            }
                                        }
                                    }
                                }
                            }

                            button {
                                r#type: "button",
                                class: "feed-story-nav-btn prev",
                                aria_label: "Tin trước",
                                onclick: move |_| {
                                    let _ = dioxus::document::eval("const el = document.getElementById('feed-story-rail'); if (el) el.scrollBy({ left: -360, behavior: 'smooth' });");
                                },
                                ChevronLeftIcon { size: 20 }
                            }
                            button {
                                r#type: "button",
                                class: "feed-story-nav-btn next",
                                aria_label: "Tin tiếp theo",
                                onclick: move |_| {
                                    let _ = dioxus::document::eval("const el = document.getElementById('feed-story-rail'); if (el) el.scrollBy({ left: 360, behavior: 'smooth' });");
                                },
                                ChevronRightIcon { size: 20 }
                            }
                        }

                        if let Some(m) = err() {
                            div { class: "status-badge error", "{m}" }
                        }

                        if loading() && posts().is_empty() {
                            div { class: "feed-empty", "Đang tải bảng tin" }
                        } else if posts().is_empty() {
                            div { class: "feed-card feed-empty",
                                FeedThreadsIcon { size: 28 }
                                p { "Chưa có bài viết nào ở đây. Hãy chia sẻ bài viết đầu tiên của bạn." }
                            }
                        } else {
                            for p in posts() {
                                {
                                    let pid_like = p.id.clone();
                                    let pid_thread = p.id.clone();
                                    let pid_reply_btn = p.id.clone();
                                    let pid_del = p.id.clone();
                                    let pid_author = p.author_id.clone();
                                    let shown_name = profile_ext::nickname_of(&p.author_id)
                                        .or_else(|| p.author_display_name.clone().filter(|n| !n.trim().is_empty()))
                                        .unwrap_or_else(|| p.author_username.clone());
                                    let fx_cls = name_effect_class(Some(&p.author_name_effect));
                                    let is_mine = p.author_id == my_id;
                                    let time_str = format_relative(&p.created_at);
                                    let expanded_replies = open_threads.read().get(&p.id).cloned();
                                    let is_expanded = expanded_replies.is_some();
                                    let p_for_repost = p.clone();
                                    let r_draft = reply_drafts.read().get(&p.id).cloned().unwrap_or_default();

                                    rsx! {
                                        article { key: "{p.id}", class: "feed-card feed-post-card",
                                            div { class: "feed-post-grid",
                                                div { class: "feed-post-gutter",
                                                    button {
                                                        r#type: "button",
                                                        class: "feed-avatar-btn",
                                                        onclick: {
                                                            let uid = pid_author.clone();
                                                            move |_| on_open_profile.call(uid.clone())
                                                        },
                                                        Avatar { name: p.author_username.clone(), avatar_url: p.author_avatar_url.clone(), size: 42 }
                                                    }
                                                    if is_expanded {
                                                        div { class: "feed-thread-line" }
                                                    }
                                                }
                                                div { class: "feed-post-body-col",
                                                    div { class: "feed-post-head",
                                                        button {
                                                            r#type: "button",
                                                            class: "feed-author-btn",
                                                            onclick: {
                                                                let uid = pid_author.clone();
                                                                move |_| on_open_profile.call(uid.clone())
                                                            },
                                                            span { class: "feed-author-name {fx_cls}", "{shown_name}" }
                                                            span { class: "feed-author-handle", "@{p.author_username}" }
                                                        }
                                                        span { class: "feed-dot", "·" }
                                                        span { class: "feed-time", "{time_str}" }
                                                        span { class: "feed-vis-pill", "{vis_label(&p.visibility)}" }
                                                        div { style: "margin-left: auto;",
                                                            if is_mine {
                                                                button {
                                                                    r#type: "button",
                                                                    class: "icon-btn danger",
                                                                    title: "Xóa bài viết",
                                                                    aria_label: "Xóa bài viết",
                                                                    onclick: move |_| {
                                                                        let id = pid_del.clone();
                                                                        spawn(async move {
                                                                            if feed::delete_post(&id).await.is_ok() {
                                                                                posts.with_mut(|list| list.retain(|x| x.id != id));
                                                                            }
                                                                        });
                                                                    },
                                                                    TrashIcon { size: 15 }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if !p.content.trim().is_empty() {
                                                        div { class: "feed-post-text", "{p.content}" }
                                                    }
                                                    if let Some(ref img) = p.media_url {
                                                        div { class: "feed-post-media",
                                                            img { src: "{platform::resolve_asset_url(img)}", alt: "Ảnh bài viết", loading: "lazy" }
                                                        }
                                                    }
                                                    if let Some(ref rep) = p.repost_of {
                                                        div { class: "feed-quote-card",
                                                            div { class: "feed-quote-head",
                                                                Avatar { name: rep.author_username.clone(), avatar_url: rep.author_avatar_url.clone(), size: 20 }
                                                                strong { "{rep.author_display_name.clone().unwrap_or_else(|| rep.author_username.clone())}" }
                                                                span { class: "feed-author-handle", "@{rep.author_username}" }
                                                            }
                                                            if !rep.content.trim().is_empty() {
                                                                div { class: "feed-quote-text", "{rep.content}" }
                                                            }
                                                            if let Some(ref rimg) = rep.media_url {
                                                                div { class: "feed-quote-media",
                                                                    img { src: "{platform::resolve_asset_url(rimg)}", alt: "" }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    div { class: "feed-actions",
                                                        button {
                                                            r#type: "button",
                                                            class: if p.liked_by_me { "feed-act-btn liked" } else { "feed-act-btn" },
                                                            onclick: move |_| {
                                                                let id = pid_like.clone();
                                                                spawn(async move {
                                                                    if let Ok((liked, cnt)) = feed::toggle_like(&id).await {
                                                                        posts.with_mut(|list| {
                                                                            if let Some(item) = list.iter_mut().find(|x| x.id == id) {
                                                                                item.liked_by_me = liked;
                                                                                item.like_count = cnt;
                                                                            }
                                                                        });
                                                                    }
                                                                });
                                                            },
                                                            HeartIcon { size: 17, filled: p.liked_by_me }
                                                            if p.like_count > 0 { span { "{p.like_count}" } }
                                                        }
                                                        button {
                                                            r#type: "button",
                                                            class: if is_expanded { "feed-act-btn active" } else { "feed-act-btn" },
                                                            onclick: move |_| {
                                                                let id = pid_thread.clone();
                                                                if open_threads.read().contains_key(&id) {
                                                                    open_threads.with_mut(|m| { m.remove(&id); });
                                                                } else {
                                                                    open_threads.with_mut(|m| { m.insert(id.clone(), Vec::new()); });
                                                                    spawn(async move {
                                                                        if let Ok(reps) = feed::list_replies(&id).await {
                                                                            open_threads.with_mut(|m| { m.insert(id, reps); });
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            MessageIcon { size: 17 }
                                                            if p.reply_count > 0 { span { "{p.reply_count}" } }
                                                        }
                                                        button {
                                                            r#type: "button",
                                                            class: "feed-act-btn",
                                                            title: "Đăng lại",
                                                            onclick: move |_| {
                                                                repost_comment.set(String::new());
                                                                repost_target.set(Some(p_for_repost.clone()));
                                                            },
                                                            RepeatIcon { size: 17 }
                                                            if p.repost_count > 0 { span { "{p.repost_count}" } }
                                                        }
                                                    }

                                                    if let Some(reps) = expanded_replies {
                                                        div { class: "feed-replies-box",
                                                            for rp in reps {
                                                                {
                                                                    let rp_name = rp.author_display_name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| rp.author_username.clone());
                                                                    let rp_time = format_relative(&rp.created_at);
                                                                    let rp_id_del = rp.id.clone();
                                                                    let parent_id_ref = pid_reply_btn.clone();
                                                                    rsx! {
                                                                        div { key: "{rp.id}", class: "feed-reply-row",
                                                                            Avatar { name: rp.author_username.clone(), avatar_url: rp.author_avatar_url.clone(), size: 30 }
                                                                            div { class: "feed-reply-body",
                                                                                div { class: "feed-reply-head",
                                                                                    strong { "{rp_name}" }
                                                                                    span { class: "feed-time", "{rp_time}" }
                                                                                    if rp.author_id == my_id {
                                                                                        button {
                                                                                            r#type: "button",
                                                                                            class: "icon-btn danger",
                                                                                            style: "margin-left: auto; width: 24px; height: 24px;",
                                                                                            onclick: move |_| {
                                                                                                let rid = rp_id_del.clone();
                                                                                                let pid = parent_id_ref.clone();
                                                                                                spawn(async move {
                                                                                                    if feed::delete_post(&rid).await.is_ok() {
                                                                                                        if let Ok(next) = feed::list_replies(&pid).await {
                                                                                                            open_threads.with_mut(|m| { m.insert(pid.clone(), next); });
                                                                                                        }
                                                                                                        posts.with_mut(|list| {
                                                                                                            if let Some(it) = list.iter_mut().find(|x| x.id == pid) {
                                                                                                                it.reply_count = (it.reply_count - 1).max(0);
                                                                                                            }
                                                                                                        });
                                                                                                    }
                                                                                                });
                                                                                            },
                                                                                            TrashIcon { size: 13 }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                div { class: "feed-reply-text", "{rp.content}" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            div { class: "feed-reply-input-row",
                                                                input {
                                                                    r#type: "text",
                                                                    placeholder: "Trả lời vào luồng",
                                                                    value: "{r_draft}",
                                                                    oninput: {
                                                                        let pid = pid_reply_btn.clone();
                                                                        move |e| {
                                                                            let v = e.value();
                                                                            reply_drafts.with_mut(|m| { m.insert(pid.clone(), v); });
                                                                        }
                                                                    },
                                                                }
                                                                button {
                                                                    r#type: "button",
                                                                    class: "btn-primary",
                                                                    disabled: r_draft.trim().is_empty(),
                                                                    onclick: {
                                                                        let pid = pid_reply_btn.clone();
                                                                        let vis = p.visibility.clone();
                                                                        move |_| {
                                                                            let txt = reply_drafts.read().get(&pid).cloned().unwrap_or_default().trim().to_string();
                                                                            if txt.is_empty() { return; }
                                                                            let pid2 = pid.clone();
                                                                            let vis2 = vis.clone();
                                                                            spawn(async move {
                                                                                if feed::create_post(&txt, None, &vis2, Some(&pid2), None).await.is_ok() {
                                                                                    reply_drafts.with_mut(|m| { m.insert(pid2.clone(), String::new()); });
                                                                                    if let Ok(next) = feed::list_replies(&pid2).await {
                                                                                        open_threads.with_mut(|m| { m.insert(pid2.clone(), next); });
                                                                                    }
                                                                                    posts.with_mut(|list| {
                                                                                        if let Some(it) = list.iter_mut().find(|x| x.id == pid2) {
                                                                                            it.reply_count += 1;
                                                                                        }
                                                                                    });
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    SendIcon { size: 15 }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Modal: Tạo tin ảnh
        if create_story_open() {
            div { class: "modal-overlay", onclick: move |_| create_story_open.set(false),
                div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Tạo tin ảnh", onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h3 { class: "modal-title", "Tạo tin ảnh" }
                        button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| create_story_open.set(false), CloseIcon { size: 18 } }
                    }
                    div { style: "display: flex; flex-direction: column; gap: var(--space-3);",
                        if let Some(murl) = story_media_url() {
                            div { style: "position: relative; max-height: 220px; overflow: hidden; border-radius: var(--radius-lg);",
                                img { src: "{platform::resolve_asset_url(&murl)}", alt: "Ảnh tin", style: "width: 100%; object-fit: cover;" }
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    style: "position: absolute; top: 6px; right: 6px; background: rgba(0,0,0,0.6); color: #fff;",
                                    aria_label: "Xóa ảnh",
                                    onclick: move |_| story_media_url.set(None),
                                    CloseIcon { size: 14 }
                                }
                            }
                        } else {
                            div { class: "inline-form",
                                input {
                                    id: "feed-story-media-upload",
                                    r#type: "file",
                                    accept: "image/png,image/jpeg,image/webp,image/gif,image/avif",
                                    style: "display: none;",
                                    onchange: move |_| {
                                        story_uploading.set(true);
                                        spawn(async move {
                                            if let Ok((bytes, mime)) = platform::read_selected_file_raw("feed-story-media-upload").await {
                                                match library::upload_media(bytes, &mime).await {
                                                    Ok(url) => { story_media_url.set(Some(url)); story_err.set(None); }
                                                    Err(_) => story_err.set(Some("Không tải ảnh lên được".into())),
                                                }
                                            }
                                            story_uploading.set(false);
                                        });
                                    },
                                }
                                label {
                                    r#for: "feed-story-media-upload",
                                    class: "btn-secondary",
                                    style: "display: inline-flex; align-items: center; gap: 6px; cursor: pointer; width: 100%; justify-content: center;",
                                    ImageIcon { size: 18 }
                                    if story_uploading() { "Đang tải ảnh lên" } else { "Chọn ảnh cho tin mới" }
                                }
                            }
                        }
                        input {
                            r#type: "text",
                            maxlength: "200",
                            placeholder: "Thêm chú thích cho tin",
                            aria_label: "Chú thích tin",
                            value: "{story_draft}",
                            oninput: move |e| story_draft.set(e.value()),
                        }
                        if let Some(e) = story_err() {
                            div { class: "status-badge error", "{e}" }
                        }
                        div { class: "modal-actions",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| create_story_open.set(false), "Hủy" }
                            button {
                                r#type: "button",
                                class: "btn-primary",
                                disabled: story_posting() || story_uploading() || (story_draft().trim().is_empty() && story_media_url().is_none()),
                                onclick: move |_| {
                                    let txt = story_draft().trim().to_string();
                                    let mu = story_media_url();
                                    story_posting.set(true);
                                    story_err.set(None);
                                    spawn(async move {
                                        match feed::create_post(&txt, mu.as_deref(), "friends", None, None).await {
                                            Ok(_) => {
                                                story_draft.set(String::new());
                                                story_media_url.set(None);
                                                create_story_open.set(false);
                                                load_posts.call(filter());
                                            }
                                            Err(e) => story_err.set(Some(format!("Đăng tin thất bại: {}", e.code))),
                                        }
                                        story_posting.set(false);
                                    });
                                },
                                if story_posting() { "Đang đăng" } else { "Đăng tin" }
                            }
                        }
                    }
                }
            }
        }

        // Modal: Chia sẻ suy nghĩ
        if thought_open() {
            div { class: "modal-overlay", onclick: move |_| thought_open.set(false),
                div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Chia sẻ suy nghĩ", onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h3 { class: "modal-title", "Chia sẻ suy nghĩ" }
                        button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| thought_open.set(false), CloseIcon { size: 18 } }
                    }
                    div { class: "note-emoji-row",
                        for em in QUICK_NOTE_EMOJIS {
                            {
                                let active = note_emoji().trim() == em;
                                rsx! {
                                    button {
                                        key: "feed-em-{em}",
                                        r#type: "button",
                                        class: if active { "note-emoji-chip active" } else { "note-emoji-chip" },
                                        onclick: move |_| note_emoji.set(em.to_string()),
                                        "{em}"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "note-input-row",
                        input {
                            class: "note-emoji-input",
                            r#type: "text",
                            maxlength: "8",
                            placeholder: "😊",
                            aria_label: "Biểu tượng cảm xúc",
                            value: "{note_emoji}",
                            oninput: move |e| note_emoji.set(e.value()),
                        }
                        input {
                            r#type: "text",
                            maxlength: "60",
                            placeholder: "Chia sẻ suy nghĩ với bạn bè",
                            aria_label: "Nội dung ghi chú",
                            value: "{note_text}",
                            oninput: move |e| note_text.set(e.value()),
                        }
                    }
                    div { class: "note-expiry-row",
                        span { "Hiển thị trong" }
                        select {
                            value: "{note_expiry}",
                            onchange: move |e| {
                                if let Ok(v) = e.value().parse::<i64>() {
                                    note_expiry.set(v);
                                }
                            },
                            option { value: "60", "1 giờ" }
                            option { value: "240", "4 giờ" }
                            option { value: "1440", "24 giờ" }
                            option { value: "0", "Không tự xóa" }
                        }
                    }
                    if let Some(e) = note_err() {
                        div { class: "status-badge error", "{e}" }
                    }
                    div { class: "modal-actions",
                        if my_has_note {
                            button {
                                r#type: "button",
                                class: "btn-ghost",
                                disabled: note_saving(),
                                onclick: move |_| {
                                    note_saving.set(true);
                                    note_err.set(None);
                                    spawn(async move {
                                        let r1 = api::update_my_profile(None, None, Some(""), Some(""), None, None).await;
                                        let r2 = profile_ext::update_me(json!({ "status_expires_in_minutes": 0 })).await;
                                        note_saving.set(false);
                                        if r1.is_ok() && r2.is_ok() {
                                            local_status_text.set(None);
                                            local_status_emoji.set(None);
                                            on_my_status_updated.call((None, None));
                                            thought_open.set(false);
                                        } else {
                                            note_err.set(Some("Không xóa được ghi chú".into()));
                                        }
                                    });
                                },
                                "Xóa tin"
                            }
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| thought_open.set(false), "Hủy" }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            disabled: note_saving() || (note_text().trim().is_empty() && note_emoji().trim().is_empty()),
                            onclick: move |_| {
                                let t = note_text().trim().to_string();
                                let em = note_emoji().trim().to_string();
                                let exp = note_expiry();
                                note_saving.set(true);
                                note_err.set(None);
                                spawn(async move {
                                    let r1 = api::update_my_profile(None, None, Some(&t), Some(&em), None, None).await;
                                    let r2 = profile_ext::update_me(json!({ "status_expires_in_minutes": exp })).await;
                                    note_saving.set(false);
                                    if r1.is_ok() && r2.is_ok() {
                                        let nt = (!t.is_empty()).then_some(t);
                                        let ne = (!em.is_empty()).then_some(em);
                                        local_status_text.set(nt.clone());
                                        local_status_emoji.set(ne.clone());
                                        on_my_status_updated.call((nt, ne));
                                        thought_open.set(false);
                                    } else {
                                        note_err.set(Some("Không lưu được ghi chú".into()));
                                    }
                                });
                            },
                            "Chia sẻ"
                        }
                    }
                }
            }
        }

        // Modal: Xem tin (Story Viewer)
        if let Some(st) = viewing_story() {
            {
                let st_author_name = st.author_display_name.clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| st.author_username.clone());
                let is_my_story = st.author_id == my_id || st.author_username == my_username;
                let st_id_del = st.id.clone();
                let st_time = format_relative(&st.created_at);
                let st_friend = friends.iter().find(|f| f.id == st.author_id || f.username == st.author_username).cloned();
                rsx! {
                    div { class: "modal-overlay", onclick: move |_| viewing_story.set(None),
                        div { class: "modal-dialog story-viewer-modal", role: "dialog", aria_modal: "true", aria_label: "Tin của {st_author_name}", onclick: move |e| e.stop_propagation(),
                            div { class: "modal-header",
                                div { style: "display: flex; align-items: center; gap: 10px;",
                                    Avatar { name: st.author_username.clone(), avatar_url: st.author_avatar_url.clone(), size: 36 }
                                    div {
                                        div { class: "modal-title", style: "font-size: 1rem;", "{st_author_name}" }
                                        div { class: "member-sub", "@{st.author_username} · {st_time}" }
                                    }
                                }
                                button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| viewing_story.set(None), CloseIcon { size: 18 } }
                            }
                            if let Some(ref murl) = st.media_url.as_ref().filter(|s| !s.trim().is_empty()) {
                                div { class: "story-viewer-media",
                                    img { src: "{platform::resolve_asset_url(murl)}", alt: "Ảnh tin" }
                                }
                            }
                            if !st.content.trim().is_empty() {
                                div { class: "story-viewer-caption", "{st.content}" }
                            }
                            if !is_my_story {
                                if let Some(fr_target) = st_friend {
                                    div { class: "note-input-row",
                                        input {
                                            r#type: "text",
                                            placeholder: "Trả lời tin của {st_author_name}",
                                            value: "{story_reply_draft}",
                                            oninput: move |e| story_reply_draft.set(e.value()),
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-primary",
                                            disabled: story_reply_draft().trim().is_empty(),
                                            onclick: move |_| {
                                                let txt = story_reply_draft().trim().to_string();
                                                if !txt.is_empty() {
                                                    on_note_reply.call((fr_target.clone(), txt));
                                                    story_reply_draft.set(String::new());
                                                    viewing_story.set(None);
                                                }
                                            },
                                            "Gửi"
                                        }
                                    }
                                }
                            }
                            div { class: "modal-actions",
                                if is_my_story {
                                    button {
                                        r#type: "button",
                                        class: "btn-ghost danger",
                                        onclick: move |_| {
                                            let id = st_id_del.clone();
                                            spawn(async move {
                                                if feed::delete_post(&id).await.is_ok() {
                                                    posts.with_mut(|list| list.retain(|x| x.id != id));
                                                    viewing_story.set(None);
                                                }
                                            });
                                        },
                                        "Xóa tin"
                                    }
                                }
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| viewing_story.set(None), "Đóng" }
                            }
                        }
                    }
                }
            }
        }

        // Modal: Xem ghi chú bạn bè
        if let Some(vf) = viewing_friend_note() {
            {
                let shown = profile_ext::nickname_of(&vf.id)
                    .or_else(|| vf.display_name.clone().filter(|n| !n.trim().is_empty()))
                    .unwrap_or_else(|| vf.username.clone());
                let full_note = format!(
                    "{} {}",
                    vf.status_emoji.as_deref().unwrap_or("").trim(),
                    vf.status_text.as_deref().unwrap_or("").trim()
                )
                .trim()
                .to_string();
                let vf_open = vf.clone();
                let vf_reply = vf.clone();
                rsx! {
                    div { class: "modal-overlay", onclick: move |_| viewing_friend_note.set(None),
                        div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Ghi chú của {shown}", onclick: move |e| e.stop_propagation(),
                            div { class: "modal-header",
                                div { style: "display: flex; align-items: center; gap: 10px;",
                                    Avatar { name: vf.username.clone(), avatar_url: vf.avatar_url.clone(), size: 36 }
                                    div {
                                        div { class: "modal-title", style: "font-size: 1rem;", "{shown}" }
                                        div { class: "member-sub", "@{vf.username}" }
                                    }
                                }
                                button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| viewing_friend_note.set(None), CloseIcon { size: 18 } }
                            }
                            div { class: "note-view-card", "{full_note}" }
                            div { class: "note-input-row",
                                input {
                                    r#type: "text",
                                    placeholder: "Trả lời ghi chú của {shown}",
                                    value: "{story_reply_draft}",
                                    oninput: move |e| story_reply_draft.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    disabled: story_reply_draft().trim().is_empty(),
                                    onclick: move |_| {
                                        let txt = story_reply_draft().trim().to_string();
                                        if !txt.is_empty() {
                                            on_note_reply.call((vf_reply.clone(), txt));
                                            story_reply_draft.set(String::new());
                                            viewing_friend_note.set(None);
                                        }
                                    },
                                    "Gửi"
                                }
                            }
                            div { class: "modal-actions",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| viewing_friend_note.set(None), "Đóng" }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        viewing_friend_note.set(None);
                                        on_open_friend.call(vf_open.clone());
                                    },
                                    "Nhắn tin"
                                }
                            }
                        }
                    }
                }
            }
        }

        // Modal: Đăng lại bài viết
        if let Some(rt) = repost_target() {
            div { class: "modal-overlay", onclick: move |_| repost_target.set(None),
                div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Đăng lại bài viết", onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "Đăng lại bài viết" }
                        button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| repost_target.set(None), CloseIcon { size: 18 } }
                    }
                    textarea {
                        rows: "3",
                        placeholder: "Thêm suy nghĩ của bạn (tùy chọn)",
                        value: "{repost_comment}",
                        oninput: move |e| repost_comment.set(e.value()),
                    }
                    div { class: "feed-quote-card",
                        strong { "@{rt.author_username}" }
                        div { class: "feed-quote-text", "{rt.content}" }
                    }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| repost_target.set(None), "Hủy" }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: move |_| {
                                let txt = repost_comment().trim().to_string();
                                let rid = rt.id.clone();
                                let vis = visibility();
                                repost_target.set(None);
                                spawn(async move {
                                    if feed::create_post(&txt, None, &vis, None, Some(&rid)).await.is_ok() {
                                        load_posts.call(filter());
                                    }
                                });
                            },
                            "Đăng lại"
                        }
                    }
                }
            }
        }
    }
}
