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

// =========================================================================
// Re-authentication & Export Authorization
// =========================================================================

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReauthMethods {
    pub has_export_password: bool,
    pub totp_enabled: bool,
    pub backup_codes_remaining: i64,
    pub passkeys_count: i64,
    pub has_google_account: bool,
    #[serde(default)]
    pub locked_until: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReauthResponse {
    pub authorization_id: String,
    pub export_token: String,
    pub scope: String,
    pub expires_in_secs: i64,
    pub requires_multi_step: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExportPreview {
    #[serde(default)]
    pub account_username: Option<String>,
    pub dm_messages: i64,
    pub group_messages: i64,
    pub channel_messages: i64,
    pub durable_media_count: i64,
    pub durable_media_bytes: i64,
    pub missing_or_client_only_count: i64,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub honest_notice: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UnencryptedZipStepResponse {
    pub ok: bool,
    #[serde(default)]
    pub preview: Option<ExportPreview>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportScopeOptions {
    pub include_account: bool,
    pub include_messages: bool,
    pub include_media: bool,
    pub include_settings: bool,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

impl Default for ExportScopeOptions {
    fn default() -> Self {
        Self {
            include_account: true,
            include_messages: true,
            include_media: true,
            include_settings: true,
            conversation_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MissingMediaItem {
    pub ref_id: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InspectArchiveResponse {
    pub detected_format: String,
    #[serde(default)]
    pub backup_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub owner_user_id: Option<String>,
    pub owner_mismatch: bool,
    pub total_entries: usize,
    pub total_plain_bytes: u64,
    pub total_messages: usize,
    pub new_messages: usize,
    pub duplicate_messages: usize,
    pub total_media: usize,
    pub new_media: usize,
    pub duplicate_media: usize,
    #[serde(default)]
    pub missing_media_in_backup: Vec<MissingMediaItem>,
    pub integrity_verified: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RestoreResult {
    #[serde(default)]
    pub backup_id: Option<String>,
    pub mode: String,
    pub restored_messages: usize,
    pub skipped_duplicate_messages: usize,
    pub skipped_unauthorized_messages: usize,
    pub restored_media: usize,
    pub skipped_duplicate_media: usize,
    pub restored_settings: bool,
    pub restored_avatar: bool,
    pub integrity_verified: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BackupJobEntry {
    pub id: String,
    pub job_kind: String,
    pub format: String,
    pub status: String,
    pub total_bytes: i64,
    pub processed_bytes: i64,
    pub total_files: i32,
    pub processed_files: i32,
    pub missing_files: i32,
    #[serde(default)]
    pub checksum_sha256: Option<String>,
    #[serde(default)]
    pub honest_notice: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct JobsResponse {
    #[serde(default)]
    pub jobs: Vec<BackupJobEntry>,
}

pub async fn get_reauth_methods() -> Result<ReauthMethods, ApiError> {
    req_json("GET", "/api/backup/reauth/methods", None).await
}

pub async fn set_export_password(
    current_password: Option<&str>,
    totp_or_backup: Option<&str>,
    new_password: &str,
) -> Result<(), ApiError> {
    let _: serde_json::Value = req_json(
        "PUT",
        "/api/backup/reauth/password",
        Some(json!({
            "current_password": current_password.filter(|s| !s.is_empty()),
            "totp_or_backup": totp_or_backup.filter(|s| !s.is_empty()),
            "new_password": new_password,
        })),
    )
    .await?;
    Ok(())
}

pub async fn register_device_passkey(label: &str) -> Result<String, ApiError> {
    let created = platform::register_device_passkey_local().await?;
    let resp: serde_json::Value = req_json(
        "POST",
        "/api/backup/reauth/passkeys/register",
        Some(json!({
            "credential_id": created.credential_id,
            "public_key_spki_b64": created.public_key_spki_b64,
            "label": label,
        })),
    )
    .await?;
    Ok(resp
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

pub async fn verify_reauth(
    scope: &str,
    method: &str,
    secret: &str,
) -> Result<ReauthResponse, ApiError> {
    if method == "passkey" {
        let ch: serde_json::Value = req_json(
            "POST",
            "/api/backup/reauth/challenge",
            Some(json!({ "purpose": "reauth" })),
        )
        .await?;
        let challenge_id = ch
            .get("challenge_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let challenge_b64 = ch
            .get("challenge_b64")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sig = platform::sign_passkey_challenge_local(&challenge_b64).await?;
        return req_json(
            "POST",
            "/api/backup/reauth",
            Some(json!({
                "scope": scope,
                "method": "passkey",
                "secret": "",
                "challenge_id": challenge_id,
                "credential_id": sig.credential_id,
                "signature_b64": sig.signature_b64,
            })),
        )
        .await;
    }
    req_json(
        "POST",
        "/api/backup/reauth",
        Some(json!({
            "scope": scope,
            "method": method,
            "secret": secret,
        })),
    )
    .await
}

pub async fn advance_unencrypted_zip_step(
    export_token: &str,
    step: &str,
) -> Result<UnencryptedZipStepResponse, ApiError> {
    req_json(
        "POST",
        "/api/backup/unencrypted-zip/unlock-step",
        Some(json!({
            "export_token": export_token,
            "step": step,
        })),
    )
    .await
}

pub async fn revoke_unencrypted_zip(export_token: &str) -> Result<(), ApiError> {
    let _: serde_json::Value = req_json(
        "POST",
        "/api/backup/unencrypted-zip/revoke",
        Some(json!({ "export_token": export_token })),
    )
    .await?;
    Ok(())
}

pub async fn get_export_preview() -> Result<ExportPreview, ApiError> {
    req_json("GET", "/api/backup/preview", None).await
}

pub async fn export_kdata_download(
    export_token: &str,
    encryption_password: &str,
    scope: &ExportScopeOptions,
) -> Result<platform::ExportDownloadResult, ApiError> {
    let body = json!({
        "export_token": export_token,
        "encryption_password": encryption_password,
        "include_account": scope.include_account,
        "include_messages": scope.include_messages,
        "include_media": scope.include_media,
        "include_settings": scope.include_settings,
        "conversation_id": scope.conversation_id,
    });
    platform::stream_export_download(
        "/api/backup/export/kdata",
        &body.to_string(),
        "KIYO_Backup.kdata",
    )
    .await
}

pub async fn export_zip_encrypted_download(
    export_token: &str,
    encryption_password: &str,
    scope: &ExportScopeOptions,
) -> Result<platform::ExportDownloadResult, ApiError> {
    let body = json!({
        "export_token": export_token,
        "encryption_password": encryption_password,
        "include_account": scope.include_account,
        "include_messages": scope.include_messages,
        "include_media": scope.include_media,
        "include_settings": scope.include_settings,
        "conversation_id": scope.conversation_id,
    });
    platform::stream_export_download(
        "/api/backup/export/zip-encrypted",
        &body.to_string(),
        "KIYO_Backup_AES256.zip",
    )
    .await
}

pub async fn export_zip_unencrypted_download(
    export_token: &str,
    scope: &ExportScopeOptions,
) -> Result<platform::ExportDownloadResult, ApiError> {
    let body = json!({
        "export_token": export_token,
        "encryption_password": null,
        "include_account": scope.include_account,
        "include_messages": scope.include_messages,
        "include_media": scope.include_media,
        "include_settings": scope.include_settings,
        "conversation_id": scope.conversation_id,
    });
    platform::stream_export_download(
        "/api/backup/export/zip-unencrypted",
        &body.to_string(),
        "KIYO_Backup_UNENCRYPTED.zip",
    )
    .await
}

pub async fn inspect_backup_input(
    input_id: &str,
    password: Option<&str>,
) -> Result<InspectArchiveResponse, ApiError> {
    let res =
        platform::upload_backup_from_input(input_id, "/api/backup/inspect", password).await?;
    serde_json::from_str(&res.body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: res.status,
    })
}

pub async fn restore_backup_input(
    input_id: &str,
    password: Option<&str>,
    mode: &str,
    conversation_id: Option<&str>,
) -> Result<RestoreResult, ApiError> {
    let mut url = format!("/api/backup/restore?mode={}", urlencoding::encode(mode));
    if let Some(cid) = conversation_id
        && !cid.trim().is_empty()
    {
        url.push_str(&format!("&conversation_id={}", urlencoding::encode(cid.trim())));
    }
    let res = platform::upload_backup_from_input(input_id, &url, password).await?;
    serde_json::from_str(&res.body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: res.status,
    })
}

pub async fn list_backup_jobs() -> Result<Vec<BackupJobEntry>, ApiError> {
    let r: JobsResponse = req_json("GET", "/api/backup/jobs", None).await?;
    Ok(r.jobs)
}

pub async fn control_backup_job(job_id: &str, action: &str) -> Result<(), ApiError> {
    let _: serde_json::Value = req_json(
        "POST",
        &format!("/api/backup/jobs/{job_id}/control"),
        Some(json!({ "action": action })),
    )
    .await?;
    Ok(())
}

// =========================================================================
// Cloud Storage Providers & Incremental Backup
// =========================================================================

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub provider_id: String,
    pub display_name: String,
    pub auth_kind: String,
    pub oauth_configured: bool,
    pub supports_resumable_upload: bool,
    pub supports_resumable_download: bool,
    pub supports_incremental_chunks: bool,
    pub supports_quota: bool,
    pub supports_folder_isolation: bool,
    pub requires_https: bool,
    pub max_file_bytes: u64,
    #[serde(default)]
    pub setup_requirements: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ProvidersListResp {
    #[serde(default)]
    pub providers: Vec<ProviderCapability>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudConnection {
    pub id: String,
    pub provider: String,
    pub account_label: String,
    #[serde(default)]
    pub remote_account: Option<String>,
    pub status: String,
    #[serde(default)]
    pub quota_total_bytes: Option<i64>,
    #[serde(default)]
    pub quota_used_bytes: Option<i64>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_backup_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ConnectionsListResp {
    #[serde(default)]
    pub connections: Vec<CloudConnection>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudSnapshotEntry {
    pub id: String,
    pub snapshot_seq: i64,
    #[serde(default)]
    pub parent_snapshot_id: Option<String>,
    pub remote_manifest_key: String,
    pub manifest_sha256: String,
    pub total_plain_bytes: i64,
    pub uploaded_delta_bytes: i64,
    pub file_count: i32,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub verified_at: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SnapshotsListResp {
    #[serde(default)]
    pub snapshots: Vec<CloudSnapshotEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudBackupRunResult {
    pub snapshot_id: String,
    pub snapshot_seq: i64,
    pub total_plain_bytes: u64,
    pub uploaded_delta_bytes: u64,
    pub deduplicated_bytes: u64,
    pub new_chunks: usize,
    pub reused_chunks: usize,
    pub file_count: usize,
    pub missing_files: usize,
    pub verified: bool,
    pub remote_manifest_key: String,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudBackupPolicy {
    pub enabled: bool,
    #[serde(default)]
    pub connection_id: Option<String>,
    pub schedule_mode: String,
    pub wifi_only: bool,
    pub charging_only: bool,
    pub bandwidth_limit_kbps: i32,
    pub storage_limit_mb: i32,
    pub retain_snapshots: i32,
    pub paused: bool,
    #[serde(default)]
    pub has_auto_key: bool,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,
}

pub async fn list_cloud_providers() -> Result<Vec<ProviderCapability>, ApiError> {
    let r: ProvidersListResp = req_json("GET", "/api/cloud/providers", None).await?;
    Ok(r.providers)
}

pub async fn list_cloud_connections() -> Result<Vec<CloudConnection>, ApiError> {
    let r: ConnectionsListResp = req_json("GET", "/api/cloud/connections", None).await?;
    Ok(r.connections)
}

pub async fn connect_cloud_credentials(
    provider: &str,
    account_label: &str,
    config: serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/cloud/connections",
        Some(json!({
            "provider": provider,
            "account_label": account_label,
            "config": config,
        })),
    )
    .await
}

pub async fn start_cloud_oauth(provider: &str) -> Result<String, ApiError> {
    let r: serde_json::Value =
        req_json("POST", &format!("/api/cloud/oauth/{provider}/start"), None).await?;
    Ok(r.get("authorize_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

pub async fn disconnect_cloud(conn_id: &str) -> Result<(), ApiError> {
    let _: serde_json::Value =
        req_json("DELETE", &format!("/api/cloud/connections/{conn_id}"), None).await?;
    Ok(())
}

pub async fn verify_cloud_connection(conn_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("POST", &format!("/api/cloud/connections/{conn_id}/verify"), None).await
}

pub async fn trigger_cloud_backup(
    conn_id: &str,
    encryption_password: &str,
) -> Result<CloudBackupRunResult, ApiError> {
    req_json(
        "POST",
        &format!("/api/cloud/connections/{conn_id}/backup"),
        Some(json!({ "encryption_password": encryption_password })),
    )
    .await
}

pub async fn list_cloud_snapshots(conn_id: &str) -> Result<Vec<CloudSnapshotEntry>, ApiError> {
    let r: SnapshotsListResp =
        req_json("GET", &format!("/api/cloud/connections/{conn_id}/backups"), None).await?;
    Ok(r.snapshots)
}

pub async fn restore_cloud_snapshot(
    conn_id: &str,
    snapshot_id: &str,
    encryption_password: &str,
    mode: &str,
) -> Result<RestoreResult, ApiError> {
    req_json(
        "POST",
        &format!("/api/cloud/connections/{conn_id}/restore"),
        Some(json!({
            "snapshot_id": snapshot_id,
            "encryption_password": encryption_password,
            "mode": mode,
        })),
    )
    .await
}

pub async fn delete_cloud_snapshot(conn_id: &str, snapshot_id: &str) -> Result<(), ApiError> {
    let _: serde_json::Value = req_json(
        "DELETE",
        &format!("/api/cloud/connections/{conn_id}/backups/{snapshot_id}"),
        None,
    )
    .await?;
    Ok(())
}

pub async fn get_cloud_policy() -> Result<CloudBackupPolicy, ApiError> {
    req_json("GET", "/api/cloud/policy", None).await
}

pub async fn update_cloud_policy(
    policy: &CloudBackupPolicy,
    auto_encryption_password: Option<&str>,
    clear_auto_key: bool,
) -> Result<CloudBackupPolicy, ApiError> {
    let mut payload = serde_json::to_value(policy).unwrap_or_else(|_| json!({}));
    if let Some(obj) = payload.as_object_mut() {
        if let Some(pw) = auto_encryption_password.filter(|s| !s.trim().is_empty()) {
            obj.insert("auto_encryption_password".into(), json!(pw.trim()));
        }
        if clear_auto_key {
            obj.insert("clear_auto_key".into(), json!(true));
        }
    }
    req_json("PUT", "/api/cloud/policy", Some(payload)).await
}
