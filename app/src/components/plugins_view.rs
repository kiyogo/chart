//! ChatW Plugin & Mod System (Minecraft-Mod-style architecture):
//! Every Bot is now a Plugin, and any Plugin can:
//! 1. Act as an interactive Chat Bot (local slash commands, auto-replies, and server HTTP Token API).
//! 2. Transform and customize the entire application's UI (theme tokens, pixel/neon/glass borders,
//!    layout & rail position, corner radius, blur, fonts, and arbitrary live Custom CSS).
//! 3. Add brand-new UI surfaces to the app (custom interactive pages on the left Navigation Rail,
//!    floating HUD widgets, full-screen particle effects, synthesized sound packs, and composer buttons).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        chat_extras::wallpaper_attrs,
        icons::{
            BackIcon, BoltIcon, BotIcon, BuildingIcon, CalcIcon, CalendarIcon,
            ClipboardCheckIcon, CloseIcon, CoinIcon, CompassIcon, CopyIcon, DiceIcon,
            GlobeIcon, GroupIcon, HelpCircleIcon, ListChecksIcon, LogInIcon,
            LogOutIcon, MegaphoneIcon, PanelRightIcon, PluginIcon, PlusIcon,
            SendIcon, SlidersIcon, TrashIcon, TypeIcon, VicoAiIcon, VideoIcon,
        },
        messages::{Composer, MessageList, ViewMessage},
    },
    models::{BotChatMessage, BotItem, GroupItem},
    platform,
    prefs::PrefsCtx,
    services::social,
};

const STORAGE_KEY: &str = "chatw_plugins_v2";
const STORE_MIGRATED_KEY: &str = "chatw_plugin_store_v1";
const MODULAR_MIGRATED_KEY: &str = "chatw_plugins_modular_v3";
pub(crate) const NOTES_KEY: &str = "chatw_plugin_notes_v1";
pub(crate) const TODOS_KEY: &str = "chatw_plugin_todos_v1";
pub(crate) const VOXEL_KEY: &str = "chatw_plugin_voxel_v1";

fn default_true() -> bool {
    true
}

fn default_hundred() -> i32 {
    100
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PluginSettingDef {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub kind: String, // "toggle" | "slider" | "select" | "text" | "color"
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub min: i32,
    #[serde(default = "default_hundred")]
    pub max: i32,
    #[serde(default)]
    pub default_value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginCommand {
    pub trigger: String,
    #[serde(default)]
    pub description: String,
    pub response: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginComposerBtn {
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PluginCustomBlock {
    #[serde(default)]
    pub kind: String, // "card" | "button" | "counter" | "checklist" | "input_action" | "timer" | "link"
    pub title: String,
    pub body: String,
    pub action_label: String,
    pub action_command: String,
    #[serde(default)]
    pub action_type: String, // "command" | "insert_chat" | "copy" | "sound" | "url" | "webhook"
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PluginTheme {
    pub bg_app: String,
    pub bg_sidebar: String,
    pub bg_chat: String,
    pub bg_surface: String,
    pub accent: String,
    pub msg_sent: String,
    pub msg_recv: String,
    pub text_primary: String,
    pub panel_radius: String,
    pub border_style: String, // "default" | "pixel" | "neon" | "glass"
    pub glass_blur: u8,
    pub rail_position: String, // "left" | "right" | "top"
    pub font_style: String,    // "default" | "pixel" | "rounded" | "serif"
    pub custom_css: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginMod {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub author: String,
    pub version: String,
    pub icon: String,
    #[serde(default)]
    pub logo_url: String,
    #[serde(default)]
    pub default_logo_url: String,
    #[serde(default = "default_true")]
    pub allow_custom_logo: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub installed: bool,
    pub enabled: bool,
    pub builtin: bool,
    pub server_bot_id: Option<String>,
    // Slash commands & quick actions
    #[serde(default)]
    pub bot_enabled: bool,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub auto_reply: String,
    #[serde(default)]
    pub webhook_url: String,
    // Visual styling (only active for theme-capable plugins)
    #[serde(default)]
    pub theme_enabled: bool,
    #[serde(default)]
    pub theme: PluginTheme,
    // Plugin workspace surfaces & reorderable custom layout
    #[serde(default)]
    pub left_app_enabled: bool,
    #[serde(default)]
    pub custom_page_enabled: bool,
    #[serde(default)]
    pub page_title: String,
    #[serde(default)]
    pub page_icon: String,
    #[serde(default)]
    pub page_kind: String, // "notes" | "soundboard" | "voxel" | "theme_sakura" | "cyber" | "tools" | "custom"
    #[serde(default)]
    pub custom_blocks: Vec<PluginCustomBlock>,
    #[serde(default)]
    pub section_order: Vec<String>,
    #[serde(default)]
    pub hidden_sections: Vec<String>,
    #[serde(default)]
    pub config: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub custom_settings: Vec<PluginSettingDef>,
    // Legacy persisted fields (kept for JSON compatibility, unused in UI)
    #[serde(default)]
    pub hud_enabled: bool,
    #[serde(default)]
    pub hud_position: String,
    #[serde(default)]
    pub hud_coords: bool,
    #[serde(default)]
    pub hud_clock: bool,
    #[serde(default)]
    pub hud_note: bool,
    #[serde(default)]
    pub hud_pomodoro: bool,
    #[serde(default)]
    pub screen_fx: String, // "none" | "sakura" | "snow" | "matrix" | "voxel"
    #[serde(default)]
    pub sound_fx: String,  // "none" | "minecraft" | "mech" | "scifi"
    #[serde(default)]
    pub composer_buttons: Vec<PluginComposerBtn>,
}

impl PluginMod {
    pub fn cfg_bool(&self, key: &str, default: bool) -> bool {
        self.config.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    pub fn cfg_i64(&self, key: &str, default: i64) -> i64 {
        self.config.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
    }

    pub fn cfg_f64(&self, key: &str, default: f64) -> f64 {
        self.config.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }

    pub fn cfg_str(&self, key: &str, default: &str) -> String {
        self.config
            .get(key)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| default.to_string())
    }

    pub fn set_cfg(&mut self, key: &str, val: serde_json::Value) {
        self.config.insert(key.to_string(), val);
    }

    pub fn ordered_sections(&self, defaults: &[&str]) -> Vec<String> {
        let mut out: Vec<String> = self
            .section_order
            .iter()
            .filter(|s| defaults.iter().any(|d| d == &s.as_str()))
            .cloned()
            .collect();
        for d in defaults {
            if !out.iter().any(|x| x == d) {
                out.push((*d).to_string());
            }
        }
        out
    }

    pub fn is_section_hidden(&self, key: &str) -> bool {
        self.hidden_sections.iter().any(|k| k == key)
    }

    pub fn toggle_section(&mut self, key: &str) {
        if let Some(pos) = self.hidden_sections.iter().position(|k| k == key) {
            self.hidden_sections.remove(pos);
        } else {
            self.hidden_sections.push(key.to_string());
        }
    }

    pub fn move_section(&mut self, key: &str, delta: isize, defaults: &[&str]) {
        let mut ord = self.ordered_sections(defaults);
        if let Some(pos) = ord.iter().position(|k| k == key) {
            let next = pos as isize + delta;
            if next >= 0 && (next as usize) < ord.len() {
                ord.swap(pos, next as usize);
                self.section_order = ord;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TodoItem {
    id: String,
    text: String,
    done: bool,
}

pub static PLUGINS: GlobalSignal<Vec<PluginMod>> = Signal::global(load_plugins_from_storage);
pub static ACTIVE_PLUGIN_PAGE: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static ACTIVE_LEFT_PLUGIN: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static EDITING_PLUGIN_ID: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static PENDING_COMPOSER_TEXT: GlobalSignal<Option<String>> = Signal::global(|| None);

fn default_builtin_plugins() -> Vec<PluginMod> {
    let mut notes_cfg = serde_json::Map::new();
    notes_cfg.insert("focus_minutes".into(), json!(25));
    notes_cfg.insert("break_minutes".into(), json!(5));
    notes_cfg.insert("sound_alert".into(), json!(true));
    notes_cfg.insert("layout_mode".into(), json!("two_col"));

    let mut sound_cfg = serde_json::Map::new();
    sound_cfg.insert("volume".into(), json!(75));
    sound_cfg.insert("pitch".into(), json!(100));
    sound_cfg.insert("columns".into(), json!(4));

    let mut voxel_cfg = serde_json::Map::new();
    voxel_cfg.insert("default_color".into(), json!("#10B981"));
    voxel_cfg.insert("show_grid".into(), json!(true));
    voxel_cfg.insert("bevel_blocks".into(), json!(true));
    voxel_cfg.insert("brush_sound".into(), json!(true));

    let mut cyber_cfg = serde_json::Map::new();
    cyber_cfg.insert("cmd_sound".into(), json!(true));

    vec![
        PluginMod {
            id: "mod:workspace_notes".into(),
            username: "workspace_notes".into(),
            display_name: "Sổ tay & Việc làm".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "📋".into(),
            logo_url: "logo:notes".into(),
            default_logo_url: "logo:notes".into(),
            allow_custom_logo: true,
            description: String::new(),
            installed: true,
            enabled: true,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![
                PluginCommand { trigger: "/todo".into(), description: "Thêm việc cần làm".into(), response: "✅ Đã thêm việc cho {user}: {args}".into(), icon: "svg:todo".into(), enabled: true },
                PluginCommand { trigger: "/note".into(), description: "Lưu ghi chú".into(), response: "📝 Đã lưu ghi chú lúc {time}: {args}".into(), icon: "svg:notes".into(), enabled: true },
            ],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: false,
            theme: PluginTheme::default(),
            left_app_enabled: true,
            custom_page_enabled: true,
            page_title: "Sổ tay".into(),
            page_icon: "📋".into(),
            page_kind: "notes".into(),
            custom_blocks: Vec::new(),
            section_order: vec!["pomodoro".into(), "todos".into(), "notes".into()],
            hidden_sections: Vec::new(),
            config: notes_cfg,
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "none".into(),
            sound_fx: "none".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:soundboard_mech".into(),
            username: "soundboard_plugin".into(),
            display_name: "Soundboard 8-Bit".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "🎹".into(),
            logo_url: "logo:sound".into(),
            default_logo_url: "logo:sound".into(),
            allow_custom_logo: true,
            description: String::new(),
            installed: false,
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![
                PluginCommand { trigger: "/sound".into(), description: "Phát âm thanh".into(), response: "🔊 Đã phát âm thanh cho {user}".into(), icon: "logo:sound".into(), enabled: true },
            ],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: false,
            theme: PluginTheme::default(),
            left_app_enabled: true,
            custom_page_enabled: true,
            page_title: "Soundboard".into(),
            page_icon: "🎹".into(),
            page_kind: "soundboard".into(),
            custom_blocks: Vec::new(),
            section_order: vec!["xp".into(), "levelup".into(), "anvil".into(), "mech".into(), "laser".into(), "coin".into(), "chime".into(), "pop".into()],
            hidden_sections: Vec::new(),
            config: sound_cfg,
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "none".into(),
            sound_fx: "mech".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:smart_assistant".into(),
            username: "quick_tools".into(),
            display_name: "Công cụ Lệnh nhanh".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "⚡".into(),
            logo_url: "logo:code".into(),
            default_logo_url: "logo:code".into(),
            allow_custom_logo: true,
            description: String::new(),
            installed: true,
            enabled: true,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![
                PluginCommand { trigger: "/calc".into(), description: "Tính biểu thức toán".into(), response: "🧮 Kết quả: {calc}".into(), icon: "svg:calc".into(), enabled: true },
                PluginCommand { trigger: "/roll".into(), description: "Tung xúc xắc 1–6".into(), response: "🎲 {user} tung xúc xắc được: {dice}".into(), icon: "svg:dice".into(), enabled: true },
                PluginCommand { trigger: "/flip".into(), description: "Tung đồng xu".into(), response: "🪙 Đồng xu rơi vào mặt: {coin}".into(), icon: "svg:coin".into(), enabled: true },
                PluginCommand { trigger: "/time".into(), description: "Chèn ngày giờ hiện tại".into(), response: "🕒 {time} · {date}".into(), icon: "svg:clock".into(), enabled: true },
                PluginCommand { trigger: "/upper".into(), description: "Chuyển sang chữ in hoa".into(), response: "🔠 {upper}".into(), icon: "svg:upper".into(), enabled: true },
            ],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: false,
            theme: PluginTheme::default(),
            left_app_enabled: true,
            custom_page_enabled: true,
            page_title: "Công cụ".into(),
            page_icon: "⚡".into(),
            page_kind: "tools".into(),
            custom_blocks: Vec::new(),
            section_order: vec!["calc".into(), "random".into(), "text".into(), "snippets".into()],
            hidden_sections: Vec::new(),
            config: serde_json::Map::new(),
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "none".into(),
            sound_fx: "none".into(),
            composer_buttons: vec![
                PluginComposerBtn { label: "/roll".into(), content: "/roll".into() },
                PluginComposerBtn { label: "/flip".into(), content: "/flip".into() },
                PluginComposerBtn { label: "/time".into(), content: "/time".into() },
            ],
        },
        PluginMod {
            id: "mod:minecraft_voxel".into(),
            username: "voxel_plugin".into(),
            display_name: "Bảng vẽ Voxel".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "🎨".into(),
            logo_url: "logo:voxel".into(),
            default_logo_url: "logo:voxel".into(),
            allow_custom_logo: false,
            description: String::new(),
            installed: false,
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![
                PluginCommand { trigger: "/craft".into(), description: "Tạo vật phẩm".into(), response: "⛏ [{user}] đã chế tạo thành công!".into(), icon: "svg:voxel".into(), enabled: true },
                PluginCommand { trigger: "/seed".into(), description: "Mã ngẫu nhiên".into(), response: "🌱 Seed: {rand100}".into(), icon: "svg:code".into(), enabled: true },
            ],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: false,
            theme: PluginTheme::default(),
            left_app_enabled: false,
            custom_page_enabled: true,
            page_title: "Bảng Voxel".into(),
            page_icon: "🎨".into(),
            page_kind: "voxel".into(),
            custom_blocks: Vec::new(),
            section_order: vec!["actions".into(), "palette".into(), "canvas".into()],
            hidden_sections: Vec::new(),
            config: voxel_cfg,
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "none".into(),
            sound_fx: "minecraft".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:sakura_glass".into(),
            username: "sakura_plugin".into(),
            display_name: "Sakura Kính mờ".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "🌸".into(),
            logo_url: "logo:sakura".into(),
            default_logo_url: "logo:sakura".into(),
            allow_custom_logo: false,
            description: String::new(),
            installed: false,
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: true,
            theme: PluginTheme {
                bg_app: "#FDF2F8".into(),
                bg_sidebar: "#FCE7F3".into(),
                bg_chat: "#FFF5F9".into(),
                bg_surface: "#FFFFFF".into(),
                accent: "#EC4899".into(),
                msg_sent: "#DB2777".into(),
                msg_recv: "#FCE7F3".into(),
                text_primary: "#4A152B".into(),
                panel_radius: "20px".into(),
                border_style: "glass".into(),
                glass_blur: 14,
                rail_position: "left".into(),
                font_style: "rounded".into(),
                custom_css: String::new(),
            },
            left_app_enabled: false,
            custom_page_enabled: false,
            page_title: "Sakura".into(),
            page_icon: "🌸".into(),
            page_kind: "theme_sakura".into(),
            custom_blocks: Vec::new(),
            section_order: Vec::new(),
            hidden_sections: Vec::new(),
            config: serde_json::Map::new(),
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "sakura".into(),
            sound_fx: "none".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:cyberpunk_hud".into(),
            username: "cyber_plugin".into(),
            display_name: "Cyber Neon".into(),
            author: "ChatW".into(),
            version: "2.0.0".into(),
            icon: "⚡".into(),
            logo_url: "logo:cyber".into(),
            default_logo_url: "logo:cyber".into(),
            allow_custom_logo: true,
            description: String::new(),
            installed: false,
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: false,
            commands: vec![
                PluginCommand { trigger: "/neon".into(), description: "Kích hoạt tín hiệu".into(), response: "⚡ Hệ thống đồng bộ {rand100}%".into(), icon: "svg:cyber".into(), enabled: true },
                PluginCommand { trigger: "/status".into(), description: "Trạng thái kết nối".into(), response: "🛰 Trực tuyến · Mã hóa E2EE P-256 + AES-256-GCM".into(), icon: "svg:globe".into(), enabled: true },
            ],
            auto_reply: String::new(),
            webhook_url: String::new(),
            theme_enabled: true,
            theme: PluginTheme {
                bg_app: "#070812".into(),
                bg_sidebar: "#0D1021".into(),
                bg_chat: "#0A0C1A".into(),
                bg_surface: "#141830".into(),
                accent: "#00F0FF".into(),
                msg_sent: "#D900FF".into(),
                msg_recv: "#161B36".into(),
                text_primary: "#E0FAFF".into(),
                panel_radius: "8px".into(),
                border_style: "neon".into(),
                glass_blur: 8,
                rail_position: "left".into(),
                font_style: "pixel".into(),
                custom_css: String::new(),
            },
            left_app_enabled: false,
            custom_page_enabled: true,
            page_title: "Trạm Cyber".into(),
            page_icon: "⚡".into(),
            page_kind: "cyber".into(),
            custom_blocks: Vec::new(),
            section_order: vec!["stats".into(), "console".into()],
            hidden_sections: Vec::new(),
            config: cyber_cfg,
            custom_settings: Vec::new(),
            hud_enabled: false,
            hud_position: String::new(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            hud_pomodoro: false,
            screen_fx: "matrix".into(),
            sound_fx: "scifi".into(),
            composer_buttons: Vec::new(),
        },
    ]
}

fn load_plugins_from_storage() -> Vec<PluginMod> {
    let mut defaults = default_builtin_plugins();
    let logo_migrated = platform::storage_get("chatw_plugin_logos_v2").is_some();
    let store_migrated = platform::storage_get(STORE_MIGRATED_KEY).is_some();
    let modular_migrated = platform::storage_get(MODULAR_MIGRATED_KEY).is_some();
    if let Some(raw) = platform::storage_get(STORAGE_KEY) {
        if let Ok(saved) = serde_json::from_str::<Vec<PluginMod>>(&raw) {
            for mut s in saved {
                if !store_migrated {
                    s.installed = s.enabled || !s.builtin;
                }
                // Always purge legacy HUD and fake bot flags
                s.hud_enabled = false;
                s.bot_enabled = false;

                if let Some(existing) = defaults.iter_mut().find(|d| d.id == s.id) {
                    if !modular_migrated {
                        s.display_name = existing.display_name.clone();
                        s.page_title = existing.page_title.clone();
                        s.page_kind = existing.page_kind.clone();
                        s.allow_custom_logo = existing.allow_custom_logo;
                        s.default_logo_url = existing.default_logo_url.clone();
                        s.description = String::new();
                        if s.id == "mod:smart_assistant" {
                            s.logo_url = existing.logo_url.clone();
                            s.left_app_enabled = existing.left_app_enabled;
                            s.custom_page_enabled = existing.custom_page_enabled;
                            s.commands = existing.commands.clone();
                        }
                        if s.id == "mod:minecraft_voxel" {
                            s.theme_enabled = false;
                            s.screen_fx = "none".into();
                        }
                        if s.section_order.is_empty() {
                            s.section_order = existing.section_order.clone();
                        }
                        if s.config.is_empty() {
                            s.config = existing.config.clone();
                        }
                    }
                    if s.default_logo_url.is_empty() {
                        s.default_logo_url = existing.default_logo_url.clone();
                    }
                    if (s.logo_url.trim().is_empty() || (!logo_migrated && s.logo_url.starts_with("svg:"))) && !existing.logo_url.is_empty() {
                        s.logo_url = existing.logo_url.clone();
                    }
                    if !s.installed {
                        s.enabled = false;
                    }
                    *existing = s;
                } else {
                    if s.default_logo_url.is_empty() {
                        s.default_logo_url = s.logo_url.clone();
                    }
                    if !s.installed {
                        s.enabled = false;
                    }
                    defaults.push(s);
                }
            }
        }
    }
    if !logo_migrated || !store_migrated || !modular_migrated {
        platform::storage_set("chatw_plugin_logos_v2", "1");
        platform::storage_set(STORE_MIGRATED_KEY, "1");
        platform::storage_set(MODULAR_MIGRATED_KEY, "1");
        if let Ok(s) = serde_json::to_string(&defaults) {
            platform::storage_set(STORAGE_KEY, &s);
        }
    }
    defaults
}

pub const PLUGIN_LOGO_PRESETS: [(&str, &str); 8] = [
    ("logo:notes", "Sổ tay"),
    ("logo:sound", "Âm thanh"),
    ("logo:bot", "Trợ lý"),
    ("logo:voxel", "Khối 3D"),
    ("logo:cyber", "Cyber"),
    ("logo:sakura", "Sakura"),
    ("logo:clock", "Đồng hồ"),
    ("logo:code", "Terminal"),
];

pub const PLUGIN_SVG_PRESETS: [(&str, &str); 9] = [
    ("svg:plugin", "Plugin"),
    ("svg:notes", "Sổ tay"),
    ("svg:sound", "Âm thanh"),
    ("svg:bot", "Bot"),
    ("svg:voxel", "Khối 3D"),
    ("svg:cyber", "Tia chớp"),
    ("svg:clock", "Đồng hồ"),
    ("svg:code", "Mã lệnh"),
    ("svg:sakura", "Hoa"),
];

fn logo_palette(logo_id: &str) -> (&'static str, &'static str) {
    match logo_id {
        "notes" => ("#10B981", "#059669"),
        "sound" => ("#6366F1", "#8B5CF6"),
        "bot" => ("#2563EB", "#06B6D4"),
        "voxel" => ("#16A34A", "#22C55E"),
        "cyber" => ("#0F172A", "#06B6D4"),
        "sakura" => ("#EC4899", "#F43F5E"),
        "clock" => ("#F59E0B", "#EA580C"),
        "code" => ("#18181B", "#22C55E"),
        _ => ("#2764E7", "#6366F1"),
    }
}

fn encode_svg_data_uri(svg_raw: &str) -> String {
    let mut out = String::with_capacity(svg_raw.len() + 32);
    out.push_str("data:image/svg+xml;utf8,");
    for ch in svg_raw.chars() {
        match ch {
            '#' => out.push_str("%23"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '"' => out.push('\''),
            '&' => out.push_str("%26"),
            '%' => out.push_str("%25"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

pub fn plugin_logo_data_url(logo_url: &str) -> Option<String> {
    let logo = logo_url.trim();
    if logo.is_empty() || logo.starts_with("svg:") {
        return None;
    }
    if logo.starts_with("<svg") {
        return Some(encode_svg_data_uri(logo));
    }
    if let Some(lid) = logo.strip_prefix("logo:") {
        let (c1, c2) = logo_palette(lid);
        let glyph = match lid {
            "notes" => r##"<path fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" d="M13 11.5h14l3 3v14a2 2 0 0 1-2 2H13a2 2 0 0 1-2-2v-15a2 2 0 0 1 2-2z"/><path fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" d="M15.5 18h9M15.5 23h6"/>"##,
            "sound" => r##"<path fill="none" stroke="#fff" stroke-width="2.8" stroke-linecap="round" d="M10 18v4M15 13v14M20 9.5v21M25 14v12M30 17.5v5"/>"##,
            "bot" => r##"<rect x="10" y="13" width="20" height="15" rx="5.5" fill="none" stroke="#fff" stroke-width="2.4"/><path fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" d="M20 8.5V13M16 20v1.5M24 20v1.5"/>"##,
            "voxel" => r##"<polygon points="20 8.5 30.5 14.5 30.5 25.5 20 31.5 9.5 25.5 9.5 14.5" fill="none" stroke="#fff" stroke-width="2.4" stroke-linejoin="round"/><path fill="none" stroke="#fff" stroke-width="2.2" d="M20 31.5V20M30.5 14.5L20 20 9.5 14.5"/>"##,
            "cyber" => r##"<polygon points="21.5 7.5 11.5 21.5 19 21.5 17.5 32.5 28.5 18.5 20.5 18.5" fill="#22D3EE"/>"##,
            "sakura" => r##"<path fill="#fff" d="M20 8.5c1.3 6.2 3.3 8.2 9.5 9.5-6.2 1.3-8.2 3.3-9.5 9.5-1.3-6.2-3.3-8.2-9.5-9.5 6.2-1.3 8.2-3.3 9.5-9.5z"/>"##,
            "clock" => r##"<circle cx="20" cy="21" r="9.5" fill="none" stroke="#fff" stroke-width="2.4"/><path fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round" d="M17 8.5h6M20 15.5V21l3.5 2.5"/>"##,
            "code" => r##"<path fill="none" stroke="#22C55E" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" d="M12.5 15l5 5-5 5M21 25h7"/>"##,
            _ => r##"<rect x="10" y="10" width="8.5" height="8.5" rx="2.2" fill="#fff"/><rect x="10" y="21.5" width="8.5" height="8.5" rx="2.2" fill="#fff"/><rect x="21.5" y="21.5" width="8.5" height="8.5" rx="2.2" fill="#fff"/><rect x="22.2" y="10.8" width="7" height="7" rx="2" transform="rotate(45 25.7 14.3)" fill="#fff"/>"##,
        };
        let raw_svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" fill="{c1}"/><path d="M0 0h40v21L0 33z" fill="{c2}" fill-opacity="0.32"/>{glyph}</svg>"#
        );
        return Some(encode_svg_data_uri(&raw_svg));
    }
    Some(logo.to_string())
}

pub(crate) fn glyph_slot_size(logo_url: &str, icon_sz: u32, logo_sz: u32) -> u32 {
    let l = logo_url.trim();
    if !l.is_empty() && !l.starts_with("svg:") {
        logo_sz
    } else {
        icon_sz
    }
}

#[component]
pub fn PluginGlyph(icon: String, #[props(default)] logo_url: String, #[props(default = 20)] size: u32) -> Element {
    let logo = logo_url.trim();
    if let Some(lid) = logo.strip_prefix("logo:") {
        let (c1, c2) = logo_palette(lid);
        return rsx! {
            svg {
                class: "plugin-brand-logo",
                width: "{size}",
                height: "{size}",
                view_box: "0 0 40 40",
                fill: "none",
                "aria-hidden": "true",
                rect { width: "40", height: "40", rx: "12", fill: "{c1}" }
                path { d: "M0 0h40v21L0 33z", fill: "{c2}", fill_opacity: "0.35" }
                match lid {
                    "notes" => rsx! {
                        path { stroke: "#fff", stroke_width: "2.4", stroke_linecap: "round", stroke_linejoin: "round", d: "M13 11.5h14l3 3v14a2 2 0 0 1-2 2H13a2 2 0 0 1-2-2v-15a2 2 0 0 1 2-2z" }
                        path { stroke: "#fff", stroke_width: "2.4", stroke_linecap: "round", d: "M15.5 18h9M15.5 23h6" }
                    },
                    "sound" => rsx! {
                        path { stroke: "#fff", stroke_width: "2.8", stroke_linecap: "round", d: "M10 18v4M15 13v14M20 9.5v21M25 14v12M30 17.5v5" }
                    },
                    "bot" => rsx! {
                        rect { x: "10", y: "13", width: "20", height: "15", rx: "5.5", stroke: "#fff", stroke_width: "2.4" }
                        path { stroke: "#fff", stroke_width: "2.4", stroke_linecap: "round", d: "M20 8.5V13M16 20v1.5M24 20v1.5" }
                    },
                    "voxel" => rsx! {
                        polygon { points: "20 8.5 30.5 14.5 30.5 25.5 20 31.5 9.5 25.5 9.5 14.5", stroke: "#fff", stroke_width: "2.4", stroke_linejoin: "round" }
                        path { stroke: "#fff", stroke_width: "2.2", d: "M20 31.5V20M30.5 14.5L20 20 9.5 14.5" }
                    },
                    "cyber" => rsx! {
                        polygon { points: "21.5 7.5 11.5 21.5 19 21.5 17.5 32.5 28.5 18.5 20.5 18.5", fill: "#22D3EE" }
                    },
                    "sakura" => rsx! {
                        path { fill: "#fff", d: "M20 8.5c1.3 6.2 3.3 8.2 9.5 9.5-6.2 1.3-8.2 3.3-9.5 9.5-1.3-6.2-3.3-8.2-9.5-9.5 6.2-1.3 8.2-3.3 9.5-9.5z" }
                    },
                    "clock" => rsx! {
                        circle { cx: "20", cy: "21", r: "9.5", stroke: "#fff", stroke_width: "2.4" }
                        path { stroke: "#fff", stroke_width: "2.4", stroke_linecap: "round", d: "M17 8.5h6M20 15.5V21l3.5 2.5" }
                    },
                    "code" => rsx! {
                        path { stroke: "#22C55E", stroke_width: "2.8", stroke_linecap: "round", stroke_linejoin: "round", d: "M12.5 15l5 5-5 5M21 25h7" }
                    },
                    _ => rsx! {
                        rect { x: "10", y: "10", width: "8.5", height: "8.5", rx: "2.2", fill: "#fff" }
                        rect { x: "10", y: "21.5", width: "8.5", height: "8.5", rx: "2.2", fill: "#fff" }
                        rect { x: "21.5", y: "21.5", width: "8.5", height: "8.5", rx: "2.2", fill: "#fff" }
                        rect { x: "22.2", y: "10.8", width: "7", height: "7", rx: "2", transform: "rotate(45 25.7 14.3)", fill: "#fff" }
                    },
                }
            }
        };
    }
    if let Some(svg_id) = logo.strip_prefix("svg:") {
        return match svg_id {
            "notes" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    path { fill: "currentColor", fill_opacity: "0.16", d: "M6.5 3.5h8l4 4v12a1.8 1.8 0 0 1-1.8 1.8H6.5A1.8 1.8 0 0 1 4.7 19.5V5.3a1.8 1.8 0 0 1 1.8-1.8z" }
                    path { d: "M14.5 3.5v4h4M8.5 12h7M8.5 16h5" }
                }
            },
            "sound" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", "aria-hidden": "true",
                    path { d: "M4 10v4M8 6.5v11M12 3.5v17M16 7.5v9M20 10.5v3" }
                }
            },
            "bot" => rsx! { BotIcon { size } },
            "voxel" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    polygon { points: "12 3 20 7.5 20 16.5 12 21 4 16.5 4 7.5", fill: "currentColor", fill_opacity: "0.16" }
                    path { d: "M12 21V12M20 7.5L12 12 4 7.5" }
                }
            },
            "cyber" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    polygon { points: "13 2.5 5 13.5 11.5 13.5 10.5 21.5 19 10.5 12.5 10.5", fill: "currentColor", fill_opacity: "0.18" }
                }
            },
            "sakura" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    path { fill: "currentColor", fill_opacity: "0.18", d: "M12 3.5c1.1 5 2.7 6.6 7.5 7.5-4.8 1.1-6.4 2.7-7.5 7.5-1.1-4.8-2.7-6.4-7.5-7.5 4.8-.9 6.4-2.5 7.5-7.5z" }
                }
            },
            "clock" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    circle { cx: "12", cy: "13", r: "8", fill: "currentColor", fill_opacity: "0.16" }
                    path { d: "M9.5 2.5h5M12 9v4l2.8 2" }
                }
            },
            "code" => rsx! {
                svg { class: "cw-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                    rect { x: "3.5", y: "4.5", width: "17", height: "15", rx: "3", fill: "currentColor", fill_opacity: "0.14" }
                    path { d: "M8 10l3 2.5-3 2.5M12.5 15h3.5" }
                }
            },
            "vico" => rsx! { VicoAiIcon { size } },
            "dice" => rsx! { span { style: "color: #7C3AED; display: inline-flex;", DiceIcon { size } } },
            "coin" => rsx! { span { style: "color: #D97706; display: inline-flex;", CoinIcon { size } } },
            "calc" => rsx! { span { style: "color: #059669; display: inline-flex;", CalcIcon { size } } },
            "upper" => rsx! { span { style: "color: #0D9488; display: inline-flex;", TypeIcon { size } } },
            "help" => rsx! { span { style: "color: #6366F1; display: inline-flex;", HelpCircleIcon { size } } },
            "compass" => rsx! { span { style: "color: #0284C7; display: inline-flex;", CompassIcon { size } } },
            "globe" => rsx! { span { style: "color: #2563EB; display: inline-flex;", GlobeIcon { size } } },
            "todo" => rsx! { span { style: "color: #16A34A; display: inline-flex;", ListChecksIcon { size } } },
            "workspace" => rsx! { span { style: "color: #2563EB; display: inline-flex;", BuildingIcon { size } } },
            "task" => rsx! { span { style: "color: #16A34A; display: inline-flex;", ListChecksIcon { size } } },
            "checkin" => rsx! { span { style: "color: #16A34A; display: inline-flex;", LogInIcon { size } } },
            "checkout" => rsx! { span { style: "color: #DC2626; display: inline-flex;", LogOutIcon { size } } },
            "leave" => rsx! { span { style: "color: #0891B2; display: inline-flex;", CalendarIcon { size } } },
            "meet" => rsx! { span { style: "color: #4F46E5; display: inline-flex;", VideoIcon { size } } },
            "approve" => rsx! { span { style: "color: #7C3AED; display: inline-flex;", ClipboardCheckIcon { size } } },
            "announce" => rsx! { span { style: "color: #EA580C; display: inline-flex;", MegaphoneIcon { size } } },
            "bolt" => rsx! { span { style: "color: #EAB308; display: inline-flex;", BoltIcon { size } } },
            _ => rsx! { PluginIcon { size } },
        };
    }
    if logo.starts_with("<svg") {
        let src = encode_svg_data_uri(logo);
        return rsx! {
            img {
                class: "plugin-custom-logo",
                src: "{src}",
                width: "{size}",
                height: "{size}",
                alt: "",
            }
        };
    }
    if !logo.is_empty() {
        return rsx! {
            img {
                class: "plugin-custom-logo",
                src: "{platform::resolve_asset_url(logo)}",
                width: "{size}",
                height: "{size}",
                alt: "",
            }
        };
    }
    let ic = icon.trim();
    if ic.is_empty() {
        rsx! { PluginIcon { size } }
    } else {
        rsx! { span { style: "font-size: {size}px; line-height: 1; display: inline-flex; align-items: center; justify-content: center;", "{ic}" } }
    }
}

pub fn save_plugins(list: &[PluginMod]) {
    if let Ok(s) = serde_json::to_string(list) {
        platform::storage_set(STORAGE_KEY, &s);
    }
    sync_plugins_to_dom(list);
}

pub fn init_plugins() {
    let list = PLUGINS.read().clone();
    sync_plugins_to_dom(&list);
}

/// Plugins that open inside the Left Column (.nav-sidebar) as an "App bên trái".
pub fn enabled_left_apps() -> Vec<(String, String, String, String)> {
    PLUGINS
        .read()
        .iter()
        .filter(|p| p.installed && p.enabled && p.left_app_enabled)
        .map(|p| {
            let icon = if p.page_icon.trim().is_empty() { p.icon.clone() } else { p.page_icon.clone() };
            let title = if p.page_title.trim().is_empty() { p.display_name.clone() } else { p.page_title.clone() };
            (p.id.clone(), icon, p.logo_url.clone(), title)
        })
        .collect()
}

/// Plugins that open as full Main Stage pages (when not already docked as Left App).
pub fn enabled_rail_pages() -> Vec<(String, String, String, String)> {
    PLUGINS
        .read()
        .iter()
        .filter(|p| p.installed && p.enabled && p.custom_page_enabled && !p.left_app_enabled)
        .map(|p| {
            let icon = if p.page_icon.trim().is_empty() { p.icon.clone() } else { p.page_icon.clone() };
            let title = if p.page_title.trim().is_empty() { p.display_name.clone() } else { p.page_title.clone() };
            (p.id.clone(), icon, p.logo_url.clone(), title)
        })
        .collect()
}

/// Resolves the most accurate icon/logo pair for a slash command trigger.
pub fn resolve_command_icon(trigger: &str, cmd_icon: &str, plugin_icon: &str, plugin_logo: &str) -> (String, String) {
    let custom = cmd_icon.trim();
    if !custom.is_empty() {
        if custom.starts_with("svg:") || custom.starts_with("logo:") || custom.starts_with("<svg") || custom.starts_with('/') || custom.starts_with("http") || custom.starts_with("data:") {
            return (String::new(), custom.to_string());
        }
        return (custom.to_string(), String::new());
    }
    let plogo = plugin_logo.trim();
    let has_brand_logo = plogo.starts_with("logo:") || plogo.starts_with("<svg") || plogo.starts_with('/') || plogo.starts_with("http") || plogo.starts_with("data:");
    match trigger.trim().to_lowercase().as_str() {
        "/roll" | "/dice" => (String::new(), "svg:dice".into()),
        "/flip" | "/coin" => (String::new(), "svg:coin".into()),
        "/time" | "/timer" | "/uptime" | "/clock" => (String::new(), "svg:clock".into()),
        "/calc" | "/math" => (String::new(), "svg:calc".into()),
        "/upper" | "/lower" | "/text" => (String::new(), "svg:upper".into()),
        "/sound" | "/sfx" | "/play" => {
            if has_brand_logo {
                (String::new(), plogo.to_string())
            } else {
                (String::new(), "svg:sound".into())
            }
        }
        "/craft" => (String::new(), "svg:voxel".into()),
        "/coords" | "/pos" => (String::new(), "svg:compass".into()),
        "/seed" => (String::new(), "svg:code".into()),
        "/neon" | "/bolt" => (String::new(), "svg:cyber".into()),
        "/status" => (String::new(), "svg:globe".into()),
        "/sakura" | "/fortune" => (String::new(), "svg:sakura".into()),
        "/todo" => (String::new(), "svg:todo".into()),
        "/note" | "/notes" => (String::new(), "svg:notes".into()),
        "/help" => (String::new(), "svg:help".into()),
        "/workspace" => (String::new(), "svg:workspace".into()),
        "/task" => (String::new(), "svg:task".into()),
        "/checkin" => (String::new(), "svg:checkin".into()),
        "/checkout" => (String::new(), "svg:checkout".into()),
        "/leave" => (String::new(), "svg:leave".into()),
        "/meet" => (String::new(), "svg:meet".into()),
        "/approve" => (String::new(), "svg:approve".into()),
        "/announce" => (String::new(), "svg:announce".into()),
        "/vico" | "/ai" => (String::new(), "svg:vico".into()),
        _ => (plugin_icon.to_string(), plugin_logo.to_string()),
    }
}

/// Enabled `/commands` across all active Plugins for the Composer `/` autocomplete popup.
pub fn enabled_slash_commands() -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in PLUGINS.read().iter().filter(|p| p.installed && p.enabled) {
        for c in &p.commands {
            if !c.enabled {
                continue;
            }
            let tr = c.trigger.trim();
            if tr.starts_with('/') && seen.insert(tr.to_lowercase()) {
                let (ic, lg) = resolve_command_icon(tr, &c.icon, &p.icon, &p.logo_url);
                out.push((ic, lg, tr.to_string(), c.description.clone()));
            }
        }
    }
    if !out.is_empty() && seen.insert("/help".to_string()) {
        out.push((String::new(), "svg:help".into(), "/help".into(), "Xem tất cả lệnh đang bật".into()));
    }
    out
}

pub fn enabled_composer_buttons() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for p in PLUGINS.read().iter().filter(|p| p.installed && p.enabled) {
        for b in &p.composer_buttons {
            if !b.label.trim().is_empty() && !b.content.trim().is_empty() {
                out.push((b.label.clone(), b.content.clone(), p.display_name.clone()));
            }
        }
    }
    out
}

/// Fake local bots are removed per Spec 032 — plugins run through their own real surfaces & slash commands.
pub fn enabled_bot_plugins_as_items() -> Vec<BotItem> {
    Vec::new()
}

fn eval_simple_math(expr: &str) -> Option<f64> {
    let clean: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.is_empty() {
        return None;
    }
    // Simple left-to-right + and - parser over * and / terms
    let mut total = 0.0;
    let mut cur_op = '+';
    let mut term_buf = String::new();
    let eval_term = |t: &str| -> Option<f64> {
        let mut acc: Option<f64> = None;
        let mut op = '*';
        for part in t.split_inclusive(['*', '/']) {
            let (num_str, next_op) = if let Some(stripped) = part.strip_suffix('*') {
                (stripped, '*')
            } else if let Some(stripped) = part.strip_suffix('/') {
                (stripped, '/')
            } else {
                (part, ' ')
            };
            let v: f64 = num_str.parse().ok()?;
            acc = Some(match acc {
                None => v,
                Some(prev) => if op == '*' { prev * v } else { prev / v },
            });
            op = next_op;
        }
        acc
    };
    for ch in clean.chars() {
        if (ch == '+' || ch == '-') && !term_buf.is_empty() {
            let v = eval_term(&term_buf)?;
            total = if cur_op == '+' { total + v } else { total - v };
            term_buf.clear();
            cur_op = ch;
        } else {
            term_buf.push(ch);
        }
    }
    if !term_buf.is_empty() {
        let v = eval_term(&term_buf)?;
        total = if cur_op == '+' { total + v } else { total - v };
    }
    Some(total)
}

fn expand_template(tpl: &str, user_name: &str, args: &str) -> String {
    let now = chrono::Local::now();
    let nanos = now.timestamp_subsec_nanos() as usize;
    let dice = (nanos % 6) + 1;
    let rand100 = (nanos % 100) + 1;
    let coin = if nanos.is_multiple_of(2) { "Ngửa" } else { "Sấp" };
    let calc_val = eval_simple_math(args)
        .map(|v| if (v - v.round()).abs() < 1e-9 { format!("{}", v as i64) } else { format!("{v:.2}") })
        .unwrap_or_else(|| if args.is_empty() { "Nhập phép tính, ví dụ /calc 12*8+4".into() } else { format!("Không tính được '{args}'") });
    tpl.replace("{user}", if user_name.is_empty() { "bạn" } else { user_name })
        .replace("{time}", &now.format("%H:%M:%S").to_string())
        .replace("{date}", &now.format("%d/%m/%Y").to_string())
        .replace("{dice}", &dice.to_string())
        .replace("{rand100}", &rand100.to_string())
        .replace("{coin}", coin)
        .replace("{args}", if args.is_empty() { "không có tham số" } else { args })
        .replace("{upper}", &args.to_uppercase())
        .replace("{calc}", &calc_val)
}

pub fn try_run_plugin_command(input: &str, user_name: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (cmd, args) = match trimmed.split_once(char::is_whitespace) {
        Some((c, a)) => (c.trim(), a.trim()),
        None => (trimmed, ""),
    };
    if cmd.eq_ignore_ascii_case("/help") || cmd.eq_ignore_ascii_case("/start") {
        let mut lines = vec!["Danh sách lệnh đang bật:".to_string()];
        for p in PLUGINS.read().iter().filter(|p| p.installed && p.enabled) {
            for c in &p.commands {
                if c.enabled {
                    lines.push(format!("• {} — {} [{}]", c.trigger, c.description, p.display_name));
                }
            }
        }
        return Some(lines.join("\n"));
    }
    for p in PLUGINS.read().iter().filter(|p| p.installed && p.enabled) {
        for c in &p.commands {
            if c.enabled && c.trigger.eq_ignore_ascii_case(cmd) {
                if p.sound_fx != "none" && p.cfg_bool("cmd_sound", true) {
                    let vol = (p.cfg_i64("volume", 75) as f64 / 100.0).clamp(0.0, 1.0);
                    let pitch = (p.cfg_i64("pitch", 100) as f64 / 100.0).clamp(0.5, 2.0);
                    play_mod_sound_opts(&p.sound_fx, vol, pitch);
                }
                return Some(expand_template(&c.response, user_name, args));
            }
        }
    }
    None
}

pub fn play_mod_sound(kind: &str) {
    play_mod_sound_opts(kind, 0.75, 1.0);
}

pub fn play_mod_sound_opts(kind: &str, volume: f64, pitch: f64) {
    if volume <= 0.01 {
        return;
    }
    let vol_scale = volume.clamp(0.05, 1.0);
    let pitch_scale = pitch.clamp(0.5, 2.0);
    let k = serde_json::to_string(kind).unwrap_or_else(|_| "\"minecraft\"".into());
    let _ = dioxus::document::eval(&format!(
        r#"
        try {{
            const AC = window.AudioContext || window.webkitAudioContext;
            if (AC) {{
                const ctx = window.__chatwAudioCtx || new AC();
                if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                const vScale = {vol_scale:.3};
                const pScale = {pitch_scale:.3};
                const tone = (f0, f1, start, dur, type, vol) => {{
                    const osc = ctx.createOscillator();
                    const g = ctx.createGain();
                    osc.type = type;
                    const t = ctx.currentTime + start;
                    osc.frequency.setValueAtTime(Math.max(40, f0 * pScale), t);
                    osc.frequency.exponentialRampToValueAtTime(Math.max(20, f1 * pScale), t + dur);
                    g.gain.setValueAtTime(0.001, t);
                    g.gain.exponentialRampToValueAtTime(Math.max(0.002, vol * vScale), t + 0.01);
                    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
                    osc.connect(g); g.connect(ctx.destination);
                    osc.start(t); osc.stop(t + dur + 0.02);
                }};
                const run = () => {{
                    const kind = {k};
                    if (kind === 'minecraft' || kind === 'xp') {{
                        tone(587, 880, 0, 0.09, 'square', 0.14);
                        tone(880, 1318, 0.08, 0.12, 'square', 0.14);
                    }} else if (kind === 'levelup') {{
                        tone(523, 659, 0, 0.1, 'square', 0.16);
                        tone(659, 784, 0.1, 0.1, 'square', 0.16);
                        tone(784, 1046, 0.2, 0.22, 'square', 0.18);
                    }} else if (kind === 'anvil') {{
                        tone(220, 110, 0, 0.18, 'sawtooth', 0.22);
                        tone(880, 440, 0.01, 0.08, 'triangle', 0.17);
                    }} else if (kind === 'mech') {{
                        tone(1800, 420, 0, 0.028, 'triangle', 0.18);
                    }} else if (kind === 'scifi' || kind === 'laser') {{
                        tone(1400, 240, 0, 0.16, 'sawtooth', 0.16);
                    }} else if (kind === 'coin') {{
                        tone(987, 987, 0, 0.07, 'square', 0.16);
                        tone(1318, 1318, 0.07, 0.22, 'square', 0.16);
                    }} else if (kind === 'pop') {{
                        tone(420, 920, 0, 0.065, 'sine', 0.18);
                    }} else {{
                        tone(660, 990, 0, 0.12, 'sine', 0.16);
                    }}
                }};
                if (ctx.state === 'suspended') ctx.resume().then(run); else run();
            }}
        }} catch (_) {{}}
        "#
    ));
}

fn hex_luma(hex: &str) -> Option<f32> {
    let h = hex.trim().strip_prefix('#')?;
    let (r, g, b) = if h.len() == 3 {
        let mut ch = h.chars();
        let r = u8::from_str_radix(&format!("{0}{0}", ch.next()?), 16).ok()? as f32;
        let g = u8::from_str_radix(&format!("{0}{0}", ch.next()?), 16).ok()? as f32;
        let b = u8::from_str_radix(&format!("{0}{0}", ch.next()?), 16).ok()? as f32;
        (r, g, b)
    } else if h.len() >= 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32;
        let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32;
        let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32;
        (r, g, b)
    } else {
        return None;
    };
    Some(0.299 * r + 0.587 * g + 0.114 * b)
}

pub fn sync_plugins_to_dom(list: &[PluginMod]) {
    let mut css = String::new();
    let mut typing_sound = "none".to_string();
    let mut sound_vol = 0.75_f64;
    let mut sound_pitch = 1.0_f64;
    for p in list.iter().filter(|p| p.installed && p.enabled) {
        if p.page_kind == "soundboard" && p.sound_fx != "none" && !p.sound_fx.is_empty() {
            typing_sound = p.sound_fx.clone();
            sound_vol = (p.cfg_i64("volume", 75) as f64 / 100.0).clamp(0.0, 1.0);
            sound_pitch = (p.cfg_i64("pitch", 100) as f64 / 100.0).clamp(0.5, 2.0);
        }
        if !p.theme_enabled {
            continue;
        }
        let t = &p.theme;
        let mut vars = String::new();
        let dark_bg = hex_luma(&t.bg_chat)
            .or_else(|| hex_luma(&t.bg_surface))
            .or_else(|| hex_luma(&t.bg_app))
            .map(|l| l < 135.0);
        let light_text = hex_luma(&t.text_primary).map(|l| l > 155.0);
        let has_colors = !t.bg_app.is_empty()
            || !t.bg_sidebar.is_empty()
            || !t.bg_chat.is_empty()
            || !t.bg_surface.is_empty()
            || !t.text_primary.is_empty();
        if has_colors {
            let is_dark = dark_bg.or(light_text).unwrap_or(false);
            let chat_bg = if !t.bg_chat.is_empty() {
                t.bg_chat.as_str()
            } else if !t.bg_app.is_empty() {
                t.bg_app.as_str()
            } else if !t.bg_surface.is_empty() {
                t.bg_surface.as_str()
            } else if is_dark {
                "#191C22"
            } else {
                "#FFFFFF"
            };
            let surface = if !t.bg_surface.is_empty() {
                t.bg_surface.as_str()
            } else if !t.bg_chat.is_empty() {
                t.bg_chat.as_str()
            } else if is_dark {
                "#22262E"
            } else {
                "#FFFFFF"
            };
            let mix_base = if !t.text_primary.is_empty() {
                t.text_primary.as_str()
            } else if is_dark {
                "#E8EAED"
            } else {
                "#171C26"
            };
            vars.push_str(if is_dark { "color-scheme: dark !important;" } else { "color-scheme: light !important;" });
            if !t.bg_app.is_empty() {
                vars.push_str(&format!("--bg-app: {} !important;", t.bg_app));
            }
            if !t.bg_sidebar.is_empty() {
                vars.push_str(&format!("--bg-sidebar: {} !important;", t.bg_sidebar));
            }
            if !t.bg_chat.is_empty() {
                vars.push_str(&format!("--bg-chat: {cb} !important;", cb = t.bg_chat));
            }
            vars.push_str(&format!(
                "--bg-primary: {cb} !important; --bg-surface: {sf} !important; --bg-elevated: color-mix(in srgb, {mb} 4%, {sf}) !important; --bg-secondary: color-mix(in srgb, {mb} 5%, {cb}) !important; --bg-surface-hover: color-mix(in srgb, {mb} 8%, {sf}) !important; --bg-hover: color-mix(in srgb, {mb} 13%, {sf}) !important; --bg-active: color-mix(in srgb, {mb} 18%, {sf}) !important; --bg-input: color-mix(in srgb, {mb} 9%, {sf}) !important; --bg-input-focus: color-mix(in srgb, {mb} 14%, {sf}) !important; --border: color-mix(in srgb, {mb} 15%, {sf}) !important; --border-subtle: color-mix(in srgb, {mb} 9%, {sf}) !important; --border-strong: color-mix(in srgb, {mb} 25%, {sf}) !important; --border-color: var(--border) !important;",
                sf = surface, mb = mix_base, cb = chat_bg
            ));
            if is_dark {
                vars.push_str("--bg-error: #3A1D20 !important; --bg-success: #15301F !important; --bg-warning: #382A12 !important; --bg-overlay: rgba(0, 0, 0, 0.56) !important; --status-warning: #F0AE3C !important; --status-success: #3DBE7E !important; --status-danger: #F2555A !important;");
            } else {
                vars.push_str("--bg-error: #FDEDEE !important; --bg-success: #E7F6EE !important; --bg-warning: #FDF4E3 !important; --bg-overlay: rgba(16, 22, 34, 0.42) !important; --status-warning: #D48A10 !important; --status-success: #169B62 !important; --status-danger: #E5484D !important;");
            }
            vars.push_str(&format!(
                "--text-primary: {mb} !important; --text: {mb} !important; --text-secondary: color-mix(in srgb, {mb} 72%, {cb}) !important; --text-muted: color-mix(in srgb, {mb} 50%, {cb}) !important; --msg-recv-text: {mb} !important; --msg-recv-meta: color-mix(in srgb, {mb} 56%, {cb}) !important;",
                mb = mix_base, cb = chat_bg
            ));
        }
        if !t.accent.is_empty() {
            let is_dark = dark_bg.or(light_text).unwrap_or(false);
            let on_acc = if hex_luma(&t.accent).is_some_and(|l| l > 155.0) { "#0B1018" } else { "#FFFFFF" };
            let acc_txt = if is_dark {
                format!("color-mix(in srgb, {} 65%, #FFFFFF)", t.accent)
            } else {
                t.accent.clone()
            };
            vars.push_str(&format!(
                "--accent: {a} !important; --accent-hover: color-mix(in srgb, {a} 85%, #000) !important; --accent-soft: color-mix(in srgb, {a} 18%, var(--bg-surface)) !important; --accent-soft-strong: color-mix(in srgb, {a} 28%, var(--bg-surface)) !important; --accent-text: {at} !important; --on-accent: {oa} !important; --bg-selected: var(--accent-soft) !important; --text-selected: var(--accent-text) !important; --text-link: var(--accent-text) !important;",
                a = t.accent, at = acc_txt, oa = on_acc
            ));
        }
        if !t.msg_sent.is_empty() {
            let bright_sent = hex_luma(&t.msg_sent).is_some_and(|l| l > 155.0);
            let sent_txt = if bright_sent { "#0B1018" } else { "#FFFFFF" };
            let sent_meta = if bright_sent { "rgba(11, 16, 24, 0.72)" } else { "rgba(255, 255, 255, 0.78)" };
            vars.push_str(&format!(
                "--msg-sent: {} !important; --msg-sent-text: {} !important; --msg-sent-meta: {} !important;",
                t.msg_sent, sent_txt, sent_meta
            ));
        }
        if !t.msg_recv.is_empty() {
            vars.push_str(&format!("--msg-recv: {} !important;", t.msg_recv));
        }
        if !t.panel_radius.is_empty() {
            vars.push_str(&format!(
                "--panel-radius: {r} !important; --radius-xl: {r} !important; --radius-lg: {r} !important; --radius-md: {r} !important; --radius-bubble: {r} !important;",
                r = t.panel_radius
            ));
        }
        match t.font_style.as_str() {
            "pixel" => vars.push_str("--font-family: 'JetBrains Mono', 'Courier New', ui-monospace, monospace !important;"),
            "rounded" => vars.push_str("--font-family: 'Nunito', 'Quicksand', 'Segoe UI', sans-serif !important;"),
            "serif" => vars.push_str("--font-family: 'Merriweather', Georgia, serif !important;"),
            _ => {}
        }
        if !vars.is_empty() {
            css.push_str(&format!(":root, [data-theme] {{ {vars} }}\n"));
        }
        match t.border_style.as_str() {
            "pixel" => {
                css.push_str(".nav-sidebar, .chat-column, .details-panel, .settings-nav, .settings-body, .search-page, .result-card, .bot-card, .mod-card, .inv-card, .message-bubble { border: 2px solid color-mix(in srgb, var(--accent) 45%, #000) !important; }\n");
            }
            "neon" => {
                css.push_str(".nav-sidebar, .chat-column, .settings-nav, .settings-body, .search-page, .result-card, .mod-card { border: 1px solid color-mix(in srgb, var(--accent) 65%, transparent) !important; box-shadow: 0 0 14px color-mix(in srgb, var(--accent) 22%, transparent) !important; }\n");
                css.push_str(".message-row.sent .message-bubble { box-shadow: 0 0 12px color-mix(in srgb, var(--msg-sent) 45%, transparent) !important; }\n");
            }
            "glass" => {
                let b = t.glass_blur.max(8);
                css.push_str(&format!(
                    ".nav-sidebar:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox)), .chat-column:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox)), .settings-nav:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox)), .settings-body:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox)), .search-page:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox)) {{ background: color-mix(in srgb, var(--bg-surface) 76%, transparent) !important; backdrop-filter: blur({b}px) !important; }}\n"
                ));
            }
            _ => {}
        }
        match t.rail_position.as_str() {
            "right" => {
                css.push_str("@media (min-width: 768px) { .workspace-shell { flex-direction: row-reverse !important; } }\n");
            }
            "top" => {
                css.push_str("@media (min-width: 768px) { .workspace-shell { flex-direction: column !important; } .app-rail { width: 100% !important; height: 3.75rem !important; flex-direction: row !important; padding: 0 var(--space-3) !important; } .rail-nav, .rail-bottom, .rail-pins { flex-direction: row !important; width: auto !important; margin: 0 !important; } .rail-brand { margin-bottom: 0 !important; margin-right: var(--space-2) !important; } }\n");
            }
            _ => {}
        }
        if !t.custom_css.trim().is_empty() {
            css.push_str(&t.custom_css);
            css.push('\n');
        }
    }
    let css_json = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into());
    let typing_json = serde_json::to_string(&typing_sound).unwrap_or_else(|_| "\"none\"".into());
    let _ = dioxus::document::eval(&format!(
        r#"
        (function() {{
            let el = document.getElementById('chatw-plugin-mods-css');
            if (!el) {{
                el = document.createElement('style');
                el.id = 'chatw-plugin-mods-css';
                document.head.appendChild(el);
            }}
            el.textContent = {css_json};
            window.__chatwTypingSound = {typing_json};
            window.__chatwSoundVol = {sound_vol:.3};
            window.__chatwSoundPitch = {sound_pitch:.3};
            if (!window.__chatwMechListener) {{
                window.__chatwMechListener = true;
                window.addEventListener('keydown', (e) => {{
                    const kind = window.__chatwTypingSound || 'none';
                    const vol = window.__chatwSoundVol ?? 0.75;
                    const pitch = window.__chatwSoundPitch ?? 1.0;
                    if (kind === 'none' || vol <= 0.01) return;
                    if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return;
                    try {{
                        const AC = window.AudioContext || window.webkitAudioContext;
                        if (!AC) return;
                        const ctx = window.__chatwAudioCtx || new AC();
                        if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                        const osc = ctx.createOscillator();
                        const g = ctx.createGain();
                        const t = ctx.currentTime;
                        if (kind === 'minecraft') {{
                            osc.type = 'square';
                            osc.frequency.setValueAtTime((e.key === 'Enter' ? 760 : 540) * pitch, t);
                            osc.frequency.exponentialRampToValueAtTime(Math.max(40, 320 * pitch), t + 0.03);
                        }} else if (kind === 'scifi') {{
                            osc.type = 'sawtooth';
                            osc.frequency.setValueAtTime((e.key === 'Enter' ? 1200 : 960) * pitch, t);
                            osc.frequency.exponentialRampToValueAtTime(Math.max(40, 260 * pitch), t + 0.035);
                        }} else {{
                            osc.type = 'triangle';
                            osc.frequency.setValueAtTime((e.key === 'Enter' ? 1100 : 1750) * pitch, t);
                            osc.frequency.exponentialRampToValueAtTime(Math.max(40, 360 * pitch), t + 0.025);
                        }}
                        g.gain.setValueAtTime(Math.max(0.002, 0.09 * vol), t);
                        g.gain.exponentialRampToValueAtTime(0.0001, t + 0.035);
                        osc.connect(g); g.connect(ctx.destination);
                        osc.start(t); osc.stop(t + 0.04);
                    }} catch (_) {{}}
                }}, true);
            }}
        }})();
        "#
    ));
}

fn plugin_error(code: &str) -> &'static str {
    match code {
        "invalid_bot_username" => "Tên định danh plugin chỉ gồm chữ cái không dấu, số hoặc gạch dưới từ 2 đến 32 ký tự",
        "username_taken_by_user" => "Tên này đang là tên tài khoản của người khác. Bạn chỉ có thể dùng tên tài khoản của chính mình",
        "bot_username_taken" => "Đã có plugin dùng tên này",
        "invalid_display_name" => "Tên hiển thị không hợp lệ",
        "bot_limit_reached" => "Bạn đã đạt số plugin máy chủ tối đa",
        "description_too_long" => "Mô tả quá dài",
        "bot_not_public" => "Plugin này chưa công khai hoặc chưa được xác minh",
        "insufficient_permissions" => "Chỉ trưởng hoặc phó nhóm được thêm plugin",
        _ => "Không thực hiện được, vui lòng thử lại",
    }
}

/// Global overlay rendered inside `Chat` workspace shell: initializes plugins and renders
/// optional ambient visual effects from active visual plugins (no floating HUD).
#[component]
pub fn PluginGlobalOverlay() -> Element {
    use_effect(move || {
        init_plugins();
    });
    let plugins = PLUGINS.read().clone();
    let active_fx = plugins
        .iter()
        .rev()
        .find(|p| p.installed && p.enabled && p.screen_fx != "none" && !p.screen_fx.is_empty())
        .map(|p| p.screen_fx.clone())
        .unwrap_or_else(|| "none".into());

    rsx! {
        if active_fx != "none" {
            div { class: "plugin-screen-fx fx-{active_fx}", aria_hidden: "true",
                for i in 0..18 {
                    i { key: "{i}", style: "--i: {i};",
                        match active_fx.as_str() {
                            "sakura" => "🌸",
                            "snow" => "❄",
                            "matrix" => "10110",
                            "voxel" => "✦",
                            _ => "",
                        }
                    }
                }
            }
        }
    }
}

fn active_wallpaper_attrs() -> (String, String) {
    let default_wp = try_use_context::<PrefsCtx>()
        .map(|c| c.prefs.read().wallpaper.clone())
        .unwrap_or_else(|| "none".into());
    wallpaper_attrs(None, &default_wp)
}

pub(crate) const SOUNDBOARD_PADS: [(&str, &str, &str); 8] = [
    ("xp", "Ngọc XP", "#10B981"),
    ("levelup", "Lên Cấp", "#22C55E"),
    ("anvil", "Tiếng Đe", "#64748B"),
    ("mech", "Switch Cơ", "#3B82F6"),
    ("laser", "Tia Laser", "#06B6D4"),
    ("coin", "Đồng Xu", "#F59E0B"),
    ("chime", "Chuông Báo", "#8B5CF6"),
    ("pop", "Hiệu Ứng Pop", "#EC4899"),
];

#[component]
fn PomodoroTimer(
    #[props(default = false)] compact: bool,
    #[props(default = 25)] focus_minutes: u32,
    #[props(default = 5)] break_minutes: u32,
    #[props(default = true)] sound_alert: bool,
) -> Element {
    let mut running = use_signal(|| false);
    let mut mode = use_signal(|| "focus");
    let mut secs = use_signal(move || focus_minutes.max(1) * 60);
    let mut last_focus = use_signal(move || focus_minutes);
    if *last_focus.read() != focus_minutes {
        last_focus.set(focus_minutes);
        if !running() && mode() == "focus" {
            secs.set(focus_minutes.max(1) * 60);
        }
    }
    let alert_flag = use_signal(move || sound_alert);
    use_effect(move || {
        spawn(async move {
            loop {
                platform::sleep_ms(1000).await;
                if *running.peek() {
                    let mut finished = false;
                    secs.with_mut(|s| {
                        if *s > 0 {
                            *s -= 1;
                            if *s == 0 {
                                finished = true;
                            }
                        }
                    });
                    if finished {
                        running.set(false);
                        if *alert_flag.peek() {
                            play_mod_sound("chime");
                        }
                    }
                }
            }
        });
    });
    let s = secs();
    let clock_str = format!("{:02}:{:02}", s / 60, s % 60);
    let is_run = running();
    let mode_label = if mode() == "break" { "Nghỉ ngắn" } else { "Tập trung" };
    rsx! {
        div { class: if compact { "left-app-card" } else { "ui-block-card" },
            "data-plugin-section": "pomodoro",
            div { class: "ui-counter-row",
                span { class: "settings-label", "Pomodoro · {mode_label}" }
                span { class: if compact { "ui-counter-val" } else { "ui-timer-display" }, "{clock_str}" }
            }
            div { class: "inline-form",
                button { r#type: "button", class: "btn-small-primary",
                    onclick: move |_| running.set(!is_run),
                    if is_run { "Tạm dừng" } else { "Bắt đầu" }
                }
                button { r#type: "button", class: "btn-secondary",
                    onclick: move |_| {
                        running.set(false);
                        if mode() == "focus" {
                            mode.set("break");
                            secs.set(break_minutes.max(1) * 60);
                        } else {
                            mode.set("focus");
                            secs.set(focus_minutes.max(1) * 60);
                        }
                    },
                    if mode() == "focus" { "Nghỉ {break_minutes}p" } else { "Tập trung {focus_minutes}p" }
                }
                button { r#type: "button", class: "btn-ghost",
                    onclick: move |_| {
                        running.set(false);
                        let m = if mode() == "break" { break_minutes } else { focus_minutes };
                        secs.set(m.max(1) * 60);
                    },
                    "Đặt lại"
                }
                button { r#type: "button", class: "btn-secondary",
                    onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(format!("Pomodoro ({mode_label}): {clock_str}")),
                    SendIcon { size: 14 } "Gửi"
                }
            }
        }
    }
}

/// Renders a workspace page opened by a Plugin, or the Plugin Manager (`studio`).
#[component]
pub fn PluginCustomPageView(plugin_id: String, my_username: String, on_close: EventHandler<()>, on_open_bot_chat: EventHandler<BotItem>) -> Element {
    let (wp_cls, wp_style) = active_wallpaper_attrs();

    if plugin_id == "studio" {
        return rsx! {
            div { class: "chat-main-container",
                div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                    header { class: "chat-header only-mobile",
                        button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại", onclick: move |_| on_close.call(()), BackIcon {} }
                    }
                    div { class: "plugin-custom-stage",
                        div { class: "settings-content", style: "max-width: 66rem; width: 100%; margin: 0 auto;",
                            BotsSettings {
                                my_username: my_username,
                                on_open_bot_chat: on_open_bot_chat,
                            }
                        }
                    }
                }
            }
        };
    }

    let plugin = PLUGINS.read().iter().find(|p| p.id == plugin_id).cloned();
    let Some(p) = plugin else {
        return rsx! {
            div { class: "chat-main-container",
                div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                    div { class: "empty-chat-view",
                        div { class: "empty-chat-inner",
                            div { class: "empty-chat-title", "Không tìm thấy Plugin" }
                            button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Quay lại" }
                        }
                    }
                }
            }
        };
    };

    let edit_pid = p.id.clone();
    let left_pid = p.id.clone();
    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column plugin-panel {wp_cls}", style: "{wp_style}",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại", onclick: move |_| on_close.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        span { class: "bot-header-icon", PluginGlyph { icon: p.page_icon.clone(), logo_url: p.logo_url.clone(), size: glyph_slot_size(&p.logo_url, 20, 38) } }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{p.display_name}" }
                        }
                    }
                    div { class: "chat-header-actions",
                        if p.left_app_enabled {
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                title: "Thu gọn sang cột bên trái",
                                onclick: move |_| {
                                    *ACTIVE_PLUGIN_PAGE.write() = None;
                                    *ACTIVE_LEFT_PLUGIN.write() = Some(left_pid.clone());
                                },
                                PanelRightIcon { size: 15 } "Thu gọn"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            onclick: move |_| {
                                *EDITING_PLUGIN_ID.write() = Some(edit_pid.clone());
                                *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                            },
                            SlidersIcon {} "Tùy chỉnh"
                        }
                    }
                }
                div { class: "plugin-custom-stage",
                    match p.page_kind.as_str() {
                        "voxel" => rsx! { VoxelSandboxPage { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                        "notes" => rsx! { NotesAndTodoPage { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                        "soundboard" => rsx! { SoundboardPage { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                        "cyber" => rsx! { CyberDashboardPage { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                        "tools" => rsx! { QuickToolsPage { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                        _ => rsx! { CustomBlocksView { plugin: p.clone(), my_username: my_username.clone(), compact: false } },
                    }
                }
            }
        }
    }
}

/// Compact Left-Sidebar App view rendered inside `.nav-sidebar` so users can run a Plugin
/// side-by-side while chatting in `.main-stage`.
#[component]
pub fn PluginLeftAppView(plugin_id: String, my_username: String, on_close: EventHandler<()>) -> Element {
    let plugin = PLUGINS.read().iter().find(|p| p.id == plugin_id).cloned();
    let Some(p) = plugin else {
        return rsx! {
            div { class: "sidebar-top-header",
                h2 { class: "sidebar-title", "Plugin" }
                div { class: "sidebar-header-actions",
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
            }
            div { class: "sidebar-empty", "Không tìm thấy Plugin" }
        };
    };
    let expand_pid = p.id.clone();
    let edit_pid = p.id.clone();
    let show_title = if p.page_title.trim().is_empty() { p.display_name.clone() } else { p.page_title.clone() };
    rsx! {
        div { class: "sidebar-top-header",
            div { class: "mod-card-title", style: "gap: 8px; flex: 1; min-width: 0;",
                PluginGlyph { icon: p.icon.clone(), logo_url: p.logo_url.clone(), size: 20 }
                h2 { class: "sidebar-title", "{show_title}" }
            }
            div { class: "sidebar-header-actions",
                if p.custom_page_enabled {
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        title: "Mở rộng",
                        aria_label: "Mở rộng",
                        onclick: move |_| {
                            *ACTIVE_LEFT_PLUGIN.write() = None;
                            *ACTIVE_PLUGIN_PAGE.write() = Some(expand_pid.clone());
                        },
                        PanelRightIcon { size: 17 }
                    }
                }
                button {
                    r#type: "button",
                    class: "icon-btn",
                    title: "Tùy chỉnh Plugin",
                    aria_label: "Tùy chỉnh Plugin",
                    onclick: move |_| {
                        *ACTIVE_LEFT_PLUGIN.write() = None;
                        *EDITING_PLUGIN_ID.write() = Some(edit_pid.clone());
                        *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                    },
                    SlidersIcon { size: 17 }
                }
                button {
                    r#type: "button",
                    class: "icon-btn",
                    title: "Đóng",
                    aria_label: "Đóng",
                    onclick: move |_| on_close.call(()),
                    CloseIcon { size: 17 }
                }
            }
        }
        div { class: "sidebar-scrollable",
            div { class: "left-app-body",
                match p.page_kind.as_str() {
                    "voxel" => rsx! { VoxelSandboxPage { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                    "notes" => rsx! { NotesAndTodoPage { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                    "soundboard" => rsx! { SoundboardPage { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                    "cyber" => rsx! { CyberDashboardPage { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                    "tools" => rsx! { QuickToolsPage { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                    _ => rsx! { CustomBlocksView { plugin: p.clone(), my_username: my_username.clone(), compact: true } },
                }
            }
        }
    }
}

pub(crate) const VOXEL_PALETTE: [(&str, &str); 12] = [
    ("Cỏ", "#22C55E"),
    ("Đất", "#92400E"),
    ("Đá", "#64748B"),
    ("Gỗ sồi", "#B45309"),
    ("Lá cây", "#15803D"),
    ("Kim cương", "#38BDF8"),
    ("Ngọc lục bảo", "#10B981"),
    ("Vàng", "#FACC15"),
    ("Đá đỏ", "#EF4444"),
    ("Hồng ngọc", "#DC2626"),
    ("Nước", "#2563EB"),
    ("Dung nham", "#F97316"),
];

#[component]
fn VoxelSandboxPage(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let default_col = plugin.cfg_str("default_color", "#10B981");
    let show_grid = plugin.cfg_bool("show_grid", true);
    let bevel_blocks = plugin.cfg_bool("bevel_blocks", true);
    let brush_sound = plugin.cfg_bool("brush_sound", true);
    let mut selected_color = use_signal(move || default_col);
    let mut grid = use_signal(|| -> Vec<String> {
        if let Some(raw) = platform::storage_get(VOXEL_KEY) {
            if let Ok(v) = serde_json::from_str::<Vec<String>>(&raw) {
                if v.len() == 192 {
                    return v;
                }
            }
        }
        let mut g = vec![String::new(); 192];
        for x in 0..16 {
            g[9 * 16 + x] = "#22C55E".into();
            g[10 * 16 + x] = "#92400E".into();
            g[11 * 16 + x] = "#64748B".into();
        }
        g
    });
    let save_grid = |g: &[String]| {
        if let Ok(s) = serde_json::to_string(g) {
            platform::storage_set(VOXEL_KEY, &s);
        }
    };
    let sections = plugin.ordered_sections(&["actions", "palette", "canvas"]);
    let grid_cls = match (show_grid, bevel_blocks) {
        (true, true) => "voxel-world-grid",
        (false, true) => "voxel-world-grid no-grid",
        (true, false) => "voxel-world-grid flat-blocks",
        (false, false) => "voxel-world-grid no-grid flat-blocks",
    };

    rsx! {
        div { class: if compact { "left-app-body" } else { "mod-page-container" },
            for sec in sections {
                if !plugin.is_section_hidden(&sec) {
                    if sec == "actions" {
                        div { key: "{sec}", class: "mod-hero-card", "data-plugin-section": "actions",
                            div {
                                h3 { class: "mod-hero-title", "{plugin.display_name}" }
                            }
                            div { class: "inline-form",
                                button { r#type: "button", class: "btn-secondary",
                                    onclick: move |_| {
                                        if brush_sound { play_mod_sound("xp"); }
                                        let mut g = vec![String::new(); 192];
                                        for x in 0..16 {
                                            g[9 * 16 + x] = "#22C55E".into();
                                            g[10 * 16 + x] = "#92400E".into();
                                            g[11 * 16 + x] = "#64748B".into();
                                        }
                                        save_grid(&g);
                                        grid.set(g);
                                    },
                                    "Địa hình mẫu"
                                }
                                button { r#type: "button", class: "btn-ghost",
                                    onclick: move |_| {
                                        let g = vec![String::new(); 192];
                                        save_grid(&g);
                                        grid.set(g);
                                    },
                                    "Xóa trắng"
                                }
                            }
                        }
                    } else if sec == "palette" {
                        div { key: "{sec}", class: "voxel-toolbar", "data-plugin-section": "palette",
                            for (name, hex) in VOXEL_PALETTE {
                                button {
                                    key: "{name}",
                                    r#type: "button",
                                    class: if selected_color() == hex { "voxel-block-btn active" } else { "voxel-block-btn" },
                                    onclick: move |_| {
                                        selected_color.set(hex.to_string());
                                        if brush_sound { play_mod_sound("mech"); }
                                    },
                                    span { class: "voxel-swatch", style: "background: {hex};" }
                                    if !compact { span { "{name}" } }
                                }
                            }
                            button {
                                r#type: "button",
                                class: if selected_color().is_empty() { "voxel-block-btn active" } else { "voxel-block-btn" },
                                onclick: move |_| selected_color.set(String::new()),
                                "Tẩy"
                            }
                        }
                    } else if sec == "canvas" {
                        div { key: "{sec}", class: "{grid_cls}", "data-plugin-section": "canvas",
                            for (idx, cell) in grid().into_iter().enumerate() {
                                button {
                                    key: "{idx}",
                                    r#type: "button",
                                    class: if cell.is_empty() { "voxel-cell" } else { "voxel-cell filled" },
                                    style: if cell.is_empty() { String::new() } else { format!("background: {cell};") },
                                    onclick: move |_| {
                                        let col = selected_color();
                                        if brush_sound {
                                            play_mod_sound(if col.is_empty() { "anvil" } else { "minecraft" });
                                        }
                                        grid.with_mut(|g| {
                                            if let Some(slot) = g.get_mut(idx) {
                                                *slot = if *slot == col { String::new() } else { col.clone() };
                                            }
                                            save_grid(g);
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !plugin.custom_blocks.is_empty() {
                CustomBlocksView { plugin: plugin.clone(), my_username: my_username.clone(), compact: compact }
            }
        }
    }
}

#[component]
fn NotesAndTodoPage(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let mut notes = use_signal(|| platform::storage_get(NOTES_KEY).unwrap_or_default());
    let mut todos = use_signal(|| -> Vec<TodoItem> {
        platform::storage_get(TODOS_KEY)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| vec![
                TodoItem { id: "1".into(), text: "Ghi chú việc ưu tiên hôm nay".into(), done: false },
            ])
    });
    let mut new_todo = use_signal(String::new);
    let save_todos = |list: &[TodoItem]| {
        if let Ok(s) = serde_json::to_string(list) {
            platform::storage_set(TODOS_KEY, &s);
        }
    };
    let focus_mins = plugin.cfg_i64("focus_minutes", 25).clamp(1, 120) as u32;
    let break_mins = plugin.cfg_i64("break_minutes", 5).clamp(1, 60) as u32;
    let sound_alert = plugin.cfg_bool("sound_alert", true);
    let layout_mode = plugin.cfg_str("layout_mode", "two_col");
    let sections = plugin.ordered_sections(&["pomodoro", "todos", "notes"]);
    let visible_sections: Vec<String> = sections.into_iter().filter(|s| !plugin.is_section_hidden(s)).collect();
    let container_cls = if compact {
        "left-app-body"
    } else if layout_mode == "two_col" {
        "mod-page-container notes-layout-grid"
    } else {
        "mod-page-container"
    };

    rsx! {
        div { class: "{container_cls}",
            for sec in visible_sections {
                if sec == "pomodoro" {
                    PomodoroTimer {
                        key: "{sec}",
                        compact: compact,
                        focus_minutes: focus_mins,
                        break_minutes: break_mins,
                        sound_alert: sound_alert,
                    }
                } else if sec == "todos" {
                    div {
                        key: "{sec}",
                        class: if compact { "left-app-card" } else { "settings-group" },
                        "data-plugin-section": "todos",
                        div { class: "settings-group-title", "Việc cần làm" }
                        form {
                            class: "inline-form",
                            onsubmit: move |e| {
                                e.prevent_default();
                                let t = new_todo().trim().to_string();
                                if t.is_empty() { return; }
                                let id = format!("{}", chrono::Utc::now().timestamp_millis());
                                todos.with_mut(|l| {
                                    l.insert(0, TodoItem { id, text: t, done: false });
                                    save_todos(l);
                                });
                                new_todo.set(String::new());
                            },
                            input { r#type: "text", placeholder: "Thêm công việc mới", value: "{new_todo}", oninput: move |e| new_todo.set(e.value()) }
                            button { r#type: "submit", class: "btn-primary", PlusIcon { size: 16 } if !compact { "Thêm" } }
                        }
                        div { class: "mod-todo-list",
                            if todos.read().is_empty() {
                                div { class: "sidebar-empty", "Chưa có công việc nào" }
                            }
                            for item in todos() {
                                {
                                    let id1 = item.id.clone();
                                    let id2 = item.id.clone();
                                    rsx! {
                                        div { key: "{item.id}", class: if item.done { "mod-todo-row done" } else { "mod-todo-row" },
                                            input {
                                                r#type: "checkbox",
                                                checked: item.done,
                                                onchange: move |e| {
                                                    let d = e.checked();
                                                    todos.with_mut(|l| {
                                                        if let Some(x) = l.iter_mut().find(|x| x.id == id1) { x.done = d; }
                                                        save_todos(l);
                                                    });
                                                }
                                            }
                                            span { class: "grow", "{item.text}" }
                                            button {
                                                r#type: "button",
                                                class: "icon-btn danger",
                                                onclick: move |_| {
                                                    todos.with_mut(|l| {
                                                        l.retain(|x| x.id != id2);
                                                        save_todos(l);
                                                    });
                                                },
                                                TrashIcon { size: 15 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if sec == "notes" {
                    div {
                        key: "{sec}",
                        class: if compact { "left-app-card" } else { "settings-group" },
                        "data-plugin-section": "notes",
                        div { class: "settings-group-title", "Ghi chú nhanh" }
                        textarea {
                            class: "mod-notes-area",
                            rows: if compact { "5" } else { "10" },
                            placeholder: "Viết ghi chú hoặc nội dung cần lưu tại đây",
                            value: "{notes}",
                            oninput: move |e| {
                                let v = e.value();
                                notes.set(v.clone());
                                platform::storage_set(NOTES_KEY, &v);
                            }
                        }
                        div { class: "inline-form",
                            button { r#type: "button", class: "btn-small-primary",
                                onclick: move |_| {
                                    let v = notes();
                                    if !v.trim().is_empty() {
                                        *PENDING_COMPOSER_TEXT.write() = Some(v);
                                    }
                                },
                                SendIcon { size: 14 } "Gửi vào chat"
                            }
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::copy_to_clipboard(&notes()), CopyIcon {} "Chép" }
                            button { r#type: "button", class: "btn-ghost", onclick: move |_| { notes.set(String::new()); platform::storage_set(NOTES_KEY, ""); }, "Xóa" }
                        }
                    }
                }
            }
            if !plugin.custom_blocks.is_empty() {
                CustomBlocksView { plugin: plugin.clone(), my_username: my_username.clone(), compact: compact }
            }
        }
    }
}

#[component]
fn SoundboardPage(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let default_keys: Vec<&str> = SOUNDBOARD_PADS.iter().map(|(k, _, _)| *k).collect();
    let ordered = plugin.ordered_sections(&default_keys);
    let volume = (plugin.cfg_i64("volume", 75) as f64 / 100.0).clamp(0.0, 1.0);
    let pitch = (plugin.cfg_i64("pitch", 100) as f64 / 100.0).clamp(0.5, 2.0);
    let cols = if compact { 2 } else { plugin.cfg_i64("columns", 4).clamp(2, 4) };
    let grid_style = format!("grid-template-columns: repeat({cols}, minmax(0, 1fr));");

    rsx! {
        div { class: if compact { "left-app-body" } else { "mod-page-container" },
            div {
                class: if compact { "soundboard-grid compact" } else { "soundboard-grid" },
                style: "{grid_style}",
                for pad_key in ordered {
                    if !plugin.is_section_hidden(&pad_key) {
                        if let Some((kind, label, color)) = SOUNDBOARD_PADS.iter().find(|(k, _, _)| *k == pad_key.as_str()).copied() {
                            button {
                                key: "{kind}",
                                r#type: "button",
                                class: "soundboard-pad",
                                "data-plugin-section": "{kind}",
                                style: "--pad-color: {color};",
                                onclick: move |_| play_mod_sound_opts(kind, volume, pitch),
                                div { class: "soundboard-pad-head",
                                    span { class: "soundboard-dot" }
                                    span { class: "soundboard-pad-title", "{label}" }
                                }
                                if !compact {
                                    span { class: "soundboard-pad-sub", "Phát âm thanh" }
                                }
                            }
                        }
                    }
                }
            }
            if !plugin.custom_blocks.is_empty() {
                CustomBlocksView { plugin: plugin.clone(), my_username: my_username.clone(), compact: compact }
            }
        }
    }
}

#[component]
fn CyberDashboardPage(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let mut terminal_out = use_signal(|| format!("Trạm lệnh {} sẵn sàng (@{}).", plugin.display_name, my_username));
    let mut last_reply = use_signal(String::new);
    let mut cmd_in = use_signal(String::new);
    let sections = plugin.ordered_sections(&["stats", "console"]);

    rsx! {
        div { class: if compact { "left-app-body" } else { "mod-page-container" },
            for sec in sections {
                if !plugin.is_section_hidden(&sec) {
                    if sec == "stats" && !compact {
                        div { key: "{sec}", class: "inv-grid", "data-plugin-section": "stats",
                            div { class: "inv-card",
                                div { class: "settings-label", "Mã hóa đầu cuối" }
                                div { class: "member-sub", "ECDH P-256 · AES-256-GCM" }
                            }
                            div { class: "inv-card",
                                div { class: "settings-label", "Plugin đang hoạt động" }
                                div { class: "member-sub", "{PLUGINS.read().iter().filter(|p| p.installed && p.enabled).count()} Plugin" }
                            }
                            div { class: "inv-card",
                                div { class: "settings-label", "Hiệu ứng nền" }
                                div { class: "member-sub", if plugin.screen_fx == "matrix" { "Matrix đang bật" } else { "Đang tắt" } }
                            }
                        }
                    } else if sec == "console" {
                        div { key: "{sec}", class: if compact { "left-app-card" } else { "settings-group" }, "data-plugin-section": "console",
                            div { class: "settings-group-title", "Bảng lệnh nhanh" }
                            pre { class: "code-block", style: if compact { "min-height: 6rem; max-height: 11rem; overflow-y: auto;" } else { "min-height: 10rem;" }, "{terminal_out}" }
                            form {
                                class: "inline-form",
                                onsubmit: {
                                    let uname = my_username.clone();
                                    move |e| {
                                        e.prevent_default();
                                        let c = cmd_in().trim().to_string();
                                        if c.is_empty() { return; }
                                        let reply = try_run_plugin_command(&c, &uname).unwrap_or_else(|| format!("Đã thực thi: {c}"));
                                        last_reply.set(reply.clone());
                                        terminal_out.with_mut(|t| {
                                            t.push_str(&format!("\n\n> {c}\n{reply}"));
                                        });
                                        cmd_in.set(String::new());
                                    }
                                },
                                input { r#type: "text", placeholder: "Nhập lệnh /neon, /status, /calc 24*7", value: "{cmd_in}", oninput: move |e| cmd_in.set(e.value()) }
                                button { r#type: "submit", class: "btn-primary", "Chạy" }
                                if !last_reply().is_empty() {
                                    button { r#type: "button", class: "btn-secondary",
                                        onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(last_reply()),
                                        SendIcon { size: 14 } "Chèn"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !plugin.custom_blocks.is_empty() {
                CustomBlocksView { plugin: plugin.clone(), my_username: my_username.clone(), compact: compact }
            }
        }
    }
}

/// Real interactive Quick Tools workspace for `mod:smart_assistant` ("Công cụ Lệnh nhanh").
#[component]
fn QuickToolsPage(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let mut calc_expr = use_signal(String::new);
    let mut rand_result = use_signal(String::new);
    let mut text_input = use_signal(String::new);
    let mut cmd_output = use_signal(String::new);
    let sections = plugin.ordered_sections(&["calc", "random", "text", "snippets"]);

    let calc_preview = eval_simple_math(&calc_expr())
        .map(|v| if (v - v.round()).abs() < 1e-9 { format!("{}", v as i64) } else { format!("{v:.2}") })
        .unwrap_or_default();

    rsx! {
        div { class: if compact { "left-app-body" } else { "mod-page-container" },
            for sec in sections {
                if !plugin.is_section_hidden(&sec) {
                    if sec == "calc" {
                        div { key: "{sec}", class: if compact { "left-app-card" } else { "settings-group" }, "data-plugin-section": "calc",
                            div { class: "settings-group-title", "Máy tính nhanh" }
                            div { class: "inline-form",
                                input {
                                    r#type: "text",
                                    placeholder: "vd: 1250*4 + 500",
                                    value: "{calc_expr}",
                                    oninput: move |e| calc_expr.set(e.value()),
                                }
                                if !calc_preview.is_empty() {
                                    button {
                                        r#type: "button",
                                        class: "btn-small-primary",
                                        onclick: {
                                            let res = format!("{} = {}", calc_expr().trim(), calc_preview);
                                            move |_| *PENDING_COMPOSER_TEXT.write() = Some(res.clone())
                                        },
                                        SendIcon { size: 14 } "Chèn"
                                    }
                                }
                            }
                            if !calc_preview.is_empty() {
                                div { class: "plugin-calc-result", "= {calc_preview}" }
                            }
                        }
                    } else if sec == "random" {
                        div { key: "{sec}", class: if compact { "left-app-card" } else { "settings-group" }, "data-plugin-section": "random",
                            div { class: "settings-group-title", "Xúc xắc & Ngẫu nhiên" }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        let n = (chrono::Local::now().timestamp_subsec_nanos() % 6) + 1;
                                        rand_result.set(format!("Xúc xắc: {n}"));
                                    },
                                    DiceIcon { size: 15 } "Xúc xắc"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        let f = if chrono::Local::now().timestamp_subsec_nanos().is_multiple_of(2) { "Ngửa" } else { "Sấp" };
                                        rand_result.set(format!("Đồng xu: {f}"));
                                    },
                                    CoinIcon { size: 15 } "Đồng xu"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        let n = (chrono::Local::now().timestamp_subsec_nanos() % 100) + 1;
                                        rand_result.set(format!("Số ngẫu nhiên: {n}/100"));
                                    },
                                    "1–100"
                                }
                            }
                            if !rand_result().is_empty() {
                                div { class: "inline-form",
                                    span { class: "settings-label grow", "{rand_result}" }
                                    button {
                                        r#type: "button",
                                        class: "btn-small-primary",
                                        onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(rand_result()),
                                        SendIcon { size: 14 } "Chèn"
                                    }
                                }
                            }
                        }
                    } else if sec == "text" {
                        div { key: "{sec}", class: if compact { "left-app-card" } else { "settings-group" }, "data-plugin-section": "text",
                            div { class: "settings-group-title", "Xử lý văn bản" }
                            textarea {
                                rows: "3",
                                placeholder: "Nhập văn bản cần chuyển đổi",
                                value: "{text_input}",
                                oninput: move |e| text_input.set(e.value()),
                            }
                            div { class: "inline-form",
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| text_input.set(text_input().to_uppercase()), "IN HOA" }
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| text_input.set(text_input().to_lowercase()), "chữ thường" }
                                button { r#type: "button", class: "btn-small-primary",
                                    disabled: text_input().trim().is_empty(),
                                    onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(text_input()),
                                    SendIcon { size: 14 } "Chèn"
                                }
                                span { class: "member-sub", "{text_input().chars().count()} ký tự" }
                            }
                        }
                    } else if sec == "snippets" {
                        div { key: "{sec}", class: if compact { "left-app-card" } else { "settings-group" }, "data-plugin-section": "snippets",
                            div { class: "settings-group-title", "Lệnh gõ tắt đang bật" }
                            div { class: "mod-preset-chips",
                                for c in plugin.commands.iter().filter(|c| c.enabled) {
                                    {
                                        let trig = c.trigger.clone();
                                        let resp = c.response.clone();
                                        let uname = my_username.clone();
                                        rsx! {
                                            button {
                                                key: "{trig}",
                                                r#type: "button",
                                                class: "btn-small-secondary",
                                                onclick: move |_| {
                                                    let out = expand_template(&resp, &uname, "");
                                                    cmd_output.set(out.clone());
                                                    *PENDING_COMPOSER_TEXT.write() = Some(out);
                                                },
                                                code { "{trig}" }
                                            }
                                        }
                                    }
                                }
                            }
                            if !cmd_output().is_empty() {
                                div { class: "member-sub", "Đã chèn: {cmd_output}" }
                            }
                        }
                    }
                }
            }
            if !plugin.custom_blocks.is_empty() {
                CustomBlocksView { plugin: plugin.clone(), my_username: my_username.clone(), compact: compact }
            }
        }
    }
}

fn load_block_state(plugin_id: &str) -> serde_json::Map<String, serde_json::Value> {
    let key = format!("chatw_plugin_state_{plugin_id}");
    platform::storage_get(&key)
        .and_then(|s| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&s).ok())
        .unwrap_or_default()
}

fn save_block_state(plugin_id: &str, map: &serde_json::Map<String, serde_json::Value>) {
    let key = format!("chatw_plugin_state_{plugin_id}");
    if let Ok(s) = serde_json::to_string(map) {
        platform::storage_set(&key, &s);
    }
}

fn parse_timer_secs(raw: &str) -> u32 {
    let t = raw.trim().to_lowercase();
    if let Some(m) = t.strip_suffix('m') {
        m.trim().parse::<u32>().unwrap_or(5).saturating_mul(60).max(10)
    } else if let Some(s) = t.strip_suffix('s') {
        s.trim().parse::<u32>().unwrap_or(60).max(5)
    } else if let Ok(v) = t.parse::<u32>() {
        v.max(5)
    } else {
        300
    }
}

fn run_block_action(plugin: &PluginMod, block: &PluginCustomBlock, user: &str, arg: &str, mut output: Signal<String>) {
    let cmd = block.action_command.trim();
    let atype = if block.action_type.is_empty() { "command" } else { block.action_type.as_str() };
    match atype {
        "insert_chat" => {
            let txt = expand_template(cmd, user, arg);
            *PENDING_COMPOSER_TEXT.write() = Some(txt);
            output.set("Đã chèn nội dung vào ô nhập chat".into());
        }
        "copy" => {
            let txt = expand_template(cmd, user, arg);
            platform::copy_to_clipboard(&txt);
            output.set("Đã sao chép vào bộ nhớ tạm".into());
        }
        "sound" => {
            play_mod_sound(if cmd.is_empty() { "pop" } else { cmd });
            output.set("Đã phát âm thanh".into());
        }
        "url" => {
            if cmd.starts_with("http://") || cmd.starts_with("https://") {
                platform::open_external_url(cmd);
                output.set(format!("Đã mở liên kết: {cmd}"));
            } else {
                output.set("Liên kết phải bắt đầu bằng http hoặc https".into());
            }
        }
        "webhook" => {
            if cmd.starts_with("http://") || cmd.starts_with("https://") {
                let url = cmd.to_string();
                let payload = json!({
                    "plugin_id": plugin.id,
                    "plugin_name": plugin.display_name,
                    "username": user,
                    "block": block.title,
                    "input": arg,
                });
                output.set("Đang gửi Webhook".into());
                spawn(async move {
                    match platform::http_request("POST", &url, Some(payload.to_string()), None).await {
                        Ok((code, body)) => {
                            let preview = body.trim();
                            if preview.is_empty() {
                                output.set(format!("Webhook thành công {code}"));
                            } else {
                                output.set(preview.chars().take(240).collect());
                            }
                        }
                        Err(e) => output.set(format!("Lỗi Webhook {}", e.status)),
                    }
                });
            } else {
                output.set("URL Webhook phải bắt đầu bằng http hoặc https".into());
            }
        }
        _ => {
            let full_cmd = if arg.is_empty() {
                cmd.to_string()
            } else if cmd.contains("{args}") {
                expand_template(cmd, user, arg)
            } else if cmd.starts_with('/') {
                format!("{cmd} {arg}")
            } else {
                arg.to_string()
            };
            let (c_head, c_rest) = match full_cmd.trim().split_once(char::is_whitespace) {
                Some((h, r)) => (h.trim(), r.trim()),
                None => (full_cmd.trim(), ""),
            };
            let res = plugin
                .commands
                .iter()
                .find(|c| c.trigger.eq_ignore_ascii_case(c_head))
                .map(|c| expand_template(&c.response, user, c_rest))
                .or_else(|| try_run_plugin_command(&full_cmd, user))
                .unwrap_or_else(|| expand_template(&full_cmd, user, arg));
            output.set(res);
        }
    }
}

#[component]
fn CustomBlocksView(plugin: PluginMod, my_username: String, #[props(default = false)] compact: bool) -> Element {
    let pid = plugin.id.clone();
    let mut output = use_signal(String::new);
    let mut block_state = use_signal(move || load_block_state(&pid));
    let mut input_vals = use_signal(std::collections::BTreeMap::<usize, String>::new);
    let mut timers = use_signal(std::collections::BTreeMap::<usize, (bool, u32)>::new);

    use_effect(move || {
        spawn(async move {
            loop {
                platform::sleep_ms(1000).await;
                let has_running = timers.peek().values().any(|(r, _)| *r);
                if has_running {
                    let mut chime = false;
                    timers.with_mut(|map| {
                        for (run, rem) in map.values_mut() {
                            if *run && *rem > 0 {
                                *rem -= 1;
                                if *rem == 0 {
                                    *run = false;
                                    chime = true;
                                }
                            }
                        }
                    });
                    if chime {
                        play_mod_sound("chime");
                    }
                }
            }
        });
    });

    rsx! {
        div { class: if compact { "left-app-body" } else { "mod-page-container" },
            if !compact && plugin.page_kind == "custom" {
                div { class: "mod-hero-card",
                    div {
                        h3 { class: "mod-hero-title", "{plugin.icon} {plugin.display_name}" }
                        p { class: "mod-hero-sub", "{plugin.description}" }
                    }
                }
            }
            if !output().is_empty() {
                div { class: "left-app-card",
                    div { class: "ui-counter-row",
                        span { class: "status-badge success", "{output}" }
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            title: "Gửi kết quả vào ô nhập chat",
                            onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(output()),
                            SendIcon { size: 14 } "Chèn"
                        }
                    }
                }
            }
            if plugin.custom_blocks.is_empty() {
                div { class: if compact { "left-app-body" } else { "ui-blocks-grid" },
                    for c in plugin.commands.iter() {
                        {
                            let trig = c.trigger.clone();
                            let desc = c.description.clone();
                            let u = my_username.clone();
                            rsx! {
                                div { key: "{trig}", class: if compact { "left-app-card" } else { "ui-block-card" },
                                    div { class: "settings-label", "{trig}" }
                                    div { class: "member-sub", "{desc}" }
                                    div { class: "inline-form",
                                        button {
                                            r#type: "button",
                                            class: "btn-small-primary",
                                            onclick: {
                                                let tr = trig.clone();
                                                let u = u.clone();
                                                move |_| {
                                                    if let Some(res) = try_run_plugin_command(&tr, &u) {
                                                        output.set(res);
                                                    }
                                                }
                                            },
                                            "Chạy {trig}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(trig.clone()),
                                            SendIcon { size: 14 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: if compact { "left-app-body" } else { "ui-blocks-grid" },
                    for (i, b) in plugin.custom_blocks.iter().enumerate() {
                        {
                            let p_clone = plugin.clone();
                            let b_clone = b.clone();
                            let u = my_username.clone();
                            let kind = if b.kind.is_empty() { "card" } else { b.kind.as_str() };
                            let bkey = format!("b_{i}");
                            rsx! {
                                div { key: "{i}", class: if compact { "left-app-card" } else { "ui-block-card" },
                                    div { class: "settings-label", "{b.title}" }
                                    if kind == "counter" {
                                        {
                                            let val = block_state().get(&bkey).and_then(|v| v.as_i64()).unwrap_or(0);
                                            let pid_s = p_clone.id.clone();
                                            let bk_dec = bkey.clone();
                                            let bk_inc = bkey.clone();
                                            let bk_rst = bkey.clone();
                                            let b_title = b.title.clone();
                                            rsx! {
                                                if !b.body.is_empty() { div { class: "member-sub", "{b.body}" } }
                                                div { class: "ui-counter-row",
                                                    span { class: "ui-counter-val", "{val}" }
                                                    div { class: "inline-form",
                                                        button { r#type: "button", class: "btn-secondary",
                                                            onclick: {
                                                                let pid_s = pid_s.clone();
                                                                move |_| {
                                                                    block_state.with_mut(|m| {
                                                                        let cur = m.get(&bk_dec).and_then(|v| v.as_i64()).unwrap_or(0);
                                                                        m.insert(bk_dec.clone(), json!(cur - 1));
                                                                        save_block_state(&pid_s, m);
                                                                    });
                                                                }
                                                            },
                                                            "−"
                                                        }
                                                        button { r#type: "button", class: "btn-small-primary",
                                                            onclick: {
                                                                let pid_s = pid_s.clone();
                                                                move |_| {
                                                                    block_state.with_mut(|m| {
                                                                        let cur = m.get(&bk_inc).and_then(|v| v.as_i64()).unwrap_or(0);
                                                                        m.insert(bk_inc.clone(), json!(cur + 1));
                                                                        save_block_state(&pid_s, m);
                                                                    });
                                                                }
                                                            },
                                                            "+"
                                                        }
                                                        button { r#type: "button", class: "btn-ghost",
                                                            onclick: {
                                                                let pid_s = pid_s.clone();
                                                                move |_| {
                                                                    block_state.with_mut(|m| {
                                                                        m.insert(bk_rst.clone(), json!(0));
                                                                        save_block_state(&pid_s, m);
                                                                    });
                                                                }
                                                            },
                                                            "0"
                                                        }
                                                        button { r#type: "button", class: "btn-secondary",
                                                            onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(format!("{b_title}: {val}")),
                                                            SendIcon { size: 14 }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if kind == "checklist" {
                                        {
                                            let items: Vec<String> = b.body
                                                .split(|c| c == '\n' || c == ',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                            let pid_s = p_clone.id.clone();
                                            rsx! {
                                                div { class: "mod-todo-list",
                                                    for (j, item_txt) in items.into_iter().enumerate() {
                                                        {
                                                            let ckey = format!("{bkey}_{j}");
                                                            let checked = block_state().get(&ckey).and_then(|v| v.as_bool()).unwrap_or(false);
                                                            let pid_c = pid_s.clone();
                                                            rsx! {
                                                                label { key: "{j}", class: if checked { "mod-todo-row done" } else { "mod-todo-row" },
                                                                    input {
                                                                        r#type: "checkbox",
                                                                        checked: checked,
                                                                        onchange: move |e| {
                                                                            let d = e.checked();
                                                                            block_state.with_mut(|m| {
                                                                                m.insert(ckey.clone(), json!(d));
                                                                                save_block_state(&pid_c, m);
                                                                            });
                                                                        }
                                                                    }
                                                                    span { class: "grow", "{item_txt}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if kind == "timer" {
                                        {
                                            let default_sec = parse_timer_secs(&b.action_command);
                                            let (t_run, t_rem) = timers().get(&i).copied().unwrap_or((false, default_sec));
                                            let clock = format!("{:02}:{:02}", t_rem / 60, t_rem % 60);
                                            let lbl = if b.action_label.is_empty() { "Bắt đầu".to_string() } else { b.action_label.clone() };
                                            let t_title = b.title.clone();
                                            rsx! {
                                                if !b.body.is_empty() { div { class: "member-sub", "{b.body}" } }
                                                div { class: "ui-counter-row",
                                                    span { class: "ui-timer-display", "{clock}" }
                                                    div { class: "inline-form",
                                                        button { r#type: "button", class: "btn-small-primary",
                                                            onclick: move |_| {
                                                                timers.with_mut(|m| {
                                                                    let cur = m.get(&i).copied().unwrap_or((false, default_sec));
                                                                    m.insert(i, (!cur.0, cur.1));
                                                                });
                                                            },
                                                            if t_run { "Tạm dừng" } else { "{lbl}" }
                                                        }
                                                        button { r#type: "button", class: "btn-ghost",
                                                            onclick: move |_| {
                                                                timers.with_mut(|m| {
                                                                    m.insert(i, (false, default_sec));
                                                                });
                                                            },
                                                            "Đặt lại"
                                                        }
                                                        button { r#type: "button", class: "btn-secondary",
                                                            onclick: move |_| *PENDING_COMPOSER_TEXT.write() = Some(format!("⏱ {t_title}: {clock}")),
                                                            SendIcon { size: 14 }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if kind == "input_action" {
                                        {
                                            let cur_val = input_vals().get(&i).cloned().unwrap_or_default();
                                            let lbl = if b.action_label.is_empty() { "Thực hiện".to_string() } else { b.action_label.clone() };
                                            rsx! {
                                                if !b.body.is_empty() { div { class: "member-sub", "{b.body}" } }
                                                form {
                                                    class: "inline-form",
                                                    onsubmit: move |e| {
                                                        e.prevent_default();
                                                        let arg = input_vals().get(&i).cloned().unwrap_or_default();
                                                        run_block_action(&p_clone, &b_clone, &u, &arg, output);
                                                    },
                                                    input {
                                                        r#type: "text",
                                                        placeholder: "Nhập nội dung",
                                                        value: "{cur_val}",
                                                        oninput: move |e| {
                                                            let v = e.value();
                                                            input_vals.with_mut(|m| { m.insert(i, v); });
                                                        }
                                                    }
                                                    button { r#type: "submit", class: "btn-small-primary", "{lbl}" }
                                                }
                                            }
                                        }
                                    } else if kind == "link" {
                                        {
                                            let href = b.action_command.clone();
                                            let lbl = if b.action_label.is_empty() { "Mở liên kết".to_string() } else { b.action_label.clone() };
                                            rsx! {
                                                if !b.body.is_empty() { div { class: "member-sub", "{b.body}" } }
                                                button {
                                                    r#type: "button",
                                                    class: "btn-secondary",
                                                    onclick: move |_| {
                                                        if href.starts_with("http://") || href.starts_with("https://") {
                                                            platform::open_external_url(&href);
                                                        }
                                                    },
                                                    "{lbl}"
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            let lbl = if b.action_label.is_empty() && kind == "button" { "Thực hiện".to_string() } else { b.action_label.clone() };
                                            rsx! {
                                                if !b.body.is_empty() { div { class: "member-sub", "{b.body}" } }
                                                if !lbl.is_empty() {
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-small-primary",
                                                        onclick: move |_| run_block_action(&p_clone, &b_clone, &u, "", output),
                                                        "{lbl}"
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
        }
    }
}

/// Chat with a Plugin (works for both server `/api/bots` AND local device Plugins).
#[component]
pub fn PluginChatView(
    bot: BotItem,
    my_id: String,
    my_name: String,
    incoming: Option<serde_json::Value>,
    groups: Vec<GroupItem>,
    on_back: EventHandler<()>,
) -> Element {
    let mut msgs = use_signal(Vec::<BotChatMessage>::new);
    let mut input = use_signal(String::new);
    let sending = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut add_open = use_signal(|| false);
    let is_local = bot.id.starts_with("mod:");
    let local_storage_key = format!("chatw_plugin_chat_{}", bot.id);
    let (wp_cls, wp_style) = active_wallpaper_attrs();

    let bot_id = bot.id.clone();
    let storage_key_load = local_storage_key.clone();
    use_effect(use_reactive!(|bot_id| {
        let sk = storage_key_load.clone();
        if bot_id.starts_with("mod:") {
            let loaded: Vec<BotChatMessage> = platform::storage_get(&sk)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            msgs.set(loaded);
            platform::scroll_chat_to_bottom(true);
        } else {
            spawn(async move {
                match social::bot_messages(&bot_id).await {
                    Ok(list) => msgs.set(list),
                    Err(_) => msgs.set(Vec::new()),
                }
                platform::scroll_chat_to_bottom(true);
            });
        }
    }));

    let bid = bot.id.clone();
    use_effect(use_reactive!(|incoming| {
        if let Some(v) = incoming.as_ref() {
            if v.get("bot_id").and_then(|x| x.as_str()) == Some(bid.as_str()) {
                if let Ok(m) = serde_json::from_value::<BotChatMessage>(v.clone()) {
                    msgs.with_mut(|l| if !l.iter().any(|x| x.id == m.id) { l.push(m) });
                    platform::scroll_chat_to_bottom(false);
                }
            }
        }
    }));

    let matching_mod = PLUGINS
        .read()
        .iter()
        .find(|p| p.installed && p.enabled && (p.id == bot.id || p.username == bot.username || p.server_bot_id.as_deref() == Some(bot.id.as_str())))
        .cloned();

    let send_id = bot.id.clone();
    let my_name_send = my_name.clone();
    let sk_send = local_storage_key.clone();
    let mod_for_send = matching_mod.clone();
    let send_text: std::rc::Rc<dyn Fn(String)> = std::rc::Rc::new({
        let send_id = send_id.clone();
        let my_name_send = my_name_send.clone();
        let sk_send = sk_send.clone();
        let mod_for_send = mod_for_send.clone();
        move |text: String| {
            let mut sending = sending;
            let mut input = input;
            let mut msgs = msgs;
            let text = text.trim().to_string();
            if text.is_empty() || sending() {
                return;
            }
            sending.set(true);
            input.set(String::new());
            let id = send_id.clone();
            let u_name = my_name_send.clone();
            let sk = sk_send.clone();
            let m_opt = mod_for_send.clone();

            let (cmd_reply, webhook_url, fallback_reply) = if let Some(ref pm) = m_opt {
                let (cmd, args) = match text.split_once(char::is_whitespace) {
                    Some((c, a)) => (c.trim(), a.trim()),
                    None => (text.as_str(), ""),
                };
                let direct_cmd = pm
                    .commands
                    .iter()
                    .find(|c| c.trigger.eq_ignore_ascii_case(cmd))
                    .map(|c| expand_template(&c.response, &u_name, args))
                    .or_else(|| {
                        if text.starts_with('/') {
                            try_run_plugin_command(&text, &u_name)
                        } else {
                            None
                        }
                    });
                let auto_tpl = if !pm.auto_reply.trim().is_empty() {
                    Some(expand_template(&pm.auto_reply, &u_name, &text))
                } else {
                    None
                };
                let wh = pm.webhook_url.trim().to_string();
                (direct_cmd, wh, auto_tpl)
            } else {
                (try_run_plugin_command(&text, &u_name), String::new(), None)
            };

            if id.starts_with("mod:") {
                let now = chrono::Utc::now().to_rfc3339();
                let now_ts = chrono::Utc::now().timestamp_millis();
                let user_msg = BotChatMessage {
                    id: format!("u-{}", now_ts),
                    seq: now_ts,
                    from_bot: false,
                    text: text.clone(),
                    created_at: now.clone(),
                };
                let immediate_reply = cmd_reply.or_else(|| {
                    if !webhook_url.is_empty() {
                        None
                    } else {
                        fallback_reply.clone().or_else(|| {
                            Some(expand_template("Đã nhận: {args}. Dùng /help để xem lệnh.", &u_name, &text))
                        })
                    }
                });
                msgs.with_mut(|l| {
                    l.push(user_msg);
                    if let Some(ref rep) = immediate_reply {
                        l.push(BotChatMessage {
                            id: format!("b-{}", now_ts + 1),
                            seq: now_ts + 1,
                            from_bot: true,
                            text: rep.clone(),
                            created_at: now.clone(),
                        });
                    }
                    if let Ok(s) = serde_json::to_string(l) {
                        platform::storage_set(&sk, &s);
                    }
                });

                if immediate_reply.is_none() && (webhook_url.starts_with("http://") || webhook_url.starts_with("https://")) {
                    let payload = json!({
                        "plugin_id": id,
                        "username": u_name,
                        "text": text,
                    });
                    spawn(async move {
                        let reply_text = match platform::http_request("POST", &webhook_url, Some(payload.to_string()), None).await {
                            Ok((_, body)) => {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                                    v.get("reply")
                                        .or_else(|| v.get("text"))
                                        .or_else(|| v.get("message"))
                                        .and_then(|s| s.as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| body.trim().to_string())
                                } else {
                                    body.trim().to_string()
                                }
                            }
                            Err(_) => fallback_reply.unwrap_or_else(|| "Không kết nối được Webhook của Plugin".into()),
                        };
                        let r_ts = chrono::Utc::now().timestamp_millis();
                        msgs.with_mut(|l| {
                            if !reply_text.is_empty() {
                                l.push(BotChatMessage {
                                    id: format!("w-{}", r_ts),
                                    seq: r_ts,
                                    from_bot: true,
                                    text: reply_text,
                                    created_at: chrono::Utc::now().to_rfc3339(),
                                });
                                if let Ok(s) = serde_json::to_string(l) {
                                    platform::storage_set(&sk, &s);
                                }
                            }
                        });
                        sending.set(false);
                        platform::scroll_chat_to_bottom(false);
                    });
                } else {
                    sending.set(false);
                    platform::scroll_chat_to_bottom(false);
                }
            } else {
                let local_reply = cmd_reply.or(fallback_reply);
                spawn(async move {
                    match social::bot_send(&id, &text).await {
                        Ok(m) => {
                            msgs.with_mut(|l| {
                                if !l.iter().any(|x| x.id == m.id) {
                                    l.push(m);
                                }
                                if let Some(rep) = local_reply {
                                    let l_ts = chrono::Utc::now().timestamp_millis();
                                    l.push(BotChatMessage {
                                        id: format!("local-{}", l_ts),
                                        seq: l_ts,
                                        from_bot: true,
                                        text: rep,
                                        created_at: chrono::Utc::now().to_rfc3339(),
                                    });
                                }
                            });
                        }
                        Err(e) => error.set(Some(plugin_error(&e.code).to_string())),
                    }
                    sending.set(false);
                    platform::scroll_chat_to_bottom(false);
                });
            }
        }
    });

    let view: Vec<ViewMessage> = msgs()
        .into_iter()
        .map(|m| ViewMessage {
            id: m.id.clone(),
            sender_id: if m.from_bot { bot.id.clone() } else { my_id.clone() },
            sender_name: if m.from_bot { bot.display_name.clone() } else { my_name.clone() },
            sender_avatar: if m.from_bot { bot.avatar_url.clone() } else { None },
            text: m.text,
            created_at: m.created_at,
            is_self: !m.from_bot,
            is_error: false,
            is_pending: false,
            reply_text: None,
            reactions: Vec::new(),
            is_pinned: false,
            is_bot: m.from_bot,
            highlighted: false,
        })
        .collect();
    let admin_groups: Vec<GroupItem> = groups.into_iter().filter(|g| matches!(g.role.as_deref(), Some("owner") | Some("admin"))).collect();
    let can_add = !is_local && (bot.owner_id == my_id || (bot.is_public && bot.verification_status == "verified"));
    let quick_cmds: Vec<PluginCommand> = matching_mod.as_ref().map(|m| m.commands.clone()).unwrap_or_default();
    let header_icon = matching_mod.as_ref().map(|m| m.icon.clone()).unwrap_or_else(|| "🤖".to_string());
    let header_logo = matching_mod.as_ref().map(|m| m.logo_url.clone()).unwrap_or_else(|| "svg:bot".to_string());
    let bot_desc = bot
        .description
        .clone()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| "Chọn lệnh nhanh bên dưới hoặc gõ / để xem gợi ý lệnh.".to_string());
    let send_text_composer = send_text.clone();
    let send_text_welcome = send_text.clone();

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column {wp_cls}", style: "{wp_style}",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại danh sách", onclick: move |_| on_back.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        div { class: "chat-item-avatar",
                            if matching_mod.is_none() && bot.avatar_url.is_some() {
                                Avatar { name: bot.display_name.clone(), avatar_url: bot.avatar_url.clone(), size: 38 }
                            } else {
                                span { class: "bot-header-icon", PluginGlyph { icon: header_icon.clone(), logo_url: header_logo.clone(), size: glyph_slot_size(&header_logo, 20, 38) } }
                            }
                        }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{bot.display_name}" }
                            span { class: "chat-header-meta", "@{bot.username}" }
                        }
                    }
                    div { class: "chat-header-actions",
                        if let Some(ref pm) = matching_mod {
                            if pm.left_app_enabled {
                                {
                                    let pid = pm.id.clone();
                                    rsx! {
                                        button { r#type: "button", class: "btn-secondary",
                                            title: "Mở ở cột bên trái",
                                            onclick: move |_| *ACTIVE_LEFT_PLUGIN.write() = Some(pid.clone()),
                                            PanelRightIcon { size: 15 } "Bên trái"
                                        }
                                    }
                                }
                            }
                            if pm.custom_page_enabled {
                                {
                                    let pid = pm.id.clone();
                                    rsx! {
                                        button { r#type: "button", class: "btn-secondary",
                                            onclick: move |_| *ACTIVE_PLUGIN_PAGE.write() = Some(pid.clone()),
                                            PluginGlyph { icon: pm.page_icon.clone(), logo_url: pm.logo_url.clone(), size: 15 } "Mở"
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(ref pm) = matching_mod {
                            {
                                let edit_id = pm.id.clone();
                                rsx! {
                                    button { r#type: "button", class: "btn-ghost", title: "Sửa Plugin",
                                        onclick: move |_| {
                                            *EDITING_PLUGIN_ID.write() = Some(edit_id.clone());
                                            *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                                        },
                                        SlidersIcon {} "Sửa"
                                    }
                                }
                            }
                        } else {
                            button { r#type: "button", class: "btn-ghost", title: "Sửa Plugin",
                                onclick: move |_| {
                                    *EDITING_PLUGIN_ID.write() = None;
                                    *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into());
                                },
                                SlidersIcon {} "Sửa"
                            }
                        }
                        if can_add && !admin_groups.is_empty() {
                            button { r#type: "button", class: if add_open() { "icon-btn active" } else { "icon-btn" }, title: "Thêm plugin vào nhóm", aria_label: "Thêm plugin vào nhóm",
                                onclick: move |e| { e.stop_propagation(); add_open.set(!add_open()); },
                                GroupIcon {}
                            }
                        }
                    }
                    if add_open() {
                        button { r#type: "button", class: "menu-dismiss", tabindex: "-1", aria_label: "Đóng", onclick: move |_| add_open.set(false) }
                        div { class: "account-menu chat-more-menu", role: "menu",
                            div { class: "menu-note", "Thêm plugin vào nhóm bạn quản lý" }
                            for g in admin_groups.iter().cloned() {
                                {
                                    let bid = bot.id.clone();
                                    let gid = g.id.clone();
                                    let gname = g.name.clone();
                                    rsx! {
                                        button { key: "{g.id}", r#type: "button", role: "menuitem", class: "account-menu-item",
                                            onclick: move |_| {
                                                add_open.set(false);
                                                let (b, gi, gn) = (bid.clone(), gid.clone(), gname.clone());
                                                spawn(async move {
                                                    match social::bot_add_to_group(&b, &gi).await {
                                                        Ok(_) => error.set(Some(format!("Đã thêm plugin vào nhóm {gn}."))),
                                                        Err(e) => error.set(Some(plugin_error(&e.code).to_string())),
                                                    }
                                                });
                                            },
                                            Avatar { name: g.name.clone(), avatar_url: g.avatar_url.clone(), size: 24 }
                                            "{g.name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(e) = error() {
                    div { class: "chat-banner", role: "status", span { "{e}" } button { r#type: "button", class: "btn-ghost", onclick: move |_| error.set(None), "Đóng" } }
                }
                if view.is_empty() {
                    div { class: "chat-messages", id: "chat-messages-scroll",
                        div { class: "bot-welcome-card",
                            span { class: "bot-welcome-icon", PluginGlyph { icon: header_icon.clone(), logo_url: header_logo.clone(), size: glyph_slot_size(&header_logo, 28, 46) } }
                            h3 { class: "bot-welcome-title", "{bot.display_name}" }
                            p { class: "bot-welcome-desc", "{bot_desc}" }
                            if !quick_cmds.is_empty() {
                                div { class: "bot-welcome-cmds",
                                    for c in quick_cmds.iter() {
                                        {
                                            let trig = c.trigger.clone();
                                            let desc = c.description.clone();
                                            let send_chip = send_text_welcome.clone();
                                            rsx! {
                                                button {
                                                    key: "{trig}",
                                                    r#type: "button",
                                                    class: "plugin-cmd-chip",
                                                    title: "{desc}",
                                                    onclick: move |_| send_chip(trig.clone()),
                                                    "{trig}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    MessageList {
                        messages: view,
                        show_sender_names: false,
                        intro_title: bot.display_name.clone(),
                        intro_sub: bot_desc,
                        on_open_profile: move |_| {},
                        on_profile_click: move |_| {},
                        on_react: move |_| {},
                    }
                }
                Composer {
                    input_id: "bot-chat-input".to_string(),
                    value: input(),
                    placeholder: format!("Nhắn tin hoặc gõ /lệnh tới @{}", bot.username),
                    sending: sending(),
                    on_input: move |v: String| input.set(v),
                    on_send: move |_| send_text_composer(input()),
                }
            }
        }
    }
}

pub use PluginChatView as BotChatView;
pub use crate::components::plugin_settings::PluginsSettings;
pub use PluginsSettings as BotsSettings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_luma_detects_dark_light_and_bright_accents() {
        assert!(hex_luma("#181E18").unwrap() < 135.0);
        assert!(hex_luma("#0A0C1A").unwrap() < 135.0);
        assert!(hex_luma("#FFF5F9").unwrap() > 135.0);
        assert!(hex_luma("#00F0FF").unwrap() > 155.0);
        assert!(hex_luma("#10B981").unwrap() < 155.0);
        assert!(hex_luma("#fff").unwrap() > 250.0);
    }

    #[test]
    fn store_plugins_split_installed_and_available() {
        let builtins = default_builtin_plugins();
        assert!(builtins.iter().any(|p| p.installed && p.enabled));
        assert!(builtins.iter().any(|p| !p.installed && !p.enabled));
    }

    #[test]
    fn legacy_plugin_json_defaults_installed_true() {
        let mut p = default_builtin_plugins().into_iter().next().unwrap();
        p.installed = false;
        let mut val = serde_json::to_value(&p).unwrap();
        val.as_object_mut().unwrap().remove("installed");
        let parsed: PluginMod = serde_json::from_value(val).unwrap();
        assert!(parsed.installed);
    }

    #[test]
    fn command_icons_resolve_distinct_vectors_instead_of_bot_logo() {
        assert_eq!(resolve_command_icon("/roll", "", "🤖", "logo:clock"), (String::new(), "svg:dice".into()));
        assert_eq!(resolve_command_icon("/flip", "", "🤖", "logo:clock"), (String::new(), "svg:coin".into()));
        assert_eq!(resolve_command_icon("/time", "", "🤖", "logo:clock"), (String::new(), "svg:clock".into()));
        assert_eq!(resolve_command_icon("/calc", "", "🤖", "logo:clock"), (String::new(), "svg:calc".into()));
        assert_eq!(resolve_command_icon("/upper", "", "🤖", "logo:clock"), (String::new(), "svg:upper".into()));
        assert_eq!(resolve_command_icon("/sound", "", "🎹", "logo:sound"), (String::new(), "logo:sound".into()));
        assert_eq!(resolve_command_icon("/workspace", "", "", ""), (String::new(), "svg:workspace".into()));
        assert_eq!(resolve_command_icon("/task", "", "", ""), (String::new(), "svg:task".into()));
    }
}

