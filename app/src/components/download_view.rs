//! Dedicated multi-platform app download page (`NavSection::Download`).
//! Displays hero actions (top & bottom), scannable QR code, Desktop OS selector
//! (Windows / macOS / Linux), and Mobile direct Android `.apk` & iOS options.

use dioxus::prelude::*;
use qrcode::{QrCode, render::svg};

use crate::{
    components::icons::{
        AndroidIcon, AppStoreLogoIcon, DownloadIcon, LinuxLogoIcon,
        MacLogoIcon, PlayStoreLogoIcon, WindowsLogoIcon,
    },
    platform,
};

const RELEASES_URL: &str = "https://github.com/kiyogo/chart/releases";
const RELEASES_LATEST_URL: &str = "https://github.com/kiyogo/chart/releases/latest";

/// Generates a crisp monochrome SVG QR code string with standard quiet zone for reliable scanning.
pub fn render_qr_svg(uri: &str, dark_fg: &str, light_bg: &str) -> String {
    match QrCode::new(uri.as_bytes()) {
        Ok(code) => code
            .render::<svg::Color<'_>>()
            .min_dimensions(180, 180)
            .quiet_zone(true)
            .dark_color(svg::Color(dark_fg))
            .light_color(svg::Color(light_bg))
            .build(),
        Err(_) => String::new(),
    }
}

#[component]
pub fn DownloadView(
    #[props(default = "all".to_string())] initial_target: String,
    on_back: EventHandler<()>,
) -> Element {
    let mut tab = use_signal(move || {
        if initial_target == "desktop" || initial_target == "mobile" {
            initial_target.clone()
        } else {
            "all".to_string()
        }
    });
    let mut selected_desktop = use_signal(|| "windows".to_string());
    let mut selected_mobile = use_signal(|| "android".to_string());
    let mut detected_os = use_signal(|| "linux".to_string());
    let mut download_url = use_signal(|| format!("{RELEASES_URL}"));

    use_effect(move || {
        spawn(async move {
            let origin = platform::app_origin().await;
            if !origin.trim().is_empty() {
                download_url.set(format!("{origin}/?page=download"));
            }
            let mut ev = dioxus::document::eval(
                r#"
                const ua = (navigator.userAgent || '').toLowerCase();
                let os = 'windows';
                if (ua.includes('android')) os = 'android';
                else if (ua.includes('iphone') || ua.includes('ipad')) os = 'ios';
                else if (ua.includes('mac os') || ua.includes('macintosh')) os = 'macos';
                else if (ua.includes('linux')) os = 'linux';
                dioxus.send(os);
                "#,
            );
            if let Ok(os) = ev.recv::<String>().await {
                detected_os.set(os.clone());
                if os == "windows" || os == "macos" || os == "linux" {
                    selected_desktop.set(os);
                } else if os == "android" || os == "ios" {
                    selected_mobile.set(os);
                }
            }
        });
    });

    let qr_svg = render_qr_svg(&download_url(), "#0F172A", "#FFFFFF");
    let show_desktop = tab() == "all" || tab() == "desktop";
    let show_mobile = tab() == "all" || tab() == "mobile";

    rsx! {
        div { class: "download-page",
            // Top bar with right-aligned quick platform buttons (no top-left title/back arrow)
            header { class: "download-topbar",
                div { class: "download-topbar-actions",
                    button {
                        r#type: "button",
                        class: if tab() == "desktop" { "btn-secondary active" } else { "btn-secondary" },
                        onclick: move |_| tab.set("desktop".to_string()),
                        "Desktop"
                    }
                    button {
                        r#type: "button",
                        class: if tab() == "mobile" { "btn-secondary active" } else { "btn-secondary" },
                        onclick: move |_| tab.set("mobile".to_string()),
                        "Mobile"
                    }
                    a {
                        class: "btn-primary download-ext-link",
                        href: "{RELEASES_LATEST_URL}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        DownloadIcon { size: 16 }
                        span { "Latest release" }
                    }
                }
            }

            div { class: "download-scroll",
                div { class: "download-inner",
                    // Hero block: large title, intro, bottom hero actions + QR code
                    section { class: "download-hero",
                        div { class: "download-hero-copy",
                            span { class: "download-eyebrow", "MULTI-PLATFORM · END-END ENCRYPTION" }
                            h1 { class: "download-hero-title",
                                "Download ChatW"
                                br {}
                                "for all your devices"
                            }
                            p { class: "download-hero-desc",
                                "Private messaging, online group meetings like Google Meet, and lag-free screen sharing on Windows, macOS, Linux, Android APK, and iOS."
                            }
                            div { class: "download-hero-actions",
                                button {
                                    r#type: "button",
                                    class: "btn-primary download-hero-btn",
                                    onclick: move |_| tab.set("desktop".to_string()),
                                    DownloadIcon { size: 17 }
                                    span { "Download for desktop" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary download-hero-btn",
                                    onclick: move |_| tab.set("mobile".to_string()),
                                    AndroidIcon { size: 17 }
                                    span { "Download APK & Mobile" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary download-hero-btn",
                                    onclick: move |_| on_back.call(()),
                                    "Use on Web"
                                }
                            }
                        }

                        div { class: "download-qr-panel cw-spotlight",
                            div { class: "download-qr-box",
                                div { class: "cw-qr-svg", dangerous_inner_html: "{qr_svg}" }
                                div { class: "cw-qr-scan", "aria-hidden": "true" }
                            }
                            div { class: "download-qr-caption",
                                strong { "Scan the code to open on your phone" }
                                span { "Supports quick download of APK for Android and iOS shortcuts" }
                            }
                        }
                    }

                    // Platform tabs
                    div { class: "download-filter-tabs", role: "tablist", aria_label: "Choose platform",
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "all" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("all".to_string()),
                            "All devices"
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "desktop" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("desktop".to_string()),
                            "Computer · Windows, macOS, Linux"
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "mobile" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("mobile".to_string()),
                            "Phone · Android APK, iOS"
                        }
                    }

                    if show_desktop {
                        section { class: "download-section cw-stagger",
                            div { class: "download-section-head",
                                h2 { "Computer Version" }
                                span { "Select your operating system to see the appropriate installation package" }
                            }
                            div { class: "download-os-grid",
                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "windows" { "download-os-card cw-spotlight selected" } else { "download-os-card cw-spotlight" },
                                    onclick: move |_| selected_desktop.set("windows".to_string()),
                                    div { class: "download-os-icon cw-brand-float", WindowsLogoIcon { size: 30 } }
                                    div { class: "download-os-meta",
                                        strong {
                                            "Windows"
                                            if detected_os() == "windows" {
                                                span { class: "cw-os-rec-pill", span { class: "cw-live-dot" } "Your machine" }
                                            }
                                        }
                                        span { "Windows 10 / 11 64-bit · .exe installer" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Download .exe"
                                    }
                                }

                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "macos" { "download-os-card cw-spotlight selected" } else { "download-os-card cw-spotlight" },
                                    onclick: move |_| selected_desktop.set("macos".to_string()),
                                    div { class: "download-os-icon cw-brand-float", MacLogoIcon { size: 30 } }
                                    div { class: "download-os-meta",
                                        strong {
                                            "macOS"
                                            if detected_os() == "macos" {
                                                span { class: "cw-os-rec-pill", span { class: "cw-live-dot" } "Your machine" }
                                            }
                                        }
                                        span { "Apple Silicon M1–M4 & Intel · .dmg package" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Download .dmg"
                                    }
                                }

                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "linux" { "download-os-card cw-spotlight selected" } else { "download-os-card cw-spotlight" },
                                    onclick: move |_| selected_desktop.set("linux".to_string()),
                                    div { class: "download-os-icon cw-brand-float", LinuxLogoIcon { size: 30 } }
                                    div { class: "download-os-meta",
                                        strong {
                                            "Linux"
                                            if detected_os() == "linux" {
                                                span { class: "cw-os-rec-pill", span { class: "cw-live-dot" } "Your machine" }
                                            }
                                        }
                                        span { "Ubuntu / Debian .deb & AppImage amd64" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Download .deb"
                                    }
                                }
                            }
                        }
                    }

                    if show_mobile {
                        section { class: "download-section cw-stagger",
                            div { class: "download-section-head",
                                h2 { "Phone version" }
                                span { "Supports direct download of .apk files for Android or installation via app store" }
                            }
                            div { class: "download-os-grid two-cols",
                                div {
                                    class: if selected_mobile() == "android" { "download-os-card cw-spotlight selected" } else { "download-os-card cw-spotlight" },
                                    onclick: move |_| selected_mobile.set("android".to_string()),
                                    div { class: "download-os-icon cw-brand-float", AndroidIcon { size: 30 } }
                                    div { class: "download-os-meta",
                                        strong {
                                            "Android · APK & CH Play"
                                            if detected_os() == "android" {
                                                span { class: "cw-os-rec-pill", span { class: "cw-live-dot" } "Your machine" }
                                            }
                                        }
                                        span { "Android 8.0 or higher · Directly install the APK file without intermediaries" }
                                    }
                                    div { class: "download-mobile-actions",
                                        a {
                                            class: "btn-primary download-card-cta",
                                            href: "{RELEASES_LATEST_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            "Download APK"
                                        }
                                        a {
                                            class: "download-store-icon-btn",
                                            href: "{RELEASES_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            aria_label: "Open on Google Play",
                                            title: "Google Play",
                                            PlayStoreLogoIcon { size: 20 }
                                        }
                                    }
                                }

                                div {
                                    class: if selected_mobile() == "ios" { "download-os-card cw-spotlight selected" } else { "download-os-card cw-spotlight" },
                                    onclick: move |_| selected_mobile.set("ios".to_string()),
                                    div { class: "download-os-icon cw-brand-float", AppStoreLogoIcon { size: 30 } }
                                    div { class: "download-os-meta",
                                        strong {
                                            "iOS · iPhone & iPad"
                                            if detected_os() == "ios" {
                                                span { class: "cw-os-rec-pill", span { class: "cw-live-dot" } "Your machine" }
                                            }
                                        }
                                        span { "iOS 15.0 or later · Download via App Store or add directly from Safari" }
                                    }
                                    div { class: "download-mobile-actions",
                                        a {
                                            class: "download-store-icon-btn",
                                            href: "{RELEASES_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            aria_label: "Open on App Store",
                                            title: "App Store",
                                            AppStoreLogoIcon { size: 20 }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary download-card-cta",
                                            onclick: move |_| on_back.call(()),
                                            "Open iOS Web"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
