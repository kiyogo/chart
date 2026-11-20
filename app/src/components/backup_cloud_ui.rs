use dioxus::prelude::*;
use serde_json::json;

use crate::{
    platform,
    storage::format_bytes_human,
    services::backup_cloud::{
        self, BackupJobEntry, CloudBackupPolicy, CloudBackupRunResult, CloudConnection,
        CloudSnapshotEntry, ExportPreview, ExportScopeOptions, InspectArchiveResponse,
        ProviderCapability, ReauthMethods, RestoreResult,
    },
};

fn format_api_error(code: &str) -> &'static str {
    match code {
        "reauth_required" | "export_reauth_required" => "Cần xác thực lại trước khi xuất dữ liệu.",
        "reauth_locked" => "Thử sai quá nhiều lần. Tài khoản xuất tạm khóa 15 phút.",
        "invalid_credentials" => "Mật khẩu hoặc mã xác thực không đúng.",
        "export_password_not_set" => "Bạn chưa thiết lập mật khẩu xuất dữ liệu.",
        "export_password_too_short" => "Mật khẩu xuất cần ít nhất 8 ký tự.",
        "encryption_password_too_short" => "Mật khẩu mã hóa cần ít nhất 6 ký tự.",
        "totp_not_enabled" => "Tài khoản chưa bật xác thực 2FA TOTP.",
        "unencrypted_zip_steps_incomplete" => "Chưa hoàn thành đủ các bước xác nhận mở khóa ZIP không mã hóa.",
        "export_token_expired"
        | "export_token_used"
        | "invalid_export_token"
        | "export_authorization_invalid_or_expired" => {
            "Phiên xác thực xuất dữ liệu đã hết hạn hoặc đã được sử dụng."
        }
        "wrong_password_or_corrupt_kdata" => "Sai mật khẩu giải mã hoặc tệp .kdata đã bị hỏng / chỉnh sửa.",
        "wrong_password" => "Mật khẩu giải mã không đúng.",
        "zip_encrypted_requires_password" => "Tệp ZIP được mã hóa AES 256, hãy nhập mật khẩu giải mã.",
        "cross_account_restore_denied" | "backup_belongs_to_another_account" => {
            "Không thể khôi phục tệp sao lưu thuộc tài khoản khác."
        }
        "corrupt_kdata" | "unsupported_archive_format" => "Định dạng tệp sao lưu không hợp lệ hoặc bị hỏng.",
        "zip_bomb_rejected" => "Tệp ZIP vượt quá giới hạn giải nén an toàn.",
        "unsafe_archive_path" | "unsafe_path_rejected" => "Tệp lưu trữ chứa đường dẫn không an toàn.",
        "cloud_provider_not_configured" => "Nhà cung cấp Cloud này chưa được quản trị viên cấu hình OAuth.",
        "invalid_cloud_endpoint_or_config" => "Địa chỉ kết nối Cloud không hợp lệ hoặc bị chặn bởi chính sách an toàn mạng.",
        _ => "Thao tác không thành công.",
    }
}

fn format_provider_label(id: &str) -> &'static str {
    match id {
        "google_drive" => "Google Drive",
        "onedrive" => "Microsoft OneDrive",
        "dropbox" => "Dropbox",
        "nextcloud" => "Nextcloud",
        "webdav" => "WebDAV",
        "s3" => "Amazon S3",
        "r2" => "Cloudflare R2",
        "minio" => "MinIO",
        "nas" => "NAS cá nhân",
        "icloud" => "Apple iCloud",
        _ => "Dịch vụ Cloud",
    }
}

#[component]
pub fn BackupExportRestorePanel() -> Element {
    let mut methods = use_signal(ReauthMethods::default);
    let mut preview = use_signal(ExportPreview::default);
    let mut jobs = use_signal(Vec::<BackupJobEntry>::new);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);

    // Scope & export credentials
    let mut include_account = use_signal(|| true);
    let mut include_messages = use_signal(|| true);
    let mut include_media = use_signal(|| true);
    let mut include_settings = use_signal(|| true);
    let mut conversation_id = use_signal(String::new);

    let mut reauth_method = use_signal(|| "password".to_string());
    let mut reauth_secret = use_signal(String::new);
    let mut enc_password = use_signal(String::new);
    let mut enc_confirm = use_signal(String::new);

    // Quick inline export password setup when none configured yet
    let mut new_export_pw = use_signal(String::new);

    // Restore state
    let mut selected_file = use_signal(|| Option::<platform::SelectedFileMeta>::None);
    let mut restore_password = use_signal(String::new);
    let mut inspected = use_signal(|| Option::<InspectArchiveResponse>::None);
    let mut restore_mode = use_signal(|| "full".to_string());
    let mut restore_conv_id = use_signal(String::new);
    let mut restore_result = use_signal(|| Option::<RestoreResult>::None);

    let reload = move || {
        spawn(async move {
            if let Ok(m) = backup_cloud::get_reauth_methods().await {
                if !m.has_export_password && m.totp_enabled {
                    reauth_method.set("totp".into());
                } else if !m.has_export_password && m.backup_codes_remaining > 0 {
                    reauth_method.set("backup_code".into());
                }
                methods.set(m);
            }
            if let Ok(p) = backup_cloud::get_export_preview().await {
                preview.set(p);
            }
            if let Ok(j) = backup_cloud::list_backup_jobs().await {
                jobs.set(j);
            }
        });
    };
    use_effect(move || reload());

    let m = methods();
    let p = preview();

    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Dữ liệu sẵn sàng sao lưu" }
            div { class: "settings-list",
                div { class: "settings-row",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Tin nhắn trong tài khoản" }
                        span { class: "settings-sublabel",
                            "Cá nhân: {p.dm_messages} · Nhóm: {p.group_messages} · Máy chủ: {p.channel_messages}"
                        }
                    }
                    span { class: "chip-privacy", "{p.durable_media_count} tệp · {format_bytes_human(p.durable_media_bytes as u64)}" }
                }
                if p.missing_or_client_only_count > 0 {
                    div { class: "settings-row",
                        div { class: "settings-row-text",
                            span { class: "settings-label", "Tệp chỉ nằm trên thiết bị khách" }
                            span { class: "settings-sublabel", "{p.honest_notice}" }
                        }
                    }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Bao gồm thông tin tài khoản" }
                    input { r#type: "checkbox", checked: include_account(), onchange: move |e| include_account.set(e.checked()) }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Bao gồm tin nhắn" }
                    input { r#type: "checkbox", checked: include_messages(), onchange: move |e| include_messages.set(e.checked()) }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Bao gồm ảnh, video, âm thanh và tệp" }
                    input { r#type: "checkbox", checked: include_media(), onchange: move |e| include_media.set(e.checked()) }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Bao gồm cài đặt cá nhân" }
                    input { r#type: "checkbox", checked: include_settings(), onchange: move |e| include_settings.set(e.checked()) }
                }
                div { class: "settings-row column-layout",
                    span { class: "settings-label", "Lọc riêng một cuộc trò chuyện (tùy chọn)" }
                    input {
                        class: "settings-input",
                        r#type: "text",
                        placeholder: "Để trống để xuất toàn bộ, hoặc nhập ID cuộc trò chuyện",
                        value: "{conversation_id}",
                        oninput: move |e| conversation_id.set(e.value()),
                    }
                }
            }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Xuất toàn bộ dữ liệu (.kdata)" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Định dạng chính thức KIYO (.kdata)" }
                        span { class: "settings-sublabel", "Mã hóa toàn phần Argon2id và XChaCha20-Poly1305 STREAM, chống giả mạo và hỗ trợ khôi phục chọn lọc." }
                    }
                    if !m.has_export_password && !m.totp_enabled && m.backup_codes_remaining == 0 {
                        div { class: "settings-row-text",
                            span { class: "settings-sublabel", "Bạn cần tạo mật khẩu xuất dữ liệu (tối thiểu 8 ký tự) để xác thực lại trước khi xuất." }
                        }
                        div { class: "inline-form",
                            input {
                                class: "settings-input",
                                id: "quick-export-pw-input",
                                r#type: "password",
                                placeholder: "Tạo mật khẩu xuất (>= 8 ký tự)",
                                value: "{new_export_pw}",
                                oninput: move |e| new_export_pw.set(e.value()),
                            }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                disabled: busy(),
                                onclick: move |_| {
                                    let pw = new_export_pw().trim().to_string();
                                    if pw.chars().count() < 8 {
                                        notice.set(Some((true, "Mật khẩu xuất cần ít nhất 8 ký tự.".into())));
                                        return;
                                    }
                                    busy.set(true);
                                    spawn(async move {
                                        match backup_cloud::set_export_password(None, None, &pw).await {
                                            Ok(()) => {
                                                new_export_pw.set(String::new());
                                                reauth_method.set("password".into());
                                                notice.set(Some((false, "Đã thiết lập mật khẩu xuất dữ liệu.".into())));
                                                reload();
                                            }
                                            Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "Lưu mật khẩu xuất"
                            }
                        }
                    } else {
                        div { class: "inline-form",
                            select {
                                class: "settings-select",
                                aria_label: "Phương thức xác thực xuất",
                                value: "{reauth_method}",
                                onchange: move |e| reauth_method.set(e.value()),
                                if m.has_export_password {
                                    option { value: "password", "Mật khẩu xuất dữ liệu" }
                                }
                                if m.totp_enabled {
                                    option { value: "totp", "Mã 2FA TOTP" }
                                }
                                if m.backup_codes_remaining > 0 {
                                    option { value: "backup_code", "Mã dự phòng ({m.backup_codes_remaining})" }
                                }
                            }
                            input {
                                class: "settings-input",
                                id: "kdata-reauth-secret",
                                r#type: "password",
                                placeholder: "Nhập mật khẩu xuất hoặc mã xác thực",
                                value: "{reauth_secret}",
                                oninput: move |e| reauth_secret.set(e.value()),
                            }
                        }
                    }
                    div { class: "inline-form",
                        input {
                            class: "settings-input",
                            id: "kdata-enc-password",
                            r#type: "password",
                            placeholder: "Mật khẩu mã hóa tệp .kdata (>= 6 ký tự)",
                            value: "{enc_password}",
                            oninput: move |e| enc_password.set(e.value()),
                        }
                        input {
                            class: "settings-input",
                            id: "kdata-enc-confirm",
                            r#type: "password",
                            placeholder: "Nhập lại mật khẩu mã hóa tệp",
                            value: "{enc_confirm}",
                            oninput: move |e| enc_confirm.set(e.value()),
                        }
                    }
                    div { class: "inline-form",
                        button {
                            r#type: "button",
                            id: "btn-export-kdata",
                            class: "btn-primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let method = reauth_method();
                                let secret = reauth_secret();
                                let epw = enc_password();
                                let econf = enc_confirm();
                                if secret.trim().is_empty() {
                                    notice.set(Some((true, "Vui lòng nhập thông tin xác thực lại tài khoản.".into())));
                                    return;
                                }
                                if epw.chars().count() < 6 {
                                    notice.set(Some((true, "Mật khẩu mã hóa tệp .kdata cần ít nhất 6 ký tự.".into())));
                                    return;
                                }
                                if epw != econf {
                                    notice.set(Some((true, "Hai lần nhập mật khẩu mã hóa tệp không khớp.".into())));
                                    return;
                                }
                                let scope = ExportScopeOptions {
                                    include_account: include_account(),
                                    include_messages: include_messages(),
                                    include_media: include_media(),
                                    include_settings: include_settings(),
                                    conversation_id: {
                                        let c = conversation_id().trim().to_string();
                                        if c.is_empty() { None } else { Some(c) }
                                    },
                                };
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    match backup_cloud::verify_reauth("kdata", &method, &secret).await {
                                        Ok(auth) => {
                                            match backup_cloud::export_kdata_download(&auth.export_token, &epw, &scope).await {
                                                Ok(dl) => {
                                                    reauth_secret.set(String::new());
                                                    notice.set(Some((
                                                        false,
                                                        format!("Đã tải tệp {} ({})", dl.filename, format_bytes_human(dl.byte_size)),
                                                    )));
                                                    reload();
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Đang xử lý xuất dữ liệu" } else { "Xuất toàn bộ dữ liệu (.kdata)" }
                        }
                    }
                }
            }
        }

        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Kiểm tra và khôi phục từ tệp sao lưu" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Chọn tệp .kdata hoặc .zip" }
                        span { class: "settings-sublabel", "Hệ thống kiểm tra chữ ký toàn vẹn, chống trùng lặp và cho phép chọn phần cần khôi phục." }
                    }
                    div { class: "inline-form",
                        label { class: "btn-secondary",
                            input {
                                id: "kdata-restore-file",
                                r#type: "file",
                                accept: ".kdata,.zip,application/octet-stream,application/zip",
                                style: "display: none;",
                                onchange: move |_| {
                                    inspected.set(None);
                                    restore_result.set(None);
                                    spawn(async move {
                                        selected_file.set(platform::selected_file_meta("kdata-restore-file").await);
                                    });
                                },
                            }
                            "Chọn tệp sao lưu"
                        }
                        if let Some(meta) = selected_file() {
                            span { class: "chip-privacy", "{meta.name} · {format_bytes_human(meta.byte_size)}" }
                        }
                    }
                    div { class: "inline-form",
                        input {
                            class: "settings-input",
                            id: "kdata-restore-password",
                            r#type: "password",
                            placeholder: "Mật khẩu giải mã (bắt buộc với .kdata hoặc ZIP mã hóa)",
                            value: "{restore_password}",
                            oninput: move |e| restore_password.set(e.value()),
                        }
                        button {
                            r#type: "button",
                            id: "btn-inspect-backup",
                            class: "btn-secondary",
                            disabled: busy() || selected_file().is_none(),
                            onclick: move |_| {
                                let pw = restore_password();
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    match backup_cloud::inspect_backup_input(
                                        "kdata-restore-file",
                                        if pw.is_empty() { None } else { Some(pw.as_str()) },
                                    ).await {
                                        Ok(info) => {
                                            inspected.set(Some(info));
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Kiểm tra tệp sao lưu"
                        }
                    }
                    if let Some(info) = inspected() {
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label",
                                    "Định dạng: {info.detected_format} · {info.total_entries} mục · {format_bytes_human(info.total_plain_bytes)}"
                                }
                                span { class: "settings-sublabel",
                                    "Tin nhắn: {info.new_messages} mới / {info.duplicate_messages} trùng · Media: {info.new_media} mới / {info.duplicate_media} trùng"
                                }
                            }
                            if info.owner_mismatch {
                                div { class: "status-badge error", "Cảnh báo: tệp này thuộc về tài khoản khác nên sẽ bị chặn khôi phục." }
                            }
                            for w in info.warnings.iter() {
                                div { class: "settings-sublabel", "{w}" }
                            }
                        }
                    }
                    div { class: "inline-form",
                        select {
                            class: "settings-select",
                            aria_label: "Chế độ khôi phục",
                            value: "{restore_mode}",
                            onchange: move |e| restore_mode.set(e.value()),
                            option { value: "full", "Khôi phục toàn bộ" }
                            option { value: "missing_only", "Chỉ bổ sung phần bị thiếu" }
                            option { value: "messages", "Chỉ tin nhắn" }
                            option { value: "images", "Chỉ hình ảnh" }
                            option { value: "videos", "Chỉ video" }
                            option { value: "audio", "Chỉ tin nhắn thoại và âm thanh" }
                            option { value: "files", "Chỉ tệp đính kèm" }
                            option { value: "avatar", "Chỉ ảnh đại diện" }
                            option { value: "settings", "Chỉ cài đặt" }
                            option { value: "single_conversation", "Chỉ một cuộc trò chuyện" }
                        }
                        if restore_mode() == "single_conversation" {
                            input {
                                class: "settings-input",
                                r#type: "text",
                                placeholder: "Nhập ID cuộc trò chuyện",
                                value: "{restore_conv_id}",
                                oninput: move |e| restore_conv_id.set(e.value()),
                            }
                        }
                        button {
                            r#type: "button",
                            id: "btn-restore-backup",
                            class: "btn-primary",
                            disabled: busy() || selected_file().is_none(),
                            onclick: move |_| {
                                let pw = restore_password();
                                let mode = restore_mode();
                                let cid = restore_conv_id();
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    match backup_cloud::restore_backup_input(
                                        "kdata-restore-file",
                                        if pw.is_empty() { None } else { Some(pw.as_str()) },
                                        &mode,
                                        if cid.trim().is_empty() { None } else { Some(cid.trim()) },
                                    ).await {
                                        Ok(res) => {
                                            notice.set(Some((
                                                false,
                                                format!("Khôi phục hoàn tất: {} tin nhắn, {} tệp phương tiện.", res.restored_messages, res.restored_media),
                                            )));
                                            restore_result.set(Some(res));
                                            reload();
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Khôi phục dữ liệu"
                        }
                    }
                    if let Some(rr) = restore_result() {
                        div { class: "settings-row-text",
                            span { class: "settings-label",
                                "Kết quả: +{rr.restored_messages} tin nhắn (bỏ qua {rr.skipped_duplicate_messages} trùng), +{rr.restored_media} tệp (bỏ qua {rr.skipped_duplicate_media} trùng)"
                            }
                            if rr.skipped_unauthorized_messages > 0 {
                                span { class: "settings-sublabel", "Đã bỏ qua {rr.skipped_unauthorized_messages} bản ghi không hợp lệ quyền." }
                            }
                        }
                    }
                }
            }
        }

        if !jobs().is_empty() {
            div { class: "settings-group",
                div { class: "settings-group-title", "Lịch sử tiến trình sao lưu" }
                div { class: "settings-list",
                    for j in jobs() {
                        div { key: "{j.id}", class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "{j.format} · {j.status}" }
                                span { class: "settings-sublabel",
                                    "{format_bytes_human(j.processed_bytes as u64)} / {format_bytes_human(j.total_bytes as u64)} · {j.processed_files}/{j.total_files} tệp"
                                    if let Some(ref c) = j.checksum_sha256 {
                                        " · SHA-256: {&c[..c.len().min(12)]}"
                                    }
                                }
                                if let Some(ref n) = j.honest_notice {
                                    span { class: "settings-sublabel", "{n}" }
                                }
                            }
                            if j.status == "running" || j.status == "paused" {
                                div { class: "inline-form",
                                    button {
                                        r#type: "button",
                                        class: "btn-ghost",
                                        onclick: {
                                            let jid = j.id.clone();
                                            let act = if j.status == "paused" { "resume" } else { "pause" };
                                            move |_| {
                                                let jid = jid.clone();
                                                spawn(async move {
                                                    let _ = backup_cloud::control_backup_job(&jid, act).await;
                                                    reload();
                                                });
                                            }
                                        },
                                        if j.status == "paused" { "Tiếp tục" } else { "Tạm dừng" }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-danger-outline",
                                        onclick: {
                                            let jid = j.id.clone();
                                            move |_| {
                                                let jid = jid.clone();
                                                spawn(async move {
                                                    let _ = backup_cloud::control_backup_job(&jid, "cancel").await;
                                                    reload();
                                                });
                                            }
                                        },
                                        "Hủy"
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

#[component]
pub fn CloudAccountsPanel() -> Element {
    let mut providers = use_signal(Vec::<ProviderCapability>::new);
    let mut connections = use_signal(Vec::<CloudConnection>::new);
    let mut policy = use_signal(CloudBackupPolicy::default);
    let mut active_conn_id = use_signal(|| Option::<String>::None);
    let mut snapshots = use_signal(Vec::<CloudSnapshotEntry>::new);
    let mut last_run = use_signal(|| Option::<CloudBackupRunResult>::None);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);

    // Connect credentials form state
    let mut connect_provider = use_signal(|| Option::<String>::None);
    let mut conn_label = use_signal(String::new);
    let mut dav_url = use_signal(String::new);
    let mut dav_user = use_signal(String::new);
    let mut dav_pass = use_signal(String::new);
    let mut dav_folder = use_signal(|| "kiyo_backups".to_string());
    let mut s3_endpoint = use_signal(String::new);
    let mut s3_region = use_signal(|| "us-east-1".to_string());
    let mut s3_bucket = use_signal(String::new);
    let mut s3_ak = use_signal(String::new);
    let mut s3_sk = use_signal(String::new);
    let mut s3_prefix = use_signal(|| "kiyo_backups".to_string());
    let mut s3_path_style = use_signal(|| true);

    // Cloud passphrase for chunk encryption & snapshot restore
    let mut cloud_passphrase = use_signal(String::new);
    let mut restore_mode = use_signal(|| "full".to_string());

    let reload = move || {
        spawn(async move {
            if let Ok(p) = backup_cloud::list_cloud_providers().await {
                providers.set(p);
            }
            if let Ok(c) = backup_cloud::list_cloud_connections().await {
                if active_conn_id().is_none()
                    && let Some(first) = c.first()
                {
                    let cid = first.id.clone();
                    active_conn_id.set(Some(cid.clone()));
                    if let Ok(s) = backup_cloud::list_cloud_snapshots(&cid).await {
                        snapshots.set(s);
                    }
                }
                connections.set(c);
            }
            if let Ok(pol) = backup_cloud::get_cloud_policy().await {
                policy.set(pol);
            }
        });
    };
    use_effect(move || reload());

    let load_snapshots = move |cid: String| {
        let mut active_conn_sig = active_conn_id;
        let mut snaps_sig = snapshots;
        active_conn_sig.set(Some(cid.clone()));
        spawn(async move {
            if let Ok(s) = backup_cloud::list_cloud_snapshots(&cid).await {
                snaps_sig.set(s);
            }
        });
    };

    let pol = policy();

    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Khóa mã hóa sao lưu đám mây" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Mật khẩu mã hóa phân mảnh kchunk_v1" }
                        span { class: "settings-sublabel", "Mọi tệp và danh mục được mã hóa đầu cuối trước khi tải lên đám mây, không lưu mật khẩu trên máy chủ." }
                    }
                    input {
                        class: "settings-input",
                        id: "cloud-backup-passphrase",
                        r#type: "password",
                        placeholder: "Nhập mật khẩu sao lưu Cloud (>= 6 ký tự)",
                        value: "{cloud_passphrase}",
                        oninput: move |e| cloud_passphrase.set(e.value()),
                    }
                }
            }
        }

        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Nhà cung cấp lưu trữ đám mây" }
            div { class: "settings-list",
                for prov in providers() {
                    div { key: "{prov.provider_id}", class: "settings-row",
                        div { class: "settings-row-text",
                            span { class: "settings-label", "{prov.display_name}" }
                            span { class: "settings-sublabel",
                                if prov.auth_kind == "oauth2_pkce" {
                                    if prov.oauth_configured {
                                        "OAuth 2.0 PKCE sẵn sàng kết nối"
                                    } else {
                                        "Quản trị viên cần cấu hình OAuth Client ID: {prov.setup_requirements.join(\", \")}"
                                    }
                                } else if prov.auth_kind == "icloud_platform" {
                                    "Chỉ hỗ trợ qua ứng dụng Apple iOS hoặc macOS"
                                } else {
                                    "Kết nối trực tiếp qua HTTPS và xác thực tài khoản"
                                }
                            }
                        }
                        if prov.auth_kind == "oauth2_pkce" {
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                disabled: busy() || !prov.oauth_configured,
                                onclick: {
                                    let pid = prov.provider_id.clone();
                                    move |_| {
                                        let pid = pid.clone();
                                        busy.set(true);
                                        spawn(async move {
                                            match backup_cloud::start_cloud_oauth(&pid).await {
                                                Ok(url) if !url.is_empty() => {
                                                    platform::open_external_url(&url);
                                                    notice.set(Some((false, "Đã mở trang xác thực OAuth của nhà cung cấp.".into())));
                                                }
                                                Ok(_) => notice.set(Some((true, "Không nhận được liên kết xác thực OAuth.".into()))),
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                                if prov.oauth_configured { "Kết nối OAuth" } else { "Chưa cấu hình" }
                            }
                        } else if prov.auth_kind == "credentials" {
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: {
                                    let pid = prov.provider_id.clone();
                                    move |_| {
                                        if connect_provider().as_deref() == Some(pid.as_str()) {
                                            connect_provider.set(None);
                                        } else {
                                            connect_provider.set(Some(pid.clone()));
                                        }
                                    }
                                },
                                if connect_provider().as_deref() == Some(prov.provider_id.as_str()) { "Đóng" } else { "Thiết lập" }
                            }
                        }
                    }
                }
            }
        }

        if let Some(ref pid) = connect_provider() {
            div { class: "settings-group",
                div { class: "settings-group-title", "Kết nối {format_provider_label(pid)}" }
                div { class: "settings-list",
                    div { class: "settings-row column-layout",
                        input {
                            class: "settings-input",
                            r#type: "text",
                            placeholder: "Tên gợi nhớ cho tài khoản Cloud",
                            value: "{conn_label}",
                            oninput: move |e| conn_label.set(e.value()),
                        }
                        if matches!(pid.as_str(), "nextcloud" | "webdav" | "nas") {
                            input {
                                class: "settings-input",
                                id: "cloud-dav-url",
                                r#type: "text",
                                placeholder: "URL WebDAV HTTPS (ví dụ https://cloud.example.com/remote.php/dav/files/user)",
                                value: "{dav_url}",
                                oninput: move |e| dav_url.set(e.value()),
                            }
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    id: "cloud-dav-user",
                                    r#type: "text",
                                    placeholder: "Tên đăng nhập",
                                    value: "{dav_user}",
                                    oninput: move |e| dav_user.set(e.value()),
                                }
                                input {
                                    class: "settings-input",
                                    id: "cloud-dav-pass",
                                    r#type: "password",
                                    placeholder: "Mật khẩu hoặc App Password",
                                    value: "{dav_pass}",
                                    oninput: move |e| dav_pass.set(e.value()),
                                }
                            }
                            input {
                                class: "settings-input",
                                r#type: "text",
                                placeholder: "Thư mục đích (mặc định kiyo_backups)",
                                value: "{dav_folder}",
                                oninput: move |e| dav_folder.set(e.value()),
                            }
                        } else {
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Endpoint HTTPS (ví dụ https://s3.us-east-1.amazonaws.com)",
                                    value: "{s3_endpoint}",
                                    oninput: move |e| s3_endpoint.set(e.value()),
                                }
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Region (us-east-1 hoặc auto)",
                                    value: "{s3_region}",
                                    oninput: move |e| s3_region.set(e.value()),
                                }
                            }
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Bucket",
                                    value: "{s3_bucket}",
                                    oninput: move |e| s3_bucket.set(e.value()),
                                }
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Prefix thư mục (kiyo_backups)",
                                    value: "{s3_prefix}",
                                    oninput: move |e| s3_prefix.set(e.value()),
                                }
                            }
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Access Key",
                                    value: "{s3_ak}",
                                    oninput: move |e| s3_ak.set(e.value()),
                                }
                                input {
                                    class: "settings-input",
                                    r#type: "password",
                                    placeholder: "Secret Key",
                                    value: "{s3_sk}",
                                    oninput: move |e| s3_sk.set(e.value()),
                                }
                            }
                            label { class: "settings-row",
                                span { class: "settings-label", "Dùng Path Style (phù hợp MinIO)" }
                                input { r#type: "checkbox", checked: s3_path_style(), onchange: move |e| s3_path_style.set(e.checked()) }
                            }
                        }
                        div { class: "inline-form",
                            button {
                                r#type: "button",
                                id: "btn-connect-cloud",
                                class: "btn-primary",
                                disabled: busy(),
                                onclick: {
                                    let pid = pid.clone();
                                    move |_| {
                                        let pid = pid.clone();
                                        let label = conn_label();
                                        let cfg = if matches!(pid.as_str(), "nextcloud" | "webdav" | "nas") {
                                            json!({
                                                "kind": pid,
                                                "endpoint": dav_url().trim(),
                                                "username": dav_user().trim(),
                                                "password": dav_pass(),
                                                "folder": dav_folder().trim(),
                                                "allow_lan": pid == "nas",
                                            })
                                        } else {
                                            json!({
                                                "kind": pid,
                                                "endpoint": s3_endpoint().trim(),
                                                "region": s3_region().trim(),
                                                "bucket": s3_bucket().trim(),
                                                "access_key": s3_ak().trim(),
                                                "secret_key": s3_sk(),
                                                "prefix": s3_prefix().trim(),
                                                "force_path_style": s3_path_style(),
                                            })
                                        };
                                        busy.set(true);
                                        notice.set(None);
                                        spawn(async move {
                                            match backup_cloud::connect_cloud_credentials(&pid, &label, cfg).await {
                                                Ok(_) => {
                                                    connect_provider.set(None);
                                                    dav_pass.set(String::new());
                                                    s3_sk.set(String::new());
                                                    notice.set(Some((false, "Đã xác minh và kết nối tài khoản đám mây.".into())));
                                                    reload();
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                                "Xác minh và kết nối"
                            }
                        }
                    }
                }
            }
        }

        if !connections().is_empty() {
            div { class: "settings-group",
                div { class: "settings-group-title", "Tài khoản đám mây đã kết nối" }
                div { class: "settings-list",
                    for conn in connections() {
                        div { key: "{conn.id}", class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label",
                                    "{format_provider_label(&conn.provider)} · {conn.account_label}"
                                }
                                span { class: "settings-sublabel",
                                    "Trạng thái: {conn.status}"
                                    if let Some(tot) = conn.quota_total_bytes {
                                        " · Đã dùng {format_bytes_human(conn.quota_used_bytes.unwrap_or(0) as u64)} / {format_bytes_human(tot as u64)}"
                                    }
                                    if let Some(ref ts) = conn.last_backup_at {
                                        " · Lần sao lưu cuối: {ts}"
                                    }
                                }
                                if let Some(ref err) = conn.last_error {
                                    span { class: "settings-sublabel", "Lỗi gần nhất: {err}" }
                                }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    disabled: busy(),
                                    onclick: {
                                        let cid = conn.id.clone();
                                        move |_| {
                                            let cid = cid.clone();
                                            let pw = cloud_passphrase();
                                            if pw.chars().count() < 6 {
                                                notice.set(Some((true, "Nhập mật khẩu mã hóa sao lưu Cloud (>= 6 ký tự) trước.".into())));
                                                return;
                                            }
                                            busy.set(true);
                                            notice.set(None);
                                            spawn(async move {
                                                match backup_cloud::trigger_cloud_backup(&cid, &pw).await {
                                                    Ok(run) => {
                                                        notice.set(Some((
                                                            false,
                                                            format!("Sao lưu ảnh chụp #{} thành công: tải lên {} (chống trùng {}).", run.snapshot_seq, format_bytes_human(run.uploaded_delta_bytes), format_bytes_human(run.deduplicated_bytes)),
                                                        )));
                                                        last_run.set(Some(run));
                                                        load_snapshots(cid);
                                                        reload();
                                                    }
                                                    Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    "Sao lưu tăng dần"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    disabled: busy(),
                                    onclick: {
                                        let cid = conn.id.clone();
                                        move |_| {
                                            let cid = cid.clone();
                                            busy.set(true);
                                            spawn(async move {
                                                match backup_cloud::verify_cloud_connection(&cid).await {
                                                    Ok(_) => {
                                                        notice.set(Some((false, "Kết nối và tệp manifest trên Cloud hợp lệ.".into())));
                                                        reload();
                                                    }
                                                    Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    "Kiểm tra"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-ghost",
                                    onclick: {
                                        let cid = conn.id.clone();
                                        move |_| load_snapshots(cid.clone())
                                    },
                                    "Xem bản sao"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-danger-outline",
                                    disabled: busy(),
                                    onclick: {
                                        let cid = conn.id.clone();
                                        move |_| {
                                            let cid = cid.clone();
                                            busy.set(true);
                                            spawn(async move {
                                                let _ = backup_cloud::disconnect_cloud(&cid).await;
                                                if active_conn_id().as_deref() == Some(cid.as_str()) {
                                                    active_conn_id.set(None);
                                                    snapshots.set(Vec::new());
                                                }
                                                reload();
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    "Ngắt kết nối"
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(run) = last_run() {
            div { class: "status-badge success",
                "Ảnh chụp #{run.snapshot_seq}: tổng {format_bytes_human(run.total_plain_bytes)}, tải mới {format_bytes_human(run.uploaded_delta_bytes)}, tái sử dụng {run.reused_chunks} phân mảnh ({format_bytes_human(run.deduplicated_bytes)})"
            }
        }

        if let Some(cid) = active_conn_id() {
            if !snapshots().is_empty() {
                div { class: "settings-group",
                    div { class: "settings-group-title", "Danh sách ảnh chụp trên đám mây" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            span { class: "settings-label", "Chế độ khôi phục từ Cloud" }
                            select {
                                class: "settings-select",
                                aria_label: "Chế độ khôi phục Cloud",
                                value: "{restore_mode}",
                                onchange: move |e| restore_mode.set(e.value()),
                                option { value: "full", "Khôi phục toàn bộ" }
                                option { value: "missing_only", "Chỉ bổ sung phần thiếu" }
                                option { value: "messages", "Chỉ tin nhắn" }
                                option { value: "media", "Chỉ phương tiện" }
                            }
                        }
                        for snap in snapshots() {
                            div { key: "{snap.id}", class: "settings-row",
                                div { class: "settings-row-text",
                                    span { class: "settings-label", "Bản sao #{snap.snapshot_seq} · {snap.status}" }
                                    span { class: "settings-sublabel",
                                        "{snap.file_count} tệp · Gốc {format_bytes_human(snap.total_plain_bytes as u64)} · Delta {format_bytes_human(snap.uploaded_delta_bytes as u64)} · {snap.created_at}"
                                    }
                                }
                                div { class: "inline-form",
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        disabled: busy(),
                                        onclick: {
                                            let cid = cid.clone();
                                            let sid = snap.id.clone();
                                            move |_| {
                                                let cid = cid.clone();
                                                let sid = sid.clone();
                                                let pw = cloud_passphrase();
                                                let mode = restore_mode();
                                                if pw.chars().count() < 6 {
                                                    notice.set(Some((true, "Nhập mật khẩu giải mã Cloud trước khi khôi phục.".into())));
                                                    return;
                                                }
                                                busy.set(true);
                                                spawn(async move {
                                                    match backup_cloud::restore_cloud_snapshot(&cid, &sid, &pw, &mode).await {
                                                        Ok(rr) => notice.set(Some((
                                                            false,
                                                            format!("Đã khôi phục từ Cloud: +{} tin nhắn, +{} tệp.", rr.restored_messages, rr.restored_media),
                                                        ))),
                                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        "Khôi phục"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-danger-outline",
                                        disabled: busy(),
                                        onclick: {
                                            let cid = cid.clone();
                                            let sid = snap.id.clone();
                                            move |_| {
                                                let cid = cid.clone();
                                                let sid = sid.clone();
                                                busy.set(true);
                                                spawn(async move {
                                                    let _ = backup_cloud::delete_cloud_snapshot(&cid, &sid).await;
                                                    load_snapshots(cid);
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        "Xóa"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Lịch trình sao lưu đám mây tự động" }
            div { class: "settings-list",
                label { class: "settings-row",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Bật tự động sao lưu đám mây" }
                        span { class: "settings-sublabel", "Chỉ tải các phần thay đổi (delta) theo lịch đã cấu hình." }
                    }
                    input {
                        r#type: "checkbox",
                        checked: pol.enabled,
                        onchange: move |e| {
                            let mut next = policy();
                            next.enabled = e.checked();
                            policy.set(next);
                        },
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Chu kỳ sao lưu" }
                    select {
                        class: "settings-select",
                        aria_label: "Chu kỳ sao lưu Cloud",
                        value: "{pol.schedule_mode}",
                        onchange: move |e| {
                            let mut next = policy();
                            next.schedule_mode = e.value();
                            policy.set(next);
                        },
                        option { value: "manual", "Thủ công" }
                        option { value: "daily", "Hàng ngày" }
                        option { value: "weekly", "Hàng tuần" }
                        option { value: "on_change", "Khi có thay đổi lớn" }
                    }
                }
                if !connections().is_empty() {
                    div { class: "settings-row",
                        span { class: "settings-label", "Tài khoản đích mặc định" }
                        select {
                            class: "settings-select",
                            aria_label: "Tài khoản Cloud mặc định",
                            value: "{pol.connection_id.clone().unwrap_or_default()}",
                            onchange: move |e| {
                                let v = e.value();
                                let mut next = policy();
                                next.connection_id = if v.is_empty() { None } else { Some(v) };
                                policy.set(next);
                            },
                            option { value: "", "Chưa chọn" }
                            for conn in connections() {
                                option { value: "{conn.id}", "{format_provider_label(&conn.provider)} · {conn.account_label}" }
                            }
                        }
                    }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Chỉ sao lưu khi dùng Wi-Fi" }
                    input {
                        r#type: "checkbox",
                        checked: pol.wifi_only,
                        onchange: move |e| {
                            let mut next = policy();
                            next.wifi_only = e.checked();
                            policy.set(next);
                        },
                    }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Chỉ sao lưu khi đang sạc pin" }
                    input {
                        r#type: "checkbox",
                        checked: pol.charging_only,
                        onchange: move |e| {
                            let mut next = policy();
                            next.charging_only = e.checked();
                            policy.set(next);
                        },
                    }
                }
                label { class: "settings-row",
                    span { class: "settings-label", "Tạm dừng đồng bộ đám mây" }
                    input {
                        r#type: "checkbox",
                        checked: pol.paused,
                        onchange: move |e| {
                            let mut next = policy();
                            next.paused = e.checked();
                            policy.set(next);
                        },
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Số bản chụp giữ lại trên Cloud" }
                    select {
                        class: "settings-select",
                        aria_label: "Số bản chụp giữ lại",
                        value: "{pol.retain_snapshots}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<i32>() {
                                let mut next = policy();
                                next.retain_snapshots = n;
                                policy.set(next);
                            }
                        },
                        option { value: "3", "3 bản" }
                        option { value: "5", "5 bản" }
                        option { value: "10", "10 bản" }
                        option { value: "20", "20 bản" }
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Áp dụng chính sách sao lưu đám mây" }
                    button {
                        r#type: "button",
                        id: "btn-save-cloud-policy",
                        class: "btn-primary",
                        disabled: busy(),
                        onclick: move |_| {
                            let next = policy();
                            busy.set(true);
                            spawn(async move {
                                match backup_cloud::update_cloud_policy(&next).await {
                                    Ok(saved) => {
                                        policy.set(saved);
                                        notice.set(Some((false, "Đã lưu chính sách sao lưu đám mây.".into())));
                                    }
                                    Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                }
                                busy.set(false);
                            });
                        },
                        "Lưu cấu hình Cloud"
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdvancedSecurityExportSection() -> Element {
    let mut methods = use_signal(ReauthMethods::default);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut busy = use_signal(|| false);

    // Export password setup
    let mut cur_pw = use_signal(String::new);
    let mut totp_or_backup = use_signal(String::new);
    let mut new_pw = use_signal(String::new);

    // Optional AES-256 ZIP
    let mut enc_zip_reauth_method = use_signal(|| "password".to_string());
    let mut enc_zip_reauth_secret = use_signal(String::new);
    let mut enc_zip_pass = use_signal(String::new);

    // Locked-by-default unencrypted ZIP multi-step gate
    let mut zip_panel_open = use_signal(|| false);
    let mut zip_step = use_signal(|| 0u8); // 0 = locked, 1 = reauth, 2 = ack_risk, 3 = final confirm
    let mut zip_reauth_method = use_signal(|| "password".to_string());
    let mut zip_reauth_secret = use_signal(String::new);
    let mut unencrypted_token = use_signal(|| Option::<String>::None);
    let mut zip_preview = use_signal(|| Option::<ExportPreview>::None);

    let reload = move || {
        spawn(async move {
            if let Ok(m) = backup_cloud::get_reauth_methods().await {
                if !m.has_export_password && m.totp_enabled {
                    enc_zip_reauth_method.set("totp".into());
                    zip_reauth_method.set("totp".into());
                } else if !m.has_export_password && m.backup_codes_remaining > 0 {
                    enc_zip_reauth_method.set("backup_code".into());
                    zip_reauth_method.set("backup_code".into());
                }
                methods.set(m);
            }
        });
    };
    use_effect(move || reload());

    let m = methods();

    let abort_and_relock = move || {
        let mut s_step = zip_step;
        let mut s_tok = unencrypted_token;
        let mut s_prev = zip_preview;
        let mut s_sec = zip_reauth_secret;
        let mut s_open = zip_panel_open;
        let tok = s_tok();
        s_step.set(0);
        s_tok.set(None);
        s_prev.set(None);
        s_sec.set(String::new());
        s_open.set(false);
        if let Some(t) = tok {
            spawn(async move {
                let _ = backup_cloud::revoke_unencrypted_zip(&t).await;
            });
        }
    };

    rsx! {
        div { class: "settings-group",
            div { class: "settings-group-title", "Mật khẩu xuất dữ liệu và xác thực sao lưu" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label",
                            if m.has_export_password { "Đổi mật khẩu xuất dữ liệu" } else { "Thiết lập mật khẩu xuất dữ liệu" }
                        }
                        span { class: "settings-sublabel", "Dùng để xác thực lại tài khoản mỗi khi xuất tệp .kdata hoặc giải mã tùy chọn nâng cao." }
                    }
                    if m.has_export_password {
                        div { class: "inline-form",
                            input {
                                class: "settings-input",
                                id: "sec-cur-export-pw",
                                r#type: "password",
                                placeholder: "Mật khẩu xuất hiện tại",
                                value: "{cur_pw}",
                                oninput: move |e| cur_pw.set(e.value()),
                            }
                            if m.totp_enabled || m.backup_codes_remaining > 0 {
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Hoặc mã 2FA / mã dự phòng",
                                    value: "{totp_or_backup}",
                                    oninput: move |e| totp_or_backup.set(e.value()),
                                }
                            }
                        }
                    }
                    div { class: "inline-form",
                        input {
                            class: "settings-input",
                            id: "sec-new-export-pw",
                            r#type: "password",
                            placeholder: "Mật khẩu xuất mới (>= 8 ký tự)",
                            value: "{new_pw}",
                            oninput: move |e| new_pw.set(e.value()),
                        }
                        button {
                            r#type: "button",
                            id: "btn-save-export-pw",
                            class: "btn-secondary",
                            disabled: busy(),
                            onclick: move |_| {
                                let np = new_pw();
                                if np.chars().count() < 8 {
                                    notice.set(Some((true, "Mật khẩu xuất cần ít nhất 8 ký tự.".into())));
                                    return;
                                }
                                let cp = cur_pw();
                                let tb = totp_or_backup();
                                busy.set(true);
                                notice.set(None);
                                spawn(async move {
                                    match backup_cloud::set_export_password(
                                        if cp.is_empty() { None } else { Some(cp.as_str()) },
                                        if tb.is_empty() { None } else { Some(tb.as_str()) },
                                        &np,
                                    ).await {
                                        Ok(()) => {
                                            cur_pw.set(String::new());
                                            totp_or_backup.set(String::new());
                                            new_pw.set(String::new());
                                            notice.set(Some((false, "Đã cập nhật mật khẩu xuất dữ liệu.".into())));
                                            reload();
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            if m.has_export_password { "Cập nhật mật khẩu" } else { "Lưu mật khẩu xuất" }
                        }
                    }
                }
            }
        }

        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Tùy chọn xuất dữ liệu nâng cao" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Xuất tệp ZIP mã hóa AES 256" }
                        span { class: "settings-sublabel", "Lưu ý: ZIP AES 256 mã hóa nội dung tệp nhưng không ẩn tên mục lục bên trong bằng mật mã như định dạng chuẩn .kdata." }
                    }
                    div { class: "inline-form",
                        select {
                            class: "settings-select",
                            aria_label: "Phương thức xác thực ZIP AES",
                            value: "{enc_zip_reauth_method}",
                            onchange: move |e| enc_zip_reauth_method.set(e.value()),
                            if m.has_export_password { option { value: "password", "Mật khẩu xuất" } }
                            if m.totp_enabled { option { value: "totp", "Mã 2FA TOTP" } }
                            if m.backup_codes_remaining > 0 { option { value: "backup_code", "Mã dự phòng" } }
                        }
                        input {
                            class: "settings-input",
                            r#type: "password",
                            placeholder: "Xác thực tài khoản",
                            value: "{enc_zip_reauth_secret}",
                            oninput: move |e| enc_zip_reauth_secret.set(e.value()),
                        }
                        input {
                            class: "settings-input",
                            r#type: "password",
                            placeholder: "Mật khẩu ZIP AES (>= 6 ký tự)",
                            value: "{enc_zip_pass}",
                            oninput: move |e| enc_zip_pass.set(e.value()),
                        }
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            disabled: busy(),
                            onclick: move |_| {
                                let method = enc_zip_reauth_method();
                                let secret = enc_zip_reauth_secret();
                                let zpw = enc_zip_pass();
                                if zpw.chars().count() < 6 {
                                    notice.set(Some((true, "Mật khẩu ZIP AES cần ít nhất 6 ký tự.".into())));
                                    return;
                                }
                                busy.set(true);
                                spawn(async move {
                                    match backup_cloud::verify_reauth("zip_encrypted", &method, &secret).await {
                                        Ok(auth) => {
                                            match backup_cloud::export_zip_encrypted_download(
                                                &auth.export_token,
                                                &zpw,
                                                &ExportScopeOptions::default(),
                                            ).await {
                                                Ok(dl) => {
                                                    enc_zip_reauth_secret.set(String::new());
                                                    notice.set(Some((false, format!("Đã xuất {} ({})", dl.filename, format_bytes_human(dl.byte_size)))));
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Xuất ZIP mã hóa"
                        }
                    }
                }

                div { class: "settings-row column-layout",
                    div { class: "settings-row",
                        style: "width: 100%; padding: 0; border: none;",
                        div { class: "settings-row-text",
                            span { class: "settings-label", "Xuất ZIP không mã hóa (tắt mặc định)" }
                            span { class: "settings-sublabel", "Đang khóa để bảo vệ dữ liệu. Bắt buộc xác thực lại và xác nhận nhiều bước trước khi dùng một lần." }
                        }
                        if !zip_panel_open() {
                            button {
                                r#type: "button",
                                id: "btn-open-unencrypted-zip",
                                class: "btn-danger-outline",
                                onclick: move |_| {
                                    zip_panel_open.set(true);
                                    zip_step.set(0);
                                },
                                "Mở khóa tùy chọn này"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "btn-ghost",
                                onclick: move |_| abort_and_relock(),
                                "Đóng và khóa lại"
                            }
                        }
                    }

                    if zip_panel_open() {
                        if zip_step() == 0 {
                            div { class: "status-badge error",
                                "Cảnh báo bảo mật: Tệp ZIP không mã hóa lưu toàn bộ tin nhắn và ảnh ở dạng đọc trực tiếp. Bất kỳ ai mở tệp này đều xem được nội dung. Luôn ưu tiên .kdata."
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    id: "btn-zip-step1-start",
                                    class: "btn-danger",
                                    onclick: move |_| zip_step.set(1),
                                    "Tôi hiểu cảnh báo, bắt đầu xác thực lại"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-ghost",
                                    onclick: move |_| abort_and_relock(),
                                    "Hủy"
                                }
                            }
                        } else if zip_step() == 1 {
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Bước 1 trên 3: Xác thực lại tài khoản" }
                            }
                            div { class: "inline-form",
                                select {
                                    class: "settings-select",
                                    aria_label: "Phương thức xác thực ZIP không mã hóa",
                                    value: "{zip_reauth_method}",
                                    onchange: move |e| zip_reauth_method.set(e.value()),
                                    if m.has_export_password { option { value: "password", "Mật khẩu xuất" } }
                                    if m.totp_enabled { option { value: "totp", "Mã 2FA TOTP" } }
                                    if m.backup_codes_remaining > 0 { option { value: "backup_code", "Mã dự phòng" } }
                                }
                                input {
                                    class: "settings-input",
                                    id: "unencrypted-zip-reauth-secret",
                                    r#type: "password",
                                    placeholder: "Nhập mật khẩu xuất hoặc mã 2FA",
                                    value: "{zip_reauth_secret}",
                                    oninput: move |e| zip_reauth_secret.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    id: "btn-zip-step1-verify",
                                    class: "btn-danger",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        let method = zip_reauth_method();
                                        let secret = zip_reauth_secret();
                                        busy.set(true);
                                        spawn(async move {
                                            match backup_cloud::verify_reauth("zip_unencrypted", &method, &secret).await {
                                                Ok(auth) => {
                                                    unencrypted_token.set(Some(auth.export_token));
                                                    zip_reauth_secret.set(String::new());
                                                    zip_step.set(2);
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                            busy.set(false);
                                        });
                                    },
                                    "Xác thực và tiếp tục"
                                }
                            }
                        } else if zip_step() == 2 {
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Bước 2 trên 3: Xác nhận rủi ro và xem trước dữ liệu" }
                                span { class: "settings-sublabel", "Không gửi tệp ZIP không mã hóa qua chat hoặc lưu trên dịch vụ công cộng." }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    id: "btn-zip-step2-ack",
                                    class: "btn-danger",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        let Some(tok) = unencrypted_token() else { return; };
                                        busy.set(true);
                                        spawn(async move {
                                            match backup_cloud::advance_unencrypted_zip_step(&tok, "ack_risk").await {
                                                Ok(resp) => {
                                                    zip_preview.set(resp.preview);
                                                    zip_step.set(3);
                                                }
                                                Err(e) => {
                                                    notice.set(Some((true, format_api_error(&e.code).into())));
                                                    abort_and_relock();
                                                }
                                            }
                                            busy.set(false);
                                        });
                                    },
                                    "Tôi chấp nhận rủi ro, xem trước dữ liệu"
                                }
                                button { r#type: "button", class: "btn-ghost", onclick: move |_| abort_and_relock(), "Hủy và khóa lại" }
                            }
                        } else if zip_step() == 3 {
                            if let Some(prev) = zip_preview() {
                                div { class: "settings-row-text",
                                    span { class: "settings-label", "Bước 3 trên 3: Xác nhận lần cuối trước khi tải" }
                                    span { class: "settings-sublabel",
                                        "Sẽ xuất {prev.dm_messages} tin cá nhân, {prev.group_messages} tin nhóm, {prev.channel_messages} tin máy chủ, {prev.durable_media_count} tệp ({format_bytes_human(prev.durable_media_bytes as u64)}) dưới dạng KHÔNG MÃ HÓA."
                                    }
                                }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    id: "btn-zip-step3-export",
                                    class: "btn-danger",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        let Some(tok) = unencrypted_token() else { return; };
                                        busy.set(true);
                                        spawn(async move {
                                            let step_res = backup_cloud::advance_unencrypted_zip_step(&tok, "confirm_final").await;
                                            if let Err(e) = step_res {
                                                notice.set(Some((true, format_api_error(&e.code).into())));
                                                abort_and_relock();
                                                busy.set(false);
                                                return;
                                            }
                                            match backup_cloud::export_zip_unencrypted_download(&tok, &ExportScopeOptions::default()).await {
                                                Ok(dl) => {
                                                    notice.set(Some((
                                                        false,
                                                        format!("Đã tải {} ({}). Tùy chọn này đã tự động khóa lại.", dl.filename, format_bytes_human(dl.byte_size)),
                                                    )));
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                            unencrypted_token.set(None);
                                            zip_preview.set(None);
                                            zip_step.set(0);
                                            zip_panel_open.set(false);
                                            busy.set(false);
                                        });
                                    },
                                    "Xác nhận lần cuối và xuất ZIP"
                                }
                                button { r#type: "button", class: "btn-ghost", onclick: move |_| abort_and_relock(), "Hủy và khóa lại" }
                            }
                        }
                    }
                }
            }
        }
    }
}
