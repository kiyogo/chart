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
        icons::{BackIcon, BotIcon, CheckIcon, CopyIcon, GroupIcon, PaletteIcon, PlusIcon, RefreshIcon, SlidersIcon, SparkleIcon, TrashIcon},
        messages::{Composer, MessageList, ViewMessage},
    },
    models::{BotChatMessage, BotItem, GroupItem},
    platform,
    services::social,
};

const STORAGE_KEY: &str = "chatw_plugins_v2";
const NOTES_KEY: &str = "chatw_plugin_notes_v1";
const TODOS_KEY: &str = "chatw_plugin_todos_v1";
const VOXEL_KEY: &str = "chatw_plugin_voxel_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginCommand {
    pub trigger: String,
    pub description: String,
    pub response: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginComposerBtn {
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginCustomBlock {
    pub title: String,
    pub body: String,
    pub action_label: String,
    pub action_command: String,
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
    pub description: String,
    pub enabled: bool,
    pub builtin: bool,
    pub server_bot_id: Option<String>,
    // Module 1: Bot & Automation
    pub bot_enabled: bool,
    pub commands: Vec<PluginCommand>,
    // Module 2: Full-App UI Overhaul
    pub theme_enabled: bool,
    pub theme: PluginTheme,
    // Module 3: New UI Surfaces & FX
    pub custom_page_enabled: bool,
    pub page_title: String,
    pub page_icon: String,
    pub page_kind: String, // "voxel" | "notes" | "soundboard" | "cyber" | "custom"
    pub custom_blocks: Vec<PluginCustomBlock>,
    pub hud_enabled: bool,
    pub hud_position: String, // "top-right" | "bottom-right" | "top-left"
    pub hud_coords: bool,
    pub hud_clock: bool,
    pub hud_note: bool,
    pub screen_fx: String, // "none" | "sakura" | "snow" | "matrix" | "voxel" | "crt"
    pub sound_fx: String,  // "none" | "minecraft" | "mech" | "scifi"
    pub composer_buttons: Vec<PluginComposerBtn>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TodoItem {
    id: String,
    text: String,
    done: bool,
}

pub static PLUGINS: GlobalSignal<Vec<PluginMod>> = Signal::global(load_plugins_from_storage);
pub static ACTIVE_PLUGIN_PAGE: GlobalSignal<Option<String>> = Signal::global(|| None);

fn default_builtin_plugins() -> Vec<PluginMod> {
    vec![
        PluginMod {
            id: "mod:minecraft_voxel".into(),
            username: "minecraft_mod".into(),
            display_name: "Minecraft Voxel UI & Sandbox".into(),
            author: "ChatW Forge".into(),
            version: "1.20.4".into(),
            icon: "⛏".into(),
            description: "Biến toàn bộ giao diện thành phong cách khối Voxel Minecraft, thêm trang Thế giới Khối 2D trên thanh điều hướng, HUD tọa độ XYZ, hạt XP lấp lánh, âm thanh 8-Bit và bot lệnh /craft, /coords, /roll.".into(),
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/craft".into(), description: "Chế tạo vật phẩm ngẫu nhiên".into(), response: "⛏ [{user}] đã chế tạo thành công: Kiếm Kim Cương Bền Bỉ III + Cúp Ngọc Lục Bảo!".into() },
                PluginCommand { trigger: "/coords".into(), description: "Xem tọa độ thế giới hiện tại".into(), response: "🧭 Tọa độ của {user}: X: 128, Y: 64, Z: -256 · Quần xã: Đồng bằng Ngọc Lục Bảo".into() },
                PluginCommand { trigger: "/seed".into(), description: "Tạo hạt giống thế giới".into(), response: "🌱 World Seed: 849201749201 · Số may mắn: {rand100}".into() },
                PluginCommand { trigger: "/roll".into(), description: "Tung xúc xắc may mắn".into(), response: "🎲 {user} tung xúc xắc được: {dice} điểm!".into() },
            ],
            theme_enabled: true,
            theme: PluginTheme {
                bg_app: "#141814".into(),
                bg_sidebar: "#1D241D".into(),
                bg_chat: "#181E18".into(),
                bg_surface: "#242D24".into(),
                accent: "#10B981".into(),
                msg_sent: "#059669".into(),
                msg_recv: "#263126".into(),
                text_primary: "#ECFDF5".into(),
                panel_radius: "0px".into(),
                border_style: "pixel".into(),
                glass_blur: 0,
                rail_position: "left".into(),
                font_style: "pixel".into(),
                custom_css: ".chat-item, .btn-primary, .btn-secondary, .result-card, .settings-row { box-shadow: inset -2px -2px 0 rgba(0,0,0,0.45), inset 2px 2px 0 rgba(255,255,255,0.12); }".into(),
            },
            custom_page_enabled: true,
            page_title: "Thế giới Voxel".into(),
            page_icon: "⛏".into(),
            page_kind: "voxel".into(),
            custom_blocks: Vec::new(),
            hud_enabled: true,
            hud_position: "top-right".into(),
            hud_coords: true,
            hud_clock: true,
            hud_note: false,
            screen_fx: "voxel".into(),
            sound_fx: "minecraft".into(),
            composer_buttons: vec![
                PluginComposerBtn { label: "⛏ /craft".into(), content: "/craft".into() },
                PluginComposerBtn { label: "🧭 /coords".into(), content: "/coords".into() },
                PluginComposerBtn { label: "💎 Kim cương".into(), content: "💎 Đã tìm thấy mạch Kim Cương tại Y=-58!".into() },
            ],
        },
        PluginMod {
            id: "mod:cyberpunk_hud".into(),
            username: "cyberpunk_mod".into(),
            display_name: "Cyberpunk Neon Overhaul".into(),
            author: "NightCity Labs".into(),
            version: "2.0.77".into(),
            icon: "⚡".into(),
            description: "Giao diện Neon Cyberpunk phát sáng với viền laser, mưa ma trận Matrix toàn màn hình, trang Trạm Điều Khiển Cyber trên thanh điều hướng, HUD đồng hồ số và bot lệnh /neon, /hack.".into(),
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/neon".into(), description: "Kích hoạt xung nhịp Neon".into(), response: "⚡ [NEON PULSE] Hệ thống quang học của {user} đã đồng bộ ở mức {rand100}%!".into() },
                PluginCommand { trigger: "/hack".into(), description: "Mô phỏng giải mã".into(), response: "💻 Giải mã nút mạng hoàn tất lúc {time} · Bảo mật E2EE: Bất khả xâm phạm.".into() },
                PluginCommand { trigger: "/status".into(), description: "Kiểm tra trạng thái trạm".into(), response: "🛰 Trạng thái Cyber: Trực tuyến · Độ trễ: 2ms · Mã hóa: P-256 + AES-256-GCM".into() },
            ],
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
                panel_radius: "6px".into(),
                border_style: "neon".into(),
                glass_blur: 8,
                rail_position: "left".into(),
                font_style: "pixel".into(),
                custom_css: String::new(),
            },
            custom_page_enabled: true,
            page_title: "Trạm Cyber".into(),
            page_icon: "⚡".into(),
            page_kind: "cyber".into(),
            custom_blocks: Vec::new(),
            hud_enabled: true,
            hud_position: "top-right".into(),
            hud_coords: false,
            hud_clock: true,
            hud_note: false,
            screen_fx: "matrix".into(),
            sound_fx: "scifi".into(),
            composer_buttons: vec![
                PluginComposerBtn { label: "⚡ /neon".into(), content: "/neon".into() },
                PluginComposerBtn { label: "🛰 /status".into(), content: "/status".into() },
            ],
        },
        PluginMod {
            id: "mod:sakura_glass".into(),
            username: "sakura_mod".into(),
            display_name: "Sakura Acrylic & Hoa Anh Đào".into(),
            author: "Haru Studio".into(),
            version: "1.4.0".into(),
            icon: "🌸".into(),
            description: "Giao diện kính mờ Acrylic trong suốt mềm mại, hiệu ứng cánh hoa anh đào rơi nhẹ trên toàn ứng dụng, thanh biểu cảm Kaomoji nhanh trong khung chat và bot /sakura, /fortune.".into(),
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/sakura".into(), description: "Gửi lời chúc hoa anh đào".into(), response: "🌸 Cánh hoa anh đào mang lời chúc tốt lành tới {user} lúc {time}!".into() },
                PluginCommand { trigger: "/fortune".into(), description: "Rút quẻ may mắn đầu ngày".into(), response: "🎴 Quẻ may mắn của {user}: Đại Cát · Chỉ số năng lượng: {rand100}/100 ✨".into() },
            ],
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
            custom_page_enabled: false,
            page_title: "Vườn Sakura".into(),
            page_icon: "🌸".into(),
            page_kind: "custom".into(),
            custom_blocks: Vec::new(),
            hud_enabled: false,
            hud_position: "top-right".into(),
            hud_coords: false,
            hud_clock: true,
            hud_note: false,
            screen_fx: "sakura".into(),
            sound_fx: "none".into(),
            composer_buttons: vec![
                PluginComposerBtn { label: "🌸 Hoa anh đào".into(), content: "🌸 Chúc một ngày tốt lành ✨".into() },
                PluginComposerBtn { label: "💖 Lời chúc".into(), content: "💖 Luôn vui vẻ và may mắn nhé ✨".into() },
                PluginComposerBtn { label: "🎴 /fortune".into(), content: "/fortune".into() },
            ],
        },
        PluginMod {
            id: "mod:workspace_notes".into(),
            username: "workspace_notes".into(),
            display_name: "Sổ Tay, Việc Cần Làm & Pomodoro".into(),
            author: "ChatW Productivity".into(),
            version: "1.2.0".into(),
            icon: "📋".into(),
            description: "Thêm trang Sổ tay ghi chú nhanh, bảng quản lý việc cần làm và đồng hồ tập trung Pomodoro ngay trên thanh điều hướng bên trái, kèm HUD ghi chú nổi và bot /todo, /note.".into(),
            enabled: true,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/todo".into(), description: "Thêm nhanh việc cần làm".into(), response: "✅ Đã ghi nhận công việc mới cho {user}: {args}".into() },
                PluginCommand { trigger: "/note".into(), description: "Lưu ghi chú nhanh".into(), response: "📝 Đã lưu vào sổ tay lúc {time}: {args}".into() },
            ],
            theme_enabled: false,
            theme: PluginTheme::default(),
            custom_page_enabled: true,
            page_title: "Sổ tay & Việc làm".into(),
            page_icon: "📋".into(),
            page_kind: "notes".into(),
            custom_blocks: Vec::new(),
            hud_enabled: false,
            hud_position: "bottom-right".into(),
            hud_coords: false,
            hud_clock: true,
            hud_note: true,
            screen_fx: "none".into(),
            sound_fx: "none".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:soundboard_mech".into(),
            username: "soundboard_mod".into(),
            display_name: "Bàn Phím Cơ & Soundboard 8-Bit".into(),
            author: "AudioCraft".into(),
            version: "1.1.0".into(),
            icon: "🎹".into(),
            description: "Thêm trang Bàn phím cơ & Soundboard tổng hợp âm thanh WebAudio trực tiếp trên thanh điều hướng, phát tiếng switch bàn phím cơ khi gõ phím và âm báo 8-Bit.".into(),
            enabled: false,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/sound".into(), description: "Phát hiệu ứng âm thanh".into(), response: "🔊 Đã phát hiệu ứng âm thanh tổng hợp cho {user}!".into() },
            ],
            theme_enabled: false,
            theme: PluginTheme::default(),
            custom_page_enabled: true,
            page_title: "Soundboard".into(),
            page_icon: "🎹".into(),
            page_kind: "soundboard".into(),
            custom_blocks: Vec::new(),
            hud_enabled: false,
            hud_position: "bottom-right".into(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            screen_fx: "none".into(),
            sound_fx: "mech".into(),
            composer_buttons: Vec::new(),
        },
        PluginMod {
            id: "mod:smart_assistant".into(),
            username: "smart_plugin".into(),
            display_name: "Trợ Lý Lệnh Nhanh & Máy Tính".into(),
            author: "ChatW Core".into(),
            version: "1.5.0".into(),
            icon: "🤖".into(),
            description: "Plugin Bot đa năng chạy ngay trên thiết bị: tính toán biểu thức với /calc, tung xúc xắc /roll, tung đồng xu /flip, xem giờ /time, đổi chữ hoa /upper và tự trả lời thông minh.".into(),
            enabled: true,
            builtin: true,
            server_bot_id: None,
            bot_enabled: true,
            commands: vec![
                PluginCommand { trigger: "/roll".into(), description: "Tung xúc xắc 1-6".into(), response: "🎲 Kết quả xúc xắc của {user}: {dice}".into() },
                PluginCommand { trigger: "/flip".into(), description: "Tung đồng xu Sấp/Ngửa".into(), response: "🪙 Đồng xu rơi vào mặt: {coin}".into() },
                PluginCommand { trigger: "/time".into(), description: "Xem giờ hiện tại".into(), response: "🕒 Giờ hệ thống hiện tại: {time} · Ngày {date}".into() },
                PluginCommand { trigger: "/calc".into(), description: "Tính nhanh biểu thức số học".into(), response: "🧮 Kết quả tính toán: {calc}".into() },
                PluginCommand { trigger: "/upper".into(), description: "Chuyển văn bản thành chữ in hoa".into(), response: "🔠 {upper}".into() },
            ],
            theme_enabled: false,
            theme: PluginTheme::default(),
            custom_page_enabled: false,
            page_title: "Trợ lý lệnh".into(),
            page_icon: "🤖".into(),
            page_kind: "custom".into(),
            custom_blocks: Vec::new(),
            hud_enabled: false,
            hud_position: "top-right".into(),
            hud_coords: false,
            hud_clock: false,
            hud_note: false,
            screen_fx: "none".into(),
            sound_fx: "none".into(),
            composer_buttons: vec![
                PluginComposerBtn { label: "🎲 /roll".into(), content: "/roll".into() },
                PluginComposerBtn { label: "🪙 /flip".into(), content: "/flip".into() },
                PluginComposerBtn { label: "🕒 /time".into(), content: "/time".into() },
            ],
        },
    ]
}

fn load_plugins_from_storage() -> Vec<PluginMod> {
    let mut defaults = default_builtin_plugins();
    if let Some(raw) = platform::storage_get(STORAGE_KEY) {
        if let Ok(saved) = serde_json::from_str::<Vec<PluginMod>>(&raw) {
            for s in saved {
                if let Some(existing) = defaults.iter_mut().find(|d| d.id == s.id) {
                    *existing = s;
                } else {
                    defaults.push(s);
                }
            }
        }
    }
    defaults
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

pub fn enabled_rail_pages() -> Vec<(String, String, String)> {
    PLUGINS
        .read()
        .iter()
        .filter(|p| p.enabled && p.custom_page_enabled)
        .map(|p| {
            let icon = if p.page_icon.trim().is_empty() { p.icon.clone() } else { p.page_icon.clone() };
            let title = if p.page_title.trim().is_empty() { p.display_name.clone() } else { p.page_title.clone() };
            (p.id.clone(), icon, title)
        })
        .collect()
}

pub fn enabled_composer_buttons() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for p in PLUGINS.read().iter().filter(|p| p.enabled) {
        for b in &p.composer_buttons {
            if !b.label.trim().is_empty() && !b.content.trim().is_empty() {
                out.push((b.label.clone(), b.content.clone(), p.display_name.clone()));
            }
        }
    }
    out
}

pub fn enabled_bot_plugins_as_items() -> Vec<BotItem> {
    PLUGINS
        .read()
        .iter()
        .filter(|p| p.enabled && p.bot_enabled && p.server_bot_id.is_none())
        .map(|p| BotItem {
            id: p.id.clone(),
            owner_id: "local".into(),
            username: p.username.clone(),
            display_name: format!("{} {}", p.icon, p.display_name),
            description: Some(p.description.clone()),
            avatar_url: None,
            is_public: true,
            verification_status: "verified".into(),
            token_hint: Some(format!("Mod v{}", p.version)),
            token: None,
            last_text: Some(p.description.clone()),
        })
        .collect()
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
        let mut lines = vec!["🧩 Danh sách lệnh từ các Plugin & Mod đang bật:".to_string()];
        for p in PLUGINS.read().iter().filter(|p| p.enabled && p.bot_enabled) {
            for c in &p.commands {
                lines.push(format!("• {} — {} [{}]", c.trigger, c.description, p.display_name));
            }
        }
        return Some(lines.join("\n"));
    }
    for p in PLUGINS.read().iter().filter(|p| p.enabled && p.bot_enabled) {
        for c in &p.commands {
            if c.trigger.eq_ignore_ascii_case(cmd) {
                if p.sound_fx != "none" {
                    play_mod_sound(&p.sound_fx);
                }
                return Some(expand_template(&c.response, user_name, args));
            }
        }
    }
    None
}

pub fn play_mod_sound(kind: &str) {
    let k = serde_json::to_string(kind).unwrap_or_else(|_| "\"minecraft\"".into());
    let _ = dioxus::document::eval(&format!(
        r#"
        try {{
            const AC = window.AudioContext || window.webkitAudioContext;
            if (AC) {{
                const ctx = window.__chatwAudioCtx || new AC();
                if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                const tone = (f0, f1, start, dur, type, vol) => {{
                    const osc = ctx.createOscillator();
                    const g = ctx.createGain();
                    osc.type = type;
                    const t = ctx.currentTime + start;
                    osc.frequency.setValueAtTime(f0, t);
                    osc.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + dur);
                    g.gain.setValueAtTime(0.001, t);
                    g.gain.exponentialRampToValueAtTime(vol, t + 0.01);
                    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
                    osc.connect(g); g.connect(ctx.destination);
                    osc.start(t); osc.stop(t + dur + 0.02);
                }};
                const run = () => {{
                    const kind = {k};
                    if (kind === 'minecraft' || kind === 'xp') {{
                        tone(587, 880, 0, 0.09, 'square', 0.12);
                        tone(880, 1318, 0.08, 0.12, 'square', 0.12);
                    }} else if (kind === 'levelup') {{
                        tone(523, 659, 0, 0.1, 'square', 0.14);
                        tone(659, 784, 0.1, 0.1, 'square', 0.14);
                        tone(784, 1046, 0.2, 0.22, 'square', 0.16);
                    }} else if (kind === 'anvil') {{
                        tone(220, 110, 0, 0.18, 'sawtooth', 0.2);
                        tone(880, 440, 0.01, 0.08, 'triangle', 0.15);
                    }} else if (kind === 'mech') {{
                        tone(1800, 420, 0, 0.028, 'triangle', 0.16);
                    }} else if (kind === 'scifi' || kind === 'laser') {{
                        tone(1400, 240, 0, 0.16, 'sawtooth', 0.14);
                    }} else if (kind === 'coin') {{
                        tone(987, 987, 0, 0.07, 'square', 0.14);
                        tone(1318, 1318, 0.07, 0.22, 'square', 0.14);
                    }} else {{
                        tone(660, 990, 0, 0.12, 'sine', 0.14);
                    }}
                }};
                if (ctx.state === 'suspended') ctx.resume().then(run); else run();
            }}
        }} catch (_) {{}}
        "#
    ));
}

pub fn sync_plugins_to_dom(list: &[PluginMod]) {
    let mut css = String::new();
    let mut mech_key = false;
    for p in list.iter().filter(|p| p.enabled) {
        if p.sound_fx == "mech" {
            mech_key = true;
        }
        if !p.theme_enabled {
            continue;
        }
        let t = &p.theme;
        let mut vars = String::new();
        if !t.bg_app.is_empty() { vars.push_str(&format!("--bg-app: {} !important;", t.bg_app)); }
        if !t.bg_sidebar.is_empty() { vars.push_str(&format!("--bg-sidebar: {} !important; --bg-primary: {} !important;", t.bg_sidebar, t.bg_sidebar)); }
        if !t.bg_chat.is_empty() { vars.push_str(&format!("--bg-chat: {} !important;", t.bg_chat)); }
        if !t.bg_surface.is_empty() { vars.push_str(&format!("--bg-surface: {} !important; --bg-elevated: {} !important; --bg-secondary: {} !important;", t.bg_surface, t.bg_surface, t.bg_surface)); }
        if !t.accent.is_empty() {
            vars.push_str(&format!(
                "--accent: {a} !important; --accent-hover: color-mix(in srgb, {a} 85%, #000) !important; --accent-soft: color-mix(in srgb, {a} 18%, transparent) !important; --accent-text: {a} !important;",
                a = t.accent
            ));
        }
        if !t.msg_sent.is_empty() { vars.push_str(&format!("--msg-sent: {} !important;", t.msg_sent)); }
        if !t.msg_recv.is_empty() { vars.push_str(&format!("--msg-recv: {} !important;", t.msg_recv)); }
        if !t.text_primary.is_empty() {
            vars.push_str(&format!(
                "--text-primary: {tp} !important; --text: {tp} !important; --msg-recv-text: {tp} !important;",
                tp = t.text_primary
            ));
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
                css.push_str(".nav-sidebar, .main-stage, .chat-column, .result-card, .bot-card, .mod-card, .settings-row, .inv-card, .message-bubble { border: 2px solid color-mix(in srgb, var(--accent) 45%, #000) !important; }\n");
            }
            "neon" => {
                css.push_str(".nav-sidebar, .main-stage, .chat-column, .result-card, .mod-card { border: 1px solid color-mix(in srgb, var(--accent) 65%, transparent) !important; box-shadow: 0 0 14px color-mix(in srgb, var(--accent) 22%, transparent) !important; }\n");
                css.push_str(".message-row.sent .message-bubble { box-shadow: 0 0 12px color-mix(in srgb, var(--msg-sent) 45%, transparent) !important; }\n");
            }
            "glass" => {
                let b = t.glass_blur.max(8);
                css.push_str(&format!(
                    ".nav-sidebar, .main-stage, .chat-column, .result-card, .mod-card, .settings-row {{ background: color-mix(in srgb, var(--bg-surface) 74%, transparent) !important; backdrop-filter: blur({b}px) !important; border: 1px solid color-mix(in srgb, var(--accent) 22%, rgba(255,255,255,0.35)) !important; }}\n"
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
            window.__chatwMechKey = {mech_key};
            if (!window.__chatwMechListener) {{
                window.__chatwMechListener = true;
                window.addEventListener('keydown', (e) => {{
                    if (!window.__chatwMechKey) return;
                    if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return;
                    try {{
                        const AC = window.AudioContext || window.webkitAudioContext;
                        if (!AC) return;
                        const ctx = window.__chatwAudioCtx || new AC();
                        if (!window.__chatwAudioCtx) window.__chatwAudioCtx = ctx;
                        const osc = ctx.createOscillator();
                        const g = ctx.createGain();
                        osc.type = 'triangle';
                        const t = ctx.currentTime;
                        osc.frequency.setValueAtTime(e.key === 'Enter' ? 1100 : 1750, t);
                        osc.frequency.exponentialRampToValueAtTime(360, t + 0.025);
                        g.gain.setValueAtTime(0.08, t);
                        g.gain.exponentialRampToValueAtTime(0.0001, t + 0.025);
                        osc.connect(g); g.connect(ctx.destination);
                        osc.start(t); osc.stop(t + 0.03);
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

/// Global overlay rendered inside `Chat` workspace shell: renders full-app Screen FX
/// particles and the Floating HUD from active Plugins/Mods.
#[component]
pub fn PluginGlobalOverlay() -> Element {
    use_effect(move || {
        init_plugins();
    });
    let plugins = PLUGINS.read().clone();
    let active_fx = plugins
        .iter()
        .rev()
        .find(|p| p.enabled && p.screen_fx != "none" && !p.screen_fx.is_empty())
        .map(|p| p.screen_fx.clone())
        .unwrap_or_else(|| "none".into());
    let hud_mod = plugins.iter().rev().find(|p| p.enabled && p.hud_enabled).cloned();
    let enabled_count = plugins.iter().filter(|p| p.enabled).count();
    let mut hud_collapsed = use_signal(|| false);
    let mut quick_note = use_signal(|| platform::storage_get(NOTES_KEY).unwrap_or_default());
    let clock_now = chrono::Local::now().format("%H:%M · %d/%m/%Y").to_string();

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
        if let Some(hm) = hud_mod {
            div { class: "plugin-floating-hud pos-{hm.hud_position}",
                div { class: "plugin-hud-bar",
                    span { class: "plugin-hud-title", "{hm.icon} {hm.display_name}" }
                    span { class: "plugin-hud-badge", "{enabled_count} Mod" }
                    button { r#type: "button", class: "plugin-hud-btn", title: "Mở Studio Plugin & Mod",
                        onclick: move |_| *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into()),
                        "Mod"
                    }
                    button { r#type: "button", class: "plugin-hud-btn",
                        onclick: move |_| hud_collapsed.set(!hud_collapsed()),
                        if hud_collapsed() { "+" } else { "−" }
                    }
                }
                if !hud_collapsed() {
                    div { class: "plugin-hud-body",
                        if hm.hud_coords {
                            div { class: "plugin-hud-row mono", "XYZ: 128, 64, -256 · FPS: 60" }
                        }
                        if hm.hud_clock {
                            div { class: "plugin-hud-row", "🕒 {clock_now}" }
                        }
                        if hm.hud_note {
                            textarea {
                                class: "plugin-hud-note",
                                rows: "2",
                                placeholder: "Ghi chú nhanh trên HUD",
                                value: "{quick_note}",
                                oninput: move |e| {
                                    let v = e.value();
                                    quick_note.set(v.clone());
                                    platform::storage_set(NOTES_KEY, &v);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Renders a custom page added to the Navigation Rail by a Plugin/Mod, or the full Mod Studio.
#[component]
pub fn PluginCustomPageView(plugin_id: String, my_username: String, on_close: EventHandler<()>, on_open_bot_chat: EventHandler<BotItem>) -> Element {
    if plugin_id == "studio" {
        return rsx! {
            div { class: "main-stage",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại", onclick: move |_| on_close.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        span { class: "self-chat-avatar", BotIcon { size: 20 } }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "Kho Plugin & Mod Giao Diện" span { class: "bot-tag", "MOD" } }
                            span { class: "chat-header-meta", "Tùy biến toàn bộ ứng dụng, thêm trang mới, hiệu ứng, HUD và lập trình Bot" }
                        }
                    }
                }
                div { class: "settings-body", style: "flex: 1; overflow-y: auto;",
                    div { class: "settings-content", style: "max-width: 66rem; width: 100%;",
                        BotsSettings {
                            my_username: my_username,
                            on_open_bot_chat: on_open_bot_chat,
                        }
                    }
                }
            }
        };
    }

    let plugin = PLUGINS.read().iter().find(|p| p.id == plugin_id).cloned();
    let Some(p) = plugin else {
        return rsx! {
            div { class: "main-stage",
                div { class: "empty-chat-view",
                    div { class: "empty-chat-inner",
                        div { class: "empty-chat-title", "Không tìm thấy trang Plugin" }
                        button { r#type: "button", class: "btn-primary", onclick: move |_| on_close.call(()), "Quay lại" }
                    }
                }
            }
        };
    };

    rsx! {
        div { class: "main-stage",
            header { class: "chat-header",
                button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại", onclick: move |_| on_close.call(()), BackIcon {} }
                div { class: "chat-header-identity",
                    span { class: "self-chat-avatar", style: "font-size: 1.25rem;", "{p.page_icon}" }
                    div { class: "chat-header-info",
                        h2 { class: "chat-header-title", "{p.page_title}" span { class: "bot-tag", "PLUGIN" } }
                        span { class: "chat-header-meta", "@{p.username} · v{p.version} · bởi {p.author}" }
                    }
                }
                div { class: "chat-header-actions",
                    if p.bot_enabled {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: {
                                let p_bot = p.clone();
                                move |_| {
                                    on_open_bot_chat.call(BotItem {
                                        id: p_bot.server_bot_id.clone().unwrap_or_else(|| p_bot.id.clone()),
                                        owner_id: "local".into(),
                                        username: p_bot.username.clone(),
                                        display_name: format!("{} {}", p_bot.icon, p_bot.display_name),
                                        description: Some(p_bot.description.clone()),
                                        avatar_url: None,
                                        is_public: true,
                                        verification_status: "verified".into(),
                                        token_hint: None,
                                        token: None,
                                        last_text: None,
                                    });
                                }
                            },
                            "Mở trò chuyện Plugin"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-ghost",
                        onclick: move |_| *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into()),
                        SlidersIcon {} "Cấu hình Mod"
                    }
                }
            }
            div { class: "plugin-custom-stage",
                match p.page_kind.as_str() {
                    "voxel" => rsx! { VoxelSandboxPage { plugin: p.clone() } },
                    "notes" => rsx! { NotesAndTodoPage { plugin: p.clone() } },
                    "soundboard" => rsx! { SoundboardPage { plugin: p.clone() } },
                    "cyber" => rsx! { CyberDashboardPage { plugin: p.clone(), my_username: my_username.clone() } },
                    _ => rsx! { CustomBlocksPage { plugin: p.clone(), my_username: my_username.clone() } },
                }
            }
        }
    }
}

const VOXEL_PALETTE: [(&str, &str); 12] = [
    ("Cỏ", "#22C55E"),
    ("Đất", "#92400E"),
    ("Đá", "#64748B"),
    ("Gỗ sồi", "#B45309"),
    ("Lá cây", "#15803D"),
    ("Kim cương", "#38BDF8"),
    ("Ngọc lục bảo", "#10B981"),
    ("Vàng", "#FACC15"),
    ("Đá đỏ", "#EF4444"),
    ("TNT", "#DC2626"),
    ("Nước", "#2563EB"),
    ("Dung nham", "#F97316"),
];

#[component]
fn VoxelSandboxPage(plugin: PluginMod) -> Element {
    let mut selected_color = use_signal(|| "#10B981".to_string());
    let mut grid = use_signal(|| -> Vec<String> {
        if let Some(raw) = platform::storage_get(VOXEL_KEY) {
            if let Ok(v) = serde_json::from_str::<Vec<String>>(&raw) {
                if v.len() == 192 {
                    return v;
                }
            }
        }
        let mut g = vec![String::new(); 192]; // 16 x 12
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

    rsx! {
        div { class: "mod-page-container",
            div { class: "mod-hero-card",
                div {
                    h3 { class: "mod-hero-title", "⛏ Thế Giới Khối Voxel 2D — {plugin.display_name}" }
                    p { class: "mod-hero-sub", "Chọn loại khối bên dưới và bấm vào lưới thế giới để đặt hoặc phá khối. Tự động lưu thế giới của bạn trên thiết bị." }
                }
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-secondary",
                        onclick: move |_| {
                            play_mod_sound("xp");
                            let mut g = vec![String::new(); 192];
                            for x in 0..16 {
                                g[9 * 16 + x] = "#22C55E".into();
                                g[10 * 16 + x] = "#92400E".into();
                                g[11 * 16 + x] = "#64748B".into();
                            }
                            save_grid(&g);
                            grid.set(g);
                        },
                        "Tạo lại địa hình"
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
            div { class: "voxel-toolbar",
                for (name, hex) in VOXEL_PALETTE {
                    button {
                        key: "{name}",
                        r#type: "button",
                        class: if selected_color() == hex { "voxel-block-btn active" } else { "voxel-block-btn" },
                        onclick: move |_| {
                            selected_color.set(hex.to_string());
                            play_mod_sound("mech");
                        },
                        span { class: "voxel-swatch", style: "background: {hex};" }
                        span { "{name}" }
                    }
                }
                button {
                    r#type: "button",
                    class: if selected_color().is_empty() { "voxel-block-btn active" } else { "voxel-block-btn" },
                    onclick: move |_| selected_color.set(String::new()),
                    "🔨 Phá khối"
                }
            }
            div { class: "voxel-world-grid",
                for (idx, cell) in grid().into_iter().enumerate() {
                    button {
                        key: "{idx}",
                        r#type: "button",
                        class: if cell.is_empty() { "voxel-cell" } else { "voxel-cell filled" },
                        style: if cell.is_empty() { String::new() } else { format!("background: {cell};") },
                        onclick: move |_| {
                            let col = selected_color();
                            play_mod_sound(if col.is_empty() { "anvil" } else { "minecraft" });
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

#[component]
fn NotesAndTodoPage(plugin: PluginMod) -> Element {
    let mut notes = use_signal(|| platform::storage_get(NOTES_KEY).unwrap_or_default());
    let mut todos = use_signal(|| -> Vec<TodoItem> {
        platform::storage_get(TODOS_KEY)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| vec![
                TodoItem { id: "1".into(), text: "Thử bật Mod giao diện Minecraft hoặc Cyberpunk".into(), done: false },
                TodoItem { id: "2".into(), text: "Tạo một Plugin tùy chỉnh riêng trong Mod Studio".into(), done: false },
            ])
    });
    let mut new_todo = use_signal(String::new);
    let save_todos = |list: &[TodoItem]| {
        if let Ok(s) = serde_json::to_string(list) {
            platform::storage_set(TODOS_KEY, &s);
        }
    };

    rsx! {
        div { class: "mod-page-container",
            div { class: "mod-two-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "📋 Danh sách việc cần làm — {plugin.display_name}" }
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
                        button { r#type: "submit", class: "btn-primary", PlusIcon { size: 16 } "Thêm" }
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
                div { class: "settings-group",
                    div { class: "settings-group-title", "📝 Sổ tay ghi chú nhanh" }
                    textarea {
                        class: "mod-notes-area",
                        rows: "14",
                        placeholder: "Viết ghi chú, ý tưởng, đoạn mã hoặc danh sách tại đây. Tự động lưu ngay khi gõ.",
                        value: "{notes}",
                        oninput: move |e| {
                            let v = e.value();
                            notes.set(v.clone());
                            platform::storage_set(NOTES_KEY, &v);
                        }
                    }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::copy_to_clipboard(&notes()), CopyIcon {} "Sao chép ghi chú" }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| { notes.set(String::new()); platform::storage_set(NOTES_KEY, ""); }, "Xóa sổ tay" }
                    }
                }
            }
        }
    }
}

#[component]
fn SoundboardPage(plugin: PluginMod) -> Element {
    let pads = [
        ("⛏ Ngọc XP Minecraft", "xp", "#10B981"),
        ("🆙 Lên Cấp Level Up", "levelup", "#22C55E"),
        ("🔨 Đe Sắt Anvil", "anvil", "#64748B"),
        ("⌨ Switch Cơ Blue", "mech", "#3B82F6"),
        ("⚡ Tia Laser Sci-Fi", "laser", "#06B6D4"),
        ("🪙 Đồng Xu 8-Bit", "coin", "#F59E0B"),
        ("🔔 Chuông Thông Báo", "chime", "#8B5CF6"),
        ("✨ Hiệu Ứng Pop", "pop", "#EC4899"),
    ];
    rsx! {
        div { class: "mod-page-container",
            div { class: "mod-hero-card",
                div {
                    h3 { class: "mod-hero-title", "🎹 Bàn Phím Cơ & Soundboard — {plugin.display_name}" }
                    p { class: "mod-hero-sub", "Bấm vào các ô âm thanh bên dưới để phát âm thanh tổng hợp thời gian thực bằng WebAudio API." }
                }
            }
            div { class: "soundboard-grid",
                for (label, kind, color) in pads {
                    button {
                        key: "{label}",
                        r#type: "button",
                        class: "soundboard-pad",
                        style: "--pad-color: {color};",
                        onclick: move |_| play_mod_sound(kind),
                        span { class: "soundboard-pad-title", "{label}" }
                        span { class: "soundboard-pad-sub", "Bấm để phát" }
                    }
                }
            }
        }
    }
}

#[component]
fn CyberDashboardPage(plugin: PluginMod, my_username: String) -> Element {
    let mut terminal_out = use_signal(|| format!("🛰 Trạm Điều Khiển {} sẵn sàng.\nNgười vận hành: @{}\nGõ hoặc bấm lệnh nhanh bên dưới để kiểm tra.", plugin.display_name, my_username));
    let mut cmd_in = use_signal(String::new);

    rsx! {
        div { class: "mod-page-container",
            div { class: "inv-grid",
                div { class: "inv-card",
                    div { class: "settings-label", "Mã hóa đầu cuối" }
                    div { class: "member-sub", "ECDH P-256 · AES-256-GCM · ECDSA" }
                }
                div { class: "inv-card",
                    div { class: "settings-label", "Trạng thái Mod Loader" }
                    div { class: "member-sub", "{PLUGINS.read().iter().filter(|p| p.enabled).count()} Plugin đang kích hoạt" }
                }
                div { class: "inv-card",
                    div { class: "settings-label", "Hiệu ứng màn hình" }
                    div { class: "member-sub", "Matrix Cyber Rain · 60 FPS" }
                }
            }
            div { class: "settings-group",
                div { class: "settings-group-title", "💻 Bảng điều khiển lệnh Plugin" }
                pre { class: "code-block", style: "min-height: 10rem;", "{terminal_out}" }
                form {
                    class: "inline-form",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let c = cmd_in().trim().to_string();
                        if c.is_empty() { return; }
                        let reply = try_run_plugin_command(&c, &my_username).unwrap_or_else(|| format!("Đã thực thi: {c}"));
                        terminal_out.with_mut(|t| {
                            t.push_str(&format!("\n\n> {c}\n{reply}"));
                        });
                        cmd_in.set(String::new());
                    },
                    input { r#type: "text", placeholder: "Nhập lệnh như /neon, /hack, /status, /roll, /calc 24*7", value: "{cmd_in}", oninput: move |e| cmd_in.set(e.value()) }
                    button { r#type: "submit", class: "btn-primary", "Chạy lệnh" }
                }
            }
        }
    }
}

#[component]
fn CustomBlocksPage(plugin: PluginMod, my_username: String) -> Element {
    let mut output = use_signal(String::new);
    rsx! {
        div { class: "mod-page-container",
            div { class: "mod-hero-card",
                div {
                    h3 { class: "mod-hero-title", "{plugin.icon} {plugin.display_name}" }
                    p { class: "mod-hero-sub", "{plugin.description}" }
                }
            }
            if !output().is_empty() {
                div { class: "status-badge success", "{output}" }
            }
            if plugin.custom_blocks.is_empty() {
                div { class: "inv-grid",
                    for c in plugin.commands.iter() {
                        {
                            let trig = c.trigger.clone();
                            let desc = c.description.clone();
                            let u = my_username.clone();
                            rsx! {
                                div { key: "{trig}", class: "inv-card",
                                    div { class: "settings-label", "{trig}" }
                                    div { class: "member-sub", "{desc}" }
                                    button {
                                        r#type: "button",
                                        class: "btn-small-primary",
                                        onclick: move |_| {
                                            if let Some(res) = try_run_plugin_command(&trig, &u) {
                                                output.set(res);
                                            }
                                        },
                                        "Chạy {trig}"
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "inv-grid",
                    for (i, b) in plugin.custom_blocks.iter().enumerate() {
                        {
                            let cmd = b.action_command.clone();
                            let u = my_username.clone();
                            rsx! {
                                div { key: "{i}", class: "inv-card",
                                    div { class: "settings-label", "{b.title}" }
                                    div { class: "member-sub", "{b.body}" }
                                    if !b.action_label.is_empty() {
                                        button {
                                            r#type: "button",
                                            class: "btn-small-primary",
                                            onclick: move |_| {
                                                let res = try_run_plugin_command(&cmd, &u).unwrap_or_else(|| expand_template(&cmd, &u, ""));
                                                output.set(res);
                                            },
                                            "{b.action_label}"
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

/// Chat with a Plugin (works for both server `/api/bots` AND local Mod Plugins!).
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
        .find(|p| p.id == bot.id || p.username == bot.username || p.server_bot_id.as_deref() == Some(bot.id.as_str()))
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

            // Check if this plugin or any enabled plugin has an instant command/auto-reply match
            let local_reply = if let Some(ref pm) = m_opt {
                let (cmd, args) = match text.split_once(char::is_whitespace) {
                    Some((c, a)) => (c.trim(), a.trim()),
                    None => (text.as_str(), ""),
                };
                pm.commands
                    .iter()
                    .find(|c| c.trigger.eq_ignore_ascii_case(cmd))
                    .map(|c| expand_template(&c.response, &u_name, args))
                    .or_else(|| try_run_plugin_command(&text, &u_name))
                    .or_else(|| {
                        if id.starts_with("mod:") {
                            Some(expand_template(
                                "🧩 Plugin {user}: Đã nhận '{args}'. Gõ /help hoặc bấm các nút lệnh nhanh bên dưới để chạy tính năng của Plugin!",
                                &u_name,
                                &text,
                            ))
                        } else {
                            None
                        }
                    })
            } else {
                try_run_plugin_command(&text, &u_name)
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
                msgs.with_mut(|l| {
                    l.push(user_msg);
                    if let Some(rep) = local_reply {
                        l.push(BotChatMessage {
                            id: format!("b-{}", now_ts + 1),
                            seq: now_ts + 1,
                            from_bot: true,
                            text: rep,
                            created_at: now,
                        });
                    }
                    if let Ok(s) = serde_json::to_string(l) {
                        platform::storage_set(&sk, &s);
                    }
                });
                sending.set(false);
                platform::scroll_chat_to_bottom(false);
            } else {
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
    let send_text_composer = send_text.clone();
    let send_text_cmds = send_text.clone();

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column wp-none",
                header { class: "chat-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại danh sách", onclick: move |_| on_back.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        Avatar { name: bot.display_name.clone(), avatar_url: bot.avatar_url.clone(), size: 38 }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{bot.display_name}" span { class: "bot-tag", "PLUGIN" } }
                            span { class: "chat-header-meta",
                                if is_local { "@{bot.username} · Plugin & Mod chạy trực tiếp trên thiết bị" } else { "@{bot.username} · Plugin kết nối API" }
                            }
                        }
                    }
                    div { class: "chat-header-actions",
                        if let Some(ref pm) = matching_mod {
                            if pm.custom_page_enabled {
                                {
                                    let pid = pm.id.clone();
                                    rsx! {
                                        button { r#type: "button", class: "btn-secondary",
                                            onclick: move |_| *ACTIVE_PLUGIN_PAGE.write() = Some(pid.clone()),
                                            "{pm.page_icon} Mở {pm.page_title}"
                                        }
                                    }
                                }
                            }
                        }
                        button { r#type: "button", class: "btn-ghost", title: "Cấu hình Plugin & Mod",
                            onclick: move |_| *ACTIVE_PLUGIN_PAGE.write() = Some("studio".into()),
                            SlidersIcon {} "Tùy chỉnh"
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
                MessageList {
                    messages: view,
                    show_sender_names: true,
                    intro_title: bot.display_name.clone(),
                    intro_sub: bot.description.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| "Gửi /help để xem danh sách lệnh của Plugin, hoặc tùy chỉnh giao diện & tính năng trong Mod Studio.".to_string()),
                    on_open_profile: move |_| {},
                    on_profile_click: move |_| {},
                    on_react: move |_| {},
                }
                if !quick_cmds.is_empty() {
                    div { class: "plugin-composer-bar",
                        for c in quick_cmds {
                            {
                                let trig = c.trigger.clone();
                                let desc = c.description.clone();
                                let send_chip = send_text_cmds.clone();
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

const THEME_PRESETS: [(&str, &str, &str, &str, &str, &str, &str, &str, &str); 6] = [
    ("Minecraft Voxel", "#141814", "#1D241D", "#181E18", "#242D24", "#10B981", "#059669", "#ECFDF5", "pixel"),
    ("Cyberpunk Neon", "#070812", "#0D1021", "#0A0C1A", "#141830", "#00F0FF", "#D900FF", "#E0FAFF", "neon"),
    ("Sakura Glass", "#FDF2F8", "#FCE7F3", "#FFF5F9", "#FFFFFF", "#EC4899", "#DB2777", "#4A152B", "glass"),
    ("AMOLED Pitch Black", "#000000", "#08080A", "#040406", "#101014", "#3B82F6", "#2563EB", "#F8FAFC", "default"),
    ("Dracula Purple", "#1E1F29", "#282A36", "#21222C", "#343746", "#BD93F9", "#FF79C6", "#F8F8F2", "neon"),
    ("Retro Terminal", "#051005", "#0A1C0A", "#071407", "#0F290F", "#22C55E", "#16A34A", "#4ADE80", "pixel"),
];

pub use PluginChatView as BotChatView;

/// Settings › Plugin & Mod Studio: manage, toggle, customize, and build Plugins & Mods.
#[component]
pub fn PluginsSettings(
    my_username: String,
    #[props(default)] on_open_bot_chat: Option<EventHandler<BotItem>>,
) -> Element {
    let mut server_bots = use_signal(Vec::<BotItem>::new);
    let mut editing_id = use_signal(|| Option::<String>::None);
    let mut editor_tab = use_signal(|| "basic"); // "basic" | "theme" | "ui" | "bot"
    let mut creating_new = use_signal(|| false);
    let mut import_open = use_signal(|| false);
    let mut import_json = use_signal(String::new);
    let mut status = use_signal(|| Option::<(bool, String)>::None);
    let mut fresh_token = use_signal(|| Option::<(String, String)>::None);
    let mut busy = use_signal(|| false);
    let mut origin = use_signal(String::new);

    // New Plugin form signals
    let mut new_username = use_signal(String::new);
    let mut new_display = use_signal(String::new);
    let mut new_icon = use_signal(|| "🧩".to_string());
    let mut new_desc = use_signal(String::new);
    let mut new_with_server = use_signal(|| false);

    // Command editor signals
    let mut cmd_trigger = use_signal(String::new);
    let mut cmd_desc = use_signal(String::new);
    let mut cmd_resp = use_signal(String::new);

    // Composer button editor signals
    let mut btn_label = use_signal(String::new);
    let mut btn_content = use_signal(String::new);

    use_effect(move || {
        init_plugins();
        spawn(async move {
            origin.set(platform::app_origin().await);
            if let Ok(list) = social::bots_mine().await {
                // Ensure every server bot also has a corresponding PluginMod entry so it can be modded!
                let mut current = PLUGINS.peek().clone();
                let mut changed = false;
                for sb in &list {
                    if !current.iter().any(|p| p.server_bot_id.as_deref() == Some(sb.id.as_str()) || p.username == sb.username) {
                        current.push(PluginMod {
                            id: format!("srv:{}", sb.id),
                            username: sb.username.clone(),
                            display_name: sb.display_name.clone(),
                            author: "Bạn".into(),
                            version: "1.0.0".into(),
                            icon: "🤖".into(),
                            description: sb.description.clone().unwrap_or_else(|| "Plugin kết nối API máy chủ".into()),
                            enabled: true,
                            builtin: false,
                            server_bot_id: Some(sb.id.clone()),
                            bot_enabled: true,
                            commands: vec![
                                PluginCommand { trigger: "/start".into(), description: "Khởi động plugin".into(), response: format!("Xin chào {{user}}! Đây là plugin @{}.", sb.username) },
                            ],
                            theme_enabled: false,
                            theme: PluginTheme::default(),
                            custom_page_enabled: false,
                            page_title: sb.display_name.clone(),
                            page_icon: "🤖".into(),
                            page_kind: "custom".into(),
                            custom_blocks: Vec::new(),
                            hud_enabled: false,
                            hud_position: "top-right".into(),
                            hud_coords: false,
                            hud_clock: false,
                            hud_note: false,
                            screen_fx: "none".into(),
                            sound_fx: "none".into(),
                            composer_buttons: Vec::new(),
                        });
                        changed = true;
                    }
                }
                if changed {
                    save_plugins(&current);
                    *PLUGINS.write() = current;
                }
                server_bots.set(list);
            }
        });
    });

    let update_plugin = move |id: &str, f: &dyn Fn(&mut PluginMod)| {
        let mut list = PLUGINS.peek().clone();
        if let Some(p) = list.iter_mut().find(|p| p.id == id) {
            f(p);
        }
        save_plugins(&list);
        *PLUGINS.write() = list;
    };

    let create_plugin = move |_| {
        let u = new_username().trim().to_lowercase().replace(' ', "_");
        let d = new_display().trim().to_string();
        if u.is_empty() || d.is_empty() || busy() {
            return;
        }
        let ic = if new_icon().trim().is_empty() { "🧩".to_string() } else { new_icon().trim().to_string() };
        let ds = if new_desc().trim().is_empty() { "Plugin & Mod tùy chỉnh cho ChatW".to_string() } else { new_desc().trim().to_string() };
        let want_srv = new_with_server();
        let author_name = my_username.clone();
        busy.set(true);
        status.set(None);
        spawn(async move {
            let mut srv_id: Option<String> = None;
            if want_srv {
                match social::bot_create(&u, &d, &ds).await {
                    Ok(b) => {
                        if let Some(tok) = b.token.clone() {
                            fresh_token.set(Some((b.username.clone(), tok)));
                        }
                        srv_id = Some(b.id.clone());
                        server_bots.with_mut(|l| l.insert(0, b));
                    }
                    Err(e) => {
                        status.set(Some((true, plugin_error(&e.code).to_string())));
                        busy.set(false);
                        return;
                    }
                }
            }
            let pid = srv_id.as_ref().map(|s| format!("srv:{s}")).unwrap_or_else(|| format!("mod:custom_{}", chrono::Utc::now().timestamp_millis()));
            let new_mod = PluginMod {
                id: pid.clone(),
                username: u,
                display_name: d.clone(),
                author: author_name,
                version: "1.0.0".into(),
                icon: ic.clone(),
                description: ds,
                enabled: true,
                builtin: false,
                server_bot_id: srv_id,
                bot_enabled: true,
                commands: vec![
                    PluginCommand { trigger: "/hello".into(), description: "Chào người dùng".into(), response: format!("{ic} Xin chào {{user}} từ Plugin {d}!") },
                ],
                theme_enabled: false,
                theme: PluginTheme::default(),
                custom_page_enabled: false,
                page_title: d,
                page_icon: ic,
                page_kind: "custom".into(),
                custom_blocks: Vec::new(),
                hud_enabled: false,
                hud_position: "top-right".into(),
                hud_coords: false,
                hud_clock: true,
                hud_note: false,
                screen_fx: "none".into(),
                sound_fx: "none".into(),
                composer_buttons: Vec::new(),
            };
            let mut list = PLUGINS.peek().clone();
            list.insert(0, new_mod);
            save_plugins(&list);
            *PLUGINS.write() = list;
            new_username.set(String::new());
            new_display.set(String::new());
            new_desc.set(String::new());
            creating_new.set(false);
            editing_id.set(Some(pid));
            status.set(Some((false, "Đã tạo Plugin mới. Bạn có thể tùy chỉnh giao diện, thêm trang, HUD và lệnh Bot bên dưới!".into())));
            busy.set(false);
        });
    };

    let base = if origin().is_empty() { "http://127.0.0.1:8085".to_string() } else { origin() };
    let plugins_list = PLUGINS.read().clone();
    let editing_mod = editing_id().and_then(|id| plugins_list.iter().find(|p| p.id == id).cloned());

    rsx! {
        div { class: "settings-section",
            div { class: "mod-studio-header",
                div {
                    div { class: "settings-group-title", "🧩 Quản lý Plugin & Mod — Giống Mod Minecraft" }
                    div { class: "member-sub", "Mỗi Plugin có thể làm Bot trò chuyện, biến đổi toàn bộ giao diện ứng dụng, thêm trang mới lên thanh điều hướng, bật HUD nổi, hiệu ứng hạt và âm thanh." }
                }
                div { class: "inline-form",
                    button { r#type: "button", class: "btn-primary", onclick: move |_| { creating_new.set(!creating_new()); import_open.set(false); },
                        PlusIcon { size: 16 } "Tạo Plugin mới"
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| { import_open.set(!import_open()); creating_new.set(false); },
                        "Nhập gói Mod JSON"
                    }
                }
            }

            if let Some((is_err, msg)) = status() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
            }

            if let Some((name, tok)) = fresh_token() {
                div { class: "token-reveal", role: "alert",
                    div { class: "settings-group-title", "Token API của Plugin @{name}" }
                    code { class: "token-code", "{tok}" }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-secondary", onclick: { let t = tok.clone(); move |_| platform::copy_to_clipboard(&t) }, CopyIcon {} "Sao chép token" }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| fresh_token.set(None), "Đã lưu" }
                    }
                }
            }

            if import_open() {
                div { class: "settings-group mod-builder-box",
                    div { class: "settings-group-title", "Nhập gói Plugin / Mod từ JSON" }
                    textarea {
                        rows: "4",
                        placeholder: "Dán chuỗi JSON của Plugin / Mod tại đây",
                        value: "{import_json}",
                        oninput: move |e| import_json.set(e.value()),
                    }
                    div { class: "inline-form",
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: move |_| {
                                match serde_json::from_str::<PluginMod>(&import_json()) {
                                    Ok(mut pm) => {
                                        if pm.id.is_empty() {
                                            pm.id = format!("mod:import_{}", chrono::Utc::now().timestamp_millis());
                                        }
                                        pm.enabled = true;
                                        let mut list = PLUGINS.peek().clone();
                                        if let Some(ex) = list.iter_mut().find(|x| x.id == pm.id) {
                                            *ex = pm;
                                        } else {
                                            list.insert(0, pm);
                                        }
                                        save_plugins(&list);
                                        *PLUGINS.write() = list;
                                        import_json.set(String::new());
                                        import_open.set(false);
                                        status.set(Some((false, "Đã cài đặt và kích hoạt gói Mod thành công!".into())));
                                    }
                                    Err(_) => status.set(Some((true, "Chuỗi JSON Mod không hợp lệ".into()))),
                                }
                            },
                            "Cài đặt Mod"
                        }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| import_open.set(false), "Đóng" }
                    }
                }
            }

            if creating_new() {
                div { class: "settings-group mod-builder-box",
                    div { class: "settings-group-title", "Tạo Plugin & Mod mới" }
                    div { class: "form-grid",
                        label { class: "form-field",
                            span { "Mã định danh @username" }
                            input { r#type: "text", maxlength: "32", placeholder: "vd: my_super_mod", value: "{new_username}", oninput: move |e| new_username.set(e.value()) }
                        }
                        label { class: "form-field",
                            span { "Tên hiển thị Plugin" }
                            input { r#type: "text", maxlength: "64", placeholder: "vd: Mod Giao Diện & Trợ Lý", value: "{new_display}", oninput: move |e| new_display.set(e.value()) }
                        }
                        label { class: "form-field",
                            span { "Biểu tượng" }
                            input { r#type: "text", maxlength: "8", placeholder: "🧩", value: "{new_icon}", oninput: move |e| new_icon.set(e.value()) }
                        }
                        label { class: "form-field wide",
                            span { "Mô tả chức năng" }
                            textarea { rows: "2", maxlength: "512", placeholder: "Plugin này biến đổi giao diện hoặc làm bot gì", value: "{new_desc}", oninput: move |e| new_desc.set(e.value()) }
                        }
                    }
                    label { class: "demo-check",
                        input { r#type: "checkbox", checked: new_with_server(), onchange: move |e| new_with_server.set(e.checked()) }
                        "Đồng thời đăng ký Token HTTP API trên máy chủ để lập trình Bot từ bên ngoài"
                    }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-primary", disabled: busy() || new_username().trim().is_empty() || new_display().trim().is_empty(), onclick: create_plugin,
                            PlusIcon { size: 16 } "Khởi tạo & Mở Mod Studio"
                        }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| creating_new.set(false), "Hủy" }
                    }
                }
            }

            // Full Visual Mod Studio Editor when editing a specific Plugin/Mod
            if let Some(em) = editing_mod {
                {
                    let eid = em.id.clone();
                    let eid_basic = em.id.clone();
                    let eid_theme = em.id.clone();
                    let eid_ui = em.id.clone();
                    let eid_bot = em.id.clone();
                    let srv_bot = em.server_bot_id.as_ref().and_then(|sid| server_bots.read().iter().find(|b| &b.id == sid).cloned());
                    let var_guide = "Biến hỗ trợ trong câu trả lời: {user}, {time}, {date}, {dice}, {rand100}, {coin}, {args}, {upper}, {calc}";
                    let resp_placeholder = "vd: Xin chào {user}! Số may mắn của bạn là {rand100}";
                    let curl_snippet = format!(
                        "# Gọi HTTP API cho Plugin có Token\ncurl {}/api/bot/TOKEN/getMe\ncurl \"{}/api/bot/TOKEN/getUpdates?offset=0&timeout=25\"\ncurl -X POST {}/api/bot/TOKEN/sendMessage -H 'Content-Type: application/json' -d '{{\"chat_id\": \"CHAT_ID\", \"text\": \"Xin chào từ Plugin!\"}}'",
                        base, base, base
                    );
                    rsx! {
                        div { class: "settings-group mod-studio-editor",
                            div { class: "mod-editor-top",
                                div { class: "mod-editor-title",
                                    span { class: "mod-card-icon", "{em.icon}" }
                                    div {
                                        div { class: "settings-group-title", "Chỉnh sửa Mod: {em.display_name}" }
                                        div { class: "member-sub", "@{em.username} · v{em.version} · Mọi thay đổi được áp dụng trực tiếp lên toàn ứng dụng" }
                                    }
                                }
                                div { class: "inline-form",
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        onclick: {
                                            let em_json = serde_json::to_string_pretty(&em).unwrap_or_default();
                                            move |_| {
                                                platform::copy_to_clipboard(&em_json);
                                                status.set(Some((false, "Đã sao chép gói Mod dạng JSON vào bộ nhớ tạm!".into())));
                                            }
                                        },
                                        CopyIcon {} "Xuất JSON"
                                    }
                                    button { r#type: "button", class: "btn-primary", onclick: move |_| editing_id.set(None), CheckIcon { size: 16 } "Hoàn tất" }
                                }
                            }

                            div { class: "segmented",
                                button { r#type: "button", class: if editor_tab() == "basic" { "active" } else { "" }, onclick: move |_| editor_tab.set("basic"), "1. Mô-đun & Vai trò" }
                                button { r#type: "button", class: if editor_tab() == "theme" { "active" } else { "" }, onclick: move |_| editor_tab.set("theme"), "2. Giao diện & CSS toàn App" }
                                button { r#type: "button", class: if editor_tab() == "ui" { "active" } else { "" }, onclick: move |_| editor_tab.set("ui"), "3. Thêm Trang, HUD & Hiệu ứng" }
                                button { r#type: "button", class: if editor_tab() == "bot" { "active" } else { "" }, onclick: move |_| editor_tab.set("bot"), "4. Bot, Lệnh & API Token" }
                            }

                            if editor_tab() == "basic" {
                                div { class: "form-grid",
                                    label { class: "form-field",
                                        span { "Tên Plugin / Mod" }
                                        input { r#type: "text", value: "{em.display_name}",
                                            oninput: { let id = eid_basic.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.display_name = v.clone()); } } }
                                    }
                                    label { class: "form-field",
                                        span { "Biểu tượng" }
                                        input { r#type: "text", maxlength: "8", value: "{em.icon}",
                                            oninput: { let id = eid_basic.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.icon = v.clone()); } } }
                                    }
                                    label { class: "form-field",
                                        span { "Phiên bản" }
                                        input { r#type: "text", maxlength: "16", value: "{em.version}",
                                            oninput: { let id = eid_basic.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.version = v.clone()); } } }
                                    }
                                    label { class: "form-field wide",
                                        span { "Mô tả" }
                                        textarea { rows: "2", value: "{em.description}",
                                            oninput: { let id = eid_basic.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.description = v.clone()); } } }
                                    }
                                }
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "🤖 Chế độ Bot trò chuyện & Xử lý lệnh /command" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.bot_enabled,
                                            onchange: { let id = eid_basic.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.bot_enabled = v); } } }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "🎨 Biến đổi & Tùy chỉnh giao diện toàn App" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.theme_enabled,
                                            onchange: { let id = eid_basic.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.theme_enabled = v); } } }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "🧭 Thêm trang giao diện mới lên thanh điều hướng chính" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.custom_page_enabled,
                                            onchange: { let id = eid_basic.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.custom_page_enabled = v); } } }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "🛰 Hiển thị bảng HUD nổi trên góc màn hình" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.hud_enabled,
                                            onchange: { let id = eid_basic.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.hud_enabled = v); } } }
                                    }
                                }
                            } else if editor_tab() == "theme" {
                                div { class: "settings-group",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Bật ghi đè giao diện toàn ứng dụng bởi Mod này" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.theme_enabled,
                                            onchange: { let id = eid_theme.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.theme_enabled = v); } } }
                                    }
                                    div { class: "settings-group-title", "Mẫu giao diện nhanh" }
                                    div { class: "mod-preset-chips",
                                        for (pname, bg_a, bg_s, bg_c, bg_sf, acc, sent, txt, bstyle) in THEME_PRESETS {
                                            button {
                                                key: "{pname}",
                                                r#type: "button",
                                                class: "btn-small-secondary",
                                                onclick: {
                                                    let id = eid_theme.clone();
                                                    move |_| {
                                                        update_plugin(&id, &|p| {
                                                            p.theme_enabled = true;
                                                            p.theme.bg_app = bg_a.into();
                                                            p.theme.bg_sidebar = bg_s.into();
                                                            p.theme.bg_chat = bg_c.into();
                                                            p.theme.bg_surface = bg_sf.into();
                                                            p.theme.accent = acc.into();
                                                            p.theme.msg_sent = sent.into();
                                                            p.theme.text_primary = txt.into();
                                                            p.theme.border_style = bstyle.into();
                                                        });
                                                    }
                                                },
                                                span { class: "voxel-swatch", style: "background: {acc};" }
                                                "{pname}"
                                            }
                                        }
                                    }
                                    div { class: "mod-color-grid",
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu nền toàn App" } }
                                            input { r#type: "color", value: if em.theme.bg_app.is_empty() { "#0C0E12" } else { em.theme.bg_app.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.bg_app = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu cột danh sách" } }
                                            input { r#type: "color", value: if em.theme.bg_sidebar.is_empty() { "#171A20" } else { em.theme.bg_sidebar.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.bg_sidebar = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu nền khung Chat" } }
                                            input { r#type: "color", value: if em.theme.bg_chat.is_empty() { "#191C22" } else { em.theme.bg_chat.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.bg_chat = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu thẻ & bề mặt" } }
                                            input { r#type: "color", value: if em.theme.bg_surface.is_empty() { "#22262E" } else { em.theme.bg_surface.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.bg_surface = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu nhấn chủ đạo" } }
                                            input { r#type: "color", value: if em.theme.accent.is_empty() { "#2764E7" } else { em.theme.accent.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.accent = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu bong bóng tin gửi" } }
                                            input { r#type: "color", value: if em.theme.msg_sent.is_empty() { "#2764E7" } else { em.theme.msg_sent.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.msg_sent = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu bong bóng tin nhận" } }
                                            input { r#type: "color", value: if em.theme.msg_recv.is_empty() { "#262C35" } else { em.theme.msg_recv.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.msg_recv = v.clone(); }); } } }
                                        }
                                        div { class: "settings-row",
                                            span { class: "grow", span { class: "settings-label", "Màu chữ chính" } }
                                            input { r#type: "color", value: if em.theme.text_primary.is_empty() { "#E8EAED" } else { em.theme.text_primary.as_str() },
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.text_primary = v.clone(); }); } } }
                                        }
                                    }
                                    div { class: "form-grid",
                                        label { class: "form-field",
                                            span { "Kiểu khung viền Mod" }
                                            select {
                                                value: "{em.theme.border_style}",
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.border_style = v.clone(); }); } },
                                                option { value: "default", "Mặc định tối giản" }
                                                option { value: "pixel", "Khối Pixel Minecraft 2px" }
                                                option { value: "neon", "Neon phát sáng" }
                                                option { value: "glass", "Kính mờ Acrylic Glass" }
                                            }
                                        }
                                        label { class: "form-field",
                                            span { "Độ bo góc toàn App" }
                                            select {
                                                value: "{em.theme.panel_radius}",
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.panel_radius = v.clone(); }); } },
                                                option { value: "", "Mặc định" }
                                                option { value: "0px", "0px · Vuông khối Voxel" }
                                                option { value: "6px", "6px · Gọn sắc nét" }
                                                option { value: "14px", "14px · Mềm mại" }
                                                option { value: "22px", "22px · Bo tròn tối đa" }
                                            }
                                        }
                                        label { class: "form-field",
                                            span { "Vị trí thanh điều hướng chính" }
                                            select {
                                                value: "{em.theme.rail_position}",
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.rail_position = v.clone(); }); } },
                                                option { value: "left", "Cột bên trái" }
                                                option { value: "right", "Cột bên phải" }
                                                option { value: "top", "Thanh ngang phía trên" }
                                            }
                                        }
                                        label { class: "form-field",
                                            span { "Phông chữ Mod" }
                                            select {
                                                value: "{em.theme.font_style}",
                                                onchange: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.font_style = v.clone(); }); } },
                                                option { value: "default", "Mặc định hệ thống" }
                                                option { value: "pixel", "Pixel / Monospace" }
                                                option { value: "rounded", "Bo tròn mềm mại" }
                                                option { value: "serif", "Cổ điển có chân" }
                                            }
                                        }
                                    }
                                    label { class: "form-field",
                                        span { "Mã CSS tùy chỉnh toàn App" }
                                        textarea {
                                            rows: "5",
                                            style: "font-family: var(--font-mono); font-size: 12px;",
                                            placeholder: "Viết bất kỳ quy tắc CSS nào để biến đổi giao diện toàn bộ ứng dụng",
                                            value: "{em.theme.custom_css}",
                                            oninput: { let id = eid_theme.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| { p.theme_enabled = true; p.theme.custom_css = v.clone(); }); } }
                                        }
                                    }
                                }
                            } else if editor_tab() == "ui" {
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "1. Trang riêng trên thanh điều hướng" }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Thêm nút mở trang của Plugin này lên thanh điều hướng chính" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.custom_page_enabled,
                                            onchange: { let id = eid_ui.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.custom_page_enabled = v); } } }
                                    }
                                    if em.custom_page_enabled {
                                        div { class: "form-grid",
                                            label { class: "form-field",
                                                span { "Tiêu đề trang" }
                                                input { r#type: "text", value: "{em.page_title}",
                                                    oninput: { let id = eid_ui.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.page_title = v.clone()); } } }
                                            }
                                            label { class: "form-field",
                                                span { "Biểu tượng trên thanh điều hướng" }
                                                input { r#type: "text", maxlength: "8", value: "{em.page_icon}",
                                                    oninput: { let id = eid_ui.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.page_icon = v.clone()); } } }
                                            }
                                            label { class: "form-field",
                                                span { "Loại giao diện trang" }
                                                select {
                                                    value: "{em.page_kind}",
                                                    onchange: { let id = eid_ui.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.page_kind = v.clone()); } },
                                                    option { value: "voxel", "Thế giới Khối Voxel 2D Minecraft" }
                                                    option { value: "notes", "Sổ tay & Danh sách việc cần làm" }
                                                    option { value: "soundboard", "Bàn phím cơ & Soundboard 8-Bit" }
                                                    option { value: "cyber", "Trạm Điều Khiển & Terminal Lệnh" }
                                                    option { value: "custom", "Bảng chức năng tùy chỉnh" }
                                                }
                                            }
                                        }
                                    }

                                    div { class: "settings-group-title", "2. Hiệu ứng toàn màn hình & Âm thanh Mod" }
                                    div { class: "form-grid",
                                        label { class: "form-field",
                                            span { "Hiệu ứng hạt toàn màn hình" }
                                            select {
                                                value: "{em.screen_fx}",
                                                onchange: { let id = eid_ui.clone(); move |e| { let v = e.value(); update_plugin(&id, &|p| p.screen_fx = v.clone()); } },
                                                option { value: "none", "Không bật" }
                                                option { value: "voxel", "Hạt Voxel XP lấp lánh" }
                                                option { value: "sakura", "Hoa anh đào rơi" }
                                                option { value: "snow", "Tuyết rơi mùa đông" }
                                                option { value: "matrix", "Mưa ma trận Matrix" }
                                            }
                                        }
                                        label { class: "form-field",
                                            span { "Gói âm thanh & Bàn phím" }
                                            select {
                                                value: "{em.sound_fx}",
                                                onchange: { let id = eid_ui.clone(); move |e| { let v = e.value(); play_mod_sound(&v); update_plugin(&id, &|p| p.sound_fx = v.clone()); } },
                                                option { value: "none", "Mặc định" }
                                                option { value: "minecraft", "Âm thanh 8-Bit Minecraft" }
                                                option { value: "mech", "Tiếng gõ bàn phím cơ trực tiếp" }
                                                option { value: "scifi", "Âm thanh Laser Sci-Fi" }
                                            }
                                        }
                                    }

                                    div { class: "settings-group-title", "3. Bảng HUD nổi trên màn hình" }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Hiển thị HUD nổi của Mod này" } }
                                        input { r#type: "checkbox", class: "switch", checked: em.hud_enabled,
                                            onchange: { let id = eid_ui.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.hud_enabled = v); } } }
                                    }
                                    if em.hud_enabled {
                                        div { class: " studio-toggles",
                                            label { class: "demo-check",
                                                input { r#type: "checkbox", checked: em.hud_coords,
                                                    onchange: { let id = eid_ui.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.hud_coords = v); } } }
                                                "Hiện tọa độ XYZ & FPS"
                                            }
                                            label { class: "demo-check",
                                                input { r#type: "checkbox", checked: em.hud_clock,
                                                    onchange: { let id = eid_ui.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.hud_clock = v); } } }
                                                "Hiện đồng hồ số"
                                            }
                                            label { class: "demo-check",
                                                input { r#type: "checkbox", checked: em.hud_note,
                                                    onchange: { let id = eid_ui.clone(); move |e| { let v = e.checked(); update_plugin(&id, &|p| p.hud_note = v); } } }
                                                "Hiện ô ghi chú nhanh trên HUD"
                                            }
                                        }
                                    }

                                    div { class: "settings-group-title", "4. Thanh nút nhanh trong khung soạn tin nhắn" }
                                    div { class: "inline-form",
                                        input { r#type: "text", placeholder: "Nhãn nút, vd: ⛏ /craft", value: "{btn_label}", oninput: move |e| btn_label.set(e.value()) }
                                        input { r#type: "text", placeholder: "Nội dung hoặc lệnh gửi đi", value: "{btn_content}", oninput: move |e| btn_content.set(e.value()) }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            onclick: {
                                                let id = eid_ui.clone();
                                                move |_| {
                                                    let l = btn_label().trim().to_string();
                                                    let c = btn_content().trim().to_string();
                                                    if l.is_empty() || c.is_empty() { return; }
                                                    update_plugin(&id, &|p| p.composer_buttons.push(PluginComposerBtn { label: l.clone(), content: c.clone() }));
                                                    btn_label.set(String::new());
                                                    btn_content.set(String::new());
                                                }
                                            },
                                            PlusIcon { size: 14 } "Thêm nút"
                                        }
                                    }
                                    div { class: "mod-preset-chips",
                                        for (bi, b) in em.composer_buttons.iter().enumerate() {
                                            span { key: "{bi}", class: "plugin-cmd-chip",
                                                "{b.label} → {b.content}"
                                                button {
                                                    r#type: "button",
                                                    class: "chip-del",
                                                    onclick: {
                                                        let id = eid_ui.clone();
                                                        move |_| update_plugin(&id, &|p| { if bi < p.composer_buttons.len() { p.composer_buttons.remove(bi); } })
                                                    },
                                                    "×"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Lệnh /command & Tự động trả lời của Plugin" }
                                    div { class: "member-sub", "{var_guide}" }
                                    div { class: "form-grid",
                                        label { class: "form-field",
                                            span { "Lệnh kích hoạt" }
                                            input { r#type: "text", placeholder: "vd: /hello hoặc /craft", value: "{cmd_trigger}", oninput: move |e| cmd_trigger.set(e.value()) }
                                        }
                                        label { class: "form-field",
                                            span { "Mô tả ngắn" }
                                            input { r#type: "text", placeholder: "Lệnh này làm gì", value: "{cmd_desc}", oninput: move |e| cmd_desc.set(e.value()) }
                                        }
                                        label { class: "form-field wide",
                                            span { "Nội dung Plugin phản hồi" }
                                            input { r#type: "text", placeholder: "{resp_placeholder}", value: "{cmd_resp}", oninput: move |e| cmd_resp.set(e.value()) }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        onclick: {
                                            let id = eid_bot.clone();
                                            move |_| {
                                                let mut trig = cmd_trigger().trim().to_string();
                                                let resp = cmd_resp().trim().to_string();
                                                if trig.is_empty() || resp.is_empty() { return; }
                                                if !trig.starts_with('/') { trig = format!("/{trig}"); }
                                                let d = if cmd_desc().trim().is_empty() { "Lệnh tùy chỉnh".to_string() } else { cmd_desc().trim().to_string() };
                                                update_plugin(&id, &|p| {
                                                    p.bot_enabled = true;
                                                    p.commands.push(PluginCommand { trigger: trig.clone(), description: d.clone(), response: resp.clone() });
                                                });
                                                cmd_trigger.set(String::new());
                                                cmd_desc.set(String::new());
                                                cmd_resp.set(String::new());
                                            }
                                        },
                                        PlusIcon { size: 15 } "Thêm lệnh cho Plugin"
                                    }
                                    div { class: "mod-todo-list",
                                        for (ci, c) in em.commands.iter().enumerate() {
                                            div { key: "{ci}", class: "mod-todo-row",
                                                code { class: "kbd", "{c.trigger}" }
                                                span { class: "settings-label", "{c.description}" }
                                                span { class: "grow member-sub", "→ {c.response}" }
                                                button {
                                                    r#type: "button",
                                                    class: "icon-btn danger",
                                                    onclick: {
                                                        let id = eid_bot.clone();
                                                        move |_| update_plugin(&id, &|p| { if ci < p.commands.len() { p.commands.remove(ci); } })
                                                    },
                                                    TrashIcon { size: 15 }
                                                }
                                            }
                                        }
                                    }

                                    div { class: "settings-group-title", "Kết nối HTTP API Token máy chủ" }
                                    if let Some(sb) = srv_bot {
                                        div { class: "bot-card",
                                            div { class: "bot-card-info",
                                                div { class: "bot-card-name", "@{sb.username}" span { class: "bot-tag", "API TOKEN" } }
                                                if let Some(h) = sb.token_hint.as_ref() { div { class: "bot-card-sub mono", "Token: {h}" } }
                                            }
                                            div { class: "bot-card-actions",
                                                button {
                                                    r#type: "button",
                                                    class: "btn-secondary",
                                                    onclick: {
                                                        let sid = sb.id.clone();
                                                        move |_| {
                                                            let sid = sid.clone();
                                                            spawn(async move {
                                                                if let Ok(nb) = social::bot_rotate(&sid).await {
                                                                    if let Some(t) = nb.token.clone() {
                                                                        fresh_token.set(Some((nb.username.clone(), t)));
                                                                    }
                                                                    server_bots.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nb.id) { *x = nb; });
                                                                }
                                                            });
                                                        }
                                                    },
                                                    RefreshIcon { size: 15 } "Cấp lại Token mới"
                                                }
                                                button {
                                                    r#type: "button",
                                                    class: "btn-ghost",
                                                    onclick: {
                                                        let sid = sb.id.clone();
                                                        let next = !sb.is_public;
                                                        move |_| {
                                                            let sid = sid.clone();
                                                            spawn(async move {
                                                                if let Ok(nb) = social::bot_update(&sid, json!({ "is_public": next })).await {
                                                                    server_bots.with_mut(|l| if let Some(x) = l.iter_mut().find(|x| x.id == nb.id) { *x = nb; });
                                                                }
                                                            });
                                                        }
                                                    },
                                                    if sb.is_public { "Chuyển về riêng tư" } else { "Công khai lên danh bạ" }
                                                }
                                            }
                                        }
                                    } else {
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            onclick: {
                                                let id = eid.clone();
                                                let u = em.username.clone();
                                                let d = em.display_name.clone();
                                                let ds = em.description.clone();
                                                move |_| {
                                                    let (id, u, d, ds) = (id.clone(), u.clone(), d.clone(), ds.clone());
                                                    spawn(async move {
                                                        match social::bot_create(&u, &d, &ds).await {
                                                            Ok(b) => {
                                                                if let Some(tok) = b.token.clone() {
                                                                    fresh_token.set(Some((b.username.clone(), tok)));
                                                                }
                                                                let sid = b.id.clone();
                                                                server_bots.with_mut(|l| l.insert(0, b));
                                                                let mut list = PLUGINS.peek().clone();
                                                                if let Some(pm) = list.iter_mut().find(|p| p.id == id) {
                                                                    pm.server_bot_id = Some(sid);
                                                                }
                                                                save_plugins(&list);
                                                                *PLUGINS.write() = list;
                                                            }
                                                            Err(e) => status.set(Some((true, plugin_error(&e.code).to_string()))),
                                                        }
                                                    });
                                                }
                                            },
                                            BotIcon { size: 16 } "Tạo Token HTTP API trên máy chủ cho Plugin này"
                                        }
                                    }
                                    pre { class: "code-block",
                                        "{curl_snippet}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Installed & Built-in Mods Grid
            div { class: "settings-group",
                div { class: "settings-group-title", "Kho Plugin & Mod trên thiết bị" }
                div { class: "mod-cards-grid",
                    for pm in plugins_list {
                        {
                            let pid = pm.id.clone();
                            let pid_edit = pm.id.clone();
                            let pid_page = pm.id.clone();
                            let pid_del = pm.id.clone();
                            let srv_id_del = pm.server_bot_id.clone();
                            let pm_chat = pm.clone();
                            let is_on = pm.enabled;
                            rsx! {
                                div { key: "{pm.id}", class: if is_on { "mod-card active" } else { "mod-card" },
                                    div { class: "mod-card-head",
                                        span { class: "mod-card-icon", "{pm.icon}" }
                                        div { class: "mod-card-title-wrap",
                                            div { class: "mod-card-title",
                                                "{pm.display_name}"
                                                span { class: "bot-tag", if pm.server_bot_id.is_some() { "PLUGIN API" } else { "MOD" } }
                                            }
                                            div { class: "mod-card-meta", "@{pm.username} · v{pm.version} · {pm.author}" }
                                        }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: is_on,
                                            aria_label: "Bật {pm.display_name}",
                                            onchange: move |e| {
                                                let v = e.checked();
                                                if v { play_mod_sound("xp"); }
                                                update_plugin(&pid, &|p| p.enabled = v);
                                            }
                                        }
                                    }
                                    p { class: "mod-card-desc", "{pm.description}" }
                                    div { class: "mod-capability-pills",
                                        if pm.theme_enabled { span { class: "mod-cap-pill", PaletteIcon {} "Giao diện App" } }
                                        if pm.custom_page_enabled { span { class: "mod-cap-pill", "🧭 Trang: {pm.page_title}" } }
                                        if pm.hud_enabled { span { class: "mod-cap-pill", "🛰 HUD nổi" } }
                                        if pm.screen_fx != "none" { span { class: "mod-cap-pill", SparkleIcon { size: 13 } "FX: {pm.screen_fx}" } }
                                        if pm.bot_enabled { span { class: "mod-cap-pill", BotIcon { size: 13 } "{pm.commands.len()} lệnh Bot" } }
                                    }
                                    div { class: "mod-card-actions",
                                        if pm.custom_page_enabled {
                                            button {
                                                r#type: "button",
                                                class: "btn-small-primary",
                                                onclick: move |_| {
                                                    update_plugin(&pid_page, &|p| p.enabled = true);
                                                    *ACTIVE_PLUGIN_PAGE.write() = Some(pid_page.clone());
                                                },
                                                "{pm.page_icon} Mở trang"
                                            }
                                        }
                                        if pm.bot_enabled {
                                            if let Some(h) = on_open_bot_chat {
                                                button {
                                                    r#type: "button",
                                                    class: "btn-small-secondary",
                                                    onclick: move |_| {
                                                        h.call(BotItem {
                                                            id: pm_chat.server_bot_id.clone().unwrap_or_else(|| pm_chat.id.clone()),
                                                            owner_id: "local".into(),
                                                            username: pm_chat.username.clone(),
                                                            display_name: format!("{} {}", pm_chat.icon, pm_chat.display_name),
                                                            description: Some(pm_chat.description.clone()),
                                                            avatar_url: None,
                                                            is_public: true,
                                                            verification_status: "verified".into(),
                                                            token_hint: None,
                                                            token: None,
                                                            last_text: None,
                                                        });
                                                    },
                                                    "Trò chuyện"
                                                }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-small-secondary",
                                            onclick: move |_| editing_id.set(if editing_id().as_deref() == Some(pid_edit.as_str()) { None } else { Some(pid_edit.clone()) }),
                                            SlidersIcon {} "Tùy chỉnh Mod"
                                        }
                                        if !pm.builtin {
                                            button {
                                                r#type: "button",
                                                class: "icon-btn danger",
                                                title: "Xóa Plugin",
                                                onclick: move |_| {
                                                    let id = pid_del.clone();
                                                    let sid = srv_id_del.clone();
                                                    let mut list = PLUGINS.peek().clone();
                                                    list.retain(|x| x.id != id);
                                                    save_plugins(&list);
                                                    *PLUGINS.write() = list;
                                                    if let Some(s) = sid {
                                                        spawn(async move {
                                                            let _ = social::bot_delete(&s).await;
                                                        });
                                                    }
                                                },
                                                TrashIcon { size: 15 }
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

pub use PluginsSettings as BotsSettings;

