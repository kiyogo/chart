//! Built-in Plugin Workspace Surfaces, Left-Sidebar Apps, Custom Blocks & Plugin Chat View.
//! Extracted from `plugins_view.rs` as a focused domain submodule (Spec 051).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        chat_extras::wallpaper_attrs,
        icons::{
            BackIcon, CloseIcon, CoinIcon, CopyIcon, DiceIcon, GroupIcon, PanelRightIcon, PlusIcon,
            SendIcon, SlidersIcon, TrashIcon,
        },
        messages::{Composer, MessageList, ViewMessage},
        plugins_view::{
            ACTIVE_LEFT_PLUGIN, ACTIVE_PLUGIN_PAGE, BotsSettings, EDITING_PLUGIN_ID, NOTES_KEY,
            PENDING_COMPOSER_TEXT, PLUGINS, PluginCommand, PluginCustomBlock, PluginGlyph,
            PluginMod, TODOS_KEY, VOXEL_KEY, eval_simple_math, expand_template, glyph_slot_size,
            play_mod_sound, play_mod_sound_opts, plugin_error, try_run_plugin_command,
        },
    },
    models::{BotChatMessage, BotItem, GroupItem},
    platform,
    prefs::PrefsCtx,
    services::social,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TodoItem {
    id: String,
    text: String,
    done: bool,
}

fn active_wallpaper_attrs() -> (String, String) {
    let default_wp = try_use_context::<PrefsCtx>()
        .map(|c| c.prefs.read().wallpaper.clone())
        .unwrap_or_else(|| "none".into());
    wallpaper_attrs(None, &default_wp)
}

pub const SOUNDBOARD_PADS: [(&str, &str, &str); 8] = [
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

pub const VOXEL_PALETTE: [(&str, &str); 12] = [
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
