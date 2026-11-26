use dioxus::prelude::*;

use crate::{
    components::icons::{GoogleIcon, HashIcon, KiyoIcon, LockIcon, LogoIcon, PhoneIcon},
    models::ProvidersResponse,
    platform,
    services::api,
};

#[component]
pub fn Login(on_login_success: EventHandler<()>) -> Element {
    let mut error = use_signal(|| Option::<String>::None);
    let mut providers = use_signal(|| Option::<ProvidersResponse>::None);
    // Spec 011: the second step (TOTP / backup code) after OIDC/OAuth sign-in.
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

    let handle_kiyo_login = move |_| {
        let kiyo_enabled = providers().map(|p| p.kiyo).unwrap_or(false);
        if !kiyo_enabled {
            error.set(Some(
                "Kiyo Sign In is not configured on the server".to_string(),
            ));
            return;
        }
        error.set(None);
        crate::crypto::e2ee::clear_ephemeral_account_password();
        spawn(async move {
            match platform::start_kiyo_oauth().await {
                Ok(true) => on_login_success.call(()),
                Ok(false) => {}
                Err(msg) => error.set(Some(msg)),
            }
        });
    };

    let handle_google_login = move |_| {
        let google_enabled = providers().map(|p| p.google).unwrap_or(false);
        if !google_enabled {
            error.set(Some(
                "Google Sign In is not configured on the server".to_string(),
            ));
            return;
        }
        error.set(None);
        crate::crypto::e2ee::clear_ephemeral_account_password();
        spawn(async move {
            match platform::start_google_oauth().await {
                Ok(true) => on_login_success.call(()),
                Ok(false) => {}
                Err(msg) => error.set(Some(msg)),
            }
        });
    };

    let google_disabled = providers().map(|p| !p.google).unwrap_or(false);

    rsx! {
        div { class: "auth-shell",
            section { class: "auth-hero", aria_label: "Introducing ChatW",
                div { class: "auth-brand",
                    span { class: "brand-mark", LogoIcon { size: 22 } }
                    span { "chatw" }
                }
                div { class: "auth-hero-copy",
                    h2 { class: "auth-hero-title", "Private messaging, neat community" }
                    p { class: "auth-hero-sub",
                        "End-to-end encrypted live chat right on your device, with groups, multi-channel communities, and voice and video calls"
                    }
                }
                div { class: "auth-features",
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", LockIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "End-to-end encrypted" }
                            div { class: "auth-feature-sub", "ECDH P-256, AES-256-GCM and ECDSA signature — the private key does not leave the device" }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", HashIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Communities and channels" }
                            div { class: "auth-feature-sub", "Categories, text channels, voice channels and role authorization" }
                        }
                    }
                    div { class: "auth-feature",
                        span { class: "auth-feature-icon", PhoneIcon { size: 18 } }
                        div {
                            div { class: "auth-feature-title", "Voice and video calling" }
                            div { class: "auth-feature-sub", "Direct WebRTC calls between devices" }
                        }
                    }
                }
                div { class: "auth-hero-foot",
                    span { "Open source" }
                    span { "Rust" }
                    span { "Dioxus" }
                }
            }

            main { class: "auth-panel",
                div { class: "auth-panel-main",
                    if twofa() {
                        div { class: "auth-card",
                            crate::components::accounts::TwoFactorPrompt {
                                on_done: move |_| { twofa.set(false); on_login_success.call(()); },
                                on_cancel: move |_| {
                                    crate::crypto::e2ee::clear_ephemeral_account_password();
                                    twofa.set(false);
                                },
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
                                h1 { class: "auth-title", "Sign in" }
                                p { class: "auth-subtitle", "Sign in with Kiyo or Google to sync your encrypted chats and settings" }
                            }

                            div { class: "auth-actions",
                                button {
                                    r#type: "button",
                                    class: "auth-btn-google auth-btn-kiyo",
                                    disabled: providers().is_none(),
                                    aria_label: "Continue with Kiyo",
                                    onclick: handle_kiyo_login,
                                    KiyoIcon { size: 20 }
                                    span { "Continue with Kiyo" }
                                }

                                if !google_disabled {
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
                            }

                            if let Some(err_msg) = error() {
                                div { class: "auth-error", role: "alert", "{err_msg}" }
                            }
                        }
                    }
                }

                if !twofa() {
                    div { class: "auth-legal-container",
                        p { class: "auth-legal",
                            "When you continue, your encryption key is generated and saved on this device, and the server only receives the public key"
                        }
                        div { class: "auth-legal-links",
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Terms)),
                                "Terms of Service"
                            }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Privacy)),
                                "Privacy Policy"
                            }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Community)),
                                "Community Standards"
                            }
                            button {
                                r#type: "button",
                                class: "auth-legal-btn",
                                onclick: move |_| active_legal_modal.set(Some(crate::components::legal::LegalDocKind::Vision)),
                                "Product Vision"
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
