use dioxus::prelude::*;

use crate::{
    components::icons::{GoogleIcon, HashIcon, LockIcon, LogoIcon, PhoneIcon},
    models::ProvidersResponse,
    platform,
    services::api,
};

fn auth_error_message(code: &str) -> String {
    match code {
        "username_taken" => "This username is already taken".to_string(),
        "invalid_credentials" => "Incorrect username or password".to_string(),
        "password_not_set_for_account" => {
            "This account has no password yet. Sign in with Google or set a password in Settings".to_string()
        }
        "account_suspended" => "This account has been suspended".to_string(),
        "invalid_password_length" => "Password must be at least 6 characters".to_string(),
        "empty_username" | "invalid_username_length" | "illegal_character" => {
            "Username must be 2 to 32 valid characters".to_string()
        }
        "" => "Authentication failed".to_string(),
        other => other.to_string(),
    }
}

#[component]
pub fn Login(on_login_success: EventHandler<()>) -> Element {
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut dev_username = use_signal(String::new);
    let mut pw_mode_register = use_signal(|| false);
    let mut auth_username = use_signal(String::new);
    let mut auth_password = use_signal(String::new);
    let mut auth_password_confirm = use_signal(String::new);
    let mut providers = use_signal(|| Option::<ProvidersResponse>::None);
    // Spec 011: the second step (TOTP / backup code) after Google or test sign-in.
    let mut twofa = use_signal(|| false);
    let mut active_legal_modal = use_signal(|| Option::<crate::components::legal::LegalDocKind>::None);

    use_effect(move || {
        spawn(async move {
            if crate::services::security::twofa_pending().await {
                twofa.set(true);
            }
            match api::auth_providers().await {
                Ok(p) => providers.set(Some(p)),
                Err(_) => error.set(Some("Unable to connect to server".to_string())),
            }
        });
    });

    let handle_google_login = move |_| {
        let google_enabled = providers().map(|p| p.google).unwrap_or(false);
        if !google_enabled {
            error.set(Some(
                "Google Sign In is not configured on the server.".to_string(),
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

    let handle_password_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        if loading() {
            return;
        }
        let uname = auth_username().trim().to_string();
        let pw = auth_password();
        let is_reg = pw_mode_register();
        if !(2..=32).contains(&uname.chars().count()) {
            error.set(Some("Username must be 2 to 32 characters".to_string()));
            return;
        }
        if pw.chars().count() < 6 || pw.len() > 128 {
            error.set(Some("Password must be between 6 and 128 characters".to_string()));
            return;
        }
        if is_reg && pw != auth_password_confirm() {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }
        loading.set(true);
        error.set(None);
        spawn(async move {
            let res = if is_reg {
                api::auth_register(&uname, &pw).await
            } else {
                api::auth_login(&uname, &pw).await
            };
            match res {
                Ok(()) => {
                    crate::crypto::e2ee::set_ephemeral_account_password(&pw);
                    auth_password.set(String::new());
                    auth_password_confirm.set(String::new());
                    if crate::services::security::twofa_pending().await {
                        twofa.set(true);
                    } else {
                        on_login_success.call(());
                    }
                }
                Err(err) => {
                    error.set(Some(auth_error_message(&err.code)));
                }
            }
            loading.set(false);
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
                Ok(()) => {
                    if crate::services::security::twofa_pending().await {
                        twofa.set(true);
                    } else {
                        on_login_success.call(());
                    }
                }
                Err(err) => {
                    let msg = if err.code.is_empty() {
                        "Login failed".to_string()
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
            section { class: "auth-hero", aria_label: "Introducing ChatW",
                div { class: "auth-brand",
                    span { class: "brand-mark", LogoIcon { size: 22 } }
                    span { "chatw" }
                }
                div { class: "auth-hero-copy",
                    h2 { class: "auth-hero-title", "Private messaging, neat community." }
                    p { class: "auth-hero-sub",
                        "End-to-end encrypted live chat right on your device, with groups, multi-channel communities, and voice and video calls."
                    }
                }
                div { class: "auth-features",
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", LockIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "End-to-end encrypted" }
                            div { class: "auth-feature-sub", "ECDH P-256, AES-256-GCM and ECDSA signature. The private key does not leave the device." }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", HashIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Communities and channels" }
                            div { class: "auth-feature-sub", "Categories, text channels, voice channels and role authorization." }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", PhoneIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Voice and video calling" }
                            div { class: "auth-feature-sub", "Direct WebRTC calls between devices." }
                        }
                    }
                }
                div { class: "auth-hero-foot", "Open source · Rust · Dioxus" }
            }

            main { class: "auth-panel",
                if twofa() {
                    div { class: "auth-card",
                        crate::components::accounts::TwoFactorPrompt {
                            on_done: move |_| { twofa.set(false); on_login_success.call(()); },
                            on_cancel: move |_| twofa.set(false),
                        }
                    }
                } else {
                div { class: "auth-card",
                    crate::components::accounts::SignedInAccounts {}
                    div { class: "auth-card-head",
                        div { class: "auth-brand auth-card-brand",
                            span { class: "brand-mark", LogoIcon { size: 22 } }
                            span { "chatw" }
                        }
                        h1 { class: "auth-title", if pw_mode_register() { "Create account" } else { "Sign in" } }
                        p { class: "auth-subtitle", "Sign in with your account password to sync keys, chats, and settings across all devices automatically." }
                    }

                    form {
                        class: "auth-pw-form",
                        onsubmit: handle_password_submit,
                        div { class: "auth-mode-tabs", role: "tablist",
                            button {
                                r#type: "button",
                                role: "tab",
                                "aria-selected": if !pw_mode_register() { "true" } else { "false" },
                                class: if !pw_mode_register() { "auth-mode-tab active" } else { "auth-mode-tab" },
                                onclick: move |_| {
                                    pw_mode_register.set(false);
                                    error.set(None);
                                },
                                "Sign in"
                            }
                            button {
                                r#type: "button",
                                role: "tab",
                                "aria-selected": if pw_mode_register() { "true" } else { "false" },
                                class: if pw_mode_register() { "auth-mode-tab active" } else { "auth-mode-tab" },
                                onclick: move |_| {
                                    pw_mode_register.set(true);
                                    error.set(None);
                                },
                                "Create account"
                            }
                        }
                        input {
                            r#type: "text",
                            class: "auth-input",
                            placeholder: "Username",
                            aria_label: "Username",
                            maxlength: "32",
                            autocomplete: "username",
                            value: "{auth_username}",
                            oninput: move |evt| {
                                auth_username.set(evt.value());
                                error.set(None);
                            },
                        }
                        input {
                            r#type: "password",
                            class: "auth-input",
                            placeholder: "Account password (min 6 chars)",
                            aria_label: "Password",
                            maxlength: "128",
                            autocomplete: if pw_mode_register() { "new-password" } else { "current-password" },
                            value: "{auth_password}",
                            oninput: move |evt| {
                                auth_password.set(evt.value());
                                error.set(None);
                            },
                        }
                        if pw_mode_register() {
                            input {
                                r#type: "password",
                                class: "auth-input",
                                placeholder: "Confirm password",
                                aria_label: "Confirm password",
                                maxlength: "128",
                                autocomplete: "new-password",
                                value: "{auth_password_confirm}",
                                oninput: move |evt| {
                                    auth_password_confirm.set(evt.value());
                                    error.set(None);
                                },
                            }
                        }
                        button {
                            r#type: "submit",
                            class: "auth-btn-primary",
                            disabled: loading(),
                            if loading() {
                                span { class: "spinner" }
                            } else if pw_mode_register() {
                                "Create account"
                            } else {
                                "Sign in"
                            }
                        }
                    }

                    if !google_disabled {
                        div { class: "auth-divider", span { "or" } }
                        button {
                            r#type: "button",
                            class: "auth-btn-google",
                            disabled: providers().is_none(),
                            aria_label: "Continue with Google",
                            onclick: handle_google_login,
                            GoogleIcon {}
                            span { "Continue with Google" }
                        }
                    }

                    if let Some(err_msg) = error() {
                        div { class: "auth-error", role: "alert", "{err_msg}" }
                    }

                    if show_dev {
                        form {
                            class: "auth-dev-form",
                            onsubmit: handle_dev_login,
                            div { class: "auth-dev-title", "Test sign in" }
                            div { class: "auth-dev-row",
                                input {
                                    r#type: "text",
                                    class: "auth-input",
                                    placeholder: "Test account name, for example alice",
                                    aria_label: "Test username",
                                    maxlength: "32",
                                    autocomplete: "off",
                                    value: "{dev_username}",
                                    oninput: move |evt| dev_username.set(evt.value()),
                                }
                                button {
                                    r#type: "submit",
                                    class: "auth-btn-secondary",
                                    disabled: loading(),
                                    if loading() { span { class: "spinner" } } else { "Enter" }
                                }
                            }
                        }
                    }

                    div { class: "auth-legal-container",
                        p { class: "auth-legal",
                            "When you continue, your encryption key is generated and saved on this device. The server only receives the public key."
                        }
                        div { class: "auth-legal-links",
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Terms)),
                                "Điều khoản dịch vụ"
                            }
                            span { class: "auth-legal-sep", "•" }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Privacy)),
                                "Chính sách bảo mật"
                            }
                            span { class: "auth-legal-sep", "•" }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Community)),
                                "Tiêu chuẩn cộng đồng"
                            }
                            span { class: "auth-legal-sep", "•" }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Vision)),
                                "Tầm nhìn sản phẩm"
                            }
                        }
                    }
                }
                }
            }
            crate::components::legal::LegalModal {
                doc: active_legal_modal(),
                on_close: move |_| active_legal_modal.set(None),
            }
        }
    }
}
