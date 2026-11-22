use dioxus::prelude::*;

use crate::{
    crypto::e2ee::{
        UserKeys, create_and_store_user_keys, enable_passphrase_vault, get_user_keys,
        is_passphrase_vault_enabled, sign_device_recovery_proof, unlock_passphrase_vault,
        wrap_sync_key_for_device,
    },
    platform,
    services::{
        api,
        storage_sync::{self, DevicesListResponse, RecoveryListResponse, StoragePolicy},
    },
    storage::{
        DEVICE_TRUST_STATE, E2EE_RECOVERY_REQUIRED, PENDING_APPROVAL_ALERT, SYNC_PROGRESS,
        LocalQuotaInfo, clear_safe_cache, format_bytes_human, get_quota_estimate, recovery,
        sync_manager,
    },
};

#[component]
pub fn StorageSettingsPanel() -> Element {
    let mut quota = use_signal(LocalQuotaInfo::default);
    let mut metrics = use_signal(|| Option::<serde_json::Value>::None);
    let mut caps = use_signal(|| Option::<serde_json::Value>::None);
    let mut policy = use_signal(|| StoragePolicy {
        cache_limit_mb: 1024,
        auto_download_mode: "wifi".into(),
        dm_relay_ttl_hours: 168,
        dm_encrypted_backup: false,
        require_device_approval: true,
        prefer_original_upload: false,
    });
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut confirm_purge_unique = use_signal(|| false);
    let mut migration_res = use_signal(|| Option::<serde_json::Value>::None);

    let reload = move || {
        spawn(async move {
            quota.set(get_quota_estimate().await);
            if let Ok(p) = storage_sync::get_policy().await {
                platform::storage_set("chatw_cache_limit_mb", &p.cache_limit_mb.to_string());
                policy.set(p);
            }
            if let Ok(m) = storage_sync::get_metrics().await {
                metrics.set(Some(m));
            }
            if let Ok(c) = storage_sync::get_capabilities().await {
                caps.set(Some(c));
            }
        });
    };
    use_effect(move || reload());

    let q = quota();
    let p = policy();
    let m = metrics();

    let server_physical = m.as_ref().and_then(|v| v["server"]["physical_bytes"].as_u64()).unwrap_or(0);
    let dedup_saved = m.as_ref().and_then(|v| v["server"]["dedup_saved_bytes"].as_u64()).unwrap_or(0);
    let comp_saved = m.as_ref().and_then(|v| v["server"]["compression_saved_bytes"].as_u64()).unwrap_or(0);
    let dm_bytes = m.as_ref().and_then(|v| v["user"]["dm_bytes"].as_u64()).unwrap_or(0);
    let group_bytes = m.as_ref().and_then(|v| v["user"]["group_bytes"].as_u64()).unwrap_or(0);
    let channel_bytes = m.as_ref().and_then(|v| v["user"]["channel_bytes"].as_u64()).unwrap_or(0);
    let at_risk = m.as_ref().and_then(|v| v["user"]["expiring_without_safe_copies"].as_i64()).unwrap_or(0);

    let s3_ready = caps.read().as_ref().and_then(|v| v["s3"]["ready"].as_bool()).unwrap_or(false);

    rsx! {
        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }
        if at_risk > 0 {
            div { class: "status-badge error", "Warning: {at_risk} private chat files are about to expire and there are not enough devices to save a copy." }
        }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Device storage" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Saved" }
                            }
                            span { class: "chip-privacy", "{q.local_object_count} files · {format_bytes_human(q.total_local_bytes)}" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Cache" }
                            }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| {
                                    spawn(async move {
                                        match clear_safe_cache(false).await {
                                            Ok((freed, kept)) => {
                                                notice.set(Some((false, format!("Freed {}, kept {kept} separate files", format_bytes_human(freed)))));
                                                reload();
                                            }
                                            Err(e) => notice.set(Some((true, format!("Unable to clear buffer: {e}")))),
                                        }
                                    });
                                },
                                "Clear cache"
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Original file on the device" }
                            }
                            if confirm_purge_unique() {
                                div { class: "inline-form",
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_purge_unique.set(false), "Cancel" }
                                    button {
                                        r#type: "button",
                                        class: "btn-danger",
                                        onclick: move |_| {
                                            confirm_purge_unique.set(false);
                                            spawn(async move {
                                                let _ = clear_safe_cache(true).await;
                                                notice.set(Some((false, "Deleted backup on device".into())));
                                                reload();
                                            });
                                        },
                                        "Confirm deletion"
                                    }
                                }
                            } else if q.pinned_unique_bytes > 0 {
                                button { r#type: "button", class: "btn-danger-outline", onclick: move |_| confirm_purge_unique.set(true), "Delete save" }
                            } else {
                                span { class: "chip-privacy", "{format_bytes_human(q.pinned_unique_bytes)}" }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Buffer limit" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Caching limit",
                                value: "{p.cache_limit_mb}",
                                onchange: move |e| {
                                    if let Ok(mb) = e.value().parse::<i32>() {
                                        let mut next = policy();
                                        next.cache_limit_mb = mb;
                                        policy.set(next.clone());
                                        platform::storage_set("chatw_cache_limit_mb", &mb.to_string());
                                        spawn(async move {
                                            let _ = storage_sync::update_policy(&next).await;
                                        });
                                    }
                                },
                                option { value: "500", "500 MB" }
                                option { value: "1024", "1 GB" }
                                option { value: "2048", "2 GB" }
                                option { value: "5120", "5 GB" }
                                option { value: "10240", "10 GB" }
                                option { value: "0", "Unlimited" }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Auto-download" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Auto download mode",
                                value: "{p.auto_download_mode}",
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.auto_download_mode = e.value();
                                    policy.set(next.clone());
                                    spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                },
                                option { value: "always", "Always" }
                                option { value: "wifi", "Wi-Fi only" }
                                option { value: "manual", "Manual" }
                            }
                        }
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Keep the original photo" }
                            }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: p.prefer_original_upload,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.prefer_original_upload = e.checked();
                                    policy.set(next.clone());
                                    spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                },
                            }
                        }
                    }
                }
            }

            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Capacity" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Chat" }
                            }
                            span { class: "chip-privacy", "Individual {format_bytes_human(dm_bytes)} · Group {format_bytes_human(group_bytes)} · Channel {format_bytes_human(channel_bytes)}" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Saved" }
                            }
                            span { class: "chip-privacy", "Save {format_bytes_human(server_physical)} · Reduce {format_bytes_human(dedup_saved + comp_saved)}" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", if s3_ready { "Expanded repository" } else { "Host server" } }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        spawn(async move {
                                            if let Ok(res) = storage_sync::run_migration_job(true).await {
                                                migration_res.set(Some(res));
                                            }
                                        });
                                    },
                                    "Check"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        spawn(async move {
                                            let _ = storage_sync::trigger_cleanup().await;
                                            reload();
                                        });
                                    },
                                    "Clean up files"
                                }
                            }
                        }
                        if let Some(mr) = migration_res() {
                            div { class: "settings-row",
                                span { class: "chip-privacy",
                                    "Check {mr[\"scanned_objects\"]} · Valid {mr[\"verified_objects\"]} · Error {mr[\"failed_objects\"]}"
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
pub fn SyncRecoverySettingsPanel(
    current_user_id: String,
    user_keys: Option<UserKeys>,
    on_keys_imported: EventHandler<UserKeys>,
) -> Element {
    let mut policy = use_signal(StoragePolicy::default);
    let mut requests = use_signal(RecoveryListResponse::default);
    let mut passphrase = use_signal(String::new);
    let mut restore_passphrase = use_signal(String::new);
    let mut twofa_code = use_signal(String::new);
    let mut has_backup = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<(bool, String)>::None);
    let progress = SYNC_PROGRESS();

    let reload = move || {
        spawn(async move {
            if let Ok(p) = storage_sync::get_policy().await {
                policy.set(p);
            }
            if let Ok(r) = storage_sync::list_recovery_requests().await {
                requests.set(r);
            }
            if let Ok(bk) = storage_sync::get_key_backup().await {
                has_backup.set(bk.get("exists").and_then(|v| v.as_bool()).unwrap_or(false));
            }
        });
    };
    use_effect(move || reload());
    let p = policy();

    rsx! {
        if let Some((is_err, msg)) = status_msg() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Device sync" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Status" }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| sync_manager::toggle_phase_b_pause(),
                                    if progress.phase_b_paused { "Continue" } else { "Pause" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        spawn(async move {
                                            match sync_manager::run_incremental_delta_sync().await {
                                                Ok(()) => {
                                                    let _ = sync_manager::report_cached_copies_to_server().await;
                                                    status_msg.set(Some((false, "Synchronization completed".into())));
                                                }
                                                Err(e) => status_msg.set(Some((true, format!("Unable to synchronize: {e}")))),
                                            }
                                        });
                                    },
                                    "Sync now"
                                }
                            }
                        }
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Confirm new machine" }
                            }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: p.require_device_approval,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.require_device_approval = e.checked();
                                    policy.set(next.clone());
                                    spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                },
                            }
                        }
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Back up private files" }
                            }
                            input {
                                r#type: "checkbox",
                                role: "switch",
                                class: "switch",
                                checked: p.dm_encrypted_backup,
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.dm_encrypted_backup = e.checked();
                                    policy.set(next.clone());
                                    spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                },
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Keep the sent file temporarily" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Transit time",
                                value: "{p.dm_relay_ttl_hours}",
                                onchange: move |e| {
                                    if let Ok(hrs) = e.value().parse::<i32>() {
                                        let mut next = policy();
                                        next.dm_relay_ttl_hours = hrs;
                                        policy.set(next.clone());
                                        spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                    }
                                },
                                option { value: "24", "24 hours" }
                                option { value: "72", "3 days" }
                                option { value: "168", "7 days" }
                                option { value: "720", "30 days" }
                            }
                        }
                    }
                }

                if !requests().requests.is_empty() {
                    div { class: "settings-group",
                        div { class: "settings-group-title", "Restore files" }
                        div { class: "settings-list",
                            for rr in requests().requests {
                                div { key: "{rr.id}", class: "settings-row",
                                    div { class: "settings-row-text",
                                        span { class: "settings-label", "{rr.file_name} · {format_bytes_human(rr.byte_size.max(0) as u64)}" }
                                    }
                                    if rr.is_incoming && (rr.status == "pending" || rr.status == "queued_offline_peer") {
                                        div { class: "inline-form",
                                            button {
                                                r#type: "button",
                                                class: "btn-primary",
                                                onclick: {
                                                    let rid = rr.id.clone();
                                                    let ref_id = rr.ref_id.clone();
                                                    let mode = rr.transport_mode.clone();
                                                    move |_| {
                                                        let rid = rid.clone();
                                                        let ref_id = ref_id.clone();
                                                        let mode = mode.clone();
                                                        spawn(async move {
                                                            recovery::respond_to_incoming_recovery(&rid, &ref_id, &mode).await;
                                                            reload();
                                                        });
                                                    }
                                                },
                                                "Send"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-secondary",
                                                onclick: {
                                                    let rid = rr.id.clone();
                                                    move |_| {
                                                        let rid = rid.clone();
                                                        spawn(async move {
                                                            let _ = storage_sync::respond_recovery_request(&rid, "deny", None, None, Some("User declines")).await;
                                                            reload();
                                                        });
                                                    }
                                                },
                                                "Refuse"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Key recovery" }
                    div { class: "settings-list",
                        if let Some(k) = user_keys.clone() {
                            div { class: "settings-row column-layout",
                                div { class: "settings-label",
                                    if has_backup() { "Key saved" } else { "Save a copy of the key" }
                                }
                                div { class: "inline-form",
                                    input {
                                        r#type: "password",
                                        placeholder: "From 6 characters",
                                        aria_label: "Key recovery password",
                                        value: "{passphrase}",
                                        oninput: move |e| passphrase.set(e.value()),
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        onclick: {
                                            let keys_for_backup = k.clone();
                                            move |_| {
                                                let keys = keys_for_backup.clone();
                                                let pass = passphrase();
                                                spawn(async move {
                                                    match sync_manager::encrypt_keys_for_backup(&keys, &pass) {
                                                        Ok((wrapped, salt, iters)) => {
                                                            match storage_sync::put_key_backup_v2(
                                                                &wrapped,
                                                                &salt,
                                                                "argon2id-v1",
                                                                sync_manager::BACKUP_ARGON2_MEM_KIB as i32,
                                                                sync_manager::BACKUP_ARGON2_TIME_COST as i32,
                                                                sync_manager::BACKUP_ARGON2_PARALLELISM as i32,
                                                                iters,
                                                                &keys.fingerprint_b64,
                                                            )
                                                            .await
                                                            {
                                                                Ok(_) => {
                                                                    passphrase.set(String::new());
                                                                    has_backup.set(true);
                                                                    status_msg.set(Some((false, "Saved a copy of the Argon2id key".into())));
                                                                }
                                                                Err(e) => status_msg.set(Some((true, format!("Backup error: {}", e.code)))),
                                                            }
                                                        }
                                                        Err(e) => status_msg.set(Some((true, e))),
                                                    }
                                                });
                                            }
                                        },
                                        "Save key"
                                    }
                                }
                            }
                            div { class: "settings-row column-layout",
                                div { class: "settings-label",
                                    if is_passphrase_vault_enabled(&current_user_id) {
                                        "This lock vault is protected by Argon2id"
                                    } else {
                                        "Protect this keystore with an E2EE password (Argon2id)"
                                    }
                                }
                                div { class: "inline-form",
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        onclick: {
                                            let uid = current_user_id.clone();
                                            let keys_for_vault = k.clone();
                                            move |_| {
                                                let pass = passphrase();
                                                if pass.chars().count() < 8 {
                                                    status_msg.set(Some((true, "Enter a password of at least 8 characters in the box above before locking the local vault".into())));
                                                    return;
                                                }
                                                match enable_passphrase_vault(&uid, &keys_for_vault, &pass) {
                                                    Ok(()) => {
                                                        passphrase.set(String::new());
                                                        status_msg.set(Some((false, "Encrypted local keystore using Argon2id".into())));
                                                    }
                                                    Err(e) => status_msg.set(Some((true, e))),
                                                }
                                            }
                                        },
                                        "Lock this device"
                                    }
                                }
                            }
                        }
                        div { class: "settings-row column-layout",
                            div { class: "settings-label", "Unlock messages" }
                            div { class: "inline-form",
                                input {
                                    r#type: "text",
                                    placeholder: "6-digit code",
                                    aria_label: "Lost device verification code",
                                    value: "{twofa_code}",
                                    oninput: move |e| twofa_code.set(e.value()),
                                }
                                input {
                                    r#type: "password",
                                    placeholder: "Lock password",
                                    aria_label: "Password decrypts the backup key",
                                    value: "{restore_passphrase}",
                                    oninput: move |e| restore_passphrase.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        let uid = current_user_id.clone();
                                        let code = twofa_code();
                                        let pass = restore_passphrase();
                                        spawn(async move {
                                            let c = if code.trim().is_empty() { None } else { Some(code.trim()) };
                                            // First attempt to decrypt the Argon2id server key backup so we can produce
                                            // a cryptographic ECDSA ownership proof (`recovery_proof_b64`) before calling recover-lost.
                                            let mut recovered_keys: Option<UserKeys> = None;
                                            let mut recovery_proof: Option<String> = None;
                                            if !pass.trim().is_empty() {
                                                if let Ok(bk) = storage_sync::get_key_backup().await
                                                    && let Some(kb) = bk.get("backup").filter(|v| !v.is_null())
                                                {
                                                    let wrapped = kb["wrapped_keys_b64"].as_str().unwrap_or("");
                                                    let salt = kb["kdf_salt_b64"].as_str().unwrap_or("");
                                                    let kdf_alg = kb["kdf_alg"].as_str();
                                                    let mem = kb["kdf_mem_kib"].as_i64().map(|n| n as i32);
                                                    let time = kb["kdf_time_cost"].as_i64().map(|n| n as i32);
                                                    let par = kb["kdf_parallelism"].as_i64().map(|n| n as i32);
                                                    let iters = kb["kdf_iterations"].as_i64().unwrap_or(50000) as i32;
                                                    match sync_manager::decrypt_keys_from_backup_v2(
                                                        &uid, wrapped, salt, kdf_alg, mem, time, par, iters, &pass,
                                                    ) {
                                                        Ok(restored) => {
                                                            if let Some(did) = DEVICE_TRUST_STATE().map(|d| d.device_id) {
                                                                recovery_proof = sign_device_recovery_proof(&restored, &did).ok();
                                                            }
                                                            recovered_keys = Some(restored);
                                                        }
                                                        Err(e) => {
                                                            status_msg.set(Some((true, e)));
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                            match storage_sync::recover_lost_devices_with_proof(
                                                c,
                                                false,
                                                recovery_proof.as_deref(),
                                                false,
                                            )
                                            .await
                                            {
                                                Ok(resp) => {
                                                    if let Some(restored) = recovered_keys {
                                                        *E2EE_RECOVERY_REQUIRED.write() = None;
                                                        on_keys_imported.call(restored);
                                                        status_msg.set(Some((false, "Unlocked messages and authenticated device".into())));
                                                    } else if let Some(kb) = resp.get("key_backup").filter(|v| !v.is_null()) {
                                                        let wrapped = kb["wrapped_keys_b64"].as_str().unwrap_or("");
                                                        let salt = kb["kdf_salt_b64"].as_str().unwrap_or("");
                                                        let kdf_alg = kb["kdf_alg"].as_str();
                                                        let mem = kb["kdf_mem_kib"].as_i64().map(|n| n as i32);
                                                        let time = kb["kdf_time_cost"].as_i64().map(|n| n as i32);
                                                        let par = kb["kdf_parallelism"].as_i64().map(|n| n as i32);
                                                        let iters = kb["kdf_iterations"].as_i64().unwrap_or(50000) as i32;
                                                        match sync_manager::decrypt_keys_from_backup_v2(
                                                            &uid, wrapped, salt, kdf_alg, mem, time, par, iters, &pass,
                                                        ) {
                                                            Ok(restored) => {
                                                                *E2EE_RECOVERY_REQUIRED.write() = None;
                                                                on_keys_imported.call(restored);
                                                                status_msg.set(Some((false, "Message unlocked".into())));
                                                            }
                                                            Err(e) => status_msg.set(Some((true, format!("Machine confirmed but {e}")))),
                                                        }
                                                    } else {
                                                        status_msg.set(Some((false, resp["honest_notice"].as_str().unwrap_or("Confirmed machine recovery").into())));
                                                    }
                                                    reload();
                                                }
                                                Err(e) => status_msg.set(Some((true, format!("Unable to restore: {}", e.code)))),
                                            }
                                        });
                                    },
                                    "Restore"
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
pub fn DeviceTrustPanel() -> Element {
    let mut list = use_signal(DevicesListResponse::default);
    let mut notice = use_signal(|| Option::<(bool, String)>::None);
    let mut code_input = use_signal(String::new);
    let current_trust = DEVICE_TRUST_STATE();

    let reload = move || {
        spawn(async move {
            if let Ok(d) = storage_sync::list_devices().await {
                list.set(d);
            }
        });
    };
    use_effect(move || reload());

    rsx! {
        if let Some(st) = current_trust {
            if st.trust_state == "pending" {
                div { class: "status-badge error",
                    "This device is waiting for confirmation from another trusted device. Confirmation code: {st.verification_code.clone().unwrap_or_default()}"
                }
            }
        }
        if !list().pending_requests.is_empty() {
            div { class: "settings-group",
                div { class: "settings-group-title", "Requires sign in from new device" }
                div { class: "settings-list",
                    for req in list().pending_requests {
                        div { key: "{req.id}", class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "{req.device_label.clone().unwrap_or_else(|| req.platform.clone())} · Confirmation code: {req.verification_code}" }
                                span { class: "settings-sublabel", "{req.user_agent.clone().unwrap_or_default()}" }
                            }
                            div { class: "inline-form",
                                input {
                                    r#type: "text",
                                    maxlength: "6",
                                    placeholder: "Enter the 6-digit code to confirm",
                                    value: "{code_input}",
                                    oninput: move |e| code_input.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: {
                                        let id = req.id.clone();
                                        let expected_code = req.verification_code.clone();
                                        let target_ecdh = req.ecdh_public_b64.clone();
                                        move |_| {
                                            let id = id.clone();
                                            let target_ecdh = target_ecdh.clone();
                                            let entered = code_input();
                                            let code_to_send = if entered.trim().is_empty() {
                                                expected_code.clone()
                                            } else {
                                                entered.trim().to_string()
                                            };
                                            spawn(async move {
                                                let wrapped = target_ecdh.as_deref().and_then(|pub_b64| {
                                                    let uid = api::auth_me_cached_id();
                                                    let my_keys = get_user_keys(uid.as_deref())?;
                                                    wrap_sync_key_for_device(&my_keys, pub_b64, &id).ok()
                                                });
                                                match storage_sync::approve_device_request(
                                                    &id,
                                                    Some(&code_to_send),
                                                    wrapped.as_deref(),
                                                )
                                                .await
                                                {
                                                    Ok(_) => notice.set(Some((false, "Approved and securely transferred E2EE keys to new devices".into()))),
                                                    Err(e) => notice.set(Some((true, format!("Code mismatch or expired: {}", e.code)))),
                                                }
                                                code_input.set(String::new());
                                                reload();
                                            });
                                        }
                                    },
                                    "Approval"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-danger-outline",
                                    onclick: {
                                        let id = req.id.clone();
                                        move |_| {
                                            let id = id.clone();
                                            spawn(async move {
                                                let _ = storage_sync::reject_device_request(&id).await;
                                                notice.set(Some((false, "Rejected new device".into())));
                                                reload();
                                            });
                                        }
                                    },
                                    "Refuse"
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((is_err, msg)) = notice() {
            div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{msg}" }
        }

        div { class: "settings-group",
            div { class: "settings-group-title", "List of devices with registered keys" }
            div { class: "settings-list",
                for dev in list().devices {
                    {
                        let dev_hint = format!(
                            "{} {} {}",
                            dev.platform,
                            dev.device_name.clone().unwrap_or_default(),
                            dev.device_label.clone().unwrap_or_default()
                        );
                        rsx! {
                            div { key: "{dev.id}", class: "settings-row cw-brand-row",
                                crate::components::brand_logos::DeviceBrandPair { text: dev_hint, size: 30 }
                                div { class: "settings-row-text",
                                    span { class: "settings-label",
                                        "{dev.device_label.clone().unwrap_or_else(|| dev.device_name.clone().unwrap_or_else(|| dev.platform.clone()))}"
                                        if dev.is_current {
                                            span { class: "chip-current",
                                                span { class: "cw-live-dot" }
                                                "This device"
                                            }
                                        }
                                    }
                                    span { class: "settings-sublabel",
                                        "{dev.platform} · "
                                        match dev.trust_state.as_str() {
                                            "trusted" => "Trust",
                                            "pending" => "Wait for confirmation",
                                            _ => "Revoked",
                                        }
                                    }
                                }
                                if !dev.is_current {
                                    button {
                                        r#type: "button",
                                        class: "btn-danger-outline",
                                        onclick: {
                                            let id = dev.id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    let _ = storage_sync::revoke_device(&id).await;
                                                    notice.set(Some((false, "Device revoked and sync permission terminated".into())));
                                                    reload();
                                                });
                                            }
                                        },
                                        "Withdrawal"
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
pub fn IncomingDeviceApprovalModal() -> Element {
    let alert = PENDING_APPROVAL_ALERT();
    let mut msg = use_signal(|| Option::<String>::None);
    let Some(req) = alert else {
        return rsx! {};
    };

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-dialog", role: "dialog", aria_modal: "true", aria_label: "Confirm new device login",
                div { class: "modal-header",
                    h2 { class: "modal-title", "The new device is asking to sign in" }
                }
                div { class: "modal-body",
                    p { "Device: {req.device_name} · {req.platform}" }
                    p { "If it's you, compare the confirmation code: ", strong { "{req.verification_code}" } }
                    if let Some(m) = msg() {
                        div { class: "status-badge error", "{m}" }
                    }
                }
                div { class: "modal-footer",
                    button {
                        r#type: "button",
                        class: "btn-danger-outline",
                        onclick: {
                            let id = req.request_id.clone();
                            move |_| {
                                let id = id.clone();
                                spawn(async move {
                                    let _ = storage_sync::reject_device_request(&id).await;
                                    *PENDING_APPROVAL_ALERT.write() = None;
                                });
                            }
                        },
                        "Refuse"
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: {
                            let id = req.request_id.clone();
                            let code = req.verification_code.clone();
                            let target_ecdh = req.ecdh_public_b64.clone();
                            move |_| {
                                let id = id.clone();
                                let code = code.clone();
                                let target_ecdh = target_ecdh.clone();
                                spawn(async move {
                                    let wrapped = target_ecdh.as_deref().and_then(|pub_b64| {
                                        let uid = api::auth_me_cached_id();
                                        let my_keys = get_user_keys(uid.as_deref())?;
                                        wrap_sync_key_for_device(&my_keys, pub_b64, &id).ok()
                                    });
                                    match storage_sync::approve_device_request(
                                        &id,
                                        Some(&code),
                                        wrapped.as_deref(),
                                    )
                                    .await
                                    {
                                        Ok(_) => *PENDING_APPROVAL_ALERT.write() = None,
                                        Err(e) => msg.set(Some(e.code)),
                                    }
                                });
                            }
                        },
                        "Accept synchronization"
                    }
                }
            }
        }
    }
}

/// Modal displayed when an existing account (`me.has_keys == true`) signs in on a device whose local
/// E2EE vault is locked with Argon2id or missing keys — preventing silent identity key overwrite.
#[component]
pub fn E2eeUnlockRecoveryModal(
    current_user_id: String,
    on_unlocked: EventHandler<UserKeys>,
) -> Element {
    let reason = E2EE_RECOVERY_REQUIRED();
    let mut pass = use_signal(String::new);
    let mut err = use_signal(|| Option::<String>::None);
    let mut confirm_reset = use_signal(|| false);
    let trust_info = DEVICE_TRUST_STATE();

    // Race-free background sync watcher: if a trusted device approves this device and delivers
    // `wrapped_sync_key` (even before WS connects or across reloads), automatically unwrap & unlock!
    let kind_opt = reason.clone();
    let uid_poll = current_user_id.clone();
    use_effect(use_reactive!(|kind_opt, uid_poll| {
        let Some(k) = kind_opt else {
            return;
        };
        if k == "locked_vault" || uid_poll.is_empty() {
            return;
        }
        let uid = uid_poll.clone();
        spawn(async move {
            for _ in 0..120 {
                platform::sleep_ms(1200).await;
                if E2EE_RECOVERY_REQUIRED.peek().is_none() {
                    break;
                }
                if let Ok(cur) = storage_sync::current_device_status().await
                    && let Some(lr) = cur.get("latest_request")
                    && lr.get("status").and_then(|v| v.as_str()) == Some("approved")
                    && let Some(wsk) = lr.get("wrapped_sync_key").and_then(|v| v.as_str())
                    && !wsk.is_empty()
                {
                    let req_id = lr.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let expected_ecdsa = api::keys_get(&uid).await.ok().map(|k| k.ecdsa_public);
                    if let Ok(recovered) = crate::crypto::e2ee::unwrap_sync_key_on_device(
                        &uid,
                        wsk,
                        req_id,
                        expected_ecdsa.as_deref(),
                    ) {
                        DEVICE_TRUST_STATE.with_mut(|st| {
                            if let Some(d) = st.as_mut() {
                                d.trust_state = "trusted".into();
                            }
                        });
                        *E2EE_RECOVERY_REQUIRED.write() = None;
                        on_unlocked.call(recovered);
                        break;
                    }
                }
            }
        });
    }));

    let Some(kind) = reason else {
        return rsx! {};
    };
    let is_locked_vault = kind == "locked_vault";
    let uid_unlock = current_user_id.clone();
    let uid_reset = current_user_id.clone();

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-dialog", role: "dialog", aria_modal: "true", aria_label: "Unlock end-to-end encryption",
                div { class: "modal-header",
                    h2 { class: "modal-title",
                        if is_locked_vault {
                            "Unlock E2EE repository (Argon2id)"
                        } else {
                            "Synchronize or restore E2EE keys"
                        }
                    }
                }
                div { class: "modal-body",
                    if is_locked_vault {
                        p { "The E2EE private key on this device is protected by the Argon2id password. Enter the password to decrypt the message." }
                    } else {
                        p { "This account has end-to-end encryption enabled on another device. ChatW does not override the lock itself to protect the entire message history." }
                        if let Some(st) = trust_info.clone() {
                            if let Some(code) = st.verification_code {
                                p { "Method 1 — Approve on old device logging in with confirmation code: ", strong { "{code}" } }
                            }
                        }
                        p { "Method 2 — Enter the E2EE key backup password (Argon2id) to decrypt directly on this machine:" }
                    }
                    input {
                        r#type: "password",
                        placeholder: if is_locked_vault { "Enter the E2EE unlock password" } else { "Enter the E2EE key backup password" },
                        aria_label: "E2EE unlock password",
                        value: "{pass}",
                        oninput: move |e| {
                            pass.set(e.value());
                            err.set(None);
                        },
                    }
                    if let Some(m) = err() {
                        div { class: "status-badge error", "{m}" }
                    }
                    if !is_locked_vault && confirm_reset() {
                        div { class: "status-badge error",
                            "Warning: Generating a new E2EE key will not be able to re-encrypt old messages if you no longer have the old device or a backup."
                        }
                    }
                }
                div { class: "modal-footer",
                    if !is_locked_vault {
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            onclick: move |_| *E2EE_RECOVERY_REQUIRED.write() = None,
                            "Later"
                        }
                        if confirm_reset() {
                            button {
                                r#type: "button",
                                class: "btn-danger",
                                onclick: move |_| {
                                    let uid = uid_reset.clone();
                                    spawn(async move {
                                        match create_and_store_user_keys(Some(&uid)) {
                                            Ok(new_keys) => {
                                                match api::keys_register_rotated(
                                                    &new_keys.ecdh_raw_b64,
                                                    &new_keys.ecdsa_raw_b64,
                                                    true,
                                                    None,
                                                )
                                                .await
                                                {
                                                    Ok(_) => {
                                                        *E2EE_RECOVERY_REQUIRED.write() = None;
                                                        on_unlocked.call(new_keys);
                                                    }
                                                    Err(e) => err.set(Some(format!("Server refused to reset key: {}", e.code))),
                                                }
                                            }
                                            Err(e) => err.set(Some(e)),
                                        }
                                    });
                                },
                                "Confirm creation of new key"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "btn-danger-outline",
                                onclick: move |_| confirm_reset.set(true),
                                "Lost all devices · Create new key"
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| {
                            let uid = uid_unlock.clone();
                            let p = pass();
                            let locked = is_locked_vault;
                            spawn(async move {
                                if locked {
                                    match unlock_passphrase_vault(&uid, &p) {
                                        Ok(k) => {
                                            *E2EE_RECOVERY_REQUIRED.write() = None;
                                            on_unlocked.call(k);
                                        }
                                        Err(e) => err.set(Some(e)),
                                    }
                                } else {
                                    match storage_sync::get_key_backup().await {
                                        Ok(bk) => {
                                            if let Some(kb) = bk.get("backup").filter(|v| !v.is_null()) {
                                                let wrapped = kb["wrapped_keys_b64"].as_str().unwrap_or("");
                                                let salt = kb["kdf_salt_b64"].as_str().unwrap_or("");
                                                let kdf_alg = kb["kdf_alg"].as_str();
                                                let mem = kb["kdf_mem_kib"].as_i64().map(|n| n as i32);
                                                let time = kb["kdf_time_cost"].as_i64().map(|n| n as i32);
                                                let par = kb["kdf_parallelism"].as_i64().map(|n| n as i32);
                                                let iters = kb["kdf_iterations"].as_i64().unwrap_or(50000) as i32;
                                                match sync_manager::decrypt_keys_from_backup_v2(
                                                    &uid, wrapped, salt, kdf_alg, mem, time, par, iters, &p,
                                                ) {
                                                    Ok(restored) => {
                                                        if let Some(did) = DEVICE_TRUST_STATE().map(|d| d.device_id) {
                                                            if let Ok(proof) = sign_device_recovery_proof(&restored, &did) {
                                                                let _ = storage_sync::recover_lost_devices_with_proof(
                                                                    None,
                                                                    false,
                                                                    Some(&proof),
                                                                    false,
                                                                )
                                                                .await;
                                                            }
                                                        }
                                                        *E2EE_RECOVERY_REQUIRED.write() = None;
                                                        on_unlocked.call(restored);
                                                    }
                                                    Err(e) => err.set(Some(e)),
                                                }
                                            } else {
                                                err.set(Some("The account has not saved a copy of the key on the server. Get approval from the old device.".into()));
                                            }
                                        }
                                        Err(e) => err.set(Some(format!("Failed to load key copy: {}", e.code))),
                                    }
                                }
                            });
                        },
                        "Unlock"
                    }
                }
            }
        }
    }
}

