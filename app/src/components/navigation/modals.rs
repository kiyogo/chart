use dioxus::prelude::*;

use crate::{
    components::icons::CloseIcon,
    models::{ChannelItem, ServerCategoryItem, ServerItem},
    platform,
    services::api,
};

#[component]
pub fn CommunityModal(
    on_close: EventHandler<()>,
    on_server_ready: EventHandler<ServerItem>,
) -> Element {
    let mut tab = use_signal(|| "create"); // "create" | "join" | "discover"
    let mut name = use_signal(String::new);
    let mut invite_code = use_signal(String::new);
    let mut discover_list = use_signal(Vec::<ServerItem>::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);

    use_effect(move || {
        if tab() == "discover" {
            spawn(async move {
                if let Ok(list) = api::servers_discover().await {
                    discover_list.set(list);
                }
            });
        }
    });

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h3 { class: "modal-title", "Cộng đồng" }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        aria_label: "Đóng",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                }
                div { class: "modal-body",
                    div { style: "display: flex; gap: 6px;",
                        button {
                            r#type: "button",
                            class: if tab() == "create" { "btn-primary" } else { "btn-secondary" },
                            onclick: move |_| { tab.set("create"); error.set(None); },
                            "Tạo mới"
                        }
                        button {
                            r#type: "button",
                            class: if tab() == "join" { "btn-primary" } else { "btn-secondary" },
                            onclick: move |_| { tab.set("join"); error.set(None); },
                            "Nhập mã mời"
                        }
                        button {
                            r#type: "button",
                            class: if tab() == "discover" { "btn-primary" } else { "btn-secondary" },
                            onclick: move |_| { tab.set("discover"); error.set(None); },
                            "Khám phá"
                        }
                    }

                    if tab() == "create" {
                        form {
                            style: "display: flex; flex-direction: column; gap: 12px;",
                            onsubmit: move |e| {
                                e.prevent_default();
                                let n = name().trim().to_string();
                                if n.len() < 2 {
                                    error.set(Some("Tên cộng đồng phải từ 2 ký tự trở lên".into()));
                                    return;
                                }
                                loading.set(true);
                                error.set(None);
                                spawn(async move {
                                    match api::servers_create(&n).await {
                                        Ok(srv) => on_server_ready.call(srv),
                                        Err(err) => error.set(Some(format!("Không thể tạo cộng đồng: {}", err.code))),
                                    }
                                    loading.set(false);
                                });
                            },
                            div { class: "form-field",
                                label { class: "form-label", "Tên cộng đồng" }
                                input {
                                    r#type: "text",
                                    placeholder: "Ví dụ: Cộng đồng Rust Việt Nam",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value()),
                                }
                            }
                            if let Some(err) = error() {
                                div { class: "status-badge error", "{err}" }
                            }
                            div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy" }
                                button {
                                    r#type: "submit",
                                    class: "btn-primary",
                                    disabled: loading() || name().trim().len() < 2,
                                    if loading() { "Đang tạo" } else { "Tạo cộng đồng" }
                                }
                            }
                        }
                    } else if tab() == "join" {
                        form {
                            style: "display: flex; flex-direction: column; gap: 12px;",
                            onsubmit: move |e| {
                                e.prevent_default();
                                let code = invite_code().trim().to_string();
                                if code.is_empty() {
                                    return;
                                }
                                loading.set(true);
                                error.set(None);
                                spawn(async move {
                                    match api::servers_join(&code).await {
                                        Ok(srv) => on_server_ready.call(srv),
                                        Err(_) => error.set(Some("Mã mời không hợp lệ hoặc đã hết hạn".into())),
                                    }
                                    loading.set(false);
                                });
                            },
                            div { class: "form-field",
                                label { class: "form-label", "Mã mời" }
                                input {
                                    r#type: "text",
                                    placeholder: "Nhập mã mời (ví dụ: a1b2c3d4)",
                                    value: "{invite_code}",
                                    oninput: move |e| invite_code.set(e.value()),
                                }
                            }
                            if let Some(err) = error() {
                                div { class: "status-badge error", "{err}" }
                            }
                            div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy" }
                                button {
                                    r#type: "submit",
                                    class: "btn-primary",
                                    disabled: loading() || invite_code().trim().is_empty(),
                                    if loading() { "Đang tham gia" } else { "Tham gia" }
                                }
                            }
                        }
                    } else {
                        div { style: "display: flex; flex-direction: column; gap: 8px; max-height: 280px; overflow-y: auto;",
                            if discover_list.read().is_empty() {
                                div { class: "empty-chat-inner", "Chưa có cộng đồng công khai" }
                            } else {
                                for srv in discover_list() {
                                    {
                                        let sid = srv.id.clone();
                                        rsx! {
                                            div { class: "request-card", key: "{srv.id}",
                                                span { class: "chat-item-name", "{srv.name}" }
                                                button {
                                                    r#type: "button",
                                                    class: "btn-small-primary",
                                                    onclick: move |_| {
                                                        let id = sid.clone();
                                                        spawn(async move {
                                                            if let Ok(joined) = api::servers_join_direct(&id).await {
                                                                on_server_ready.call(joined);
                                                            }
                                                        });
                                                    },
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

#[component]
pub fn CreateCategoryModal(
    server_id: String,
    on_close: EventHandler<()>,
    on_created: EventHandler<ServerCategoryItem>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h3 { class: "modal-title", "Tạo danh mục kênh" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                form {
                    class: "modal-body",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let n = name().trim().to_string();
                        if n.is_empty() {
                            return;
                        }
                        let sid = server_id.clone();
                        loading.set(true);
                        spawn(async move {
                            match api::servers_create_category(&sid, &n).await {
                                Ok(cat) => on_created.call(cat),
                                Err(err) => error.set(Some(format!("Không thể tạo danh mục: {}", err.code))),
                            }
                            loading.set(false);
                        });
                    },
                    div { class: "form-field",
                        label { class: "form-label", "Tên danh mục" }
                        input {
                            r#type: "text",
                            placeholder: "Ví dụ: THÔNG TIN, TRÒ CHUYỆN, GIỌNG NÓI",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    }
                    div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy" }
                        button { r#type: "submit", class: "btn-primary", disabled: loading() || name().trim().is_empty(), "Tạo danh mục" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CreateChannelModal(
    server_id: String,
    category_id: Option<String>,
    on_close: EventHandler<()>,
    on_created: EventHandler<ChannelItem>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut channel_type = use_signal(|| "text".to_string());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h3 { class: "modal-title", "Tạo kênh mới" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                form {
                    class: "modal-body",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let n = name().trim().to_string();
                        if n.is_empty() {
                            return;
                        }
                        let sid = server_id.clone();
                        let cid = category_id.clone();
                        let ctype = channel_type();
                        loading.set(true);
                        spawn(async move {
                            match api::servers_create_channel(&sid, &n, cid.as_deref(), &ctype).await {
                                Ok(ch) => on_created.call(ch),
                                Err(err) => error.set(Some(format!("Không thể tạo kênh: {}", err.code))),
                            }
                            loading.set(false);
                        });
                    },
                    div { class: "form-field",
                        label { class: "form-label", "Loại kênh" }
                        select {
                            value: "{channel_type}",
                            onchange: move |e| channel_type.set(e.value()),
                            option { value: "text", "Kênh văn bản (#)" }
                            option { value: "voice", "Kênh thoại (Voice WebRTC)" }
                            option { value: "announcement", "Kênh thông báo" }
                            option { value: "forum", "Kênh diễn đàn (chủ đề và trả lời)" }
                            option { value: "event", "Kênh sự kiện" }
                        }
                    }
                    div { class: "form-field",
                        label { class: "form-label", "Tên kênh" }
                        input {
                            r#type: "text",
                            placeholder: "Ví dụ: lap-trinh hoặc phong-hop",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    }
                    div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy" }
                        button { r#type: "submit", class: "btn-primary", disabled: loading() || name().trim().is_empty(), "Tạo kênh" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ServerInviteModal(server_id: String, server_name: String, on_close: EventHandler<()>) -> Element {
    let mut code = use_signal(|| Option::<String>::None);
    let mut copied = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    let sid = server_id.clone();
    use_effect(move || {
        let s = sid.clone();
        spawn(async move {
            match api::servers_create_invite(&s).await {
                Ok(c) => code.set(Some(c)),
                Err(e) => error.set(Some(format!("Không thể tạo mã mời: {}", e.code))),
            }
        });
    });

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            aria_modal: "true",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h3 { class: "modal-title", "Mời vào {server_name}" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                div { class: "modal-body",
                    if let Some(c) = code() {
                        p { style: "font-size: 13px; color: var(--text-secondary);",
                            "Chia sẻ mã mời này để bạn bè tham gia cộng đồng:"
                        }
                        div { style: "display: flex; gap: 8px;",
                            input { r#type: "text", readonly: true, value: "{c}", style: "flex: 1; font-family: var(--font-mono);" }
                            button {
                                r#type: "button",
                                class: "btn-primary",
                                onclick: move |_| {
                                    platform::copy_to_clipboard(&c);
                                    copied.set(true);
                                },
                                if copied() { "Đã sao chép" } else { "Sao chép" }
                            }
                        }
                    } else if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    } else {
                        div { class: "empty-chat-inner", "Đang tạo mã mời" }
                    }
                }
            }
        }
    }
}

