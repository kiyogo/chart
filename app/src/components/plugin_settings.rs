//! Spec 032: Modular Per-Plugin Settings & Store UI.
//! Every plugin renders its own dedicated, reorderable, live-functional settings panel.
//! Zero fake Bot / Trang / Giao diện / HUD badges or cloned generic description forms.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        icons::{
            BackIcon, CheckIcon, CopyIcon, DownloadIcon, PlusIcon, SlidersIcon, TrashIcon,
        },
        plugins_view::{
            ACTIVE_LEFT_PLUGIN, ACTIVE_PLUGIN_PAGE, EDITING_PLUGIN_ID, PLUGINS, PluginCommand,
            PluginComposerBtn, PluginCustomBlock, PluginGlyph, PluginMod, PluginSettingDef,
            PluginTheme, SOUNDBOARD_PADS, VOXEL_PALETTE, glyph_slot_size, init_plugins,
            play_mod_sound, play_mod_sound_opts, save_plugins, try_run_plugin_command,
            NOTES_KEY, TODOS_KEY, VOXEL_KEY,
        },
    },
    models::BotItem,
    platform,
    services::{library, social},
};

fn update_plugin_by_id(id: &str, f: &dyn Fn(&mut PluginMod)) {
    let mut list = PLUGINS.peek().clone();
    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
        f(p);
    }
    save_plugins(&list);
    *PLUGINS.write() = list;
}

fn open_plugin_surface(pm: &PluginMod) {
    let pid = pm.id.clone();
    update_plugin_by_id(&pid, &|p| {
        p.installed = true;
        p.enabled = true;
    });
    if pm.left_app_enabled && pm.cfg_str("open_slot", "sidebar") == "sidebar" {
        *ACTIVE_PLUGIN_PAGE.write() = None;
        *ACTIVE_LEFT_PLUGIN.write() = Some(pid);
    } else {
        *ACTIVE_LEFT_PLUGIN.write() = None;
        *ACTIVE_PLUGIN_PAGE.write() = Some(pid);
    }
}

fn plugin_has_workspace(pm: &PluginMod) -> bool {
    pm.page_kind != "theme_sakura" && (pm.left_app_enabled || pm.custom_page_enabled || !pm.custom_blocks.is_empty())
}

/// Reusable reorderable + toggleable section list (live controls for any plugin's layout).
#[component]
fn SectionOrderControl(
    plugin_id: String,
    items: Vec<(String, String)>,
    ordered_keys: Vec<String>,
    hidden_keys: Vec<String>,
    default_keys: Vec<String>,
) -> Element {
    let total = ordered_keys.len();
    rsx! {
        div { class: "plugin-order-list",
            for (idx, key) in ordered_keys.iter().enumerate() {
                {
                    let label = items
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, l)| l.clone())
                        .unwrap_or_else(|| key.clone());
                    let is_visible = !hidden_keys.iter().any(|h| h == key);
                    let pid_up = plugin_id.clone();
                    let pid_dn = plugin_id.clone();
                    let pid_tg = plugin_id.clone();
                    let k_up = key.clone();
                    let k_dn = key.clone();
                    let k_tg = key.clone();
                    let defs_up = default_keys.clone();
                    let defs_dn = default_keys.clone();
                    rsx! {
                        div { key: "ord-{key}", class: if is_visible { "plugin-order-row" } else { "plugin-order-row dimmed" },
                            span { class: "plugin-order-index", "{idx + 1}" }
                            span { class: "plugin-order-label grow", "{label}" }
                            div { class: "plugin-order-btns",
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: idx == 0,
                                    title: "Đưa lên",
                                    aria_label: "Đưa {label} lên",
                                    onclick: move |_| {
                                        let refs: Vec<&str> = defs_up.iter().map(String::as_str).collect();
                                        update_plugin_by_id(&pid_up, &|p| p.move_section(&k_up, -1, &refs));
                                    },
                                    "↑"
                                }
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: idx + 1 >= total,
                                    title: "Đưa xuống",
                                    aria_label: "Đưa {label} xuống",
                                    onclick: move |_| {
                                        let refs: Vec<&str> = defs_dn.iter().map(String::as_str).collect();
                                        update_plugin_by_id(&pid_dn, &|p| p.move_section(&k_dn, 1, &refs));
                                    },
                                    "↓"
                                }
                            }
                            input {
                                r#type: "checkbox",
                                class: "switch",
                                checked: is_visible,
                                aria_label: "Hiện {label}",
                                onchange: move |_| update_plugin_by_id(&pid_tg, &|p| p.toggle_section(&k_tg)),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Settings › Plugin: clean store & per-plugin modular settings views.
#[component]
pub fn PluginsSettings(
    my_username: String,
    #[props(default)] on_open_bot_chat: Option<EventHandler<BotItem>>,
) -> Element {
    let _ = on_open_bot_chat;
    let mut creating_new = use_signal(|| false);
    let mut import_open = use_signal(|| false);
    let mut import_json = use_signal(String::new);
    let mut status = use_signal(|| Option::<(bool, String)>::None);
    let mut logo_editor_open = use_signal(|| false);

    // Create new plugin form signals (clean, no description or bot boilerplate)
    let mut new_display = use_signal(String::new);
    let mut new_username = use_signal(String::new);
    let mut new_logo = use_signal(|| "logo:code".to_string());
    let mut new_allow_logo = use_signal(|| true);
    let mut new_template = use_signal(|| "blocks".to_string()); // "blocks" | "commands" | "style"

    use_effect(move || {
        init_plugins();
    });

    let plugins_list = PLUGINS.read().clone();
    let editing_mod = EDITING_PLUGIN_ID().and_then(|id| plugins_list.iter().find(|p| p.id == id).cloned());

    // 1. Create New Custom Plugin Screen
    if creating_new() {
        let author_name = my_username.clone();
        return rsx! {
            div { class: "settings-section",
                div { class: "mod-editor-top",
                    div { class: "mod-editor-title",
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| creating_new.set(false), BackIcon {} "Quay lại" }
                        div { class: "settings-group-title", "Tạo Plugin mới" }
                    }
                }
                if let Some((is_err, msg)) = status() {
                    div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
                }
                div { class: "plugin-cfg-card",
                    div { class: "form-grid",
                        label { class: "form-field",
                            span { "Tên Plugin" }
                            input {
                                r#type: "text",
                                maxlength: "64",
                                placeholder: "vd: Bảng tính nhanh",
                                value: "{new_display}",
                                oninput: move |e| new_display.set(e.value()),
                            }
                        }
                        label { class: "form-field",
                            span { "Mã định danh" }
                            input {
                                r#type: "text",
                                maxlength: "32",
                                placeholder: "vd: quick_sheet",
                                value: "{new_username}",
                                oninput: move |e| new_username.set(e.value()),
                            }
                        }
                    }
                    div { class: "plugin-field-row",
                        span { class: "settings-label", "Logo của Plugin" }
                        div { class: "inline-form",
                            PluginGlyph { icon: String::new(), logo_url: new_logo(), size: 34 }
                            input {
                                r#type: "text",
                                placeholder: "URL ảnh hoặc mã <svg>",
                                value: "{new_logo}",
                                oninput: move |e| new_logo.set(e.value()),
                            }
                            label { class: "btn-avatar-upload",
                                input {
                                    id: "new-plugin-logo-file",
                                    r#type: "file",
                                    accept: "image/png,image/jpeg,image/webp,image/svg+xml,image/avif,image/gif",
                                    style: "display: none;",
                                    onchange: move |_| {
                                        spawn(async move {
                                            if let Ok((bytes, mime)) = platform::read_selected_file_raw("new-plugin-logo-file").await {
                                                if let Ok(u) = library::upload_media(bytes, &mime).await {
                                                    new_logo.set(u);
                                                }
                                            }
                                        });
                                    }
                                }
                                "Tải ảnh"
                            }
                        }
                    }
                    div { class: "plugin-field-row",
                        span { class: "settings-label", "Kiểu khởi tạo" }
                        div { class: "segmented",
                            button { r#type: "button", class: if new_template() == "blocks" { "active" } else { "" }, onclick: move |_| new_template.set("blocks".into()), "Khối tiện ích" }
                            button { r#type: "button", class: if new_template() == "commands" { "active" } else { "" }, onclick: move |_| new_template.set("commands".into()), "Lệnh gõ tắt" }
                            button { r#type: "button", class: if new_template() == "style" { "active" } else { "" }, onclick: move |_| new_template.set("style".into()), "Tùy biến CSS" }
                        }
                    }
                    label { class: "settings-row toggle-row",
                        span { class: "grow", span { class: "settings-label", "Cho phép người dùng đổi Logo" } }
                        input {
                            r#type: "checkbox",
                            class: "switch",
                            checked: new_allow_logo(),
                            onchange: move |e| new_allow_logo.set(e.checked()),
                        }
                    }
                    div { class: "inline-form",
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            disabled: new_display().trim().is_empty(),
                            onclick: move |_| {
                                let d = new_display().trim().to_string();
                                if d.is_empty() { return; }
                                let u_raw = new_username().trim().to_lowercase().replace(' ', "_");
                                let u = if u_raw.is_empty() {
                                    format!("plugin_{}", chrono::Utc::now().timestamp_millis() % 100000)
                                } else {
                                    u_raw
                                };
                                let pid = format!("mod:custom_{}", chrono::Utc::now().timestamp_millis());
                                let lg = if new_logo().trim().is_empty() { "logo:code".to_string() } else { new_logo().trim().to_string() };
                                let tpl = new_template();
                                let custom_blocks = if tpl == "blocks" {
                                    vec![PluginCustomBlock {
                                        kind: "counter".into(),
                                        title: "Bộ đếm".into(),
                                        body: String::new(),
                                        action_label: String::new(),
                                        action_command: String::new(),
                                        action_type: "command".into(),
                                    }]
                                } else {
                                    Vec::new()
                                };
                                let commands = if tpl == "commands" {
                                    vec![PluginCommand {
                                        trigger: "/ping".into(),
                                        description: "Phản hồi nhanh".into(),
                                        response: format!("Đã chạy lệnh từ {d} lúc {{time}}"),
                                        icon: String::new(),
                                        enabled: true,
                                    }]
                                } else {
                                    Vec::new()
                                };
                                let new_mod = PluginMod {
                                    id: pid.clone(),
                                    username: u,
                                    display_name: d.clone(),
                                    author: author_name.clone(),
                                    version: "1.0.0".into(),
                                    icon: String::new(),
                                    logo_url: lg.clone(),
                                    default_logo_url: lg,
                                    allow_custom_logo: new_allow_logo(),
                                    description: String::new(),
                                    installed: true,
                                    enabled: true,
                                    builtin: false,
                                    server_bot_id: None,
                                    bot_enabled: false,
                                    commands,
                                    auto_reply: String::new(),
                                    webhook_url: String::new(),
                                    theme_enabled: tpl == "style",
                                    theme: PluginTheme::default(),
                                    left_app_enabled: tpl == "blocks",
                                    custom_page_enabled: tpl != "style",
                                    page_title: d,
                                    page_icon: String::new(),
                                    page_kind: "custom".into(),
                                    custom_blocks,
                                    section_order: Vec::new(),
                                    hidden_sections: Vec::new(),
                                    config: serde_json::Map::new(),
                                    custom_settings: vec![PluginSettingDef {
                                        key: "compact_mode".into(),
                                        label: "Chế độ gọn".into(),
                                        kind: "toggle".into(),
                                        options: Vec::new(),
                                        min: 0,
                                        max: 100,
                                        default_value: "false".into(),
                                    }],
                                    hud_enabled: false,
                                    hud_position: String::new(),
                                    hud_coords: false,
                                    hud_clock: false,
                                    hud_note: false,
                                    hud_pomodoro: false,
                                    screen_fx: "none".into(),
                                    sound_fx: "none".into(),
                                    composer_buttons: Vec::new(),
                                };
                                let mut list = PLUGINS.peek().clone();
                                list.insert(0, new_mod);
                                save_plugins(&list);
                                *PLUGINS.write() = list;
                                new_display.set(String::new());
                                new_username.set(String::new());
                                creating_new.set(false);
                                *EDITING_PLUGIN_ID.write() = Some(pid);
                                status.set(Some((false, "Đã tạo Plugin mới.".into())));
                            },
                            PlusIcon { size: 16 } "Tạo Plugin"
                        }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| creating_new.set(false), "Hủy" }
                    }
                }
            }
        };
    }

    // 2. Import JSON Screen
    if import_open() {
        return rsx! {
            div { class: "settings-section",
                div { class: "mod-editor-top",
                    div { class: "mod-editor-title",
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| import_open.set(false), BackIcon {} "Quay lại" }
                        div { class: "settings-group-title", "Nhập Plugin JSON" }
                    }
                }
                if let Some((is_err, msg)) = status() {
                    div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
                }
                div { class: "plugin-cfg-card",
                    textarea {
                        rows: "7",
                        placeholder: "Dán cấu hình JSON của Plugin tại đây",
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
                                        if pm.default_logo_url.is_empty() {
                                            pm.default_logo_url = pm.logo_url.clone();
                                        }
                                        pm.hud_enabled = false;
                                        pm.bot_enabled = false;
                                        pm.installed = true;
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
                                        status.set(Some((false, "Đã cài đặt Plugin.".into())));
                                    }
                                    Err(_) => status.set(Some((true, "JSON không hợp lệ".into()))),
                                }
                            },
                            "Cài đặt"
                        }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| import_open.set(false), "Hủy" }
                    }
                }
            }
        };
    }

    // 3. Plugin Store & Installed List (ZERO Bot / Trang / Giao diện / HUD pills, ZERO description bloat)
    if editing_mod.is_none() {
        let installed_plugins: Vec<PluginMod> = plugins_list.iter().filter(|p| p.installed).cloned().collect();
        let store_plugins: Vec<PluginMod> = plugins_list.iter().filter(|p| !p.installed).cloned().collect();

        let render_card = move |pm: PluginMod| {
            let pid_toggle = pm.id.clone();
            let pid_install = pm.id.clone();
            let pid_uninstall = pm.id.clone();
            let pid_edit = pm.id.clone();
            let pid_del = pm.id.clone();
            let srv_id_del = pm.server_bot_id.clone();
            let pm_open = pm.clone();
            let is_installed = pm.installed;
            let is_on = pm.installed && pm.enabled;
            let has_open = plugin_has_workspace(&pm);
            rsx! {
                div { key: "{pm.id}", class: if is_on { "mod-card active" } else { "mod-card" },
                    div { class: "mod-card-head",
                        span { class: "mod-card-icon", PluginGlyph { icon: pm.icon.clone(), logo_url: pm.logo_url.clone(), size: glyph_slot_size(&pm.logo_url, 20, 38) } }
                        div { class: "mod-card-title-wrap",
                            div { class: "mod-card-title", "{pm.display_name}" }
                            div { class: "member-sub", "v{pm.version} · {pm.author}" }
                        }
                        if is_installed {
                            input {
                                r#type: "checkbox",
                                class: "switch",
                                checked: is_on,
                                aria_label: "Bật {pm.display_name}",
                                onchange: move |e| {
                                    let v = e.checked();
                                    if v { play_mod_sound("xp"); }
                                    update_plugin_by_id(&pid_toggle, &|p| p.enabled = v);
                                }
                            }
                        }
                    }
                    div { class: "mod-card-actions",
                        if is_installed {
                            if has_open {
                                button {
                                    r#type: "button",
                                    class: "btn-small-primary",
                                    onclick: move |_| open_plugin_surface(&pm_open),
                                    "Mở"
                                }
                            }
                            button {
                                r#type: "button",
                                class: "btn-small-secondary",
                                onclick: move |_| {
                                    status.set(None);
                                    logo_editor_open.set(false);
                                    *EDITING_PLUGIN_ID.write() = Some(pid_edit.clone());
                                },
                                SlidersIcon { size: 14 } "Tùy chỉnh"
                            }
                            button {
                                r#type: "button",
                                class: "btn-small-secondary",
                                onclick: move |_| {
                                    status.set(None);
                                    if ACTIVE_LEFT_PLUGIN.peek().as_deref() == Some(pid_uninstall.as_str()) {
                                        *ACTIVE_LEFT_PLUGIN.write() = None;
                                    }
                                    if ACTIVE_PLUGIN_PAGE.peek().as_deref() == Some(pid_uninstall.as_str()) {
                                        *ACTIVE_PLUGIN_PAGE.write() = None;
                                    }
                                    if EDITING_PLUGIN_ID.peek().as_deref() == Some(pid_uninstall.as_str()) {
                                        *EDITING_PLUGIN_ID.write() = None;
                                    }
                                    update_plugin_by_id(&pid_uninstall, &|p| { p.installed = false; p.enabled = false; });
                                },
                                TrashIcon { size: 14 } "Gỡ"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "btn-small-primary",
                                onclick: move |_| {
                                    status.set(None);
                                    play_mod_sound("xp");
                                    update_plugin_by_id(&pid_install, &|p| { p.installed = true; p.enabled = true; });
                                },
                                DownloadIcon { size: 14 } "Cài đặt"
                            }
                            if !pm.builtin {
                                button {
                                    r#type: "button",
                                    class: "icon-btn danger",
                                    title: "Xóa",
                                    onclick: move |_| {
                                        let id = pid_del.clone();
                                        let sid = srv_id_del.clone();
                                        let mut list = PLUGINS.peek().clone();
                                        list.retain(|x| x.id != id);
                                        save_plugins(&list);
                                        *PLUGINS.write() = list;
                                        if let Some(s) = sid {
                                            spawn(async move { let _ = social::bot_delete(&s).await; });
                                        }
                                    },
                                    TrashIcon { size: 15 }
                                }
                            }
                        }
                    }
                }
            }
        };

        return rsx! {
            div { class: "settings-section",
                div { class: "mod-studio-header",
                    div { class: "settings-group-title", "Plugin" }
                    div { class: "inline-form",
                        button { r#type: "button", class: "btn-primary", onclick: move |_| { creating_new.set(true); import_open.set(false); },
                            PlusIcon { size: 16 } "Tạo mới"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| { import_open.set(true); creating_new.set(false); },
                            "Nhập JSON"
                        }
                    }
                }

                if let Some((is_err, msg)) = status() {
                    div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
                }

                div { class: "mod-section-heading", "Đã cài đặt ({installed_plugins.len()})" }
                if installed_plugins.is_empty() {
                    div { class: "mod-store-empty", "Chưa cài đặt Plugin nào." }
                } else {
                    div { class: "mod-cards-grid",
                        for pm in installed_plugins {
                            {render_card(pm)}
                        }
                    }
                }

                if !store_plugins.is_empty() {
                    div { class: "mod-section-heading", "Khám phá ({store_plugins.len()})" }
                    div { class: "mod-cards-grid",
                        for pm in store_plugins {
                            {render_card(pm)}
                        }
                    }
                }
            }
        };
    }

    // 4. Dedicated Per-Plugin Settings & Layout Customizer
    let em = editing_mod.unwrap();
    let eid = em.id.clone();
    let em_open = em.clone();
    let has_open = plugin_has_workspace(&em);
    let def_logo = if em.default_logo_url.is_empty() { em.logo_url.clone() } else { em.default_logo_url.clone() };

    rsx! {
        div { class: "settings-section", "data-plugin-editor-kind": "{em.page_kind}",
            div { class: "mod-editor-top",
                div { class: "mod-editor-title",
                    button {
                        r#type: "button",
                        class: "btn-ghost",
                        onclick: move |_| {
                            logo_editor_open.set(false);
                            *EDITING_PLUGIN_ID.write() = None;
                        },
                        BackIcon {} "Quay lại"
                    }
                    span { class: "mod-card-icon", PluginGlyph { icon: em.icon.clone(), logo_url: em.logo_url.clone(), size: glyph_slot_size(&em.logo_url, 20, 38) } }
                    div { class: "settings-group-title", "{em.display_name}" }
                    if em.allow_custom_logo {
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            style: "font-size: 0.8125rem;",
                            onclick: move |_| logo_editor_open.set(!logo_editor_open()),
                            "Đổi logo"
                        }
                    }
                }
                div { class: "inline-form",
                    if has_open {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| open_plugin_surface(&em_open),
                            "Mở"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: {
                            let em_json = serde_json::to_string_pretty(&em).unwrap_or_default();
                            move |_| {
                                platform::copy_to_clipboard(&em_json);
                                status.set(Some((false, "Đã sao chép JSON.".into())));
                            }
                        },
                        CopyIcon {} "Xuất JSON"
                    }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: em.enabled,
                        aria_label: "Bật {em.display_name}",
                        onchange: {
                            let id = eid.clone();
                            move |e| {
                                let v = e.checked();
                                update_plugin_by_id(&id, &|p| p.enabled = v);
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| {
                            logo_editor_open.set(false);
                            *EDITING_PLUGIN_ID.write() = None;
                        },
                        CheckIcon { size: 16 } "Xong"
                    }
                }
            }

            if em.allow_custom_logo && logo_editor_open() {
                div { class: "plugin-cfg-card plugin-logo-inline",
                    PluginGlyph { icon: em.icon.clone(), logo_url: em.logo_url.clone(), size: 36 }
                    input {
                        r#type: "text",
                        placeholder: "URL ảnh hoặc mã <svg>",
                        value: "{em.logo_url}",
                        oninput: {
                            let id = eid.clone();
                            move |e| {
                                let v = e.value();
                                update_plugin_by_id(&id, &|p| p.logo_url = v.clone());
                            }
                        }
                    }
                    label { class: "btn-avatar-upload",
                        input {
                            id: "plugin-inline-logo-file",
                            r#type: "file",
                            accept: "image/png,image/jpeg,image/webp,image/svg+xml,image/avif,image/gif",
                            style: "display: none;",
                            onchange: {
                                let id = eid.clone();
                                move |_| {
                                    let id = id.clone();
                                    spawn(async move {
                                        if let Ok((bytes, mime)) = platform::read_selected_file_raw("plugin-inline-logo-file").await {
                                            if let Ok(u) = library::upload_media(bytes, &mime).await {
                                                update_plugin_by_id(&id, &|p| p.logo_url = u.clone());
                                                status.set(Some((false, "Đã cập nhật logo.".into())));
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        "Tải ảnh"
                    }
                    if !def_logo.is_empty() && em.logo_url != def_logo {
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            onclick: {
                                let id = eid.clone();
                                let orig = def_logo.clone();
                                move |_| update_plugin_by_id(&id, &|p| p.logo_url = orig.clone())
                            },
                            "Mặc định"
                        }
                    }
                }
            }

            if let Some((is_err, msg)) = status() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
            }

            // Dispatch to EACH plugin's own dedicated, unique UI
            match em.page_kind.as_str() {
                "notes" => rsx! { NotesPluginSettings { plugin: em.clone() } },
                "soundboard" => rsx! { SoundboardPluginSettings { plugin: em.clone() } },
                "voxel" => rsx! { VoxelPluginSettings { plugin: em.clone() } },
                "theme_sakura" => rsx! { SakuraPluginSettings { plugin: em.clone() } },
                "cyber" => rsx! { CyberPluginSettings { plugin: em.clone() } },
                "tools" => rsx! { ToolsPluginSettings { plugin: em.clone(), my_username: my_username.clone() } },
                _ => rsx! { CustomPluginSettings { plugin: em.clone() } },
            }
        }
    }
}

/// 1. Sổ tay & Việc làm — Dedicated Settings & Reorderable Layout Panel
#[component]
fn NotesPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let focus_mins = plugin.cfg_i64("focus_minutes", 25);
    let break_mins = plugin.cfg_i64("break_minutes", 5);
    let sound_alert = plugin.cfg_bool("sound_alert", true);
    let layout_mode = plugin.cfg_str("layout_mode", "two_col");
    let open_slot = plugin.cfg_str("open_slot", "sidebar");
    let defaults = ["pomodoro", "todos", "notes"];
    let ordered = plugin.ordered_sections(&defaults);

    let todo_count = platform::storage_get(TODOS_KEY)
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
        .map(|v| v.len())
        .unwrap_or(0);
    let note_chars = platform::storage_get(NOTES_KEY).map(|s| s.chars().count()).unwrap_or(0);
    let mut cleared_msg = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Sắp xếp khu vực Sổ tay" }
                SectionOrderControl {
                    plugin_id: pid.clone(),
                    items: vec![
                        ("pomodoro".into(), "Đồng hồ Pomodoro".into()),
                        ("todos".into(), "Danh sách việc cần làm".into()),
                        ("notes".into(), "Ghi chú nhanh".into()),
                    ],
                    ordered_keys: ordered,
                    hidden_keys: plugin.hidden_sections.clone(),
                    default_keys: defaults.iter().map(|s| (*s).to_string()).collect(),
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Kiểu bố cục" }
                    div { class: "segmented",
                        button {
                            r#type: "button",
                            class: if layout_mode == "two_col" { "active" } else { "" },
                            onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("layout_mode", json!("two_col"))) },
                            "2 cột song song"
                        }
                        button {
                            r#type: "button",
                            class: if layout_mode == "stacked" { "active" } else { "" },
                            onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("layout_mode", json!("stacked"))) },
                            "Xếp dọc"
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Vị trí mở mặc định" }
                    div { class: "segmented",
                        button {
                            r#type: "button",
                            class: if open_slot == "sidebar" { "active" } else { "" },
                            onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.left_app_enabled = true; p.set_cfg("open_slot", json!("sidebar")); }) },
                            "Cột bên trái"
                        }
                        button {
                            r#type: "button",
                            class: if open_slot == "main" { "active" } else { "" },
                            onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.custom_page_enabled = true; p.set_cfg("open_slot", json!("main")); }) },
                            "Cửa sổ chính"
                        }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Cấu hình Pomodoro" }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Phiên tập trung ({focus_mins} phút)" }
                    div { class: "mod-preset-chips",
                        for m in [15_i64, 25, 45, 60] {
                            button {
                                key: "fm-{m}",
                                r#type: "button",
                                class: if focus_mins == m { "btn-small-primary" } else { "btn-small-secondary" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("focus_minutes", json!(m))) },
                                "{m} phút"
                            }
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Thời gian nghỉ ({break_mins} phút)" }
                    div { class: "mod-preset-chips",
                        for m in [5_i64, 10, 15] {
                            button {
                                key: "bm-{m}",
                                r#type: "button",
                                class: if break_mins == m { "btn-small-primary" } else { "btn-small-secondary" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("break_minutes", json!(m))) },
                                "{m} phút"
                            }
                        }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Chuông báo khi hết phiên" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: sound_alert,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg("sound_alert", json!(v))); } }
                    }
                }
                div { class: "plugin-stats-strip",
                    span { "{todo_count} việc" }
                    span { "·" }
                    span { "{note_chars} ký tự ghi chú" }
                    button {
                        r#type: "button",
                        class: "btn-ghost",
                        style: "margin-left: auto; font-size: 0.8125rem;",
                        onclick: move |_| {
                            if let Some(raw) = platform::storage_get(TODOS_KEY) {
                                if let Ok(mut items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
                                    items.retain(|it| !it.get("done").and_then(|v| v.as_bool()).unwrap_or(false));
                                    if let Ok(s) = serde_json::to_string(&items) {
                                        platform::storage_set(TODOS_KEY, &s);
                                        cleared_msg.set(Some("Đã dọn các việc đã hoàn thành.".into()));
                                    }
                                }
                            }
                        },
                        "Dọn việc đã xong"
                    }
                }
                if let Some(m) = cleared_msg() {
                    div { class: "member-sub", "{m}" }
                }
            }
        }
    }
}

/// 2. Soundboard 8-Bit — Dedicated Audio Mixer & Reorderable Pad Controller
#[component]
fn SoundboardPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let volume = plugin.cfg_i64("volume", 75).clamp(0, 100);
    let pitch = plugin.cfg_i64("pitch", 100).clamp(50, 180);
    let cols = plugin.cfg_i64("columns", 4).clamp(2, 4);
    let vol_f = volume as f64 / 100.0;
    let pitch_f = pitch as f64 / 100.0;
    let default_keys: Vec<String> = SOUNDBOARD_PADS.iter().map(|(k, _, _)| (*k).to_string()).collect();
    let default_refs: Vec<&str> = SOUNDBOARD_PADS.iter().map(|(k, _, _)| *k).collect();
    let ordered = plugin.ordered_sections(&default_refs);
    let total = ordered.len();

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Bộ trộn âm thanh" }
                div { class: "plugin-slider-row",
                    div { class: "plugin-slider-head",
                        span { class: "settings-label", "Âm lượng tổng" }
                        span { class: "member-sub mono", "{volume}%" }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "100",
                        value: "{volume}",
                        oninput: {
                            let id = pid.clone();
                            move |e| {
                                if let Ok(v) = e.value().parse::<i64>() {
                                    update_plugin_by_id(&id, &|p| p.set_cfg("volume", json!(v)));
                                }
                            }
                        }
                    }
                }
                div { class: "plugin-slider-row",
                    div { class: "plugin-slider-head",
                        span { class: "settings-label", "Cao độ âm (Pitch)" }
                        span { class: "member-sub mono", "{pitch}%" }
                    }
                    input {
                        r#type: "range",
                        min: "50",
                        max: "180",
                        value: "{pitch}",
                        oninput: {
                            let id = pid.clone();
                            move |e| {
                                if let Ok(v) = e.value().parse::<i64>() {
                                    update_plugin_by_id(&id, &|p| p.set_cfg("pitch", json!(v)));
                                }
                            }
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Âm thanh khi gõ phím" }
                    div { class: "segmented",
                        for (val, lbl) in [("none", "Tắt"), ("mech", "Phím cơ"), ("minecraft", "Retro"), ("scifi", "Sci-Fi")] {
                            button {
                                key: "ts-{val}",
                                r#type: "button",
                                class: if plugin.sound_fx == val { "active" } else { "" },
                                onclick: {
                                    let id = pid.clone();
                                    move |_| {
                                        if val != "none" { play_mod_sound_opts(val, vol_f, pitch_f); }
                                        update_plugin_by_id(&id, &|p| p.sound_fx = val.into());
                                    }
                                },
                                "{lbl}"
                            }
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Số cột bàn phím" }
                    div { class: "segmented",
                        for c in [2_i64, 3, 4] {
                            button {
                                key: "sc-{c}",
                                r#type: "button",
                                class: if cols == c { "active" } else { "" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("columns", json!(c))) },
                                "{c} cột"
                            }
                        }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Sắp xếp & Nghe thử từng phím" }
                div { class: "plugin-order-list",
                    for (idx, pad_key) in ordered.iter().enumerate() {
                        if let Some((kind, label, color)) = SOUNDBOARD_PADS.iter().find(|(k, _, _)| *k == pad_key.as_str()).copied() {
                            {
                                let is_visible = !plugin.is_section_hidden(kind);
                                let pid_up = pid.clone();
                                let pid_dn = pid.clone();
                                let pid_tg = pid.clone();
                                let defs_up = default_keys.clone();
                                let defs_dn = default_keys.clone();
                                rsx! {
                                    div { key: "pad-{kind}", class: if is_visible { "plugin-order-row" } else { "plugin-order-row dimmed" },
                                        span { class: "voxel-swatch", style: "background: {color};" }
                                        span { class: "plugin-order-label grow", "{label}" }
                                        button {
                                            r#type: "button",
                                            class: "btn-ghost",
                                            style: "padding: 2px 8px; font-size: 0.78rem;",
                                            onclick: move |_| play_mod_sound_opts(kind, vol_f, pitch_f),
                                            "Nghe"
                                        }
                                        div { class: "plugin-order-btns",
                                            button {
                                                r#type: "button",
                                                class: "plugin-step-btn",
                                                disabled: idx == 0,
                                                onclick: move |_| {
                                                    let refs: Vec<&str> = defs_up.iter().map(String::as_str).collect();
                                                    update_plugin_by_id(&pid_up, &|p| p.move_section(kind, -1, &refs));
                                                },
                                                "↑"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "plugin-step-btn",
                                                disabled: idx + 1 >= total,
                                                onclick: move |_| {
                                                    let refs: Vec<&str> = defs_dn.iter().map(String::as_str).collect();
                                                    update_plugin_by_id(&pid_dn, &|p| p.move_section(kind, 1, &refs));
                                                },
                                                "↓"
                                            }
                                        }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: is_visible,
                                            onchange: move |_| update_plugin_by_id(&pid_tg, &|p| p.toggle_section(kind)),
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

/// 3. Bảng vẽ Voxel — Dedicated Canvas & Palette Config (Plugin owns its brand logo)
#[component]
fn VoxelPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let default_col = plugin.cfg_str("default_color", "#10B981");
    let show_grid = plugin.cfg_bool("show_grid", true);
    let bevel_blocks = plugin.cfg_bool("bevel_blocks", true);
    let brush_sound = plugin.cfg_bool("brush_sound", true);
    let defaults = ["actions", "palette", "canvas"];
    let ordered = plugin.ordered_sections(&defaults);
    let mut canvas_msg = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Cọ vẽ & Hiển thị lưới" }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Màu khối mặc định" }
                    div { class: "plugin-swatch-strip",
                        for (name, hex) in VOXEL_PALETTE {
                            button {
                                key: "vc-{name}",
                                r#type: "button",
                                class: if default_col == hex { "plugin-swatch-btn active" } else { "plugin-swatch-btn" },
                                style: "background: {hex};",
                                title: "{name}",
                                aria_label: "{name}",
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| p.set_cfg("default_color", json!(hex))) },
                            }
                        }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Hiển thị đường lưới ô vuông" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: show_grid,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg("show_grid", json!(v))); } }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Khối viền nổi 3D" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: bevel_blocks,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg("bevel_blocks", json!(v))); } }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Âm thanh khi đặt khối" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: brush_sound,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg("brush_sound", json!(v))); } }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Sắp xếp khu vực bảng vẽ" }
                SectionOrderControl {
                    plugin_id: pid.clone(),
                    items: vec![
                        ("actions".into(), "Thanh thao tác nhanh".into()),
                        ("palette".into(), "Bảng chọn màu khối".into()),
                        ("canvas".into(), "Lưới vẽ Voxel 2D".into()),
                    ],
                    ordered_keys: ordered,
                    hidden_keys: plugin.hidden_sections.clone(),
                    default_keys: defaults.iter().map(|s| (*s).to_string()).collect(),
                }
                div { class: "inline-form",
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: move |_| {
                            let mut g = vec![String::new(); 192];
                            for x in 0..16 {
                                g[9 * 16 + x] = "#22C55E".into();
                                g[10 * 16 + x] = "#92400E".into();
                                g[11 * 16 + x] = "#64748B".into();
                            }
                            if let Ok(s) = serde_json::to_string(&g) {
                                platform::storage_set(VOXEL_KEY, &s);
                                canvas_msg.set(Some("Đã khôi phục địa hình mẫu.".into()));
                            }
                        },
                        "Địa hình mẫu"
                    }
                    button {
                        r#type: "button",
                        class: "btn-ghost",
                        onclick: move |_| {
                            let g = vec![String::new(); 192];
                            if let Ok(s) = serde_json::to_string(&g) {
                                platform::storage_set(VOXEL_KEY, &s);
                                canvas_msg.set(Some("Đã xóa trắng bản vẽ.".into()));
                            }
                        },
                        "Xóa trắng"
                    }
                }
                if let Some(m) = canvas_msg() {
                    div { class: "member-sub", "{m}" }
                }
            }
        }
    }
}

/// 4. Sakura Kính mờ — Dedicated Visual Appearance Plugin Settings
#[component]
fn SakuraPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let t = &plugin.theme;
    let palettes = [
        ("Anh đào sáng", "#FDF2F8", "#FCE7F3", "#FFF5F9", "#FFFFFF", "#EC4899", "#DB2777", "#FCE7F3", "#4A152B"),
        ("Phấn hồng kem", "#FFF8F6", "#FDECE8", "#FFFAF8", "#FFFFFF", "#F43F5E", "#E11D48", "#FFE4E6", "#4C1D24"),
        ("Đêm anh đào", "#1A101B", "#241626", "#1F1221", "#2B1B2E", "#F472B6", "#DB2777", "#36223A", "#FCE7F3"),
    ];

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Tông màu Sakura" }
                div { class: "mod-preset-chips",
                    for (name, bg_a, bg_s, bg_c, bg_sf, acc, sent, recv, txt) in palettes {
                        button {
                            key: "sk-{name}",
                            r#type: "button",
                            class: if t.bg_app == bg_a && t.accent == acc { "btn-small-primary" } else { "btn-small-secondary" },
                            onclick: {
                                let id = pid.clone();
                                move |_| update_plugin_by_id(&id, &|p| {
                                    p.theme_enabled = true;
                                    p.theme.bg_app = bg_a.into();
                                    p.theme.bg_sidebar = bg_s.into();
                                    p.theme.bg_chat = bg_c.into();
                                    p.theme.bg_surface = bg_sf.into();
                                    p.theme.accent = acc.into();
                                    p.theme.msg_sent = sent.into();
                                    p.theme.msg_recv = recv.into();
                                    p.theme.text_primary = txt.into();
                                })
                            },
                            span { class: "voxel-swatch", style: "background: {acc};" }
                            "{name}"
                        }
                    }
                }
                div { class: "settings-row",
                    span { class: "grow", span { class: "settings-label", "Màu nhấn chính" } }
                    input {
                        r#type: "color",
                        value: if t.accent.is_empty() { "#EC4899" } else { t.accent.as_str() },
                        onchange: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.accent = v.clone(); }); } }
                    }
                }
                div { class: "plugin-slider-row",
                    div { class: "plugin-slider-head",
                        span { class: "settings-label", "Độ mờ kính" }
                        span { class: "member-sub mono", "{t.glass_blur}px" }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "24",
                        value: "{t.glass_blur}",
                        oninput: {
                            let id = pid.clone();
                            move |e| {
                                if let Ok(v) = e.value().parse::<u8>() {
                                    update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.glass_blur = v; p.theme.border_style = if v > 0 { "glass".into() } else { "default".into() }; });
                                }
                            }
                        }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Hiệu ứng cánh hoa rơi" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: plugin.screen_fx == "sakura",
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.screen_fx = if v { "sakura".into() } else { "none".into() }); } }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Khung & Chữ" }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Độ bo góc" }
                    div { class: "segmented",
                        for (val, lbl) in [("8px", "Gọn"), ("14px", "Vừa"), ("20px", "Mềm"), ("24px", "Tròn")] {
                            button {
                                key: "sr-{val}",
                                r#type: "button",
                                class: if t.panel_radius == val { "active" } else { "" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.panel_radius = val.into(); }) },
                                "{lbl}"
                            }
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Phông chữ" }
                    div { class: "segmented",
                        for (val, lbl) in [("default", "Hệ thống"), ("rounded", "Bo tròn"), ("serif", "Có chân")] {
                            button {
                                key: "sf-{val}",
                                r#type: "button",
                                class: if t.font_style == val { "active" } else { "" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.font_style = val.into(); }) },
                                "{lbl}"
                            }
                        }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Vị trí thanh điều hướng" }
                    div { class: "segmented",
                        for (val, lbl) in [("left", "Trái"), ("right", "Phải"), ("top", "Trên")] {
                            button {
                                key: "sp-{val}",
                                r#type: "button",
                                class: if t.rail_position == val { "active" } else { "" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.rail_position = val.into(); }) },
                                "{lbl}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 5. Cyber Neon — Dedicated Neon Theme & Console Config
#[component]
fn CyberPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let t = &plugin.theme;
    let defaults = ["stats", "console"];
    let ordered = plugin.ordered_sections(&defaults);
    let cmd_sound = plugin.cfg_bool("cmd_sound", true);
    let neon_packs = [
        ("Cyan Điện tử", "#070812", "#0D1021", "#0A0C1A", "#141830", "#00F0FF", "#D900FF", "#E0FAFF"),
        ("Tím Synthwave", "#11081E", "#190D2B", "#150A24", "#23123A", "#FF4FD8", "#7C3AED", "#F5E8FF"),
        ("Xanh Matrix", "#050E08", "#0A1A10", "#07140B", "#0F2617", "#22C55E", "#16A34A", "#DCFCE7"),
    ];

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Ánh sáng Neon" }
                div { class: "mod-preset-chips",
                    for (name, bg_a, bg_s, bg_c, bg_sf, acc, sent, txt) in neon_packs {
                        button {
                            key: "cy-{name}",
                            r#type: "button",
                            class: if t.accent == acc { "btn-small-primary" } else { "btn-small-secondary" },
                            onclick: {
                                let id = pid.clone();
                                move |_| update_plugin_by_id(&id, &|p| {
                                    p.theme_enabled = true;
                                    p.theme.bg_app = bg_a.into();
                                    p.theme.bg_sidebar = bg_s.into();
                                    p.theme.bg_chat = bg_c.into();
                                    p.theme.bg_surface = bg_sf.into();
                                    p.theme.accent = acc.into();
                                    p.theme.msg_sent = sent.into();
                                    p.theme.text_primary = txt.into();
                                })
                            },
                            span { class: "voxel-swatch", style: "background: {acc};" }
                            "{name}"
                        }
                    }
                }
                div { class: "settings-row",
                    span { class: "grow", span { class: "settings-label", "Màu viền phát sáng" } }
                    input {
                        r#type: "color",
                        value: if t.accent.is_empty() { "#00F0FF" } else { t.accent.as_str() },
                        onchange: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.accent = v.clone(); }); } }
                    }
                }
                div { class: "settings-row",
                    span { class: "grow", span { class: "settings-label", "Màu bong bóng gửi" } }
                    input {
                        r#type: "color",
                        value: if t.msg_sent.is_empty() { "#D900FF" } else { t.msg_sent.as_str() },
                        onchange: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.msg_sent = v.clone(); }); } }
                    }
                }
                div { class: "plugin-field-row",
                    span { class: "settings-label", "Kiểu khung" }
                    div { class: "segmented",
                        for (val, lbl) in [("neon", "Neon"), ("glass", "Kính mờ"), ("default", "Tối giản")] {
                            button {
                                key: "cb-{val}",
                                r#type: "button",
                                class: if t.border_style == val { "active" } else { "" },
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.border_style = val.into(); }) },
                                "{lbl}"
                            }
                        }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Hiệu ứng mưa mã Matrix" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: plugin.screen_fx == "matrix",
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.screen_fx = if v { "matrix".into() } else { "none".into() }); } }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Bố cục Trạm lệnh" }
                SectionOrderControl {
                    plugin_id: pid.clone(),
                    items: vec![
                        ("stats".into(), "Thẻ thông số kết nối".into()),
                        ("console".into(), "Bảng chạy lệnh nhanh".into()),
                    ],
                    ordered_keys: ordered,
                    hidden_keys: plugin.hidden_sections.clone(),
                    default_keys: defaults.iter().map(|s| (*s).to_string()).collect(),
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Âm thanh phản hồi Sci-Fi" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: cmd_sound,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg("cmd_sound", json!(v))); } }
                    }
                }
                label { class: "form-field",
                    span { "CSS bổ sung" }
                    textarea {
                        rows: "3",
                        style: "font-family: var(--font-mono); font-size: 12px;",
                        placeholder: "Quy tắc CSS tùy chọn",
                        value: "{t.custom_css}",
                        oninput: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.theme_enabled = true; p.theme.custom_css = v.clone(); }); } }
                    }
                }
            }
        }
    }
}

/// 6. Công cụ Lệnh nhanh — Dedicated Slash Commands & Quick Tools Customizer (NO Bot mode)
#[component]
fn ToolsPluginSettings(plugin: PluginMod, my_username: String) -> Element {
    let pid = plugin.id.clone();
    let defaults = ["calc", "random", "text", "snippets"];
    let ordered = plugin.ordered_sections(&defaults);

    let mut test_input = use_signal(|| "/calc 12*8+4".to_string());
    let mut test_output = use_signal(String::new);
    let mut cmd_trigger = use_signal(String::new);
    let mut cmd_resp = use_signal(String::new);
    let mut btn_label = use_signal(String::new);
    let mut btn_content = use_signal(String::new);
    let cmds_len = plugin.commands.len();

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Sắp xếp tiện ích" }
                SectionOrderControl {
                    plugin_id: pid.clone(),
                    items: vec![
                        ("calc".into(), "Máy tính nhanh".into()),
                        ("random".into(), "Xúc xắc & Ngẫu nhiên".into()),
                        ("text".into(), "Xử lý văn bản".into()),
                        ("snippets".into(), "Lệnh gõ tắt".into()),
                    ],
                    ordered_keys: ordered,
                    hidden_keys: plugin.hidden_sections.clone(),
                    default_keys: defaults.iter().map(|s| (*s).to_string()).collect(),
                }
                div { class: "settings-group-title", style: "margin-top: var(--space-2);", "Chạy thử lệnh" }
                div { class: "inline-form",
                    input {
                        r#type: "text",
                        placeholder: "vd: /calc 24*7 hoặc /roll",
                        value: "{test_input}",
                        oninput: move |e| test_input.set(e.value()),
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: {
                            let uname = my_username.clone();
                            move |_| {
                                let res = try_run_plugin_command(&test_input(), &uname)
                                    .unwrap_or_else(|| "Không khớp lệnh đang bật".into());
                                test_output.set(res);
                            }
                        },
                        "Chạy"
                    }
                }
                if !test_output().is_empty() {
                    div { class: "plugin-calc-result", "{test_output}" }
                }
                div { class: "settings-group-title", style: "margin-top: var(--space-2);", "Nút chèn nhanh khung chat" }
                div { class: "inline-form",
                    input { r#type: "text", placeholder: "Nhãn", value: "{btn_label}", oninput: move |e| btn_label.set(e.value()) }
                    input { r#type: "text", placeholder: "Lệnh hoặc chữ", value: "{btn_content}", oninput: move |e| btn_content.set(e.value()) }
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: {
                            let id = pid.clone();
                            move |_| {
                                let l = btn_label().trim().to_string();
                                let c = btn_content().trim().to_string();
                                if l.is_empty() || c.is_empty() { return; }
                                update_plugin_by_id(&id, &|p| p.composer_buttons.push(PluginComposerBtn { label: l.clone(), content: c.clone() }));
                                btn_label.set(String::new());
                                btn_content.set(String::new());
                            }
                        },
                        PlusIcon { size: 14 }
                    }
                }
                div { class: "mod-preset-chips",
                    for (bi, b) in plugin.composer_buttons.iter().enumerate() {
                        span { key: "cb-{bi}", class: "plugin-cmd-chip",
                            "{b.label} → {b.content}"
                            button {
                                r#type: "button",
                                class: "chip-del",
                                onclick: {
                                    let id = pid.clone();
                                    move |_| update_plugin_by_id(&id, &|p| { if bi < p.composer_buttons.len() { p.composer_buttons.remove(bi); } })
                                },
                                "×"
                            }
                        }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Lệnh gõ tắt (/command)" }
                div { class: "inline-form",
                    input { r#type: "text", placeholder: "/tenlenh", style: "max-width: 8rem;", value: "{cmd_trigger}", oninput: move |e| cmd_trigger.set(e.value()) }
                    input { r#type: "text", placeholder: "Nội dung trả về (hỗ trợ {{user}}, {{time}}, {{calc}})", value: "{cmd_resp}", oninput: move |e| cmd_resp.set(e.value()) }
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: {
                            let id = pid.clone();
                            move |_| {
                                let mut tr = cmd_trigger().trim().to_string();
                                let rp = cmd_resp().trim().to_string();
                                if tr.is_empty() || rp.is_empty() { return; }
                                if !tr.starts_with('/') { tr = format!("/{tr}"); }
                                update_plugin_by_id(&id, &|p| p.commands.push(PluginCommand {
                                    trigger: tr.clone(),
                                    description: tr.clone(),
                                    response: rp.clone(),
                                    icon: String::new(),
                                    enabled: true,
                                }));
                                cmd_trigger.set(String::new());
                                cmd_resp.set(String::new());
                            }
                        },
                        PlusIcon { size: 14 } "Thêm"
                    }
                }
                div { class: "plugin-order-list",
                    for (ci, c) in plugin.commands.iter().enumerate() {
                        div { key: "cmd-{ci}-{c.trigger}", class: if c.enabled { "plugin-order-row" } else { "plugin-order-row dimmed" },
                            code { class: "kbd", "{c.trigger}" }
                            span { class: "member-sub grow", style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{c.response}" }
                            div { class: "plugin-order-btns",
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: ci == 0,
                                    onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if ci > 0 && ci < p.commands.len() { p.commands.swap(ci, ci - 1); } }) },
                                    "↑"
                                }
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: ci + 1 >= cmds_len,
                                    onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if ci + 1 < p.commands.len() { p.commands.swap(ci, ci + 1); } }) },
                                    "↓"
                                }
                            }
                            input {
                                r#type: "checkbox",
                                class: "switch",
                                checked: c.enabled,
                                onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| { if let Some(x) = p.commands.get_mut(ci) { x.enabled = v; } }); } }
                            }
                            button {
                                r#type: "button",
                                class: "icon-btn danger",
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if ci < p.commands.len() { p.commands.remove(ci); } }) },
                                TrashIcon { size: 14 }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 7. Custom / Third-Party Declarative Plugin Settings (Schema-driven settings + Reorderable Custom Blocks)
#[component]
fn CustomPluginSettings(plugin: PluginMod) -> Element {
    let pid = plugin.id.clone();
    let mut blk_kind = use_signal(|| "card".to_string());
    let mut blk_title = use_signal(String::new);
    let mut blk_body = use_signal(String::new);
    let mut blk_action_type = use_signal(|| "insert_chat".to_string());
    let mut blk_action_label = use_signal(String::new);
    let mut blk_action_cmd = use_signal(String::new);
    let block_count = plugin.custom_blocks.len();

    rsx! {
        div { class: "plugin-cfg-grid",
            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Tùy chỉnh riêng của Plugin" }
                label { class: "form-field",
                    span { "Tên hiển thị" }
                    input {
                        r#type: "text",
                        value: "{plugin.display_name}",
                        oninput: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.display_name = v.clone(); p.page_title = v.clone(); }); } }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Cho phép người dùng đổi Logo" } }
                    input {
                        r#type: "checkbox",
                        class: "switch",
                        checked: plugin.allow_custom_logo,
                        onchange: { let id = pid.clone(); move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.allow_custom_logo = v); } }
                    }
                }
                // Render declarative custom_settings schema defined by this plugin
                for def in plugin.custom_settings.iter() {
                    {
                        let skey = def.key.clone();
                        let slbl = def.label.clone();
                        match def.kind.as_str() {
                            "toggle" => {
                                let def_b = def.default_value == "true";
                                let cur = plugin.cfg_bool(&skey, def_b);
                                let id = pid.clone();
                                rsx! {
                                    label { key: "cs-{skey}", class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "{slbl}" } }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: cur,
                                            onchange: move |e| { let v = e.checked(); update_plugin_by_id(&id, &|p| p.set_cfg(&skey, json!(v))); }
                                        }
                                    }
                                }
                            }
                            "slider" => {
                                let def_i = def.default_value.parse::<i64>().unwrap_or(def.min as i64);
                                let cur = plugin.cfg_i64(&skey, def_i);
                                let id = pid.clone();
                                rsx! {
                                    div { key: "cs-{skey}", class: "plugin-slider-row",
                                        div { class: "plugin-slider-head",
                                            span { class: "settings-label", "{slbl}" }
                                            span { class: "member-sub mono", "{cur}" }
                                        }
                                        input {
                                            r#type: "range",
                                            min: "{def.min}",
                                            max: "{def.max}",
                                            value: "{cur}",
                                            oninput: move |e| {
                                                if let Ok(v) = e.value().parse::<i64>() {
                                                    update_plugin_by_id(&id, &|p| p.set_cfg(&skey, json!(v)));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "select" => {
                                let cur = plugin.cfg_str(&skey, &def.default_value);
                                let id = pid.clone();
                                rsx! {
                                    label { key: "cs-{skey}", class: "form-field",
                                        span { "{slbl}" }
                                        select {
                                            value: "{cur}",
                                            onchange: move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| p.set_cfg(&skey, json!(v))); },
                                            for opt in def.options.iter() {
                                                option { key: "{opt}", value: "{opt}", "{opt}" }
                                            }
                                        }
                                    }
                                }
                            }
                            "color" => {
                                let cur = plugin.cfg_str(&skey, if def.default_value.is_empty() { "#2563EB" } else { &def.default_value });
                                let id = pid.clone();
                                rsx! {
                                    div { key: "cs-{skey}", class: "settings-row",
                                        span { class: "grow", span { class: "settings-label", "{slbl}" } }
                                        input {
                                            r#type: "color",
                                            value: "{cur}",
                                            onchange: move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| p.set_cfg(&skey, json!(v))); }
                                        }
                                    }
                                }
                            }
                            _ => {
                                let cur = plugin.cfg_str(&skey, &def.default_value);
                                let id = pid.clone();
                                rsx! {
                                    label { key: "cs-{skey}", class: "form-field",
                                        span { "{slbl}" }
                                        input {
                                            r#type: "text",
                                            value: "{cur}",
                                            oninput: move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| p.set_cfg(&skey, json!(v))); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if plugin.theme_enabled || !plugin.theme.custom_css.is_empty() {
                    label { class: "form-field",
                        span { "CSS tùy chỉnh" }
                        textarea {
                            rows: "4",
                            style: "font-family: var(--font-mono); font-size: 12px;",
                            placeholder: "Nhập CSS tùy chỉnh",
                            value: "{plugin.theme.custom_css}",
                            oninput: { let id = pid.clone(); move |e| { let v = e.value(); update_plugin_by_id(&id, &|p| { p.theme_enabled = !v.trim().is_empty(); p.theme.custom_css = v.clone(); }); } }
                        }
                    }
                }
            }

            div { class: "plugin-cfg-card",
                div { class: "settings-group-title", "Khối giao diện & Thứ tự hiển thị" }
                div { class: "form-grid",
                    label { class: "form-field",
                        span { "Loại khối" }
                        select { value: "{blk_kind}", onchange: move |e| blk_kind.set(e.value()),
                            option { value: "card", "Thẻ thông tin" }
                            option { value: "button", "Nút bấm" }
                            option { value: "counter", "Bộ đếm" }
                            option { value: "checklist", "Danh sách việc" }
                            option { value: "input_action", "Ô nhập & chạy" }
                            option { value: "timer", "Đồng hồ đếm ngược" }
                            option { value: "link", "Liên kết" }
                        }
                    }
                    label { class: "form-field",
                        span { "Tiêu đề khối" }
                        input { r#type: "text", placeholder: "vd: Bộ đếm", value: "{blk_title}", oninput: move |e| blk_title.set(e.value()) }
                    }
                    label { class: "form-field",
                        span { "Hành động" }
                        select { value: "{blk_action_type}", onchange: move |e| blk_action_type.set(e.value()),
                            option { value: "insert_chat", "Chèn vào chat" }
                            option { value: "command", "Chạy lệnh" }
                            option { value: "copy", "Sao chép" }
                            option { value: "sound", "Phát âm thanh" }
                            option { value: "url", "Mở liên kết" }
                        }
                    }
                    label { class: "form-field",
                        span { "Nhãn nút" }
                        input { r#type: "text", placeholder: "vd: Chạy", value: "{blk_action_label}", oninput: move |e| blk_action_label.set(e.value()) }
                    }
                    label { class: "form-field wide",
                        span { "Nội dung / Tham số" }
                        input { r#type: "text", placeholder: "Nội dung, lệnh, thời gian (5m) hoặc URL", value: "{blk_action_cmd}", oninput: move |e| blk_action_cmd.set(e.value()) }
                    }
                }
                button {
                    r#type: "button",
                    class: "btn-secondary",
                    onclick: {
                        let id = pid.clone();
                        move |_| {
                            let t = blk_title().trim().to_string();
                            if t.is_empty() { return; }
                            let blk = PluginCustomBlock {
                                kind: blk_kind(),
                                title: t,
                                body: blk_body().trim().to_string(),
                                action_label: blk_action_label().trim().to_string(),
                                action_command: blk_action_cmd().trim().to_string(),
                                action_type: blk_action_type(),
                            };
                            update_plugin_by_id(&id, &|p| {
                                p.custom_page_enabled = true;
                                p.custom_blocks.push(blk.clone());
                            });
                            blk_title.set(String::new());
                            blk_body.set(String::new());
                            blk_action_label.set(String::new());
                            blk_action_cmd.set(String::new());
                        }
                    },
                    PlusIcon { size: 15 } "Thêm khối"
                }
                div { class: "plugin-order-list",
                    for (bi, b) in plugin.custom_blocks.iter().enumerate() {
                        div { key: "blk-{bi}-{b.title}", class: "plugin-order-row",
                            code { class: "kbd", "{b.kind}" }
                            span { class: "plugin-order-label grow", "{b.title}" }
                            div { class: "plugin-order-btns",
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: bi == 0,
                                    onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if bi > 0 && bi < p.custom_blocks.len() { p.custom_blocks.swap(bi, bi - 1); } }) },
                                    "↑"
                                }
                                button {
                                    r#type: "button",
                                    class: "plugin-step-btn",
                                    disabled: bi + 1 >= block_count,
                                    onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if bi + 1 < p.custom_blocks.len() { p.custom_blocks.swap(bi, bi + 1); } }) },
                                    "↓"
                                }
                            }
                            button {
                                r#type: "button",
                                class: "icon-btn danger",
                                onclick: { let id = pid.clone(); move |_| update_plugin_by_id(&id, &|p| { if bi < p.custom_blocks.len() { p.custom_blocks.remove(bi); } }) },
                                TrashIcon { size: 14 }
                            }
                        }
                    }
                }
            }
        }
    }
}
