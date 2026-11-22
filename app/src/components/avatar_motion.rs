//! Avatar Motion Studio: 40 declarative avatar effects with speed, intensity,
//! colour, direction, easing, trigger and repeat count. A motion is plain data
//! (validated again by the server) rendered with CSS classes and variables, so
//! another user's profile can never run code in this app.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::avatar::Avatar;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MotionKind {
    /// Moves / filters the avatar itself.
    Transform,
    /// A ring or overlay around / over the avatar.
    Ring,
    /// Small particles around the avatar.
    Particles,
}

use MotionKind::{Particles, Ring, Transform};

/// (id, English name, kind, base seconds per cycle at speed 1).
pub const MOTIONS: [(&str, &str, MotionKind, f32); 40] = [
    ("spin", "Spin", Transform, 6.0),
    ("pulse", "Pulse", Transform, 1.8),
    ("breathe", "Breathe", Transform, 4.0),
    ("bounce", "Bounce", Transform, 1.6),
    ("float", "Float", Transform, 3.0),
    ("shake", "Shake", Transform, 0.9),
    ("nod", "Nod", Transform, 1.6),
    ("tilt", "Tilt", Transform, 2.4),
    ("jitter", "Jitter", Transform, 0.5),
    ("wobble", "Wobble", Transform, 2.4),
    ("zoom", "Zoom", Transform, 2.2),
    ("swing", "Swing", Transform, 2.0),
    ("flip", "Flip", Transform, 3.0),
    ("glow", "Glow", Ring, 2.2),
    ("flash", "Flash", Transform, 1.6),
    ("ring-cycle", "Color Ring", Ring, 4.0),
    ("hue", "Hue Shift", Transform, 5.0),
    ("rainbow", "Rainbow Ring", Ring, 4.0),
    ("gradient-ring", "Gradient Ring", Ring, 3.0),
    ("halo", "Halo", Ring, 2.8),
    ("orbit", "Orbit", Particles, 3.5),
    ("sparkle", "Sparkle", Particles, 2.4),
    ("stars", "Stars", Particles, 3.0),
    ("hearts", "Hearts", Particles, 3.2),
    ("snow", "Snow", Particles, 4.0),
    ("rain", "Rain", Particles, 1.2),
    ("fire", "Fire", Particles, 1.6),
    ("electric", "Electric", Ring, 1.2),
    ("smoke", "Smoke", Particles, 4.0),
    ("bubbles", "Bubbles", Particles, 3.6),
    ("ripple", "Ripple", Ring, 2.2),
    ("wave", "Wave", Transform, 2.0),
    ("heartbeat", "Heartbeat", Transform, 1.4),
    ("hologram", "Hologram", Transform, 3.0),
    ("glitch", "Glitch", Transform, 2.6),
    ("pixel", "Pixel", Transform, 2.0),
    ("scanline", "Scanline", Ring, 2.0),
    ("shine", "Shine", Ring, 2.6),
    ("confetti", "Confetti", Particles, 3.0),
    ("morph", "Morph", Transform, 5.0),
];

pub const TRIGGERS: [(&str, &str); 3] = [("always", "Always"), ("hover", "On hover"), ("click", "On click")];
pub const EASINGS: [(&str, &str); 5] =
    [("linear", "All"), ("ease", "Smooth"), ("ease-in-out", "Smooth both ends"), ("ease-out", "Slow down"), ("bounce", "Bouncing")];

fn d_speed() -> f64 {
    1.0
}
fn d_intensity() -> u8 {
    60
}
fn d_trigger() -> String {
    "always".into()
}
fn d_easing() -> String {
    "linear".into()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AvatarMotion {
    pub effect: String,
    #[serde(default = "d_speed")]
    pub speed: f64,
    #[serde(default = "d_intensity")]
    pub intensity: u8,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default = "d_trigger")]
    pub trigger: String,
    #[serde(default = "d_easing")]
    pub easing: String,
    #[serde(default)]
    pub repeat: u8,
}

impl Default for AvatarMotion {
    fn default() -> Self {
        Self::of("spin")
    }
}

fn is_hex(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

impl AvatarMotion {
    pub fn of(effect: &str) -> Self {
        Self {
            effect: effect.to_string(),
            speed: 1.0,
            intensity: 60,
            color: None,
            reverse: false,
            trigger: "always".into(),
            easing: "linear".into(),
            repeat: 0,
        }
    }

    pub fn meta(&self) -> Option<(&'static str, &'static str, MotionKind, f32)> {
        MOTIONS.iter().copied().find(|m| m.0 == self.effect)
    }

    /// Same rules as the server; anything else is not rendered.
    pub fn is_valid(&self) -> bool {
        self.meta().is_some()
            && (0.25..=4.0).contains(&self.speed)
            && (1..=100).contains(&self.intensity)
            && self.color.as_deref().is_none_or(is_hex)
            && TRIGGERS.iter().any(|t| t.0 == self.trigger)
            && EASINGS.iter().any(|e| e.0 == self.easing)
            && self.repeat <= 10
    }

    /// (classes, inline CSS variables) for the decoration wrapper.
    pub fn render_attrs(&self, force_hover: bool) -> (String, String) {
        let Some((id, _, kind, base)) = self.meta().filter(|_| self.is_valid()) else {
            return (String::new(), String::new());
        };
        let kind = match kind {
            Transform => "transform",
            Ring => "ring",
            Particles => "particles",
        };
        let trigger = if force_hover && self.trigger == "always" { "hover" } else { self.trigger.as_str() };
        let dur = (base as f64 / self.speed).clamp(0.15, 30.0);
        let ease = if self.easing == "bounce" { "cubic-bezier(.34,1.56,.64,1)".to_string() } else { self.easing.clone() };
        let iter = if self.repeat == 0 { "infinite".to_string() } else { self.repeat.to_string() };
        let mut style = format!(
            "--avm-anim: avm-{id}; --avm-dur: {dur:.2}s; --avm-int: {:.2}; --avm-dir: {}; --avm-ease: {ease}; --avm-iter: {iter};",
            self.intensity as f64 / 100.0,
            if self.reverse { "reverse" } else { "normal" },
        );
        if let Some(c) = self.color.as_deref().filter(|c| is_hex(c)) {
            style.push_str(&format!(" --avm-color: {c};"));
        }
        (format!("avm avm-{id} avm-k-{kind} avm-t-{trigger}"), style)
    }
}

/// Map a legacy one-word avatar effect to a motion.
pub fn legacy_motion(effect: Option<&str>) -> Option<AvatarMotion> {
    effect.filter(|e| !e.is_empty() && *e != "none").map(AvatarMotion::of).filter(|m| m.is_valid())
}

/// Ring / particle layers drawn over the avatar for the given motion.
#[component]
pub fn MotionLayers(kind: String) -> Element {
    rsx! {
        if kind == "ring" {
            span { class: "avm-ring", aria_hidden: "true" }
        } else if kind == "particles" {
            span { class: "avm-fx", aria_hidden: "true", i {} i {} i {} i {} i {} i {} }
        }
    }
}

/// A standalone animated avatar (studio previews and the Demo Gallery).
#[component]
pub fn MotionAvatar(name: String, avatar_url: Option<String>, size: u32, motion: AvatarMotion) -> Element {
    let (cls, style) = motion.render_attrs(false);
    let kind = cls.split(' ').find_map(|c| c.strip_prefix("avm-k-")).unwrap_or_default().to_string();
    let mut on = use_signal(|| false);
    rsx! {
        span { class: if on() { "deco-wrap {cls} avm-on" } else { "deco-wrap {cls}" }, style: "--deco-size: {size}px; {style}",
            onclick: move |_| on.set(!on()),
            span { class: "avatar-motion", Avatar { name, avatar_url, size } }
            MotionLayers { kind }
        }
    }
}

/// Full editor: effect grid, parameters and a live preview.
#[component]
pub fn AvatarStudio(
    name: String,
    avatar_url: Option<String>,
    value: Option<AvatarMotion>,
    on_change: EventHandler<Option<AvatarMotion>>,
) -> Element {
    let current = value.clone();
    let m = current.clone().unwrap_or_default();
    let set = move |f: &dyn Fn(&mut AvatarMotion)| {
        let mut next = value.clone().unwrap_or_default();
        f(&mut next);
        on_change.call(Some(next));
    };
    let set1 = set.clone();
    let set2 = set.clone();
    let set3 = set.clone();
    let set4 = set.clone();
    let set5 = set.clone();
    let set6 = set.clone();
    let set7 = set.clone();
    let set8 = set.clone();
    rsx! {
        div { class: "avatar-studio",
            div { class: "studio-preview",
                if let Some(mm) = current.clone() {
                    MotionAvatar { key: "{mm.effect}-{mm.trigger}", name: name.clone(), avatar_url: avatar_url.clone(), size: 112, motion: mm.clone() }
                    div { class: "studio-preview-name", "{mm.meta().map(|x| x.1).unwrap_or_default()}" }
                    if mm.trigger != "always" {
                        div { class: "member-sub", if mm.trigger == "hover" { "Hover over the photo to view" } else { "Click on the photo to turn it on/off" } }
                    }
                } else {
                    Avatar { name: name.clone(), avatar_url: avatar_url.clone(), size: 112 }
                    div { class: "studio-preview-name", "No effect" }
                }
            }
            div { class: "studio-controls",
                div { class: "settings-group-title", "Effects" }
                div { class: "motion-grid", role: "radiogroup", aria_label: "Avatar effects",
                    button { r#type: "button", role: "radio", aria_checked: "{current.is_none()}",
                        class: if current.is_none() { "motion-tile active" } else { "motion-tile" },
                        onclick: move |_| on_change.call(None),
                        span { class: "motion-none", "∅" }
                        span { class: "motion-name", "None" }
                    }
                    for (id, label, _, _) in MOTIONS {
                        {
                            let active = current.as_ref().is_some_and(|c| c.effect == id);
                            let mut tile = AvatarMotion::of(id);
                            tile.trigger = "hover".into();
                            tile.color = m.color.clone();
                            let set = set.clone();
                            rsx! {
                                button { key: "{id}", r#type: "button", role: "radio", aria_checked: "{active}", title: "{label}",
                                    class: if active { "motion-tile active" } else { "motion-tile" },
                                    onclick: move |_| set(&|x: &mut AvatarMotion| x.effect = id.to_string()),
                                    MotionAvatar { name: name.clone(), avatar_url: avatar_url.clone(), size: 40, motion: tile }
                                    span { class: "motion-name", "{label}" }
                                }
                            }
                        }
                    }
                }
                if current.is_some() {
                    div { class: "studio-params",
                        label { class: "range-row",
                            span { "Speed" }
                            input { r#type: "range", min: "0.25", max: "4", step: "0.25", value: "{m.speed}", aria_label: "Speed",
                                oninput: move |e| { let v = e.value().parse::<f64>().unwrap_or(1.0); set1(&|x: &mut AvatarMotion| x.speed = v.clamp(0.25, 4.0)) } }
                            span { class: "range-value", "{m.speed:.2}×" }
                        }
                        label { class: "range-row",
                            span { "Intensity" }
                            input { r#type: "range", min: "1", max: "100", value: "{m.intensity}", aria_label: "Intensity",
                                oninput: move |e| { let v = e.value().parse::<u8>().unwrap_or(60); set2(&|x: &mut AvatarMotion| x.intensity = v.clamp(1, 100)) } }
                            span { class: "range-value", "{m.intensity}%" }
                        }
                        div { class: "range-row",
                            span { "Color" }
                            input { r#type: "color", aria_label: "Effect color", value: "{m.color.clone().unwrap_or_else(|| \"#2764E7\".into())}",
                                oninput: move |e| { let v = e.value().to_uppercase(); set3(&|x: &mut AvatarMotion| x.color = Some(v.clone())) } }
                            button { r#type: "button", class: "btn-small-secondary", onclick: move |_| set4(&|x: &mut AvatarMotion| x.color = None), "According to profile color" }
                        }
                        div { class: "range-row",
                            span { "Direction" }
                            div { class: "segmented", role: "radiogroup", aria_label: "Direction",
                                button { r#type: "button", role: "radio", aria_checked: "{!m.reverse}", class: if !m.reverse { "active" } else { "" },
                                    onclick: { let s = set5.clone(); move |_| s(&|x: &mut AvatarMotion| x.reverse = false) }, "Clockwise" }
                                button { r#type: "button", role: "radio", aria_checked: "{m.reverse}", class: if m.reverse { "active" } else { "" },
                                    onclick: move |_| set5(&|x: &mut AvatarMotion| x.reverse = true), "Opposite direction" }
                            }
                        }
                        div { class: "range-row",
                            span { "Activate" }
                            div { class: "segmented", role: "radiogroup", aria_label: "Activate",
                                for (tid, tl) in TRIGGERS {
                                    {
                                        let s = set6.clone();
                                        rsx! {
                                            button { key: "{tid}", r#type: "button", role: "radio", aria_checked: "{m.trigger == tid}", class: if m.trigger == tid { "active" } else { "" },
                                                onclick: move |_| s(&|x: &mut AvatarMotion| x.trigger = tid.to_string()), "{tl}" }
                                        }
                                    }
                                }
                            }
                        }
                        label { class: "range-row",
                            span { "Movement type" }
                            select { aria_label: "Movement type", value: "{m.easing}",
                                onchange: move |e| { let v = e.value(); set7(&|x: &mut AvatarMotion| x.easing = v.clone()) },
                                for (eid, el) in EASINGS { option { key: "{eid}", value: "{eid}", selected: m.easing == eid, "{el}" } }
                            }
                        }
                        label { class: "range-row",
                            span { "Number of iterations" }
                            input { r#type: "range", min: "0", max: "10", value: "{m.repeat}", aria_label: "Number of iterations",
                                oninput: move |e| { let v = e.value().parse::<u8>().unwrap_or(0); set8(&|x: &mut AvatarMotion| x.repeat = v.min(10)) } }
                            span { class: "range-value", if m.repeat == 0 { "Forever" } else { "{m.repeat} times" } }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_catalogue_matches_server_rules() {
        assert_eq!(MOTIONS.len(), 40);
        let mut ids: Vec<&str> = MOTIONS.iter().map(|m| m.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 40);
        let m = AvatarMotion { speed: 2.0, reverse: true, color: Some("#FF0000".into()), ..AvatarMotion::of("orbit") };
        let (cls, style) = m.render_attrs(false);
        assert!(cls.contains("avm-orbit") && cls.contains("avm-k-particles") && cls.contains("avm-t-always"));
        assert!(style.contains("--avm-dur: 1.75s") && style.contains("reverse") && style.contains("#FF0000"));
        // Lists can force hover-only playback.
        assert!(m.render_attrs(true).0.contains("avm-t-hover"));
        // Invalid values never reach CSS.
        let bad = AvatarMotion { color: Some("red;}".into()), ..AvatarMotion::of("spin") };
        assert_eq!(bad.render_attrs(false), (String::new(), String::new()));
        assert!(legacy_motion(Some("wobble")).is_some() && legacy_motion(Some("none")).is_none());
    }
}
