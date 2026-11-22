use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    security::log_event,
};

use super::{
    incremental::{
        delete_snapshot_safely, enforce_snapshot_retention, next_run_interval_secs,
        restore_from_cloud_snapshot, run_incremental_cloud_backup,
    },
    open_cloud_config,
    providers::{all_provider_catalog, instantiate_provider},
    seal_cloud_config,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/providers", get(handle_list_providers))
        .route("/connections", get(handle_list_connections).post(handle_connect_credentials))
        .route("/connections/{id}", delete(handle_disconnect))
        .route("/connections/{id}/backups", get(handle_list_backups))
        .route("/connections/{id}/backup", post(handle_trigger_backup))
        .route("/connections/{id}/verify", post(handle_verify_connection))
        .route("/connections/{id}/restore", post(handle_restore_snapshot))
        .route("/connections/{id}/backups/{snapshot_id}", delete(handle_delete_snapshot))
        .route("/oauth/{provider}/start", post(handle_oauth_start))
        .route("/oauth/{provider}/callback", get(handle_oauth_callback))
        .route("/policy", get(handle_get_policy).put(handle_update_policy))
}

async fn require_user(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let raw = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    Ok(authenticate_session(&state.db, &state.config, raw).await?.id)
}

async fn handle_list_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let _ = require_user(&state, &headers).await?;
    Ok(Json(json!({ "providers": all_provider_catalog() })))
}

async fn handle_list_connections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, provider, account_label, remote_account, status, quota_total_bytes, quota_used_bytes, last_error, last_backup_at, created_at
         FROM cloud_connections WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let mut list = Vec::with_capacity(rows.len());
    for r in rows {
        let last_backup: Option<chrono::DateTime<Utc>> = r.try_get("last_backup_at")?;
        let created: chrono::DateTime<Utc> = r.try_get("created_at")?;
        list.push(json!({
            "id": r.try_get::<Uuid, _>("id")?,
            "provider": r.try_get::<String, _>("provider")?,
            "account_label": r.try_get::<String, _>("account_label")?,
            "remote_account": r.try_get::<Option<String>, _>("remote_account")?,
            "status": r.try_get::<String, _>("status")?,
            "quota_total_bytes": r.try_get::<Option<i64>, _>("quota_total_bytes")?,
            "quota_used_bytes": r.try_get::<Option<i64>, _>("quota_used_bytes")?,
            "last_error": r.try_get::<Option<String>, _>("last_error")?,
            "last_backup_at": last_backup.map(|t| t.to_rfc3339()),
            "created_at": created.to_rfc3339(),
        }));
    }
    Ok(Json(json!({ "connections": list })))
}

#[derive(Deserialize)]
struct ConnectCredsBody {
    provider: String, // nextcloud | webdav | nas | s3 | r2 | minio
    #[serde(default)]
    account_label: String,
    config: Value,
}

async fn handle_connect_credentials(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<ConnectCredsBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    if !matches!(b.provider.as_str(), "nextcloud" | "webdav" | "nas" | "s3" | "r2" | "minio") {
        return Err(AppError::BadRequest("provider_requires_oauth_or_native"));
    }

    let provider = instantiate_provider(&b.provider, &b.config).map_err(AppError::from)?;
    let (remote_acc, quota) = provider.verify_connection(&state.http_client).await.map_err(AppError::from)?;

    let sealed = seal_cloud_config(&state.config.session_secret, &b.config)?;
    let id = Uuid::new_v4();
    let label = if b.account_label.trim().is_empty() {
        remote_acc.clone().unwrap_or_else(|| b.provider.clone())
    } else {
        b.account_label.trim().chars().take(64).collect()
    };

    sqlx::query(
        "INSERT INTO cloud_connections (id, user_id, provider, account_label, remote_account, config_enc, status, quota_total_bytes, quota_used_bytes)
         VALUES ($1, $2, $3, $4, $5, $6, 'connected', $7, $8)",
    )
    .bind(id)
    .bind(uid)
    .bind(&b.provider)
    .bind(&label)
    .bind(&remote_acc)
    .bind(sealed)
    .bind(quota.total_bytes.map(|v| v as i64))
    .bind(quota.used_bytes.map(|v| v as i64))
    .execute(&state.db)
    .await?;

    log_event(&state, uid, "cloud_connect", &headers).await;
    Ok(Json(json!({
        "id": id,
        "provider": b.provider,
        "account_label": label,
        "remote_account": remote_acc,
        "status": "connected",
        "quota": quota,
    })))
}

async fn handle_disconnect(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let row = sqlx::query("SELECT provider, config_enc FROM cloud_connections WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("cloud_connection_not_found"))?;

    let prov_id: String = row.try_get("provider")?;
    let enc: Vec<u8> = row.try_get("config_enc")?;
    if let Ok(cfg) = open_cloud_config(&state.config.session_secret, &enc) {
        if let Ok(prov) = instantiate_provider(&prov_id, &cfg) {
            let _ = prov.revoke_authorization(&state.http_client).await;
        }
    }

    sqlx::query("DELETE FROM cloud_connections WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(uid)
        .execute(&state.db)
        .await?;
    log_event(&state, uid, "cloud_disconnect", &headers).await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct OAuthStartBody {
    #[serde(default)]
    use_app_data: bool,
}

async fn handle_oauth_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    Json(b): Json<OAuthStartBody>,
) -> AppResult<Response> {
    let uid = require_user(&state, &headers).await?;
    let mut verifier_raw = [0u8; 32];
    getrandom::fill(&mut verifier_raw).map_err(|_| AppError::Internal)?;
    let verifier = URL_SAFE_NO_PAD.encode(verifier_raw);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let mut state_raw = [0u8; 24];
    getrandom::fill(&mut state_raw).map_err(|_| AppError::Internal)?;
    let csrf_state = URL_SAFE_NO_PAD.encode(state_raw);
    let redirect_uri = format!("{}/api/cloud/oauth/{}/callback", state.config.public_origin, provider);

    let (auth_url, scope) = match provider.as_str() {
        "google_drive" => {
            let Some(cid) = std::env::var("GOOGLE_DRIVE_CLIENT_ID").ok().filter(|v| !v.is_empty() && !v.starts_with("dev-")) else {
                return Ok((
                    StatusCode::PRECONDITION_REQUIRED,
                    Json(json!({
                        "error": "provider_not_configured_by_admin",
                        "provider": "google_drive",
                        "message": "The server does not have GOOGLE_DRIVE_CLIENT_ID configured for the Google Drive API.",
                        "requirements": all_provider_catalog().into_iter().find(|p| p.provider_id == "google_drive")
                    })),
                )
                    .into_response());
            };
            let sc = if b.use_app_data {
                "https://www.googleapis.com/auth/drive.appdata https://www.googleapis.com/auth/userinfo.email"
            } else {
                "https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/userinfo.email"
            };
            let u = format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&code_challenge={}&code_challenge_method=S256&state={}",
                urlencoding::encode(&cid),
                urlencoding::encode(&redirect_uri),
                urlencoding::encode(sc),
                urlencoding::encode(&challenge),
                urlencoding::encode(&csrf_state),
            );
            (u, sc.to_string())
        }
        "onedrive" => {
            let Some(cid) = std::env::var("ONEDRIVE_CLIENT_ID").ok().filter(|v| !v.is_empty()) else {
                return Ok((
                    StatusCode::PRECONDITION_REQUIRED,
                    Json(json!({
                        "error": "provider_not_configured_by_admin",
                        "provider": "onedrive",
                        "message": "The server has not configured ONEDRIVE_CLIENT_ID on Microsoft Entra.",
                        "requirements": all_provider_catalog().into_iter().find(|p| p.provider_id == "onedrive")
                    })),
                )
                    .into_response());
            };
            let sc = "Files.ReadWrite.AppFolder offline_access User.Read";
            let u = format!(
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
                urlencoding::encode(&cid),
                urlencoding::encode(&redirect_uri),
                urlencoding::encode(sc),
                urlencoding::encode(&challenge),
                urlencoding::encode(&csrf_state),
            );
            (u, sc.to_string())
        }
        "dropbox" => {
            let Some(cid) = std::env::var("DROPBOX_APP_KEY").ok().filter(|v| !v.is_empty()) else {
                return Ok((
                    StatusCode::PRECONDITION_REQUIRED,
                    Json(json!({
                        "error": "provider_not_configured_by_admin",
                        "provider": "dropbox",
                        "message": "The server has not configured DROPBOX_APP_KEY (Dropbox App Folder).",
                        "requirements": all_provider_catalog().into_iter().find(|p| p.provider_id == "dropbox")
                    })),
                )
                    .into_response());
            };
            let sc = "files.metadata.read files.content.write files.content.read account_info.read";
            let u = format!(
                "https://www.dropbox.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&token_access_type=offline&code_challenge={}&code_challenge_method=S256&state={}",
                urlencoding::encode(&cid),
                urlencoding::encode(&redirect_uri),
                urlencoding::encode(&challenge),
                urlencoding::encode(&csrf_state),
            );
            (u, sc.to_string())
        }
        _ => return Err(AppError::BadRequest("unsupported_oauth_provider")),
    };

    let enc_verifier = seal_cloud_config(
        &state.config.session_secret,
        &json!({ "verifier": verifier, "use_app_data": b.use_app_data }),
    )?;
    sqlx::query(
        "INSERT INTO cloud_oauth_states (state, user_id, provider, code_verifier_enc, redirect_uri, scope_used, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, now() + interval '10 minutes')",
    )
    .bind(&csrf_state)
    .bind(uid)
    .bind(&provider)
    .bind(enc_verifier)
    .bind(&redirect_uri)
    .bind(&scope)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "auth_url": auth_url, "state": csrf_state })).into_response())
}

#[derive(Deserialize)]
struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn handle_oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    Query(q): Query<OAuthCallbackQuery>,
) -> AppResult<Response> {
    if q.error.is_some() {
        return Ok(Redirect::to("/?cloud_oauth=cancelled").into_response());
    }
    let code = q.code.ok_or(AppError::BadRequest("missing_code"))?;
    let csrf_state = q.state.ok_or(AppError::BadRequest("missing_state"))?;
    let uid = require_user(&state, &headers).await?;

    let row = sqlx::query(
        "DELETE FROM cloud_oauth_states WHERE state = $1 AND user_id = $2 AND provider = $3 AND expires_at > now()
         RETURNING code_verifier_enc, redirect_uri",
    )
    .bind(&csrf_state)
    .bind(uid)
    .bind(&provider)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let enc_v: Vec<u8> = row.try_get("code_verifier_enc")?;
    let redirect_uri: String = row.try_get("redirect_uri")?;
    let st_json = open_cloud_config(&state.config.session_secret, &enc_v)?;
    let verifier = st_json["verifier"].as_str().unwrap_or_default();
    let use_app_data = st_json["use_app_data"].as_bool().unwrap_or(false);

    let tok_json: Value = match provider.as_str() {
        "google_drive" => {
            let cid = std::env::var("GOOGLE_DRIVE_CLIENT_ID").map_err(|_| AppError::Internal)?;
            let csec = std::env::var("GOOGLE_DRIVE_CLIENT_SECRET").unwrap_or_default();
            let mut form = vec![
                ("code", code.as_str()),
                ("client_id", cid.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("code_verifier", verifier),
                ("grant_type", "authorization_code"),
            ];
            if !csec.is_empty() {
                form.push(("client_secret", csec.as_str()));
            }
            state
                .http_client
                .post("https://oauth2.googleapis.com/token")
                .form(&form)
                .send()
                .await
                .map_err(|_| AppError::Internal)?
                .json()
                .await
                .map_err(|_| AppError::Internal)?
        }
        "onedrive" => {
            let cid = std::env::var("ONEDRIVE_CLIENT_ID").map_err(|_| AppError::Internal)?;
            state
                .http_client
                .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
                .form(&[
                    ("code", code.as_str()),
                    ("client_id", cid.as_str()),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("code_verifier", verifier),
                    ("grant_type", "authorization_code"),
                ])
                .send()
                .await
                .map_err(|_| AppError::Internal)?
                .json()
                .await
                .map_err(|_| AppError::Internal)?
        }
        "dropbox" => {
            let cid = std::env::var("DROPBOX_APP_KEY").map_err(|_| AppError::Internal)?;
            state
                .http_client
                .post("https://api.dropboxapi.com/oauth2/token")
                .form(&[
                    ("code", code.as_str()),
                    ("client_id", cid.as_str()),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("code_verifier", verifier),
                    ("grant_type", "authorization_code"),
                ])
                .send()
                .await
                .map_err(|_| AppError::Internal)?
                .json()
                .await
                .map_err(|_| AppError::Internal)?
        }
        _ => return Err(AppError::BadRequest("unsupported_oauth_provider")),
    };

    let cfg = json!({
        "access_token": tok_json["access_token"],
        "refresh_token": tok_json["refresh_token"],
        "use_app_data": use_app_data,
    });
    let prov = instantiate_provider(&provider, &cfg).map_err(AppError::from)?;
    let (remote_acc, quota) = prov.verify_connection(&state.http_client).await.map_err(AppError::from)?;
    let sealed = seal_cloud_config(&state.config.session_secret, &cfg)?;
    let id = Uuid::new_v4();
    let label = remote_acc.clone().unwrap_or_else(|| provider.clone());

    sqlx::query(
        "INSERT INTO cloud_connections (id, user_id, provider, account_label, remote_account, config_enc, status, quota_total_bytes, quota_used_bytes)
         VALUES ($1, $2, $3, $4, $5, $6, 'connected', $7, $8)",
    )
    .bind(id)
    .bind(uid)
    .bind(&provider)
    .bind(&label)
    .bind(&remote_acc)
    .bind(sealed)
    .bind(quota.total_bytes.map(|v| v as i64))
    .bind(quota.used_bytes.map(|v| v as i64))
    .execute(&state.db)
    .await?;

    log_event(&state, uid, "cloud_connect", &headers).await;
    Ok(Redirect::to("/?cloud_oauth=connected").into_response())
}

async fn handle_list_backups(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conn_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, snapshot_seq, parent_snapshot_id, remote_manifest_key, manifest_sha256, total_plain_bytes, uploaded_delta_bytes, file_count, status, created_at, verified_at
         FROM cloud_snapshots WHERE user_id = $1 AND connection_id = $2 AND status <> 'deleted' ORDER BY snapshot_seq DESC",
    )
    .bind(uid)
    .bind(conn_id)
    .fetch_all(&state.db)
    .await?;

    let mut snaps = Vec::with_capacity(rows.len());
    for r in rows {
        let created: chrono::DateTime<Utc> = r.try_get("created_at")?;
        let verified: Option<chrono::DateTime<Utc>> = r.try_get("verified_at")?;
        snaps.push(json!({
            "id": r.try_get::<Uuid, _>("id")?,
            "snapshot_seq": r.try_get::<i64, _>("snapshot_seq")?,
            "parent_snapshot_id": r.try_get::<Option<Uuid>, _>("parent_snapshot_id")?,
            "remote_manifest_key": r.try_get::<String, _>("remote_manifest_key")?,
            "manifest_sha256": r.try_get::<String, _>("manifest_sha256")?,
            "total_plain_bytes": r.try_get::<i64, _>("total_plain_bytes")?,
            "uploaded_delta_bytes": r.try_get::<i64, _>("uploaded_delta_bytes")?,
            "file_count": r.try_get::<i32, _>("file_count")?,
            "status": r.try_get::<String, _>("status")?,
            "created_at": created.to_rfc3339(),
            "verified_at": verified.map(|t| t.to_rfc3339()),
        }));
    }
    Ok(Json(json!({ "snapshots": snaps })))
}

#[derive(Deserialize)]
struct TriggerBackupBody {
    encryption_password: String,
}

async fn handle_trigger_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conn_id): Path<Uuid>,
    Json(b): Json<TriggerBackupBody>,
) -> AppResult<Json<super::incremental::CloudBackupRunResult>> {
    let uid = require_user(&state, &headers).await?;
    let res = run_incremental_cloud_backup(
        &state.db,
        &state.storage,
        &state.http_client,
        &state.config.session_secret,
        uid,
        conn_id,
        &b.encryption_password,
    )
    .await?;
    if let Ok(Some(retain)) = sqlx::query_scalar::<_, i32>(
        "SELECT retain_snapshots FROM cloud_backup_policies WHERE user_id = $1 AND connection_id = $2",
    )
    .bind(uid)
    .bind(conn_id)
    .fetch_optional(&state.db)
    .await
    {
        let _ = enforce_snapshot_retention(
            &state.db,
            &state.http_client,
            &state.config.session_secret,
            uid,
            conn_id,
            retain,
        )
        .await;
    }
    log_event(&state, uid, "cloud_backup_verified", &headers).await;
    Ok(Json(res))
}

async fn handle_verify_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conn_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let row = sqlx::query("SELECT provider, config_enc FROM cloud_connections WHERE id = $1 AND user_id = $2")
        .bind(conn_id)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("cloud_connection_not_found"))?;

    let prov_id: String = row.try_get("provider")?;
    let enc: Vec<u8> = row.try_get("config_enc")?;
    let cfg = open_cloud_config(&state.config.session_secret, &enc)?;
    let prov = instantiate_provider(&prov_id, &cfg).map_err(AppError::from)?;
    let (acc, quota) = match prov.verify_connection(&state.http_client).await {
        Ok(ok) => ok,
        Err(e) => {
            sqlx::query("UPDATE cloud_connections SET status = 'auth_error', last_error = $1, updated_at = now() WHERE id = $2")
                .bind(e.to_string())
                .bind(conn_id)
                .execute(&state.db)
                .await?;
            return Err(AppError::from(e));
        }
    };

    // Also verify latest snapshot manifest SHA-256 on remote
    let latest_snap = sqlx::query(
        "SELECT id, remote_manifest_key, manifest_sha256 FROM cloud_snapshots WHERE user_id = $1 AND connection_id = $2 AND status = 'verified' ORDER BY snapshot_seq DESC LIMIT 1",
    )
    .bind(uid)
    .bind(conn_id)
    .fetch_optional(&state.db)
    .await?;

    let mut latest_verified: Option<bool> = None;
    if let Some(sr) = latest_snap {
        let key: String = sr.try_get("remote_manifest_key")?;
        let sha: String = sr.try_get("manifest_sha256")?;
        latest_verified = Some(prov.verify_backup(&state.http_client, &key, &sha).await.unwrap_or(false));
    }

    sqlx::query(
        "UPDATE cloud_connections SET status = 'connected', remote_account = COALESCE($1, remote_account),
         quota_total_bytes = $2, quota_used_bytes = $3, last_error = NULL, updated_at = now() WHERE id = $4",
    )
    .bind(&acc)
    .bind(quota.total_bytes.map(|v| v as i64))
    .bind(quota.used_bytes.map(|v| v as i64))
    .bind(conn_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "status": "connected",
        "remote_account": acc,
        "quota": quota,
        "latest_snapshot_verified": latest_verified,
    })))
}

#[derive(Deserialize)]
struct CloudRestoreBody {
    snapshot_id: Uuid,
    encryption_password: String,
    #[serde(default = "default_full")]
    mode: String,
}
fn default_full() -> String {
    "full".into()
}

async fn handle_restore_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conn_id): Path<Uuid>,
    Json(b): Json<CloudRestoreBody>,
) -> AppResult<Json<crate::backup::engine::RestoreResultResponse>> {
    let uid = require_user(&state, &headers).await?;
    let res = restore_from_cloud_snapshot(
        &state.db,
        &state.storage,
        &state.http_client,
        &state.config.session_secret,
        uid,
        conn_id,
        b.snapshot_id,
        &b.encryption_password,
        &b.mode,
    )
    .await?;
    log_event(&state, uid, "backup_restore", &headers).await;
    Ok(Json(res))
}

async fn handle_delete_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conn_id, snapshot_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let conn = sqlx::query("SELECT provider, config_enc FROM cloud_connections WHERE id = $1 AND user_id = $2")
        .bind(conn_id)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("cloud_connection_not_found"))?;
    let prov_id: String = conn.try_get("provider")?;
    let enc: Vec<u8> = conn.try_get("config_enc")?;
    let cfg = open_cloud_config(&state.config.session_secret, &enc)?;
    let prov = instantiate_provider(&prov_id, &cfg).map_err(AppError::from)?;

    let mf_key: String = sqlx::query_scalar(
        "SELECT remote_manifest_key FROM cloud_snapshots WHERE id = $1 AND user_id = $2 AND connection_id = $3",
    )
    .bind(snapshot_id)
    .bind(uid)
    .bind(conn_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("cloud_snapshot_not_found"))?;

    delete_snapshot_safely(&state.db, prov.as_ref(), &state.http_client, uid, conn_id, snapshot_id, &mf_key).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn handle_get_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT enabled, connection_id, schedule_mode, wifi_only, charging_only, bandwidth_limit_kbps, storage_limit_mb, retain_snapshots, paused, last_run_at, next_run_at, (auto_key_enc IS NOT NULL) AS has_auto_key
         FROM cloud_backup_policies WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = row {
        let last_run: Option<chrono::DateTime<Utc>> = r.try_get("last_run_at")?;
        let next_run: Option<chrono::DateTime<Utc>> = r.try_get("next_run_at")?;
        Ok(Json(json!({
            "enabled": r.try_get::<bool, _>("enabled")?,
            "connection_id": r.try_get::<Option<Uuid>, _>("connection_id")?,
            "schedule_mode": r.try_get::<String, _>("schedule_mode")?,
            "wifi_only": r.try_get::<bool, _>("wifi_only")?,
            "charging_only": r.try_get::<bool, _>("charging_only")?,
            "bandwidth_limit_kbps": r.try_get::<i32, _>("bandwidth_limit_kbps")?,
            "storage_limit_mb": r.try_get::<i32, _>("storage_limit_mb")?,
            "retain_snapshots": r.try_get::<i32, _>("retain_snapshots")?,
            "paused": r.try_get::<bool, _>("paused")?,
            "has_auto_key": r.try_get::<bool, _>("has_auto_key")?,
            "last_run_at": last_run.map(|t| t.to_rfc3339()),
            "next_run_at": next_run.map(|t| t.to_rfc3339()),
        })))
    } else {
        Ok(Json(json!({
            "enabled": false,
            "connection_id": null,
            "schedule_mode": "manual",
            "wifi_only": true,
            "charging_only": false,
            "bandwidth_limit_kbps": 0,
            "storage_limit_mb": 0,
            "retain_snapshots": 5,
            "paused": false,
            "has_auto_key": false,
            "last_run_at": null,
            "next_run_at": null,
        })))
    }
}

#[derive(Deserialize)]
struct UpdatePolicyBody {
    enabled: bool,
    connection_id: Option<Uuid>,
    schedule_mode: String,
    wifi_only: bool,
    charging_only: bool,
    bandwidth_limit_kbps: i32,
    storage_limit_mb: i32,
    retain_snapshots: i32,
    paused: bool,
    #[serde(default)]
    auto_encryption_password: Option<String>,
    #[serde(default)]
    clear_auto_key: bool,
}

async fn handle_update_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<UpdatePolicyBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    if !matches!(b.schedule_mode.as_str(), "manual" | "daily" | "weekly" | "on_change") {
        return Err(AppError::BadRequest("invalid_schedule_mode"));
    }

    let sealed_auto_key: Option<Vec<u8>> = if let Some(ref pw) = b.auto_encryption_password {
        let trimmed = pw.trim();
        if !trimmed.is_empty() {
            if trimmed.chars().count() < 6 {
                return Err(AppError::BadRequest("encryption_password_too_short"));
            }
            Some(seal_cloud_config(&state.config.session_secret, &json!({ "passphrase": trimmed }))?)
        } else {
            None
        }
    } else {
        None
    };

    let next_secs = if b.enabled && b.schedule_mode != "manual" && !b.paused {
        Some(next_run_interval_secs(&b.schedule_mode))
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO cloud_backup_policies (user_id, enabled, connection_id, schedule_mode, wifi_only, charging_only, bandwidth_limit_kbps, storage_limit_mb, retain_snapshots, paused, auto_key_enc, next_run_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CASE WHEN $12::bigint IS NULL THEN NULL ELSE now() + ($12::text || ' seconds')::interval END, now())
         ON CONFLICT (user_id) DO UPDATE SET
           enabled = EXCLUDED.enabled,
           connection_id = EXCLUDED.connection_id,
           schedule_mode = EXCLUDED.schedule_mode,
           wifi_only = EXCLUDED.wifi_only,
           charging_only = EXCLUDED.charging_only,
           bandwidth_limit_kbps = EXCLUDED.bandwidth_limit_kbps,
           storage_limit_mb = EXCLUDED.storage_limit_mb,
           retain_snapshots = EXCLUDED.retain_snapshots,
           paused = EXCLUDED.paused,
           auto_key_enc = CASE
             WHEN $13 THEN NULL
             WHEN EXCLUDED.auto_key_enc IS NOT NULL THEN EXCLUDED.auto_key_enc
             ELSE cloud_backup_policies.auto_key_enc
           END,
           next_run_at = CASE
             WHEN $12::bigint IS NULL THEN NULL
             ELSE COALESCE(cloud_backup_policies.next_run_at, now() + ($12::text || ' seconds')::interval)
           END,
           updated_at = now()",
    )
    .bind(uid)
    .bind(b.enabled)
    .bind(b.connection_id)
    .bind(&b.schedule_mode)
    .bind(b.wifi_only)
    .bind(b.charging_only)
    .bind(b.bandwidth_limit_kbps.max(0))
    .bind(b.storage_limit_mb.max(0))
    .bind(b.retain_snapshots.clamp(1, 50))
    .bind(b.paused)
    .bind(sealed_auto_key)
    .bind(next_secs)
    .bind(b.clear_auto_key)
    .execute(&state.db)
    .await?;

    handle_get_policy(State(state), headers).await
}
