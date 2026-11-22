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
    let mut is_enterprise = use_signal(|| false);
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
                    h3 { class: "modal-title", "Communities" }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        aria_label: "Close",
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
                            "Create new"
                        }
                        button {
                            r#type: "button",
                            class: if tab() == "join" { "btn-primary" } else { "btn-secondary" },
                            onclick: move |_| { tab.set("join"); error.set(None); },
                            "Enter invitation code"
                        }
                        button {
                            r#type: "button",
                            class: if tab() == "discover" { "btn-primary" } else { "btn-secondary" },
                            onclick: move |_| { tab.set("discover"); error.set(None); },
                            "Explore"
                        }
                    }

                    if tab() == "create" {
                        form {
                            style: "display: flex; flex-direction: column; gap: 12px;",
                            onsubmit: move |e| {
                                e.prevent_default();
                                if loading() {
                                    return;
                                }
                                let n = name().trim().to_string();
                                if n.len() < 2 {
                                    error.set(Some("Community name must be 2 characters or more".into()));
                                    return;
                                }
                                let ent = is_enterprise();
                                loading.set(true);
                                error.set(None);
                                spawn(async move {
                                    match api::servers_create(&n, ent).await {
                                        Ok(srv) => on_server_ready.call(srv),
                                        Err(err) => error.set(Some(format!("Unable to create community: {}", err.code))),
                                    }
                                    loading.set(false);
                                });
                            },
                            div { class: "form-field",
                                label { class: "form-label", "Community name" }
                                input {
                                    r#type: "text",
                                    placeholder: "For example: Vietnam Rust Community",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value()),
                                }
                            }
                            label { class: "settings-row toggle-row", style: "margin: 0;",
                                span { class: "grow",
                                    span { class: "settings-label", "Enterprise mode" }
                                    span { class: "ent-switch-desc", "Additional management and working tools for businesses." }
                                }
                                input {
                                    r#type: "checkbox",
                                    class: "switch",
                                    checked: is_enterprise(),
                                    onchange: move |e| is_enterprise.set(e.checked()),
                                }
                            }
                            if let Some(err) = error() {
                                div { class: "status-badge error", "{err}" }
                            }
                            div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                                button {
                                    r#type: "submit",
                                    class: "btn-primary",
                                    disabled: loading() || name().trim().len() < 2,
                                    if loading() { "Creating" } else { "Create a community" }
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
                                        Err(_) => error.set(Some("Invitation code is invalid or has expired".into())),
                                    }
                                    loading.set(false);
                                });
                            },
                            div { class: "form-field",
                                label { class: "form-label", "Invitation code" }
                                input {
                                    r#type: "text",
                                    placeholder: "Enter the invitation code, for example a1b2c3d4",
                                    value: "{invite_code}",
                                    oninput: move |e| invite_code.set(e.value()),
                                }
                            }
                            if let Some(err) = error() {
                                div { class: "status-badge error", "{err}" }
                            }
                            div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                                button {
                                    r#type: "submit",
                                    class: "btn-primary",
                                    disabled: loading() || invite_code().trim().is_empty(),
                                    if loading() { "Participating" } else { "Join" }
                                }
                            }
                        }
                    } else {
                        div { style: "display: flex; flex-direction: column; gap: 8px; max-height: 280px; overflow-y: auto;",
                            if discover_list.read().is_empty() {
                                div { class: "empty-chat-inner", "There is no public community yet" }
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
                                                    "Join"
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
                    h3 { class: "modal-title", "Create channel categories" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
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
                                Err(err) => error.set(Some(format!("Unable to create category: {}", err.code))),
                            }
                            loading.set(false);
                        });
                    },
                    div { class: "form-field",
                        label { class: "form-label", "Category name" }
                        input {
                            r#type: "text",
                            placeholder: "For example: INFORMATION, CHAT, VOICE",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    }
                    div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                        button { r#type: "submit", class: "btn-primary", disabled: loading() || name().trim().is_empty(), "Create categories" }
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
                    h3 { class: "modal-title", "Create a new channel" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
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
                                Err(err) => error.set(Some(format!("Unable to create channel: {}", err.code))),
                            }
                            loading.set(false);
                        });
                    },
                    div { class: "form-field",
                        label { class: "form-label", "Channel type" }
                        select {
                            value: "{channel_type}",
                            onchange: move |e| channel_type.set(e.value()),
                            option { value: "text", "Text channel" }
                            option { value: "voice", "Voice channel" }
                            option { value: "announcement", "Notification channel" }
                            option { value: "forum", "Forum channel" }
                            option { value: "event", "Event channel" }
                        }
                    }
                    div { class: "form-field",
                        label { class: "form-label", "Channel name" }
                        input {
                            r#type: "text",
                            placeholder: "For example: lap-trinh or phong-hop",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    }
                    div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Cancel" }
                        button { r#type: "submit", class: "btn-primary", disabled: loading() || name().trim().is_empty(), "Create a channel" }
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
                Err(e) => error.set(Some(format!("Unable to generate invitation code: {}", e.code))),
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
                    h3 { class: "modal-title", "Please come in {server_name}" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                div { class: "modal-body",
                    if let Some(c) = code() {
                        p { style: "font-size: 13px; color: var(--text-secondary);",
                            "Share this invitation code for your friends to join the community:"
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
                                if copied() { "Copied" } else { "Copy" }
                            }
                        }
                    } else if let Some(err) = error() {
                        div { class: "status-badge error", "{err}" }
                    } else {
                        div { class: "empty-chat-inner", "Generating invitation code" }
                    }
                }
            }
        }
    }
}

