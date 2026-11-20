//! Google Meet / Zoom-style Group Call & Meeting Link Hub ("Họp nhóm").
//! Allows 1-click meeting link creation, joining via link or room code, and rejoining active rooms.

use dioxus::prelude::*;

use crate::{
    components::icons::{
        BackIcon, CheckIcon, CopyIcon, LinkIcon, MicIcon, PlusIcon, RefreshIcon, VideoIcon,
    },
    models::CallSessionItem,
    platform,
    services::calls,
};

fn build_meet_link(origin: &str, code: &str) -> String {
    if origin.is_empty() {
        format!("http://127.0.0.1:8085/?meet={code}")
    } else {
        format!("{}/?meet={code}", origin.trim_end_matches('/'))
    }
}

#[component]
pub fn MeetView(
    my_id: String,
    my_username: String,
    my_avatar_url: Option<String>,
    on_back: EventHandler<()>,
    on_join_session: EventHandler<CallSessionItem>,
) -> Element {
    let _ = (&my_id, &my_avatar_url);
    let mut origin = use_signal(|| "http://127.0.0.1:8085".to_string());
    let mut meet_title = use_signal(String::new);
    let mut call_type = use_signal(|| "video".to_string());
    let mut creating = use_signal(|| false);
    let mut created_session = use_signal(|| Option::<CallSessionItem>::None);
    let mut copied_key = use_signal(|| Option::<String>::None);

    let mut join_input = use_signal(String::new);
    let mut joining = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let mut recent_meets = use_signal(Vec::<CallSessionItem>::new);
    let mut loading_list = use_signal(|| true);

    let reload_meets = use_callback(move |_: ()| {
        loading_list.set(true);
        spawn(async move {
            let org = platform::app_origin().await;
            if !org.is_empty() {
                origin.set(org);
            }
            if let Ok(list) = calls::calls_list_meets().await {
                recent_meets.set(list);
            }
            loading_list.set(false);
        });
    });

    use_effect(move || {
        reload_meets.call(());
    });

    let copy_text = use_callback(move |(text, key): (String, String)| {
        platform::copy_to_clipboard(&text);
        copied_key.set(Some(key.clone()));
        spawn(async move {
            platform::sleep_ms(2000).await;
            if copied_key().as_deref() == Some(key.as_str()) {
                copied_key.set(None);
            }
        });
    });

    let do_join = use_callback(move |raw: String| {
        let code = calls::extract_meet_code(&raw);
        if code.is_empty() {
            error_msg.set(Some("Vui lòng nhập mã phòng hoặc dán đường dẫn hợp lệ".into()));
            return;
        }
        joining.set(true);
        error_msg.set(None);
        spawn(async move {
            match calls::calls_join_meet(&code).await {
                Ok(sess) => {
                    joining.set(false);
                    on_join_session.call(sess);
                }
                Err(_) => {
                    joining.set(false);
                    error_msg.set(Some(
                        "Không tìm thấy phòng họp với mã hoặc đường dẫn này".into(),
                    ));
                }
            }
        });
    });

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column meet-column",
                header { class: "chat-header only-mobile",
                    button {
                        r#type: "button",
                        class: "icon-btn back-btn",
                        aria_label: "Quay lại",
                        onclick: move |_| on_back.call(()),
                        BackIcon {}
                    }
                    h2 { class: "chat-header-title", "Họp nhóm" }
                }

                div { class: "feed-scroll",
                    div { class: "feed-inner meet-inner",
                        // Hero header
                        div { class: "feed-card meet-hero-card",
                            div { class: "meet-hero-icon",
                                VideoIcon { size: 28 }
                            }
                            div { class: "meet-hero-text",
                                h1 { class: "meet-hero-title", "Cuộc họp & Gọi nhóm trực tuyến" }
                                p { class: "meet-hero-sub",
                                    "Tạo đường dẫn phòng họp chỉ với 1 chạm như Google Meet & Zoom. Gửi link cho bất kỳ ai để tham gia gọi thoại, video và chia sẻ màn hình ngay lập tức."
                                }
                            }
                        }

                        if let Some(err) = error_msg() {
                            div { class: "status-badge error", "{err}" }
                        }

                        // Grid: Create Meeting & Join Meeting
                        div { class: "meet-action-grid",
                            // Card 1: Create new meeting
                            div { class: "feed-card meet-card",
                                div { class: "meet-card-head",
                                    span { class: "meet-card-badge", PlusIcon { size: 16 } }
                                    div {
                                        h2 { class: "meet-card-title", "Tạo cuộc họp mới" }
                                        p { class: "meet-card-desc", "Tạo mã phòng và đường dẫn để mời mọi người tham gia" }
                                    }
                                }

                                div { class: "meet-form",
                                    label { class: "form-field",
                                        span { class: "form-label", "Tên cuộc họp" }
                                        input {
                                            r#type: "text",
                                            maxlength: "110",
                                            placeholder: "Cuộc họp của {my_username}",
                                            aria_label: "Tên cuộc họp",
                                            value: "{meet_title}",
                                            oninput: move |e| meet_title.set(e.value()),
                                        }
                                    }

                                    div { class: "meet-mode-row",
                                        button {
                                            r#type: "button",
                                            class: if call_type() == "video" { "meet-mode-chip active" } else { "meet-mode-chip" },
                                            onclick: move |_| call_type.set("video".into()),
                                            VideoIcon { size: 16 }
                                            "Gọi Video & Thoại"
                                        }
                                        button {
                                            r#type: "button",
                                            class: if call_type() == "audio" { "meet-mode-chip active" } else { "meet-mode-chip" },
                                            onclick: move |_| call_type.set("audio".into()),
                                            MicIcon { size: 16 }
                                            "Chỉ âm thanh"
                                        }
                                    }

                                    div { class: "meet-btn-row",
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            disabled: creating(),
                                            onclick: move |_| {
                                                let t = meet_title().trim().to_string();
                                                let ct = call_type();
                                                creating.set(true);
                                                error_msg.set(None);
                                                spawn(async move {
                                                    let title_opt = (!t.is_empty()).then_some(t);
                                                    match calls::calls_create_meet(title_opt, &ct).await {
                                                        Ok(sess) => {
                                                            if let Some(code) = sess.meet_code.as_deref() {
                                                                let link = build_meet_link(&origin(), code);
                                                                platform::copy_to_clipboard(&link);
                                                                copied_key.set(Some("created-link".into()));
                                                            }
                                                            created_session.set(Some(sess));
                                                            reload_meets.call(());
                                                        }
                                                        Err(e) => error_msg.set(Some(format!("Không tạo được phòng họp: {}", e.code))),
                                                    }
                                                    creating.set(false);
                                                });
                                            },
                                            LinkIcon { size: 16 }
                                            if creating() { "Đang tạo" } else { "Tạo link phòng họp" }
                                        }

                                        button {
                                            r#type: "button",
                                            class: "btn-primary",
                                            disabled: creating(),
                                            onclick: move |_| {
                                                let t = meet_title().trim().to_string();
                                                let ct = call_type();
                                                creating.set(true);
                                                error_msg.set(None);
                                                spawn(async move {
                                                    let title_opt = (!t.is_empty()).then_some(t);
                                                    match calls::calls_create_meet(title_opt, &ct).await {
                                                        Ok(sess) => {
                                                            created_session.set(Some(sess.clone()));
                                                            reload_meets.call(());
                                                            on_join_session.call(sess);
                                                        }
                                                        Err(e) => error_msg.set(Some(format!("Không tạo được phòng họp: {}", e.code))),
                                                    }
                                                    creating.set(false);
                                                });
                                            },
                                            VideoIcon { size: 16 }
                                            "Bắt đầu họp ngay"
                                        }
                                    }
                                }

                                // Created link result box
                                if let Some(sess) = created_session() {
                                    {
                                        let code = sess.meet_code.clone().unwrap_or_else(|| sess.id.clone());
                                        let link = build_meet_link(&origin(), &code);
                                        let link_for_copy = link.clone();
                                        let code_for_copy = code.clone();
                                        let sess_join = sess.clone();
                                        let title_disp = sess.meet_title.clone().unwrap_or_else(|| "Cuộc họp mới".into());
                                        rsx! {
                                            div { class: "meet-created-box",
                                                div { class: "meet-created-header",
                                                    span { class: "meet-created-title", "{title_disp}" }
                                                    span { class: "meet-code-pill", "Mã: {code}" }
                                                }
                                                div { class: "meet-link-row",
                                                    input {
                                                        class: "meet-link-input",
                                                        r#type: "text",
                                                        readonly: true,
                                                        aria_label: "Đường dẫn phòng họp",
                                                        value: "{link}",
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-secondary",
                                                        aria_label: "Sao chép link phòng họp",
                                                        onclick: move |_| copy_text.call((link_for_copy.clone(), "created-link".into())),
                                                        if copied_key().as_deref() == Some("created-link") {
                                                            CheckIcon { size: 15 }
                                                            "Đã chép link"
                                                        } else {
                                                            CopyIcon { size: 15 }
                                                            "Sao chép link"
                                                        }
                                                    }
                                                }
                                                div { class: "meet-created-actions",
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-ghost",
                                                        onclick: move |_| copy_text.call((code_for_copy.clone(), "created-code".into())),
                                                        if copied_key().as_deref() == Some("created-code") {
                                                            CheckIcon { size: 14 }
                                                            "Đã chép mã"
                                                        } else {
                                                            CopyIcon { size: 14 }
                                                            "Chép mã phòng"
                                                        }
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-primary",
                                                        onclick: move |_| on_join_session.call(sess_join.clone()),
                                                        VideoIcon { size: 16 }
                                                        "Vào phòng ngay"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Card 2: Join existing meeting by link or code
                            div { class: "feed-card meet-card",
                                div { class: "meet-card-head",
                                    span { class: "meet-card-badge join", LinkIcon { size: 16 } }
                                    div {
                                        h2 { class: "meet-card-title", "Tham gia cuộc họp" }
                                        p { class: "meet-card-desc", "Dán đường dẫn phòng họp hoặc nhập mã phòng để kết nối ngay" }
                                    }
                                }

                                form {
                                    class: "meet-form",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                        do_join.call(join_input());
                                    },
                                    label { class: "form-field",
                                        span { class: "form-label", "Đường dẫn hoặc mã phòng" }
                                        input {
                                            r#type: "text",
                                            placeholder: "Ví dụ: abc-defg-hij hoặc http://.../?meet=abc-defg-hij",
                                            aria_label: "Nhập mã hoặc link phòng họp",
                                            value: "{join_input}",
                                            oninput: move |e| join_input.set(e.value()),
                                        }
                                    }
                                    button {
                                        r#type: "submit",
                                        class: "btn-primary",
                                        style: "width: 100%; justify-content: center;",
                                        disabled: joining() || join_input().trim().is_empty(),
                                        VideoIcon { size: 16 }
                                        if joining() { "Đang kết nối" } else { "Tham gia ngay" }
                                    }
                                }
                            }
                        }

                        // Recent / Active rooms list
                        div { class: "feed-card meet-list-card",
                            div { class: "meet-list-header",
                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    h2 { class: "meet-card-title", "Phòng họp gần đây & đang hoạt động" }
                                }
                                button {
                                    r#type: "button",
                                    class: "icon-btn",
                                    title: "Làm mới danh sách",
                                    aria_label: "Làm mới danh sách",
                                    onclick: move |_| reload_meets.call(()),
                                    RefreshIcon { size: 16 }
                                }
                            }

                            if loading_list() && recent_meets().is_empty() {
                                div { class: "feed-empty", "Đang tải danh sách phòng họp" }
                            } else if recent_meets().is_empty() {
                                div { class: "feed-empty", "Chưa có phòng họp nào. Hãy tạo cuộc họp đầu tiên ở trên." }
                            } else {
                                div { class: "meet-room-list",
                                    for m in recent_meets() {
                                        {
                                            let code = m.meet_code.clone().unwrap_or_else(|| m.id.clone());
                                            let link = build_meet_link(&origin(), &code);
                                            let title_str = m.meet_title.clone().unwrap_or_else(|| format!("Cuộc họp của {}", m.caller_name));
                                            let active_count = m.participants.iter().filter(|p| p.left_at.is_none()).count();
                                            let is_active = m.status == "active" && active_count > 0;
                                            let copy_k = format!("list-{code}");
                                            let copy_k2 = copy_k.clone();
                                            let code_join = code.clone();
                                            rsx! {
                                                div { key: "{m.id}", class: "meet-room-item",
                                                    div { class: "meet-room-info",
                                                        div { class: "meet-room-top",
                                                            span { class: "meet-room-title", "{title_str}" }
                                                            span { class: "meet-code-pill", "{code}" }
                                                            if is_active {
                                                                span { class: "meet-status-pill live", "Đang họp · {active_count} người" }
                                                            }
                                                        }
                                                        div { class: "meet-room-meta",
                                                            "Chủ phòng: {m.caller_name} · Link: {link}"
                                                        }
                                                    }
                                                    div { class: "meet-room-actions",
                                                        button {
                                                            r#type: "button",
                                                            class: "btn-secondary",
                                                            onclick: move |_| copy_text.call((link.clone(), copy_k.clone())),
                                                            if copied_key().as_deref() == Some(copy_k2.as_str()) {
                                                                CheckIcon { size: 14 }
                                                                "Đã chép"
                                                            } else {
                                                                CopyIcon { size: 14 }
                                                                "Chép link"
                                                            }
                                                        }
                                                        button {
                                                            r#type: "button",
                                                            class: "btn-primary",
                                                            onclick: move |_| do_join.call(code_join.clone()),
                                                            VideoIcon { size: 15 }
                                                            "Tham gia"
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
