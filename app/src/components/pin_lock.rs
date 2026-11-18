//! App lock screen: asks for the PIN after inactivity (or on every start when
//! enabled). Wrong attempts slow down: after 5 misses a growing cool-down.

use dioxus::prelude::*;

use crate::{
    components::{
        icons::LockIcon,
        settings_more::{hash_pin, same},
    },
    prefs::PrefsCtx,
};

#[component]
pub fn PinLock(username: String, on_unlock: EventHandler<()>, on_forgot: EventHandler<()>) -> Element {
    let c = use_context::<PrefsCtx>();
    let mut pin = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut shake = use_signal(|| false);
    let mut attempts = use_signal(|| 0u32);
    let mut wait_until = use_signal(|| 0i64);
    let mut checking = use_signal(|| false);
    let mut confirm_forgot = use_signal(|| false);
    use_effect(move || {
        let _ = dioxus::document::eval("setTimeout(() => document.getElementById('pin-lock')?.focus(), 30);");
    });
    let mut submit = move || {
        let now = chrono::Utc::now().timestamp();
        if checking() || pin().is_empty() {
            return;
        }
        if now < wait_until() {
            error.set(Some(format!("Thử lại sau {} giây", wait_until() - now)));
            return;
        }
        checking.set(true);
        let entered = pin();
        spawn(async move {
            let p = c.prefs.peek().clone();
            let ok = hash_pin(&entered, &p.pin_salt).await.is_some_and(|h| same(&h, &p.pin_hash));
            if ok {
                attempts.set(0);
                pin.set(String::new());
                error.set(None);
                on_unlock.call(());
            } else {
                let n = attempts() + 1;
                attempts.set(n);
                pin.set(String::new());
                shake.set(true);
                if n >= 5 {
                    let secs = 30i64 << (n - 5).min(5);
                    wait_until.set(chrono::Utc::now().timestamp() + secs);
                    error.set(Some(format!("Sai mã PIN {n} lần. Thử lại sau {secs} giây.")));
                } else {
                    error.set(Some("Mã PIN không đúng".into()));
                }
                crate::platform::sleep_ms(450).await;
                shake.set(false);
            }
            checking.set(false);
        });
    };
    let len = pin().len();
    rsx! {
        div {
            id: "pin-lock",
            class: "lock-screen",
            role: "dialog",
            aria_modal: "true",
            aria_label: "ChatW đang khóa",
            tabindex: "0",
            onkeydown: move |e| {
                match e.key() {
                    Key::Character(ch) if ch.len() == 1 && ch.chars().all(|c| c.is_ascii_digit()) => {
                        if pin().len() < 8 { pin.with_mut(|p| p.push_str(&ch)); }
                    }
                    Key::Backspace => { pin.with_mut(|p| { p.pop(); }); }
                    Key::Enter => submit(),
                    _ => {}
                }
            },
            div { class: if shake() { "lock-card shake" } else { "lock-card" },
                span { class: "self-chat-avatar", LockIcon { size: 22 } }
                div {
                    h2 { class: "modal-title", "ChatW đang khóa" }
                    p { class: "modal-sub", "Nhập mã PIN của {username} để tiếp tục" }
                }
                div { class: "lock-dots", aria_label: "Đã nhập {len} số",
                    for i in 0..len.max(4) {
                        span { key: "{i}", class: if i < len { "lock-dot filled" } else { "lock-dot" } }
                    }
                }
                if let Some(e) = error() { div { class: "status-badge error", role: "alert", "{e}" } }
                div { class: "lock-pad",
                    for d in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
                        button { key: "{d}", r#type: "button", class: "lock-key", onclick: move |_| if pin().len() < 8 { pin.with_mut(|p| p.push_str(d)) }, "{d}" }
                    }
                    button { r#type: "button", class: "lock-key ghost", aria_label: "Xóa một số", onclick: move |_| pin.with_mut(|p| { p.pop(); }), "Xóa" }
                    button { r#type: "button", class: "lock-key", onclick: move |_| if pin().len() < 8 { pin.with_mut(|p| p.push('0')) }, "0" }
                    button { r#type: "button", class: "lock-key ghost", disabled: checking() || pin().is_empty(), onclick: move |_| submit(), "Mở" }
                }
                if confirm_forgot() {
                    p { class: "modal-sub", "Quên mã PIN thì đăng xuất và đăng nhập lại bằng tài khoản; mã PIN trên thiết bị này sẽ bị xóa." }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_forgot.set(false), "Hủy" }
                        button { r#type: "button", class: "btn-danger", onclick: move |_| on_forgot.call(()), "Đăng xuất" }
                    }
                } else {
                    button { r#type: "button", class: "btn-ghost", onclick: move |_| confirm_forgot.set(true), "Quên mã PIN?" }
                }
            }
        }
    }
}
