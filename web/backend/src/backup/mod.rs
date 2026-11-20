pub mod engine;
pub mod reauth;

use std::io::Write;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reauth/methods", get(handle_get_reauth_methods))
        .route("/reauth/password", put(handle_set_export_password))
        .route("/reauth/passkeys/register", post(handle_register_passkey))
        .route("/reauth/challenge", post(handle_reauth_challenge))
        .route("/reauth", post(handle_reauth))
        .route("/unencrypted-zip/unlock-step", post(handle_unencrypted_zip_step))
        .route("/unencrypted-zip/revoke", post(handle_unencrypted_zip_revoke))
        .route("/preview", get(handle_export_preview))
        .route("/export/kdata", post(handle_export_kdata))
        .route("/export/zip-encrypted", post(handle_export_zip_encrypted))
        .route("/export/zip-unencrypted", post(handle_export_zip_unencrypted))
        .route(
            "/inspect",
            post(handle_inspect_archive).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route(
            "/restore",
            post(handle_restore_archive).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route("/jobs", get(handle_list_jobs))
        .route("/jobs/{id}/control", post(handle_control_job))
}

async fn require_user(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let raw = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    Ok(authenticate_session(&state.db, &state.config, raw).await?.id)
}

fn header_str(headers: &HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

async fn handle_get_reauth_methods(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<reauth::ReauthMethodsResponse>> {
    let uid = require_user(&state, &headers).await?;
    Ok(Json(reauth::get_reauth_methods(&state.db, uid).await?))
}

#[derive(Deserialize)]
struct SetPasswordBody {
    #[serde(default)]
    current_password: Option<String>,
    #[serde(default)]
    totp_or_backup: Option<String>,
    new_password: String,
}

async fn handle_set_export_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<SetPasswordBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    reauth::set_export_password(
        &state,
        &headers,
        uid,
        b.current_password.as_deref(),
        b.totp_or_backup.as_deref(),
        &b.new_password,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct RegisterPasskeyBody {
    credential_id: String,
    public_key_spki_b64: String,
    #[serde(default)]
    label: String,
}

async fn handle_register_passkey(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<RegisterPasskeyBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let id = reauth::register_device_passkey(&state, &headers, uid, &b.credential_id, &b.public_key_spki_b64, &b.label).await?;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
struct ChallengeBody {
    #[serde(default = "default_reauth")]
    purpose: String,
}
fn default_reauth() -> String {
    "reauth".into()
}

async fn handle_reauth_challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<ChallengeBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let (id, challenge_b64) = reauth::issue_passkey_challenge(&state.db, uid, &b.purpose).await?;
    Ok(Json(json!({ "challenge_id": id, "challenge_b64": challenge_b64 })))
}

async fn handle_reauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<reauth::ReauthRequest>,
) -> AppResult<Json<reauth::ReauthResponse>> {
    let uid = require_user(&state, &headers).await?;
    Ok(Json(reauth::verify_and_issue_export_token(&state, &headers, uid, &req).await?))
}

#[derive(Deserialize)]
struct ZipStepBody {
    #[serde(default)]
    export_token: String,
    step: String,
}

async fn handle_unencrypted_zip_step(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<ZipStepBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let tok = if b.export_token.is_empty() {
        header_str(&headers, "x-export-token")
    } else {
        b.export_token
    };
    reauth::advance_unencrypted_zip_step(&state.db, uid, &tok, &b.step).await?;
    let preview = if b.step == "ack_risk" {
        Some(engine::build_export_preview(&state.db, uid).await?)
    } else {
        None
    };
    Ok(Json(json!({ "ok": true, "preview": preview })))
}

#[derive(Deserialize)]
struct RevokeBody {
    #[serde(default)]
    export_token: String,
}

async fn handle_unencrypted_zip_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<RevokeBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let tok = if b.export_token.is_empty() {
        header_str(&headers, "x-export-token")
    } else {
        b.export_token
    };
    reauth::revoke_export_token(&state.db, uid, &tok).await?;
    Ok(Json(json!({ "revoked": true })))
}

async fn handle_export_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<engine::ExportPreviewResponse>> {
    let uid = require_user(&state, &headers).await?;
    Ok(Json(engine::build_export_preview(&state.db, uid).await?))
}

#[derive(Deserialize)]
struct ExportKdataBody {
    encryption_password: String,
    #[serde(default)]
    export_token: Option<String>,
    #[serde(flatten)]
    scope: engine::ExportScopeOptions,
}

struct ChannelStreamWriter {
    tx: tokio::sync::mpsc::Sender<Result<Bytes, std::io::Error>>,
    hasher: sha2::Sha256,
    written: u64,
}

impl Write for ChannelStreamWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use sha2::Digest;
        if buf.is_empty() {
            return Ok(0);
        }
        self.hasher.update(buf);
        self.written += buf.len() as u64;
        self.tx
            .blocking_send(Ok(Bytes::copy_from_slice(buf)))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "client disconnected"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

async fn handle_export_kdata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExportKdataBody>,
) -> AppResult<Response> {
    let uid = require_user(&state, &headers).await?;
    if body.encryption_password.chars().count() < 6 {
        return Err(AppError::BadRequest("encryption_password_too_short"));
    }
    let tok = body
        .export_token
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| header_str(&headers, "x-export-token"));
    reauth::consume_export_token(&state.db, uid, &tok, "kdata").await?;

    let dataset = engine::prepare_backup_dataset(&state.db, &state.storage, uid, &body.scope).await?;
    let job_id = Uuid::new_v4();
    let total_plain = dataset.manifest.total_plain_bytes as i64;
    let total_files = dataset.entries.len() as i32;
    let missing_cnt = dataset.manifest.missing_media.len() as i32;
    let honest_notice = if missing_cnt > 0 {
        format!("Có {missing_cnt} tệp chat riêng chưa có bản sao máy chủ khi xuất.")
    } else {
        "Bản sao lưu .kdata đã mã hóa đầy đủ.".to_string()
    };

    sqlx::query(
        "INSERT INTO backup_jobs (id, user_id, job_kind, format, status, total_bytes, total_files, missing_files, honest_notice)
         VALUES ($1, $2, 'export', 'kdata', 'running', $3, $4, $5, $6)",
    )
    .bind(job_id)
    .bind(uid)
    .bind(total_plain)
    .bind(total_files)
    .bind(missing_cnt)
    .bind(&honest_notice)
    .execute(&state.db)
    .await?;

    log_event(&state, uid, "export_kdata", &headers).await;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(16);
    let pw = body.encryption_password;
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        use sha2::Digest;
        let sink = ChannelStreamWriter {
            tx,
            hasher: sha2::Sha256::new(),
            written: 0,
        };
        let res = engine::write_prepared_dataset_to_kdata(sink, dataset, &pw, None);
        let rt = tokio::runtime::Handle::current();
        match res {
            Ok((finished_sink, _mf)) => {
                let sha = hex::encode(finished_sink.hasher.finalize());
                let written = finished_sink.written as i64;
                rt.spawn(async move {
                    let _ = sqlx::query(
                        "UPDATE backup_jobs SET status = 'verified', processed_bytes = $1, processed_files = total_files, checksum_sha256 = $2, updated_at = now() WHERE id = $3",
                    )
                    .bind(written)
                    .bind(sha)
                    .bind(job_id)
                    .execute(&db)
                    .await;
                });
            }
            Err(_) => {
                rt.spawn(async move {
                    let _ = sqlx::query(
                        "UPDATE backup_jobs SET status = 'failed', error_message = 'stream_export_error', updated_at = now() WHERE id = $1",
                    )
                    .bind(job_id)
                    .execute(&db)
                    .await;
                });
            }
        }
    });

    let stream = futures_util::stream::unfold(rx, |mut r| async move {
        r.recv().await.map(|item| (item, r))
    });
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let disp = format!("attachment; filename=\"KIYO_Backup_{date}.kdata\"");

    let mut resp = Response::new(Body::from_stream(stream));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/vnd.kiyo.kdata".parse().unwrap(),
    );
    resp.headers_mut()
        .insert(header::CONTENT_DISPOSITION, disp.parse().unwrap());
    resp.headers_mut()
        .insert("x-backup-job-id", job_id.to_string().parse().unwrap());
    Ok(resp)
}

#[derive(Deserialize)]
struct ExportZipBody {
    #[serde(default)]
    encryption_password: Option<String>,
    #[serde(default)]
    export_token: Option<String>,
    #[serde(flatten)]
    scope: engine::ExportScopeOptions,
}

async fn handle_export_zip_encrypted(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExportZipBody>,
) -> AppResult<Response> {
    let uid = require_user(&state, &headers).await?;
    let pw = body
        .encryption_password
        .as_deref()
        .filter(|p| p.chars().count() >= 6)
        .ok_or(AppError::BadRequest("zip_password_too_short"))?;
    let tok = body
        .export_token
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| header_str(&headers, "x-export-token"));
    reauth::consume_export_token(&state.db, uid, &tok, "zip_encrypted").await?;

    let dataset = engine::prepare_backup_dataset(&state.db, &state.storage, uid, &body.scope).await?;
    let (zip_bytes, _mf) = engine::build_zip_from_dataset(dataset, Some(pw)).await?;
    log_event(&state, uid, "export_zip_aes256", &headers).await;

    let date = Utc::now().format("%Y-%m-%d").to_string();
    let disp = format!("attachment; filename=\"KIYO_Backup_{date}_AES256.zip\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, disp),
        ],
        zip_bytes,
    )
        .into_response())
}

async fn handle_export_zip_unencrypted(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExportZipBody>,
) -> AppResult<Response> {
    let uid = require_user(&state, &headers).await?;
    let tok = body
        .export_token
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| header_str(&headers, "x-export-token"));
    // Strictly requires multi-step confirmation + immediate single-use revocation!
    reauth::consume_export_token(&state.db, uid, &tok, "zip_unencrypted").await?;

    let dataset = engine::prepare_backup_dataset(&state.db, &state.storage, uid, &body.scope).await?;
    let (zip_bytes, _mf) = engine::build_zip_from_dataset(dataset, None).await?;
    log_event(&state, uid, "export_zip_unencrypted", &headers).await;

    let date = Utc::now().format("%Y-%m-%d").to_string();
    let disp = format!("attachment; filename=\"KIYO_Backup_{date}_UNENCRYPTED.zip\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, disp),
        ],
        zip_bytes,
    )
        .into_response())
}

#[derive(Deserialize)]
struct RestoreQuery {
    #[serde(default = "default_full")]
    mode: String,
    #[serde(default)]
    conversation_id: Option<String>,
}
fn default_full() -> String {
    "full".into()
}

async fn handle_inspect_archive(
    State(state): State<AppState>,
    headers: HeaderMap,
    raw_bytes: Bytes,
) -> AppResult<Json<engine::InspectArchiveResponse>> {
    let uid = require_user(&state, &headers).await?;
    let pw = headers
        .get("x-backup-password")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let res = engine::inspect_backup_archive(&state.db, uid, &raw_bytes, pw.as_deref()).await?;
    Ok(Json(res))
}

async fn handle_restore_archive(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<RestoreQuery>,
    raw_bytes: Bytes,
) -> AppResult<Json<engine::RestoreResultResponse>> {
    let uid = require_user(&state, &headers).await?;
    let pw = headers
        .get("x-backup-password")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let opts = engine::RestoreOptions {
        mode: q.mode,
        conversation_id: q.conversation_id,
        password: pw,
    };
    let res = engine::restore_backup_archive(&state.db, &state.storage, uid, &raw_bytes, &opts).await?;
    log_event(&state, uid, "backup_restore", &headers).await;
    Ok(Json(res))
}

async fn handle_list_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, job_kind, format, status, total_bytes, processed_bytes, total_files, processed_files, missing_files, checksum_sha256, honest_notice, error_message, created_at, updated_at
         FROM backup_jobs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let mut jobs = Vec::with_capacity(rows.len());
    for r in rows {
        let id: Uuid = r.try_get("id")?;
        let created_at: chrono::DateTime<Utc> = r.try_get("created_at")?;
        let updated_at: chrono::DateTime<Utc> = r.try_get("updated_at")?;
        jobs.push(json!({
            "id": id,
            "job_kind": r.try_get::<String, _>("job_kind")?,
            "format": r.try_get::<String, _>("format")?,
            "status": r.try_get::<String, _>("status")?,
            "total_bytes": r.try_get::<i64, _>("total_bytes")?,
            "processed_bytes": r.try_get::<i64, _>("processed_bytes")?,
            "total_files": r.try_get::<i32, _>("total_files")?,
            "processed_files": r.try_get::<i32, _>("processed_files")?,
            "missing_files": r.try_get::<i32, _>("missing_files")?,
            "checksum_sha256": r.try_get::<Option<String>, _>("checksum_sha256")?,
            "honest_notice": r.try_get::<Option<String>, _>("honest_notice")?,
            "error_message": r.try_get::<Option<String>, _>("error_message")?,
            "created_at": created_at.to_rfc3339(),
            "updated_at": updated_at.to_rfc3339(),
        }));
    }
    Ok(Json(json!({ "jobs": jobs })))
}

#[derive(Deserialize)]
struct ControlBody {
    action: String, // "pause" | "resume" | "cancel"
}

async fn handle_control_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
    Json(b): Json<ControlBody>,
) -> AppResult<Json<Value>> {
    let uid = require_user(&state, &headers).await?;
    let next_status = match b.action.as_str() {
        "pause" => "paused",
        "resume" => "running",
        "cancel" => "cancelled",
        _ => return Err(AppError::BadRequest("invalid_job_action")),
    };
    let affected = sqlx::query(
        "UPDATE backup_jobs SET status = $1, updated_at = now()
         WHERE id = $2 AND user_id = $3 AND status IN ('preparing', 'running', 'paused')",
    )
    .bind(next_status)
    .bind(job_id)
    .bind(uid)
    .execute(&state.db)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound("job_not_found_or_finished"));
    }
    Ok(Json(json!({ "id": job_id, "status": next_status })))
}
