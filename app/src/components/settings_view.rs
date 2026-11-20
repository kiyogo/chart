use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        details_panel::format_fingerprint,
        guide::GuideSettings,
        settings_extra::{AccountSecurity, CustomizationSettings, LibrarySettings},
        settings_more::{AdvancedLookSettings, AutoReplySettings, ChatPrefsSettings, LanguageSettings, LockSettings, NotificationSettings, PrivacySettingsPanel},
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
    ("Tài khoản", SettingsTab::Account, "Tài khoản"),
    ("Tài khoản", SettingsTab::Privacy, "Quyền riêng tư"),
    ("Tài khoản", SettingsTab::Security, "Bảo mật"),
    ("Tài khoản", SettingsTab::Lock, "Khóa PIN"),
    ("Tài khoản", SettingsTab::Session, "Thiết bị"),
    ("Tài khoản", SettingsTab::Storage, "Lưu trữ"),
    ("Tài khoản", SettingsTab::SyncRecovery, "Đồng bộ"),
    ("Tài khoản", SettingsTab::Backup, "Sao lưu"),
    ("Tài khoản", SettingsTab::Cloud, "Đám mây"),
    ("Ứng dụng", SettingsTab::Chat, "Trò chuyện"),
    ("Ứng dụng", SettingsTab::Notifications, "Thông báo"),
    ("Ứng dụng", SettingsTab::AutoReply, "Trả lời tự động"),
    ("Ứng dụng", SettingsTab::Appearance, "Giao diện"),
    ("Ứng dụng", SettingsTab::Customize, "Tùy chỉnh"),
    ("Ứng dụng", SettingsTab::Language, "Ngôn ngữ"),
    ("Sáng tạo", SettingsTab::Stickers, "Thư viện"),
    ("Sáng tạo", SettingsTab::Studio, "Studio"),
    ("Sáng tạo", SettingsTab::Inventory, "Kho đồ"),
    ("Sáng tạo", SettingsTab::Demo, "Demo"),
    ("Khác", SettingsTab::Business, "Kinh doanh"),
    ("Khác", SettingsTab::Bots, "Plugin"),
    ("Khác", SettingsTab::Guide, "Hướng dẫn"),
];

fn tab_label(t: SettingsTab) -> &'static str {
    NAV.iter().find(|x| x.1 == t).map(|x| x.2).unwrap_or("")
}

/// Extra search words for each settings page (folded, Vietnamese and English).
fn tab_keywords(t: SettingsTab) -> &'static str {
    match t {
        SettingsTab::Account => "tai khoan ho so avatar anh dai dien anh bia ten username profile avatar studio hieu ung trang thai danh xung website so thich",
        SettingsTab::Privacy => "quyen rieng tu chan block da xem dang soan ket ban goi privacy",
        SettingsTab::Security => "bao mat ma hoa khoa sao luu e2ee van tay backup security xac minh hai buoc 2fa totp ma du phong nhat ky tai khoan da tai khoan chuyen tai khoan",
        SettingsTab::Lock => "khoa pin ma pin lock",
        SettingsTab::Session => "thiet bi phien dang xuat device session",
        SettingsTab::Storage => "luu tru bo nho dem quota opfs indexeddb s3 minio",
        SettingsTab::SyncRecovery => "dong bo khoi phuc da thiet bi backup webrtc",
        SettingsTab::Backup => "sao luu xuat du lieu khoi phuc kdata zip export import restore",
        SettingsTab::Cloud => "dam may cloud google drive onedrive dropbox nextcloud webdav s3 minio nas snapshot",
        SettingsTab::Chat => "tro chuyen goi y sticker dan excel bang enter tra loi bam dup chat",
        SettingsTab::Notifications => "thong bao am thanh am bao notification sound",
        SettingsTab::AutoReply => "tra loi tu dong vang mat auto reply",
        SettingsTab::Appearance => "giao dien chu de toi sang mau nhan co chu theme dark light",
        SettingsTab::Customize => "tuy chinh mau bong bong hinh nen phong chu dong bo thiet bi hieu ung avatar danh sach sync wallpaper",
        SettingsTab::Language => "ngon ngu language tieng anh",
        SettingsTab::Stickers => "thu vien sticker emoji gif tai len upload library",
        SettingsTab::Business => "kinh doanh ban hang san pham business",
        SettingsTab::Bots => "plugin mod bot token api giao dien",
        SettingsTab::Guide => "huong dan phim tat cau hoi tro giup help faq",
        SettingsTab::Inventory => "kho vat pham inventory thanh tich achievement huy hieu badge sticker pack emoji gif ban ve trang bi equipped template bo cuc",
        SettingsTab::Studio => "studio ve hoat anh animation drawing layer lop khung hinh gif sticker emoji avatar anh bia ve tay",
        SettingsTab::Demo => "demo thu hieu ung avatar xoay ten bang ten nameplate hinh nen playground gallery",
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
            header { class: "chat-header",
                button {
                    r#type: "button",
                    class: "icon-btn",
                    aria_label: "Đóng cài đặt",
                    onclick: move |_| {
                        // On phones the detail page is a separate screen: go back to the list first.
                        spawn(async move {
                            if has_detail() && platform::get_viewport_width().await < 768 {
                                has_detail.set(false);
                            } else {
                                on_close.call(());
                            }
                        });
                    },
                    BackIcon {}
                }
                h2 { class: "chat-header-title", "Cài đặt" }
            }

            div { class: if has_detail() { "settings-page has-detail" } else { "settings-page" },
                // Left Settings Nav (on mobile this is the first screen)
                nav { class: "settings-nav", aria_label: "Danh mục cài đặt",
                    div { class: "settings-search search-field",
                        crate::components::icons::SearchIcon { size: 16 }
                        input { r#type: "search", placeholder: "Tìm cài đặt", aria_label: "Tìm cài đặt", value: "{nav_query}",
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
                        div { class: "sidebar-empty", "Không tìm thấy cài đặt phù hợp" }
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
                    title: "Kéo để thay đổi kích thước danh mục",
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
                                        "Chỉnh sửa hồ sơ"
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
                                                                Err(_) => avatar_error.set(Some("Không thể cập nhật ảnh đại diện".into())),
                                                            },
                                                            Err(e) if e == "no_file" => {}
                                                            Err(_) => avatar_error.set(Some("Không thể xử lý tệp ảnh".into())),
                                                        }
                                                        is_uploading_avatar.set(false);
                                                    });
                                                },
                                            }
                                            if is_uploading_avatar() { "Đang tải ảnh" } else { "Đổi ảnh đại diện" }
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
                                                                Ok(_) => banner_status.set(Some((false, "Đã lưu ảnh bìa hồ sơ".into()))),
                                                                Err(_) => banner_status.set(Some((true, "Không thể tải ảnh bìa".into()))),
                                                            },
                                                            Err(e) if e == "no_file" => {}
                                                            Err(_) => banner_status.set(Some((true, "Không thể xử lý ảnh bìa".into()))),
                                                        }
                                                        is_uploading_banner.set(false);
                                                    });
                                                },
                                            }
                                            if is_uploading_banner() { "Đang tải bìa" } else { "Đổi ảnh bìa" }
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
                                    on_username_changed: move |n: String| if let Some(h) = on_username_changed { h.call(n) },
                                }
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Thông tin định danh" }
                                    div { class: "settings-list",
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Tên tài khoản" }
                                            span { class: "settings-value", "{current_username}" }
                                        }
                                        div { class: "settings-row column-layout",
                                            span { class: "settings-label", "Mã định danh" }
                                            span { class: "settings-value mono", "{current_user_id}" }
                                        }
                                    }
                                }
                            },

                            SettingsTab::Appearance => rsx! {
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Chủ đề" }
                                    div { class: "theme-preview", role: "radiogroup", aria_label: "Chủ đề",
                                        for (val, label) in [("light", "Sáng"), ("dark", "Tối"), ("system", "Theo hệ thống")] {
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
                                                        "light" => "background: #F7F8FA;",
                                                        "dark" => "background: #111419;",
                                                        _ => "background: linear-gradient(90deg, #F7F8FA 50%, #111419 50%);",
                                                    },
                                                    i { style: "width: 28%; background: var(--accent);" }
                                                    i { style: "flex: 1; background: rgba(127,127,127,0.25);" }
                                                }
                                                span { style: "display: inline-flex; align-items: center; gap: 6px;",
                                                    match val {
                                                        "light" => rsx! { SunIcon { size: 15 } },
                                                        "dark" => rsx! { MoonIcon { size: 15 } },
                                                        _ => rsx! { MonitorIcon { size: 15 } },
                                                    }
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Màu nhấn" }
                                    div { class: "settings-group-desc", "Áp dụng cho nút chính, bong bóng tin nhắn đã gửi và mục đang chọn." }
                                    div { class: "accent-swatches", role: "radiogroup", aria_label: "Màu nhấn",
                                        for (val, color, label) in [
                                            ("blue", "#2764E7", "Xanh dương"),
                                            ("violet", "#6D4AE0", "Tím"),
                                            ("green", "#138A5A", "Xanh lá"),
                                            ("orange", "#D9620F", "Cam"),
                                            ("rose", "#D6336C", "Hồng"),
                                        ] {
                                            button {
                                                key: "{val}",
                                                r#type: "button",
                                                role: "radio",
                                                title: "{label}",
                                                aria_label: "{label}",
                                                aria_checked: "{appearance().accent == val}",
                                                class: if appearance().accent == val { "accent-swatch active" } else { "accent-swatch" },
                                                style: "background: {color};",
                                                onclick: move |_| update_appearance(&|a| a.accent = val.to_string()),
                                                if appearance().accent == val { CheckIcon { size: 16 } }
                                            }
                                        }
                                    }
                                }

                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Chữ và mật độ" }
                                    div { class: "settings-list",
                                        div { class: "settings-row column-layout",
                                            div { class: "settings-row-text",
                                                span { class: "settings-label", "Cỡ chữ" }
                                                span { class: "settings-sublabel", "Tự động tăng nhẹ trên màn hình 2K/4K. Không dùng phóng to toàn trang." }
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
                                                    "Tự động"
                                                }
                                                input {
                                                    r#type: "range",
                                                    min: "13",
                                                    max: "20",
                                                    step: "1",
                                                    aria_label: "Cỡ chữ",
                                                    disabled: appearance().font_size == "auto",
                                                    value: if appearance().font_size == "auto" { "15".to_string() } else { appearance().font_size },
                                                    oninput: move |e| {
                                                        let v = e.value();
                                                        update_appearance(&|a| a.font_size = v.clone());
                                                    },
                                                }
                                                span { class: "range-value",
                                                    if appearance().font_size == "auto" { "Tự động" } else { "{appearance().font_size}px" }
                                                }
                                            }
                                        }
                                        div { class: "settings-row",
                                            div { class: "settings-row-text",
                                                span { class: "settings-label", "Mật độ hiển thị" }
                                                span { class: "settings-sublabel", "Khoảng cách danh sách, tin nhắn và kích thước nút." }
                                            }
                                            div { class: "segmented", role: "radiogroup", aria_label: "Mật độ hiển thị",
                                                for (val, label) in [("compact", "Gọn"), ("comfortable", "Vừa"), ("spacious", "Rộng")] {
                                                    button {
                                                        key: "{val}",
                                                        r#type: "button",
                                                        role: "radio",
                                                        aria_checked: "{appearance().density == val}",
                                                        class: if appearance().density == val { "active" } else { "" },
                                                        onclick: move |_| update_appearance(&|a| a.density = val.to_string()),
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Xem trước" }
                                    div { class: "settings-preview-chat",
                                        div { class: "message-row received group-start group-end",
                                            div { class: "message-content-wrapper",
                                                div { class: "message-bubble", "Bố cục mới dễ đọc hơn nhiều trên màn hình lớn." span { class: "bubble-meta", "09:41" } }
                                            }
                                        }
                                        div { class: "message-row sent group-start group-end",
                                            div { class: "message-content-wrapper",
                                                div { class: "message-bubble", "Đồng ý, cỡ chữ và màu nhấn đổi ngay lập tức." span { class: "bubble-meta", "09:42" } }
                                            }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        style: "align-self: flex-start;",
                                        onclick: move |_| {
                                            let d = platform::Appearance::default();
                                            platform::apply_appearance(&d);
                                            appearance.set(d);
                                        },
                                        "Khôi phục mặc định"
                                    }
                                }
                            },

                            SettingsTab::Security => rsx! {
                                crate::components::accounts::SecuritySettings {}
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Mã hóa đầu cuối" }
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Thuật toán trao đổi khóa & mã hóa" }
                                        span { class: "settings-value", "ECDH P-256 / HKDF-SHA-256 / AES-256-GCM" }
                                    }
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Chữ ký xác thực người gửi" }
                                        span { class: "settings-value", "ECDSA P-256 / SHA-256" }
                                    }
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Trạng thái khóa thiết bị" }
                                        span { class: "settings-value tag-active",
                                            if user_keys.is_some() { "Đã kích hoạt cục bộ" } else { "Chưa có khóa" }
                                        }
                                    }
                                    if let Some(ref keys) = user_keys {
                                        div { class: "settings-row column-layout",
                                            div { style: "display: flex; justify-content: space-between; width: 100%; align-items: center;",
                                                span { class: "settings-label", "Vân tay khóa công khai" }
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
                                                    if copied_fp() { "Đã sao chép" } else { "Sao chép" }
                                                }
                                            }
                                            div { class: "fingerprint", "{format_fingerprint(&keys.fingerprint_b64)}" }
                                        }
                                    }
                                }

                                // Client-only E2EE Key Backup & Recovery
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Sao lưu & Khôi phục khóa cục bộ" }
                                    div { class: "settings-row column-layout",
                                        div { style: "font-size: 13px; color: var(--text-secondary);",
                                            "Khóa riêng tư chỉ lưu trên thiết bị này, không bao giờ gửi lên máy chủ. Bản sao lưu chứa khóa riêng tư ở dạng chưa mã hóa: chỉ lưu ở nơi an toàn, không gửi qua tin nhắn hay email."
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
                                                span { "Xuất & sao chép khóa sao lưu" }
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
                                        label { class: "form-label", "Khôi phục từ bản sao lưu JSON" }
                                        textarea {
                                            style: "width: 100%; height: 68px; font-family: var(--font-mono); font-size: 11px;",
                                            placeholder: "Dán chuỗi JSON khóa sao lưu tại đây",
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
                                                                import_status.set(Some((false, "Đã khôi phục khóa E2EE thành công".into())));
                                                            });
                                                        }
                                                        Err(_) => {
                                                            import_status.set(Some((true, "Chuỗi khóa sao lưu không hợp lệ".into())));
                                                        }
                                                    }
                                                }
                                            },
                                            "Khôi phục khóa"
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
                            },

                            SettingsTab::Customize => rsx! { crate::components::settings_more::SyncMotionSettings {} CustomizationSettings {} AdvancedLookSettings {} },
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
                                crate::components::storage_ui::DeviceTrustPanel {}
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Quản lý phiên" }
                                    div { class: "settings-row",
                                        div {
                                            div { style: "font-weight: 600;", "Đăng xuất tài khoản" }
                                            div { style: "font-size: 12px; color: var(--text-secondary);",
                                                "Kết thúc phiên hiện tại trên trình duyệt này. Khóa E2EE cục bộ vẫn được giữ riêng cho tài khoản."
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-danger-outline",
                                            onclick: move |_| on_logout.call(()),
                                            LogoutIcon {}
                                            span { "Đăng xuất" }
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

