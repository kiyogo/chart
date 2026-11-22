use dioxus::prelude::*;

use crate::{
    components::icons::CloseIcon,
    services::groups::{groups_add_member, groups_create_invite},
};

#[component]
pub fn GroupInviteModal(group_id: String, on_close: EventHandler<()>) -> Element {
    let mut invite_code = use_signal(String::new);
    let mut copied = use_signal(|| false);
    let mut is_loading = use_signal(|| true);

    let gid = group_id.clone();
    use_effect(move || {
        let gid = gid.clone();
        spawn(async move {
            if let Ok(inv) = groups_create_invite(&gid, None, None).await {
                invite_code.set(inv.code);
            }
            is_loading.set(false);
        });
    });

    let handle_copy = move |_| {
        let code = invite_code();
        let js = format!(
            "if (navigator.clipboard) {{ navigator.clipboard.writeText('{}'); }}",
            code
        );
        let _ = dioxus::document::eval(&js);
        copied.set(true);
    };

    rsx! {
        div { class: "settings-modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "settings-modal",
                style: "max-width: 440px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "settings-modal-header",
                    h2 { "Group invitation link" }
                    button {
                        class: "settings-close-btn",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                }
                div { class: "settings-modal-content",
                    p {
                        style: "color: var(--text-secondary); font-size: 13px; margin-bottom: 16px;",
                        "Share this invite code or link with anyone so they can join the group."
                    }

                    if is_loading() {
                        div { style: "text-align: center; padding: 20px;", "Creating link" }
                    } else {
                        div { class: "settings-section",
                            label { class: "settings-label", "Invitation code" }
                            div { style: "display: flex; gap: 8px; margin-top: 6px;",
                                input {
                                    class: "search-input",
                                    style: "flex: 1; font-family: monospace; font-size: 14px;",
                                    r#type: "text",
                                    readonly: true,
                                    value: "{invite_code}",
                                }
                                button {
                                    class: "add-friend-submit-btn",
                                    onclick: handle_copy,
                                    if copied() { "Copied!" } else { "Copy" }
                                }
                            }
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; margin-top: 24px;",
                        button {
                            class: "partner-action-btn",
                            onclick: move |_| on_close.call(()),
                            "Close"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AddMemberModal(
    group_id: String,
    on_close: EventHandler<()>,
    on_member_added: EventHandler<()>,
) -> Element {
    let mut username = use_signal(String::new);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut is_submitting = use_signal(|| false);

    let handle_submit = move |_| {
        let uname = username();
        if uname.trim().is_empty() {
            error_msg.set(Some("Please enter username".to_string()));
            return;
        }

        let gid = group_id.clone();
        is_submitting.set(true);
        error_msg.set(None);

        spawn(async move {
            match groups_add_member(&gid, None, Some(uname.trim().to_string())).await {
                Ok(_) => {
                    is_submitting.set(false);
                    on_member_added.call(());
                }
                Err(err) => {
                    is_submitting.set(false);
                    error_msg.set(Some(format!("Not found or unable to add: {err}")));
                }
            }
        });
    };

    rsx! {
        div { class: "settings-modal-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "settings-modal",
                style: "max-width: 440px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "settings-modal-header",
                    h2 { "Add members" }
                    button {
                        class: "settings-close-btn",
                        onclick: move |_| on_close.call(()),
                        CloseIcon {}
                    }
                }
                div { class: "settings-modal-content",
                    if let Some(err) = error_msg() {
                        div { class: "form-error", style: "margin-bottom: 12px;", "{err}" }
                    }

                    div { class: "settings-section",
                        label { class: "settings-label", "Username" }
                        input {
                            class: "search-input",
                            style: "width: 100%; box-sizing: border-box; margin-top: 6px;",
                            r#type: "text",
                            placeholder: "Enter account name",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                    }

                    div { style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 20px;",
                        button {
                            class: "partner-action-btn",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button {
                            class: "add-friend-submit-btn",
                            disabled: is_submitting(),
                            onclick: handle_submit,
                            if is_submitting() { "Adding" } else { "Add" }
                        }
                    }
                }
            }
        }
    }
}

