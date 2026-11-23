//! Per-account UI preferences stored on this device and shared through a
//! Dioxus context so the rail, chat and settings react immediately.

use serde::{Deserialize, Serialize};

use crate::platform;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Show joined communities as shortcuts in the left rail.
    pub pin_communities: bool,
    /// Show group chats as shortcuts in the left rail.
    pub pin_groups: bool,
    /// Default chat background (preset id) for chats without a shared wallpaper.
    pub wallpaper: String,
    /// Play a sound for new messages.
    pub sound: bool,
    /// Render animated profile effects.
    pub profile_effects: bool,

    // Chat behaviour
    /// Double-click the empty area beside a message to reply.
    pub double_click_reply: bool,
    pub sticker_suggest: bool,
    pub mention_suggest: bool,
    pub emoji_suggest: bool,
    pub enter_to_send: bool,
    /// Spreadsheet paste: ask | image | table | text
    pub paste_mode: String,
    pub show_link_previews: bool,

    // Notifications
    pub notify: bool,
    /// Show message text in system notifications (off: sender only).
    pub notify_preview: bool,
    pub notify_groups: bool,
    pub notify_calls: bool,
    /// pop | chime | soft | bubble
    pub sound_name: String,

    // App lock (PIN). The PIN itself is never stored: only a salted PBKDF2 hash.
    pub lock_enabled: bool,
    /// Lock after this many minutes without activity.
    pub lock_minutes: u32,
    /// Ask for the PIN every time ChatW is opened.
    pub lock_on_start: bool,
    pub pin_salt: String,
    pub pin_hash: String,

    // Auto reply (sent from this device while ChatW is open; E2EE keeps the server blind)
    pub auto_reply: bool,
    pub auto_reply_text: String,
    /// always | away | hours
    pub auto_reply_mode: String,
    pub auto_reply_from: String,
    pub auto_reply_to: String,

    // Appearance customisation
    /// "" (theme accent) or #RRGGBB
    pub custom_accent: String,
    pub bubble_self: String,
    pub bubble_other: String,
    /// round | soft | square | leaf
    pub bubble_shape: String,
    /// circle | squircle | square
    pub avatar_shape: String,
    /// narrow | normal | wide | full
    pub chat_width: String,
    /// system | rounded | serif | mono | dyslexic
    pub font_family: String,
    /// App background: "" | preset id | /uploads/media/... | color:#RRGGBB
    pub app_wallpaper: String,
    /// Dim percentage over the app wallpaper (0-80)
    pub app_wallpaper_dim: u8,
    /// Blur in px for the app wallpaper (0-24)
    pub app_wallpaper_blur: u8,
    /// Opacity percentage for white/surface panels (0-100; 100 = solid opaque)
    pub panel_opacity: u8,
    /// Whether frosted glass blur is enabled on translucent panels (default false)
    pub panel_glass: bool,
    /// Backdrop glass blur in px for translucent panels (0-32)
    pub panel_blur: u8,
    pub show_avatars: bool,
    pub time_24h: bool,
    pub reduce_motion: bool,
    /// UI language (BCP-47)
    pub language: String,

    // Spec 009: library favourites / recents (English names, synced across devices)
    pub fav_stickers: Vec<String>,
    pub recent_stickers: Vec<String>,
    pub fav_gifs: Vec<String>,
    pub fav_emoji: Vec<String>,
    /// Avatar motion in lists (conversations, friends): always | hover | off
    pub avatar_fx_lists: String,
    /// Sync these preferences to my other devices (the PIN lock never leaves the device).
    pub sync_settings: bool,
    /// Spec 067: Show the Developer menu in Settings with advanced/technical tools (device-local).
    pub developer_mode: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            pin_communities: false,
            pin_groups: false,
            wallpaper: "none".to_string(),
            sound: true,
            profile_effects: true,
            double_click_reply: true,
            sticker_suggest: true,
            mention_suggest: true,
            emoji_suggest: true,
            enter_to_send: true,
            paste_mode: "ask".to_string(),
            show_link_previews: true,
            notify: true,
            notify_preview: false,
            notify_groups: true,
            notify_calls: true,
            sound_name: "pop".to_string(),
            lock_enabled: false,
            lock_minutes: 5,
            lock_on_start: false,
            pin_salt: String::new(),
            pin_hash: String::new(),
            auto_reply: false,
            auto_reply_text: "I am currently busy and will reply as soon as possible.".to_string(),
            auto_reply_mode: "away".to_string(),
            auto_reply_from: "18:00".to_string(),
            auto_reply_to: "08:00".to_string(),
            custom_accent: String::new(),
            bubble_self: String::new(),
            bubble_other: String::new(),
            bubble_shape: "round".to_string(),
            avatar_shape: "circle".to_string(),
            chat_width: "normal".to_string(),
            font_family: "system".to_string(),
            app_wallpaper: String::new(),
            app_wallpaper_dim: 0,
            app_wallpaper_blur: 0,
            panel_opacity: 100,
            panel_glass: false,
            panel_blur: 12,
            show_avatars: true,
            time_24h: true,
            reduce_motion: false,
            language: crate::i18n::detect_default_language().to_string(),
            fav_stickers: Vec::new(),
            recent_stickers: Vec::new(),
            fav_gifs: Vec::new(),
            fav_emoji: Vec::new(),
            avatar_fx_lists: "hover".to_string(),
            sync_settings: true,
            developer_mode: false,
        }
    }
}

/// Never synced: device-local security lock and developer mode stay on this device only.
pub const DEVICE_ONLY: [&str; 6] = ["pin_salt", "pin_hash", "lock_enabled", "lock_minutes", "lock_on_start", "developer_mode"];

fn dedupe(list: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    list.retain(|x| seen.insert(x.clone()));
}

/// Sticker / GIF paths we serve ourselves.
pub fn valid_media_path(u: &str) -> bool {
    (u.starts_with("/stickers/") || u.starts_with("/uploads/"))
        && u.len() <= 200
        && !u.contains("..")
        && u.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

pub const WALLPAPERS: [&str; 9] = ["none", "dots", "bubbles", "aurora", "paper", "sunset", "ocean", "mint", "night"];

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Accept only values the CSS layer can safely interpolate.
pub fn valid_background(v: &str) -> bool {
    v.is_empty()
        || WALLPAPERS.contains(&v)
        || v.strip_prefix("color:").is_some_and(is_hex_color)
        || (v.starts_with("/uploads/media/") && v[15..].chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'))
}

impl Prefs {
    fn key(user_id: &str) -> String {
        format!("chatw_prefs_{user_id}")
    }

    pub fn load(user_id: &str) -> Self {
        let mut p: Prefs = platform::storage_get(&Self::key(user_id))
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        p.sanitize();
        p
    }

    fn sanitize(&mut self) {
        let d = Prefs::default();
        if !WALLPAPERS.contains(&self.wallpaper.as_str()) {
            self.wallpaper = d.wallpaper.clone();
        }
        let pick = |v: &mut String, allowed: &[&str], fallback: &str| {
            if !allowed.contains(&v.as_str()) {
                *v = fallback.to_string();
            }
        };
        pick(&mut self.paste_mode, &["ask", "image", "table", "text"], "ask");
        pick(&mut self.sound_name, &["pop", "chime", "soft", "bubble"], "pop");
        pick(&mut self.auto_reply_mode, &["always", "away", "hours"], "away");
        pick(&mut self.bubble_shape, &["round", "soft", "square", "leaf"], "round");
        pick(&mut self.avatar_shape, &["circle", "squircle", "square"], "circle");
        pick(&mut self.chat_width, &["narrow", "normal", "wide", "full"], "normal");
        pick(
            &mut self.font_family,
            &[
                "system", "lgbt", "minecraft", "modern", "rounded", "geometric",
                "humanist", "expressive", "serif", "classic", "slab", "mono",
                "pixel", "futuristic", "handwriting", "calligraphy", "dyslexic",
            ],
            "system",
        );
        for c in [&mut self.custom_accent, &mut self.bubble_self, &mut self.bubble_other] {
            if !c.is_empty() && !is_hex_color(c) {
                c.clear();
            }
        }
        if !valid_background(&self.app_wallpaper) {
            self.app_wallpaper.clear();
        }
        self.app_wallpaper_dim = self.app_wallpaper_dim.min(80);
        self.app_wallpaper_blur = self.app_wallpaper_blur.min(24);
        self.panel_opacity = self.panel_opacity.min(100);
        self.panel_blur = self.panel_blur.min(32);
        self.lock_minutes = self.lock_minutes.clamp(1, 240);
        if self.auto_reply_text.chars().count() > 500 {
            self.auto_reply_text = self.auto_reply_text.chars().take(500).collect();
        }
        if crate::i18n::lang_index(&self.language).is_none() {
            self.language = d.language;
        }
        pick(&mut self.avatar_fx_lists, &["always", "hover", "off"], "hover");
        for (list, max) in [(&mut self.fav_stickers, 60), (&mut self.recent_stickers, 24), (&mut self.fav_gifs, 60)] {
            list.retain(|u| valid_media_path(u));
            dedupe(list);
            list.truncate(max);
        }
        self.fav_emoji.retain(|e| !e.is_empty() && e.len() <= 32 && !e.chars().any(|c| c.is_ascii() && !c.is_ascii_digit() && c != '#' && c != '*'));
        dedupe(&mut self.fav_emoji);
        self.fav_emoji.truncate(40);
    }

    /// Preferences that may leave this device (the PIN lock never does).
    pub fn sync_payload(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut m = match serde_json::to_value(self) {
            Ok(serde_json::Value::Object(m)) => m,
            _ => serde_json::Map::new(),
        };
        for k in DEVICE_ONLY {
            m.remove(k);
        }
        m
    }

    /// Apply preferences from another device, keeping this device's own PIN lock.
    pub fn merged_with(&self, remote: &serde_json::Map<String, serde_json::Value>) -> Prefs {
        let mut mine = match serde_json::to_value(self) {
            Ok(serde_json::Value::Object(m)) => m,
            _ => return self.clone(),
        };
        for (k, v) in remote {
            if !DEVICE_ONLY.contains(&k.as_str()) && mine.contains_key(k) {
                mine.insert(k.clone(), v.clone());
            }
        }
        let mut p: Prefs = serde_json::from_value(serde_json::Value::Object(mine)).unwrap_or_else(|_| self.clone());
        p.sanitize();
        p
    }

    pub fn sanitize_pub(&mut self) {
        self.sanitize();
    }

    pub fn save(&self, user_id: &str) {
        if let Ok(json) = serde_json::to_string(self) {
            platform::storage_set(&Self::key(user_id), &json);
        }
    }
}

/// Context value: the current user id plus the live preferences signal.
#[derive(Clone, Copy)]
pub struct PrefsCtx {
    pub prefs: dioxus::prelude::Signal<Prefs>,
    pub user_id: dioxus::prelude::Signal<String>,
}

impl PrefsCtx {
    pub fn update(mut self, f: impl FnOnce(&mut Prefs)) {
        use dioxus::prelude::*;
        let mut p = self.prefs.read().clone();
        f(&mut p);
        p.sanitize();
        p.save(&self.user_id.read());
        crate::platform::apply_customization(&p);
        crate::i18n::set_language(&p.language);
        let sync = p.sync_settings;
        self.prefs.set(p);
        if sync {
            crate::services::settings_sync::schedule_push(&self);
        }
    }

    /// Replace preferences with ones that came from another device (no push back).
    pub fn apply_remote(mut self, p: Prefs) {
        use dioxus::prelude::*;
        p.save(&self.user_id.read());
        crate::platform::apply_customization(&p);
        crate::i18n::set_language(&p.language);
        self.prefs.set(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_never_carries_the_pin_lock() {
        let mut p = Prefs::default();
        p.pin_hash = "h".into();
        p.lock_enabled = true;
        p.developer_mode = true;
        let payload = p.sync_payload();
        assert!(!payload.contains_key("pin_hash") && !payload.contains_key("lock_enabled") && !payload.contains_key("developer_mode"));
        assert!(payload.contains_key("fav_stickers"));
        let mut remote = payload.clone();
        remote.insert("pin_hash".into(), serde_json::json!("evil"));
        remote.insert("lock_enabled".into(), serde_json::json!(false));
        remote.insert("custom_accent".into(), serde_json::json!("#112233"));
        remote.insert("fav_stickers".into(), serde_json::json!(["/stickers/openmoji/1F600.svg", "javascript:x", "/stickers/../x"]));
        let merged = p.merged_with(&remote);
        assert_eq!(merged.pin_hash, "h");
        assert!(merged.lock_enabled);
        assert_eq!(merged.custom_accent, "#112233");
        assert_eq!(merged.fav_stickers, vec!["/stickers/openmoji/1F600.svg".to_string()]);
    }

    #[test]
    fn spec084_font_family_sanitizes_all_17_fonts_including_lgbt_and_minecraft() {
        let valid = [
            "system", "lgbt", "minecraft", "modern", "rounded", "geometric",
            "humanist", "expressive", "serif", "classic", "slab", "mono",
            "pixel", "futuristic", "handwriting", "calligraphy", "dyslexic",
        ];
        for f in valid {
            let mut p = Prefs::default();
            p.font_family = f.to_string();
            p.sanitize();
            assert_eq!(p.font_family, f);
        }
        let mut bad = Prefs::default();
        bad.font_family = "unknown-injected-font".to_string();
        bad.sanitize();
        assert_eq!(bad.font_family, "system");
    }
}
