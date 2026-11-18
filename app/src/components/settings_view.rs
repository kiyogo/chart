use dioxus::prelude::*;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BackIcon, KeyIcon, LogoutIcon, MoonIcon, SettingsIcon, ShieldIcon, SunIcon},
    },
    crypto::e2ee::{UserKeys, export_user_keys_json, import_user_keys_json},
    platform,
    services::api,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Account,
    Appearance,
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
    let mut theme = use_signal(platform::load_saved_theme);
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
                    onclick: move |_| on_close.call(()),
                    BackIcon {}
                }
                h2 { class: "chat-header-title", "Cài đặt hệ thống" }
            }

            div { class: "settings-page",
                // Left Settings Nav
                nav { class: "settings-nav", aria_label: "Danh mục cài đặt",
                    div { class: "settings-nav-title", "Tài khoản" }
                    button {
                        r#type: "button",
                        class: if tab() == SettingsTab::Account { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| tab.set(SettingsTab::Account),
                        SettingsIcon {}
                        span { "Tài khoản & Hồ sơ" }
                    }
                    button {
                        r#type: "button",
                        class: if tab() == SettingsTab::Appearance { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| tab.set(SettingsTab::Appearance),
                        if theme() == "dark" { MoonIcon {} } else { SunIcon {} }
                        span { "Giao diện" }
                    }
                    div { class: "settings-nav-title", "Bảo mật" }
                    button {
                        r#type: "button",
                        class: if tab() == SettingsTab::Security { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| tab.set(SettingsTab::Security),
                        ShieldIcon { size: 16 }
                        span { "Mã hóa & Khóa E2EE" }
                    }
                    button {
                        r#type: "button",
                        class: if tab() == SettingsTab::Session { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| tab.set(SettingsTab::Session),
                        LogoutIcon {}
                        span { "Phiên đăng nhập" }
                    }
                }

                // Right Settings Body
                div { class: "settings-body",
                    div { class: "settings-content",
                        match tab() {
                            SettingsTab::Account => rsx! {
                                div { class: "settings-profile-card",
                                    Avatar {
                                        name: current_username.clone(),
                                        avatar_url: my_avatar_url.clone(),
                                        size: 60,
                                    }
                                    div { class: "settings-profile-info",
                                        div { class: "settings-profile-name", "{current_username}" }
                                        div { class: "settings-profile-sub", "Tài khoản cá nhân" }
                                        div { style: "display: flex; gap: 8px; margin-top: 6px; flex-wrap: wrap;",
                                            button {
                                                r#type: "button",
                                                class: "btn-primary",
                                                onclick: move |_| on_open_profile.call(()),
                                                "Chỉnh sửa hồ sơ"
                                            }
                                        }
                                    }
                                    div { class: "settings-profile-action",
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
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Tên tài khoản" }
                                        span { class: "settings-value", "{current_username}" }
                                    }
                                    div { class: "settings-row column-layout",
                                        span { class: "settings-label", "Mã định danh (UUID)" }
                                        span { class: "settings-value mono", "{current_user_id}" }
                                    }
                                }
                            },

                            SettingsTab::Appearance => rsx! {
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Chế độ màu sắc" }
                                    div { class: "settings-row",
                                        div {
                                            div { style: "font-weight: 600;", "Giao diện hiển thị" }
                                            div { style: "font-size: 12px; color: var(--text-secondary);",
                                                "Tùy chọn giao diện sáng mặc định hoặc tối chuyên nghiệp, lưu tự động trên thiết bị."
                                            }
                                        }
                                        div { style: "display: flex; gap: 8px;",
                                            button {
                                                r#type: "button",
                                                class: if theme() == "light" { "btn-primary" } else { "btn-secondary" },
                                                onclick: move |_| {
                                                    theme.set("light".into());
                                                    platform::apply_theme("light");
                                                },
                                                SunIcon { size: 15 }
                                                span { "Sáng" }
                                            }
                                            button {
                                                r#type: "button",
                                                class: if theme() == "dark" { "btn-primary" } else { "btn-secondary" },
                                                onclick: move |_| {
                                                    theme.set("dark".into());
                                                    platform::apply_theme("dark");
                                                },
                                                MoonIcon { size: 15 }
                                                span { "Tối" }
                                            }
                                        }
                                    }
                                }
                            },

                            SettingsTab::Security => rsx! {
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Mã hóa đầu cuối (E2EE)" }
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Thuật toán trao đổi khóa & mã hóa" }
                                        span { class: "settings-value nowrap", "ECDH P-256 / HKDF-SHA-256 / AES-256-GCM" }
                                    }
                                    div { class: "settings-row",
                                        span { class: "settings-label", "Chữ ký xác thực người gửi" }
                                        span { class: "settings-value nowrap", "ECDSA P-256 / SHA-256" }
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
                                            span { class: "settings-value mono", "{keys.fingerprint_b64}" }
                                        }
                                    }
                                }

                                // Client-only E2EE Key Backup & Recovery
                                div { class: "settings-group",
                                    div { class: "settings-group-title", "Sao lưu & Khôi phục khóa cục bộ" }
                                    div { class: "settings-row column-layout",
                                        div { style: "font-size: 13px; color: var(--text-secondary);",
                                            "Khóa riêng tư chỉ lưu trên thiết bị này, không bao giờ gửi lên máy chủ. Sao lưu khóa để đọc lại tin nhắn cũ khi đổi trình duyệt."
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
