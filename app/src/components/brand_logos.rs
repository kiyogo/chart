//! Authentic multicolor pure-SVG brand logos for Cloud providers, Operating Systems,
//! Web Browsers, Auth & Security badges, and Language flags (Spec 049).
//! Zero external CDN requests — instant vector rendering on Web, Desktop & Mobile.

use dioxus::prelude::*;

/// Renders the brand SVG logo for a cloud backup provider ID.
#[component]
pub fn CloudProviderLogo(provider_id: String, #[props(default = 32)] size: u32) -> Element {
    let pid = provider_id.to_lowercase();
    rsx! {
        span {
            class: "brand-badge cw-brand-float",
            "data-brand": "{pid}",
            "aria-hidden": "true",
            match pid.as_str() {
                "google_drive" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        // Official-style Google Drive folded triangle
                        path { d: "M17.2 7.5h13.6l13.2 22.8H30.4L17.2 7.5z", fill: "#FBBC04" }
                        path { d: "M4 30.3L17.2 7.5l6.8 11.8-13.2 22.8L4 30.3z", fill: "#0F9D58" }
                        path { d: "M10.8 42.1l6.8-11.8H44l-6.8 11.8H10.8z", fill: "#4285F4" }
                        path { d: "M30.4 30.3H44l-6.8 11.8-6.8-11.8z", fill: "#EA4335", opacity: "0.9" }
                        path { d: "M10.8 42.1l6.8-11.8-6.8-11.8L4 30.3l6.8 11.8z", fill: "#00832D", opacity: "0.28" }
                    }
                },
                "onedrive" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M18.8 15.5c4.6 0 8.5 2.7 10.3 6.6.8-.2 1.6-.3 2.5-.3 5.2 0 9.4 4.1 9.4 9.2 0 4.9-3.9 9-8.9 9H14.2C8.6 40 4 35.6 4 30.1c0-4.9 3.6-9 8.4-9.8 1.4-2.8 3.8-4.8 6.4-4.8z", fill: "#0364B8" }
                        path { d: "M29.1 22.1c.8-.2 1.6-.3 2.5-.3 5.2 0 9.4 4.1 9.4 9.2 0 4.9-3.9 9-8.9 9H22.5l6.6-17.9z", fill: "#0078D4" }
                        path { d: "M12.4 20.3c1.8-.3 3.7.1 5.3 1l14.4 8.7H41c-.2 4.8-4.1 10-8.9 10H14.2C8.6 40 4 35.6 4 30.1c0-4.9 3.6-9 8.4-9.8z", fill: "#28A8EA" }
                        path { d: "M14.2 40h17.9c3.4 0 6.4-1.9 7.9-4.7L24 27.6 10.6 38.9c1.1.7 2.3 1.1 3.6 1.1z", fill: "#50D9FF", opacity: "0.85" }
                    }
                },
                "dropbox" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M14.5 8L4 14.8l10.5 6.8L24 14.8 14.5 8z", fill: "#0061FF" }
                        path { d: "M33.5 8L24 14.8l9.5 6.8L44 14.8 33.5 8z", fill: "#0061FF" }
                        path { d: "M4 28.4l10.5 6.8L24 28.4l-9.5-6.8L4 28.4z", fill: "#0061FF" }
                        path { d: "M33.5 21.6L24 28.4l9.5 6.8L44 28.4l-10.5-6.8z", fill: "#0061FF" }
                        path { d: "M24 30.8l-9.5 6.8 9.5 6.4 9.5-6.4L24 30.8z", fill: "#2B7FFF" }
                    }
                },
                "nextcloud" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "3", y: "3", width: "42", height: "42", rx: "12", fill: "#0082C9" }
                        circle { cx: "24", cy: "24", r: "7.5", stroke: "#FFFFFF", stroke_width: "3.4" }
                        circle { cx: "11.5", cy: "24", r: "4.6", stroke: "#FFFFFF", stroke_width: "3.2" }
                        circle { cx: "36.5", cy: "24", r: "4.6", stroke: "#FFFFFF", stroke_width: "3.2" }
                    }
                },
                "webdav" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "4", y: "4", width: "40", height: "40", rx: "11", fill: "#0D9488" }
                        circle { cx: "24", cy: "24", r: "11.5", stroke: "#FFFFFF", stroke_width: "2.4" }
                        ellipse { cx: "24", cy: "24", rx: "5.5", ry: "11.5", stroke: "#FFFFFF", stroke_width: "2.1" }
                        path { d: "M12.8 24h22.4M15 17.5h18M15 30.5h18", stroke: "#FFFFFF", stroke_width: "2.1", stroke_linecap: "round" }
                    }
                },
                "s3" | "amazon_s3" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M10 14l4 24c.4 2.3 4.8 4 10 4s9.6-1.7 10-4l4-24H10z", fill: "#E25444" }
                        path { d: "M24 14v28c5.2 0 9.6-1.7 10-4l4-24H24z", fill: "#C53A2B" }
                        ellipse { cx: "24", cy: "14", rx: "14", ry: "4.8", fill: "#FF8475" }
                        ellipse { cx: "24", cy: "14", rx: "10.5", ry: "2.8", fill: "#8C2218", opacity: "0.45" }
                        path { d: "M11 16c-3.2 2-3.2 9 1 11M37 16c3.2 2 3.2 9-1 11", stroke: "#F97316", stroke_width: "2.6", stroke_linecap: "round" }
                    }
                },
                "r2" | "cloudflare_r2" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M33.2 35H13.5c-4.7 0-8.5-3.6-8.5-8.1 0-4.1 3.1-7.5 7.2-8 1.6-4.5 6-7.7 11.1-7.7 6.1 0 11.1 4.5 11.8 10.3 4.1.6 7.4 4 7.4 8.2 0 2.9-2.4 5.3-5.3 5.3h-4z", fill: "#F38020" }
                        path { d: "M35.1 21.5c-.6-2.9-2.4-5.4-4.8-7 1.3 2.5 1.8 5.4 1.3 8.4l-18.8 12.1h24.4c2.9 0 5.3-2.4 5.3-5.3 0-4.2-3.3-7.6-7.4-8.2z", fill: "#FBAE40" }
                    }
                },
                "minio" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "4", y: "4", width: "40", height: "40", rx: "11", fill: "#C72E49" }
                        path { d: "M13 33V16l11 9.5L35 16v17", stroke: "#FFFFFF", stroke_width: "3.8", stroke_linecap: "round", stroke_linejoin: "round" }
                        circle { cx: "24", cy: "13", r: "2.5", fill: "#FFD166" }
                    }
                },
                "nas" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "6", y: "7", width: "36", height: "34", rx: "8", fill: "#334155" }
                        rect { x: "11", y: "13", width: "11", height: "22", rx: "3", fill: "#475569" }
                        rect { x: "26", y: "13", width: "11", height: "22", rx: "3", fill: "#475569" }
                        circle { cx: "16.5", cy: "18", r: "2", fill: "#22C55E" }
                        circle { cx: "31.5", cy: "18", r: "2", fill: "#22C55E" }
                        path { d: "M14 29h5M29 29h5", stroke: "#94A3B8", stroke_width: "2.2", stroke_linecap: "round" }
                    }
                },
                "icloud" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        defs {
                            linearGradient { id: "icloud-g", x1: "0", y1: "0", x2: "1", y2: "1",
                                stop { offset: "0%", stop_color: "#60C5FF" }
                                stop { offset: "100%", stop_color: "#0A84FF" }
                            }
                        }
                        path { d: "M15.4 37.5h18.4c5.1 0 9.2-4 9.2-9 0-4.6-3.5-8.4-8.1-8.9C33.4 13.7 28.1 9.5 21.8 9.5c-6.8 0-12.4 5-13.2 11.6C5.8 22.1 4 24.8 4 28c0 5.3 4.3 9.5 9.6 9.5h1.8z", fill: "url(#icloud-g)" }
                    }
                },
                _ => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M15 36h19a8 8 0 0 0 1.4-15.9A11 11 0 0 0 14 17.5a9.5 9.5 0 0 0 1 18.5z", fill: "var(--accent)", opacity: "0.9" }
                    }
                },
            }
        }
    }
}

/// Iconic colorful Linux Tux penguin pure-SVG logo.
#[component]
pub fn LinuxTuxLogo(#[props(default = 24)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 48 48",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            // Tux body
            path { d: "M24 5c-6.2 0-9.6 4.4-9.8 10.6-.2 4.6-2.4 8.3-4.6 13-1.4 3-.4 6.6 2.5 8.1 2.2 1.2 4.6 3.3 11.9 3.3s9.7-2.1 11.9-3.3c2.9-1.5 3.9-5.1 2.5-8.1-2.2-4.7-4.4-8.4-4.6-13C33.6 9.4 30.2 5 24 5z", fill: "#1E293B" }
            // White belly
            ellipse { cx: "24", cy: "29", rx: "9.2", ry: "10.2", fill: "#F8FAFC" }
            // Eyes
            ellipse { cx: "20.2", cy: "13.5", rx: "2.3", ry: "2.8", fill: "#FFFFFF" }
            ellipse { cx: "27.8", cy: "13.5", rx: "2.3", ry: "2.8", fill: "#FFFFFF" }
            circle { cx: "20.7", cy: "13.7", r: "1.1", fill: "#0F172A" }
            circle { cx: "27.3", cy: "13.7", r: "1.1", fill: "#0F172A" }
            // Orange beak
            path { d: "M19.8 17.8c2.4-1.5 6-1.5 8.4 0 1 .6.8 2-.2 2.6-2.3 1.4-5.7 1.4-8 0-1-.6-1.2-2-.2-2.6z", fill: "#F59E0B" }
            // Orange webbed feet
            path { d: "M11.5 39.2c-1.4-1.2-.8-3.6 1.2-4.2l5.8-1.4c1.8-.4 3.4 1.1 3 2.9-.5 2.4-2.8 4.4-6.3 4.2-1.5-.1-2.8-.7-3.7-1.5z", fill: "#F59E0B" }
            path { d: "M36.5 39.2c1.4-1.2.8-3.6-1.2-4.2l-5.8-1.4c-1.8-.4-3.4 1.1-3 2.9.5 2.4 2.8 4.4 6.3 4.2 1.5-.1 2.8-.7 3.7-1.5z", fill: "#F59E0B" }
        }
    }
}

/// Renders an OS logo auto-detected from any device label, platform, or user-agent string.
#[component]
pub fn OsBrandLogo(text: String, #[props(default = 20)] size: u32) -> Element {
    let lower = text.to_lowercase();
    let kind = if lower.contains("windows") || lower.contains("win32") || lower.contains("win64") {
        "windows"
    } else if lower.contains("iphone") || lower.contains("ipad") || lower.contains("ios") {
        "ios"
    } else if lower.contains("mac") || lower.contains("darwin") || lower.contains("apple") {
        "macos"
    } else if lower.contains("android") {
        "android"
    } else if lower.contains("linux") || lower.contains("ubuntu") || lower.contains("debian") || lower.contains("fedora") || lower.contains("arch") {
        "linux"
    } else {
        "web"
    };
    rsx! {
        span { class: "os-brand-logo", "data-os": "{kind}", "aria-hidden": "true",
            match kind {
                "windows" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "2.5", y: "2.5", width: "8.8", height: "8.8", rx: "1.4", fill: "#0078D4" }
                        rect { x: "12.7", y: "2.5", width: "8.8", height: "8.8", rx: "1.4", fill: "#28A8EA" }
                        rect { x: "2.5", y: "12.7", width: "8.8", height: "8.8", rx: "1.4", fill: "#0078D4" }
                        rect { x: "12.7", y: "12.7", width: "8.8", height: "8.8", rx: "1.4", fill: "#50D9FF" }
                    }
                },
                "macos" | "ios" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "currentColor", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M16.2 12.7c0-2.1 1.7-3.1 1.8-3.2-1-1.4-2.5-1.6-3-1.6-1.3-.1-2.5.8-3.1.8s-1.6-.8-2.7-.7c-1.4 0-2.7.8-3.4 2.1-1.5 2.5-.4 6.3 1 8.4.7 1 1.5 2.1 2.6 2.1 1-.1 1.4-.7 2.7-.7s1.6.7 2.7.6c1.1 0 1.8-1 2.5-2 .8-1.2 1.1-2.3 1.1-2.4-.1 0-2.2-.8-2.2-3.4zM14.1 6.4c.6-.7 1-1.7.9-2.7-.8 0-1.9.6-2.5 1.3-.5.6-1 1.6-.9 2.6.9.1 1.9-.5 2.5-1.2z" }
                    }
                },
                "android" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M5 14a7 7 0 0 1 14 0v2.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 16.5V14z", fill: "#3DDC84" }
                        path { d: "M8.2 8.2L6.6 5.4M15.8 8.2l1.6-2.8", stroke: "#3DDC84", stroke_width: "1.8", stroke_linecap: "round" }
                        circle { cx: "9.4", cy: "12.8", r: "1", fill: "#FFFFFF" }
                        circle { cx: "14.6", cy: "12.8", r: "1", fill: "#FFFFFF" }
                    }
                },
                "linux" => rsx! { LinuxTuxLogo { size } },
                _ => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        circle { cx: "12", cy: "12", r: "9.5", fill: "#3B82F6", opacity: "0.18" }
                        circle { cx: "12", cy: "12", r: "9.5", stroke: "#2563EB", stroke_width: "1.8" }
                        ellipse { cx: "12", cy: "12", rx: "4.5", ry: "9.5", stroke: "#2563EB", stroke_width: "1.6" }
                        path { d: "M3 12h18", stroke: "#2563EB", stroke_width: "1.6" }
                    }
                },
            }
        }
    }
}

/// Renders a web browser or native desktop client brand logo parsed from device text.
#[component]
pub fn BrowserBrandLogo(text: String, #[props(default = 24)] size: u32) -> Element {
    let lower = text.to_lowercase();
    let kind = if lower.contains("brave") {
        "brave"
    } else if lower.contains("edge") || lower.contains("edg/") {
        "edge"
    } else if lower.contains("firefox") || lower.contains("fxios") {
        "firefox"
    } else if lower.contains("opera") || lower.contains("opr/") {
        "opera"
    } else if lower.contains("chrome") || lower.contains("chromium") || lower.contains("crios") {
        "chrome"
    } else if lower.contains("safari") {
        "safari"
    } else if lower.contains("chatw") || lower.contains("desktop") {
        "desktop"
    } else {
        "web"
    };

    rsx! {
        span { class: "browser-brand-logo", "data-browser": "{kind}", "aria-hidden": "true",
            match kind {
                "chrome" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        // Red top arc
                        path { d: "M24 4C15.5 4 8.3 9.3 5.4 16.8l9.8 17 8.8-15.2H42.6C40.2 10.1 32.8 4 24 4z", fill: "#EA4335" }
                        // Yellow right arc
                        path { d: "M24 18.6l8.8 15.2L24 44c11 0 20-9 20-20 0-1.9-.3-3.7-.8-5.4H24z", fill: "#FBBC04" }
                        // Green bottom-left arc
                        path { d: "M5.4 16.8C4.5 19 4 21.4 4 24c0 11 9 20 20 20l8.8-10.2-17.6 0-9.8-17z", fill: "#34A853" }
                        // White halo + Blue core
                        circle { cx: "24", cy: "24", r: "9.2", fill: "#FFFFFF" }
                        circle { cx: "24", cy: "24", r: "7.2", fill: "#1A73E8" }
                    }
                },
                "firefox" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        circle { cx: "24", cy: "24", r: "19", fill: "#4F46E5" }
                        path { d: "M39 14c-2-4-6-7-11-8 3 3 3.5 7 1 10-2-2-6-3-9-2-5 1.5-8 6.5-7 12 1 6 6.5 10.5 13 10.5 8 0 15-6.5 15-15 0-2.7-.7-5.3-2-7.5z", fill: "#F97316" }
                        path { d: "M35 16c-1.5 3.5-5 5.5-9 5-4-.5-7 2-7.5 5.5-.5 4 2.5 7.5 6.8 7.5 5.8 0 10.7-4.8 10.7-11 0-2.5-.4-4.9-1-7z", fill: "#FACC15" }
                    }
                },
                "safari" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        circle { cx: "24", cy: "24", r: "20", fill: "#0EA5E9" }
                        circle { cx: "24", cy: "24", r: "17", stroke: "#FFFFFF", stroke_width: "1.6", opacity: "0.7" }
                        path { d: "M33 15l-5.5 12.5L20.5 20.5 33 15z", fill: "#EF4444" }
                        path { d: "M15 33l5.5-12.5 7 7L15 33z", fill: "#FFFFFF" }
                        circle { cx: "24", cy: "24", r: "2.2", fill: "#1E293B" }
                    }
                },
                "edge" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        circle { cx: "24", cy: "24", r: "19", fill: "#0078D4" }
                        path { d: "M8 26c1-10 9-17 19-16 8 .8 14 7 15 14-3-4-8-6-13-5-6 1-10 6-9 12 1 5 5 8 10 9-11 3-23-3-22-14z", fill: "#38BDF8" }
                        path { d: "M12 31c2 6 8 11 16 11 6 0 11-3 14-7-4 2-9 2-13 0-5-2-7-7-5-12-7 1-11 4-12 8z", fill: "#22D3EE" }
                    }
                },
                "brave" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        path { d: "M10 10l5-3h18l5 3 2 10-4 15-12 8-12-8-4-15 2-10z", fill: "#FB542B" }
                        path { d: "M16 18l8-4 8 4-2 10-6 7-6-7-2-10z", fill: "#FFFFFF" }
                        path { d: "M21 25l3 4 3-4h-6z", fill: "#FB542B" }
                    }
                },
                "opera" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        circle { cx: "24", cy: "24", r: "19", fill: "#FF1B2D" }
                        ellipse { cx: "24", cy: "24", rx: "7.5", ry: "13", fill: "#FFFFFF" }
                    }
                },
                "desktop" => rsx! {
                    svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { x: "4", y: "4", width: "40", height: "40", rx: "11", fill: "var(--accent)" }
                        path { d: "M13 18l5 13 6-10 6 10 5-13", stroke: "#FFFFFF", stroke_width: "4", stroke_linecap: "round", stroke_linejoin: "round" }
                    }
                },
                _ => rsx! { OsBrandLogo { text, size } },
            }
        }
    }
}

/// Paired living badge displaying Browser/App logo with an overlapping corner OS mini-logo
/// (e.g. Chrome + Linux Tux, Safari + Apple, Edge + Windows).
#[component]
pub fn DeviceBrandPair(text: String, #[props(default = 32)] size: u32) -> Element {
    let lower = text.to_lowercase();
    let has_browser = lower.contains("chrome")
        || lower.contains("firefox")
        || lower.contains("safari")
        || lower.contains("edge")
        || lower.contains("brave")
        || lower.contains("opera")
        || lower.contains("desktop")
        || lower.contains("chatw");
    let has_os = lower.contains("linux")
        || lower.contains("windows")
        || lower.contains("mac")
        || lower.contains("android")
        || lower.contains("ios")
        || lower.contains("iphone")
        || lower.contains("ubuntu")
        || lower.contains("debian");
    let os_size = (size / 2).max(14);
    rsx! {
        span { class: "device-brand-pair cw-brand-float", "aria-hidden": "true",
            if has_browser {
                BrowserBrandLogo { text: text.clone(), size }
                if has_os {
                    span { class: "device-os-corner",
                        OsBrandLogo { text: text.clone(), size: os_size }
                    }
                }
            } else {
                OsBrandLogo { text: text.clone(), size }
            }
        }
    }
}

/// Official 4-color Google 'G' mark for Google Sign-In rows.
#[component]
pub fn GoogleAuthLogo(#[props(default = 24)] size: u32) -> Element {
    rsx! {
        span { class: "brand-badge cw-brand-float", "data-brand": "google", "aria-hidden": "true",
            svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                path { d: "M43.6 24.5c0-1.5-.1-2.9-.4-4.3H24v8.2h11c-.5 2.5-1.9 4.7-4.1 6.1v5.1h6.6c3.9-3.6 6.1-8.8 6.1-15.1z", fill: "#4285F4" }
                path { d: "M24 44.5c5.5 0 10.2-1.8 13.6-4.9l-6.6-5.1c-1.8 1.2-4.2 2-7 2-5.4 0-9.9-3.6-11.5-8.5H5.7v5.3C9.1 40 16 44.5 24 44.5z", fill: "#34A853" }
                path { d: "M12.5 28c-.4-1.2-.7-2.6-.7-4s.3-2.8.7-4v-5.3H5.7C4.3 17.5 3.5 20.6 3.5 24s.8 6.5 2.2 9.3L12.5 28z", fill: "#FBBC05" }
                path { d: "M24 11.5c3 0 5.7 1 7.8 3.1l5.9-5.9C34.1 5.4 29.5 3.5 24 3.5 16 3.5 9.1 8 5.7 14.7l6.8 5.3c1.6-4.9 6.1-8.5 11.5-8.5z", fill: "#EA4335" }
            }
        }
    }
}

/// Animated emerald E2EE shield mark for Security & Privacy panels.
#[component]
pub fn E2eeLiveShield(#[props(default = 28)] size: u32) -> Element {
    rsx! {
        span { class: "brand-badge cw-shield-pulse", "data-brand": "e2ee", "aria-hidden": "true",
            svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                path { d: "M24 4l16 6v12c0 10.6-6.9 18.6-16 22-9.1-3.4-16-11.4-16-22V10l16-6z", fill: "#16A34A", opacity: "0.16" }
                path { d: "M24 7l13 4.8v10c0 8.8-5.6 15.5-13 18.5-7.4-3-13-9.7-13-18.5v-10L24 7z", fill: "#16A34A" }
                path { d: "M17.5 24.5l4.4 4.4 8.8-9.2", stroke: "#FFFFFF", stroke_width: "3.4", stroke_linecap: "round", stroke_linejoin: "round" }
            }
        }
    }
}

/// Crisp multicolor 2FA authenticator shield logo.
#[component]
pub fn TwoFactorLogo(#[props(default = 28)] size: u32) -> Element {
    rsx! {
        span { class: "brand-badge cw-brand-float", "data-brand": "2fa", "aria-hidden": "true",
            svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                rect { x: "6", y: "6", width: "36", height: "36", rx: "11", fill: "#6366F1", opacity: "0.14" }
                rect { x: "13", y: "21", width: "22", height: "16", rx: "5", fill: "#4F46E5" }
                path { d: "M18 21v-4.5a6 6 0 0 1 12 0V21", stroke: "#4F46E5", stroke_width: "3.2", stroke_linecap: "round" }
                circle { cx: "24", cy: "28.5", r: "2.4", fill: "#FFFFFF" }
                path { d: "M24 30.8v2.4", stroke: "#FFFFFF", stroke_width: "2.4", stroke_linecap: "round" }
            }
        }
    }
}

/// Passkey / Device Key biometric badge.
#[component]
pub fn PasskeyLogo(#[props(default = 28)] size: u32) -> Element {
    rsx! {
        span { class: "brand-badge cw-brand-float", "data-brand": "passkey", "aria-hidden": "true",
            svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                rect { x: "5", y: "5", width: "38", height: "38", rx: "11", fill: "#0EA5E9", opacity: "0.14" }
                circle { cx: "19", cy: "21", r: "6.5", fill: "#0284C7" }
                circle { cx: "19", cy: "21", r: "2.4", fill: "#FFFFFF" }
                path { d: "M25.2 21H36M32 21v4.5M36 21v3.2", stroke: "#0284C7", stroke_width: "3.2", stroke_linecap: "round" }
            }
        }
    }
}

/// Encrypted ZIP archive package logo.
#[component]
pub fn ZipBackupLogo(#[props(default = 28)] size: u32) -> Element {
    rsx! {
        span { class: "brand-badge cw-brand-float", "data-brand": "zip", "aria-hidden": "true",
            svg { width: "{size}", height: "{size}", view_box: "0 0 48 48", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                rect { x: "8", y: "6", width: "32", height: "36", rx: "8", fill: "#F59E0B", opacity: "0.18" }
                rect { x: "11", y: "9", width: "26", height: "30", rx: "6", fill: "#F59E0B" }
                path { d: "M24 13v22", stroke: "#FFFFFF", stroke_width: "2.6", stroke_dasharray: "3 3", stroke_linecap: "round" }
                rect { x: "20", y: "21", width: "8", height: "8", rx: "2.5", fill: "#FFFFFF" }
            }
        }
    }
}

/// Crisp rounded pure-SVG flag / locale icon for LanguageSettings.
#[component]
pub fn LanguageFlagLogo(code: String, #[props(default = 22)] size: u32) -> Element {
    let h = ((size as f32) * 0.72).round() as u32;
    rsx! {
        span { class: "lang-flag-badge", "data-lang-flag": "{code}", "aria-hidden": "true",
            match code.as_str() {
                "vi" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#DA251D" }
                        path { d: "M15 5l1.6 4.8h5l-4 2.9 1.5 4.8-4.1-3-4.1 3 1.5-4.8-4-2.9h5L15 5z", fill: "#FFFF00" }
                    }
                },
                "en" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#1D4ED8" }
                        path { d: "M0 0l30 22M30 0L0 22", stroke: "#FFFFFF", stroke_width: "3.4" }
                        path { d: "M0 0l30 22M30 0L0 22", stroke: "#DC2626", stroke_width: "1.6" }
                        path { d: "M15 0v22M0 11h30", stroke: "#FFFFFF", stroke_width: "5" }
                        path { d: "M15 0v22M0 11h30", stroke: "#DC2626", stroke_width: "2.8" }
                    }
                },
                "ja" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#F8FAFC" }
                        circle { cx: "15", cy: "11", r: "5.2", fill: "#BC002D" }
                    }
                },
                "ko" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#F8FAFC" }
                        path { d: "M15 5.8a5.2 5.2 0 0 1 5.2 5.2H9.8A5.2 5.2 0 0 1 15 5.8z", fill: "#CD2E3A" }
                        path { d: "M15 16.2a5.2 5.2 0 0 1-5.2-5.2h10.4a5.2 5.2 0 0 1-5.2 5.2z", fill: "#0047A0" }
                    }
                },
                "zh-CN" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#DE2910" }
                        path { d: "M7.5 4.2l.9 2.7h2.8L9 8.5l.9 2.7-2.4-1.7-2.4 1.7.9-2.7-2.2-1.6h2.8l.9-2.7z", fill: "#FFDE00" }
                    }
                },
                "zh-TW" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FE0000" }
                        rect { width: "14", height: "11", rx: "2", fill: "#000095" }
                        circle { cx: "7", cy: "5.5", r: "2.8", fill: "#FFFFFF" }
                    }
                },
                "th" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#A51931" }
                        rect { y: "4", width: "30", height: "14", fill: "#FFFFFF" }
                        rect { y: "7.5", width: "30", height: "7", fill: "#2D2A4A" }
                    }
                },
                "fr" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFFFFF" }
                        rect { width: "10", height: "22", rx: "2", fill: "#0055A4" }
                        rect { x: "20", width: "10", height: "22", rx: "2", fill: "#EF4135" }
                    }
                },
                "de" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#DD0000" }
                        rect { width: "30", height: "7.4", rx: "2", fill: "#111827" }
                        rect { y: "14.6", width: "30", height: "7.4", rx: "2", fill: "#FFCE00" }
                    }
                },
                "es" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#AA151B" }
                        rect { y: "5.5", width: "30", height: "11", fill: "#F1BF00" }
                    }
                },
                "it" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFFFFF" }
                        rect { width: "10", height: "22", rx: "2", fill: "#009246" }
                        rect { x: "20", width: "10", height: "22", rx: "2", fill: "#CE2B37" }
                    }
                },
                "id" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFFFFF" }
                        rect { width: "30", height: "11", rx: "2", fill: "#CE1126" }
                    }
                },
                "pt-BR" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#009C3B" }
                        path { d: "M15 3.5L26 11l-11 7.5L4 11l11-7.5z", fill: "#FFDF00" }
                        circle { cx: "15", cy: "11", r: "3.5", fill: "#002776" }
                    }
                },
                "ru" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#0039A6" }
                        rect { width: "30", height: "7.4", rx: "2", fill: "#F8FAFC" }
                        rect { y: "14.6", width: "30", height: "7.4", rx: "2", fill: "#D52B1E" }
                    }
                },
                "uk" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFD700" }
                        rect { width: "30", height: "11", rx: "2", fill: "#005BBB" }
                    }
                },
                "nl" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFFFFF" }
                        rect { width: "30", height: "7.4", rx: "2", fill: "#AE1C28" }
                        rect { y: "14.6", width: "30", height: "7.4", rx: "2", fill: "#21468B" }
                    }
                },
                "pl" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#F8FAFC" }
                        rect { y: "11", width: "30", height: "11", rx: "2", fill: "#DC143C" }
                    }
                },
                "tr" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#E30A17" }
                        circle { cx: "12.5", cy: "11", r: "4.6", fill: "#FFFFFF" }
                        circle { cx: "13.8", cy: "11", r: "3.7", fill: "#E30A17" }
                        circle { cx: "18.8", cy: "11", r: "1.6", fill: "#FFFFFF" }
                    }
                },
                "hi" => rsx! {
                    svg { width: "{size}", height: "{h}", view_box: "0 0 30 22", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                        rect { width: "30", height: "22", rx: "4", fill: "#FFFFFF" }
                        rect { width: "30", height: "7.4", rx: "2", fill: "#FF9933" }
                        rect { y: "14.6", width: "30", height: "7.4", rx: "2", fill: "#138808" }
                        circle { cx: "15", cy: "11", r: "2.3", stroke: "#000080", stroke_width: "1.2" }
                    }
                },
                _ => {
                    let short: String = code.split('-').next().unwrap_or(&code).to_uppercase();
                    rsx! {
                        span { class: "lang-code-pill", "{short}" }
                    }
                }
            }
        }
    }
}
