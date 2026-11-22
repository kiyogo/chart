use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{models::ApiError, platform};

async fn req_json<T: serde::de::DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StoragePolicy {
    pub cache_limit_mb: i32,
    pub auto_download_mode: String,
    pub dm_relay_ttl_hours: i32,
    pub dm_encrypted_backup: bool,
    pub require_device_approval: bool,
    pub prefer_original_upload: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConvStoragePolicy {
    pub allow_peer_recovery: bool,
    pub dm_encrypted_backup: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub id: String,
    pub is_current: bool,
    #[serde(default)]
    pub client_device_id: Option<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub device_label: Option<String>,
    pub platform: String,
    pub trust_state: String,
    pub has_key: bool,
    #[serde(default)]
    pub last_seen_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceApprovalPending {
    pub id: String,
    pub requesting_device_id: String,
    #[serde(default)]
    pub device_label: Option<String>,
    pub platform: String,
    pub verification_code: String,
    #[serde(default)]
    pub ecdh_public_b64: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DevicesListResponse {
    pub devices: Vec<DeviceEntry>,
    pub pending_requests: Vec<DeviceApprovalPending>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecoveryRequestEntry {
    pub id: String,
    pub ref_id: String,
    pub requester_user_id: String,
    pub target_user_id: String,
    pub is_incoming: bool,
    pub transport_mode: String,
    pub status: String,
    #[serde(default)]
    pub relay_ref_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    pub file_name: String,
    pub mime_type: String,
    pub byte_size: i64,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecoveryListResponse {
    pub requests: Vec<RecoveryRequestEntry>,
}

pub async fn get_capabilities() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/storage/capabilities", None).await
}

pub async fn get_policy() -> Result<StoragePolicy, ApiError> {
    req_json("GET", "/api/storage/policy", None).await
}

pub async fn update_policy(policy: &StoragePolicy) -> Result<StoragePolicy, ApiError> {
    req_json("PUT", "/api/storage/policy", Some(serde_json::to_value(policy).unwrap_or_default())).await
}

pub async fn get_conv_policy(peer_id: &str) -> Result<ConvStoragePolicy, ApiError> {
    req_json("GET", &format!("/api/storage/conversations/{}", urlencoding::encode(peer_id)), None).await
}

pub async fn update_conv_policy(peer_id: &str, policy: &ConvStoragePolicy) -> Result<ConvStoragePolicy, ApiError> {
    req_json(
        "PUT",
        &format!("/api/storage/conversations/{}", urlencoding::encode(peer_id)),
        Some(serde_json::to_value(policy).unwrap_or_default()),
    )
    .await
}

pub async fn get_metrics() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/storage/metrics", None).await
}

pub async fn trigger_cleanup() -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/cleanup", None).await
}

pub async fn run_migration_job(dry_run: bool) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/migration/run", Some(json!({ "dry_run": dry_run }))).await
}

pub async fn get_ref_meta(ref_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("GET", &format!("/api/storage/references/{}/meta", urlencoding::encode(ref_id)), None).await
}

pub async fn ack_relay_copy(ref_id: &str, local_tier: &str, sha256_hex: &str, has_original: bool) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/storage/relay/{}/ack", urlencoding::encode(ref_id)),
        Some(json!({
            "local_tier": local_tier,
            "sha256_hex": sha256_hex,
            "has_original": has_original,
        })),
    )
    .await
}

pub async fn delete_reference(ref_id: &str, scope: &str, force: bool) -> Result<serde_json::Value, ApiError> {
    req_json(
        "DELETE",
        &format!(
            "/api/storage/references/{}?scope={}&force={}",
            urlencoding::encode(ref_id),
            urlencoding::encode(scope),
            force
        ),
        None,
    )
    .await
}

pub async fn forward_reference(
    from_ref_id: &str,
    target_scope_type: &str,
    target_scope_id: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/storage/forward",
        Some(json!({
            "from_ref_id": from_ref_id,
            "target_scope_type": target_scope_type,
            "target_scope_id": target_scope_id,
        })),
    )
    .await
}

pub async fn upload_attachment_scoped(
    bytes: Vec<u8>,
    content_type: &str,
    scope_type: &str,
    scope_id: Option<&str>,
    file_name: Option<&str>,
    mime_type: Option<&str>,
    sha256_plain: Option<&str>,
    is_e2ee: bool,
    preserve_original: bool,
) -> Result<serde_json::Value, ApiError> {
    let mut qs = vec![
        format!("scope_type={}", urlencoding::encode(scope_type)),
        format!("is_e2ee={is_e2ee}"),
        format!("preserve_original={preserve_original}"),
    ];
    if let Some(sid) = scope_id {
        qs.push(format!("scope_id={}", urlencoding::encode(sid)));
    }
    if let Some(fnm) = file_name {
        qs.push(format!("file_name={}", urlencoding::encode(fnm)));
    }
    if let Some(mt) = mime_type {
        qs.push(format!("mime_type={}", urlencoding::encode(mt)));
    }
    if let Some(sp) = sha256_plain {
        qs.push(format!("sha256_plain={}", urlencoding::encode(sp)));
    }
    let path = format!("/api/attachments?{}", qs.join("&"));
    let (_, body) = platform::http_request("POST", &path, None, Some((bytes, content_type))).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

pub async fn register_device(
    client_device_id: &str,
    platform_kind: &str,
    device_name: &str,
    ecdh_pub_b64: Option<&str>,
    ecdsa_pub_b64: Option<&str>,
    fingerprint_b64: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/devices/register",
        Some(json!({
            "client_device_id": client_device_id,
            "platform": platform_kind,
            "device_name": device_name,
            "ecdh_public_b64": ecdh_pub_b64,
            "ecdsa_public_b64": ecdsa_pub_b64,
            "key_fingerprint_b64": fingerprint_b64,
        })),
    )
    .await
}

pub async fn current_device_status() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/devices/current", None).await
}

pub async fn list_devices() -> Result<DevicesListResponse, ApiError> {
    req_json("GET", "/api/devices", None).await
}

pub async fn approve_device_request(
    request_id: &str,
    verification_code: Option<&str>,
    wrapped_sync_key: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/devices/requests/{}/approve", urlencoding::encode(request_id)),
        Some(json!({
            "verification_code": verification_code,
            "wrapped_sync_key": wrapped_sync_key,
        })),
    )
    .await
}

pub async fn reject_device_request(request_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/devices/requests/{}/reject", urlencoding::encode(request_id)),
        Some(json!({})),
    )
    .await
}

pub async fn revoke_device(device_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("DELETE", &format!("/api/devices/{}", urlencoding::encode(device_id)), None).await
}

pub async fn recover_lost_devices(code: Option<&str>, revoke_others: bool) -> Result<serde_json::Value, ApiError> {
    recover_lost_devices_with_proof(code, revoke_others, None, false).await
}

pub async fn recover_lost_devices_with_proof(
    code: Option<&str>,
    revoke_others: bool,
    recovery_proof_b64: Option<&str>,
    confirm_emergency_reset: bool,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/devices/recover-lost",
        Some(json!({
            "totp_or_backup_code": code,
            "revoke_unreachable_devices": revoke_others,
            "recovery_proof_b64": recovery_proof_b64,
            "confirm_emergency_reset": confirm_emergency_reset,
        })),
    )
    .await
}

pub async fn put_key_backup(
    wrapped_keys_b64: &str,
    kdf_salt_b64: &str,
    kdf_iterations: i32,
    key_fingerprint_b64: &str,
) -> Result<serde_json::Value, ApiError> {
    put_key_backup_v2(
        wrapped_keys_b64,
        kdf_salt_b64,
        "argon2id-v1",
        19_456,
        2,
        1,
        kdf_iterations,
        key_fingerprint_b64,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn put_key_backup_v2(
    wrapped_keys_b64: &str,
    kdf_salt_b64: &str,
    kdf_alg: &str,
    kdf_mem_kib: i32,
    kdf_time_cost: i32,
    kdf_parallelism: i32,
    kdf_iterations: i32,
    key_fingerprint_b64: &str,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "PUT",
        "/api/keys/backup",
        Some(json!({
            "wrapped_keys_b64": wrapped_keys_b64,
            "kdf_salt_b64": kdf_salt_b64,
            "kdf_alg": kdf_alg,
            "kdf_mem_kib": kdf_mem_kib,
            "kdf_time_cost": kdf_time_cost,
            "kdf_parallelism": kdf_parallelism,
            "kdf_iterations": kdf_iterations,
            "key_fingerprint_b64": key_fingerprint_b64,
        })),
    )
    .await
}

pub async fn get_key_backup() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/keys/backup", None).await
}

pub async fn delta_sync(dm_since: i64, group_since: i64, channel_since: i64) -> Result<serde_json::Value, ApiError> {
    req_json(
        "GET",
        &format!("/api/sync/delta?dm_since={dm_since}&group_since={group_since}&channel_since={channel_since}&limit=100"),
        None,
    )
    .await
}

pub async fn report_locations(items: Vec<serde_json::Value>) -> Result<serde_json::Value, ApiError> {
    req_json("PUT", "/api/storage/locations/report", Some(json!({ "items": items }))).await
}

pub async fn list_recovery_requests() -> Result<RecoveryListResponse, ApiError> {
    req_json("GET", "/api/recovery/requests", None).await
}

pub async fn create_recovery_request(ref_id: &str, preferred_transport: Option<&str>) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/recovery/requests",
        Some(json!({
            "ref_id": ref_id,
            "preferred_transport": preferred_transport,
        })),
    )
    .await
}

pub async fn respond_recovery_request(
    request_id: &str,
    action: &str,
    transport_mode: Option<&str>,
    relay_ref_id: Option<&str>,
    reason: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/recovery/requests/{}/respond", urlencoding::encode(request_id)),
        Some(json!({
            "action": action,
            "transport_mode": transport_mode,
            "relay_ref_id": relay_ref_id,
            "reason": reason,
        })),
    )
    .await
}

pub async fn complete_recovery_request(
    request_id: &str,
    sha256_hex: Option<&str>,
    local_tier: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/recovery/requests/{}/complete", urlencoding::encode(request_id)),
        Some(json!({
            "sha256_hex": sha256_hex,
            "local_tier": local_tier,
        })),
    )
    .await
}
