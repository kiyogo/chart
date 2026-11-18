use dioxus::prelude::*;

use crate::{
    components::icons::{GoogleIcon, HashIcon, LockIcon, LogoIcon, PhoneIcon},
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
        div { class: "auth-shell",
            section { class: "auth-hero", aria_label: "Giới thiệu chatw",
                div { class: "auth-brand",
                    span { class: "brand-mark", LogoIcon { size: 22 } }
                    span { "chatw" }
                }
                div { class: "auth-hero-copy",
                    h2 { class: "auth-hero-title", "Nhắn tin riêng tư, cộng đồng gọn gàng." }
                    p { class: "auth-hero-sub",
                        "Trò chuyện trực tiếp được mã hóa đầu cuối ngay trên thiết bị, cùng nhóm, cộng đồng nhiều kênh và cuộc gọi thoại, video."
                    }
                }
                div { class: "auth-features",
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", LockIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Mã hóa đầu cuối" }
                            div { class: "auth-feature-sub", "ECDH P-256, AES-256-GCM và chữ ký ECDSA. Khóa riêng tư không rời thiết bị." }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", HashIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Cộng đồng và kênh" }
                            div { class: "auth-feature-sub", "Danh mục, kênh văn bản, kênh thoại và phân quyền theo vai trò." }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", PhoneIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Gọi thoại và video" }
                            div { class: "auth-feature-sub", "Cuộc gọi WebRTC trực tiếp giữa các thiết bị." }
                        }
                    }
                }
                div { class: "auth-hero-foot", "Mã nguồn mở · Rust · Dioxus" }
            }

            main { class: "auth-panel",
                div { class: "auth-card",
                    div { class: "auth-card-head",
                        div { class: "auth-brand auth-card-brand",
                            span { class: "brand-mark", LogoIcon { size: 22 } }
                            span { "chatw" }
                        }
                        h1 { class: "auth-title", "Đăng nhập" }
                        p { class: "auth-subtitle", "Dùng tài khoản Google để tiếp tục. Lần đầu đăng nhập, bạn sẽ chọn tên người dùng." }
                    }

                    button {
                        r#type: "button",
                        class: "auth-btn-google",
                        disabled: google_disabled || providers().is_none(),
                        aria_label: "Tiếp tục với Google",
                        onclick: handle_google_login,
                        GoogleIcon {}
                        span { "Tiếp tục với Google" }
                    }
                    if google_disabled {
                        div { class: "form-hint", "Máy chủ chưa cấu hình đăng nhập Google." }
                    }

                    if let Some(err_msg) = error() {
                        div { class: "auth-error", role: "alert", "{err_msg}" }
                    }

                    if show_dev {
                        form {
                            class: "auth-dev-form",
                            onsubmit: handle_dev_login,
                            div { class: "auth-dev-title", "Đăng nhập thử nghiệm (chỉ máy chủ cục bộ)" }
                            div { class: "auth-dev-row",
                                input {
                                    r#type: "text",
                                    class: "auth-input",
                                    placeholder: "Tên tài khoản thử, ví dụ alice",
                                    aria_label: "Tên tài khoản thử nghiệm",
                                    maxlength: "32",
                                    autocomplete: "off",
                                    value: "{dev_username}",
                                    oninput: move |evt| dev_username.set(evt.value()),
                                }
                                button {
                                    r#type: "submit",
                                    class: "auth-btn-secondary",
                                    disabled: loading(),
                                    if loading() { span { class: "spinner" } } else { "Vào" }
                                }
                            }
                        }
                    }

                    p { class: "auth-legal",
                        "Khi tiếp tục, khóa mã hóa của bạn được tạo và lưu trên thiết bị này. Máy chủ chỉ nhận khóa công khai."
                    }
                }
            }
        }
    }
}
