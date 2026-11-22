use dioxus::prelude::*;

use crate::{
    components::icons::LogoIcon,
    crypto::e2ee::{create_and_store_user_keys, get_user_keys},
    services::api,
};

#[component]
pub fn SetupUsername(on_complete: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let handle_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        let trimmed = name().trim().to_string();
        let char_len = trimmed.chars().count();
        if !(2..=32).contains(&char_len) {
            error.set("Name must be between 2 and 32 characters".to_string());
            return;
        }

        loading.set(true);
        error.set(String::new());

        spawn(async move {
            match api::users_set_name(&trimmed).await {
                Ok(user_resp) => {
                    let keys_res = match get_user_keys(Some(&user_resp.id)) {
                        Some(k) => Ok(k),
                        None => create_and_store_user_keys(Some(&user_resp.id)),
                    };
                    let Ok(keys) = keys_res else {
                        error.set("A secure encryption keystore cannot be created on this device".to_string());
                        loading.set(false);
                        return;
                    };
                    match api::keys_register(&keys.ecdh_raw_b64, &keys.ecdsa_raw_b64).await {
                        Ok(_) => on_complete.call(()),
                        Err(err)
                            if err.code == "keys_already_registered"
                                || err.code == "identity_key_already_exists_require_recovery_or_rotation" =>
                        {
                            on_complete.call(())
                        }
                        Err(_) => {
                            error.set("Cannot save name. Please try again".to_string());
                        }
                    }
                }
                Err(err) => {
                    if err.code == "username_taken" {
                        error.set("This name is already in use".to_string());
                    } else {
                        error.set("Cannot save name. Please try again".to_string());
                    }
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "auth-container",
            form { class: "auth-box", onsubmit: handle_submit,
                div { class: "auth-logo",
                    LogoIcon { size: 44 }
                }
                h2 { class: "auth-title", "Give your name" }
                input {
                    r#type: "text",
                    placeholder: "Enter username",
                    value: "{name}",
                    disabled: loading(),
                    autofocus: true,
                    aria_label: "Username",
                    oninput: move |evt| {
                        name.set(evt.value());
                        error.set(String::new());
                    },
                }
                if !error().is_empty() {
                    div { class: "error-text", "{error}" }
                }
                button {
                    r#type: "submit",
                    class: "auth-btn-primary",
                    disabled: loading(),
                    aria_label: "Continue",
                    if loading() { "Saving" } else { "Continue" }
                }
            }
        }
    }
}

