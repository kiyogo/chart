use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        details_panel::format_fingerprint,
        guide::GuideSettings,
        settings_extra::{AccountSecurity, CustomizationSettings, LibrarySettings},
        settings_more::{AppearanceColorsShapes, AppearanceWallpapersGlass, AutoReplySettings, ChatPrefsSettings, LanguageSettings, LockSettings, NotificationSettings, PrivacySettingsPanel},
        business_view::BusinessSettings,
        plugins_view::BotsSettings,
        icons::{
            BackIcon, CheckIcon, ChevronRightIcon, KeyIcon, LogoutIcon, MonitorIcon,
            MoonIcon, BookIcon, LibraryIcon, PaletteIcon, ShieldIcon, SlidersIcon, SunIcon, UserIcon,
            BellIcon, BlockIcon, GlobeIcon, LockIcon, MessageIcon, PluginIcon, ReplyIcon, StoreIcon,
        },
    },
    crypto::e2ee::{UserKeys, export_user_keys_json, import_user_keys_json},
    platform,
    services::api,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Account,
    Appearance,
    Customize,
    Stickers,
    Guide,
    Security,
    Session,
    Storage,
    SyncRecovery,
    Backup,
    Cloud,
    Chat,
    Notifications,
    Privacy,
    Lock,
    AutoReply,
    Business,
    Bots,
    Language,
    Demo,
    Studio,
    Inventory,
}

/// (group title, tab, short label). Labels stay on one line; no descriptions.
const NAV: [(&str, SettingsTab, &str); 22] = [
    ("Account", SettingsTab::Account, "Account"),
    ("Account", SettingsTab::Privacy, "Privacy"),
    ("Account", SettingsTab::Security, "Security"),
    ("Account", SettingsTab::Lock, "App lock"),
    ("Account", SettingsTab::Session, "Devices"),
    ("Account", SettingsTab::Storage, "Storage"),
    ("Account", SettingsTab::SyncRecovery, "Sync"),
    ("Account", SettingsTab::Backup, "Backup"),
    ("Account", SettingsTab::Cloud, "Cloud"),
    ("Application", SettingsTab::Chat, "Chat"),
    ("Application", SettingsTab::Notifications, "Notifications"),
    ("Application", SettingsTab::AutoReply, "Auto reply"),
    ("Application", SettingsTab::Appearance, "Appearance"),
    ("Application", SettingsTab::Customize, "Customize"),
    ("Application", SettingsTab::Language, "Language"),
    ("Creative", SettingsTab::Stickers, "Library"),
    ("Creative", SettingsTab::Studio, "Draw"),
    ("Creative", SettingsTab::Inventory, "Inventory"),
    ("Creative", SettingsTab::Demo, "Showcase"),
    ("Other", SettingsTab::Business, "Business"),
    ("Other", SettingsTab::Bots, "Extensions"),
    ("Other", SettingsTab::Guide, "Guide"),
];

fn tab_label(t: SettingsTab) -> &'static str {
    NAV.iter().find(|x| x.1 == t).map(|x| x.2).unwrap_or("")
}

/// Extra search words for each settings page (folded, Vietnamese and English).
fn tab_keywords(t: SettingsTab) -> &'static str {
    match t {
        SettingsTab::Account => "account tai khoan ho so avatar anh dai dien anh bia ten username profile avatar studio hieu ung trang thai danh xung website so thich",
        SettingsTab::Privacy => "privacy quyen rieng tu chan block da xem dang soan ket ban goi",
        SettingsTab::Security => "security bao mat ma hoa khoa sao luu e2ee van tay backup xac minh hai buoc 2fa totp ma du phong nhat ky tai khoan da tai khoan chuyen tai khoan",
        SettingsTab::Lock => "app lock khoa pin ma pin lock",
        SettingsTab::Session => "devices session thiet bi phien dang xuat device",
        SettingsTab::Storage => "storage cache luu tru bo nho dem quota opfs indexeddb s3 minio",
        SettingsTab::SyncRecovery => "sync recovery dong bo khoi phuc da thiet bi backup webrtc",
        SettingsTab::Backup => "backup export import restore sao luu xuat du lieu khoi phuc kdata zip",
        SettingsTab::Cloud => "cloud dam may google drive onedrive dropbox nextcloud webdav s3 minio nas snapshot",
        SettingsTab::Chat => "chat tro chuyen goi y sticker dan excel bang enter tra loi bam dup",
        SettingsTab::Notifications => "notifications notification sound thong bao am thanh am bao",
        SettingsTab::AutoReply => "auto reply autoreply tra loi tu dong vang mat",
        SettingsTab::Appearance => "appearance theme dark light accent wallpaper font giao dien chu de toi sang mau nhan co chu mau bong bong hinh nen phong chu trong suot do mo khung kinh",
        SettingsTab::Customize => "customize tuy chinh ghim cong dong nhom dong bo thiet bi hieu ung avatar danh sach am bao sync",
        SettingsTab::Language => "language locale ngon ngu tieng anh english",
        SettingsTab::Stickers => "library sticker emoji gif upload thu vien tai len",
        SettingsTab::Business => "business kinh doanh ban hang san pham",
        SettingsTab::Bots => "plugins plugin extensions bot bots tien ich mo rong am thanh so tay",
        SettingsTab::Guide => "guide help faq huong dan phim tat cau hoi tro giup",
        SettingsTab::Inventory => "inventory kho vat pham thanh tich achievement huy hieu badge sticker pack emoji gif ban ve trang bi equipped template bo cuc",
        SettingsTab::Studio => "draw studio ve hoat anh animation drawing layer lop khung hinh gif sticker emoji avatar anh bia ve tay",
        SettingsTab::Demo => "preview demo thu hieu ung avatar xoay ten bang ten nameplate hinh nen playground gallery",
    }
}

#[component]
pub fn SettingsView(
    current_user_id: String,
    current_username: String,
    my_avatar_url: Option<String>,
    user_keys: Option<UserKeys>,
    on_close: EventHandler<()>,
    on_open_profile: EventHandler<()>,
    on_avatar_updated: EventHandler<String>,
    on_keys_imported: EventHandler<UserKeys>,
    on_logout: EventHandler<()>,
    #[props(default)] on_username_changed: Option<EventHandler<String>>,
    #[props(default)] on_replay_tour: Option<EventHandler<()>>,
    #[props(default)] on_lock_now: Option<EventHandler<()>>,
) -> Element {
    let on_replay_tour = on_replay_tour.unwrap_or_else(|| EventHandler::new(|_| {}));
    let mut tab = use_signal(|| SettingsTab::Account);
    let mut nav_query = use_signal(String::new);
    let mut has_detail = use_signal(|| false);
    let mut appearance = use_signal(platform::load_appearance);
    let prefs_ctx = use_context::<crate::prefs::PrefsCtx>();
    let mut update_appearance = move |f: &dyn Fn(&mut platform::Appearance)| {
        let mut a = appearance();
        f(&mut a);
        platform::apply_appearance(&a);
        appearance.set(a);
    };
    let mut is_uploading_avatar = use_signal(|| false);
    let mut avatar_error = use_signal(|| Option::<String>::None);
    let mut is_uploading_banner = use_signal(|| false);
    let mut banner_status = use_signal(|| Option::<(bool, String)>::None);

    // E2EE Key backup & restore state
    let mut exported_json = use_signal(|| Option::<String>::None);
    let mut import_input = use_signal(String::new);
    let mut import_status = use_signal(|| Option::<(bool, String)>::None);
    let mut copied_fp = use_signal(|| false);

    rsx! {
        div { class: "main-stage",
            div { class: if has_detail() { "settings-page has-detail" } else { "settings-page" },
                if has_detail() {
                    div { class: "settings-mobile-bar",
                        button {
                            r#type: "button",
                            class: "icon-btn",
                            aria_label: "Back to settings",
                            onclick: move |_| has_detail.set(false),
                            BackIcon {}
                        }
                        span { class: "settings-mobile-title", "Settings" }
                    }
                }
                // Left Settings Nav (on mobile this is the first screen)
                nav { class: "settings-nav", aria_label: "Settings categories",
                    div { class: "settings-search search-field",
                        crate::components::icons::SearchIcon { size: 16 }
                        input { r#type: "search", placeholder: "Search settings", aria_label: "Search settings", value: "{nav_query}",
                            oninput: move |e| nav_query.set(e.value()),
                            onkeydown: move |e| if e.key() == Key::Enter {
                                let q = crate::components::composer::fold(nav_query().trim());
                                if let Some((_, t, _)) = NAV.iter().find(|(_, t, l)| crate::components::composer::fold(l).contains(&q) || tab_keywords(*t).contains(&q)) {
                                    tab.set(*t);
                                    has_detail.set(true);
                                }
                            },
                        }
                    }
                    if !nav_query().trim().is_empty() && !NAV.iter().any(|(_, t, l)| { let q = crate::components::composer::fold(nav_query().trim()); crate::components::composer::fold(l).contains(&q) || tab_keywords(*t).contains(&q) }) {
                        div { class: "sidebar-empty", "No matching settings found" }
                    }
                    for (i, (group, t, label)) in NAV.iter().copied().enumerate().filter(|(_, (_, t, l))| { let q = crate::components::composer::fold(nav_query().trim()); q.is_empty() || crate::components::composer::fold(l).contains(&q) || tab_keywords(*t).contains(&q) }) {
                        if nav_query().trim().is_empty() && (i == 0 || NAV[i - 1].0 != group) {
                            div { class: "settings-nav-title", "{group}" }
                        }

                        button {
                            
                            r#type: "button",
                            class: if tab() == t { "settings-nav-item active" } else { "settings-nav-item" },
                            aria_current: if tab() == t { "page" } else { "false" },
                            onclick: move |_| { tab.set(t); has_detail.set(true); },
                            match t {
                                SettingsTab::Account => rsx! { UserIcon {} },
                                SettingsTab::Appearance => rsx! { PaletteIcon {} },
                                SettingsTab::Security => rsx! { ShieldIcon { size: 18 } },
                                SettingsTab::Session => rsx! { crate::components::icons::DevicesIcon { size: 18 } },
                                SettingsTab::Storage => rsx! { SlidersIcon {} },
                                SettingsTab::SyncRecovery => rsx! { crate::components::icons::RefreshIcon { size: 18 } },
                                SettingsTab::Backup => rsx! { crate::components::icons::ArchiveIcon { size: 18 } },
                                SettingsTab::Cloud => rsx! { crate::components::icons::CloudIcon { size: 18 } },
                                SettingsTab::Customize => rsx! { SlidersIcon {} },
                                SettingsTab::Stickers => rsx! { LibraryIcon {} },
                                SettingsTab::Guide => rsx! { BookIcon {} },
                                SettingsTab::Chat => rsx! { MessageIcon { size: 18 } },
                                SettingsTab::Notifications => rsx! { BellIcon {} },
                                SettingsTab::Privacy => rsx! { BlockIcon { size: 18 } },
                                SettingsTab::Lock => rsx! { LockIcon { size: 18 } },
                                SettingsTab::AutoReply => rsx! { ReplyIcon {} },
                                SettingsTab::Business => rsx! { StoreIcon { size: 18 } },
                                SettingsTab::Bots => rsx! { PluginIcon { size: 18 } },
                                SettingsTab::Language => rsx! { GlobeIcon { size: 18 } },
                                SettingsTab::Demo => rsx! { crate::components::icons::SparkleIcon { size: 18 } },
                                SettingsTab::Studio => rsx! { crate::components::icons::ImageIcon { size: 18 } },
                                SettingsTab::Inventory => rsx! { crate::components::icons::CrownIcon { size: 18 } },
                            }
                            span { "{label}" }
                            span { class: "chevron", ChevronRightIcon { size: 16 } }
                        }
                    }
                }

                div {
                    class: "panel-resizer",
                    "data-resizer": "settings-nav",
                    title: "Drag to resize sidebar",
                }

                // Right Settings Body
                div { class: "settings-body",
                    div { class: "settings-content",
                        div {
                            h1 { class: "settings-page-title", "{tab_label(tab())}" }
                        }
                        match tab() {
                            SettingsTab::Account => rsx! {
                                div { class: "settings-profile-card",
                                  div { class: "settings-profile-banner" }
                                  div { class: "settings-profile-main",
                                    div { class: "settings-avatar-wrapper",
                                        Avatar {
                                            name: current_username.clone(),
                                            avatar_url: my_avatar_url.clone(),
                                            size: 72,
                                        }
                                    }
                                    div { class: "settings-profile-info",
                                        div { class: "settings-profile-name", "{current_username}" }
                                        div { class: "settings-profile-sub", "@{current_username}" }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        onclick: move |_| on_open_profile.call(()),
                                        "Edit profile"
                                    }
                                  }
                                  div { class: "settings-profile-action", style: "padding: 0 var(--space-5) var(--space-5);",
                                        label { class: "btn-avatar-upload",
                                            input {
                                                id: "avatar-file-input",
                                                r#type: "file",
                                                accept: "image/*",
                                                disabled: is_uploading_avatar(),
                                                style: "display: none;",
                                                onchange: move |_| {
                                                    is_uploading_avatar.set(true);
                                                    avatar_error.set(None);
                                                    spawn(async move {
                                                        match platform::process_selected_avatar_bytes().await {
                                                            Ok(bytes) => match api::users_upload_avatar(bytes).await {
                                                                Ok(resp) => on_avatar_updated.call(resp.avatar_url),
                                                                Err(_) => avatar_error.set(Some("Unable to update avatar".into())),
                                                            },
                                                            Err(e) if e == "no_file" => {}
                                                            Err(_) => avatar_error.set(Some("Unable to process image file".into())),
                                                        }
                                                        is_uploading_avatar.set(false);
                                                    });
                                                },
                                            }
                                            if is_uploading_avatar() { "Uploading avatar" } else { "Change avatar" }
                                        }
                                        label { class: "btn-avatar-upload",
                                            input {
                                                id: "banner-file-input",
                                                r#type: "file",
                                                accept: "image/*",
                                                disabled: is_uploading_banner(),
                                                style: "display: none;",
                                                onchange: move |_| {
                                                    is_uploading_banner.set(true);
                                                    banner_status.set(None);
                                                    spawn(async move {
                                                        match platform::process_selected_banner_bytes().await {
                                                            Ok(bytes) => match api::users_upload_banner(bytes).await {
                                                                Ok(_) => banner_status.set(Some((false, "Profile banner saved".into()))),
                                                                Err(_) => banner_status.set(Some((true, "Unable to upload banner image".into()))),
                                                            },
                                                            Err(e) if e == "no_file" => {}
                                                            Err(_) => banner_status.set(Some((true, "Unable to process banner image".into()))),
                                                        }
                                                        is_uploading_banner.set(false);
                                                    });
                                                },
                                            }
                                            if is_uploading_banner() { "Uploading banner" } else { "Change banner" }
                                        }
                                        if let Some(err) = avatar_error() {
                                            span { class: "error-text", "{err}" }
                                        }
                                        if let Some((is_err, msg)) = banner_status() {
                                            div {
                                                class: if is_err { "status-badge error" } else { "status-badge success" },
                                                "{msg}"
                                            }
                                        }
                                  }
                                }

                                AccountSecurity {
                                    current_username: current_username.clone(),
                                    current_user_id: current_user_id.clone(),
                                    on_username_changed: move |n: String| if let Some(h) = on_username_changed { h.call(n) },
                                }
                            },

                            SettingsTab::Appearance => rsx! {
                                div { class: "settings-cols",
                                    div { class: "settings-col",
                                        div { class: "settings-group",
                                            div { class: "settings-group-title", "Theme" }
                                            div { class: "theme-preview theme-preview-5", role: "radiogroup", aria_label: "Theme",
                                                for (val, label) in [
                                                    ("light", "Light"),
                                                    ("ash", "Ash"),
                                                    ("dark", "Dark"),
                                                    ("oled", "Obsidian"),
                                                    ("system", "System"),
                                                ] {
                                                    button {
                                                        key: "{val}",
                                                        r#type: "button",
                                                        role: "radio",
                                                        aria_checked: "{appearance().theme == val}",
                                                        class: if appearance().theme == val { "theme-card active" } else { "theme-card" },
                                                        onclick: move |_| update_appearance(&|a| a.theme = val.to_string()),
                                                        div {
                                                            class: "theme-card-swatch",
                                                            style: match val {
                                                                "light" => "background: #F4F6FA;",
                                                                "ash" => "background: #313338;",
                                                                "dark" => "background: #1C1F26;",
                                                                "oled" => "background: #06070A;",
                                                                _ => "background: linear-gradient(90deg, #F4F6FA 50%, #1C1F26 50%);",
                                                            },
                                                            i { style: "width: 28%; background: var(--accent);" }
                                                            i {
                                                                style: match val {
                                                                    "light" => "flex: 1; background: #D8DEE8;",
                                                                    "ash" => "flex: 1; background: #434751;",
                                                                    "dark" => "flex: 1; background: #2B303C;",
                                                                    "oled" => "flex: 1; background: #161923;",
                                                                    _ => "flex: 1; background: rgba(127, 134, 148, 0.30);",
                                                                },
                                                            }
                                                        }
                                                        span { style: "display: inline-flex; align-items: center; gap: 5px; white-space: nowrap;",
                                                            match val {
                                                                "light" => rsx! { SunIcon { size: 14 } },
                                                                "ash" | "dark" | "oled" => rsx! { MoonIcon { size: 14 } },
                                                                _ => rsx! { MonitorIcon { size: 14 } },
                                                            }
                                                            "{label}"
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        div { class: "settings-group",
                                            div { class: "settings-group-title", "Accent color" }
                                            div { class: "settings-group-desc", "Applies to primary buttons, sent bubbles, and selected items." }
                                            div { class: "accent-swatches", role: "radiogroup", aria_label: "Accent color",
                                                {
                                                    let cur_custom = prefs_ctx.prefs.read().custom_accent.clone();
                                                    let cur_custom_empty = cur_custom.is_empty();
                                                    let preset_hex = platform::accent_preset_hex(appearance().accent.as_str());
                                                    let picker_val = if cur_custom_empty { preset_hex.to_string() } else { cur_custom.clone() };
                                                    let custom_bg = if cur_custom_empty {
                                                        "background: conic-gradient(from 0deg, #FF3B30, #FF9500, #FFCC00, #34C759, #00C7BE, #007AFF, #AF52DE, #FF2D55, #FF3B30);".to_string()
                                                    } else {
                                                        format!("background: {cur_custom};")
                                                    };
                                                    rsx! {
                                                        for (val, color, label) in [
                                                            ("blue", "#2E7CF6", "Blue"),
                                                            ("violet", "#8B5CF6", "Violet"),
                                                            ("green", "#10B981", "Green"),
                                                            ("orange", "#F97316", "Orange"),
                                                            ("rose", "#F43F7E", "Rose"),
                                                            ("cyan", "#06B6D4", "Cyan"),
                                                        ] {
                                                            {
                                                                let is_sw_active = cur_custom_empty && appearance().accent == val;
                                                                rsx! {
                                                                    button {
                                                                        key: "{val}",
                                                                        r#type: "button",
                                                                        role: "radio",
                                                                        title: "{label}",
                                                                        aria_label: "{label}",
                                                                        aria_checked: "{is_sw_active}",
                                                                        class: if is_sw_active { "accent-swatch active" } else { "accent-swatch" },
                                                                        style: "background: {color};",
                                                                        onclick: move |_| {
                                                                            if !prefs_ctx.prefs.read().custom_accent.is_empty() {
                                                                                prefs_ctx.update(|p| p.custom_accent.clear());
                                                                            }
                                                                            update_appearance(&|a| a.accent = val.to_string());
                                                                        },
                                                                        if is_sw_active { CheckIcon { size: 16 } }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        span {
                                                            class: if !cur_custom_empty { "accent-swatch accent-swatch-custom active" } else { "accent-swatch accent-swatch-custom" },
                                                            style: "{custom_bg}",
                                                            title: "Custom color",
                                                            input {
                                                                r#type: "color",
                                                                class: "accent-swatch-color-input",
                                                                aria_label: "Accent color",
                                                                value: "{picker_val}",
                                                                onchange: move |e| {
                                                                    let v = e.value().to_uppercase();
                                                                    prefs_ctx.update(move |p| p.custom_accent = v);
                                                                },
                                                            }
                                                            if !cur_custom_empty { CheckIcon { size: 16 } }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        div { class: "settings-group",
                                            div { class: "settings-list",
                                                div { class: "settings-row column-layout",
                                                    div { class: "settings-row-text",
                                                        span { class: "settings-label", "Font size" }
                                                        span { class: "settings-sublabel", "Auto-scale for comfortable reading or choose a size below" }
                                                    }
                                                    div { class: "range-row",
                                                        label { style: "display: inline-flex; align-items: center; gap: 6px; font-size: var(--text-sm); white-space: nowrap;",
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: appearance().font_size == "auto",
                                                                onchange: move |e| {
                                                                    let auto = e.checked();
                                                                    update_appearance(&|a| a.font_size = if auto { "auto".to_string() } else { "15".to_string() });
                                                                },
                                                            }
                                                            "Auto"
                                                        }
                                                        {
                                                            let cur_font: f32 = if appearance().font_size == "auto" { 15.0 } else { appearance().font_size.parse().unwrap_or(15.0) };
                                                            let font_pct = (((cur_font - 13.0) / 7.0) * 100.0).clamp(0.0, 100.0).round() as u8;
                                                            rsx! {
                                                                input {
                                                                    r#type: "range",
                                                                    min: "13",
                                                                    max: "20",
                                                                    step: "1",
                                                                    style: "--range-pct: {font_pct}%;",
                                                                    aria_label: "Font size",
                                                                    disabled: appearance().font_size == "auto",
                                                                    value: if appearance().font_size == "auto" { "15".to_string() } else { appearance().font_size },
                                                                    oninput: move |e| {
                                                                        let v = e.value();
                                                                        update_appearance(&|a| a.font_size = v.clone());
                                                                    },
                                                                }
                                                            }
                                                        }
                                                        span { class: "range-value",
                                                            if appearance().font_size == "auto" { "Auto" } else { "{appearance().font_size}px" }
                                                        }
                                                    }
                                                }
                                                div { class: "settings-row",
                                                    div { class: "settings-row-text",
                                                        span { class: "settings-label", "Display density" }
                                                        span { class: "settings-sublabel", "Spacing between messages and lists" }
                                                    }
                                                    select {
                                                        class: "cw-dropdown",
                                                        aria_label: "Display density",
                                                        value: "{appearance().density}",
                                                        onchange: move |e| {
                                                            let v = e.value();
                                                            update_appearance(&|a| a.density = v.clone());
                                                        },
                                                        for (val, label) in [("compact", "Compact"), ("comfortable", "Normal"), ("spacious", "Wide")] {
                                                            option {
                                                                key: "{val}",
                                                                value: "{val}",
                                                                selected: appearance().density == val,
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        AppearanceColorsShapes {}
                                    }

                                    div { class: "settings-col",
                                        div { class: "settings-group",
                                            div { class: "settings-group-title", "Preview" }
                                            {
                                                let preview_wp = prefs_ctx.prefs.read().wallpaper.clone();
                                                let preview_cls = if preview_wp.is_empty() || preview_wp == "none" {
                                                    "settings-preview-chat".to_string()
                                                } else {
                                                    format!("settings-preview-chat wp-{preview_wp}")
                                                };
                                                rsx! {
                                                    div { class: "{preview_cls}",
                                                        div { class: "message-row received group-start group-end",
                                                            div { class: "message-content-wrapper",
                                                                div { class: "message-bubble", "Text and background auto-balance for easy reading." span { class: "bubble-meta", "09:41" } }
                                                            }
                                                        }
                                                        div { class: "message-row sent group-start group-end",
                                                            div { class: "message-content-wrapper",
                                                                div { class: "message-bubble", "Crisp and clear in every theme style." span { class: "bubble-meta", "09:42" } }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        AppearanceWallpapersGlass {
                                            on_reset_appearance: move |_| {
                                                let d = platform::Appearance::default();
                                                platform::apply_appearance(&d);
                                                appearance.set(d);
                                            },
                                        }
                                    }
                                }
                            },

                            SettingsTab::Security => rsx! {
                                div { class: "settings-cols",
                                    div { class: "settings-col",
                                        crate::components::accounts::SecuritySettings {}
                                        div { class: "settings-group",
                                            div { class: "settings-row cw-brand-row",
                                                crate::components::brand_logos::E2eeLiveShield { size: 30 }
                                                div { class: "settings-row-text",
                                                    span { class: "settings-label", "Protection standard" }
                                                    span { class: "settings-sublabel", "Highest-grade end-to-end encryption" }
                                                }
                                                span { class: "settings-value tag-active",
                                                    span { class: "cw-live-dot" }
                                                    if user_keys.is_some() { "Active & secure" } else { "No key yet" }
                                                }
                                            }
                                            if let Some(ref keys) = user_keys {
                                                div { class: "settings-row column-layout",
                                                    div { style: "display: flex; justify-content: space-between; width: 100%; align-items: center;",
                                                        span { class: "settings-label", "Security fingerprint" }
                                                        button {
                                                            r#type: "button",
                                                            class: "btn-secondary",
                                                            onclick: {
                                                                let fp = keys.fingerprint_b64.clone();
                                                                move |_| {
                                                                    platform::copy_to_clipboard(&fp);
                                                                    copied_fp.set(true);
                                                                }
                                                            },
                                                            if copied_fp() { "Copied" } else { "Copy" }
                                                        }
                                                    }
                                                    div { class: "fingerprint", "{format_fingerprint(&keys.fingerprint_b64)}" }
                                                }
                                            }
                                        }
                                    }

                                    div { class: "settings-col",
                                        div { class: "settings-group",
                                            div { class: "settings-group-title", "Backup security key" }
                                            div { class: "settings-row column-layout",
                                                div { style: "font-size: 13px; color: var(--text-secondary);",
                                                    "Your security key is stored only on this device and never sent over the network. You can back it up to transfer to another device."
                                                }
                                                if let Some(ref keys) = user_keys {
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-secondary",
                                                        onclick: {
                                                            let k = keys.clone();
                                                            move |_| {
                                                                let j = export_user_keys_json(&k);
                                                                platform::copy_to_clipboard(&j);
                                                                exported_json.set(Some(j));
                                                            }
                                                        },
                                                        KeyIcon { size: 15 }
                                                        span { "Copy security key" }
                                                    }
                                                }
                                                if let Some(ref exp) = exported_json() {
                                                    textarea {
                                                        readonly: true,
                                                        style: "width: 100%; height: 68px; font-family: var(--font-mono); font-size: 11px;",
                                                        value: "{exp}",
                                                    }
                                                }
                                            }

                                            div { class: "settings-row column-layout",
                                                label { class: "form-label", "Import security key" }
                                                textarea {
                                                    style: "width: 100%; height: 68px; font-family: var(--font-mono); font-size: 11px;",
                                                    placeholder: "Paste security key here",
                                                    value: "{import_input}",
                                                    oninput: move |e| import_input.set(e.value()),
                                                }
                                                button {
                                                    r#type: "button",
                                                    class: "btn-primary",
                                                    disabled: import_input().trim().is_empty(),
                                                    onclick: {
                                                        let uid = current_user_id.clone();
                                                        move |_| {
                                                            let raw = import_input();
                                                            match import_user_keys_json(&uid, &raw) {
                                                                Ok(restored) => {
                                                                    let ecdh = restored.ecdh_raw_b64.clone();
                                                                    let ecdsa = restored.ecdsa_raw_b64.clone();
                                                                    let restored_clone = restored.clone();
                                                                    spawn(async move {
                                                                        let _ = api::keys_register(&ecdh, &ecdsa).await;
                                                                        on_keys_imported.call(restored_clone);
                                                                        import_input.set(String::new());
                                                                        import_status.set(Some((false, "Key restored successfully".into())));
                                                                    });
                                                                }
                                                                Err(_) => {
                                                                    import_status.set(Some((true, "Invalid key".into())));
                                                                }
                                                            }
                                                        }
                                                    },
                                                    "Restore key"
                                                }
                                                if let Some((is_err, msg)) = import_status() {
                                                    div {
                                                        class: if is_err { "status-badge error" } else { "status-badge success" },
                                                        "{msg}"
                                                    }
                                                }
                                            }
                                        }
                                        crate::components::backup_cloud_ui::AdvancedSecurityExportSection {}
                                    }
                                }
                            },

                            SettingsTab::Customize => rsx! {
                                div { class: "settings-cols",
                                    div { class: "settings-col", CustomizationSettings {} }
                                    div { class: "settings-col", crate::components::settings_more::SyncMotionSettings {} }
                                }
                            },
                            SettingsTab::Inventory => rsx! { crate::components::inventory::Inventory { my_id: current_user_id.clone() } },
                            SettingsTab::Studio => rsx! { crate::components::studio::StudioHome { my_id: current_user_id.clone() } },
                            SettingsTab::Demo => rsx! { crate::components::demo_gallery::DemoGallery { my_id: current_user_id.clone(), my_name: current_username.clone(), my_avatar: my_avatar_url.clone() } },
                            SettingsTab::Chat => rsx! { ChatPrefsSettings {} },
                            SettingsTab::Notifications => rsx! { NotificationSettings {} },
                            SettingsTab::Privacy => rsx! { PrivacySettingsPanel {} },
                            SettingsTab::Lock => rsx! { LockSettings { on_lock_now: move |_| if let Some(h) = on_lock_now { h.call(()) } } },
                            SettingsTab::AutoReply => rsx! { AutoReplySettings {} },
                            SettingsTab::Business => rsx! { BusinessSettings {} },
                            SettingsTab::Bots => rsx! { BotsSettings { my_username: current_username.clone() } },
                            SettingsTab::Language => rsx! { LanguageSettings {} },
                            SettingsTab::Stickers => rsx! { LibrarySettings { current_user_id: current_user_id.clone() } },
                            SettingsTab::Guide => rsx! { GuideSettings { on_replay_tour: move |_| on_replay_tour.call(()) } },
                            SettingsTab::Storage => rsx! { crate::components::storage_ui::StorageSettingsPanel {} },
                            SettingsTab::SyncRecovery => rsx! {
                                crate::components::storage_ui::SyncRecoverySettingsPanel {
                                    current_user_id: current_user_id.clone(),
                                    user_keys: user_keys.clone(),
                                    on_keys_imported: on_keys_imported,
                                }
                            },
                            SettingsTab::Backup => rsx! {
                                crate::components::backup_cloud_ui::BackupExportRestorePanel {}
                            },
                            SettingsTab::Cloud => rsx! {
                                crate::components::backup_cloud_ui::CloudAccountsPanel {}
                            },
                            SettingsTab::Session => rsx! {
                                div { class: "settings-cols",
                                    div { class: "settings-col",
                                        crate::components::storage_ui::DeviceTrustPanel {}
                                    }
                                    div { class: "settings-col",
                                        div { class: "settings-group",
                                            div { class: "settings-row",
                                                div { class: "settings-row-text",
                                                    span { class: "settings-label", "Sign out of account" }
                                                    span { class: "settings-sublabel", "Sign out of this device. Your messages remain safe." }
                                                }
                                                button {
                                                    r#type: "button",
                                                    class: "btn-danger-outline",
                                                    onclick: move |_| on_logout.call(()),
                                                    LogoutIcon {}
                                                    span { "Log out" }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

