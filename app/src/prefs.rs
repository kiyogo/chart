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
    /// Chat background: none | dots | bubbles | aurora | paper
    pub wallpaper: String,
    /// Play a sound for new messages.
    pub sound: bool,
    /// Render animated profile effects.
    pub profile_effects: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            pin_communities: false,
            pin_groups: false,
            wallpaper: "none".to_string(),
            sound: true,
            profile_effects: true,
        }
    }
}

const WALLPAPERS: [&str; 5] = ["none", "dots", "bubbles", "aurora", "paper"];

impl Prefs {
    fn key(user_id: &str) -> String {
        format!("chatw_prefs_{user_id}")
    }

    pub fn load(user_id: &str) -> Self {
        let mut p: Prefs = platform::storage_get(&Self::key(user_id))
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        if !WALLPAPERS.contains(&p.wallpaper.as_str()) {
            p.wallpaper = "none".to_string();
        }
        p
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
        p.save(&self.user_id.read());
        self.prefs.set(p);
    }
}
