//! ChatW icon set — "ChatW Duo".
//!
//! Original SVG icons drawn on a 24px grid: rounded 1.8px strokes plus a soft
//! filled layer in the current color (16% opacity). The duotone layer is what
//! gives ChatW its own look; every icon keeps the same optical size and weight.

use dioxus::prelude::*;

/// Base wrapper: stroke layer uses `currentColor`; children with class `duo`
/// render as the soft fill layer.
#[component]
fn Duo(size: u32, children: Element) -> Element {
    rsx! {
        svg {
            class: "cw-icon",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            "focusable": "false",
            {children}
        }
    }
}

macro_rules! duo_fill {
    () => {
        "currentColor"
    };
}

/// Brand mark: a speech bubble carrying a "w".
#[component]
pub fn LogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.18", stroke: "none", d: "M6 3.5h12A3.5 3.5 0 0 1 21.5 7v7.5A3.5 3.5 0 0 1 18 18h-6.5l-4.6 3.3c-.5.36-1.2 0-1.2-.62V18H6a3.5 3.5 0 0 1-3.5-3.5V7A3.5 3.5 0 0 1 6 3.5z" }
        path { d: "M6 3.5h12A3.5 3.5 0 0 1 21.5 7v7.5A3.5 3.5 0 0 1 18 18h-6.5l-4.6 3.3c-.5.36-1.2 0-1.2-.62V18H6a3.5 3.5 0 0 1-3.5-3.5V7A3.5 3.5 0 0 1 6 3.5z" }
        path { d: "M7.5 9l1.7 4.5L12 9.6l2.8 3.9L16.5 9" }
    } }
}

#[component]
pub fn MessageIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", stroke: "none", d: "M6.5 4h11A3.5 3.5 0 0 1 21 7.5v6A3.5 3.5 0 0 1 17.5 17H12l-4.4 3.2c-.46.33-1.1 0-1.1-.56V17A3.5 3.5 0 0 1 3 13.5v-6A3.5 3.5 0 0 1 6.5 4z" }
        path { d: "M6.5 4h11A3.5 3.5 0 0 1 21 7.5v6A3.5 3.5 0 0 1 17.5 17H12l-4.4 3.2c-.46.33-1.1 0-1.1-.56V17A3.5 3.5 0 0 1 3 13.5v-6A3.5 3.5 0 0 1 6.5 4z" }
        circle { cx: "8.5", cy: "10.5", r: "1", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "10.5", r: "1", fill: "currentColor", stroke: "none" }
        circle { cx: "15.5", cy: "10.5", r: "1", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn FriendsIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "9", cy: "8", r: "3.5", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M2.8 19.5c.6-3.3 3.1-5.5 6.2-5.5s5.6 2.2 6.2 5.5" }
        path { d: "M15.2 4.9a3.3 3.3 0 0 1 0 6.2" }
        path { d: "M17.4 14.4c2 .7 3.4 2.6 3.8 5.1" }
    } }
}

#[component]
pub fn UserIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "8", r: "4", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M4.5 20.2c.8-3.8 3.8-6.2 7.5-6.2s6.7 2.4 7.5 6.2" }
    } }
}

#[component]
pub fn UserPlusIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "10", cy: "8", r: "3.8", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M3 20c.7-3.6 3.5-5.9 7-5.9 1.4 0 2.7.4 3.8 1" }
        path { d: "M18.5 13.5v6M15.5 16.5h6" }
    } }
}

#[component]
pub fn GroupIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "8.5", r: "3.2", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M6.8 19.5c.6-2.9 2.6-4.8 5.2-4.8s4.6 1.9 5.2 4.8" }
        circle { cx: "5.2", cy: "10.2", r: "2.2" }
        circle { cx: "18.8", cy: "10.2", r: "2.2" }
        path { d: "M2 18.3c.3-1.6 1.3-2.7 2.7-3.1M22 18.3c-.3-1.6-1.3-2.7-2.7-3.1" }
    } }
}

#[component]
pub fn SettingsIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M10.3 3.2a3.4 3.4 0 0 1 3.4 0l5.1 2.95a3.4 3.4 0 0 1 1.7 2.95v5.8a3.4 3.4 0 0 1-1.7 2.95l-5.1 2.95a3.4 3.4 0 0 1-3.4 0l-5.1-2.95a3.4 3.4 0 0 1-1.7-2.95V9.1a3.4 3.4 0 0 1 1.7-2.95z" }
        circle { cx: "12", cy: "12", r: "3" }
    } }
}

#[component]
pub fn SlidersIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3", y: "4.5", width: "18", height: "6", rx: "3" }
        circle { cx: "15.5", cy: "7.5", r: "1.6", fill: "currentColor", stroke: "none" }
        rect { x: "3", y: "13.5", width: "18", height: "6", rx: "3", fill: duo_fill!(), fill_opacity: "0.16" }
        circle { cx: "8.5", cy: "16.5", r: "1.6", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn EmojiIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M8 9.6q1-1.2 2 0M14 9.6q1-1.2 2 0" }
        path { d: "M7.8 13.6c1 1.9 2.5 2.9 4.2 2.9s3.2-1 4.2-2.9" }
    } }
}

#[component]
pub fn StickerIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M7 3.5h10A3.5 3.5 0 0 1 20.5 7v6.5l-7 7H7A3.5 3.5 0 0 1 3.5 17V7A3.5 3.5 0 0 1 7 3.5z" }
        path { d: "M20.5 13.5h-3.5a3.5 3.5 0 0 0-3.5 3.5v3.5" }
        path { d: "M8.6 10.5q.9-1 1.8 0M13.6 10.5q.9-1 1.8 0" }
    } }
}

#[component]
pub fn SendIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.2", d: "M4.4 4.6l15.4 6.5c.75.32.75 1.38 0 1.7L4.4 19.4c-.8.34-1.6-.45-1.27-1.24L5.8 12 3.13 5.84c-.33-.8.47-1.58 1.27-1.24z" }
        path { d: "M4.4 4.6l15.4 6.5c.75.32.75 1.38 0 1.7L4.4 19.4c-.8.34-1.6-.45-1.27-1.24L5.8 12 3.13 5.84c-.33-.8.47-1.58 1.27-1.24z" }
        path { d: "M5.8 12h6" }
    } }
}

#[component]
pub fn BackIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M10 5.5L3.5 12l6.5 6.5" } path { d: "M4 12h16.5" } } }
}

#[component]
pub fn ChevronLeftIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M14.5 5.5L8 12l6.5 6.5" } } }
}

#[component]
pub fn ChevronRightIcon(#[props(default = 14)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M9.5 5.5L16 12l-6.5 6.5" } } }
}

#[component]
pub fn ChevronDownIcon(#[props(default = 14)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M5.5 9.5L12 16l6.5-6.5" } } }
}

#[component]
pub fn LogoutIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M13 3.5H7A3 3 0 0 0 4 6.5v11a3 3 0 0 0 3 3h6" }
        path { d: "M13 3.5H7A3 3 0 0 0 4 6.5v11a3 3 0 0 0 3 3h6" }
        path { d: "M16.5 8l4 4-4 4M20.5 12H10" }
    } }
}

#[component]
pub fn MenuIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M4 7h16M4 12h11M4 17h7" } } }
}

#[component]
pub fn CloseIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M6.5 6.5l11 11M17.5 6.5l-11 11" } } }
}

#[component]
pub fn ServerIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.5", y: "4", width: "17", height: "7", rx: "3", fill: duo_fill!(), fill_opacity: "0.16" }
        rect { x: "3.5", y: "13", width: "17", height: "7", rx: "3" }
        circle { cx: "7.5", cy: "7.5", r: "1", fill: "currentColor", stroke: "none" }
        circle { cx: "7.5", cy: "16.5", r: "1", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PhoneIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M6.6 3.5h2.1c.5 0 .95.34 1.07.83l.9 3.6a1.1 1.1 0 0 1-.4 1.15l-1.5 1.1a12.2 12.2 0 0 0 5.05 5.05l1.1-1.5a1.1 1.1 0 0 1 1.15-.4l3.6.9c.49.12.83.57.83 1.07v2.1a2.6 2.6 0 0 1-2.8 2.6C9.6 19.4 4.6 14.4 4 6.3a2.6 2.6 0 0 1 2.6-2.8z" }
        path { d: "M6.6 3.5h2.1c.5 0 .95.34 1.07.83l.9 3.6a1.1 1.1 0 0 1-.4 1.15l-1.5 1.1a12.2 12.2 0 0 0 5.05 5.05l1.1-1.5a1.1 1.1 0 0 1 1.15-.4l3.6.9c.49.12.83.57.83 1.07v2.1a2.6 2.6 0 0 1-2.8 2.6C9.6 19.4 4.6 14.4 4 6.3a2.6 2.6 0 0 1 2.6-2.8z" }
    } }
}

#[component]
pub fn PhoneOffIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M3.2 13.6c4.9-4.6 12.7-4.6 17.6 0 .5.47.53 1.25.08 1.76l-1.4 1.55c-.38.42-1 .52-1.5.24l-2.3-1.3a1.1 1.1 0 0 1-.55-1.02l.08-1.7a10 10 0 0 0-6.42 0l.08 1.7c.02.42-.2.82-.55 1.02l-2.3 1.3c-.5.28-1.12.18-1.5-.24l-1.4-1.55a1.25 1.25 0 0 1 .08-1.76z" }
        path { d: "M3.2 13.6c4.9-4.6 12.7-4.6 17.6 0 .5.47.53 1.25.08 1.76l-1.4 1.55c-.38.42-1 .52-1.5.24l-2.3-1.3a1.1 1.1 0 0 1-.55-1.02l.08-1.7a10 10 0 0 0-6.42 0l.08 1.7c.02.42-.2.82-.55 1.02l-2.3 1.3c-.5.28-1.12.18-1.5-.24l-1.4-1.55a1.25 1.25 0 0 1 .08-1.76z" }
    } }
}

#[component]
pub fn VideoIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "5.5", width: "13", height: "13", rx: "3.5", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M15.5 10.2l4.6-2.6c.6-.34 1.4.1 1.4.8v7.2c0 .7-.8 1.14-1.4.8l-4.6-2.6" }
    } }
}

#[component]
pub fn VideoOffIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M15.5 12.5V9a3.5 3.5 0 0 0-3.5-3.5H8M4.2 6.2A3.5 3.5 0 0 0 2.5 9v6A3.5 3.5 0 0 0 6 18.5h6a3.5 3.5 0 0 0 3-1.7" }
        path { d: "M15.5 10.2l4.6-2.6c.6-.34 1.4.1 1.4.8v7.2c0 .4-.25.72-.6.85" }
        path { d: "M3 3l18 18" }
    } }
}

#[component]
pub fn MicIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "8.5", y: "2.5", width: "7", height: "12", rx: "3.5", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M5 11.5a7 7 0 0 0 14 0M12 18.5v3" }
    } }
}

#[component]
pub fn MicOffIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M15.5 10V6a3.5 3.5 0 0 0-6.6-1.6M8.5 9.5V11a3.5 3.5 0 0 0 5.6 2.8" }
        path { d: "M5 11.5a7 7 0 0 0 11.3 5.5M18.6 13.6c.26-.66.4-1.37.4-2.1M12 18.5v3" }
        path { d: "M3 3l18 18" }
    } }
}

#[component]
pub fn PlusIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M12 5v14M5 12h14" } } }
}

#[component]
pub fn InfoIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M12 11v5.5" }
        circle { cx: "12", cy: "7.8", r: "1.1", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn HashIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M9.5 3.5L7.5 20.5M16.5 3.5l-2 17M4 9h16.5M3.5 15H20" } } }
}

#[component]
pub fn VolumeIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M4 9.2h2.8L11 5.6c.6-.5 1.5-.08 1.5.7v11.4c0 .78-.9 1.2-1.5.7l-4.2-3.6H4a1 1 0 0 1-1-1v-3.6a1 1 0 0 1 1-1z" }
        path { d: "M4 9.2h2.8L11 5.6c.6-.5 1.5-.08 1.5.7v11.4c0 .78-.9 1.2-1.5.7l-4.2-3.6H4a1 1 0 0 1-1-1v-3.6a1 1 0 0 1 1-1z" }
        path { d: "M16 9a4.2 4.2 0 0 1 0 6M18.8 6.4a7.8 7.8 0 0 1 0 11.2" }
    } }
}

#[component]
pub fn MegaphoneIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M4 10v4a1.5 1.5 0 0 0 1.5 1.5H8l9.5 4.2V4.3L8 8.5H5.5A1.5 1.5 0 0 0 4 10z" }
        path { d: "M4 10v4a1.5 1.5 0 0 0 1.5 1.5H8l9.5 4.2V4.3L8 8.5H5.5A1.5 1.5 0 0 0 4 10z" }
        path { d: "M8 15.5l1.2 4M20.5 10v4" }
    } }
}

#[component]
pub fn ShieldIcon(#[props(default = 14)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M12 2.8l7.5 3v5.6c0 4.7-3.2 8.3-7.5 9.8-4.3-1.5-7.5-5.1-7.5-9.8V5.8z" }
        path { d: "M12 2.8l7.5 3v5.6c0 4.7-3.2 8.3-7.5 9.8-4.3-1.5-7.5-5.1-7.5-9.8V5.8z" }
        path { d: "M8.8 12l2.2 2.2 4.2-4.4" }
    } }
}

#[component]
pub fn LockIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "4", y: "10.5", width: "16", height: "10.5", rx: "3", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M7.5 10.5V8a4.5 4.5 0 0 1 9 0v2.5" }
        path { d: "M12 14.5v2.5" }
    } }
}

#[component]
pub fn KeyIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "8", cy: "15.5", r: "4.5", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M11.3 12.3L20 3.6M16.5 7.1l2.4 2.4M18.4 5.2l2 2" }
        circle { cx: "8", cy: "15.5", r: "1.2", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn SunIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "4.2", fill: duo_fill!(), fill_opacity: "0.2" }
        path { d: "M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" }
    } }
}

#[component]
pub fn MoonIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.18", d: "M20 14.6A8.3 8.3 0 0 1 9.4 4a8.3 8.3 0 1 0 10.6 10.6z" }
        path { d: "M20 14.6A8.3 8.3 0 0 1 9.4 4a8.3 8.3 0 1 0 10.6 10.6z" }
        path { d: "M16.5 3.5v3M15 5h3" }
    } }
}

#[component]
pub fn MonitorIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "3.5", width: "19", height: "13", rx: "3", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M9 20.5h6M12 16.5v4" }
    } }
}

#[component]
pub fn SearchIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "10.5", cy: "10.5", r: "6.5", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M15.4 15.4l5.1 5.1" }
    } }
}

#[component]
pub fn CheckIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M5 12.5l4.5 4.5L19 7.5" } } }
}

#[component]
pub fn CopyIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "8.5", y: "8.5", width: "12", height: "12", rx: "3", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M15.5 8.5V6.5a3 3 0 0 0-3-3h-6a3 3 0 0 0-3 3v6a3 3 0 0 0 3 3h2" }
    } }
}

#[component]
pub fn BellIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M12 3a6 6 0 0 0-6 6v3.5L4.3 16a.7.7 0 0 0 .62 1h14.16a.7.7 0 0 0 .62-1L18 12.5V9a6 6 0 0 0-6-6z" }
        path { d: "M12 3a6 6 0 0 0-6 6v3.5L4.3 16a.7.7 0 0 0 .62 1h14.16a.7.7 0 0 0 .62-1L18 12.5V9a6 6 0 0 0-6-6z" }
        path { d: "M9.8 20a2.4 2.4 0 0 0 4.4 0" }
    } }
}

#[component]
pub fn PaletteIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.16", d: "M12 3a9 9 0 0 0 0 18c1.2 0 1.8-.9 1.5-1.9-.3-1 .2-2.1 1.4-2.1H17a4 4 0 0 0 4-4C21 7 17 3 12 3z" }
        path { d: "M12 3a9 9 0 0 0 0 18c1.2 0 1.8-.9 1.5-1.9-.3-1 .2-2.1 1.4-2.1H17a4 4 0 0 0 4-4C21 7 17 3 12 3z" }
        circle { cx: "7.8", cy: "11", r: "1.2", fill: "currentColor", stroke: "none" }
        circle { cx: "10.5", cy: "7.3", r: "1.2", fill: "currentColor", stroke: "none" }
        circle { cx: "15.2", cy: "7.8", r: "1.2", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PanelRightIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3", y: "4", width: "18", height: "16", rx: "3.5" }
        path { fill: duo_fill!(), fill_opacity: "0.2", stroke: "none", d: "M14 4h3.5A3.5 3.5 0 0 1 21 7.5v9a3.5 3.5 0 0 1-3.5 3.5H14z" }
        path { d: "M14 4v16" }
    } }
}

#[component]
pub fn CompassIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9" }
        path { fill: duo_fill!(), fill_opacity: "0.25", d: "M15.6 8.4l-2 5.2-5.2 2 2-5.2z" }
        path { d: "M15.6 8.4l-2 5.2-5.2 2 2-5.2z" }
    } }
}

#[component]
pub fn TypeIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M5 7V5h14v2M12 5v14M9 19h6" } } }
}

#[component]
pub fn GoogleIcon() -> Element {
    rsx! {
        svg {
            width: "18",
            height: "18",
            view_box: "0 0 24 24",
            path {
                fill: "#4285F4",
                d: "M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z",
            }
            path {
                fill: "#34A853",
                d: "M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z",
            }
            path {
                fill: "#FBBC05",
                d: "M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z",
            }
            path {
                fill: "#EA4335",
                d: "M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z",
            }
        }
    }
}


#[component]
pub fn ReplyIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M9.5 5.5L3.5 11l6 5.5v-3.2c4.8-.3 8 1.2 11 5.2-.6-6-4.2-9.7-11-10.3z" }
        path { d: "M9.5 5.5L3.5 11l6 5.5v-3.2c4.8-.3 8 1.2 11 5.2-.6-6-4.2-9.7-11-10.3z" }
    } }
}

#[component]
pub fn ClockIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M12 7v5l3.2 2" }
    } }
}

#[component]
pub fn GifIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "5", width: "19", height: "14", rx: "4", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M10 9.5H8.2A1.7 1.7 0 0 0 6.5 11.2v1.6a1.7 1.7 0 0 0 1.7 1.7H10V12H8.6" }
        path { d: "M12.5 9.5v5M15.5 14.5v-5H18M15.5 12h2" }
    } }
}

#[component]
pub fn SmilePlusIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M20.5 12a8.5 8.5 0 1 1-6.1-8.16" }
        path { d: "M20.5 12a8.5 8.5 0 1 1-6.1-8.16" }
        path { d: "M8.6 10q.8-1 1.6 0M13.8 10q.8-1 1.6 0M8.2 14q1.5 2.4 3.8 2.4t3.8-2.4" }
        path { d: "M19 2.5v5M16.5 5h5" }
    } }
}

#[component]
pub fn SparkleIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.2", d: "M12 3l2.1 5.9L20 11l-5.9 2.1L12 19l-2.1-5.9L4 11l5.9-2.1z" }
        path { d: "M12 3l2.1 5.9L20 11l-5.9 2.1L12 19l-2.1-5.9L4 11l5.9-2.1z" }
        path { d: "M19 17.5v3M17.5 19h3" }
    } }
}

#[component]
pub fn BrushIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M8.5 14.5c-2 0-3.5 1.5-3.5 3.5 0 1-.5 2-1.5 2.5 3 .5 7-.5 7-4" }
        path { d: "M8.5 14.5c-2 0-3.5 1.5-3.5 3.5 0 1-.5 2-1.5 2.5 3 .5 7-.5 7-4" }
        path { d: "M9.5 13.5l9.8-9.8a1.8 1.8 0 0 1 2.5 2.5L12 16" }
    } }
}

#[component]
pub fn DevicesIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "4", width: "14", height: "10", rx: "2.5", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M6 18h7M9.5 14v4" }
        rect { x: "16", y: "8", width: "5.5", height: "11", rx: "1.8" }
    } }
}

#[component]
pub fn BookIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5z" }
        path { d: "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5zM4 20.5A2.5 2.5 0 0 0 6.5 23H20v-5" }
        path { d: "M9 7.5h7M9 11h5" }
    } }
}

#[component]
pub fn LibraryIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3", y: "3", width: "8", height: "8", rx: "2.5", fill: "currentColor", fill_opacity: "0.16" }
        rect { x: "13", y: "3", width: "8", height: "8", rx: "2.5" }
        rect { x: "3", y: "13", width: "8", height: "8", rx: "2.5" }
        circle { cx: "17", cy: "17", r: "4", fill: "currentColor", fill_opacity: "0.16" }
    } }
}

#[component]
pub fn PinIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M9 3.5h6l-1 5 3.5 3.5v1.5h-11V12L10 8.5z" }
        path { d: "M9 3.5h6l-1 5 3.5 3.5v1.5h-11V12L10 8.5zM12 13.5v7" }
    } }
}

#[component]
pub fn TagIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M3.5 4.5v6.3a2 2 0 0 0 .6 1.4l7.7 7.7a2 2 0 0 0 2.8 0l5.3-5.3a2 2 0 0 0 0-2.8L12.2 4.1a2 2 0 0 0-1.4-.6H4.5a1 1 0 0 0-1 1z" }
        path { d: "M3.5 4.5v6.3a2 2 0 0 0 .6 1.4l7.7 7.7a2 2 0 0 0 2.8 0l5.3-5.3a2 2 0 0 0 0-2.8L12.2 4.1a2 2 0 0 0-1.4-.6H4.5a1 1 0 0 0-1 1z" }
        circle { cx: "8", cy: "8", r: "1.4", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn MoreIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "5.5", cy: "12", r: "1.6", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "12", r: "1.6", fill: "currentColor", stroke: "none" }
        circle { cx: "18.5", cy: "12", r: "1.6", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn BlockIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "8.5", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M6 6l12 12" }
    } }
}

#[component]
pub fn BotIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "4", y: "7", width: "16", height: "12", rx: "4", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M12 3.5V7M9 12.5v1M15 12.5v1M2.5 12v3M21.5 12v3" }
        circle { cx: "12", cy: "3", r: "1.2", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PluginIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M14 7V4.5a2.5 2.5 0 0 0-5 0V7H6a2 2 0 0 0-2 2v3a2.5 2.5 0 0 0 0 5v3a2 2 0 0 0 2 2h3v-2.5a2.5 2.5 0 0 1 5 0V22h3a2 2 0 0 0 2-2v-3a2.5 2.5 0 0 1 0-5V9a2 2 0 0 0-2-2h-3z" }
        path { d: "M14 7V4.5a2.5 2.5 0 0 0-5 0V7H6a2 2 0 0 0-2 2v3a2.5 2.5 0 0 0 0 5v3a2 2 0 0 0 2 2h3v-2.5a2.5 2.5 0 0 1 5 0V22h3a2 2 0 0 0 2-2v-3a2.5 2.5 0 0 1 0-5V9a2 2 0 0 0-2-2h-3z" }
    } }
}

#[component]
pub fn StoreIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M4 9.5h16v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" }
        path { d: "M3.5 9.5L5.2 4.5h13.6l1.7 5M4 9.5h16v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zM9.5 20.5v-5h5v5" }
    } }
}

#[component]
pub fn GlobeIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M3 12h18M12 3c2.6 2.6 3.8 5.6 3.8 9s-1.2 6.4-3.8 9M12 3c-2.6 2.6-3.8 5.6-3.8 9s1.2 6.4 3.8 9" }
    } }
}

#[component]
pub fn LinkIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M10 14a4.5 4.5 0 0 0 6.4 0l3-3a4.5 4.5 0 0 0-6.4-6.4l-1.2 1.2M14 10a4.5 4.5 0 0 0-6.4 0l-3 3a4.5 4.5 0 0 0 6.4 6.4l1.2-1.2" }
    } }
}

#[component]
pub fn BellOffIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M6 16.5V11a6 6 0 0 1 9.4-4.9M18 11v5.5l1.5 1.5H8" }
        path { d: "M6 16.5V11a6 6 0 0 1 9.4-4.9M18 11v5.5l1.5 1.5H8M4.5 18h1M10 21h4M3 3l18 18" }
    } }
}

#[component]
pub fn ImageIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.5", y: "4.5", width: "17", height: "15", rx: "3.5", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M3.5 16l4.5-4.5 3.5 3.5 2.5-2.5 6.5 6" }
        rect { x: "3.5", y: "4.5", width: "17", height: "15", rx: "3.5" }
        circle { cx: "15.5", cy: "9", r: "1.6" }
    } }
}

#[component]
pub fn CrownIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M4 8l4 3.5L12 5l4 6.5L20 8l-1.5 10h-13z" }
        path { d: "M4 8l4 3.5L12 5l4 6.5L20 8l-1.5 10h-13z" }
    } }
}

#[component]
pub fn TrashIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.16", d: "M6 7h12l-1 12.5a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 7 19.5z" }
        path { d: "M4.5 7h15M9.5 7V4.5h5V7M6 7l1 12.5a1.5 1.5 0 0 0 1.5 1.5h7a1.5 1.5 0 0 0 1.5-1.5L18 7M10 11v6M14 11v6" }
    } }
}

#[component]
pub fn ShareIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "18", cy: "5.5", r: "2.5", fill: "currentColor", fill_opacity: "0.16" }
        circle { cx: "6", cy: "12", r: "2.5", fill: "currentColor", fill_opacity: "0.16" }
        circle { cx: "18", cy: "18.5", r: "2.5", fill: "currentColor", fill_opacity: "0.16" }
        path { d: "M8.3 10.8l7.4-4.1M8.3 13.2l7.4 4.1" }
    } }
}

#[component]
pub fn RefreshIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M19.5 12a7.5 7.5 0 1 1-2.2-5.3M19.5 4.5v4h-4" }
    } }
}
