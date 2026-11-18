use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};
use crate::models::AttachmentItem;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handle_upload_attachment).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/{id}", get(handle_download_attachment).delete(handle_delete_attachment))
}

async fn handle_upload_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<AttachmentItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if body.is_empty() {
        return Err(AppError::BadRequest("empty_file_payload"));
    }

    let file_size = body.len() as i64;
    let file_name = headers
        .get("x-file-name")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "attachment.bin".to_string());

    let mime_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let attachment_id = Uuid::new_v4();
    let attach_dir = state.config.uploads_dir.join("attachments");
    tokio::fs::create_dir_all(&attach_dir).await.map_err(|e| {
        tracing::error!(error = %e, "failed to create attachments dir");
        AppError::Internal
    })?;

    let filename_on_disk = format!("{}.bin", attachment_id);
    let filepath = attach_dir.join(&filename_on_disk);

    tokio::fs::write(&filepath, &body).await.map_err(|e| {
        tracing::error!(error = %e, "failed to save attachment");
        AppError::Internal
    })?;

    let file_path_str = filepath.to_string_lossy().to_string();

    let row = sqlx::query!(
        r#"
        INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, uploader_id, file_name, file_size, mime_type, created_at
        "#,
        attachment_id,
        user.id,
        file_path_str,
        file_name,
        file_size,
        mime_type
    )
    .fetch_one(&state.db)
    .await?;

    let download_url = format!("/api/attachments/{}", row.id);

    Ok(Json(AttachmentItem {
        id: row.id,
        uploader_id: row.uploader_id,
        file_name: row.file_name,
        file_size: row.file_size,
        mime_type: row.mime_type,
        download_url,
        created_at: row.created_at,
    }))
}

async fn handle_download_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(attachment_id): Path<Uuid>,
) -> AppResult<Response> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query!(
        "SELECT file_path, file_name, mime_type FROM attachments WHERE id = $1",
        attachment_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("attachment_not_found"))?;

    let data = tokio::fs::read(&row.file_path).await.map_err(|e| {
        tracing::error!(error = %e, "failed to read attachment file");
        AppError::NotFound("file_not_found_on_disk")
    })?;

    let content_disposition = format!("attachment; filename=\"{}\"", row.file_name);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, row.mime_type),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        data,
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

    let row = sqlx::query!(
        "SELECT file_path, uploader_id FROM attachments WHERE id = $1",
        attachment_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("attachment_not_found"))?;

    if row.uploader_id != user.id {
        return Err(AppError::Forbidden("only_uploader_can_delete_attachment"));
    }

    sqlx::query!("DELETE FROM attachments WHERE id = $1", attachment_id)
        .execute(&state.db)
        .await?;

    let _ = tokio::fs::remove_file(&row.file_path).await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

