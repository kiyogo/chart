use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        business_view::BusinessSettings,
        details_panel::format_fingerprint,
        guide::GuideSettings,
        icons::{
            BackIcon, BoltIcon, CheckIcon, ChevronRightIcon, CloudIcon, GlobeIcon, InfoIcon,
            KeyIcon, LogoutIcon, MessageIcon, MonitorIcon, MoonIcon, PaletteIcon, SearchIcon,
            ShieldIcon, SparkleIcon, SunIcon, UserIcon,
        },
        plugins_view::BotsSettings,
        settings_about_dev::{AboutOverviewPanel, DeveloperDiagnosticsPanel},
        settings_extra::{AccountSecurity, CustomizationSettings, LibrarySettings},
        settings_more::{
            AppearanceColorsShapes, AppearanceWallpapersGlass, AutoReplySettings,
            ChatPrefsSettings, LanguageSettings, LockSettings, NotificationSettings,
            PrivacySettingsPanel, SyncMotionSettings,
        },
    },
    crypto::e2ee::{UserKeys, export_user_keys_json, import_user_keys_json},
    platform,
    services::api,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Account,
    PrivacySecurity,
    ChatNotifications,
    Appearance,
    Language,
    DataBackup,
    Creative,
    About,
    Developer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountSub {
    Profile,
    Devices,
    Business,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivacySub {
    Privacy,
    AppLock,
    Security,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChatSub {
    Chat,
    Notifications,
    AutoReply,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppearanceSub {
    Look,
    Personalize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DataSub {
    Storage,
    Backup,
    Cloud,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CreativeSub {
    Library,
    Draw,
    Inventory,
    Showcase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AboutSub {
    Overview,
    Terms,
    Privacy,
    Community,
    Vision,
    Guide,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DevSub {
    AdvancedExport,
    EncryptionKeys,
    SyncRecovery,
    Extensions,
    Diagnostics,
}

/// Compact 8 everyday top-level Settings items + conditional 9th Developer item.
const NAV: [(SettingsTab, &str); 9] = [
    (SettingsTab::Account, "Account"),
    (SettingsTab::PrivacySecurity, "Privacy & security"),
    (SettingsTab::ChatNotifications, "Chat & notifications"),
    (SettingsTab::Appearance, "Appearance"),
    (SettingsTab::Language, "Language"),
    (SettingsTab::DataBackup, "Data & backup"),
    (SettingsTab::Creative, "Creative"),
    (SettingsTab::About, "About"),
    (SettingsTab::Developer, "Developer"),
];

fn tab_label(t: SettingsTab) -> &'static str {
    NAV.iter().find(|x| x.0 == t).map(|x| x.1).unwrap_or("")
}

#[inline]
fn tab_bit(t: SettingsTab) -> u16 {
    1u16 << (t as u16)
}

#[inline]
fn set_sig_if_changed<T: PartialEq + 'static>(mut sig: Signal<T>, val: T) {
    if *sig.peek() != val {
        sig.set(val);
    }
}

#[component]
fn SettingsPageShell(has_detail: Signal<bool>, children: Element) -> Element {
    let hd = has_detail();
    rsx! {
        div { class: if hd { "settings-page has-detail" } else { "settings-page" },
            if hd {
                div { class: "settings-mobile-bar",
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        aria_label: "Back to settings",
                        onclick: move |_| set_sig_if_changed(has_detail, false),
                        BackIcon {}
                    }
                    span { class: "settings-mobile-title", "Settings" }
                }
            }
            {children}
        }
    }
}

#[component]
fn SettingsNavPane(
    tab: Signal<SettingsTab>,
    mut mounted_tabs: Signal<u16>,
    on_select: EventHandler<(SettingsTab, String)>,
) -> Element {
    let mut nav_query = use_signal(String::new);
    let prefs_ctx = use_context::<crate::prefs::PrefsCtx>();
    let dev_mode = prefs_ctx.prefs.read().developer_mode;
    let cur = tab();
    let effective_tab = if !dev_mode && cur == SettingsTab::Developer {
        SettingsTab::About
    } else {
        cur
    };
    let visible_nav: Vec<(SettingsTab, &str)> = NAV
        .iter()
        .copied()
        .filter(|(t, _)| *t != SettingsTab::Developer || dev_mode)
        .collect();
    let folded_q = crate::components::composer::fold(nav_query().trim());
    let filtered_nav: Vec<(SettingsTab, &str)> = visible_nav
        .iter()
        .copied()
        .filter(|(t, l)| {
            folded_q.is_empty()
                || crate::components::composer::fold(l).contains(&folded_q)
                || tab_keywords(*t).contains(&folded_q)
        })
        .collect();

    rsx! {
        nav { class: "settings-nav", aria_label: "Settings categories",
            div { class: "settings-search search-field",
                SearchIcon { size: 16 }
                input {
                    r#type: "search",
                    placeholder: "Search settings",
                    aria_label: "Search settings",
                    value: "{nav_query}",
                    oninput: move |e| nav_query.set(e.value()),
                    onkeydown: move |e| if e.key() == Key::Enter {
                        let raw = nav_query.peek().clone();
                        let q = crate::components::composer::fold(raw.trim());
                        if let Some((t, _)) = NAV
                            .iter()
                            .copied()
                            .filter(|(t, _)| *t != SettingsTab::Developer || prefs_ctx.prefs.peek().developer_mode)
                            .find(|(t, l)| crate::components::composer::fold(l).contains(&q) || tab_keywords(*t).contains(&q))
                        {
                            on_select.call((t, raw));
                        }
                    },
                }
            }

            if !folded_q.is_empty() && filtered_nav.is_empty() {
                div { class: "sidebar-empty", "No matching settings found" }
            }

            for (t, label) in filtered_nav {
                button {
                    key: "{label}",
                    r#type: "button",
                    class: if effective_tab == t {
                        if t == SettingsTab::Developer { "settings-nav-item settings-nav-dev active" } else { "settings-nav-item active" }
                    } else {
                        if t == SettingsTab::Developer { "settings-nav-item settings-nav-dev" } else { "settings-nav-item" }
                    },
                    aria_current: if effective_tab == t { "page" } else { "false" },
                    onmouseenter: move |_| {
                        let bit = tab_bit(t);
                        let cur_mask = *mounted_tabs.peek();
                        if (cur_mask & bit) == 0 {
                            mounted_tabs.set(cur_mask | bit);
                        }
                    },
                    onmousedown: move |e| {
                        if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                            crate::storage::msg_store::perf_mark_interaction("settings:tab");
                            let raw = nav_query.peek().clone();
                            on_select.call((t, raw));
                        }
                    },
                    onclick: move |_| {
                        let raw = nav_query.peek().clone();
                        on_select.call((t, raw));
                    },
                    match t {
                        SettingsTab::Account => rsx! { UserIcon {} },
                        SettingsTab::PrivacySecurity => rsx! { ShieldIcon { size: 18 } },
                        SettingsTab::ChatNotifications => rsx! { MessageIcon { size: 18 } },
                        SettingsTab::Appearance => rsx! { PaletteIcon {} },
                        SettingsTab::Language => rsx! { GlobeIcon { size: 18 } },
                        SettingsTab::DataBackup => rsx! { CloudIcon { size: 18 } },
                        SettingsTab::Creative => rsx! { SparkleIcon { size: 18 } },
                        SettingsTab::About => rsx! { InfoIcon { size: 18 } },
                        SettingsTab::Developer => rsx! { BoltIcon { size: 18 } },
                    }
                    span { "{label}" }
                    span { class: "chevron", ChevronRightIcon { size: 16 } }
                }
            }
        }
    }
}

#[component]
fn SettingsHeaderTitle(tab: Signal<SettingsTab>) -> Element {
    let prefs_ctx = use_context::<crate::prefs::PrefsCtx>();
    let dev_mode = prefs_ctx.prefs.read().developer_mode;
    let cur = tab();
    let effective_tab = if !dev_mode && cur == SettingsTab::Developer {
        SettingsTab::About
    } else {
        cur
    };
    rsx! {
        div {
            h1 { class: "settings-page-title", "{tab_label(effective_tab)}" }
        }
    }
}

/// Spec 166: Isolated Keep-Alive tab panel. Subscribes via `Memo<bool>` so ONLY the
/// outgoing and incoming tabs flip their `cw-keepalive-box` class (< 0.5ms!).
#[component]
fn SettingsKeepAlivePane(
    tab: Signal<SettingsTab>,
    mounted_tabs: Signal<u16>,
    target: SettingsTab,
    children: Element,
) -> Element {
    let prefs_ctx = use_context::<crate::prefs::PrefsCtx>();
    let bit = tab_bit(target);
    let is_mounted = use_memo(move || (mounted_tabs() & bit) != 0);
    let is_active = use_memo(move || {
        let cur = tab();
        let dev_mode = prefs_ctx.prefs.read().developer_mode;
        let eff = if !dev_mode && cur == SettingsTab::Developer {
            SettingsTab::About
        } else {
            cur
        };
        eff == target
    });
    if !is_mounted() {
        return rsx! {};
    }
    rsx! {
        div {
            class: if is_active() { "cw-keepalive-box" } else { "cw-keepalive-box hidden" },
            {children}
        }
    }
}

/// Bilingual search keywords so finding any nested setting takes 1 keystroke.
fn tab_keywords(t: SettingsTab) -> &'static str {
    match t {
        SettingsTab::Account => "account tai khoan ho so avatar anh dai dien anh bia ten username profile devices session thiet bi phien dang xuat log out business kinh doanh ban hang san pham",
        SettingsTab::PrivacySecurity => "privacy quyen rieng tu chan block da xem dang soan ket ban goi app lock khoa pin ma pin security bao mat xac minh hai buoc 2fa totp ma du phong nhat ky",
        SettingsTab::ChatNotifications => "chat tro chuyen goi y sticker dan excel bang enter tra loi bam dup notifications notification sound thong bao am thanh am bao auto reply autoreply tra loi tu dong vang mat",
        SettingsTab::Appearance => "appearance theme dark light accent wallpaper font giao dien chu de toi sang mau nhan co chu mau bong bong hinh nen phong chu trong suot do mo khung kinh personalize customize tuy chinh ghim cong dong nhom hieu ung avatar danh sach",
        SettingsTab::Language => "language locale ngon ngu tieng viet tieng anh english",
        SettingsTab::DataBackup => "data storage cache luu tru bo nho dem quota backup export import restore sao luu khoi phuc kdata cloud dam may google drive onedrive dropbox nextcloud webdav s3 minio nas snapshot",
        SettingsTab::Creative => "creative sang tao library sticker emoji gif upload thu vien draw studio ve hoat anh animation layer khung hinh inventory kho do vat pham thanh tich huy hieu showcase preview demo xem thu hieu ung nameplate",
        SettingsTab::About => "about gioi thieu version phien ban build system he thong license giay phep terms dieu khoan thoa thuan privacy chinh sach bao mat community tieu chuan cong dong vision tam nhin san pham guide help faq huong dan phim tat tro giup developer mode che do nha phat trien",
        SettingsTab::Developer => "developer nha phat trien advanced export xuat du lieu nang cao zip aes-256 encryption keys khoa ma hoa e2ee van tay fingerprint backup json sync recovery dong bo khoi phuc da thiet bi webrtc extensions plugins bot tien ich mo rong diagnostics chan doan",
    }
}

#[component]
fn SubTabBar(items: Vec<(&'static str, &'static str)>, active: &'static str, on_select: EventHandler<&'static str>) -> Element {
    rsx! {
        div { class: "settings-subtabs", role: "tablist",
            for (id, label) in items {
                button {
                    key: "{id}",
                    r#type: "button",
                    role: "tab",
                    aria_selected: "{active == id}",
                    class: if active == id { "settings-subtab active" } else { "settings-subtab" },
                    onmousedown: move |e| {
                        if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                            crate::storage::msg_store::perf_mark_interaction("settings:subtab");
                            on_select.call(id);
                        }
                    },
                    onclick: move |_| on_select.call(id),
                    "{label}"
                }
            }
        }
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
    let tab = use_signal(|| SettingsTab::Account);
    let mut mounted_tabs = use_signal(|| tab_bit(SettingsTab::Account));
    let mut tabs_prewarmed = use_hook(|| CopyValue::new(false));
    let account_sub = use_signal(|| AccountSub::Profile);
    let privacy_sub = use_signal(|| PrivacySub::Privacy);
    let chat_sub = use_signal(|| ChatSub::Chat);
    let appearance_sub = use_signal(|| AppearanceSub::Look);
    let data_sub = use_signal(|| DataSub::Storage);
    let creative_sub = use_signal(|| CreativeSub::Library);
    let about_sub = use_signal(|| AboutSub::Overview);
    let dev_sub = use_signal(|| DevSub::AdvancedExport);

    let has_detail = use_signal(|| false);
    let prefs_ctx = use_context::<crate::prefs::PrefsCtx>();
    let appearance = move || prefs_ctx.prefs.read().to_appearance();

    // Spec 166: Stagger mounting core everyday Settings tabs in idle slices after Settings
    // mounts (pre-warmed hidden at boot) so even the VERY FIRST click on any core tab is < 1ms!
    use_effect(move || {
        if *tabs_prewarmed.peek() {
            return;
        }
        tabs_prewarmed.set(true);
        spawn(async move {
            for t in [
                SettingsTab::PrivacySecurity,
                SettingsTab::ChatNotifications,
                SettingsTab::Appearance,
                SettingsTab::Language,
                SettingsTab::DataBackup,
                SettingsTab::About,
            ] {
                platform::sleep_ms(55).await;
                let bit = tab_bit(t);
                let cur_mask = *mounted_tabs.peek();
                if (cur_mask & bit) == 0 {
                    mounted_tabs.set(cur_mask | bit);
                }
            }
        });
    });

    let update_appearance = move |f: &dyn Fn(&mut platform::Appearance)| {
        prefs_ctx.update(|p| {
            let mut a = p.to_appearance();
            f(&mut a);
            p.set_appearance(&a);
        });
    };
    let mut is_uploading_avatar = use_signal(|| false);
    let mut avatar_error = use_signal(|| Option::<String>::None);
    let mut is_uploading_banner = use_signal(|| false);
    let mut banner_status = use_signal(|| Option::<(bool, String)>::None);

    // Raw E2EE Key backup & restore state (lives in Developer -> Encryption keys)
    let mut exported_json = use_signal(|| Option::<String>::None);
    let mut import_input = use_signal(String::new);
    let mut import_status = use_signal(|| Option::<(bool, String)>::None);
    let mut copied_fp = use_signal(|| false);

    // Deep-link sub-tab matching when navigating from search query
    let mut select_tab_for_query = move |t: SettingsTab, raw_q: &str| {
        let bit = tab_bit(t);
        let cur_mask = *mounted_tabs.peek();
        if (cur_mask & bit) == 0 {
            mounted_tabs.set(cur_mask | bit);
        }
        set_sig_if_changed(tab, t);
        set_sig_if_changed(has_detail, true);
        let q = crate::components::composer::fold(raw_q.trim());
        if q.is_empty() {
            return;
        }
        match t {
            SettingsTab::Account => {
                if q.contains("device") || q.contains("thiet bi") || q.contains("session") || q.contains("dang xuat") || q.contains("log out") {
                    set_sig_if_changed(account_sub, AccountSub::Devices);
                } else if q.contains("business") || q.contains("kinh doanh") || q.contains("ban hang") || q.contains("san pham") {
                    set_sig_if_changed(account_sub, AccountSub::Business);
                } else {
                    set_sig_if_changed(account_sub, AccountSub::Profile);
                }
            }
            SettingsTab::PrivacySecurity => {
                if q.contains("lock") || q.contains("khoa") || q.contains("pin") {
                    set_sig_if_changed(privacy_sub, PrivacySub::AppLock);
                } else if q.contains("security") || q.contains("bao mat") || q.contains("2fa") || q.contains("totp") || q.contains("xac minh") {
                    set_sig_if_changed(privacy_sub, PrivacySub::Security);
                } else {
                    set_sig_if_changed(privacy_sub, PrivacySub::Privacy);
                }
            }
            SettingsTab::ChatNotifications => {
                if q.contains("notif") || q.contains("thong bao") || q.contains("sound") || q.contains("am bao") || q.contains("am thanh") {
                    set_sig_if_changed(chat_sub, ChatSub::Notifications);
                } else if q.contains("auto") || q.contains("tu dong") || q.contains("vang mat") {
                    set_sig_if_changed(chat_sub, ChatSub::AutoReply);
                } else {
                    set_sig_if_changed(chat_sub, ChatSub::Chat);
                }
            }
            SettingsTab::Appearance => {
                if q.contains("personal") || q.contains("custom") || q.contains("tuy chinh") || q.contains("ghim") || q.contains("hieu ung avatar") {
                    set_sig_if_changed(appearance_sub, AppearanceSub::Personalize);
                } else {
                    set_sig_if_changed(appearance_sub, AppearanceSub::Look);
                }
            }
            SettingsTab::DataBackup => {
                if q.contains("cloud") || q.contains("dam may") || q.contains("drive") || q.contains("dropbox") || q.contains("s3") {
                    set_sig_if_changed(data_sub, DataSub::Cloud);
                } else if q.contains("backup") || q.contains("sao luu") || q.contains("restore") || q.contains("khoi phuc") || q.contains("kdata") {
                    set_sig_if_changed(data_sub, DataSub::Backup);
                } else {
                    set_sig_if_changed(data_sub, DataSub::Storage);
                }
            }
            SettingsTab::Creative => {
                if q.contains("draw") || q.contains("ve") || q.contains("studio") || q.contains("animation") {
                    set_sig_if_changed(creative_sub, CreativeSub::Draw);
                } else if q.contains("inventory") || q.contains("kho") || q.contains("huy hieu") {
                    set_sig_if_changed(creative_sub, CreativeSub::Inventory);
                } else if q.contains("showcase") || q.contains("demo") || q.contains("xem thu") {
                    set_sig_if_changed(creative_sub, CreativeSub::Showcase);
                } else {
                    set_sig_if_changed(creative_sub, CreativeSub::Library);
                }
            }
            SettingsTab::About => {
                if q.contains("terms") || q.contains("dieu khoan") || q.contains("thoa thuan") {
                    set_sig_if_changed(about_sub, AboutSub::Terms);
                } else if q.contains("privacy") || q.contains("chinh sach") || q.contains("bao mat") {
                    set_sig_if_changed(about_sub, AboutSub::Privacy);
                } else if q.contains("community") || q.contains("tieu chuan") || q.contains("cong dong") {
                    set_sig_if_changed(about_sub, AboutSub::Community);
                } else if q.contains("vision") || q.contains("tam nhin") || q.contains("san pham") {
                    set_sig_if_changed(about_sub, AboutSub::Vision);
                } else if q.contains("guide") || q.contains("huong dan") || q.contains("phim tat") || q.contains("faq") {
                    set_sig_if_changed(about_sub, AboutSub::Guide);
                } else {
                    set_sig_if_changed(about_sub, AboutSub::Overview);
                }
            }
            SettingsTab::Developer => {
                if q.contains("key") || q.contains("khoa") || q.contains("fingerprint") || q.contains("van tay") || q.contains("e2ee") {
                    set_sig_if_changed(dev_sub, DevSub::EncryptionKeys);
                } else if q.contains("sync") || q.contains("dong bo") || q.contains("webrtc") || q.contains("recovery") {
                    set_sig_if_changed(dev_sub, DevSub::SyncRecovery);
                } else if q.contains("ext") || q.contains("plugin") || q.contains("bot") || q.contains("tien ich") {
                    set_sig_if_changed(dev_sub, DevSub::Extensions);
                } else if q.contains("diag") || q.contains("chan doan") {
                    set_sig_if_changed(dev_sub, DevSub::Diagnostics);
                } else {
                    set_sig_if_changed(dev_sub, DevSub::AdvancedExport);
                }
            }
            SettingsTab::Language => {}
        }
    };

    rsx! {
        div { class: "main-stage",
            SettingsPageShell { has_detail,
                SettingsNavPane {
                    tab,
                    mounted_tabs,
                    on_select: move |(t, raw): (SettingsTab, String)| select_tab_for_query(t, &raw),
                }

                div {
                    class: "panel-resizer",
                    "data-resizer": "settings-nav",
                    title: "Drag to resize sidebar",
                }

                // Right Settings Body
                div { class: "settings-body",
                    div { class: "settings-content",
                        SettingsHeaderTitle { tab }

                        // 1. Account (Profile | Devices | Business)
                        SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::Account,
                            SubTabBar {
                                items: vec![
                                    ("profile", "Profile"),
                                    ("devices", "Devices"),
                                    ("business", "Business"),
                                ],
                                active: match account_sub() {
                                    AccountSub::Profile => "profile",
                                    AccountSub::Devices => "devices",
                                    AccountSub::Business => "business",
                                },
                                on_select: move |id| match id {
                                    "devices" => set_sig_if_changed(account_sub, AccountSub::Devices),
                                    "business" => set_sig_if_changed(account_sub, AccountSub::Business),
                                    _ => set_sig_if_changed(account_sub, AccountSub::Profile),
                                },
                            }
                                match account_sub() {
                                    AccountSub::Profile => rsx! {
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
                                                        accept: "image/png,image/jpeg,image/webp,image/avif,image/gif",
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
                                                                    Err(e) if e == "svg_not_allowed" => avatar_error.set(Some("Không hỗ trợ gửi tệp SVG".into())),
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
                                                        accept: "image/png,image/jpeg,image/webp,image/avif,image/gif",
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
                                                                    Err(e) if e == "svg_not_allowed" => banner_status.set(Some((true, "Không hỗ trợ gửi tệp SVG".into()))),
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
                                    AccountSub::Devices => rsx! {
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
                                    AccountSub::Business => rsx! { BusinessSettings {} },
                                }
                            }

                            // 2. Privacy & security (Privacy | App lock | Security — everyday friendly controls)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::PrivacySecurity,
                                SubTabBar {
                                    items: vec![
                                        ("privacy", "Privacy"),
                                        ("applock", "App lock"),
                                        ("security", "Security"),
                                    ],
                                    active: match privacy_sub() {
                                        PrivacySub::Privacy => "privacy",
                                        PrivacySub::AppLock => "applock",
                                        PrivacySub::Security => "security",
                                    },
                                    on_select: move |id| match id {
                                        "applock" => set_sig_if_changed(privacy_sub, PrivacySub::AppLock),
                                        "security" => set_sig_if_changed(privacy_sub, PrivacySub::Security),
                                        _ => set_sig_if_changed(privacy_sub, PrivacySub::Privacy),
                                    },
                                }
                                match privacy_sub() {
                                    PrivacySub::Privacy => rsx! { PrivacySettingsPanel {} },
                                    PrivacySub::AppLock => rsx! {
                                        LockSettings { on_lock_now: move |_| if let Some(h) = on_lock_now { h.call(()) } }
                                    },
                                    PrivacySub::Security => rsx! {
                                        div { class: "settings-cols",
                                            div { class: "settings-col",
                                                crate::components::accounts::SecuritySettings {}
                                            }
                                            div { class: "settings-col",
                                                DeviceEncryptionKeyCard {
                                                    current_user_id: current_user_id.clone(),
                                                    user_keys: user_keys.clone(),
                                                    on_keys_imported: on_keys_imported,
                                                }
                                            }
                                        }
                                    },
                                }
                            }

                            // 3. Chat & notifications (Chat | Notifications | Auto reply)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::ChatNotifications,
                                SubTabBar {
                                    items: vec![
                                        ("chat", "Chat"),
                                        ("notifications", "Notifications"),
                                        ("autoreply", "Auto reply"),
                                    ],
                                    active: match chat_sub() {
                                        ChatSub::Chat => "chat",
                                        ChatSub::Notifications => "notifications",
                                        ChatSub::AutoReply => "autoreply",
                                    },
                                    on_select: move |id| match id {
                                        "notifications" => set_sig_if_changed(chat_sub, ChatSub::Notifications),
                                        "autoreply" => set_sig_if_changed(chat_sub, ChatSub::AutoReply),
                                        _ => set_sig_if_changed(chat_sub, ChatSub::Chat),
                                    },
                                }
                                match chat_sub() {
                                    ChatSub::Chat => rsx! { ChatPrefsSettings {} },
                                    ChatSub::Notifications => rsx! { NotificationSettings {} },
                                    ChatSub::AutoReply => rsx! { AutoReplySettings {} },
                                }
                            }

                            // 4. Appearance (Theme & look | Personalize)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::Appearance,
                                SubTabBar {
                                    items: vec![
                                        ("look", "Theme & look"),
                                        ("personalize", "Personalize"),
                                    ],
                                    active: match appearance_sub() {
                                        AppearanceSub::Look => "look",
                                        AppearanceSub::Personalize => "personalize",
                                    },
                                    on_select: move |id| match id {
                                        "personalize" => set_sig_if_changed(appearance_sub, AppearanceSub::Personalize),
                                        _ => set_sig_if_changed(appearance_sub, AppearanceSub::Look),
                                    },
                                }
                                match appearance_sub() {
                                    AppearanceSub::Look => rsx! {
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
                                                                        div { class: "message-bubble", span { class: "preview-bubble-text", "Text and background auto-balance for easy reading." } span { class: "bubble-meta", "09:41" } }
                                                                    }
                                                                }
                                                                div { class: "message-row sent group-start group-end",
                                                                    div { class: "message-content-wrapper",
                                                                        div { class: "message-bubble", span { class: "preview-bubble-text", "Crisp and clear in every theme style." } span { class: "bubble-meta", "09:42" } }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                AppearanceWallpapersGlass {
                                                    on_reset_appearance: move |_| {
                                                        let d = platform::Appearance::default();
                                                        prefs_ctx.update(|p| p.set_appearance(&d));
                                                    },
                                                }
                                            }
                                        }
                                    },
                                    AppearanceSub::Personalize => rsx! {
                                        div { class: "settings-cols",
                                            div { class: "settings-col", CustomizationSettings {} }
                                            div { class: "settings-col", SyncMotionSettings {} }
                                        }
                                    },
                                }
                            }

                            // 5. Language (197 world languages directly)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::Language,
                                LanguageSettings {}
                            }

                            // 6. Data & backup (Storage | Backup | Cloud)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::DataBackup,
                                SubTabBar {
                                    items: vec![
                                        ("storage", "Storage"),
                                        ("backup", "Backup"),
                                        ("cloud", "Cloud"),
                                    ],
                                    active: match data_sub() {
                                        DataSub::Storage => "storage",
                                        DataSub::Backup => "backup",
                                        DataSub::Cloud => "cloud",
                                    },
                                    on_select: move |id| match id {
                                        "backup" => set_sig_if_changed(data_sub, DataSub::Backup),
                                        "cloud" => set_sig_if_changed(data_sub, DataSub::Cloud),
                                        _ => set_sig_if_changed(data_sub, DataSub::Storage),
                                    },
                                }
                                match data_sub() {
                                    DataSub::Storage => rsx! { crate::components::storage_ui::StorageSettingsPanel {} },
                                    DataSub::Backup => rsx! { crate::components::backup_cloud_ui::BackupExportRestorePanel {} },
                                    DataSub::Cloud => rsx! { crate::components::backup_cloud_ui::CloudAccountsPanel {} },
                                }
                            }

                            // 7. Creative (Library | Draw | Inventory | Showcase)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::Creative,
                                SubTabBar {
                                    items: vec![
                                        ("library", "Library"),
                                        ("draw", "Draw"),
                                        ("inventory", "Inventory"),
                                        ("showcase", "Showcase"),
                                    ],
                                    active: match creative_sub() {
                                        CreativeSub::Library => "library",
                                        CreativeSub::Draw => "draw",
                                        CreativeSub::Inventory => "inventory",
                                        CreativeSub::Showcase => "showcase",
                                    },
                                    on_select: move |id| match id {
                                        "draw" => set_sig_if_changed(creative_sub, CreativeSub::Draw),
                                        "inventory" => set_sig_if_changed(creative_sub, CreativeSub::Inventory),
                                        "showcase" => set_sig_if_changed(creative_sub, CreativeSub::Showcase),
                                        _ => set_sig_if_changed(creative_sub, CreativeSub::Library),
                                    },
                                }
                                match creative_sub() {
                                    CreativeSub::Library => rsx! { LibrarySettings { current_user_id: current_user_id.clone() } },
                                    CreativeSub::Draw => rsx! { crate::components::studio::StudioHome { my_id: current_user_id.clone() } },
                                    CreativeSub::Inventory => rsx! { crate::components::inventory::Inventory { my_id: current_user_id.clone() } },
                                    CreativeSub::Showcase => rsx! {
                                        crate::components::demo_gallery::DemoGallery {
                                            my_id: current_user_id.clone(),
                                            my_name: current_username.clone(),
                                            my_avatar: my_avatar_url.clone(),
                                        }
                                    },
                                }
                            }

                            // 8. About (Overview — iOS/Android About + Developer mode toggle | Terms | Privacy | Community | Vision | Guide)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::About,
                                SubTabBar {
                                    items: vec![
                                        ("overview", "Overview"),
                                        ("terms", "Điều khoản dịch vụ"),
                                        ("privacy", "Chính sách bảo mật"),
                                        ("community", "Tiêu chuẩn cộng đồng"),
                                        ("vision", "Tầm nhìn sản phẩm"),
                                        ("guide", "Guide"),
                                    ],
                                    active: match about_sub() {
                                        AboutSub::Overview => "overview",
                                        AboutSub::Terms => "terms",
                                        AboutSub::Privacy => "privacy",
                                        AboutSub::Community => "community",
                                        AboutSub::Vision => "vision",
                                        AboutSub::Guide => "guide",
                                    },
                                    on_select: move |id| match id {
                                        "terms" => set_sig_if_changed(about_sub, AboutSub::Terms),
                                        "privacy" => set_sig_if_changed(about_sub, AboutSub::Privacy),
                                        "community" => set_sig_if_changed(about_sub, AboutSub::Community),
                                        "vision" => set_sig_if_changed(about_sub, AboutSub::Vision),
                                        "guide" => set_sig_if_changed(about_sub, AboutSub::Guide),
                                        _ => set_sig_if_changed(about_sub, AboutSub::Overview),
                                    },
                                }
                                match about_sub() {
                                    AboutSub::Overview => rsx! {
                                        AboutOverviewPanel {
                                            has_keys: user_keys.is_some(),
                                            on_open_developer: move |_| {
                                                let bit = tab_bit(SettingsTab::Developer);
                                                let cur_mask = *mounted_tabs.peek();
                                                if (cur_mask & bit) == 0 {
                                                    mounted_tabs.set(cur_mask | bit);
                                                }
                                                set_sig_if_changed(tab, SettingsTab::Developer);
                                                set_sig_if_changed(dev_sub, DevSub::AdvancedExport);
                                            },
                                            on_open_guide: move |_| set_sig_if_changed(about_sub, AboutSub::Guide),
                                            on_replay_tour: move |_| on_replay_tour.call(()),
                                            on_open_terms: move |_| set_sig_if_changed(about_sub, AboutSub::Terms),
                                            on_open_privacy: move |_| set_sig_if_changed(about_sub, AboutSub::Privacy),
                                            on_open_community: move |_| set_sig_if_changed(about_sub, AboutSub::Community),
                                            on_open_vision: move |_| set_sig_if_changed(about_sub, AboutSub::Vision),
                                        }
                                    },
                                    AboutSub::Terms => rsx! {
                                        crate::components::legal::LegalPanel { kind: crate::components::legal::LegalDocKind::Terms }
                                    },
                                    AboutSub::Privacy => rsx! {
                                        crate::components::legal::LegalPanel { kind: crate::components::legal::LegalDocKind::Privacy }
                                    },
                                    AboutSub::Community => rsx! {
                                        crate::components::legal::LegalPanel { kind: crate::components::legal::LegalDocKind::Community }
                                    },
                                    AboutSub::Vision => rsx! {
                                        crate::components::legal::LegalPanel { kind: crate::components::legal::LegalDocKind::Vision }
                                    },
                                    AboutSub::Guide => rsx! { GuideSettings { on_replay_tour: move |_| on_replay_tour.call(()) } },
                                }
                            }

                            // 9. Developer (Conditional — ALL advanced/technical/complex features consolidated cleanly)
                            SettingsKeepAlivePane { tab, mounted_tabs, target: SettingsTab::Developer,
                                SubTabBar {
                                    items: vec![
                                        ("export", "Advanced export"),
                                        ("keys", "Encryption keys"),
                                        ("sync", "Sync & recovery"),
                                        ("extensions", "Extensions"),
                                        ("diagnostics", "Diagnostics"),
                                    ],
                                    active: match dev_sub() {
                                        DevSub::AdvancedExport => "export",
                                        DevSub::EncryptionKeys => "keys",
                                        DevSub::SyncRecovery => "sync",
                                        DevSub::Extensions => "extensions",
                                        DevSub::Diagnostics => "diagnostics",
                                    },
                                    on_select: move |id| match id {
                                        "keys" => set_sig_if_changed(dev_sub, DevSub::EncryptionKeys),
                                        "sync" => set_sig_if_changed(dev_sub, DevSub::SyncRecovery),
                                        "extensions" => set_sig_if_changed(dev_sub, DevSub::Extensions),
                                        "diagnostics" => set_sig_if_changed(dev_sub, DevSub::Diagnostics),
                                        _ => set_sig_if_changed(dev_sub, DevSub::AdvancedExport),
                                    },
                                }
                                match dev_sub() {
                                    DevSub::AdvancedExport => rsx! {
                                        crate::components::backup_cloud_ui::AdvancedSecurityExportSection {}
                                    },
                                    DevSub::EncryptionKeys => rsx! {
                                        div { class: "settings-cols",
                                            div { class: "settings-col",
                                                div { class: "settings-group",
                                                    div { class: "settings-group-title", "Security fingerprint" }
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
                                                    } else {
                                                        div { class: "sidebar-empty", "No key yet" }
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
                                            }
                                        }
                                    },
                                    DevSub::SyncRecovery => rsx! {
                                        crate::components::storage_ui::SyncRecoverySettingsPanel {
                                            current_user_id: current_user_id.clone(),
                                            user_keys: user_keys.clone(),
                                            on_keys_imported: on_keys_imported,
                                        }
                                    },
                                    DevSub::Extensions => rsx! {
                                        BotsSettings { my_username: current_username.clone() }
                                    },
                                    DevSub::Diagnostics => rsx! {
                                        DeveloperDiagnosticsPanel {
                                            current_user_id: current_user_id.clone(),
                                            has_keys: user_keys.is_some(),
                                            on_reset_appearance: move |_| {
                                                let d = platform::Appearance::default();
                                                prefs_ctx.update(|p| p.set_appearance(&d));
                                            },
                                            on_disable_dev: move |_| {
                                                prefs_ctx.update(|p| p.developer_mode = false);
                                                let bit = tab_bit(SettingsTab::About);
                                                let cur_mask = *mounted_tabs.peek();
                                                if (cur_mask & bit) == 0 {
                                                    mounted_tabs.set(cur_mask | bit);
                                                }
                                                set_sig_if_changed(tab, SettingsTab::About);
                                                set_sig_if_changed(about_sub, AboutSub::Overview);
                                            },
                                        }
                                    },
                                }
                            }
                    }
                }
            }
        }
    }
}

/// Live E2EE device key status & 1-click sync/reset card rendered inside
/// Settings > Privacy & security > Security (Spec 076).
#[component]
fn DeviceEncryptionKeyCard(
    current_user_id: String,
    user_keys: Option<UserKeys>,
    on_keys_imported: EventHandler<UserKeys>,
) -> Element {
    let mut server_fp = use_signal(|| Option::<String>::None);
    let mut device_trust = use_signal(|| Option::<String>::None);
    let mut checked = use_signal(|| false);
    let mut confirm_reset = use_signal(|| false);
    let mut need_totp = use_signal(|| false);
    let mut totp_input = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);

    let uid_eff = current_user_id.clone();
    let local_fp_key = user_keys.as_ref().map(|k| k.fingerprint_b64.clone()).unwrap_or_default();
    use_effect(use_reactive!(|uid_eff, local_fp_key| {
        let uid = uid_eff.clone();
        let _ = &local_fp_key;
        spawn(async move {
            let (srv_res, dev_res) = futures_util::join!(
                api::keys_get(&uid),
                crate::services::storage_sync::current_device_status()
            );
            server_fp.set(srv_res.ok().map(|k| k.fingerprint));
            let st = dev_res
                .ok()
                .and_then(|v| {
                    v.get("device")
                        .and_then(|d| d.get("trust_state"))
                        .and_then(|s| s.as_str())
                        .map(str::to_string)
                })
                .or_else(|| crate::storage::DEVICE_TRUST_STATE.peek().as_ref().map(|d| d.trust_state.clone()));
            device_trust.set(st);
            checked.set(true);
        });
    }));

    let local_matches_server = match (&user_keys, server_fp()) {
        (Some(k), Some(sfp)) => k.fingerprint_b64 == sfp || k.identity_fingerprint_b64 == sfp,
        (Some(_), None) => true,
        (None, _) => false,
    };
    let is_pending_device = device_trust().as_deref() == Some("pending")
        || crate::storage::DEVICE_TRUST_STATE()
            .as_ref()
            .is_some_and(|d| d.trust_state == "pending");
    let has_mismatch = checked() && (user_keys.is_none() || !local_matches_server || is_pending_device);
    let uid_reset = current_user_id.clone();

    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "End-to-end encryption" }
            div { class: "settings-row cw-brand-row",
                crate::components::brand_logos::E2eeLiveShield { size: 30 }
                div { class: "settings-row-text",
                    span { class: "settings-label", "Protection standard" }
                    span { class: "settings-sublabel", "Highest-grade end-to-end encryption" }
                }
                if has_mismatch {
                    span { class: "settings-value tag-danger", "Key mismatch" }
                } else if user_keys.is_some() {
                    span { class: "settings-value tag-active",
                        span { class: "cw-live-dot" }
                        "Active & secure"
                    }
                } else {
                    span { class: "settings-value tag-active",
                        span { class: "cw-live-dot" }
                        "No key yet"
                    }
                }
            }

            if has_mismatch && !confirm_reset() {
                div { class: "status-badge error",
                    "This device's encryption key does not match your account. Sync from your trusted device or reset this device's key to resume messaging."
                }
            }

            if let Some((is_err, text)) = notice() {
                div {
                    class: if is_err { "status-badge error" } else { "status-badge success" },
                    role: "status",
                    "{text}"
                }
            }

            if !confirm_reset() {
                div { class: "inline-form", style: "margin-top: var(--space-2); flex-wrap: wrap;",
                    if has_mismatch {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| {
                                *crate::storage::E2EE_RECOVERY_REQUIRED.write() = Some("server_key_mismatch".to_string());
                            },
                            "Sync from trusted device"
                        }
                    }
                    button {
                        r#type: "button",
                        class: if has_mismatch { "btn-danger-outline" } else { "btn-secondary" },
                        onclick: move |_| {
                            notice.set(None);
                            confirm_reset.set(true);
                        },
                        "Reset device key"
                    }
                }
            } else {
                div { style: "margin-top: var(--space-2); display: flex; flex-direction: column; gap: var(--space-2);",
                    div { class: "status-badge error",
                        "Generating a new key lets this device send and receive new messages immediately. Old messages encrypted with a previous key cannot be decrypted without that key or a backup."
                    }
                    if need_totp() {
                        input {
                            r#type: "text",
                            maxlength: "12",
                            placeholder: "6-digit code or backup code",
                            aria_label: "Current code or backup code",
                            value: "{totp_input}",
                            oninput: move |e| totp_input.set(e.value()),
                        }
                    }
                    div { class: "inline-form",
                        button {
                            r#type: "button",
                            class: "btn-danger",
                            disabled: busy(),
                            onclick: move |_| {
                                if busy() {
                                    return;
                                }
                                let uid = uid_reset.clone();
                                let code_now = totp_input();
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    let code_opt = if code_now.trim().is_empty() { None } else { Some(code_now.trim()) };
                                    match crate::components::storage_ui::reset_device_encryption_key(&uid, code_opt).await {
                                        Ok(new_keys) => {
                                            server_fp.set(Some(new_keys.fingerprint_b64.clone()));
                                            device_trust.set(Some("trusted".into()));
                                            confirm_reset.set(false);
                                            need_totp.set(false);
                                            totp_input.set(String::new());
                                            notice.set(Some((false, "Device encryption key reset. You can send messages now.".into())));
                                            on_keys_imported.call(new_keys);
                                        }
                                        Err(e) => {
                                            if e.contains("Two-factor code required") {
                                                need_totp.set(true);
                                            }
                                            notice.set(Some((true, e)));
                                        }
                                    }
                                    busy.set(false);
                                });
                            },
                            "Confirm reset key"
                        }
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            disabled: busy(),
                            onclick: move |_| {
                                confirm_reset.set(false);
                                need_totp.set(false);
                                notice.set(None);
                            },
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
}
