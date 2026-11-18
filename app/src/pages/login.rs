use dioxus::prelude::*;

use crate::{
    components::icons::{GoogleIcon, LogoIcon},
    models::ProvidersResponse,
    platform,
    services::api,
};

#[component]
pub fn Login(on_login_success: EventHandler<()>) -> Element {
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut dev_username = use_signal(String::new);
    let mut providers = use_signal(|| Option::<ProvidersResponse>::None);

    use_effect(move || {
        spawn(async move {
            match api::auth_providers().await {
                Ok(p) => providers.set(Some(p)),
                Err(_) => error.set(Some("Không kết nối được máy chủ".to_string())),
            }
        });
    });

    let handle_google_login = move |_| {
        let google_enabled = providers().map(|p| p.google).unwrap_or(false);
        if !google_enabled {
            error.set(Some(
                "Đăng nhập Google chưa được cấu hình trên máy chủ (GOOGLE_CLIENT_ID).".to_string(),
            ));
            return;
        }
        error.set(None);
        spawn(async move {
            match platform::start_google_oauth().await {
                Ok(true) => on_login_success.call(()),
                Ok(false) => {}
                Err(msg) => error.set(Some(msg)),
            }
        });
    };

    let handle_dev_login = move |evt: Event<FormData>| {
        evt.prevent_default();
        if loading() {
            return;
        }
        loading.set(true);
        error.set(None);
        let uname = dev_username().trim().to_string();
        let uname_opt = if uname.is_empty() { None } else { Some(uname) };
        spawn(async move {
            match api::auth_dev_login(uname_opt).await {
                Ok(()) => on_login_success.call(()),
                Err(err) => {
                    let msg = if err.code.is_empty() {
                        "Đăng nhập không thành công".to_string()
                    } else {
                        err.code
                    };
                    error.set(Some(msg));
                }
            }
            loading.set(false);
        });
    };

    let google_disabled = providers().map(|p| !p.google).unwrap_or(false);
    let show_dev = providers().map(|p| p.dev).unwrap_or(false);

    rsx! {
        div { class: "auth-container",
            div { class: "auth-box",
                div { class: "auth-logo",
                    LogoIcon { size: 44 }
                }
                h1 { class: "auth-title", "chatw" }
                p { class: "auth-subtitle", "Ứng dụng nhắn tin bảo mật E2EE" }

                button {
                    r#type: "button",
                    class: "auth-btn-primary",
                    disabled: google_disabled,
                    aria_label: "Tiếp tục với Google",
                    onclick: handle_google_login,
                    GoogleIcon {}
                    span { "Tiếp tục với Google" }
                }

                if show_dev {
                    div { class: "auth-divider",
                        span { "chế độ phát triển cục bộ" }
                    }

                    form {
                        class: "auth-dev-form",
                        onsubmit: handle_dev_login,
                        input {
                            r#type: "text",
                            class: "auth-input",
                            placeholder: "Tên tài khoản thử (ví dụ: alice)",
                            maxlength: "32",
                            value: "{dev_username}",
                            oninput: move |evt| dev_username.set(evt.value()),
                        }
                        button {
                            r#type: "submit",
                            class: "auth-btn-secondary",
                            disabled: loading(),
                            if loading() { "Đang đăng nhập" } else { "Đăng nhập nhanh" }
                        }
                    }
                }

                if let Some(err_msg) = error() {
                    div { class: "auth-error", "{err_msg}" }
                }
            }
        }
    }
}

