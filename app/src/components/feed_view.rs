//! Spec 016: Threads-style Social Feed ("Mới") backed by `/api/feed` and `StoriesNotesBar`.

use std::collections::HashMap;

use chrono::{DateTime, Local, Utc};
use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{
            BackIcon, CloseIcon, FeedThreadsIcon, HeartIcon, ImageIcon, MessageIcon, RepeatIcon,
            SendIcon, TrashIcon,
        },
        navigation::conversations::StoriesNotesBar,
        profile_card::name_effect_class,
    },
    models::FriendItem,
    platform,
    services::{
        feed::{self, FeedPostItem},
        library,
    },
};

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
    let mut draft = use_signal(String::new);
    let mut media_url = use_signal(|| Option::<String>::None);
    let mut visibility = use_signal(|| "friends".to_string());
    let mut posting = use_signal(|| false);
    let mut uploading_img = use_signal(|| false);
    let mut err = use_signal(|| Option::<String>::None);

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
                        // Stories / Ghi chú bar
                        div { class: "feed-card feed-stories-card",
                            StoriesNotesBar {
                                strip_id: "feed-notes-strip".to_string(),
                                my_username: my_username.clone(),
                                my_avatar_url: my_avatar_url.clone(),
                                my_status_text: my_status_text.clone(),
                                my_status_emoji: my_status_emoji.clone(),
                                friends: friends.clone(),
                                on_my_status_updated: on_my_status_updated,
                                on_open_friend: on_open_friend,
                                on_note_reply: on_note_reply,
                            }
                        }

                        // Compose card
                        div { class: "feed-card feed-compose-card",
                            div { class: "feed-compose-top",
                                Avatar { name: my_username.clone(), avatar_url: my_avatar_url.clone(), size: 42 }
                                div { class: "feed-compose-main",
                                    div { class: "feed-compose-author", "{my_username}" }
                                    textarea {
                                        class: "feed-compose-input",
                                        rows: "2",
                                        maxlength: "1000",
                                        placeholder: "Có gì mới?",
                                        aria_label: "Viết bài mới",
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
                                            input {
                                                id: "feed-media-upload",
                                                r#type: "file",
                                                accept: "image/png,image/jpeg,image/webp,image/gif,image/avif",
                                                style: "display: none;",
                                                onchange: move |_| {
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
                                                class: "icon-btn",
                                                title: "Thêm ảnh",
                                                aria_label: "Thêm ảnh",
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
                                    let shown_name = crate::services::profile_ext::nickname_of(&p.author_id)
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
