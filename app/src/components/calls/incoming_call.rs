use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{PhoneIcon, PhoneOffIcon, VideoIcon},
    },
    models::CallSessionItem,
};

#[component]
pub fn IncomingCallModal(
    session: CallSessionItem,
    on_accept: EventHandler<()>,
    on_reject: EventHandler<()>,
) -> Element {
    let is_video = session.call_type == "video";
    let is_group = session.group_id.is_some();

    rsx! {
        div { class: "settings-modal-backdrop", style: "background: rgba(0, 0, 0, 0.75); z-index: 9999;",
            div {
                class: "settings-modal",
                style: "max-width: 360px; text-align: center; padding: 24px;",
                onclick: move |e| e.stop_propagation(),
                div { style: "display: flex; justify-content: center; margin-bottom: 16px;",
                    Avatar {
                        user_id: session.caller_id.clone(),
                        username: session.caller_name.clone(),
                        avatar_url: session.caller_avatar_url.clone(),
                        size: 80,
                    }
                }

                h3 { style: "margin: 0 0 6px 0; font-size: 18px;", "{session.caller_name}" }

                p { style: "color: var(--text-secondary); font-size: 14px; margin: 0 0 24px 0;",
                    if is_group {
                        if let Some(ref gname) = session.group_name {
                            span { "Group call from {gname}" }
                        } else {
                            span { "Group call" }
                        }
                    } else if is_video {
                        span { "Video call to" }
                    } else {
                        span { "Incoming voice call" }
                    }
                }

                div { style: "display: flex; justify-content: center; gap: 32px; align-items: center;",
                    // Reject Button
                    button {
                        class: "partner-action-btn call-btn call-btn-danger",
                        onclick: move |_| on_reject.call(()),
                        PhoneOffIcon { size: 24 }
                    }

                    // Accept Button
                    button {
                        class: "partner-action-btn call-btn call-btn-success",
                        onclick: move |_| on_accept.call(()),
                        if is_video {
                            VideoIcon { size: 24 }
                        } else {
                            PhoneIcon { size: 24 }
                        }
                    }
                }
            }
        }
    }
}
