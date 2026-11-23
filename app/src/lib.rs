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
    crypto::e2ee::{
        clear_legacy_active_key, create_and_store_user_keys, get_user_keys,
        has_locked_passphrase_vault, lock_and_wipe_session_keys,
    },
    models::CurrentUser,
    pages::{chat::Chat, login::Login, setup_username::SetupUsername},
    services::api,
    storage::E2EE_RECOVERY_REQUIRED,
};

const THEME_CSS: &str = include_str!("../assets/theme.css");
const MAIN_CSS: &str = include_str!("../assets/main.css");
const FONTS_CSS: &str = include_str!("../assets/fonts.css");

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
            // 0ms Offline-First Hydration (Spec 071): if a cached session profile exists in IndexedDB,
            // render the Chat shell immediately before network verification (survives complete offline boot!).
            let mut hydrated_offline = false;
            if let Some(cached_me) = platform::load_dialogs_cache("me_profile").await {
                let cid = cached_me.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let cuname = cached_me.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let cav = cached_me.get("avatar_url").and_then(|v| v.as_str()).map(str::to_string);
                if !cid.is_empty() && !cuname.is_empty() {
                    current_user.set(Some(CurrentUser {
                        id: cid,
                        username: cuname,
                        avatar_url: cav,
                    }));
                    status.set(AuthStatus::Ready);
                    hydrated_offline = true;
                }
            }

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

                    // Safe E2EE identity key lifecycle:
                    // 1. If account has NO E2EE keys on server yet (`!me.has_keys`), enroll this first device.
                    // 2. If account ALREADY has E2EE keys on server (`me.has_keys`), NEVER auto-generate a new
                    //    keypair and silently overwrite `device_keys`! Instead signal recovery/approval/unlock.
                    *E2EE_RECOVERY_REQUIRED.write() = None;
                    if !me.has_keys {
                        let keys_res = match get_user_keys(Some(&me.id)) {
                            Some(k) => Ok(k),
                            None => create_and_store_user_keys(Some(&me.id)),
                        };
                        if let Ok(keys) = keys_res {
                            let _ = api::keys_register(&keys.ecdh_raw_b64, &keys.ecdsa_raw_b64).await;
                        }
                    } else if has_locked_passphrase_vault(&me.id) {
                        *E2EE_RECOVERY_REQUIRED.write() = Some("locked_vault".to_string());
                    } else if let Some(local_keys) = get_user_keys(Some(&me.id)) {
                        if let Ok(srv) = api::keys_get(&me.id).await {
                            let matches_srv = srv.fingerprint == local_keys.fingerprint_b64
                                || srv.fingerprint == local_keys.identity_fingerprint_b64;
                            if !matches_srv {
                                *E2EE_RECOVERY_REQUIRED.write() =
                                    Some("server_key_mismatch".to_string());
                            }
                        }
                    } else {
                        // Existing account on a fresh browser/device: await trusted-device sync or backup passphrase.
                        *E2EE_RECOVERY_REQUIRED.write() = Some("missing_local_keys".to_string());
                    }

                    platform::save_dialogs_cache(
                        "me_profile",
                        &serde_json::json!({
                            "id": me.id,
                            "username": uname,
                            "avatar_url": me.avatar_url,
                        }),
                    )
                    .await;
                    current_user.set(Some(CurrentUser {
                        id: me.id,
                        username: uname,
                        avatar_url: me.avatar_url,
                    }));
                    status.set(AuthStatus::Ready);
                }
                Err(err) => {
                    if hydrated_offline && err.status == 0 {
                        // Offline boot: stay in Ready offline mode with IndexedDB dialogs, messages & outbox!
                        return;
                    }
                    if err.status == 401 || err.status == 403 {
                        platform::wipe_client_session_on_logout().await;
                    }
                    current_user.set(None);
                    status.set(AuthStatus::Unauthenticated);
                }
            }
        });
    };

    use_effect(move || {
        platform::apply_appearance(&platform::load_appearance());
        i18n::init_from_storage();
        check_auth();
    });

    let handle_logout = move |_| {
        let uid = current_user().map(|u| u.id);
        spawn(async move {
            let _ = api::auth_logout().await;
            platform::wipe_client_session_on_logout().await;
            lock_and_wipe_session_keys(uid.as_deref());
            clear_legacy_active_key();
            *E2EE_RECOVERY_REQUIRED.write() = None;
            current_user.set(None);
            status.set(AuthStatus::Unauthenticated);
        });
    };

    rsx! {
        document::Style { {THEME_CSS} }
        document::Style { {MAIN_CSS} }
        document::Style { {FONTS_CSS} }
        div { class: "app-layout",
            div { class: "app-content",
                match status() {
                    AuthStatus::Loading => rsx! {
                        div { class: "boot-screen", role: "status",
                            span { class: "brand-mark", components::icons::LogoIcon { size: 22 } }
                            span { "Initializing session" }
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
