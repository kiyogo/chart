//! Dedicated multi-platform app download page (`NavSection::Download`).
//! Displays hero actions (top & bottom), scannable QR code, Desktop OS selector
//! (Windows / macOS / Linux), and Mobile direct Android `.apk` & iOS options.

use dioxus::prelude::*;
use qrcode::{QrCode, render::svg};

use crate::{
    components::icons::{
        AndroidIcon, AppStoreLogoIcon, BackIcon, DownloadIcon, LinuxLogoIcon, LogoIcon,
        MacLogoIcon, PlayStoreLogoIcon, WindowsLogoIcon,
    },
    platform,
};

const RELEASES_URL: &str = "https://github.com/kiyogo/chart/releases";
const RELEASES_LATEST_URL: &str = "https://github.com/kiyogo/chart/releases/latest";

/// Generates a crisp monochrome SVG QR code string for any URL.
pub fn render_qr_svg(uri: &str, dark_fg: &str, light_bg: &str) -> String {
    match QrCode::new(uri.as_bytes()) {
        Ok(code) => code
            .render::<svg::Color<'_>>()
            .min_dimensions(156, 156)
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
    let mut download_url = use_signal(|| format!("{RELEASES_URL}"));

    use_effect(move || {
        spawn(async move {
            let origin = platform::app_origin().await;
            if !origin.trim().is_empty() {
                download_url.set(format!("{origin}/?page=download"));
            }
        });
    });

    let qr_svg = render_qr_svg(&download_url(), "#0F172A", "#FFFFFF");
    let show_desktop = tab() == "all" || tab() == "desktop";
    let show_mobile = tab() == "all" || tab() == "mobile";

    rsx! {
        div { class: "download-page",
            // Top bar with Back + quick platform buttons ("nút ở trên")
            header { class: "download-topbar",
                div { class: "download-brand-group",
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        aria_label: "Quay lại tin nhắn",
                        onclick: move |_| on_back.call(()),
                        BackIcon {}
                    }
                    span { class: "download-brand-badge",
                        LogoIcon { size: 22 }
                        span { "chatw" }
                    }
                }
                div { class: "download-topbar-actions",
                    button {
                        r#type: "button",
                        class: if tab() == "desktop" { "btn-secondary active" } else { "btn-secondary" },
                        onclick: move |_| tab.set("desktop".to_string()),
                        "Máy tính"
                    }
                    button {
                        r#type: "button",
                        class: if tab() == "mobile" { "btn-secondary active" } else { "btn-secondary" },
                        onclick: move |_| tab.set("mobile".to_string()),
                        "Điện thoại"
                    }
                    a {
                        class: "btn-primary download-ext-link",
                        href: "{RELEASES_LATEST_URL}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        DownloadIcon { size: 16 }
                        span { "Bản phát hành mới nhất" }
                    }
                }
            }

            div { class: "download-scroll",
                div { class: "download-inner",
                    // Hero block: large title, intro, bottom hero actions + QR code
                    section { class: "download-hero",
                        div { class: "download-hero-copy",
                            span { class: "download-eyebrow", "ĐA NỀN TẢNG · MÃ HÓA ĐẦU CUỐI" }
                            h1 { class: "download-hero-title",
                                "Tải ứng dụng chatw"
                                br {}
                                "cho mọi thiết bị của bạn"
                            }
                            p { class: "download-hero-desc",
                                "Nhắn tin riêng tư, họp nhóm trực tuyến như Google Meet và chia sẻ màn hình sắc nét không độ trễ trên Windows, macOS, Linux, Android APK và iOS."
                            }
                            div { class: "download-hero-actions",
                                button {
                                    r#type: "button",
                                    class: "btn-primary download-hero-btn",
                                    onclick: move |_| tab.set("desktop".to_string()),
                                    DownloadIcon { size: 17 }
                                    span { "Tải cho máy tính" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary download-hero-btn",
                                    onclick: move |_| tab.set("mobile".to_string()),
                                    AndroidIcon { size: 17 }
                                    span { "Tải APK & Di động" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary download-hero-btn",
                                    onclick: move |_| on_back.call(()),
                                    "Dùng ngay trên Web"
                                }
                            }
                        }

                        div { class: "download-qr-panel",
                            div { class: "download-qr-box", dangerous_inner_html: "{qr_svg}" }
                            div { class: "download-qr-caption",
                                strong { "Quét mã để mở trên điện thoại" }
                                span { "Hỗ trợ tải nhanh APK cho Android và lối tắt iOS" }
                            }
                        }
                    }

                    // Platform tabs
                    div { class: "download-filter-tabs", role: "tablist", aria_label: "Chọn nền tảng",
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "all" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("all".to_string()),
                            "Tất cả thiết bị"
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "desktop" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("desktop".to_string()),
                            "Máy tính (Windows, macOS, Linux)"
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            class: if tab() == "mobile" { "download-tab active" } else { "download-tab" },
                            onclick: move |_| tab.set("mobile".to_string()),
                            "Điện thoại (Android APK, iOS)"
                        }
                    }

                    if show_desktop {
                        section { class: "download-section",
                            div { class: "download-section-head",
                                h2 { "Phiên bản Máy tính" }
                                span { "Chọn hệ điều hành của bạn để xem gói cài đặt phù hợp" }
                            }
                            div { class: "download-os-grid",
                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "windows" { "download-os-card selected" } else { "download-os-card" },
                                    onclick: move |_| selected_desktop.set("windows".to_string()),
                                    div { class: "download-os-icon", WindowsLogoIcon { size: 28 } }
                                    div { class: "download-os-meta",
                                        strong { "Windows" }
                                        span { "Windows 10 / 11 (64-bit) · Trình cài đặt .exe" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Tải .exe"
                                    }
                                }

                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "macos" { "download-os-card selected" } else { "download-os-card" },
                                    onclick: move |_| selected_desktop.set("macos".to_string()),
                                    div { class: "download-os-icon", MacLogoIcon { size: 28 } }
                                    div { class: "download-os-meta",
                                        strong { "macOS" }
                                        span { "Apple Silicon (M1–M4) & Intel · Gói .dmg" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Tải .dmg"
                                    }
                                }

                                button {
                                    r#type: "button",
                                    class: if selected_desktop() == "linux" { "download-os-card selected" } else { "download-os-card" },
                                    onclick: move |_| selected_desktop.set("linux".to_string()),
                                    div { class: "download-os-icon", LinuxLogoIcon { size: 28 } }
                                    div { class: "download-os-meta",
                                        strong { "Linux" }
                                        span { "Ubuntu / Debian (.deb) & AppImage (amd64)" }
                                    }
                                    a {
                                        class: "btn-primary download-card-cta",
                                        href: "{RELEASES_LATEST_URL}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        onclick: move |e| e.stop_propagation(),
                                        "Tải .deb"
                                    }
                                }
                            }
                        }
                    }

                    if show_mobile {
                        section { class: "download-section",
                            div { class: "download-section-head",
                                h2 { "Phiên bản Điện thoại" }
                                span { "Hỗ trợ tải trực tiếp tệp .apk cho Android hoặc cài đặt qua cửa hàng ứng dụng" }
                            }
                            div { class: "download-os-grid two-cols",
                                div {
                                    class: if selected_mobile() == "android" { "download-os-card selected" } else { "download-os-card" },
                                    onclick: move |_| selected_mobile.set("android".to_string()),
                                    div { class: "download-os-icon", AndroidIcon { size: 28 } }
                                    div { class: "download-os-meta",
                                        strong { "Android (.apk & CH Play)" }
                                        span { "Android 8.0 trở lên · Cài trực tiếp tệp APK không qua trung gian" }
                                    }
                                    div { class: "download-mobile-actions",
                                        a {
                                            class: "btn-primary download-card-cta",
                                            href: "{RELEASES_LATEST_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            "Tải APK"
                                        }
                                        a {
                                            class: "download-store-icon-btn",
                                            href: "{RELEASES_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            aria_label: "Mở trên Google Play",
                                            title: "Google Play",
                                            PlayStoreLogoIcon { size: 20 }
                                        }
                                    }
                                }

                                div {
                                    class: if selected_mobile() == "ios" { "download-os-card selected" } else { "download-os-card" },
                                    onclick: move |_| selected_mobile.set("ios".to_string()),
                                    div { class: "download-os-icon", AppStoreLogoIcon { size: 28 } }
                                    div { class: "download-os-meta",
                                        strong { "iOS (iPhone & iPad)" }
                                        span { "iOS 15.0 trở lên · Tải qua App Store hoặc thêm trực tiếp từ Safari" }
                                    }
                                    div { class: "download-mobile-actions",
                                        a {
                                            class: "download-store-icon-btn",
                                            href: "{RELEASES_URL}",
                                            target: "_blank",
                                            rel: "noopener noreferrer",
                                            aria_label: "Mở trên App Store",
                                            title: "App Store",
                                            AppStoreLogoIcon { size: 20 }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary download-card-cta",
                                            onclick: move |_| on_back.call(()),
                                            "Mở Web iOS"
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
