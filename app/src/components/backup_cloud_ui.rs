use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::brand_logos::{CloudProviderLogo, PasskeyLogo, ZipBackupLogo},
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
        "reauth_required" | "export_reauth_required" => "Re-authentication is required before exporting data.",
        "reauth_locked" => "Trial and error too many times. Account is temporarily locked for 15 minutes.",
        "invalid_credentials" => "Password or authentication code is incorrect.",
        "export_password_not_set" => "You have not set a data export password.",
        "export_password_too_short" => "The export password needs to be at least 8 characters.",
        "encryption_password_too_short" => "Encryption passwords need at least 6 characters.",
        "totp_not_enabled" => "The account has not enabled 2FA TOTP authentication.",
        "unencrypted_zip_steps_incomplete" => "Unencrypted ZIP unlock confirmation steps have not been completed.",
        "export_token_expired"
        | "export_token_used"
        | "invalid_export_token"
        | "export_authorization_invalid_or_expired" => {
            "The data export authentication session has expired or has been used."
        }
        "wrong_password_or_corrupt_kdata" => "Wrong decryption password or .kdata file has been corrupted/edited.",
        "wrong_password" => "The decryption password is incorrect.",
        "zip_encrypted_requires_password" => "The ZIP file is AES 256 encrypted, enter the decryption password.",
        "cross_account_restore_denied" | "backup_belongs_to_another_account" => {
            "A backup file belonging to another account cannot be restored."
        }
        "corrupt_kdata" | "unsupported_archive_format" => "The backup file format is invalid or corrupted.",
        "zip_bomb_rejected" => "The ZIP file exceeds the safe decompression limit.",
        "unsafe_archive_path" | "unsafe_path_rejected" => "The archive file contains unsafe paths.",
        "cloud_provider_not_configured" => "This Cloud provider has not had OAuth configured by the administrator.",
        "invalid_cloud_endpoint_or_config" => "The Cloud connection address is invalid or blocked by network security policy.",
        _ => "Operation failed.",
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
        "nas" => "Personal NAS",
        "icloud" => "Apple iCloud",
        _ => "Cloud service",
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
                if !m.has_export_password {
                    if m.passkeys_count > 0 {
                        reauth_method.set("passkey".into());
                    } else if m.totp_enabled {
                        reauth_method.set("totp".into());
                    } else if m.backup_codes_remaining > 0 {
                        reauth_method.set("backup_code".into());
                    }
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
        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Backup data" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Messages in account" }
                                span { class: "settings-sublabel",
                                    "Individual: {p.dm_messages} · Group: {p.group_messages} · Server: {p.channel_messages}"
                                }
                            }
                            span { class: "chip-privacy", "{p.durable_media_count} files · {format_bytes_human(p.durable_media_bytes as u64)}" }
                        }
                        if p.missing_or_client_only_count > 0 {
                            div { class: "settings-row",
                                div { class: "settings-row-text",
                                    span { class: "settings-label", "The file resides only on this device" }
                                    span { class: "settings-sublabel", "{p.honest_notice}" }
                                }
                            }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Account information" }
                            input { r#type: "checkbox", role: "switch", class: "switch", checked: include_account(), onchange: move |e| include_account.set(e.checked()) }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Messages" }
                            input { r#type: "checkbox", role: "switch", class: "switch", checked: include_messages(), onchange: move |e| include_messages.set(e.checked()) }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Photos, videos, audio and files" }
                            input { r#type: "checkbox", role: "switch", class: "switch", checked: include_media(), onchange: move |e| include_media.set(e.checked()) }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Personal settings" }
                            input { r#type: "checkbox", role: "switch", class: "switch", checked: include_settings(), onchange: move |e| include_settings.set(e.checked()) }
                        }
                        div { class: "settings-row column-layout",
                            span { class: "settings-label", "Export only one conversation" }
                            input {
                                class: "settings-input",
                                r#type: "text",
                                placeholder: "Leave blank to export all, or enter chat code",
                                value: "{conversation_id}",
                                oninput: move |e| conversation_id.set(e.value()),
                            }
                        }
                    }
                }

                div { class: "settings-group",
                    div { class: "settings-group-title", "Download the backup file to your device" }
                    div { class: "settings-list",
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "KIYO secure backup file" }
                                span { class: "settings-sublabel", "All data is locked with a private password so only you can open it." }
                            }
                            if !m.has_export_password && !m.totp_enabled && m.backup_codes_remaining == 0 && m.passkeys_count == 0 {
                                div { class: "settings-row-text",
                                    span { class: "settings-sublabel", "Create a data export password of 8 characters or more before downloading the file." }
                                }
                                div { class: "inline-form",
                                    input {
                                        class: "settings-input",
                                        id: "quick-export-pw-input",
                                        r#type: "password",
                                        placeholder: "Password output from 8 characters",
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
                                                notice.set(Some((true, "The export password needs to be at least 8 characters.".into())));
                                                return;
                                            }
                                            busy.set(true);
                                            spawn(async move {
                                                match backup_cloud::set_export_password(None, None, &pw).await {
                                                    Ok(()) => {
                                                        new_export_pw.set(String::new());
                                                        reauth_method.set("password".into());
                                                        notice.set(Some((false, "Data export password has been set.".into())));
                                                        reload();
                                                    }
                                                    Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        "Save export password"
                                    }
                                }
                            } else {
                                div { class: "inline-form",
                                    select {
                                        class: "settings-select",
                                        aria_label: "Export authentication method",
                                        value: "{reauth_method}",
                                        onchange: move |e| reauth_method.set(e.value()),
                                        if m.has_export_password {
                                            option { value: "password", "Data export password" }
                                        }
                                        if m.passkeys_count > 0 {
                                            option { value: "passkey", "Device security key · {m.passkeys_count}" }
                                        }
                                        if m.totp_enabled {
                                            option { value: "totp", "2-step verification code" }
                                        }
                                        if m.backup_codes_remaining > 0 {
                                            option { value: "backup_code", "Backup code · {m.backup_codes_remaining}" }
                                        }
                                    }
                                    if reauth_method() != "passkey" {
                                        input {
                                            class: "settings-input",
                                            id: "kdata-reauth-secret",
                                            r#type: "password",
                                            placeholder: "Enter the export password or confirmation code",
                                            value: "{reauth_secret}",
                                            oninput: move |e| reauth_secret.set(e.value()),
                                        }
                                    } else {
                                        span { class: "settings-sublabel", "Authenticate yourself using the security key on this machine." }
                                    }
                                }
                            }
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    id: "kdata-enc-password",
                                    r#type: "password",
                                    placeholder: "File opening password from 6 characters",
                                    value: "{enc_password}",
                                    oninput: move |e| enc_password.set(e.value()),
                                }
                                input {
                                    class: "settings-input",
                                    id: "kdata-enc-confirm",
                                    r#type: "password",
                                    placeholder: "Re-enter the password to open the file",
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
                                        if method != "passkey" && secret.trim().is_empty() {
                                            notice.set(Some((true, "Please enter account confirmation information.".into())));
                                            return;
                                        }
                                        if epw.chars().count() < 6 {
                                            notice.set(Some((true, "The file opening password needs to be at least 6 characters.".into())));
                                            return;
                                        }
                                        if epw != econf {
                                            notice.set(Some((true, "The two attempts to enter the file opening password do not match.".into())));
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
                                                                format!("File {} · {} loaded", dl.filename, format_bytes_human(dl.byte_size)),
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
                                    if busy() { "Exporting data" } else { "Export backup file" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Restore from backup file" }
                    div { class: "settings-list",
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Select the backup file on your device" }
                                span { class: "settings-sublabel", "Automatically checks files, ignores duplicates, and allows you to select which parts to retrieve." }
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
                                    "Select the backup file"
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
                                    placeholder: "Password to open backup file",
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
                                    "Check the file"
                                }
                            }
                            if let Some(info) = inspected() {
                                div { class: "settings-row column-layout",
                                    div { class: "settings-row-text",
                                        span { class: "settings-label",
                                            "KIYO · {info.detected_format} · {info.total_entries} items · {format_bytes_human(info.total_plain_bytes)}"
                                        }
                                        span { class: "settings-sublabel",
                                            "Message: {info.new_messages} new, {info.duplicate_messages} duplicate · File: {info.new_media} new, {info.duplicate_media} duplicate"
                                        }
                                    }
                                    if info.owner_mismatch {
                                        div { class: "status-badge error", "This file belongs to another account so it cannot be restored." }
                                    }
                                    for w in info.warnings.iter() {
                                        div { class: "settings-sublabel", "{w}" }
                                    }
                                }
                            }
                            div { class: "inline-form",
                                select {
                                    class: "settings-select",
                                    aria_label: "Recovery mode",
                                    value: "{restore_mode}",
                                    onchange: move |e| restore_mode.set(e.value()),
                                    option { value: "full", "Full restore" }
                                    option { value: "missing_only", "Just supplement the missing part" }
                                    option { value: "messages", "Messages only" }
                                    option { value: "images", "Image only" }
                                    option { value: "videos", "Video only" }
                                    option { value: "audio", "Record audio and audio only" }
                                    option { value: "files", "Attachments only" }
                                    option { value: "avatar", "Representative photo only" }
                                    option { value: "settings", "Installation only" }
                                    option { value: "single_conversation", "Just one conversation" }
                                }
                                if restore_mode() == "single_conversation" {
                                    input {
                                        class: "settings-input",
                                        r#type: "text",
                                        placeholder: "Enter the chat code",
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
                                                        format!("Restore complete: {} messages, {} media files.", res.restored_messages, res.restored_media),
                                                    )));
                                                    restore_result.set(Some(res));
                                                    reload();
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                            busy.set(false);
                                        });
                                    },
                                    "Restore data"
                                }
                            }
                            if let Some(rr) = restore_result() {
                                div { class: "settings-row-text",
                                    span { class: "settings-label",
                                        "Result: add {rr.restored_messages} messages, ignore duplicate {rr.skipped_duplicate_messages} · add {rr.restored_media} files"
                                    }
                                    if rr.skipped_unauthorized_messages > 0 {
                                        span { class: "settings-sublabel", "Ignored {rr.skipped_unauthorized_messages} records with invalid permissions." }
                                    }
                                }
                            }
                        }
                    }
                }

                if !jobs().is_empty() {
                    div { class: "settings-group",
                        div { class: "settings-group-title", "Backup history" }
                        div { class: "settings-list",
                            for j in jobs() {
                                div { key: "{j.id}", class: "settings-row",
                                    div { class: "settings-row-text",
                                        span { class: "settings-label", "{j.format} · {j.status}" }
                                        span { class: "settings-sublabel",
                                            "{format_bytes_human(j.processed_bytes as u64)} / {format_bytes_human(j.total_bytes as u64)} · {j.processed_files}/{j.total_files} files"
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
                                                if j.status == "paused" { "Continue" } else { "Pause" }
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
                                                "Cancel"
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
    let mut auto_backup_passphrase = use_signal(String::new);
    let mut clear_auto_key = use_signal(|| false);
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
        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }
        if let Some(run) = last_run() {
            div { class: "status-badge success",
                "Snapshot #{run.snapshot_seq}: total {format_bytes_human(run.total_plain_bytes)}, new load {format_bytes_human(run.uploaded_delta_bytes)}, save {format_bytes_human(run.deduplicated_bytes)}"
            }
        }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Cloud password" }
                    div { class: "settings-list",
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Lock password" }
                            }
                            input {
                                class: "settings-input",
                                id: "cloud-backup-passphrase",
                                r#type: "password",
                                placeholder: "From 6 characters or more",
                                value: "{cloud_passphrase}",
                                oninput: move |e| cloud_passphrase.set(e.value()),
                            }
                        }
                    }
                }

                div { class: "settings-group",
                    div { class: "settings-group-title", "Cloud service" }
                    div { class: "settings-list",
                        for prov in providers() {
                            div { key: "{prov.provider_id}", class: "settings-row cw-brand-row",
                                CloudProviderLogo { provider_id: prov.provider_id.clone(), size: 32 }
                                div { class: "settings-row-text",
                                    span { class: "settings-label", "{prov.display_name}" }
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
                                                            notice.set(Some((false, "Cloud login page opened.".into())));
                                                        }
                                                        Ok(_) => notice.set(Some((true, "Cannot open connection link.".into()))),
                                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        if prov.oauth_configured { "Connect" } else { "Support coming soon" }
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
                                        if connect_provider().as_deref() == Some(prov.provider_id.as_str()) { "Close" } else { "Setup" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-col",
                if let Some(ref pid) = connect_provider() {
                    div { class: "settings-group",
                        div { class: "settings-group-title",
                            CloudProviderLogo { provider_id: pid.clone(), size: 22 }
                            span { "Connection {format_provider_label(pid)}" }
                        }
                        div { class: "settings-list",
                            div { class: "settings-row column-layout",
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Reminiscent name",
                                    value: "{conn_label}",
                                    oninput: move |e| conn_label.set(e.value()),
                                }
                                if matches!(pid.as_str(), "nextcloud" | "webdav" | "nas") {
                                    input {
                                        class: "settings-input",
                                        id: "cloud-dav-url",
                                        r#type: "text",
                                        placeholder: "WebDAV server address",
                                        value: "{dav_url}",
                                        oninput: move |e| dav_url.set(e.value()),
                                    }
                                    div { class: "inline-form",
                                        input {
                                            class: "settings-input",
                                            id: "cloud-dav-user",
                                            r#type: "text",
                                            placeholder: "Username",
                                            value: "{dav_user}",
                                            oninput: move |e| dav_user.set(e.value()),
                                        }
                                        input {
                                            class: "settings-input",
                                            id: "cloud-dav-pass",
                                            r#type: "password",
                                            placeholder: "Password",
                                            value: "{dav_pass}",
                                            oninput: move |e| dav_pass.set(e.value()),
                                        }
                                    }
                                    input {
                                        class: "settings-input",
                                        r#type: "text",
                                        placeholder: "Storage folder name",
                                        value: "{dav_folder}",
                                        oninput: move |e| dav_folder.set(e.value()),
                                    }
                                } else {
                                    div { class: "inline-form",
                                        input {
                                            class: "settings-input",
                                            r#type: "text",
                                            placeholder: "HTTPS server address",
                                            value: "{s3_endpoint}",
                                            oninput: move |e| s3_endpoint.set(e.value()),
                                        }
                                        input {
                                            class: "settings-input",
                                            r#type: "text",
                                            placeholder: "Area",
                                            value: "{s3_region}",
                                            oninput: move |e| s3_region.set(e.value()),
                                        }
                                    }
                                    div { class: "inline-form",
                                        input {
                                            class: "settings-input",
                                            r#type: "text",
                                            placeholder: "Name of the repository",
                                            value: "{s3_bucket}",
                                            oninput: move |e| s3_bucket.set(e.value()),
                                        }
                                        input {
                                            class: "settings-input",
                                            r#type: "text",
                                            placeholder: "Root directory",
                                            value: "{s3_prefix}",
                                            oninput: move |e| s3_prefix.set(e.value()),
                                        }
                                    }
                                    div { class: "inline-form",
                                        input {
                                            class: "settings-input",
                                            r#type: "text",
                                            placeholder: "Access code",
                                            value: "{s3_ak}",
                                            oninput: move |e| s3_ak.set(e.value()),
                                        }
                                        input {
                                            class: "settings-input",
                                            r#type: "password",
                                            placeholder: "Secret code",
                                            value: "{s3_sk}",
                                            oninput: move |e| s3_sk.set(e.value()),
                                        }
                                    }
                                    label { class: "settings-row",
                                        span { class: "settings-label", "Use direct path for MinIO" }
                                        input { r#type: "checkbox", role: "switch", class: "switch", checked: s3_path_style(), onchange: move |e| s3_path_style.set(e.checked()) }
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
                                                            notice.set(Some((false, "Cloud account connected.".into())));
                                                            reload();
                                                        }
                                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        "Verify and connect"
                                    }
                                }
                            }
                        }
                    }
                }

                if !connections().is_empty() {
                    div { class: "settings-group",
                        div { class: "settings-group-title", "Cloud account connected" }
                        div { class: "settings-list",
                            for conn in connections() {
                                div { key: "{conn.id}", class: "settings-row column-layout",
                                    div { class: "cw-brand-row",
                                        CloudProviderLogo { provider_id: conn.provider.clone(), size: 28 }
                                        div { class: "settings-row-text",
                                            span { class: "settings-label",
                                                "{format_provider_label(&conn.provider)} · {conn.account_label}"
                                            }
                                            span { class: "settings-sublabel",
                                                "Status: {conn.status}"
                                                if let Some(tot) = conn.quota_total_bytes {
                                                    " · Used {format_bytes_human(conn.quota_used_bytes.unwrap_or(0) as u64)} / {format_bytes_human(tot as u64)}"
                                                }
                                                if let Some(ref ts) = conn.last_backup_at {
                                                    " · Last backup: {ts}"
                                                }
                                            }
                                            if let Some(ref err) = conn.last_error {
                                                span { class: "settings-sublabel", "Most recent error: {err}" }
                                            }
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
                                                        notice.set(Some((true, "Enter the backup password from the previous 6 characters.".into())));
                                                        return;
                                                    }
                                                    busy.set(true);
                                                    notice.set(None);
                                                    spawn(async move {
                                                        match backup_cloud::trigger_cloud_backup(&cid, &pw).await {
                                                            Ok(run) => {
                                                                notice.set(Some((
                                                                    false,
                                                                    format!("Photo #{} successfully · uploaded {}", run.snapshot_seq, format_bytes_human(run.uploaded_delta_bytes)),
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
                                            "Incremental backups"
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
                                                                notice.set(Some((false, "Cloud connection is working fine.".into())));
                                                                reload();
                                                            }
                                                            Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                        }
                                                        busy.set(false);
                                                    });
                                                }
                                            },
                                            "Check"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "btn-ghost",
                                            onclick: {
                                                let cid = conn.id.clone();
                                                move |_| load_snapshots(cid.clone())
                                            },
                                            "See copy"
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
                                            "Disconnect"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(cid) = active_conn_id() {
                    if !snapshots().is_empty() {
                        div { class: "settings-group",
                            div { class: "settings-group-title", "Cloud copy" }
                            div { class: "settings-list",
                                div { class: "settings-row",
                                    span { class: "settings-label", "Part to restore" }
                                    select {
                                        class: "settings-select",
                                        aria_label: "Cloud recovery mode",
                                        value: "{restore_mode}",
                                        onchange: move |e| restore_mode.set(e.value()),
                                        option { value: "full", "Full restore" }
                                        option { value: "missing_only", "Just supplement the missing part" }
                                        option { value: "messages", "Messages only" }
                                        option { value: "media", "Means only" }
                                    }
                                }
                                for snap in snapshots() {
                                    div { key: "{snap.id}", class: "settings-row",
                                        div { class: "settings-row-text",
                                            span { class: "settings-label", "Copy #{snap.snapshot_seq} · {snap.status}" }
                                            span { class: "settings-sublabel",
                                                "{snap.file_count} file · Original {format_bytes_human(snap.total_plain_bytes as u64)} · New {format_bytes_human(snap.uploaded_delta_bytes as u64)} · {snap.created_at}"
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
                                                            notice.set(Some((true, "Enter the backup password before restoring.".into())));
                                                            return;
                                                        }
                                                        busy.set(true);
                                                        spawn(async move {
                                                            match backup_cloud::restore_cloud_snapshot(&cid, &sid, &pw, &mode).await {
                                                                Ok(rr) => notice.set(Some((
                                                                    false,
                                                                    format!("Recovered: added {} messages, {} files.", rr.restored_messages, rr.restored_media),
                                                                ))),
                                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                                            }
                                                            busy.set(false);
                                                        });
                                                    }
                                                },
                                                "Restore"
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
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "settings-group",
                    div { class: "settings-group-title", "Automatic backup" }
                    div { class: "settings-list",
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Automatic backup" }
                            }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: pol.enabled,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.enabled = e.checked();
                                    policy.set(next);
                                },
                            }
                        }
                        div { class: "settings-row",
                            span { class: "settings-label", "Cycle" }
                            select {
                                class: "settings-select",
                                aria_label: "Cloud backup cycle",
                                value: "{pol.schedule_mode}",
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.schedule_mode = e.value();
                                    policy.set(next);
                                },
                                option { value: "manual", "Manual" }
                                option { value: "daily", "Every day" }
                                option { value: "weekly", "Every week" }
                                option { value: "on_change", "When changed" }
                            }
                        }
                        if !connections().is_empty() {
                            div { class: "settings-row",
                                span { class: "settings-label", "Destination account" }
                                select {
                                    class: "settings-select",
                                    aria_label: "Default Cloud account",
                                    value: "{pol.connection_id.clone().unwrap_or_default()}",
                                    onchange: move |e| {
                                        let v = e.value();
                                        let mut next = policy();
                                        next.connection_id = if v.is_empty() { None } else { Some(v) };
                                        policy.set(next);
                                    },
                                    option { value: "", "Not selected yet" }
                                    for conn in connections() {
                                        option { value: "{conn.id}", "{format_provider_label(&conn.provider)} · {conn.account_label}" }
                                    }
                                }
                            }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Only over Wi-Fi" }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: pol.wifi_only,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.wifi_only = e.checked();
                                    policy.set(next);
                                },
                            }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Only when charging" }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: pol.charging_only,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.charging_only = e.checked();
                                    policy.set(next);
                                },
                            }
                        }
                        label { class: "settings-row",
                            span { class: "settings-label", "Pause" }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: pol.paused,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.paused = e.checked();
                                    policy.set(next);
                                },
                            }
                        }
                        div { class: "settings-row",
                            span { class: "settings-label", "Copy retained" }
                            select {
                                class: "settings-select",
                                aria_label: "Number of copies kept",
                                value: "{pol.retain_snapshots}",
                                onchange: move |e| {
                                    if let Ok(n) = e.value().parse::<i32>() {
                                        let mut next = policy();
                                        next.retain_snapshots = n;
                                        policy.set(next);
                                    }
                                },
                                option { value: "3", "3 copies" }
                                option { value: "5", "5 copies" }
                                option { value: "10", "10 copies" }
                                option { value: "20", "20 copies" }
                            }
                        }
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Automatic password" }
                            }
                            div { class: "inline-form",
                                input {
                                    class: "settings-input",
                                    id: "cloud-auto-passphrase",
                                    r#type: "password",
                                    placeholder: if pol.has_auto_key { "Change password automatically" } else { "From 6 characters" },
                                    value: "{auto_backup_passphrase}",
                                    oninput: move |e| auto_backup_passphrase.set(e.value()),
                                }
                                if pol.has_auto_key {
                                    label { class: "inline-form",
                                        input { r#type: "checkbox", checked: clear_auto_key(), onchange: move |e| clear_auto_key.set(e.checked()) }
                                        span { class: "settings-sublabel", "Delete key" }
                                    }
                                }
                            }
                        }
                        div { class: "settings-row",
                            span { class: "settings-label", "Save calendar" }
                            button {
                                r#type: "button",
                                id: "btn-save-cloud-policy",
                                class: "btn-primary",
                                disabled: busy(),
                                onclick: move |_| {
                                    let next = policy();
                                    let auto_pw = auto_backup_passphrase();
                                    let clear_key = clear_auto_key();
                                    busy.set(true);
                                    spawn(async move {
                                        match backup_cloud::update_cloud_policy(
                                            &next,
                                            if auto_pw.trim().is_empty() { None } else { Some(auto_pw.trim()) },
                                            clear_key,
                                        ).await {
                                            Ok(saved) => {
                                                policy.set(saved);
                                                auto_backup_passphrase.set(String::new());
                                                clear_auto_key.set(false);
                                                notice.set(Some((false, "Saved cloud backup schedule.".into())));
                                            }
                                            Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "Save configuration"
                            }
                        }
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
                if !m.has_export_password {
                    if m.passkeys_count > 0 {
                        enc_zip_reauth_method.set("passkey".into());
                        zip_reauth_method.set("passkey".into());
                    } else if m.totp_enabled {
                        enc_zip_reauth_method.set("totp".into());
                        zip_reauth_method.set("totp".into());
                    } else if m.backup_codes_remaining > 0 {
                        enc_zip_reauth_method.set("backup_code".into());
                        zip_reauth_method.set("backup_code".into());
                    }
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
            div { class: "settings-group-title", "Data export password" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "settings-row-text",
                        span { class: "settings-label",
                            if m.has_export_password { "Change data export password" } else { "Set data export password" }
                        }
                        span { class: "settings-sublabel", "Used to reconfirm your account each time you download a backup." }
                    }
                    if m.has_export_password {
                        div { class: "inline-form",
                            input {
                                class: "settings-input",
                                id: "sec-cur-export-pw",
                                r#type: "password",
                                placeholder: "Password appears now",
                                value: "{cur_pw}",
                                oninput: move |e| cur_pw.set(e.value()),
                            }
                            if m.totp_enabled || m.backup_codes_remaining > 0 {
                                input {
                                    class: "settings-input",
                                    r#type: "text",
                                    placeholder: "Or 2-step verification code",
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
                            placeholder: "New export password from 8 characters",
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
                                    notice.set(Some((true, "The export password needs to be at least 8 characters.".into())));
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
                                            notice.set(Some((false, "Updated data export password.".into())));
                                            reload();
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            if m.has_export_password { "Update password" } else { "Save export password" }
                        }
                    }
                }
                div { class: "settings-row cw-brand-row",
                    PasskeyLogo { size: 30 }
                    div { class: "settings-row-text",
                        span { class: "settings-label", "Security lock on device" }
                        span { class: "settings-sublabel", "Registered {m.passkeys_count} key on this machine for quick password-less authentication." }
                    }
                    button {
                        r#type: "button",
                        id: "btn-register-passkey",
                        class: "btn-secondary",
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            notice.set(None);
                            spawn(async move {
                                match backup_cloud::register_device_passkey("Lock KIYO device").await {
                                    Ok(_) => {
                                        notice.set(Some((false, "Created and registered the P-256 security key on the device.".into())));
                                        reload();
                                    }
                                    Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                }
                                busy.set(false);
                            });
                        },
                        "Register device lock"
                    }
                }
            }
        }

        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "Advanced data export" }
            div { class: "settings-list",
                div { class: "settings-row column-layout",
                    div { class: "cw-brand-row",
                        ZipBackupLogo { size: 28 }
                        div { class: "settings-row-text",
                            span { class: "settings-label", "Export ZIP file with password" }
                            span { class: "settings-sublabel", "Protect the content with a password, but don't hide the file names inside like the primary backup." }
                        }
                    }
                    div { class: "inline-form",
                        select {
                            class: "settings-select",
                            aria_label: "ZIP AES authentication method",
                            value: "{enc_zip_reauth_method}",
                            onchange: move |e| enc_zip_reauth_method.set(e.value()),
                            if m.has_export_password || (m.passkeys_count == 0 && !m.totp_enabled && m.backup_codes_remaining == 0) { option { value: "password", "Export password" } }
                            if m.passkeys_count > 0 { option { value: "passkey", "Device Lock · {m.passkeys_count}" } }
                            if m.totp_enabled { option { value: "totp", "2-step verification code" } }
                            if m.backup_codes_remaining > 0 { option { value: "backup_code", "Backup code" } }
                        }
                        if enc_zip_reauth_method() != "passkey" {
                            input {
                                class: "settings-input",
                                r#type: "password",
                                placeholder: "Confirm account",
                                value: "{enc_zip_reauth_secret}",
                                oninput: move |e| enc_zip_reauth_secret.set(e.value()),
                            }
                        }
                        input {
                            class: "settings-input",
                            r#type: "password",
                            placeholder: "ZIP password from 6 characters",
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
                                    notice.set(Some((true, "ZIP passwords need at least 6 characters.".into())));
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
                                                    notice.set(Some((false, format!("Exported {} · {}", dl.filename, format_bytes_human(dl.byte_size)))));
                                                }
                                                Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                            }
                                        }
                                        Err(e) => notice.set(Some((true, format_api_error(&e.code).into()))),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Export ZIP with password"
                        }
                    }
                }

                div { class: "settings-row column-layout",
                    div { class: "settings-row",
                        style: "width: 100%; padding: 0; border: none;",
                        div { class: "settings-row-text",
                            span { class: "settings-label", "Passwordless ZIP export" }
                            span { class: "settings-sublabel", "Locking to protect data. Need to confirm many steps before using." }
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
                                "Unlock this option"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "btn-ghost",
                                onclick: move |_| abort_and_relock(),
                                "Close and lock"
                            }
                        }
                    }

                    if zip_panel_open() {
                        if zip_step() == 0 {
                            div { class: "status-badge error",
                                "Warning: Passwordless ZIP files save all messages and photos in an open form. Anyone who opens the file can read it."
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    id: "btn-zip-step1-start",
                                    class: "btn-danger",
                                    onclick: move |_| zip_step.set(1),
                                    "I understand the warning, continue"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-ghost",
                                    onclick: move |_| abort_and_relock(),
                                    "Cancel"
                                }
                            }
                        } else if zip_step() == 1 {
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Step 1 of 3: Confirm your account" }
                            }
                            div { class: "inline-form",
                                select {
                                    class: "settings-select",
                                    aria_label: "Unencrypted ZIP authentication method",
                                    value: "{zip_reauth_method}",
                                    onchange: move |e| zip_reauth_method.set(e.value()),
                                    if m.has_export_password { option { value: "password", "Export password" } }
                                    if m.passkeys_count > 0 { option { value: "passkey", "Device Lock · {m.passkeys_count}" } }
                                    if m.totp_enabled { option { value: "totp", "2-step verification code" } }
                                    if m.backup_codes_remaining > 0 { option { value: "backup_code", "Backup code" } }
                                }
                                if zip_reauth_method() != "passkey" {
                                    input {
                                        class: "settings-input",
                                        id: "unencrypted-zip-reauth-secret",
                                        r#type: "password",
                                        placeholder: "Enter the export password or verification code",
                                        value: "{zip_reauth_secret}",
                                        oninput: move |e| zip_reauth_secret.set(e.value()),
                                    }
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
                                    "Confirm and continue"
                                }
                            }
                        } else if zip_step() == 2 {
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Step 2 of 3: Preview data" }
                                span { class: "settings-sublabel", "Do not send passwordless ZIP files over the internet or in public places." }
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
                                    "I agree, preview the data"
                                }
                                button { r#type: "button", class: "btn-ghost", onclick: move |_| abort_and_relock(), "Cancel and lock again" }
                            }
                        } else if zip_step() == 3 {
                            if let Some(prev) = zip_preview() {
                                div { class: "settings-row-text",
                                    span { class: "settings-label", "Step 3 of 3: Final confirmation" }
                                    span { class: "settings-sublabel",
                                        "Will download {prev.dm_messages} private messages, {prev.group_messages} group messages, {prev.channel_messages} server messages, {prev.durable_media_count} files · {format_bytes_human(prev.durable_media_bytes as u64)} without password."
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
                                                        format!("Loaded {} · {}. This option has locked itself.", dl.filename, format_bytes_human(dl.byte_size)),
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
                                    "Final confirmation and ZIP export"
                                }
                                button { r#type: "button", class: "btn-ghost", onclick: move |_| abort_and_relock(), "Cancel and lock again" }
                            }
                        }
                    }
                }
            }
        }
    }
}
