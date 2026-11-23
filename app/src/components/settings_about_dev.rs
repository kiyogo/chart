//! Spec 067: iOS/Android-style About screen with Developer Mode toggle and
//! compact Developer Diagnostics panel.

use dioxus::prelude::*;

use crate::{
    components::icons::{BookIcon, BoltIcon, CopyIcon, FileTextIcon, LockIcon, LogoIcon, ShieldIcon, SparkleIcon},
    platform,
    prefs::PrefsCtx,
};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_BUILD: &str = "2026.11.23";

fn runtime_platform_label() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "Web (WebAssembly)"
    } else {
        "Desktop"
    }
}

fn build_diagnostics_text(lang: &str, theme: &str, accent: &str, panel_opacity: u8, has_keys: bool) -> String {
    format!(
        "ChatW v{APP_VERSION} (Build {APP_BUILD})\nPlatform: {}\nLanguage: {lang}\nTheme: {theme} | Accent: {accent} | Panel opacity: {panel_opacity}%\nEncryption: Double Ratchet + X25519 + AES-256-GCM ({})",
        runtime_platform_label(),
        if has_keys { "Keys loaded" } else { "No local keys" }
    )
}

#[component]
pub fn AboutOverviewPanel(
    has_keys: bool,
    on_open_developer: EventHandler<()>,
    on_open_guide: EventHandler<()>,
    on_replay_tour: EventHandler<()>,
    #[props(default)] on_open_terms: EventHandler<()>,
    #[props(default)] on_open_privacy: EventHandler<()>,
    #[props(default)] on_open_community: EventHandler<()>,
) -> Element {
    let prefs_ctx = use_context::<PrefsCtx>();
    let prefs = prefs_ctx.prefs.read().clone();
    let dev_mode = prefs.developer_mode;
    let appearance = platform::load_appearance();
    let mut build_taps = use_signal(|| 0u8);
    let mut copied_diag = use_signal(|| false);

    let active_accent = if prefs.custom_accent.is_empty() {
        appearance.accent.clone()
    } else {
        prefs.custom_accent.clone()
    };

    let diag_text = build_diagnostics_text(
        &prefs.language,
        &appearance.theme,
        &active_accent,
        prefs.panel_opacity,
        has_keys,
    );

    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                // Hero identity card (iOS / Android About style)
                div { class: "about-hero",
                    div { class: "about-hero-logo",
                        LogoIcon { size: 32 }
                    }
                    div { class: "about-hero-text",
                        div { class: "about-hero-title-row",
                            span { class: "about-hero-name", "ChatW" }
                            span { class: "about-version-pill", "v{APP_VERSION}" }
                        }
                        div { class: "about-hero-sub", "Private end-to-end encrypted messenger" }
                    }
                }

                // Grouped iOS / Android-style device & app info rows
                div { class: "settings-group",
                    div { class: "settings-group-title", "App information" }
                    div { class: "about-card",
                        div { class: "about-row",
                            span { class: "about-row-label", "App name" }
                            span { class: "about-row-val", "ChatW" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Version" }
                            span { class: "about-row-val mono", "{APP_VERSION}" }
                        }
                        button {
                            r#type: "button",
                            class: "about-row about-row-interactive",
                            aria_label: "Build number",
                            onclick: move |_| {
                                if !prefs_ctx.prefs.read().developer_mode {
                                    let next = build_taps().saturating_add(1);
                                    if next >= 7 {
                                        build_taps.set(0);
                                        prefs_ctx.update(|p| p.developer_mode = true);
                                    } else {
                                        build_taps.set(next);
                                    }
                                }
                            },
                            span { class: "about-row-label", "Build number" }
                            span { class: "about-row-val mono", "{APP_BUILD}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Platform" }
                            span { class: "about-row-val", "{runtime_platform_label()}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Encryption" }
                            span { class: "about-row-val", "Double Ratchet • X25519 • AES-256-GCM" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Privacy model" }
                            span { class: "about-row-val", "Zero-knowledge blind relay" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Languages" }
                            span { class: "about-row-val", "197 languages" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "License" }
                            span { class: "about-row-val", "Open Source (MIT)" }
                        }
                    }
                    if !dev_mode && build_taps() >= 2 && build_taps() < 7 {
                        div { class: "about-tap-hint",
                            "Tap {7 - build_taps()} more times to enable developer mode"
                        }
                    }
                }
            }

            div { class: "settings-col",
                // Developer Mode toggle card
                div { class: "settings-group",
                    div { class: "settings-group-title", "Developer options" }
                    div { class: "about-dev-card",
                        label { class: "about-dev-toggle-row",
                            div { class: "about-dev-head",
                                span { class: "about-dev-icon", BoltIcon { size: 18 } }
                                div { class: "about-dev-text",
                                    span { class: "about-dev-title", "Developer mode" }
                                    span { class: "about-dev-hint", "Show the Developer menu with advanced data export, encryption keys, sync recovery, extensions, and diagnostics." }
                                }
                            }
                            input {
                                r#type: "checkbox",
                                class: "switch",
                                aria_label: "Developer mode",
                                checked: dev_mode,
                                onchange: move |e| {
                                    let on = e.checked();
                                    build_taps.set(0);
                                    prefs_ctx.update(|p| p.developer_mode = on);
                                },
                            }
                        }
                        if dev_mode {
                            div { class: "about-dev-footer",
                                span { class: "settings-value tag-active",
                                    span { class: "cw-live-dot" }
                                    "Enabled"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| on_open_developer.call(()),
                                    "Open Developer"
                                }
                            }
                        }
                    }
                }

                // Helpful resources & diagnostics card
                div { class: "settings-group",
                    div { class: "settings-group-title", "Support & diagnostics" }
                    div { class: "about-card",
                        div { class: "about-row",
                            span { class: "about-row-label", "User guide & shortcuts" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_open_guide.call(()),
                                BookIcon { size: 15 }
                                span { "Open guide" }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Interactive tour" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_replay_tour.call(()),
                                SparkleIcon { size: 15 }
                                span { "Start tour" }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "System summary" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: {
                                    let d = diag_text.clone();
                                    move |_| {
                                        platform::copy_to_clipboard(&d);
                                        copied_diag.set(true);
                                    }
                                },
                                CopyIcon { size: 15 }
                                span { if copied_diag() { "Copied" } else { "Copy diagnostics" } }
                            }
                        }
                    }
                }

                // Legal, Privacy & Community standards card
                div { class: "settings-group",
                    div { class: "settings-group-title", "Legal & policies" }
                    div { class: "about-card",
                        div { class: "about-row",
                            span { class: "about-row-label", "Điều khoản dịch vụ" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_open_terms.call(()),
                                FileTextIcon { size: 15 }
                                span { "Xem điều khoản" }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Chính sách bảo mật" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_open_privacy.call(()),
                                ShieldIcon { size: 15 }
                                span { "Xem chính sách" }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Tiêu chuẩn cộng đồng" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_open_community.call(()),
                                LockIcon { size: 15 }
                                span { "Xem tiêu chuẩn" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DeveloperDiagnosticsPanel(
    current_user_id: String,
    has_keys: bool,
    on_reset_appearance: EventHandler<()>,
    on_disable_dev: EventHandler<()>,
) -> Element {
    let prefs_ctx = use_context::<PrefsCtx>();
    let prefs = prefs_ctx.prefs.read().clone();
    let appearance = platform::load_appearance();
    let mut copied = use_signal(|| false);

    let active_accent = if prefs.custom_accent.is_empty() {
        appearance.accent.clone()
    } else {
        prefs.custom_accent.clone()
    };
    let active_wp = if prefs.app_wallpaper.is_empty() || prefs.app_wallpaper == "none" {
        "None".to_string()
    } else {
        prefs.app_wallpaper.clone()
    };
    let diag_text = build_diagnostics_text(
        &prefs.language,
        &appearance.theme,
        &active_accent,
        prefs.panel_opacity,
        has_keys,
    );

    rsx! {
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Runtime diagnostics" }
                    div { class: "about-card",
                        div { class: "about-row",
                            span { class: "about-row-label", "Account ID" }
                            span { class: "about-row-val mono", "{current_user_id}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Encryption status" }
                            span { class: "about-row-val",
                                ShieldIcon { size: 14 }
                                if has_keys { "Active & secure" } else { "No key yet" }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Language" }
                            span { class: "about-row-val mono", "{prefs.language}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Theme" }
                            span { class: "about-row-val mono", "{appearance.theme}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Accent color" }
                            span { class: "about-row-val mono", "{active_accent}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "App wallpaper" }
                            span { class: "about-row-val mono", "{active_wp}" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Panel transparency" }
                            span { class: "about-row-val mono", "{100u8.saturating_sub(prefs.panel_opacity)}%" }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Version" }
                            span { class: "about-row-val mono", "v{APP_VERSION} ({APP_BUILD})" }
                        }
                    }
                }
            }

            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Maintenance" }
                    div { class: "about-card",
                        div { class: "about-row",
                            span { class: "about-row-label", "Diagnostic report" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: {
                                    let d = diag_text.clone();
                                    move |_| {
                                        platform::copy_to_clipboard(&d);
                                        copied.set(true);
                                    }
                                },
                                CopyIcon { size: 15 }
                                span { if copied() { "Copied" } else { "Copy diagnostics" } }
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Appearance defaults" }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| on_reset_appearance.call(()),
                                "Reset default appearance"
                            }
                        }
                        div { class: "about-row",
                            span { class: "about-row-label", "Developer mode" }
                            button {
                                r#type: "button",
                                class: "btn-danger-outline",
                                onclick: move |_| on_disable_dev.call(()),
                                "Turn off developer mode"
                            }
                        }
                    }
                }
            }
        }
    }
}
