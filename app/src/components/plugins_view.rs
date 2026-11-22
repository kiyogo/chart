//! Unified ChatW Plugin Extension Bridge & Safe Legacy Data Migration (Spec 058).
//!
//! Legacy built-in local plugins (`PluginMod`, `chatw_plugins_v2`, and the 6 hardcoded built-ins)
//! have been removed so ChatW runs exclusively on ONE single sandboxed Plugin Platform
//! (`PluginHubPanel` + `PLUGIN_INSTALLATIONS` + `ChatwPluginHost`).
//!
//! Any user-created data left in legacy storage slots (`chatw_plugin_notes_v1*`,
//! `chatw_plugin_todos_v1*`, `chatw_plugin_voxel_v1*`, `chatw_plugins_v2*`) is archived
//! idempotently per account (`chatw_legacy_plugin_archive_v1_{uid}`) so users can export
//! their data without ever losing a single note or todo.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        icons::{
            BoltIcon, BotIcon, BuildingIcon, CalcIcon, CalendarIcon, ClipboardCheckIcon, CoinIcon,
            CompassIcon, DiceIcon, GlobeIcon, HelpCircleIcon, ListChecksIcon, LogInIcon,
            LogOutIcon, MegaphoneIcon, PluginIcon, TypeIcon, VicoAiIcon, VideoIcon,
        },
        plugin_hub::PLUGIN_INSTALLATIONS,
    },
    platform,
};

pub static ACTIVE_PLUGIN_ACCOUNT_ID: GlobalSignal<String> = Signal::global(String::new);
pub static ACTIVE_PLUGIN_PAGE: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static ACTIVE_LEFT_PLUGIN: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static EDITING_PLUGIN_ID: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static PENDING_COMPOSER_TEXT: GlobalSignal<Option<String>> = Signal::global(|| None);

fn legacy_archive_key(uid: &str) -> String {
    format!("chatw_legacy_plugin_archive_v1_{}", uid.trim())
}

fn legacy_migrated_flag(uid: &str) -> String {
    format!("chatw_legacy_plugin_migrated_v1_{}", uid.trim())
}

/// Switches the active account namespace for the Plugin Platform and idempotently archives
/// any legacy user-created notes/todos/voxel data without ever executing legacy built-in plugins.
pub fn set_plugin_account_id(uid: &str) {
    let clean = uid.trim().to_string();
    if *ACTIVE_PLUGIN_ACCOUNT_ID.peek() == clean {
        return;
    }
    *ACTIVE_PLUGIN_ACCOUNT_ID.write() = clean.clone();
    if !clean.is_empty() {
        migrate_legacy_plugin_data_once(&clean);
    }
    cleanup_legacy_plugin_dom_artifacts();
}

/// Idempotently archives legacy plugin user data into `chatw_legacy_plugin_archive_v1_{uid}`
/// so zero user data is ever lost after removing the legacy built-in plugin engine.
pub fn migrate_legacy_plugin_data_once(uid: &str) {
    let clean = uid.trim();
    if clean.is_empty() {
        return;
    }
    let flag_key = legacy_migrated_flag(clean);
    if platform::storage_get(&flag_key).as_deref() == Some("1") {
        return;
    }

    let read_pair = |base: &str| -> Option<serde_json::Value> {
        let scoped = format!("{base}_{clean}");
        let raw = platform::storage_get(&scoped)
            .filter(|s| !s.trim().is_empty())
            .or_else(|| platform::storage_get(base).filter(|s| !s.trim().is_empty()))?;
        serde_json::from_str(&raw).ok().or_else(|| Some(json!(raw)))
    };

    let notes = read_pair("chatw_plugin_notes_v1");
    let todos = read_pair("chatw_plugin_todos_v1");
    let voxel = read_pair("chatw_plugin_voxel_v1");
    let raw_plugins = read_pair("chatw_plugins_v2");

    // Only archive custom user-authored plugins or non-empty notes/todos/voxel data.
    let custom_plugins: Option<serde_json::Value> = raw_plugins.and_then(|v| {
        v.as_array().map(|arr| {
            let customs: Vec<serde_json::Value> = arr
                .iter()
                .filter(|item| item.get("builtin").and_then(|b| b.as_bool()) == Some(false))
                .cloned()
                .collect();
            json!(customs)
        })
    });
    let has_custom = custom_plugins
        .as_ref()
        .and_then(|v| v.as_array())
        .is_some_and(|a| !a.is_empty());

    if notes.is_some() || todos.is_some() || voxel.is_some() || has_custom {
        let archive = json!({
            "schema": "chatw.legacy_plugin_archive.v1",
            "user_id": clean,
            "archived_at": chrono::Utc::now().to_rfc3339(),
            "notes": notes,
            "todos": todos,
            "voxel": voxel,
            "custom_plugins": if has_custom { custom_plugins } else { None },
        });
        if let Ok(serialized) = serde_json::to_string_pretty(&archive) {
            platform::storage_set(&legacy_archive_key(clean), &serialized);
        }
    }

    platform::storage_set(&flag_key, "1");
}

/// Returns archived legacy plugin data JSON for `uid` if the user previously created notes/todos/voxel data.
pub fn get_legacy_plugin_archive_json(uid: &str) -> Option<String> {
    let clean = uid.trim();
    if clean.is_empty() {
        return None;
    }
    platform::storage_get(&legacy_archive_key(clean)).filter(|s| !s.trim().is_empty())
}

/// Clear archived legacy plugin export after the user explicitly chooses to dismiss it.
pub fn clear_legacy_plugin_archive(uid: &str) {
    let clean = uid.trim();
    if !clean.is_empty() {
        platform::storage_set(&legacy_archive_key(clean), "");
    }
}

/// Remove any leftover `<style id="chatw-plugin-mods-css">` or legacy DOM theme attributes
/// left over from the deleted legacy plugin system.
fn cleanup_legacy_plugin_dom_artifacts() {
    let _ = dioxus::document::eval(
        r#"
        (() => {
            try {
                const el = document.getElementById('chatw-plugin-mods-css');
                if (el) el.remove();
                document.documentElement.removeAttribute('data-plugin-border');
                document.documentElement.removeAttribute('data-plugin-font');
                document.documentElement.removeAttribute('data-plugin-rail');
                window.__chatwTypingSound = 'none';
            } catch (_) {}
        })();
        "#,
    );
}

/// Installed sandboxed plugins from the single Plugin Platform that declare `ui.sidebar` permission.
pub fn enabled_left_apps() -> Vec<(String, String, String, String)> {
    PLUGIN_INSTALLATIONS
        .read()
        .iter()
        .filter(|inst| {
            inst.enabled
                && inst
                    .grants
                    .iter()
                    .any(|g| g.permission == "ui.sidebar" && g.revoked_at.is_none())
                && inst.manifest.get("ui_contributions").and_then(|c| c.get("sidebar")).is_some()
        })
        .map(|inst| {
            (
                inst.id.clone(),
                String::new(),
                if inst.icon_url.is_empty() {
                    "svg:plugin".to_string()
                } else {
                    inst.icon_url.clone()
                },
                inst.name.clone(),
            )
        })
        .collect()
}

/// Installed sandboxed plugins from the single Plugin Platform that declare `ui.page` permission.
pub fn enabled_rail_pages() -> Vec<(String, String, String, String)> {
    PLUGIN_INSTALLATIONS
        .read()
        .iter()
        .filter(|inst| {
            inst.enabled
                && inst
                    .grants
                    .iter()
                    .any(|g| g.permission == "ui.page" && g.revoked_at.is_none())
                && inst.manifest.get("ui_contributions").and_then(|c| c.get("page")).is_some()
        })
        .map(|inst| {
            (
                inst.id.clone(),
                String::new(),
                if inst.icon_url.is_empty() {
                    "svg:plugin".to_string()
                } else {
                    inst.icon_url.clone()
                },
                inst.name.clone(),
            )
        })
        .collect()
}

pub fn glyph_slot_size(logo_url: &str, default_px: u32, logo_px: u32) -> u32 {
    let u = logo_url.trim();
    if !u.is_empty() && !u.starts_with("svg:") {
        logo_px
    } else {
        default_px
    }
}

#[component]
pub fn PluginGlyph(
    icon: String,
    #[props(default)] logo_url: String,
    #[props(default = 20)] size: u32,
) -> Element {
    let trimmed = logo_url.trim();
    if !trimmed.is_empty() {
        if let Some(svg_kind) = trimmed.strip_prefix("svg:") {
            return match svg_kind {
                "dice" => rsx! { DiceIcon { size } },
                "coin" => rsx! { CoinIcon { size } },
                "calc" => rsx! { CalcIcon { size } },
                "upper" => rsx! { TypeIcon { size } },
                "clock" => rsx! { CalendarIcon { size } },
                "workspace" => rsx! { BuildingIcon { size } },
                "task" => rsx! { ListChecksIcon { size } },
                "checkin" => rsx! { LogInIcon { size } },
                "checkout" => rsx! { LogOutIcon { size } },
                "approve" => rsx! { ClipboardCheckIcon { size } },
                "meeting" => rsx! { VideoIcon { size } },
                "announce" => rsx! { MegaphoneIcon { size } },
                "help" => rsx! { HelpCircleIcon { size } },
                "vico" => rsx! { VicoAiIcon { size } },
                "bot" => rsx! { BotIcon { size } },
                "bolt" => rsx! { BoltIcon { size } },
                "globe" => rsx! { GlobeIcon { size } },
                "compass" => rsx! { CompassIcon { size } },
                _ => rsx! { PluginIcon { size } },
            };
        }
        if (trimmed.starts_with("/uploads/") || trimmed.starts_with("https://"))
            && !trimmed.contains("..")
        {
            let resolved = platform::resolve_asset_url(trimmed);
            return rsx! {
                img {
                    src: "{resolved}",
                    alt: "Plugin",
                    width: "{size}",
                    height: "{size}",
                    style: "border-radius: 8px; object-fit: cover;",
                }
            };
        }
    }
    if !icon.trim().is_empty() {
        rsx! { span { style: "font-size: {size}px; line-height: 1;", "{icon}" } }
    } else {
        rsx! { PluginIcon { size } }
    }
}

pub(crate) fn plugin_error(code: &str) -> &'static str {
    match code {
        "invalid_bot_username" => "Tên định danh bot chỉ gồm chữ cái không dấu, số hoặc gạch dưới từ 2 đến 32 ký tự",
        "username_taken_by_user" => "Tên này đang là tên tài khoản của người khác",
        "bot_username_taken" => "Đã có bot dùng tên này",
        "invalid_display_name" => "Tên hiển thị không hợp lệ",
        "bot_limit_reached" => "Bạn đã đạt số bot máy chủ tối đa",
        "description_too_long" => "Mô tả quá dài",
        "bot_not_public" => "Bot này chưa công khai hoặc chưa được xác minh",
        "insufficient_permissions" => "Chỉ trưởng hoặc phó nhóm được thêm bot",
        _ => "Không thực hiện được, vui lòng thử lại",
    }
}

/// Global workspace lifecycle hook that cleans up any legacy plugin DOM artifacts.
#[component]
pub fn PluginGlobalOverlay() -> Element {
    use_effect(move || {
        cleanup_legacy_plugin_dom_artifacts();
    });
    rsx! {}
}

pub use crate::components::plugin_pages::{
    PluginChatView, PluginCustomPageView, PluginLeftAppView,
};
pub use PluginChatView as BotChatView;
pub use crate::components::plugin_settings::PluginsSettings;
pub use PluginsSettings as BotsSettings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_plugin_user_data_is_archived_idempotently_without_data_loss() {
        let uid = format!("mig_u_{}", uuid::Uuid::new_v4());
        platform::storage_set(&format!("chatw_plugin_notes_v1_{uid}"), "Ghi chú quan trọng của tôi");
        platform::storage_set(
            &format!("chatw_plugin_todos_v1_{uid}"),
            r#"[{"id":"1","text":"Hoàn thành báo cáo","done":false}]"#,
        );

        migrate_legacy_plugin_data_once(&uid);
        let archived = get_legacy_plugin_archive_json(&uid).expect("archive created");
        assert!(archived.contains("Ghi chú quan trọng của tôi"));
        assert!(archived.contains("Hoàn thành báo cáo"));

        // Running again is a no-op and preserves the archive
        migrate_legacy_plugin_data_once(&uid);
        assert_eq!(get_legacy_plugin_archive_json(&uid).as_deref(), Some(archived.as_str()));
    }
}
