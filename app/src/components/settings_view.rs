use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        details_panel::format_fingerprint,
        settings_extra::{CustomizationSettings, StickerSettings},
        icons::{
            BackIcon, CheckIcon, ChevronRightIcon, KeyIcon, LogoutIcon, MonitorIcon,
            MoonIcon, PaletteIcon, ShieldIcon, SlidersIcon, StickerIcon, SunIcon, UserIcon,
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
    Security,
    Session,
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
) -> Element {
    let mut tab = use_signal(|| SettingsTab::Account);
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
                    div { class: "settings-nav-title", "Người dùng" }
                    for (t, label) in [
                        (SettingsTab::Account, "Tài khoản của tôi"),
                        (SettingsTab::Appearance, "Giao diện"),
                        (SettingsTab::Customize, "Tùy chỉnh"),
                        (SettingsTab::Stickers, "Sticker"),
                        (SettingsTab::Security, "Bảo mật và mã hóa"),
                        (SettingsTab::Session, "Phiên đăng nhập"),
                    ] {
                        button {
                            key: "{label}",
                            r#type: "button",
                            class: if tab() == t { "settings-nav-item active" } else { "settings-nav-item" },
                            aria_current: if tab() == t { "page" } else { "false" },
                            onclick: move |_| { tab.set(t); has_detail.set(true); },
                            match t {
                                SettingsTab::Account => rsx! { UserIcon {} },
                                SettingsTab::Appearance => rsx! { PaletteIcon {} },
                                SettingsTab::Security => rsx! { ShieldIcon { size: 18 } },
                                SettingsTab::Session => rsx! { LogoutIcon {} },
                                SettingsTab::Customize => rsx! { SlidersIcon {} },
                                SettingsTab::Stickers => rsx! { StickerIcon {} },
                            }
                            span { "{label}" }
                            span { class: "chevron", ChevronRightIcon { size: 16 } }
                        }
                    }
                }

                // Right Settings Body
                div { class: "settings-body",
                    div { class: "settings-content",
                        div {
                            h1 { class: "settings-page-title",
                                match tab() {
                                    SettingsTab::Account => "Tài khoản của tôi",
                                    SettingsTab::Appearance => "Giao diện",
                                    SettingsTab::Security => "Bảo mật và mã hóa",
                                    SettingsTab::Session => "Phiên đăng nhập",
                                    SettingsTab::Customize => "Tùy chỉnh",
                                    SettingsTab::Stickers => "Sticker",
                                }
                            }
                            p { class: "settings-page-desc",
                                match tab() {
                                    SettingsTab::Account => "Ảnh đại diện, ảnh bìa và thông tin định danh của bạn.",
                                    SettingsTab::Appearance => "Chủ đề màu, màu nhấn, cỡ chữ và mật độ hiển thị. Lưu trên thiết bị này.",
                                    SettingsTab::Security => "Khóa mã hóa đầu cuối của thiết bị, vân tay khóa, sao lưu và khôi phục.",
                                    SettingsTab::Session => "Quản lý phiên đăng nhập trên trình duyệt này.",
                                    SettingsTab::Customize => "Cột điều hướng, hình nền trò chuyện, âm báo và hiệu ứng. Lưu riêng cho tài khoản trên thiết bị này.",
                                    SettingsTab::Stickers => "Tạo bộ sticker riêng và tải ảnh lên để gửi trong mọi cuộc trò chuyện.",
                                }
                            }
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

                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Thông tin định danh" }
                                    div { class: "settings-list",
                                        div { class: "settings-row",
                                            span { class: "settings-label", "Tên tài khoản" }
                                            span { class: "settings-value", "{current_username}" }
                                        }
                                        div { class: "settings-row column-layout",
                                            span { class: "settings-label", "Mã định danh (UUID)" }
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
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Mã hóa đầu cuối (E2EE)" }
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
                                                span { class: "settings-label", "Vân tay khóa công khai (SHA-256)" }
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
                            },

                            SettingsTab::Customize => rsx! { CustomizationSettings {} },
                            SettingsTab::Stickers => rsx! { StickerSettings { current_user_id: current_user_id.clone() } },
                            SettingsTab::Session => rsx! {
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

