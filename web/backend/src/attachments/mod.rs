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

    let bytes = state
        .storage
        .existing
        .get_object(&file_path)
        .await
        .map_err(|_| AppError::NotFound("attachment_file_missing"))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", file_name.replace('"', "")),
            ),
        ],
        bytes,
    )
        .into_response())
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
