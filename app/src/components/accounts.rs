//! Spec 011 interface: several accounts on one device, two-step verification
//! (TOTP + backup codes) and the security log.

use dioxus::prelude::*;

use crate::{
    components::avatar::Avatar,
    platform,
    services::security::{self, Account},
};

const ORDER_KEY: &str = "chatw_account_order";
const HIDDEN_KEY: &str = "chatw_account_hidden";

fn load_list(key: &str) -> Vec<String> {
    platform::storage_get(key).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

fn save_list(key: &str, v: &[String]) {
    platform::storage_set(key, &serde_json::to_string(v).unwrap_or_default());
}

/// Accounts in the order chosen on this device.
fn ordered(mut list: Vec<Account>) -> Vec<Account> {
    let order = load_list(ORDER_KEY);
    list.sort_by_key(|a| order.iter().position(|x| x == &a.user_id).unwrap_or(usize::MAX));
    list
}

/// QR code (SVG) for the authenticator app.
fn qr_svg(uri: &str) -> String {
    use qrcode::{QrCode, render::svg};
    QrCode::new(uri.as_bytes())
        .map(|c| c.render::<svg::Color>().min_dimensions(184, 184).dark_color(svg::Color("#0F172A")).light_color(svg::Color("#FFFFFF")).build())
        .unwrap_or_default()
}

/// The second step after signing in (TOTP code or a backup code).
#[component]
pub fn TwoFactorPrompt(on_done: EventHandler<()>, on_cancel: EventHandler<()>) -> Element {
    let mut code = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    use_hook(|| platform::focus_selector("#twofa-code"));
    let submit = move |e: Event<FormData>| {
        e.prevent_default();
        if busy() || code().trim().is_empty() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            match security::twofa_verify(code().trim()).await {
                Ok(()) => on_done.call(()),
                Err(e) => {
                    error.set(Some(security::error_text(&e.code)));
                    code.set(String::new());
                }
            }
            busy.set(false);
        });
    };
    rsx! {
        form { class: "twofa-card", onsubmit: submit,
            h1 { class: "auth-title", "Xác minh hai bước" }
            p { class: "auth-subtitle", "Nhập mã 6 số trong ứng dụng xác thực hoặc một mã dự phòng dạng xxxx-xxxx." }
            input { id: "twofa-code", class: "auth-input twofa-input", r#type: "text", inputmode: "text", autocomplete: "one-time-code", maxlength: "12",
                aria_label: "Mã xác minh", placeholder: "123456", value: "{code}", oninput: move |e| code.set(e.value()) }
            if let Some(e) = error() { div { class: "auth-error", role: "alert", "{e}" } }
            div { class: "inline-form",
                button { r#type: "submit", class: "btn-primary", disabled: busy() || code().trim().len() < 6, if busy() { "Đang kiểm tra" } else { "Xác minh" } }
                button { r#type: "button", class: "btn-secondary", onclick: move |_| on_cancel.call(()), "Quay lại" }
            }
        }
    }
}

/// Accounts already signed in on this device, shown on the sign-in screen.
#[component]
pub fn SignedInAccounts() -> Element {
    let mut list = use_signal(Vec::<Account>::new);
    let mut error = use_signal(|| Option::<String>::None);
    use_hook(move || {
        spawn(async move {
            if let Ok(l) = security::accounts().await {
                list.set(ordered(l));
            }
        });
    });
    if list.read().is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "signed-in-accounts",
            div { class: "auth-dev-title", "Tài khoản đã đăng nhập trên thiết bị này" }
            for a in list() {
                {
                    let id = a.user_id.clone();
                    rsx! {
                        button { key: "{a.user_id}", r#type: "button", class: "account-row",
                            onclick: move |_| {
                                let id = id.clone();
                                spawn(async move {
                                    match security::switch_account(&id).await {
                                        Ok(()) => security::reload_app(),
                                        Err(e) => error.set(Some(security::error_text(&e.code))),
                                    }
                                });
                            },
                            Avatar { name: a.name(), avatar_url: a.avatar_url.clone(), size: 36 }
                            span { class: "grow", "{a.name()}" }
                            if a.unread > 0 { span { class: "unread-badge", "{a.unread}" } }
                            span { class: "member-sub", "Tiếp tục" }
                        }
                    }
                }
            }
            if let Some(e) = error() { div { class: "auth-error", "{e}" } }
        }
    }
}

/// Account switcher inside the rail's account menu.
#[component]
pub fn AccountSwitcher(on_close: EventHandler<()>) -> Element {
    let mut list = use_signal(Vec::<Account>::new);
    let mut error = use_signal(|| Option::<String>::None);
    use_hook(move || {
        spawn(async move {
            if let Ok(l) = security::accounts().await {
                list.set(ordered(l));
            }
        });
    });
    let hidden = load_list(HIDDEN_KEY);
    let others: Vec<Account> = list().into_iter().filter(|a| !a.active && !hidden.contains(&a.user_id)).collect();
    let count = list.read().len();
    rsx! {
        for a in others {
            {
                let id = a.user_id.clone();
                rsx! {
                    button { key: "{a.user_id}", r#type: "button", class: "menu-item account-switch", role: "menuitem", title: "Chuyển sang {a.name()}",
                        onclick: move |_| {
                            let id = id.clone();
                            spawn(async move {
                                match security::switch_account(&id).await {
                                    Ok(()) => security::reload_app(),
                                    Err(e) => error.set(Some(security::error_text(&e.code))),
                                }
                            });
                        },
                        Avatar { name: a.name(), avatar_url: a.avatar_url.clone(), size: 22 }
                        span { class: "grow", "{a.name()}" }
                        if a.unread > 0 { span { class: "unread-badge", "{a.unread}" } }
                    }
                }
            }
        }
        if count < 5 {
            button { r#type: "button", class: "menu-item", role: "menuitem",
                onclick: move |_| {
                    // Not tied to the menu: it closes before the request finishes.
                    dioxus::core::spawn_forever(async move {
                        match security::add_account().await {
                            Ok(()) => security::reload_app(),
                            Err(e) => error.set(Some(security::error_text(&e.code))),
                        }
                    });
                    on_close.call(());
                },
                crate::components::icons::PlusIcon { size: 18 } span { "Thêm tài khoản" }
            }
        }
        if count > 1 {
            button { r#type: "button", class: "menu-item danger", role: "menuitem",
                onclick: move |_| {
                    dioxus::core::spawn_forever(async move {
                        let _ = security::logout_all().await;
                        security::reload_app();
                    });
                },
                crate::components::icons::LogoutIcon {} span { "Đăng xuất tất cả tài khoản" }
            }
        }
        if let Some(e) = error() { div { class: "status-badge error", "{e}" } }
    }
}

/// Settings › Bảo mật: accounts on this device, two-step verification and the security log.
#[component]
pub fn SecuritySettings() -> Element {
    let mut st = use_signal(|| Option::<security::Status>::None);
    let mut events = use_signal(Vec::<security::Event>::new);
    let mut accounts = use_signal(Vec::<Account>::new);
    let mut setup = use_signal(|| Option::<(String, String)>::None);
    let mut codes = use_signal(Vec::<String>::new);
    let mut code = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut hidden = use_signal(|| load_list(HIDDEN_KEY));
    let mut acc_drag_from = use_signal(|| Option::<usize>::None);
    let mut acc_drag_over = use_signal(|| Option::<usize>::None);
    let reload = move || {
        spawn(async move {
            if let Ok(s) = security::status().await {
                st.set(Some(s));
            }
            if let Ok(e) = security::events().await {
                events.set(e);
            }
            if let Ok(a) = security::accounts().await {
                accounts.set(ordered(a));
            }
        });
    };
    use_hook(move || reload());

    let start_setup = move |_| {
        msg.set(None);
        spawn(async move {
            match security::totp_setup().await {
                Ok(s) => setup.set(Some(s)),
                Err(e) => msg.set(Some((true, security::error_text(&e.code)))),
            }
        });
    };
    let mut act = move |what: &'static str| {
        if busy() {
            return;
        }
        busy.set(true);
        msg.set(None);
        spawn(async move {
            let c = code().trim().to_string();
            let r = match what {
                "enable" => security::totp_enable(&c).await.map(|cs| {
                    codes.set(cs);
                    setup.set(None);
                    "Đã bật xác minh hai bước. Hãy cất 10 mã dự phòng ở nơi an toàn."
                }),
                "disable" => security::totp_disable(&c).await.map(|_| {
                    codes.set(Vec::new());
                    "Đã tắt xác minh hai bước"
                }),
                _ => security::regenerate_codes(&c).await.map(|cs| {
                    codes.set(cs);
                    "Đã tạo 10 mã dự phòng mới; các mã cũ không dùng được nữa"
                }),
            };
            match r {
                Ok(m) => {
                    msg.set(Some((false, m.into())));
                    code.set(String::new());
                    reload();
                }
                Err(e) => msg.set(Some((true, security::error_text(&e.code)))),
            }
            busy.set(false);
        });
    };

    let s = st();
    let enabled = s.as_ref().is_some_and(|s| s.totp_enabled);
    rsx! {
        div { class: "settings-section security-settings",
            div { class: "settings-group-title", "Tài khoản" }
            p { class: "settings-row-desc", "Tối đa 5 tài khoản. Mỗi tài khoản có khóa mã hóa, cài đặt, chủ đề, thông báo và mã PIN riêng; chuyển nhanh trong menu ảnh đại diện ở góc dưới." }
            div { class: "account-list", "data-drag-group": "account-order",
                for (i, a) in accounts().into_iter().enumerate() {
                    {
                        let id = a.user_id.clone();
                        let (id_hide, id_out) = (id.clone(), id.clone());
                        let is_hidden = hidden.read().contains(&id);
                        let dragging_now = acc_drag_from() == Some(i);
                        let over_cls = match (acc_drag_from(), acc_drag_over()) {
                            (Some(from), Some(ov)) if ov == i && from != i => {
                                if i < from { " drag-over-before" } else { " drag-over-after" }
                            }
                            _ => "",
                        };
                        let row_cls = format!(
                            "account-row static draggable-item{}{}",
                            if dragging_now { " is-dragging" } else { "" },
                            over_cls
                        );
                        rsx! {
                            div {
                                key: "{id}",
                                class: "{row_cls}",
                                draggable: "true",
                                "data-drag-item": "true",
                                "data-drag-key": "{id}",
                                ondragstart: move |_| {
                                    acc_drag_from.set(Some(i));
                                    acc_drag_over.set(None);
                                },
                                ondragover: move |e| {
                                    e.prevent_default();
                                    if acc_drag_from().is_some() && acc_drag_over() != Some(i) {
                                        acc_drag_over.set(Some(i));
                                    }
                                },
                                ondrop: move |e| {
                                    e.prevent_default();
                                    if let Some(from) = acc_drag_from() {
                                        let mut ids: Vec<String> = accounts.read().iter().map(|x| x.user_id.clone()).collect();
                                        if from != i && from < ids.len() && i < ids.len() {
                                            let moved = ids.remove(from);
                                            ids.insert(i, moved);
                                            save_list(ORDER_KEY, &ids);
                                            accounts.set(ordered(accounts()));
                                        }
                                    }
                                    acc_drag_from.set(None);
                                    acc_drag_over.set(None);
                                },
                                ondragend: move |_| {
                                    acc_drag_from.set(None);
                                    acc_drag_over.set(None);
                                },
                                Avatar { name: a.name(), avatar_url: a.avatar_url.clone(), size: 32 }
                                span { class: "grow", "{a.name()}" if a.active { span { class: "member-sub", " · đang dùng" } } }
                                if a.unread > 0 { span { class: "unread-badge", "{a.unread}" } }
                                if !a.active {
                                    button { r#type: "button", class: "btn-small-secondary",
                                        onclick: move |_| {
                                            let mut h = hidden();
                                            if h.contains(&id_hide) { h.retain(|x| x != &id_hide) } else { h.push(id_hide.clone()) }
                                            save_list(HIDDEN_KEY, &h);
                                            hidden.set(h);
                                        },
                                        if is_hidden { "Hiện trong menu" } else { "Ẩn khỏi menu" }
                                    }
                                    button { r#type: "button", class: "btn-small-secondary danger",
                                        onclick: move |_| {
                                            let id = id_out.clone();
                                            spawn(async move {
                                                match security::logout_account(&id).await {
                                                    Ok(()) => reload(),
                                                    Err(e) => msg.set(Some((true, security::error_text(&e.code)))),
                                                }
                                            });
                                        }, "Đăng xuất" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-group-title", "Xác minh hai bước" }
            if let Some((err, m)) = msg() { div { class: if err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" } }
            if enabled {
                div { class: "settings-row",
                    span { class: "grow",
                        span { class: "settings-label", "Xác minh hai bước" }
                        span { class: "settings-sublabel", "Đang bật · Còn {s.as_ref().map(|x| x.backup_codes_left).unwrap_or(0)} mã dự phòng" }
                    }
                    span { class: "chip-privacy", "Đang bật" }
                }
                div { class: "inline-form",
                    input { r#type: "text", maxlength: "12", aria_label: "Mã hiện tại hoặc mã dự phòng", placeholder: "Mã 6 số hoặc mã dự phòng", value: "{code}", oninput: move |e| code.set(e.value()) }
                    button { r#type: "button", class: "btn-secondary", disabled: busy() || code().trim().is_empty(), onclick: move |_| act("regen"), "Tạo lại mã dự phòng" }
                    button { r#type: "button", class: "btn-secondary danger", disabled: busy() || code().trim().is_empty(), onclick: move |_| act("disable"), "Tắt xác minh hai bước" }
                }
            } else if let Some((secret, uri)) = setup() {
                div { class: "totp-setup",
                    div { class: "totp-qr", aria_label: "Mã QR để quét bằng ứng dụng xác thực", role: "img", dangerous_inner_html: "{qr_svg(&uri)}" }
                    div { class: "totp-steps",
                        ol { class: "guide-steps",
                            li { "Mở ứng dụng xác thực và quét mã QR." }
                            li { "Không quét được? Nhập khóa: " code { class: "totp-secret", "{secret}" } }
                            li { "Nhập mã 6 số ứng dụng hiển thị để bật." }
                        }
                        div { class: "inline-form",
                            input { r#type: "text", inputmode: "numeric", maxlength: "6", aria_label: "Mã 6 số để bật xác minh hai bước", placeholder: "123456", value: "{code}", oninput: move |e| code.set(e.value()) }
                            button { r#type: "button", class: "btn-primary", disabled: busy() || code().trim().len() != 6, onclick: move |_| act("enable"), "Bật" }
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| setup.set(None), "Hủy" }
                        }
                    }
                }
            } else {
                div { class: "settings-row",
                    span { class: "grow",
                        span { class: "settings-label", "Xác minh hai bước" }
                        span { class: "settings-sublabel", "Chưa bật" }
                    }
                    button { r#type: "button", class: "btn-primary", onclick: start_setup, "Thiết lập" }
                }
            }
            if !codes.read().is_empty() {
                div { class: "backup-codes",
                    div { class: "settings-label", "Mã dự phòng" }
                    div { class: "backup-grid", for c in codes() { code { key: "{c}", "{c}" } } }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::copy_to_clipboard(&codes().join("\n")), "Sao chép" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| codes.set(Vec::new()), "Tôi đã cất mã" }
                    }
                }
            }

            div { class: "settings-group-title", "Nhật ký bảo mật" }
            p { class: "settings-row-desc", "100 hoạt động gần nhất. Địa chỉ IP chỉ giữ phần mạng." }
            if events.read().is_empty() {
                div { class: "sidebar-empty", "Trống" }
            } else {
                div { class: "security-log",
                    for (i, ev) in events().into_iter().enumerate() {
                        div { key: "{i}", class: if ev.kind == "new_device" || ev.kind == "failed_2fa" { "security-event warn" } else { "security-event" },
                            span { class: "settings-label", "{security::event_label(&ev.kind)}" }
                            span { class: "member-sub", "{ev.device.clone().unwrap_or_default()} · {ev.ip.clone().unwrap_or_default()} · {platform::format_local_time(&ev.at)}" }
                        }
                    }
                }
            }
        }
    }
}
