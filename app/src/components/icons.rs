//! ChatW icon set — "ChatW Duo v2" (Sculpted Modern Duotone).
//!
//! Original SVG icons drawn on a full-bleed 24px optical grid (2..22):
//! confident 2.05px rounded strokes paired with a rich duotone fill layer
//! (22% opacity) and crisp solid accent points for high legibility at all sizes.

use dioxus::prelude::*;

/// Base wrapper: stroke layer uses `currentColor`; duotone children render soft fill depth.
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
            stroke_width: "2.05",
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

/// Brand mark: sculpted speech bubble carrying a crisp rounded "W".
#[component]
pub fn LogoIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", stroke: "none", d: "M5.5 2.5h13A3.8 3.8 0 0 1 22.3 6.3v8.4a3.8 3.8 0 0 1-3.8 3.8h-6.8l-4.9 3.4c-.55.38-1.3 0-1.3-.68v-2.72A3.8 3.8 0 0 1 1.7 14.7V6.3A3.8 3.8 0 0 1 5.5 2.5z" }
        path { d: "M5.5 2.5h13A3.8 3.8 0 0 1 22.3 6.3v8.4a3.8 3.8 0 0 1-3.8 3.8h-6.8l-4.9 3.4c-.55.38-1.3 0-1.3-.68v-2.72A3.8 3.8 0 0 1 1.7 14.7V6.3A3.8 3.8 0 0 1 5.5 2.5z" }
        path { stroke_width: "2.25", d: "M7.2 8.4l1.9 5.1L12 9.2l2.9 4.3 1.9-5.1" }
    } }
}

#[component]
pub fn MessageIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", stroke: "none", d: "M6 3h12a4 4 0 0 1 4 4v7a4 4 0 0 1-4 4h-6.2l-4.7 3.3c-.52.36-1.25 0-1.25-.64V18A4 4 0 0 1 2 14V7a4 4 0 0 1 4-4z" }
        path { d: "M6 3h12a4 4 0 0 1 4 4v7a4 4 0 0 1-4 4h-6.2l-4.7 3.3c-.52.36-1.25 0-1.25-.64V18A4 4 0 0 1 2 14V7a4 4 0 0 1 4-4z" }
        circle { cx: "8.2", cy: "10.5", r: "1.35", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "10.5", r: "1.35", fill: "currentColor", stroke: "none" }
        circle { cx: "15.8", cy: "10.5", r: "1.35", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn FriendsIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "9.2", cy: "7.4", r: "3.9", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.22", stroke: "none", d: "M2.1 20.6c.6-3.8 3.4-6.1 7.1-6.1s6.5 2.3 7.1 6.1z" }
        path { d: "M2.1 20.6c.6-3.8 3.4-6.1 7.1-6.1s6.5 2.3 7.1 6.1" }
        path { fill: duo_fill!(), fill_opacity: "0.14", stroke: "none", d: "M15.8 4.4a3.5 3.5 0 0 1 0 6.6z" }
        path { d: "M15.8 4.4a3.5 3.5 0 0 1 0 6.6" }
        path { d: "M18 14.8c2.1.8 3.6 2.8 4 5.8" }
    } }
}

#[component]
pub fn UserIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "7.8", r: "4.2", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.2", stroke: "none", d: "M3.8 20.8c.9-4.1 4.1-6.6 8.2-6.6s7.3 2.5 8.2 6.6z" }
        path { d: "M3.8 20.8c.9-4.1 4.1-6.6 8.2-6.6s7.3 2.5 8.2 6.6" }
    } }
}

#[component]
pub fn UserPlusIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "9.8", cy: "7.8", r: "4", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.18", stroke: "none", d: "M2.5 20.5c.8-3.8 3.8-6.2 7.3-6.2 1.5 0 2.9.4 4.1 1.1v5.1z" }
        path { d: "M2.5 20.5c.8-3.8 3.8-6.2 7.3-6.2 1.5 0 2.9.4 4.1 1.1" }
        path { d: "M19 13.5v6.5M15.75 16.75h6.5" }
    } }
}

#[component]
pub fn GroupIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "8", r: "3.5", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.2", stroke: "none", d: "M6.2 20.2c.7-3.2 2.9-5.2 5.8-5.2s5.1 2 5.8 5.2z" }
        path { d: "M6.2 20.2c.7-3.2 2.9-5.2 5.8-5.2s5.1 2 5.8 5.2" }
        circle { cx: "4.8", cy: "10", r: "2.4", fill: duo_fill!(), fill_opacity: "0.16" }
        circle { cx: "19.2", cy: "10", r: "2.4", fill: duo_fill!(), fill_opacity: "0.16" }
        path { d: "M1.6 19c.3-1.8 1.4-3 2.9-3.4M22.4 19c-.3-1.8-1.4-3-2.9-3.4" }
    } }
}

#[component]
pub fn SettingsIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path {
            fill: duo_fill!(),
            fill_opacity: "0.22",
            fill_rule: "evenodd",
            stroke: "none",
            d: "M10.4 2.55a1.9 1.9 0 0 1 3.2 0l.65 1.1a1.8 1.8 0 0 0 2.2.76l1.18-.44a1.9 1.9 0 0 1 2.4 2.4l-.44 1.18a1.8 1.8 0 0 0 .76 2.2l1.1.65a1.9 1.9 0 0 1 0 3.2l-1.1.65a1.8 1.8 0 0 0-.76 2.2l.44 1.18a1.9 1.9 0 0 1-2.4 2.4l-1.18-.44a1.8 1.8 0 0 0-2.2.76l-.65 1.1a1.9 1.9 0 0 1-3.2 0l-.65-1.1a1.8 1.8 0 0 0-2.2-.76l-1.18.44a1.9 1.9 0 0 1-2.4-2.4l.44-1.18a1.8 1.8 0 0 0-.76-2.2l-1.1-.65a1.9 1.9 0 0 1 0-3.2l1.1-.65a1.8 1.8 0 0 0 .76-2.2l-.44-1.18a1.9 1.9 0 0 1 2.4-2.4l1.18.44a1.8 1.8 0 0 0 2.2-.76zM12 8.7a3.3 3.3 0 1 0 0 6.6 3.3 3.3 0 1 0 0-6.6z",
        }
        path {
            d: "M10.4 2.55a1.9 1.9 0 0 1 3.2 0l.65 1.1a1.8 1.8 0 0 0 2.2.76l1.18-.44a1.9 1.9 0 0 1 2.4 2.4l-.44 1.18a1.8 1.8 0 0 0 .76 2.2l1.1.65a1.9 1.9 0 0 1 0 3.2l-1.1.65a1.8 1.8 0 0 0-.76 2.2l.44 1.18a1.9 1.9 0 0 1-2.4 2.4l-1.18-.44a1.8 1.8 0 0 0-2.2.76l-.65 1.1a1.9 1.9 0 0 1-3.2 0l-.65-1.1a1.8 1.8 0 0 0-2.2-.76l-1.18.44a1.9 1.9 0 0 1-2.4-2.4l.44-1.18a1.8 1.8 0 0 0-.76-2.2l-1.1-.65a1.9 1.9 0 0 1 0-3.2l1.1-.65a1.8 1.8 0 0 0 .76-2.2l-.44-1.18a1.9 1.9 0 0 1 2.4-2.4l1.18.44a1.8 1.8 0 0 0 2.2-.76z",
        }
        circle { cx: "12", cy: "12", r: "3.3" }
    } }
}

#[component]
pub fn SlidersIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "4", width: "19", height: "6.4", rx: "3.2" }
        circle { cx: "15.8", cy: "7.2", r: "1.8", fill: "currentColor", stroke: "none" }
        rect { x: "2.5", y: "13.6", width: "19", height: "6.4", rx: "3.2", fill: duo_fill!(), fill_opacity: "0.22" }
        circle { cx: "8.2", cy: "16.8", r: "1.8", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn EmojiIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.5", fill: duo_fill!(), fill_opacity: "0.22" }
        circle { cx: "8.8", cy: "9.8", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "15.2", cy: "9.8", r: "1.3", fill: "currentColor", stroke: "none" }
        path { d: "M7.8 14c1.1 2.1 2.6 3.1 4.2 3.1s3.1-1 4.2-3.1" }
    } }
}

#[component]
pub fn StickerIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M6.5 2.5h11a4 4 0 0 1 4 4v7.2l-7.8 7.8H6.5a4 4 0 0 1-4-4v-11a4 4 0 0 1 4-4z" }
        path { d: "M21.5 13.7h-4a3.8 3.8 0 0 0-3.8 3.8v4" }
        circle { cx: "9.2", cy: "10", r: "1.25", fill: "currentColor", stroke: "none" }
        circle { cx: "14.8", cy: "10", r: "1.25", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn SendIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.25", d: "M3.8 3.8l16.8 7.3c.82.36.82 1.44 0 1.8L3.8 20.2c-.86.38-1.74-.48-1.36-1.34L5.6 12 2.44 5.14C2.06 4.28 2.94 3.42 3.8 3.8z" }
        path { d: "M5.6 12h6.8" }
    } }
}

#[component]
pub fn BackIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M10.2 5L3.2 12l7 7" } path { d: "M3.8 12h17" } } }
}

#[component]
pub fn ChevronLeftIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M14.8 5L7.8 12l7 7" } } }
}

#[component]
pub fn ChevronRightIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M9.2 5l7 7-7 7" } } }
}

#[component]
pub fn ChevronDownIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M5 9.2l7 7 7-7" } } }
}

#[component]
pub fn LogoutIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", stroke: "none", d: "M13 2.8H6.8a3.3 3.3 0 0 0-3.3 3.3v11.8a3.3 3.3 0 0 0 3.3 3.3H13z" }
        path { d: "M13 2.8H6.8a3.3 3.3 0 0 0-3.3 3.3v11.8a3.3 3.3 0 0 0 3.3 3.3H13" }
        path { d: "M16.8 7.6l4.4 4.4-4.4 4.4M21.2 12H9.8" }
    } }
}

#[component]
pub fn MenuIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M3.5 6.5h17M3.5 12h12M3.5 17.5h8" } } }
}

#[component]
pub fn CloseIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M6 6l12 12M18 6L6 18" } } }
}

#[component]
pub fn ServerIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.8", y: "3.2", width: "18.4", height: "7.6", rx: "3.2", fill: duo_fill!(), fill_opacity: "0.22" }
        rect { x: "2.8", y: "13.2", width: "18.4", height: "7.6", rx: "3.2" }
        circle { cx: "7.2", cy: "7", r: "1.2", fill: "currentColor", stroke: "none" }
        circle { cx: "7.2", cy: "17", r: "1.2", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PhoneIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M6.2 2.8h2.3c.55 0 1.04.38 1.18.92l.95 3.8a1.2 1.2 0 0 1-.44 1.26l-1.6 1.2a13 13 0 0 0 5.43 5.43l1.2-1.6a1.2 1.2 0 0 1 1.26-.44l3.8.95c.54.14.92.63.92 1.18v2.3a2.8 2.8 0 0 1-3.02 2.8C9.3 20.1 3.9 14.7 3.4 5.82A2.8 2.8 0 0 1 6.2 2.8z" }
    } }
}

#[component]
pub fn PhoneOffIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M2.6 13.6c5.3-4.9 13.5-4.9 18.8 0 .54.5.56 1.35.08 1.88l-1.5 1.65c-.4.45-1.08.56-1.62.26l-2.45-1.38a1.2 1.2 0 0 1-.6-1.1l.08-1.8a10.5 10.5 0 0 0-6.78 0l.08 1.8c.02.46-.22.88-.6 1.1l-2.45 1.38c-.54.3-1.22.19-1.62-.26l-1.5-1.65a1.34 1.34 0 0 1 .08-1.88z" }
    } }
}

#[component]
pub fn VideoIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "1.8", y: "4.8", width: "14.2", height: "14.4", rx: "4.2", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.3", d: "M16 9.8l4.9-2.9c.68-.4 1.55.1 1.55.88v8.44c0 .78-.87 1.28-1.55.88L16 14.2z" }
        circle { cx: "6.4", cy: "9.2", r: "1.25", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn VideoOffIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M16 12.5V8.8a4 4 0 0 0-4-4H7.5M3.8 5.8A4 4 0 0 0 2 9v6a4 4 0 0 0 4 4h6a4 4 0 0 0 3.4-1.9" }
        path { d: "M16 9.8l4.9-2.9c.68-.4 1.55.1 1.55.88v8.44c0 .44-.27.8-.65.94" }
        path { d: "M2.5 2.5l19 19" }
    } }
}

#[component]
pub fn MicIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "8", y: "2", width: "8", height: "12.5", rx: "4", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M4.5 11.2a7.5 7.5 0 0 0 15 0M12 18.7v3.3M8.8 22h6.4" }
    } }
}

#[component]
pub fn MicOffIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M16 10V6a4 4 0 0 0-7.4-2M8 9.2V11a4 4 0 0 0 6.2 3.3" }
        path { d: "M4.5 11.2a7.5 7.5 0 0 0 12 6M19.2 13.6c.2-.75.3-1.55.3-2.4M12 18.7v3.3" }
        path { d: "M2.5 2.5l19 19" }
    } }
}

#[component]
pub fn PlusIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size, path { stroke_width: "2.3", d: "M12 4v16M4 12h16" } } }
}

#[component]
pub fn InfoIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.5", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M12 11v6" }
        circle { cx: "12", cy: "7.5", r: "1.3", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn HashIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M9.5 3L7.2 21M16.8 3l-2.3 18M3.5 8.8h17.5M3 15.2h17.5" } } }
}

#[component]
pub fn VolumeIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M3.2 8.8h3.2L11 5c.65-.52 1.6-.08 1.6.75v12.5c0 .83-.95 1.27-1.6.75l-4.6-3.8H3.2a1.2 1.2 0 0 1-1.2-1.2v-4a1.2 1.2 0 0 1 1.2-1.2z" }
        path { d: "M16.4 8.6a4.8 4.8 0 0 1 0 6.8M19.4 5.8a8.6 8.6 0 0 1 0 12.4" }
    } }
}

#[component]
pub fn MegaphoneIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M3.5 9.8v4.4a1.8 1.8 0 0 0 1.8 1.8H8l10 4.5V3.5L8 8H5.3a1.8 1.8 0 0 0-1.8 1.8z" }
        path { d: "M8 16l1.4 4.5M21 9.5v5" }
    } }
}

#[component]
pub fn ShieldIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M12 2.2l8 3.2v6c0 5.1-3.4 9-8 10.6-4.6-1.6-8-5.5-8-10.6v-6z" }
        path { d: "M8.6 12l2.4 2.4 4.6-4.8" }
    } }
}

#[component]
pub fn LockIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.5", y: "10.2", width: "17", height: "11.2", rx: "3.5", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M7.2 10.2V7.6a4.8 4.8 0 0 1 9.6 0v2.6" }
        circle { cx: "12", cy: "15.5", r: "1.4", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn KeyIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "7.8", cy: "15.8", r: "4.8", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M11.3 12.3L20.8 2.8M16.8 6.8l2.6 2.6M18.8 4.8l2.2 2.2" }
        circle { cx: "7.8", cy: "15.8", r: "1.35", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn SunIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "4.6", fill: duo_fill!(), fill_opacity: "0.25" }
        path { d: "M12 2v2.2M12 19.8V22M2 12h2.2M19.8 12H22M4.9 4.9l1.6 1.6M17.5 17.5l1.6 1.6M4.9 19.1l1.6-1.6M17.5 6.5l1.6-1.6" }
    } }
}

#[component]
pub fn MoonIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M20.6 14.8A8.8 8.8 0 0 1 9.2 3.4a8.8 8.8 0 1 0 11.4 11.4z" }
        path { d: "M17 3v3.2M15.4 4.6h3.2" }
    } }
}

#[component]
pub fn MonitorIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2", y: "3", width: "20", height: "13.8", rx: "3.2", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M8.5 21h7M12 16.8V21" }
    } }
}

#[component]
pub fn SearchIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "10.5", cy: "10.5", r: "7.6", fill: duo_fill!(), fill_opacity: "0.22" }
        path { stroke_opacity: "0.5", stroke_width: "1.7", d: "M7.2 8a4 4 0 0 1 4.4-1.4" }
        path { stroke_width: "2.45", d: "M16.1 16.1l5.3 5.3" }
    } }
}

#[component]
pub fn CheckIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M4.5 12.6l4.8 4.8L19.5 6.8" } } }
}

#[component]
pub fn CopyIcon(#[props(default = 19)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "8.2", y: "8.2", width: "12.8", height: "12.8", rx: "3.2", fill: duo_fill!(), fill_opacity: "0.22" }
        path { d: "M15.8 8.2V6a3 3 0 0 0-3-3H6a3 3 0 0 0-3 3v6.8a3 3 0 0 0 3 3h2.2" }
    } }
}

#[component]
pub fn BellIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M12 2.2v1.3" }
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M12 3.5a6.4 6.4 0 0 0-6.4 6.4v3.4l-1.9 3.2c-.35.58.07 1.3.75 1.3h15.1c.68 0 1.1-.72.75-1.3l-1.9-3.2V9.9A6.4 6.4 0 0 0 12 3.5z" }
        path { d: "M9.4 20.5a2.8 2.8 0 0 0 5.2 0" }
    } }
}

#[component]
pub fn PaletteIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", d: "M12 2.5a9.5 9.5 0 0 0 0 19c1.3 0 2-1 1.6-2.1-.3-1.1.2-2.2 1.5-2.2h2.1a4.3 4.3 0 0 0 4.3-4.3C21.5 7.2 17.3 2.5 12 2.5z" }
        circle { cx: "7.5", cy: "11.2", r: "1.35", fill: "currentColor", stroke: "none" }
        circle { cx: "10.6", cy: "7.1", r: "1.35", fill: "currentColor", stroke: "none" }
        circle { cx: "15.5", cy: "7.8", r: "1.35", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PanelRightIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "3.5", width: "19", height: "17", rx: "3.8" }
        path { fill: duo_fill!(), fill_opacity: "0.24", stroke: "none", d: "M14.2 3.5h3.5a3.8 3.8 0 0 1 3.8 3.8v9.4a3.8 3.8 0 0 1-3.8 3.8h-3.5z" }
        path { d: "M14.2 3.5v17" }
    } }
}

#[component]
pub fn CompassIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.6", fill: duo_fill!(), fill_opacity: "0.18" }
        path { fill: "currentColor", stroke: "none", d: "M16.5 7.5l-2.4 6.4-4-4z" }
        path { fill: duo_fill!(), fill_opacity: "0.35", stroke: "none", d: "M7.5 16.5l2.4-6.4 4 4z" }
        path { stroke_width: "1.75", d: "M16.5 7.5l-2.4 6.4-6.6 2.6 2.4-6.4z" }
    } }
}

#[component]
pub fn TypeIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size, path { d: "M4.5 6.8V4.5h15v2.3M12 4.5v15M8.5 19.5h7" } } }
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
pub fn ReplyIcon(#[props(default = 19)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M9.5 4.8L3 10.8l6.5 6v-3.5c5-.3 8.4 1.3 11.5 5.6-.7-6.4-4.5-10.3-11.5-11z" }
    } }
}

#[component]
pub fn ClockIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.5", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M12 6.8v5.4l3.6 2.2" }
    } }
}

#[component]
pub fn GifIcon(#[props(default = 24)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2", y: "4.5", width: "20", height: "15", rx: "4.2", fill: "currentColor", fill_opacity: "0.22" }
        path { stroke_width: "1.95", d: "M9.8 9.2H7.9a1.8 1.8 0 0 0-1.8 1.8v2a1.8 1.8 0 0 0 1.8 1.8h1.9V12H8.2" }
        path { stroke_width: "1.95", d: "M12.5 9.2v5.6M15.4 14.8V9.2h2.8M15.4 12h2.2" }
    } }
}

#[component]
pub fn SmilePlusIcon(#[props(default = 19)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: "currentColor", fill_opacity: "0.22", stroke: "none" }
        path { d: "M21 12a9 9 0 1 1-6.4-8.6" }
        circle { cx: "9.2", cy: "10", r: "1.2", fill: "currentColor", stroke: "none" }
        circle { cx: "14.8", cy: "10", r: "1.2", fill: "currentColor", stroke: "none" }
        path { d: "M8 14.2q1.6 2.5 4 2.5t4-2.5" }
        path { d: "M19.2 2v5.4M16.5 4.7h5.4" }
    } }
}

#[component]
pub fn SparkleIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.24", d: "M12 2.2l2.3 6.5L21 11l-6.7 2.3L12 19.8l-2.3-6.5L3 11l6.7-2.3z" }
        path { d: "M19.2 17.4v3.4M17.5 19.1h3.4" }
    } }
}

#[component]
pub fn BrushIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M8.5 14.5c-2.2 0-3.8 1.6-3.8 3.8 0 1-.5 2-1.7 2.6 3.2.6 7.5-.5 7.5-4.3" }
        path { d: "M9.6 13.4l10-10a2 2 0 0 1 2.8 2.8l-10 10" }
    } }
}

#[component]
pub fn DevicesIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2", y: "3.5", width: "14.5", height: "10.8", rx: "2.8", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M5.8 18.5h7M9.3 14.3v4.2" }
        rect { x: "15.8", y: "7.8", width: "6.2", height: "12.2", rx: "2" }
    } }
}

#[component]
pub fn BookIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M3.5 5A2.5 2.5 0 0 1 6 2.5h14.5v16H6A2.5 2.5 0 0 0 3.5 21z" }
        path { d: "M3.5 21A2.5 2.5 0 0 0 6 23h14.5v-4.5" }
        path { d: "M8.8 7.5h7.4M8.8 11.2h5.2" }
    } }
}

#[component]
pub fn LibraryIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "2.5", width: "8.4", height: "8.4", rx: "2.6", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "13.1", y: "2.5", width: "8.4", height: "8.4", rx: "2.6" }
        rect { x: "2.5", y: "13.1", width: "8.4", height: "8.4", rx: "2.6" }
        circle { cx: "17.3", cy: "17.3", r: "4.2", fill: "currentColor", fill_opacity: "0.22" }
    } }
}

#[component]
pub fn PinIcon(#[props(default = 19)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M8.8 3h6.4l-1.1 5.4 3.7 3.7v1.6H6.2v-1.6l3.7-3.7z" }
        path { d: "M12 13.7v7.5" }
    } }
}

#[component]
pub fn TagIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M3 4.2v6.6a2.2 2.2 0 0 0 .65 1.55l8 8a2.2 2.2 0 0 0 3.1 0l5.6-5.6a2.2 2.2 0 0 0 0-3.1l-8-8A2.2 2.2 0 0 0 10.8 3H4.2A1.2 1.2 0 0 0 3 4.2z" }
        circle { cx: "7.8", cy: "7.8", r: "1.5", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn MoreIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "5", cy: "12", r: "1.85", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "12", r: "1.85", fill: "currentColor", stroke: "none" }
        circle { cx: "19", cy: "12", r: "1.85", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn BlockIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M5.8 5.8l12.4 12.4" }
    } }
}

#[component]
pub fn BotIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.5", y: "6.8", width: "17", height: "13", rx: "4.2", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M12 3v3.8M2 11.8v3.4M22 11.8v3.4" }
        circle { cx: "9", cy: "13.2", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "15", cy: "13.2", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "2.6", r: "1.3", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn PluginIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.4", y: "2.4", width: "8", height: "8", rx: "2.6", fill: duo_fill!(), fill_opacity: "0.22" }
        rect { x: "2.4", y: "13.6", width: "8", height: "8", rx: "2.6", fill: duo_fill!(), fill_opacity: "0.14" }
        rect { x: "13.6", y: "13.6", width: "8", height: "8", rx: "2.6", fill: duo_fill!(), fill_opacity: "0.14" }
        rect { x: "14.1", y: "3.1", width: "6.8", height: "6.8", rx: "2", transform: "rotate(45 17.5 6.5)", fill: duo_fill!(), fill_opacity: "0.34" }
    } }
}

#[component]
pub fn StoreIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M3.5 9.2h17v9.6a2.2 2.2 0 0 1-2.2 2.2H5.7a2.2 2.2 0 0 1-2.2-2.2z" }
        path { d: "M2.8 9.2L4.8 3.8h14.4l2 5.4M9.2 21v-5.5h5.6V21" }
    } }
}

#[component]
pub fn GlobeIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.5", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M2.5 12h19M12 2.5c2.8 2.8 4.1 6 4.1 9.5s-1.3 6.7-4.1 9.5M12 2.5c-2.8 2.8-4.1 6-4.1 9.5s1.3 6.7 4.1 9.5" }
    } }
}

#[component]
pub fn LinkIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M9.8 14.2a4.8 4.8 0 0 0 6.8 0l3.1-3.1a4.8 4.8 0 0 0-6.8-6.8l-1.3 1.3M14.2 9.8a4.8 4.8 0 0 0-6.8 0l-3.1 3.1a4.8 4.8 0 0 0 6.8 6.8l1.3-1.3" }
    } }
}

#[component]
pub fn BellOffIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", stroke: "none", d: "M5.6 16.8V10.8a6.4 6.4 0 0 1 10-5.3L18.4 10.8v6l1.8 1.6H7.6z" }
        path { d: "M5.6 16.8V10.8a6.4 6.4 0 0 1 10-5.3M18.4 10.8v6l1.8 1.6H7.6M4 18.4h1.2M9.6 21.2h4.8M2.5 2.5l19 19" }
    } }
}

#[component]
pub fn ImageIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.8", y: "3.8", width: "18.4", height: "16.4", rx: "3.8", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M2.8 16.2l4.8-4.8 3.8 3.8 2.8-2.8 7 6.4" }
        circle { cx: "15.8", cy: "8.8", r: "1.7", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn CrownIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.24", d: "M3.2 7.5l4.4 3.8L12 4.2l4.4 7.1 4.4-3.8-1.6 11.3H4.8z" }
    } }
}

#[component]
pub fn TrashIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", stroke: "none", d: "M5.5 6.8h13l-1.1 13a1.8 1.8 0 0 1-1.8 1.7H8.4a1.8 1.8 0 0 1-1.8-1.7z" }
        path { d: "M4 6.8h16M9.2 6.8V4a1.5 1.5 0 0 1 1.5-1.5h2.6A1.5 1.5 0 0 1 14.8 4v2.8M5.5 6.8l1.1 13a1.8 1.8 0 0 0 1.8 1.7h7.2a1.8 1.8 0 0 0 1.8-1.7l1.1-13M9.8 11v6.2M14.2 11v6.2" }
    } }
}

#[component]
pub fn ShareIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "18.2", cy: "5.2", r: "2.8", fill: "currentColor", fill_opacity: "0.22" }
        circle { cx: "5.8", cy: "12", r: "2.8", fill: "currentColor", fill_opacity: "0.22" }
        circle { cx: "18.2", cy: "18.8", r: "2.8", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M8.3 10.6l7.4-4M8.3 13.4l7.4 4" }
    } }
}

#[component]
pub fn RefreshIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M20.2 12a8.2 8.2 0 1 1-2.4-5.8M20.2 3.8v4.6h-4.6" }
    } }
}

#[component]
pub fn DownloadIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: duo_fill!(), fill_opacity: "0.22", stroke: "none", d: "M3.2 14.5h17.6v3.2a3.3 3.3 0 0 1-3.3 3.3H6.5a3.3 3.3 0 0 1-3.3-3.3z" }
        path { d: "M3.2 14.5v3.2a3.3 3.3 0 0 0 3.3 3.3h11a3.3 3.3 0 0 0 3.3-3.3v-3.2" }
        path { stroke_width: "2.25", d: "M12 3v12M7.2 10.4L12 15.2l4.8-4.8" }
    } }
}

#[component]
pub fn BuildingIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.2", y: "2.6", width: "10.8", height: "18.8", rx: "2.2", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M2.5 21.4h19M14 8.5h5.2a1.8 1.8 0 0 1 1.8 1.8v11.1M7.4 7.4h2.4M7.4 11.4h2.4M7.4 15.4h2.4M17.2 12.8h.6M17.2 16.4h.6" }
    } }
}

#[component]
pub fn BriefcaseIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.5", y: "6.8", width: "19", height: "13.8", rx: "3.2", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M8.2 6.8V5a2 2 0 0 1 2-2h3.6a2 2 0 0 1 2 2v1.8M2.5 12.6h19" }
    } }
}

#[component]
pub fn ListChecksIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.8", y: "3.2", width: "18.4", height: "17.6", rx: "3.4", fill: "currentColor", fill_opacity: "0.2" }
        path { d: "M6.4 9.2l1.6 1.6 2.8-3.2M6.4 15.2l1.6 1.6 2.8-3.2M13.2 9.2h4.6M13.2 15.2h4.6" }
    } }
}

#[component]
pub fn CalendarIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.8", y: "4.6", width: "18.4", height: "16.4", rx: "3.2", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M2.8 9.6h18.4M7.8 2.6v3.4M16.2 2.6v3.4M7.8 13.8h2M12 13.8h2M16 13.8h.6" }
    } }
}

#[component]
pub fn FileTextIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M6 2.6h8.2l5.3 5.3v11.5a2.1 2.1 0 0 1-2.1 2.1H6a2.1 2.1 0 0 1-2.1-2.1V4.7A2.1 2.1 0 0 1 6 2.6z" }
        path { d: "M14.2 2.6v5.3h5.3M8 12.2h8M8 15.8h8M8 18.8h4.5" }
    } }
}

#[component]
pub fn ClipboardCheckIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "4.2", y: "4.2", width: "15.6", height: "17.2", rx: "3", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M9.2 4.2a2.8 2.8 0 0 1 5.6 0M8.6 13.2l2.3 2.3 4.6-4.6" }
    } }
}

#[component]
pub fn BarChartIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3", y: "12.2", width: "4.2", height: "8.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "9.9", y: "7.2", width: "4.2", height: "13.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "16.8", y: "3.2", width: "4.2", height: "17.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
    } }
}

#[component]
pub fn NetworkIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "9", y: "2.6", width: "6", height: "4.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "2.6", y: "16.6", width: "6", height: "4.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "15.4", y: "16.6", width: "6", height: "4.8", rx: "1.6", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M12 7.4V12M5.6 16.6V12h12.8v4.6" }
    } }
}

#[component]
pub fn EditIcon(#[props(default = 18)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M15 3.6l5.4 5.4L8.4 21H3v-5.4L15 3.6z" }
        path { d: "M12.4 6.2l5.4 5.4" }
    } }
}

#[component]
pub fn PlugIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", d: "M6.5 7.8h11v3.8a5.5 5.5 0 0 1-11 0V7.8z" }
        path { d: "M9.2 2.8v5M14.8 2.8v5M12 17.1v4.3" }
    } }
}

/// Official Vico AI logo downloaded from https://vico.kiyo.vn/img/favicon.svg
#[component]
pub fn VicoAiIcon(#[props(default = 18)] size: u32, #[props(default = false)] full: bool) -> Element {
    static VICO_GRAD_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let uid = use_hook(|| VICO_GRAD_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let grad_id = format!("vico-g-{uid}");
    let fill_url = format!("url(#{grad_id})");
    let rect_rx = if full { "0" } else { "7" };
    rsx! {
        svg {
            class: "svg-icon vico-ai-svg",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 32 32",
            "aria-hidden": "true",
            defs {
                linearGradient {
                    id: "{grad_id}",
                    x1: "0",
                    y1: "0",
                    x2: "1",
                    y2: "1",
                    stop { offset: "0", stop_color: "#8E74FF" }
                    stop { offset: ".55", stop_color: "#5B3FE8" }
                    stop { offset: "1", stop_color: "#4232C9" }
                }
            }
            rect { width: "32", height: "32", rx: "{rect_rx}", fill: "{fill_url}" }
            path {
                d: "M23.5 11.5 16 25",
                stroke: "#fff",
                stroke_opacity: ".7",
                stroke_width: "5.6",
                stroke_linecap: "round",
                fill: "none",
            }
            path {
                d: "M8.5 11.5 16 25",
                stroke: "#fff",
                stroke_width: "5.6",
                stroke_linecap: "round",
                fill: "none",
            }
            path {
                d: "M16 3Q16.6 7.6 20.2 8.2Q16.6 8.8 16 13.4Q15.4 8.8 11.8 8.2Q15.4 7.6 16 3Z",
                fill: "#fff",
            }
        }
    }
}

/// Threads / Social Feed ("Mới") icon
#[component]
pub fn FeedThreadsIcon(#[props(default = 22)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.5", fill: duo_fill!(), fill_opacity: "0.2", stroke: "none" }
        circle { cx: "12", cy: "12", r: "3.7" }
        path { d: "M15.7 11.2v1.6a2.9 2.9 0 0 0 5.8 0V12a9.5 9.5 0 1 0-3.6 7.45" }
    } }
}

#[component]
pub fn HeartIcon(#[props(default = 18)] size: u32, #[props(default = false)] filled: bool) -> Element {
    rsx! {
        svg {
            class: "cw-icon",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: if filled { "currentColor" } else { "none" },
            stroke: "currentColor",
            stroke_width: "2.05",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M21.2 4.4a5.7 5.7 0 0 0-8.1 0L12 5.5l-1.1-1.1a5.7 5.7 0 0 0-8.1 8.1l1.1 1.1L12 21.5l8.1-7.9 1.1-1.1a5.7 5.7 0 0 0 0-8.1z" }
        }
    }
}

#[component]
pub fn RepeatIcon(#[props(default = 16)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M17 2l4 4-4 4" }
        path { d: "M3 11v-1a4 4 0 0 1 4-4h14" }
        path { d: "M7 22l-4-4 4-4" }
        path { d: "M21 13v1a4 4 0 0 1-4 4H3" }
    } }
}

#[component]
pub fn DiceIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.8", y: "2.8", width: "18.4", height: "18.4", rx: "4.4", fill: "currentColor", fill_opacity: "0.22" }
        circle { cx: "8.2", cy: "8.2", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "15.8", cy: "8.2", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "12", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "8.2", cy: "15.8", r: "1.3", fill: "currentColor", stroke: "none" }
        circle { cx: "15.8", cy: "15.8", r: "1.3", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn CoinIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.2", fill: "currentColor", fill_opacity: "0.22" }
        circle { cx: "12", cy: "12", r: "4.8" }
        path { d: "M12 7.2v9.6" }
    } }
}

#[component]
pub fn CalcIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3.8", y: "2.2", width: "16.4", height: "19.6", rx: "3.4", fill: "currentColor", fill_opacity: "0.22" }
        rect { x: "7", y: "5.2", width: "10", height: "3.8", rx: "1.2" }
        circle { cx: "8", cy: "13", r: "1.1", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "13", r: "1.1", fill: "currentColor", stroke: "none" }
        circle { cx: "16", cy: "13", r: "1.1", fill: "currentColor", stroke: "none" }
        circle { cx: "8", cy: "17.2", r: "1.1", fill: "currentColor", stroke: "none" }
        circle { cx: "12", cy: "17.2", r: "1.1", fill: "currentColor", stroke: "none" }
        circle { cx: "16", cy: "17.2", r: "1.1", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn HelpCircleIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        circle { cx: "12", cy: "12", r: "9.4", fill: "currentColor", fill_opacity: "0.22" }
        path { d: "M9.3 9.2a2.8 2.8 0 0 1 5.4.9c0 1.9-2.7 2.4-2.7 3.9" }
        circle { cx: "12", cy: "17.4", r: "1.2", fill: "currentColor", stroke: "none" }
    } }
}

#[component]
pub fn LogInIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", stroke: "none", d: "M15 2.8h3.4a2.8 2.8 0 0 1 2.8 2.8v12.8a2.8 2.8 0 0 1-2.8 2.8H15z" }
        path { d: "M15 2.8h3.4a2.8 2.8 0 0 1 2.8 2.8v12.8a2.8 2.8 0 0 1-2.8 2.8H15" }
        path { d: "M10.2 17l5-5-5-5M15.2 12H2.8" }
    } }
}

#[component]
pub fn LogOutIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { fill: "currentColor", fill_opacity: "0.22", stroke: "none", d: "M9 2.8H5.6a2.8 2.8 0 0 0-2.8 2.8v12.8a2.8 2.8 0 0 0 2.8 2.8H9z" }
        path { d: "M9 21.2H5.6a2.8 2.8 0 0 1-2.8-2.8V5.6a2.8 2.8 0 0 1 2.8-2.8H9" }
        path { d: "M16.2 17l5-5-5-5M21.2 12H8.8" }
    } }
}

#[component]
pub fn BoltIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        polygon { points: "13.2 2.2 4.6 13.4 11.4 13.4 10.2 21.8 19.4 10.6 12.6 10.6", fill: "currentColor", fill_opacity: "0.24" }
    } }
}

#[component]
pub fn ArchiveIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "2.4", y: "3.4", width: "19.2", height: "4.6", rx: "1.6", fill: duo_fill!(), fill_opacity: "0.22" }
        path { fill: duo_fill!(), fill_opacity: "0.14", stroke: "none", d: "M4 8h16v10.2a2.4 2.4 0 0 1-2.4 2.4H6.4A2.4 2.4 0 0 1 4 18.2z" }
        path { d: "M4 8v10.2a2.4 2.4 0 0 0 2.4 2.4h11.2a2.4 2.4 0 0 0 2.4-2.4V8" }
        path { d: "M9.8 13.2h4.4" }
    } }
}

#[component]
pub fn CloudIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        path { d: "M17.8 19.4H6.8a4.7 4.7 0 0 1-.4-9.38A5.7 5.7 0 0 1 17.6 8.8a4.2 4.2 0 0 1 .2 10.6Z", fill: duo_fill!(), fill_opacity: "0.22" }
    } }
}

/// Clean single-glyph Google Play Store logo mark.
#[component]
pub fn PlayStoreLogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            path { fill: "#4285F4", d: "M3.6 2.4a1.4 1.4 0 0 0-.6 1.15v16.9c0 .48.22.9.6 1.15l.1.06 9.48-9.48v-.36L3.7 2.34l-.1.06z" }
            path { fill: "#34A853", d: "M16.34 15.34l-3.16-3.16v-.36l3.16-3.16.14.08 3.74 2.12c1.07.6 1.07 1.6 0 2.2l-3.74 2.13-.14.15z" }
            path { fill: "#FBBC05", d: "M16.48 15.19L13.18 11.9 3.6 21.48c.35.37.94.42 1.62.03l11.26-6.32z" }
            path { fill: "#EA4335", d: "M16.48 8.81L5.22 2.49C4.54 2.1 3.95 2.15 3.6 2.52L13.18 12.1l3.3-3.29z" }
        }
    }
}

/// Clean single-glyph Apple App Store icon mark.
#[component]
pub fn AppStoreLogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            rect { x: "2", y: "2", width: "20", height: "20", rx: "5.2", fill: "#0A84FF" }
            path { d: "M11.6 6.2l-4.5 7.8M12.4 6.2l4.5 7.8M6.2 15.2h11.6M8.2 16.4l-.8 1.4M15.8 16.4l.8 1.4", stroke: "#FFFFFF", stroke_width: "1.8", stroke_linecap: "round" }
        }
    }
}

#[component]
pub fn WindowsLogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "#0078D4",
            xmlns: "http://www.w3.org/2000/svg",
            path { d: "M3 5.2l8-1.1v7.4H3V5.2zm9-1.24L21 2.7v8.8h-9V3.96zM3 12.5h8v7.4l-8-1.1v-6.3zm9 0h9v8.8l-9-1.26V12.5z" }
        }
    }
}

#[component]
pub fn MacLogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "currentColor",
            xmlns: "http://www.w3.org/2000/svg",
            path { d: "M16.2 12.7c0-2.1 1.7-3.1 1.8-3.2-1-1.4-2.5-1.6-3-1.6-1.3-.1-2.5.8-3.1.8s-1.6-.8-2.7-.7c-1.4 0-2.7.8-3.4 2.1-1.5 2.5-.4 6.3 1 8.4.7 1 1.5 2.1 2.6 2.1 1-.1 1.4-.7 2.7-.7s1.6.7 2.7.6c1.1 0 1.8-1 2.5-2 .8-1.2 1.1-2.3 1.1-2.4-.1 0-2.2-.8-2.2-3.4zM14.1 6.4c.6-.7 1-1.7.9-2.7-.8 0-1.9.6-2.5 1.3-.5.6-1 1.6-.9 2.6.9.1 1.9-.5 2.5-1.2z" }
        }
    }
}

#[component]
pub fn LinuxLogoIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! { Duo { size,
        rect { x: "3", y: "4", width: "18", height: "16", rx: "3", fill: duo_fill!(), fill_opacity: "0.14" }
        path { d: "M7.5 9.5l3 2.5-3 2.5M12.5 14.5h4" }
    } }
}

#[component]
pub fn AndroidIcon(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            path { d: "M6.5 11a5.5 5.5 0 0 1 11 0v6.5a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2V11z", fill: "#34A853" }
            path { d: "M9 6.2L7.4 3.8M15 6.2l1.6-2.4", stroke: "#34A853", stroke_width: "1.8", stroke_linecap: "round" }
            circle { cx: "10", cy: "9.2", r: "0.9", fill: "#FFFFFF" }
            circle { cx: "14", cy: "9.2", r: "0.9", fill: "#FFFFFF" }
        }
    }
}
