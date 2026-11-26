//! Personal media library: generic image upload (used by custom emoji, saved GIFs,
//! profile decorations / nameplates / effects) plus per-user emoji and GIF lists.

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};

const MAX_MEDIA_BYTES: usize = 10 * 1024 * 1024;
const MAX_EMOJIS: i64 = 200;
const MAX_GIFS: i64 = 300;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload_media).layer(DefaultBodyLimit::max(MAX_MEDIA_BYTES)))
        .route("/emojis", get(list_emojis).post(create_emoji))
        .route("/emojis/{id}", delete(delete_emoji))
        .route("/gifs", get(list_gifs).post(create_gif))
        .route("/gifs/{id}", delete(delete_gif))
}

/// Real image type from magic bytes; the Content-Type header is never trusted.
pub fn sniff_image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && matches!(&bytes[8..12], b"avif" | b"avis" | b"mif1" | b"msf1")
    {
        Some("avif")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else {
        None
    }
}

fn ext_to_mime(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "application/octet-stream",
    }
}

async fn auth(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let token = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    Ok(authenticate_session(&state.db, &state.config, token).await?.id)
}

async fn upload_media(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> AppResult<Json<serde_json::Value>> {
    let uid = auth(&state, &headers).await?;
    if body.is_empty() || body.len() > MAX_MEDIA_BYTES {
        return Err(AppError::BadRequest("invalid_media_size"));
    }
    let ct = headers.get(axum::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok());
    if crate::storage::router::is_blocked_svg_upload(None, None, ct, Some(&body)) {
        return Err(AppError::BadRequest("svg_upload_not_allowed"));
    }
    let ext = sniff_image_ext(&body).ok_or(AppError::BadRequest("unsupported_media_format"))?;
    let mime = ext_to_mime(ext);
    let dir = state.config.uploads_dir.join("media");
    tokio::fs::create_dir_all(&dir).await.map_err(|_| AppError::Internal)?;
    let filename = format!("{}.{ext}", Uuid::new_v4());
    let file_path = dir.join(&filename);
    tokio::fs::write(&file_path, &body).await.map_err(|_| AppError::Internal)?;

    if let Err(err) = sqlx::query(
        "INSERT INTO library_media (file_name, uploader_id, mime_type, size_bytes) VALUES ($1, $2, $3, $4) ON CONFLICT (file_name) DO NOTHING",
    )
    .bind(&filename)
    .bind(uid)
    .bind(mime)
    .bind(body.len() as i64)
    .execute(&state.db)
    .await
    {
        let _ = tokio::fs::remove_file(&file_path).await;
        return Err(err.into());
    }

    Ok(Json(serde_json::json!({ "url": format!("/uploads/media/{filename}"), "animated": ext == "gif" })))
}

#[derive(Serialize)]
struct EmojiItem {
    id: Uuid,
    name: String,
    file_url: String,
}

#[derive(Deserialize)]
struct CreateEmoji {
    name: String,
    file_url: String,
}

fn valid_emoji_name(n: &str) -> bool {
    (2..=32).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

async fn list_emojis(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<EmojiItem>>> {
    let uid = auth(&state, &headers).await?;
    let rows = sqlx::query!("SELECT id, name, file_url FROM custom_emojis WHERE owner_id = $1 ORDER BY name", uid)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(|r| EmojiItem { id: r.id, name: r.name, file_url: r.file_url }).collect()))
}

async fn create_emoji(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<CreateEmoji>) -> AppResult<Json<EmojiItem>> {
    let uid = auth(&state, &headers).await?;
    let name = p.name.trim().to_string();
    if !valid_emoji_name(&name) {
        return Err(AppError::BadRequest("invalid_emoji_name"));
    }
    if !p.file_url.starts_with("/uploads/media/") || p.file_url.contains("..") {
        return Err(AppError::BadRequest("invalid_emoji_url"));
    }
    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM custom_emojis WHERE owner_id = $1"#, uid)
        .fetch_one(&state.db)
        .await?;
    if count >= MAX_EMOJIS {
        return Err(AppError::BadRequest("emoji_limit_reached"));
    }
    let res = sqlx::query!(
        "INSERT INTO custom_emojis (owner_id, name, file_url) VALUES ($1, $2, $3) RETURNING id, name, file_url",
        uid,
        name,
        p.file_url
    )
    .fetch_one(&state.db)
    .await;
    match res {
        Ok(r) => Ok(Json(EmojiItem { id: r.id, name: r.name, file_url: r.file_url })),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AppError::Conflict("emoji_name_taken")),
        Err(e) => Err(e.into()),
    }
}

async fn delete_emoji(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth(&state, &headers).await?;
    let res = sqlx::query!("DELETE FROM custom_emojis WHERE id = $1 AND owner_id = $2", id, uid)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("emoji_not_found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Serialize)]
struct GifItem {
    id: Uuid,
    name: String,
    file_url: String,
}

#[derive(Deserialize)]
struct CreateGif {
    name: String,
    file_url: String,
}

async fn list_gifs(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<GifItem>>> {
    let uid = auth(&state, &headers).await?;
    let rows = sqlx::query!("SELECT id, name, file_url FROM saved_gifs WHERE owner_id = $1 ORDER BY created_at DESC", uid)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(|r| GifItem { id: r.id, name: r.name, file_url: r.file_url }).collect()))
}

async fn create_gif(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<CreateGif>) -> AppResult<Json<GifItem>> {
    let uid = auth(&state, &headers).await?;
    let name = p.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AppError::BadRequest("invalid_gif_name"));
    }
    let ok_url = (p.file_url.starts_with("/uploads/media/") || p.file_url.starts_with("/stickers/")) && !p.file_url.contains("..");
    if !ok_url {
        return Err(AppError::BadRequest("invalid_gif_url"));
    }
    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM saved_gifs WHERE owner_id = $1"#, uid)
        .fetch_one(&state.db)
        .await?;
    if count >= MAX_GIFS {
        return Err(AppError::BadRequest("gif_limit_reached"));
    }
    let r = sqlx::query!(
        "INSERT INTO saved_gifs (owner_id, name, file_url) VALUES ($1, $2, $3) RETURNING id, name, file_url",
        uid,
        name,
        p.file_url
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(GifItem { id: r.id, name: r.name, file_url: r.file_url }))
}

async fn delete_gif(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth(&state, &headers).await?;
    let res = sqlx::query!("DELETE FROM saved_gifs WHERE id = $1 AND owner_id = $2", id, uid)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("gif_not_found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}
