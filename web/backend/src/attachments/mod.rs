use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    storage::{
        StorageAdapter, migration,
        permissions::authorize_legacy_attachment_read,
        router::{UploadMetaQuery, ingest_media_bytes, serve_media_reference},
    },
};

const MAX_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024; // 50 MB max single-shot upload

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handle_upload_attachment).layer(DefaultBodyLimit::max(MAX_ATTACHMENT_BYTES)),
        )
        .route(
            "/{id}",
            get(handle_download_attachment).delete(handle_delete_attachment),
        )
}

/// Authenticated routes for `/uploads/*` (excluding `/uploads/avatars` which is public `.avif` only).
/// Blocks unauthenticated access and completely denies `/uploads/cas/*`, `/uploads/relay/*`, `/uploads/encrypted_backup/*`.
pub fn protected_uploads_routes() -> Router<AppState> {
    Router::new()
        .route("/media/{file_name}", get(handle_protected_media_file))
        .route("/banners/{file_name}", get(handle_protected_banner_file))
        .route("/stickers/{file_name}", get(handle_protected_sticker_file))
        .route("/{*rest}", get(handle_deny_private_uploads_dir))
}

async fn handle_deny_private_uploads_dir() -> StatusCode {
    StatusCode::FORBIDDEN
}

fn is_safe_single_filename(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

fn parse_http_range(h: &str) -> Option<(u64, Option<u64>)> {
    let rest = h.trim().strip_prefix("bytes=")?;
    let first = rest.split(',').next()?.trim();
    let (s, e) = first.split_once('-')?;
    let start: u64 = s.trim().parse().ok()?;
    let end: Option<u64> = if e.trim().is_empty() {
        None
    } else {
        Some(e.trim().parse().ok()?)
    };
    if let Some(end_val) = end
        && end_val < start
    {
        return None;
    }
    Some((start, end))
}

fn guess_mime_from_filename(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".avif") {
        "image/avif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else {
        "application/octet-stream"
    }
}

async fn serve_existing_key_with_range(
    state: &AppState,
    storage_key: &str,
    file_name: &str,
    mime_type: &str,
    range_hdr: Option<&str>,
) -> AppResult<Response> {
    let safe_name = file_name.replace('"', "");
    if let Some((start, end_opt)) = range_hdr.and_then(parse_http_range) {
        let meta = state
            .storage
            .existing
            .get_metadata(storage_key)
            .await
            .map_err(|_| AppError::NotFound("attachment_file_missing"))?;
        let total_size = meta.byte_size;
        let end = end_opt
            .unwrap_or(total_size.saturating_sub(1))
            .min(total_size.saturating_sub(1));
        let (chunk, full_len) = state
            .storage
            .existing
            .get_object_range(storage_key, start, end)
            .await
            .map_err(|_| AppError::NotFound("attachment_file_missing"))?;
        let end_actual = start + (chunk.len() as u64).saturating_sub(1);
        return Ok((
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_TYPE, mime_type.to_string()),
                (
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end_actual}/{full_len}"),
                ),
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{safe_name}\""),
                ),
            ],
            chunk,
        )
            .into_response());
    }

    let stream = state
        .storage
        .existing
        .get_object_stream(storage_key)
        .await
        .map_err(|_| AppError::NotFound("attachment_file_missing"))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime_type.to_string()),
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{safe_name}\""),
            ),
        ],
        stream,
    )
        .into_response())
}

async fn handle_protected_media_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> AppResult<Response> {
    if !is_safe_single_filename(&file_name) {
        return Err(AppError::BadRequest("invalid_file_name"));
    }
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    let range_hdr = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

    let rel_key = format!("media/{file_name}");
    if let Some(att_row) = sqlx::query(
        "SELECT uploader_id, file_name, mime_type, scope_type, scope_id, ref_id FROM attachments WHERE file_path = $1 OR file_path = $2 LIMIT 1",
    )
    .bind(&rel_key)
    .bind(&file_name)
    .fetch_optional(&state.db)
    .await?
    {
        if let Ok(Some(ref_id)) = att_row.try_get::<Option<Uuid>, _>("ref_id") {
            return serve_media_reference(&state.db, &state.storage, user.id, ref_id, None, range_hdr).await;
        }
        let uploader_id: Uuid = att_row.try_get("uploader_id")?;
        let scope_type: Option<String> = att_row.try_get("scope_type")?;
        let scope_id: Option<Uuid> = att_row.try_get("scope_id")?;
        if let Some(st) = scope_type {
            crate::storage::permissions::authorize_read(&state.db, user.id, &st, scope_id, uploader_id).await?;
        } else {
            authorize_legacy_attachment_read(&state.db, user.id, uploader_id).await?;
        }
        let fname: String = att_row.try_get("file_name").unwrap_or_else(|_| file_name.clone());
        let mime: String = att_row
            .try_get("mime_type")
            .unwrap_or_else(|_| guess_mime_from_filename(&file_name).to_string());
        return serve_existing_key_with_range(&state, &rel_key, &fname, &mime, range_hdr).await;
    }

    let mime = guess_mime_from_filename(&file_name);
    serve_existing_key_with_range(&state, &rel_key, &file_name, mime, range_hdr).await
}

async fn handle_protected_banner_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> AppResult<Response> {
    if !is_safe_single_filename(&file_name) {
        return Err(AppError::BadRequest("invalid_file_name"));
    }
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;
    let range_hdr = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let rel_key = format!("banners/{file_name}");
    let mime = guess_mime_from_filename(&file_name);
    serve_existing_key_with_range(&state, &rel_key, &file_name, mime, range_hdr).await
}

async fn handle_protected_sticker_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> AppResult<Response> {
    if !is_safe_single_filename(&file_name) {
        return Err(AppError::BadRequest("invalid_file_name"));
    }
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;
    let range_hdr = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let rel_key = format!("stickers/{file_name}");
    let mime = guess_mime_from_filename(&file_name);
    serve_existing_key_with_range(&state, &rel_key, &file_name, mime, range_hdr).await
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub variant: Option<String>,
}

async fn handle_upload_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<UploadMetaQuery>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let header_mime = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let header_name = headers
        .get("x-file-name")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let meta = UploadMetaQuery {
        scope_type: query.scope_type.or_else(|| {
            headers.get("x-scope-type").and_then(|v| v.to_str().ok()).map(str::to_string)
        }),
        scope_id: query.scope_id.or_else(|| {
            headers.get("x-scope-id").and_then(|v| v.to_str().ok()).and_then(|s| Uuid::parse_str(s).ok())
        }),
        message_id: query.message_id,
        file_name: query.file_name.or(header_name),
        mime_type: query.mime_type.or_else(|| {
            headers.get("x-mime-type").and_then(|v| v.to_str().ok()).map(str::to_string)
        }),
        sha256_plain: query.sha256_plain.or_else(|| {
            headers.get("x-sha256-plain").and_then(|v| v.to_str().ok()).map(str::to_string)
        }),
        sha256_cipher: query.sha256_cipher.or_else(|| {
            headers.get("x-sha256-cipher").and_then(|v| v.to_str().ok()).map(str::to_string)
        }),
        is_e2ee: query.is_e2ee,
        preserve_original: query.preserve_original,
        backup_dm: query.backup_dm,
        forward_from_ref_id: query.forward_from_ref_id,
    };

    let result = ingest_media_bytes(
        &state.db,
        &state.storage,
        user.id,
        user.device_id,
        body.to_vec(),
        header_mime,
        meta,
    )
    .await?;

    Ok(Json(result))
}

async fn handle_download_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(attachment_id): Path<Uuid>,
    Query(q): Query<DownloadQuery>,
) -> AppResult<Response> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let range_hdr = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

    // Prefer authorized media_references resolution (enforces DM, Group & Server RBAC checks + variants).
    let exists_in_refs: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_references WHERE id = $1)")
        .bind(attachment_id)
        .fetch_one(&state.db)
        .await?;
    if exists_in_refs {
        return serve_media_reference(
            &state.db,
            &state.storage,
            user.id,
            attachment_id,
            q.variant.as_deref(),
            range_hdr,
        )
        .await;
    }

    // Legacy fallback for rows uploaded prior to migration 0026: close IDOR by validating uploader/shared scope.
    let row = sqlx::query(
        "SELECT uploader_id, file_path, file_name, mime_type, scope_type, scope_id, ref_id FROM attachments WHERE id = $1",
    )
    .bind(attachment_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("attachment_not_found"))?;

    if let Ok(Some(ref_id)) = row.try_get::<Option<Uuid>, _>("ref_id") {
        return serve_media_reference(&state.db, &state.storage, user.id, ref_id, q.variant.as_deref(), range_hdr).await;
    }

    let uploader_id: Uuid = row.try_get("uploader_id")?;
    let scope_type: Option<String> = row.try_get("scope_type")?;
    let scope_id: Option<Uuid> = row.try_get("scope_id")?;
    if let Some(st) = scope_type {
        crate::storage::permissions::authorize_read(&state.db, user.id, &st, scope_id, uploader_id).await?;
    } else {
        authorize_legacy_attachment_read(&state.db, user.id, uploader_id).await?;
    }

    let file_path: String = row.try_get("file_path")?;
    let file_name: String = row.try_get("file_name")?;
    let mime_type: String = row.try_get("mime_type")?;

    serve_existing_key_with_range(&state, &file_path, &file_name, &mime_type, range_hdr).await
}

async fn handle_delete_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(attachment_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query(
        "SELECT file_path, uploader_id, ref_id FROM attachments WHERE id = $1",
    )
    .bind(attachment_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("attachment_not_found"))?;

    let uploader_id: Uuid = row.try_get("uploader_id")?;
    if uploader_id != user.id {
        return Err(AppError::Forbidden("not_uploader"));
    }

    if let Ok(Some(ref_id)) = row.try_get::<Option<Uuid>, _>("ref_id") {
        let mr = sqlx::query(
            "SELECT object_id, optimized_object_id, thumb_object_id FROM media_references WHERE id = $1",
        )
        .bind(ref_id)
        .fetch_optional(&state.db)
        .await?;
        if let Some(m) = mr {
            for col in ["object_id", "optimized_object_id", "thumb_object_id"] {
                if let Ok(Some(oid)) = m.try_get::<Option<Uuid>, _>(col) {
                    let _ = migration::release_object_reference(&state.db, &state.storage, oid).await;
                }
            }
            let _ = sqlx::query(
                "UPDATE media_references SET object_id = NULL, optimized_object_id = NULL, thumb_object_id = NULL, deleted_for_all = TRUE, availability_state = 'deleted_for_everyone' WHERE id = $1",
            )
            .bind(ref_id)
            .execute(&state.db)
            .await;
        }
    } else {
        let file_path: String = row.try_get("file_path")?;
        let _ = state.storage.existing.delete_object(&file_path).await;
    }

    sqlx::query("DELETE FROM attachments WHERE id = $1")
        .bind(attachment_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}
