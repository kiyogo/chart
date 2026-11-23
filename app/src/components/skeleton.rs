//! Unified Shimmer Skeleton Loading Components for chatw (Spec 095)
//!
//! Provides lightweight, fluid 60fps shimmer placeholder elements for:
//! - Conversation cards in the left sidebar
//! - Stories/Notes horizontal strip
//! - Chat message bubbles (replacing static spinners)
//! - Server channels & friends lists
//! - Right-side details panel

use dioxus::prelude::*;

#[component]
pub fn SkeletonStories() -> Element {
    rsx! {
        div {
            class: "skeleton-notes-strip",
            role: "status",
            aria_label: "Loading stories",
            for i in 0..5 {
                div {
                    key: "sk-story-{i}",
                    class: "skeleton-story-item",
                    div { class: "skeleton-circle skeleton-story-avatar" }
                    div { class: "skeleton-box skeleton-story-name" }
                }
            }
        }
    }
}

#[component]
pub fn SkeletonConversations(#[props(default = 6)] count: usize) -> Element {
    rsx! {
        div {
            class: "skeleton-conversations",
            role: "status",
            aria_label: "Loading conversations",
            for i in 0..count {
                div {
                    key: "sk-conv-{i}",
                    class: "skeleton-conv-row",
                    div { class: "skeleton-circle skeleton-conv-avatar" }
                    div { class: "skeleton-conv-body",
                        div { class: "skeleton-conv-line-top",
                            div {
                                class: "skeleton-box skeleton-conv-title",
                                style: if i % 2 == 0 { "width: 48%;" } else { "width: 36%;" },
                            }
                            div { class: "skeleton-box skeleton-conv-badge" }
                        }
                        div {
                            class: "skeleton-box skeleton-conv-sub",
                            style: if i % 3 == 0 { "width: 78%;" } else if i % 2 == 0 { "width: 60%;" } else { "width: 70%;" },
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SkeletonMessages() -> Element {
    rsx! {
        div {
            class: "skeleton-messages-wrap",
            role: "status",
            aria_label: "Loading messages",
            // 1. Received message
            div { class: "skeleton-msg-row received",
                div { class: "skeleton-circle skeleton-msg-avatar" }
                div { class: "skeleton-box skeleton-msg-bubble", style: "width: 220px; height: 44px;" }
            }
            // 2. Sent message
            div { class: "skeleton-msg-row sent",
                div { class: "skeleton-box skeleton-msg-bubble", style: "width: 170px; height: 38px;" }
            }
            // 3. Received longer message
            div { class: "skeleton-msg-row received",
                div { class: "skeleton-circle skeleton-msg-avatar" }
                div { class: "skeleton-box skeleton-msg-bubble", style: "width: 270px; height: 60px;" }
            }
            // 4. Sent message
            div { class: "skeleton-msg-row sent",
                div { class: "skeleton-box skeleton-msg-bubble", style: "width: 200px; height: 42px;" }
            }
            // 5. Received short message
            div { class: "skeleton-msg-row received",
                div { class: "skeleton-circle skeleton-msg-avatar" }
                div { class: "skeleton-box skeleton-msg-bubble", style: "width: 180px; height: 36px;" }
            }
        }
    }
}

#[component]
pub fn SkeletonChannels(#[props(default = 5)] count: usize) -> Element {
    rsx! {
        div {
            class: "skeleton-channels",
            role: "status",
            aria_label: "Loading channels",
            for i in 0..count {
                div {
                    key: "sk-ch-{i}",
                    class: "skeleton-channel-row",
                    div { class: "skeleton-box skeleton-channel-icon" }
                    div {
                        class: "skeleton-box skeleton-channel-title",
                        style: if i % 2 == 0 { "width: 65%;" } else { "width: 48%;" },
                    }
                }
            }
        }
    }
}

#[component]
pub fn SkeletonDetails() -> Element {
    rsx! {
        div {
            class: "skeleton-details-wrap",
            role: "status",
            aria_label: "Loading details",
            div { class: "skeleton-box skeleton-details-banner" }
            div { class: "skeleton-circle skeleton-details-avatar" }
            div { class: "skeleton-box skeleton-details-title" }
            div { class: "skeleton-box skeleton-details-sub" }
            div { class: "skeleton-details-actions",
                for i in 0..4 {
                    div { key: "sk-act-{i}", class: "skeleton-box skeleton-details-action-btn" }
                }
            }
        }
    }
}
