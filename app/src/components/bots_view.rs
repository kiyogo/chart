//! Bots, Telegram style: chat with a bot, and create / manage your own bots
//! (token shown once, rotate, public listing after verification, API docs).
//! Bot chats are relayed by the server and are not end-to-end encrypted.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BackIcon, BotIcon, CopyIcon, GroupIcon, PlusIcon, RefreshIcon, TrashIcon},
        messages::{Composer, MessageList, ViewMessage},
    },
    models::{BotChatMessage, BotItem, GroupItem},
    platform,
    services::social,
};

fn bot_error(code: &str) -> &'static str {
    match code {
        "invalid_bot_username" => "Tên bot chỉ gồm chữ cái không dấu, số hoặc gạch dưới (2 đến 32 ký tự)",
        "username_taken_by_user" => "Tên này đang là tên tài khoản của người khác. Bạn chỉ có thể dùng tên tài khoản của chính mình",
        "bot_username_taken" => "Đã có bot dùng tên này",
        "invalid_display_name" => "Tên hiển thị không hợp lệ",
        "bot_limit_reached" => "Bạn đã đạt số bot tối đa",
        "description_too_long" => "Mô tả quá dài",
        "bot_not_public" => "Bot này chưa công khai hoặc chưa được xác minh",
        "insufficient_permissions" => "Chỉ trưởng hoặc phó nhóm được thêm bot",
        _ => "Không thực hiện được, vui lòng thử lại",
    }
}

#[component]
pub fn BotChatView(
    bot: BotItem,
    my_id: String,
    my_name: String,
    /// Latest `bot.message` websocket payload, forwarded by the chat page.
    incoming: Option<serde_json::Value>,
    groups: Vec<GroupItem>,
    on_back: EventHandler<()>,
) -> Element {
    let mut msgs = use_signal(Vec::<BotChatMessage>::new);
    let mut input = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut add_open = use_signal(|| false);
    let bot_id = bot.id.clone();
    use_effect(use_reactive!(|bot_id| {
        spawn(async move {
            match social::bot_messages(&bot_id).await {
                Ok(list) => msgs.set(list),
                Err(_) => msgs.set(Vec::new()),
            }
            platform::scroll_chat_to_bottom(true);
        });
    }));
    let bid = bot.id.clone();
    use_effect(use_reactive!(|incoming| {
        if let Some(v) = incoming.as_ref() {
            if v.get("bot_id").and_then(|x| x.as_str()) == Some(bid.as_str()) {
                if let Ok(m) = serde_json::from_value::<BotChatMessage>(v.clone()) {
                    msgs.with_mut(|l| if !l.iter().any(|x| x.id == m.id) { l.push(m) });
                    platform::scroll_chat_to_bottom(false);
                }
            }
        }
    }));
    let send_id = bot.id.clone();
    let send = move |_| {
        let text = input().trim().to_string();
        if text.is_empty() || sending() {
            return;
        }
        sending.set(true);
        input.set(String::new());
        let id = send_id.clone();
        spawn(async move {
            match social::bot_send(&id, &text).await {
                Ok(m) => msgs.with_mut(|l| if !l.iter().any(|x| x.id == m.id) { l.push(m) }),
                Err(e) => error.set(Some(bot_error(&e.code).to_string())),
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
    let admin_groups: Vec<GroupItem> = groups.into_iter().filter(|g| matches!(g.role.as_deref(), Some("owner") | Some("admin"))).collect();
    let can_add = bot.owner_id == my_id || (bot.is_public && bot.verification_status == "verified");

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column wp-none",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại danh sách", onclick: move |_| on_back.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        Avatar { name: bot.display_name.clone(), avatar_url: bot.avatar_url.clone(), size: 38 }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{bot.display_name}" span { class: "bot-tag", "BOT" } }
                            span { class: "chat-header-meta", "@{bot.username} · không mã hóa đầu cuối" }
                        }
                    }
                    div { class: "chat-header-actions",
                        if can_add && !admin_groups.is_empty() {
                            button { r#type: "button", class: if add_open() { "icon-btn active" } else { "icon-btn" }, title: "Thêm bot vào nhóm", aria_label: "Thêm bot vào nhóm",
                                onclick: move |e| { e.stop_propagation(); add_open.set(!add_open()); },
                                GroupIcon {}
                            }
                        }
                    }
                    if add_open() {
                        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Đóng", onclick: move |_| add_open.set(false) }
                        div { class: "account-menu chat-more-menu", role: "menu",
                            div { class: "menu-note", "Thêm vào nhóm bạn quản lý" }
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
                                                        Ok(_) => error.set(Some(format!("Đã thêm bot vào nhóm {gn}. Bot nhận các lệnh /lệnh và tin nhắc tên @bot trong nhóm."))),
                                                        Err(e) => error.set(Some(bot_error(&e.code).to_string())),
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
                    div { class: "chat-banner", role: "status", span { "{e}" } button { r#type: "button", class: "btn-ghost", onclick: move |_| error.set(None), "Đóng" } }
                }
                MessageList {
                    messages: view,
                    show_sender_names: true,
                    intro_title: bot.display_name.clone(),
                    intro_sub: bot.description.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| "Gửi /start để bắt đầu. Tin nhắn với bot được máy chủ chuyển tiếp tới chủ bot và không mã hóa đầu cuối.".to_string()),
                    on_open_profile: move |_| {},
                    on_profile_click: move |_| {},
                    on_react: move |_| {},
                }
                Composer {
                    input_id: "bot-chat-input".to_string(),
                    value: input(),
                    placeholder: format!("Nhắn tin tới @{}", bot.username),
                    sending: sending(),
                    on_input: move |v: String| input.set(v),
                    on_send: send,
                }
            }
        }
    }
}

/// Settings › Bot: create and manage your own bots.
#[component]
pub fn BotsSettings(my_username: String) -> Element {
    let mut bots = use_signal(Vec::<BotItem>::new);
    let mut username = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut fresh_token = use_signal(|| Option::<(String, String)>::None);
    let mut busy = use_signal(|| false);
    let mut confirm_delete = use_signal(|| Option::<BotItem>::None);
    let mut origin = use_signal(String::new);
    use_effect(move || {
        spawn(async move {
            origin.set(platform::app_origin().await);
            if let Ok(list) = social::bots_mine().await {
                bots.set(list);
            }
        });
    });
    let create = move |_| {
        let u = username().trim().to_lowercase();
        let d = display().trim().to_string();
        if u.is_empty() || d.is_empty() || busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        let ds = desc().trim().to_string();
        spawn(async move {
            match social::bot_create(&u, &d, &ds).await {
                Ok(b) => {
                    if let Some(tok) = b.token.clone() {
                        fresh_token.set(Some((b.username.clone(), tok)));
                    }
                    bots.with_mut(|l| l.insert(0, b));
                    username.set(String::new());
                    display.set(String::new());
                    desc.set(String::new());
                }
                Err(e) => error.set(Some(bot_error(&e.code).to_string())),
            }
            busy.set(false);
        });
    };
    let base = if origin().is_empty() { "https://chatw.example".to_string() } else { origin() };
    rsx! {
        div { class: "settings-section",
            h2 { class: "settings-section-title", "Bot" }
            p { class: "settings-section-desc",
                "Tạo bot giống Telegram: nhận token, tự viết code bằng bất kỳ ngôn ngữ nào và gọi API HTTP của ChatW. Tên bot không được trùng tên tài khoản của người khác; bạn được dùng chính tên tài khoản của mình (@{my_username})."
            }
            div { class: "settings-group",
                div { class: "settings-group-title", "Tạo bot mới" }
                div { class: "form-grid",
                    label { class: "form-field",
                        span { "Tên bot (username)" }
                        input { r#type: "text", maxlength: "32", placeholder: "vd: shop_helper", value: "{username}", oninput: move |e| username.set(e.value()) }
                    }
                    label { class: "form-field",
                        span { "Tên hiển thị" }
                        input { r#type: "text", maxlength: "64", placeholder: "vd: Trợ lý cửa hàng", value: "{display}", oninput: move |e| display.set(e.value()) }
                    }
                    label { class: "form-field wide",
                        span { "Mô tả" }
                        textarea { rows: "2", maxlength: "512", placeholder: "Bot này làm gì", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
                button { r#type: "button", class: "btn-primary", disabled: busy() || username().trim().is_empty() || display().trim().is_empty(), onclick: create,
                    PlusIcon { size: 16 } "Tạo bot"
                }
            }
            if let Some((name, tok)) = fresh_token() {
                div { class: "token-reveal", role: "alert",
                    div { class: "settings-group-title", "Token của @{name}" }
                    p { class: "settings-group-desc", "Token chỉ hiển thị một lần. Lưu nó ở nơi an toàn, không chia sẻ công khai. Mất token thì tạo token mới." }
                    code { class: "token-code", "{tok}" }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-secondary", onclick: { let t = tok.clone(); move |_| platform::copy_to_clipboard(&t) }, CopyIcon {} "Sao chép token" }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| fresh_token.set(None), "Tôi đã lưu" }
                    }
                }
            }
            div { class: "settings-group",
                div { class: "settings-group-title", "Bot của bạn" }
                if bots.read().is_empty() {
                    div { class: "sidebar-empty", "Chưa có bot nào" }
                }
                for b in bots() {
                    {
                        let b_rot = b.clone();
                        let b_pub = b.clone();
                        let b_del = b.clone();
                        let status = match (b.is_public, b.verification_status.as_str()) {
                            (false, _) => "Riêng tư (chỉ bạn trò chuyện được)",
                            (true, "verified") => "Công khai · đã xác minh",
                            (true, "rejected") => "Công khai · bị từ chối xác minh",
                            _ => "Công khai · đang chờ xác minh",
                        };
                        rsx! {
                            div { key: "{b.id}", class: "bot-card",
                                Avatar { name: b.display_name.clone(), avatar_url: b.avatar_url.clone(), size: 44 }
                                div { class: "bot-card-info",
                                    div { class: "bot-card-name", "{b.display_name}" span { class: "bot-tag", "BOT" } }
                                    div { class: "bot-card-sub", "@{b.username} · {status}" }
                                    if let Some(h) = b.token_hint.as_ref() { div { class: "bot-card-sub mono", "Token: {h}" } }
                                }
                                div { class: "bot-card-actions",
                                    button { r#type: "button", class: "btn-ghost", title: "Tạo token mới (token cũ ngừng hoạt động)",
                                        onclick: move |_| {
                                            let id = b_rot.id.clone();
                                            spawn(async move {
                                                if let Ok(nb) = social::bot_rotate(&id).await {
                                                    if let Some(t) = nb.token.clone() { fresh_token.set(Some((nb.username.clone(), t))); }
                                                    bots.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nb.id) { *x = BotItem { token: None, ..nb.clone() }; });
                                                }
                                            });
                                        },
                                        RefreshIcon { size: 16 } "Token mới"
                                    }
                                    button { r#type: "button", class: "btn-ghost",
                                        onclick: move |_| {
                                            let id = b_pub.id.clone();
                                            let next = !b_pub.is_public;
                                            spawn(async move {
                                                match social::bot_update(&id, json!({ "is_public": next })).await {
                                                    Ok(nb) => bots.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nb.id) { *x = nb.clone(); }),
                                                    Err(e) => error.set(Some(bot_error(&e.code).to_string())),
                                                }
                                            });
                                        },
                                        if b.is_public { "Chuyển về riêng tư" } else { "Công khai (gửi xác minh)" }
                                    }
                                    button { r#type: "button", class: "icon-btn danger", title: "Xóa bot", aria_label: "Xóa bot {b.username}", onclick: move |_| confirm_delete.set(Some(b_del.clone())), TrashIcon { size: 16 } }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "settings-group",
                div { class: "settings-group-title", BotIcon { size: 16 } " Lập trình bot" }
                p { class: "settings-group-desc", "API giống Telegram Bot API. Thay TOKEN bằng token của bot. Bot nhận tin nhắn riêng, lệnh /lệnh và tin nhắc @tên_bot trong nhóm đã thêm bot." }
                pre { class: "code-block",
                    "# Thông tin bot\ncurl {base}/api/bot/TOKEN/getMe\n\n# Nhận cập nhật (long polling tối đa 25 giây)\ncurl \"{base}/api/bot/TOKEN/getUpdates?offset=0&timeout=25\"\n\n# Trả lời (chat.id lấy từ update)\ncurl -X POST {base}/api/bot/TOKEN/sendMessage \\\n  -H 'Content-Type: application/json' \\\n  -d '{{\"chat_id\": \"CHAT_ID\", \"text\": \"Xin chào!\"}}'"
                }
                pre { class: "code-block",
                    "# Python: bot trả lời lại mọi tin nhắn\nimport requests, time\nAPI = \"{base}/api/bot/TOKEN\"\noffset = 0\nwhile True:\n    r = requests.get(f\"{{API}}/getUpdates\", params={{\"offset\": offset, \"timeout\": 25}}).json()\n    for u in r.get(\"result\", []):\n        offset = u[\"update_id\"] + 1\n        msg = u.get(\"message\") or {{}}\n        if msg.get(\"text\"):\n            requests.post(f\"{{API}}/sendMessage\", json={{\"chat_id\": msg[\"chat\"][\"id\"], \"text\": \"Bạn vừa gửi: \" + msg[\"text\"]}})"
                }
            }
        }
        if let Some(b) = confirm_delete() {
            div { class: "modal-overlay", onclick: move |_| confirm_delete.set(None),
                div { class: "modal-content compact", role: "alertdialog", aria_modal: "true", onclick: move |e| e.stop_propagation(),
                    h3 { class: "modal-title", "Xóa bot @{b.username}?" }
                    p { class: "modal-sub", "Bot, token và toàn bộ tin nhắn với bot sẽ bị xóa vĩnh viễn. Không thể hoàn tác." }
                    div { class: "modal-actions",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_delete.set(None), "Hủy" }
                        button { r#type: "button", class: "btn-danger",
                            onclick: move |_| {
                                let id = b.id.clone();
                                confirm_delete.set(None);
                                spawn(async move {
                                    if social::bot_delete(&id).await.is_ok() {
                                        bots.with_mut(|l| l.retain(|x| x.id != id));
                                    }
                                });
                            },
                            TrashIcon { size: 16 } "Xóa vĩnh viễn"
                        }
                    }
                }
            }
        }
    }
}
