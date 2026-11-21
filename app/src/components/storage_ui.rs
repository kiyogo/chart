use dioxus::prelude::*;

use crate::{
    crypto::e2ee::{UserKeys, import_user_keys_json},
    platform,
    services::storage_sync::{self, DevicesListResponse, RecoveryListResponse, StoragePolicy},
    storage::{
        DEVICE_TRUST_STATE, PENDING_APPROVAL_ALERT, SYNC_PROGRESS, LocalQuotaInfo,
        availability_badge_label, clear_safe_cache, format_bytes_human, get_quota_estimate,
        recovery, sync_manager,
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
            div { class: "status-badge error", "Cảnh báo: có {at_risk} tệp chat riêng sắp hết hạn mà chưa đủ thiết bị lưu bản sao." }
        }
        div { class: "settings-cols",
            div { class: "settings-col",
                div { class: "settings-group",
                    div { class: "settings-group-title", "Bộ nhớ trên máy này" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Lưu trữ trên máy" }
                                span { class: "settings-sublabel",
                                    if q.persisted { "Lưu bền vững đã bật" } else { "Chế độ lưu trình duyệt" }
                                }
                            }
                            span { class: "chip-privacy", "{q.local_object_count} tệp · {format_bytes_human(q.total_local_bytes)}" }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Bộ nhớ đệm có thể dọn" }
                                span { class: "settings-sublabel", "Tệp nhóm và máy chủ: {format_bytes_human(q.disposable_cache_bytes)}" }
                            }
                            button {
                                r#type: "button",
                                class: "btn-secondary",
                                onclick: move |_| {
                                    spawn(async move {
                                        match clear_safe_cache(false).await {
                                            Ok((freed, kept)) => {
                                                notice.set(Some((false, format!("Đã giải phóng {}, giữ nguyên {kept} tệp riêng chưa sao lưu", format_bytes_human(freed)))));
                                                reload();
                                            }
                                            Err(e) => notice.set(Some((true, format!("Không dọn được đệm: {e}")))),
                                        }
                                    });
                                },
                                "Xóa bộ nhớ đệm"
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Tệp gốc chỉ có trên máy này" }
                                span { class: "settings-sublabel", "Được giữ an toàn khi dọn đệm: {format_bytes_human(q.pinned_unique_bytes)}" }
                            }
                            if confirm_purge_unique() {
                                div { class: "inline-form",
                                    button { r#type: "button", class: "btn-ghost", onclick: move |_| confirm_purge_unique.set(false), "Hủy" }
                                    button {
                                        r#type: "button",
                                        class: "btn-danger",
                                        onclick: move |_| {
                                            confirm_purge_unique.set(false);
                                            spawn(async move {
                                                let _ = clear_safe_cache(true).await;
                                                notice.set(Some((false, "Đã xóa toàn bộ bản lưu trên máy này".into())));
                                                reload();
                                            });
                                        },
                                        "Xác nhận xóa"
                                    }
                                }
                            } else if q.pinned_unique_bytes > 0 {
                                button { r#type: "button", class: "btn-danger-outline", onclick: move |_| confirm_purge_unique.set(true), "Xóa bản lưu duy nhất" }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Giới hạn bộ nhớ đệm" }
                                span { class: "settings-sublabel", "Tự động dọn đệm khi vượt mức chọn" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Giới hạn bộ nhớ đệm",
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
                                option { value: "1024", "1 GB · Mặc định" }
                                option { value: "2048", "2 GB" }
                                option { value: "5120", "5 GB" }
                                option { value: "10240", "10 GB" }
                                option { value: "0", "Không giới hạn" }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Tự động tải ảnh và video" }
                                span { class: "settings-sublabel", "Chọn lúc tự động tải tệp về máy" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Chế độ tự động tải",
                                value: "{p.auto_download_mode}",
                                onchange: move |e| {
                                    let mut next = policy();
                                    next.auto_download_mode = e.value();
                                    policy.set(next.clone());
                                    spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                },
                                option { value: "always", "Luôn tự động" }
                                option { value: "wifi", "Ưu tiên Wi-Fi" }
                                option { value: "manual", "Chỉ tải khi mở" }
                            }
                        }
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Giữ ảnh gốc khi gửi" }
                                span { class: "settings-sublabel", "Tắt để tự nén ảnh gọn nhẹ mà vẫn rõ nét" }
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
                    div { class: "settings-group-title", "Dung lượng đang dùng" }
                    div { class: "settings-list",
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Theo cuộc trò chuyện" }
                                span { class: "settings-sublabel",
                                    "Chat riêng: {format_bytes_human(dm_bytes)} · Nhóm: {format_bytes_human(group_bytes)} · Máy chủ: {format_bytes_human(channel_bytes)}"
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Dung lượng đã tiết kiệm" }
                                span { class: "settings-sublabel",
                                    "Trùng lặp: {format_bytes_human(dedup_saved)} · Nén tệp: {format_bytes_human(comp_saved)}"
                                }
                            }
                            span { class: "chip-privacy", "Đang lưu: {format_bytes_human(server_physical)}" }
                        }
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Hệ thống lưu trữ" }
                                span { class: "settings-sublabel",
                                    if s3_ready { "Đã kết nối kho lưu trữ mở rộng" } else { "Lưu trực tiếp trên máy chủ" }
                                }
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
                                    "Kiểm tra dữ liệu"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-ghost",
                                    onclick: move |_| {
                                        spawn(async move {
                                            let _ = storage_sync::trigger_cleanup().await;
                                            reload();
                                        });
                                    },
                                    "Dọn tệp hết hạn"
                                }
                            }
                        }
                        if let Some(mr) = migration_res() {
                            div { class: "settings-row",
                                span { class: "settings-sublabel",
                                    "Đã kiểm tra {mr[\"scanned_objects\"]} tệp · Hợp lệ {mr[\"verified_objects\"]} · Lỗi {mr[\"failed_objects\"]}"
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Sao lưu an toàn" }
                                span { class: "settings-sublabel", "Bạn có thể tải tệp sao lưu về máy hoặc lưu lên đám mây ở mục Sao lưu và Đám mây." }
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
                    div { class: "settings-group-title", "Đồng bộ nhiều thiết bị" }
                    div { class: "settings-list",
                        div { class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Trạng thái đồng bộ" }
                                span { class: "settings-sublabel",
                                    if progress.phase_a_done { "Đã đồng bộ · {progress.last_sync_label}" } else { "Sẵn sàng đồng bộ dữ liệu mới" }
                                }
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| sync_manager::toggle_phase_b_pause(),
                                    if progress.phase_b_paused { "Tiếp tục tải nền" } else { "Tạm dừng tải nền" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        spawn(async move {
                                            match sync_manager::run_incremental_delta_sync().await {
                                                Ok(()) => {
                                                    let _ = sync_manager::report_cached_copies_to_server().await;
                                                    status_msg.set(Some((false, "Đã đồng bộ xong dữ liệu mới".into())));
                                                }
                                                Err(e) => status_msg.set(Some((true, format!("Chưa đồng bộ được: {e}")))),
                                            }
                                        });
                                    },
                                    "Đồng bộ ngay"
                                }
                            }
                        }
                        label { class: "settings-row",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "Hỏi xác nhận khi đăng nhập máy mới" }
                                span { class: "settings-sublabel", "Máy mới cần nhập mã 6 số từ máy cũ để tải dữ liệu" }
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
                                span { class: "settings-label", "Lưu bản sao mã hóa cho tệp chat riêng" }
                                span { class: "settings-sublabel", "Giữ bản mã hóa trên máy chủ phòng khi mất hết máy" }
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
                                span { class: "settings-label", "Thời gian giữ tệp gửi tạm" }
                                span { class: "settings-sublabel", "Tự xóa sau khi máy nhận đã tải về" }
                            }
                            select {
                                class: "settings-select",
                                aria_label: "Thời hạn trung chuyển",
                                value: "{p.dm_relay_ttl_hours}",
                                onchange: move |e| {
                                    if let Ok(hrs) = e.value().parse::<i32>() {
                                        let mut next = policy();
                                        next.dm_relay_ttl_hours = hrs;
                                        policy.set(next.clone());
                                        spawn(async move { let _ = storage_sync::update_policy(&next).await; });
                                    }
                                },
                                option { value: "24", "24 giờ" }
                                option { value: "72", "3 ngày" }
                                option { value: "168", "7 ngày · Mặc định" }
                                option { value: "720", "30 ngày" }
                            }
                        }
                    }
                }

                div { class: "settings-group",
                    div { class: "settings-group-title", "Yêu cầu khôi phục tệp" }
                    div { class: "settings-list",
                        if requests().requests.is_empty() {
                            div { class: "settings-row",
                                span { class: "settings-sublabel", "Chưa có yêu cầu khôi phục tệp nào." }
                            }
                        } else {
                            for rr in requests().requests {
                                div { key: "{rr.id}", class: "settings-row",
                                    div { class: "settings-row-text",
                                        span { class: "settings-label", "{rr.file_name} · {format_bytes_human(rr.byte_size.max(0) as u64)}" }
                                        span { class: "settings-sublabel",
                                            if rr.is_incoming { "Yêu cầu đến · " } else { "Yêu cầu của bạn · " }
                                            "{availability_badge_label(&rr.status)}"
                                        }
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
                                                "Gửi khôi phục"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: {
                                                    let rid = rr.id.clone();
                                                    move |_| {
                                                        let rid = rid.clone();
                                                        spawn(async move {
                                                            let _ = storage_sync::respond_recovery_request(&rid, "deny", None, None, Some("Người dùng từ chối")).await;
                                                            reload();
                                                        });
                                                    }
                                                },
                                                "Từ chối"
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
                    div { class: "settings-group-title", "Khôi phục khi mất máy" }
                    div { class: "settings-list",
                        if let Some(k) = user_keys.clone() {
                            div { class: "settings-row column-layout",
                                div { class: "settings-label",
                                    if has_backup() { "Đã lưu bản sao chìa khóa an toàn trên máy chủ" } else { "Lưu bản sao chìa khóa bằng mật khẩu riêng" }
                                }
                                div { class: "inline-form",
                                    input {
                                        r#type: "password",
                                        placeholder: "Mật khẩu từ 6 ký tự trở lên",
                                        aria_label: "Mật khẩu khôi phục khóa",
                                        value: "{passphrase}",
                                        oninput: move |e| passphrase.set(e.value()),
                                    }
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        onclick: move |_| {
                                            let keys = k.clone();
                                            let pass = passphrase();
                                            spawn(async move {
                                                match sync_manager::encrypt_keys_for_backup(&keys, &pass) {
                                                    Ok((wrapped, salt, iters)) => {
                                                        match storage_sync::put_key_backup(&wrapped, &salt, iters, &keys.fingerprint_b64).await {
                                                            Ok(_) => {
                                                                passphrase.set(String::new());
                                                                has_backup.set(true);
                                                                status_msg.set(Some((false, "Đã lưu bản sao chìa khóa thành công".into())));
                                                            }
                                                            Err(e) => status_msg.set(Some((true, format!("Lỗi sao lưu: {}", e.code)))),
                                                        }
                                                    }
                                                    Err(e) => status_msg.set(Some((true, e))),
                                                }
                                            });
                                        },
                                        "Lưu bản sao khóa"
                                    }
                                }
                            }
                        }
                        div { class: "settings-row column-layout",
                            div { class: "settings-label", "Khôi phục lại quyền đọc tin nhắn" }
                            span { class: "settings-sublabel", "Nhập mã xác minh nếu có và mật khẩu khôi phục bạn đã đặt." }
                            div { class: "inline-form",
                                input {
                                    r#type: "text",
                                    placeholder: "Mã 6 số hoặc mã dự phòng",
                                    aria_label: "Mã xác minh mất thiết bị",
                                    value: "{twofa_code}",
                                    oninput: move |e| twofa_code.set(e.value()),
                                }
                                input {
                                    r#type: "password",
                                    placeholder: "Mật khẩu khôi phục khóa",
                                    aria_label: "Mật khẩu giải mã khóa sao lưu",
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
                                            match storage_sync::recover_lost_devices(c, false).await {
                                                Ok(resp) => {
                                                    if let Some(kb) = resp.get("key_backup").filter(|v| !v.is_null()) {
                                                        let wrapped = kb["wrapped_keys_b64"].as_str().unwrap_or("");
                                                        let salt = kb["kdf_salt_b64"].as_str().unwrap_or("");
                                                        let iters = kb["kdf_iterations"].as_i64().unwrap_or(50000) as i32;
                                                        match sync_manager::decrypt_keys_from_backup(&uid, wrapped, salt, iters, &pass) {
                                                            Ok(restored) => {
                                                                on_keys_imported.call(restored);
                                                                status_msg.set(Some((false, "Đã xác nhận máy mới và mở khóa tin nhắn thành công".into())));
                                                            }
                                                            Err(e) => status_msg.set(Some((true, format!("Đã xác nhận máy nhưng {e}")))),
                                                        }
                                                    } else {
                                                        status_msg.set(Some((false, resp["honest_notice"].as_str().unwrap_or("Đã xác nhận máy khôi phục").into())));
                                                    }
                                                    let _ = import_user_keys_json;
                                                    reload();
                                                }
                                                Err(e) => status_msg.set(Some((true, format!("Không khôi phục được: {}", e.code)))),
                                            }
                                        });
                                    },
                                    "Khôi phục quyền truy cập"
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
                    "Thiết bị này đang chờ xác nhận từ thiết bị tin cậy khác. Mã xác nhận: {st.verification_code.clone().unwrap_or_default()}"
                }
            }
        }
        if !list().pending_requests.is_empty() {
            div { class: "settings-group",
                div { class: "settings-group-title", "Yêu cầu đăng nhập từ thiết bị mới" }
                div { class: "settings-list",
                    for req in list().pending_requests {
                        div { key: "{req.id}", class: "settings-row column-layout",
                            div { class: "settings-row-text",
                                span { class: "settings-label", "{req.device_label.clone().unwrap_or_else(|| req.platform.clone())} · Mã xác nhận: {req.verification_code}" }
                                span { class: "settings-sublabel", "{req.user_agent.clone().unwrap_or_default()}" }
                            }
                            div { class: "inline-form",
                                input {
                                    r#type: "text",
                                    maxlength: "6",
                                    placeholder: "Nhập mã 6 số để xác nhận",
                                    value: "{code_input}",
                                    oninput: move |e| code_input.set(e.value()),
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    onclick: {
                                        let id = req.id.clone();
                                        move |_| {
                                            let id = id.clone();
                                            let code = code_input();
                                            spawn(async move {
                                                let c = if code.trim().is_empty() { None } else { Some(code.trim()) };
                                                match storage_sync::approve_device_request(&id, c, None).await {
                                                    Ok(_) => notice.set(Some((false, "Đã phê duyệt thiết bị mới".into()))),
                                                    Err(e) => notice.set(Some((true, format!("Mã không khớp hoặc hết hạn: {}", e.code)))),
                                                }
                                                code_input.set(String::new());
                                                reload();
                                            });
                                        }
                                    },
                                    "Phê duyệt"
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
                                                notice.set(Some((false, "Đã từ chối thiết bị mới".into())));
                                                reload();
                                            });
                                        }
                                    },
                                    "Từ chối"
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
            div { class: "settings-group-title", "Danh sách thiết bị đã đăng ký khóa" }
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
                                                "Thiết bị này"
                                            }
                                        }
                                    }
                                    span { class: "settings-sublabel",
                                        "{dev.platform} · "
                                        match dev.trust_state.as_str() {
                                            "trusted" => "Tin cậy",
                                            "pending" => "Chờ xác nhận",
                                            _ => "Đã thu hồi",
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
                                                    notice.set(Some((false, "Đã thu hồi thiết bị và chấm dứt quyền đồng bộ".into())));
                                                    reload();
                                                });
                                            }
                                        },
                                        "Thu hồi"
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
    let Some(req) = alert else {
        return rsx! {};
    };
    let mut msg = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-dialog", role: "dialog", aria_modal: "true", aria_label: "Xác nhận đăng nhập thiết bị mới",
                div { class: "modal-header",
                    h2 { class: "modal-title", "Thiết bị mới đang yêu cầu đăng nhập" }
                }
                div { class: "modal-body",
                    p { "Thiết bị: {req.device_name} · {req.platform}" }
                    p { "Nếu đúng là bạn, đối chiếu mã xác nhận: ", strong { "{req.verification_code}" } }
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
                        "Từ chối"
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: {
                            let id = req.request_id.clone();
                            let code = req.verification_code.clone();
                            move |_| {
                                let id = id.clone();
                                let code = code.clone();
                                spawn(async move {
                                    match storage_sync::approve_device_request(&id, Some(&code), None).await {
                                        Ok(_) => *PENDING_APPROVAL_ALERT.write() = None,
                                        Err(e) => msg.set(Some(e.code)),
                                    }
                                });
                            }
                        },
                        "Chấp nhận đồng bộ"
                    }
                }
            }
        }
    }
}
