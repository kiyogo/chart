//! Unified Sandboxed Plugin Pages, Left-Sidebar Sandbox Host & Real Backend Bot Chat View (Spec 058).
//! All hardcoded built-in plugin pages (Notes/Todo, Soundboard, Voxel, Sakura, Cyber, QuickTools)
//! have been removed; installed plugins render strictly inside the opaque-origin sandboxed iframe host.

use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        chat_extras::wallpaper_attrs,
        icons::{BackIcon, CloseIcon, GroupIcon, SlidersIcon},
        messages::{Composer, MessageList, ViewMessage},
        plugin_hub::{PLUGIN_INSTALLATIONS, PluginHubPanel},
        plugins_view::{ACTIVE_PLUGIN_PAGE, EDITING_PLUGIN_ID, PluginGlyph, glyph_slot_size, plugin_error},
    },
    models::{BotChatMessage, BotItem, GroupItem},
    platform,
    prefs::PrefsCtx,
    services::{api, plugin_api, social},
};

fn active_wallpaper_attrs() -> (String, String) {
    let default_wp = try_use_context::<PrefsCtx>()
        .map(|c| c.prefs.read().wallpaper.clone())
        .unwrap_or_else(|| "none".into());
    wallpaper_attrs(None, &default_wp)
}

/// Renders either the unified Plugin Hub (`studio`) or a sandboxed custom page for an installed Plugin.
#[component]
pub fn PluginCustomPageView(
    plugin_id: String,
    my_username: String,
    on_close: EventHandler<()>,
    on_open_bot_chat: EventHandler<BotItem>,
) -> Element {
    let _ = on_open_bot_chat;
    let (wp_cls, wp_style) = active_wallpaper_attrs();

    if plugin_id == "studio" {
        let uid = api::auth_me_cached_id().unwrap_or_default();
        return rsx! {
            div { class: "chat-main-container",
                div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                    header { class: "chat-header only-mobile",
                        button { r#type: "button", class: "icon-btn back-btn", aria_label: "Back", onclick: move |_| on_close.call(()), BackIcon {} }
                    }
                    div { class: "plugin-custom-stage",
                        div { class: "settings-content", style: "max-width: 66rem; width: 100%; margin: 0 auto;",
                            PluginHubPanel {
                                my_user_id: uid,
                                my_username: my_username,
                            }
                        }
                    }
                }
            }
        };
    }

    let inst = PLUGIN_INSTALLATIONS
        .read()
        .iter()
        .find(|i| i.id == plugin_id && i.enabled)
        .cloned();

    let Some(inst) = inst else {
        return rsx! {
            div { class: "chat-main-container",
                div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                    div { class: "empty-state", "Plugin does not exist or has been disabled." }
                }
            }
        };
    };

    let mount_id = inst.id.clone();
    use_effect(use_reactive!(|mount_id| {
        plugin_api::host_mount_ui("#plugin-page-sandbox-stage", &mount_id, "page");
    }));

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Back", onclick: move |_| on_close.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        span { class: "bot-header-icon",
                            PluginGlyph { icon: String::new(), logo_url: inst.icon_url.clone(), size: 22 }
                        }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{inst.name}" }
                            span { class: "chat-header-meta", "{inst.plugin_key} · v{inst.version}" }
                        }
                    }
                    div { class: "chat-header-actions",
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            onclick: move |_| {
                                *EDITING_PLUGIN_ID.write() = None;
                                *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                            },
                            SlidersIcon {} "Plugin Management"
                        }
                    }
                }
                div {
                    id: "plugin-page-sandbox-stage",
                    class: "plugin-custom-stage",
                    style: "flex: 1; min-height: 420px; display: flex; flex-direction: column;",
                }
            }
        }
    }
}

/// Renders a sandboxed sidebar panel contributed by an installed Plugin (`ui.sidebar`).
#[component]
pub fn PluginLeftAppView(
    plugin_id: String,
    my_username: String,
    on_close: EventHandler<()>,
) -> Element {
    let _ = my_username;
    let inst = PLUGIN_INSTALLATIONS
        .read()
        .iter()
        .find(|i| i.id == plugin_id && i.enabled)
        .cloned();

    let Some(inst) = inst else {
        return rsx! {
            aside { class: "left-plugin-panel",
                div { class: "empty-state", "Plugin is disabled or not installed." }
            }
        };
    };

    let mount_id = inst.id.clone();
    use_effect(use_reactive!(|mount_id| {
        plugin_api::host_mount_ui("#plugin-sidebar-sandbox-stage", &mount_id, "sidebar");
    }));

    rsx! {
        aside { class: "left-plugin-panel", aria_label: "{inst.name}",
            header { class: "left-plugin-header",
                div { class: "left-plugin-title-wrap",
                    PluginGlyph { icon: String::new(), logo_url: inst.icon_url.clone(), size: 18 }
                    span { class: "left-plugin-title", "{inst.name}" }
                }
                button { r#type: "button", class: "icon-btn", aria_label: "Close", onclick: move |_| on_close.call(()), CloseIcon {} }
            }
            div {
                id: "plugin-sidebar-sandbox-stage",
                class: "left-plugin-body",
                style: "flex: 1; min-height: 300px; display: flex; flex-direction: column;",
            }
        }
    }
}

/// Real backend Bot conversation view (`social::bot_messages`, `social::bot_send`, `social::bot_add_to_group`).
#[component]
pub fn PluginChatView(
    bot: BotItem,
    my_id: String,
    my_name: String,
    #[props(default)] incoming: Option<serde_json::Value>,
    groups: Vec<GroupItem>,
    on_back: EventHandler<()>,
) -> Element {
    let mut msgs = use_signal(Vec::<BotChatMessage>::new);
    let mut input = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut add_open = use_signal(|| false);
    let (wp_cls, wp_style) = active_wallpaper_attrs();

    let bid_load = bot.id.clone();
    use_effect(use_reactive!(|bid_load| {
        let id = bid_load.clone();
        msgs.set(Vec::new());
        error.set(None);
        spawn(async move {
            if let Ok(list) = social::bot_messages(&id).await {
                msgs.set(list);
                platform::scroll_chat_to_bottom(true);
            }
        });
    }));

    let bid_inc = bot.id.clone();
    use_effect(use_reactive!(|incoming, bid_inc| {
        if let Some(val) = incoming
            && val.get("bot_id").and_then(|s| s.as_str()) == Some(bid_inc.as_str())
            && let Some(m_val) = val.get("message")
            && let Ok(msg) = serde_json::from_value::<BotChatMessage>(m_val.clone())
        {
            msgs.with_mut(|l| {
                if !l.iter().any(|x| x.id == msg.id) {
                    l.push(msg);
                }
            });
            platform::scroll_chat_to_bottom(false);
        }
    }));

    let bid_send = bot.id.clone();
    let mut send_text = move |raw: String| {
        let text = raw.trim().to_string();
        if text.is_empty() || sending() {
            return;
        }
        sending.set(true);
        error.set(None);
        input.set(String::new());
        let id = bid_send.clone();
        spawn(async move {
            match social::bot_send(&id, &text).await {
                Ok(m) => {
                    msgs.with_mut(|l| {
                        if !l.iter().any(|x| x.id == m.id) {
                            l.push(m);
                        }
                    });
                }
                Err(e) => error.set(Some(plugin_error(&e.code).to_string())),
            }
            sending.set(false);
            platform::scroll_chat_to_bottom(false);
        });
    };

    let view: Vec<ViewMessage> = msgs()
        .into_iter()
        .map(|m| ViewMessage {
            id: m.id.clone(),
            sender_id: if m.from_bot { bot.id.clone() } else { my_id.clone() },
            sender_name: if m.from_bot { bot.display_name.clone() } else { my_name.clone() },
            sender_avatar: if m.from_bot { bot.avatar_url.clone() } else { None },
            text: m.text,
            created_at: m.created_at,
            is_self: !m.from_bot,
            is_error: false,
            is_pending: false,
            reply_text: None,
            reactions: Vec::new(),
            is_pinned: false,
            is_bot: m.from_bot,
            highlighted: false,
        })
        .collect();

    let admin_groups: Vec<GroupItem> = groups
        .into_iter()
        .filter(|g| matches!(g.role.as_deref(), Some("owner") | Some("admin")))
        .collect();
    let can_add = bot.owner_id == my_id || (bot.is_public && bot.verification_status == "verified");
    let header_logo = "svg:bot".to_string();
    let bot_desc = bot
        .description
        .clone()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| "Send a message or type / to interact with the bot.".to_string());

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column {wp_cls}", style: "{wp_style}",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Back to the list", onclick: move |_| on_back.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        div { class: "chat-item-avatar",
                            if bot.avatar_url.is_some() {
                                Avatar { name: bot.display_name.clone(), avatar_url: bot.avatar_url.clone(), size: 38 }
                            } else {
                                span { class: "bot-header-icon", PluginGlyph { icon: String::new(), logo_url: header_logo.clone(), size: glyph_slot_size(&header_logo, 20, 38) } }
                            }
                        }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{bot.display_name}" }
                            span { class: "chat-header-meta", "@{bot.username}" }
                        }
                    }
                    div { class: "chat-header-actions",
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            title: "Plugin Management",
                            onclick: move |_| {
                                *EDITING_PLUGIN_ID.write() = None;
                                *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                            },
                            SlidersIcon {} "Plugin"
                        }
                        if can_add && !admin_groups.is_empty() {
                            button { r#type: "button", class: if add_open() { "icon-btn active" } else { "icon-btn" }, title: "Add bots to the group", aria_label: "Add bots to the group",
                                onclick: move |e| { e.stop_propagation(); add_open.set(!add_open()); },
                                GroupIcon {}
                            }
                        }
                    }
                    if add_open() {
                        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Close", onclick: move |_| add_open.set(false) }
                        div { class: "account-menu chat-more-menu", role: "menu",
                            div { class: "menu-note", "Add bots to groups you manage" }
                            for g in admin_groups.iter().cloned() {
                                {
                                    let bid = bot.id.clone();
                                    let gid = g.id.clone();
                                    let gname = g.name.clone();
                                    rsx! {
                                        button { key: "{g.id}", r#type: "button", role: "menuitem", class: "account-menu-item",
                                            onclick: move |_| {
                                                add_open.set(false);
                                                let (b, gi, gn) = (bid.clone(), gid.clone(), gname.clone());
                                                spawn(async move {
                                                    match social::bot_add_to_group(&b, &gi).await {
                                                        Ok(_) => error.set(Some(format!("Added bot to group {gn}."))),
                                                        Err(e) => error.set(Some(plugin_error(&e.code).to_string())),
                                                    }
                                                });
                                            },
                                            Avatar { name: g.name.clone(), avatar_url: g.avatar_url.clone(), size: 24 }
                                            "{g.name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(e) = error() {
                    div { class: "chat-banner", role: "status", span { "{e}" } button { r#type: "button", class: "btn-ghost", onclick: move |_| error.set(None), "Close" } }
                }
                if view.is_empty() {
                    div { class: "chat-messages", id: "chat-messages-scroll",
                        div { class: "bot-welcome-card",
                            span { class: "bot-welcome-icon", PluginGlyph { icon: String::new(), logo_url: header_logo.clone(), size: 28 } }
                            h3 { class: "bot-welcome-title", "{bot.display_name}" }
                            p { class: "bot-welcome-desc", "{bot_desc}" }
                        }
                    }
                } else {
                    MessageList {
                        messages: view,
                        show_sender_names: false,
                        intro_title: bot.display_name.clone(),
                        intro_sub: bot_desc,
                        on_open_profile: move |_| {},
                        on_profile_click: move |_| {},
                        on_react: move |_| {},
                    }
                }
                Composer {
                    input_id: "bot-chat-input".to_string(),
                    value: input(),
                    placeholder: format!("Text or type /command to @{}", bot.username),
                    sending: sending(),
                    on_input: move |v: String| input.set(v),
                    on_send: move |_| send_text(input()),
                }
            }
        }
    }
}
