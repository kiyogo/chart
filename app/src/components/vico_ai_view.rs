//! Spec 016: Standalone "Chat với Vico AI" interface (independent from Enterprise AI).
//! Persists real conversations in PostgreSQL via `/api/vico/*`.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BackIcon, CloseIcon, PlusIcon, SendIcon, SlidersIcon, TrashIcon, VicoAiIcon},
    },
    services::vico::{self, VicoConversation, VicoMessage},
};

#[component]
pub fn VicoAiView(
    my_username: String,
    my_avatar_url: Option<String>,
    on_back: EventHandler<()>,
) -> Element {
    let mut conversations = use_signal(Vec::<VicoConversation>::new);
    let mut active_conv_id = use_signal(|| Option::<String>::None);
    let mut messages = use_signal(Vec::<VicoMessage>::new);
    let mut input = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| Option::<String>::None);

    let mut config_open = use_signal(|| false);
    let mut cfg_endpoint = use_signal(String::new);
    let mut cfg_model = use_signal(|| "gpt-4o-mini".to_string());
    let mut cfg_key = use_signal(String::new);
    let mut cfg_has_key = use_signal(|| false);
    let mut cfg_msg = use_signal(|| Option::<(bool, String)>::None);

    let load_msgs = use_callback(move |cid: String| {
        spawn(async move {
            match vico::list_messages(&cid).await {
                Ok(list) => {
                    messages.set(list);
                    err.set(None);
                }
                Err(e) => err.set(Some(format!("Unable to download chat history: {}", e.code))),
            }
        });
    });

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            if let Ok(list) = vico::list_conversations().await {
                let first_id = list.first().map(|c| c.id.clone());
                conversations.set(list);
                if let Some(cid) = first_id {
                    active_conv_id.set(Some(cid.clone()));
                    if let Ok(ms) = vico::list_messages(&cid).await {
                        messages.set(ms);
                    }
                }
            }
            if let Ok(c) = vico::get_config().await {
                cfg_endpoint.set(c.api_endpoint);
                cfg_model.set(if c.model_name.is_empty() { "gpt-4o-mini".into() } else { c.model_name });
                cfg_has_key.set(c.has_api_key);
            }
            loading.set(false);
        });
    });

    let send_text = use_callback(move |raw: String| {
        let text = raw.trim().to_string();
        if text.is_empty() || sending() {
            return;
        }
        let Some(cid) = active_conv_id() else {
            return;
        };
        input.set(String::new());
        sending.set(true);
        err.set(None);
        spawn(async move {
            match vico::send_message(&cid, &text).await {
                Ok(resp) => {
                    messages.with_mut(|m| {
                        m.push(resp.user_message);
                        m.push(resp.assistant_message);
                    });
                    if let Ok(convs) = vico::list_conversations().await {
                        conversations.set(convs);
                    }
                }
                Err(e) => {
                    err.set(Some(format!("Unable to send message: {}", e.code)));
                    input.set(text);
                }
            }
            sending.set(false);
        });
    });

    let send_btn = send_text.clone();
    let send_key = send_text.clone();

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column vico-chat-column",
                header { class: "chat-header vico-chat-header",
                    button {
                        r#type: "button",
                        class: "icon-btn back-btn",
                        aria_label: "Back",
                        onclick: move |_| on_back.call(()),
                        BackIcon {}
                    }
                    div { class: "chat-header-identity",
                        span { class: "vico-round-avatar", style: "width: 34px; height: 34px;",
                            VicoAiIcon { size: 34, full: true }
                        }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "Vico AI" }
                            span { class: "chat-header-meta",
                                if cfg_has_key() && !cfg_endpoint().trim().is_empty() {
                                    "Connecting {cfg_model()}"
                                } else {
                                    "Live AI chat"
                                }
                            }
                        }
                    }
                    div { class: "chat-header-actions",
                        if !conversations().is_empty() {
                            select {
                                class: "feed-vis-select",
                                aria_label: "Select Vico AI conversation",
                                value: "{active_conv_id().unwrap_or_default()}",
                                onchange: move |e| {
                                    let cid = e.value();
                                    active_conv_id.set(Some(cid.clone()));
                                    load_msgs.call(cid);
                                },
                                for c in conversations() {
                                    option { key: "{c.id}", value: "{c.id}", "{c.title}" }
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            title: "New chat",
                            onclick: move |_| {
                                spawn(async move {
                                    if let Ok(nc) = vico::create_conversation(None).await {
                                        let nid = nc.id.clone();
                                        conversations.with_mut(|l| l.insert(0, nc));
                                        active_conv_id.set(Some(nid));
                                        messages.set(Vec::new());
                                    }
                                });
                            },
                            PlusIcon { size: 15 } " Feed"
                        }
                        if let Some(cid) = active_conv_id() {
                            button {
                                r#type: "button",
                                class: "icon-btn danger",
                                title: "Delete the current conversation",
                                aria_label: "Delete chat",
                                onclick: move |_| {
                                    let id = cid.clone();
                                    spawn(async move {
                                        if vico::delete_conversation(&id).await.is_ok() {
                                            if let Ok(list) = vico::list_conversations().await {
                                                let next = list.first().map(|c| c.id.clone());
                                                conversations.set(list);
                                                active_conv_id.set(next.clone());
                                                if let Some(nid) = next {
                                                    load_msgs.call(nid);
                                                } else {
                                                    messages.set(Vec::new());
                                                }
                                            }
                                        }
                                    });
                                },
                                TrashIcon { size: 16 }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            title: "Configure the AI model API",
                            aria_label: "API configuration",
                            onclick: move |_| { cfg_msg.set(None); config_open.set(true); },
                            SlidersIcon { size: 17 }
                        }
                    }
                }

                div { class: "vico-chat-scroll",
                    div { class: "vico-chat-stream",
                        if let Some(m) = err() {
                            div { class: "status-badge error", "{m}" }
                        }

                        if !loading() && messages().is_empty() {
                            div { class: "vico-empty-welcome",
                                span { class: "vico-round-avatar", style: "width: 52px; height: 52px;",
                                    VicoAiIcon { size: 52, full: true }
                                }
                                p { "Ask anything, calculate, summarize text or configure your LLM API." }
                                div { class: "vico-quick-chips",
                                    for suggestion in [
                                        "Hello Vico AI",
                                        "256 * 18 + 420",
                                        "What time is it?",
                                        "How does E2EE encryption work?",
                                    ] {
                                        {
                                            let s_txt = suggestion.to_string();
                                            let st = send_text.clone();
                                            rsx! {
                                                button {
                                                    key: "{suggestion}",
                                                    r#type: "button",
                                                    class: "filter-chip",
                                                    onclick: move |_| st.call(s_txt.clone()),
                                                    "{suggestion}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        for msg in messages() {
                            if msg.role == "user" {
                                div { key: "{msg.id}", class: "vico-msg-row user",
                                    div { class: "vico-msg-bubble user", "{msg.content}" }
                                    Avatar { name: my_username.clone(), avatar_url: my_avatar_url.clone(), size: 30 }
                                }
                            } else {
                                div { key: "{msg.id}", class: "vico-msg-row assistant",
                                    div { class: "vico-msg-avatar", VicoAiIcon { size: 30, full: true } }
                                    div { class: "vico-msg-bubble assistant", "{msg.content}" }
                                }
                            }
                        }

                        if sending() {
                            div { class: "vico-msg-row assistant",
                                div { class: "vico-msg-avatar", VicoAiIcon { size: 30, full: true } }
                                div { class: "vico-msg-bubble assistant muted", "Thinking" }
                            }
                        }
                    }
                }

                div { class: "composer",
                    div { class: "composer-inner",
                        div { class: "composer-box",
                            textarea {
                                id: "vico-ai-input",
                                class: "composer-input",
                                rows: "1",
                                placeholder: "Message Vico AI",
                                aria_label: "Message Vico AI",
                                value: "{input}",
                                oninput: move |e| input.set(e.value()),
                                onkeydown: move |e| {
                                    if e.key() == Key::Enter && !e.modifiers().shift() && !e.is_composing() {
                                        e.prevent_default();
                                        send_key.call(input());
                                    }
                                },
                            }
                            button {
                                r#type: "button",
                                class: "send-btn",
                                title: "Send",
                                aria_label: "Send to Vico AI",
                                disabled: sending() || input().trim().is_empty(),
                                onclick: move |_| send_btn.call(input()),
                                SendIcon {}
                            }
                        }
                    }
                }
            }
        }

        if config_open() {
            div { class: "modal-overlay", onclick: move |_| config_open.set(false),
                div { class: "modal-dialog note-modal", role: "dialog", aria_modal: "true", aria_label: "Vico AI configuration", onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header",
                        h2 { class: "modal-title", "API connection for Vico AI" }
                        button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| config_open.set(false), CloseIcon { size: 18 } }
                    }
                    label { class: "form-field",
                        span { class: "form-label", "OpenAI-compatible Endpoint" }
                        input {
                            r#type: "url",
                            placeholder: "https://api.openai.com/v1",
                            value: "{cfg_endpoint}",
                            oninput: move |e| cfg_endpoint.set(e.value()),
                        }
                    }
                    label { class: "form-field",
                        span { class: "form-label", "Model name" }
                        input {
                            r#type: "text",
                            placeholder: "gpt-4o-mini",
                            value: "{cfg_model}",
                            oninput: move |e| cfg_model.set(e.value()),
                        }
                    }
                    label { class: "form-field",
                        span { class: "form-label", if cfg_has_key() { "API Key (leave blank if kept)" } else { "API Key" } }
                        input {
                            r#type: "password",
                            placeholder: "sk-",
                            value: "{cfg_key}",
                            oninput: move |e| cfg_key.set(e.value()),
                        }
                    }
                    if let Some((is_err, txt)) = cfg_msg() {
                        div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{txt}" }
                    }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| config_open.set(false), "Close" }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: move |_| {
                                let ep = cfg_endpoint().trim().to_string();
                                let md = cfg_model().trim().to_string();
                                let ak = cfg_key().trim().to_string();
                                let key_opt = if ak.is_empty() { None } else { Some(ak.clone()) };
                                spawn(async move {
                                    match vico::save_config(&ep, key_opt.as_deref(), &md).await {
                                        Ok(_) => {
                                            if key_opt.is_some() { cfg_has_key.set(true); }
                                            cfg_key.set(String::new());
                                            cfg_msg.set(Some((false, "Vico AI configuration saved".into())));
                                        }
                                        Err(e) => cfg_msg.set(Some((true, format!("Save failed: {}", e.code)))),
                                    }
                                });
                            },
                            "Save configuration"
                        }
                    }
                }
            }
        }
    }
}
