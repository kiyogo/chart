pub mod components;
pub mod crypto;
pub mod i18n;
pub mod media;
pub mod models;
pub mod pages;
pub mod platform;
pub mod prefs;
pub mod services;
pub mod storage;

use dioxus::prelude::*;

use crate::{
    crypto::e2ee::{clear_legacy_active_key, create_and_store_user_keys, get_user_keys},
    models::CurrentUser,
    pages::{chat::Chat, login::Login, setup_username::SetupUsername},
    services::api,
};

const THEME_CSS: &str = include_str!("../assets/theme.css");
const MAIN_CSS: &str = include_str!("../assets/main.css");

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthStatus {
    Loading,
    Unauthenticated,
    NeedsName,
    Ready,
}

#[component]
pub fn App() -> Element {
    let mut status = use_signal(|| AuthStatus::Loading);
    let mut current_user = use_signal(|| Option::<CurrentUser>::None);

    let check_auth = move || {
        spawn(async move {
            match api::auth_me().await {
                Ok(me) => {
                    let uname = match me.username {
                        Some(u) if !u.is_empty() => u,
                        _ => {
                            current_user.set(Some(CurrentUser {
                                id: me.id,
                                username: String::new(),
                                avatar_url: me.avatar_url,
                            }));
                            status.set(AuthStatus::NeedsName);
                            return;
                        }
                    };

                    let (keys, newly_generated) = match get_user_keys(Some(&me.id)) {
                        Some(k) => (k, false),
                        None => (create_and_store_user_keys(Some(&me.id)), true),
                    };

                    // Verify server key matches local active key so peers encrypt to this device's key
                    let server_key_matches = if me.has_keys && !newly_generated {
                        api::keys_get(&me.id)
                            .await
                            .map(|srv| srv.fingerprint == keys.fingerprint_b64)
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if !me.has_keys || newly_generated || !server_key_matches {
                        let _ = api::keys_register(&keys.ecdh_raw_b64, &keys.ecdsa_raw_b64).await;
                    }

                    current_user.set(Some(CurrentUser {
                        id: me.id,
                        username: uname,
                        avatar_url: me.avatar_url,
                    }));
                    status.set(AuthStatus::Ready);
                }
                Err(_) => {
                    status.set(AuthStatus::Unauthenticated);
                }
            }
        });
    };

    use_effect(move || {
        platform::apply_appearance(&platform::load_appearance());
        check_auth();
    });

    let handle_logout = move |_| {
        spawn(async move {
            let _ = api::auth_logout().await;
            clear_legacy_active_key();
            current_user.set(None);
            status.set(AuthStatus::Unauthenticated);
        });
    };

    rsx! {
        document::Style { {THEME_CSS} }
        document::Style { {MAIN_CSS} }
        div { class: "app-layout",
            div { class: "app-content",
                match status() {
                    AuthStatus::Loading => rsx! {
                        div { class: "boot-screen", role: "status",
                            span { class: "brand-mark", components::icons::LogoIcon { size: 22 } }
                            span { "Đang khởi tạo phiên làm việc" }
                        }
                    },
                    AuthStatus::Unauthenticated => rsx! {
                        Login { on_login_success: move |_| check_auth() }
                    },
                    AuthStatus::NeedsName => rsx! {
                        SetupUsername { on_complete: move |_| check_auth() }
                    },
                    AuthStatus::Ready => {
                        if let Some(user) = current_user() {
                            rsx! {
                                Chat {
                                    key: "{user.id}",
                                    current_user_id: user.id,
                                    current_username: user.username,
                                    current_user_avatar_url: user.avatar_url,
                                    on_logout: handle_logout,
                                    on_username_changed: move |name: String| {
                                        current_user.with_mut(|u| {
                                            if let Some(cur) = u {
                                                cur.username = name;
                                            }
                                        });
                                    },
                                    on_avatar_updated: move |new_url: String| {
                                        current_user.with_mut(|u| {
                                            if let Some(cur) = u {
                                                cur.avatar_url = Some(new_url);
                                            }
                                        });
                                    },
                                }
                            }
                        } else {
                            rsx! { div { class: "auth-container" } }
                        }
                    }
                }
            }
        }
    }
}
