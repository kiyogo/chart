use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    models::{
        AddStickerRequest, CreateStickerPackRequest, StickerItem, StickerPackItem,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/packs", get(list_packs).post(create_pack))
        .route("/packs/{pack_id}", get(get_pack).delete(delete_pack))
        .route("/packs/{pack_id}/stickers", post(add_sticker))
        .route("/packs/{pack_id}/stickers/{sticker_id}", delete(delete_sticker))
        .route(
            "/upload",
            post(upload_sticker_image).layer(DefaultBodyLimit::max(MAX_STICKER_BYTES)),
        )
}

const MAX_STICKER_BYTES: usize = 512 * 1024;

/// Detect the real image type from magic bytes (the Content-Type header is not trusted).
fn sniff_sticker_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && (&bytes[8..12] == b"avif" || &bytes[8..12] == b"avis") {
        Some("avif")
    } else {
        None
    }
}

// Upload a sticker image. Returns the served URL to use in `add_sticker`.
async fn upload_sticker_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    if body.is_empty() || body.len() > MAX_STICKER_BYTES {
        return Err(AppError::BadRequest("invalid_sticker_size"));
    }
    let ext = sniff_sticker_ext(&body).ok_or(AppError::BadRequest("unsupported_sticker_format"))?;
    let dir = state.config.uploads_dir.join("stickers");
    tokio::fs::create_dir_all(&dir).await.map_err(|_| AppError::Internal)?;
    let filename = format!("{}.{ext}", Uuid::new_v4());
    tokio::fs::write(dir.join(&filename), &body).await.map_err(|_| AppError::Internal)?;
    Ok(Json(serde_json::json!({ "url": format!("/uploads/stickers/{filename}"), "is_animated": ext == "gif" })))
}

// Delete a whole pack (author only; the built-in pack has no author and cannot be deleted).
async fn delete_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(pack_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    let pack = sqlx::query!("SELECT author_id FROM sticker_packs WHERE id = $1", pack_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("pack_not_found"))?;
    if pack.author_id != Some(user.id) {
        return Err(AppError::Forbidden("not_pack_author"));
    }
    sqlx::query!("DELETE FROM sticker_packs WHERE id = $1", pack_id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// Delete one sticker from a pack (author only).
async fn delete_sticker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((pack_id, sticker_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    let pack = sqlx::query!("SELECT author_id FROM sticker_packs WHERE id = $1", pack_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("pack_not_found"))?;
    if pack.author_id != Some(user.id) {
        return Err(AppError::Forbidden("not_pack_author"));
    }
    let res = sqlx::query!("DELETE FROM stickers WHERE id = $1 AND pack_id = $2", sticker_id, pack_id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("sticker_not_found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// 1. List all sticker packs with stickers
async fn list_packs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<StickerPackItem>>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // Built-in packs (no author) and the caller's own packs; other users' packs stay private.
    let packs = sqlx::query!(
        r#"
        SELECT id, name, description, author_id, cover_url, created_at
        FROM sticker_packs
        WHERE author_id IS NULL OR author_id = $1
        ORDER BY (author_id IS NULL) DESC, created_at ASC
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let pack_ids: Vec<Uuid> = packs.iter().map(|p| p.id).collect();
    let all_stickers = sqlx::query!(
        r#"
        SELECT id, pack_id, name, file_url, is_animated, created_at
        FROM stickers
        WHERE pack_id = ANY($1)
        ORDER BY created_at ASC
        "#,
        &pack_ids
    )
    .fetch_all(&state.db)
    .await?;

    let mut stickers_by_pack: std::collections::HashMap<Uuid, Vec<StickerItem>> = std::collections::HashMap::new();
    for s in all_stickers {
        stickers_by_pack.entry(s.pack_id).or_default().push(StickerItem {
            id: s.id,
            pack_id: s.pack_id,
            name: s.name,
            file_url: s.file_url,
            is_animated: s.is_animated,
            created_at: s.created_at,
        });
    }

    let result = packs
        .into_iter()
        .map(|p| {
            let stickers = stickers_by_pack.remove(&p.id).unwrap_or_default();
            StickerPackItem {
                id: p.id,
                name: p.name,
                description: p.description,
                author_id: p.author_id,
                cover_url: p.cover_url,
                created_at: p.created_at,
                stickers,
            }
        })
        .collect();

    Ok(Json(result))
}

// 2. Get pack details
async fn get_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(pack_id): Path<Uuid>,
) -> AppResult<Json<StickerPackItem>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let p = sqlx::query!(
        r#"
        SELECT id, name, description, author_id, cover_url, created_at
        FROM sticker_packs
        WHERE id = $1
        "#,
        pack_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("pack_not_found"))?;

    if p.author_id.is_some() && p.author_id != Some(_user.id) {
        return Err(AppError::Forbidden("not_pack_author"));
    }

    let stickers_rows = sqlx::query!(
        r#"
        SELECT id, pack_id, name, file_url, is_animated, created_at
        FROM stickers
        WHERE pack_id = $1
        ORDER BY created_at ASC
        "#,
        pack_id
    )
    .fetch_all(&state.db)
    .await?;

    let stickers = stickers_rows
        .into_iter()
        .map(|s| StickerItem {
            id: s.id,
            pack_id: s.pack_id,
            name: s.name,
            file_url: s.file_url,
            is_animated: s.is_animated,
            created_at: s.created_at,
        })
        .collect();

    Ok(Json(StickerPackItem {
        id: p.id,
        name: p.name,
        description: p.description,
        author_id: p.author_id,
        cover_url: p.cover_url,
        created_at: p.created_at,
        stickers,
    }))
}

// 3. Create pack
async fn create_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateStickerPackRequest>,
) -> AppResult<Json<StickerPackItem>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("pack_name_empty"));
    }
    if let Some(ref c) = payload.cover_url {
        let trimmed = c.trim();
        if !trimmed.is_empty() && (!trimmed.starts_with("/uploads/stickers/") || trimmed.contains("..") || trimmed.len() > 300) {
            return Err(AppError::BadRequest("invalid_cover_url"));
        }
    }

    let pack_id = Uuid::new_v4();
    let row = sqlx::query!(
        r#"
        INSERT INTO sticker_packs (id, name, description, author_id, cover_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, description, author_id, cover_url, created_at
        "#,
        pack_id,
        payload.name.trim(),
        payload.description,
        user.id,
        payload.cover_url
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(StickerPackItem {
        id: row.id,
        name: row.name,
        description: row.description,
        author_id: row.author_id,
        cover_url: row.cover_url,
        created_at: row.created_at,
        stickers: Vec::new(),
    }))
}

// 4. Add sticker to pack
async fn add_sticker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(pack_id): Path<Uuid>,
    Json(payload): Json<AddStickerRequest>,
) -> AppResult<Json<StickerItem>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let pack = sqlx::query!("SELECT author_id FROM sticker_packs WHERE id = $1", pack_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("pack_not_found"))?;

    if pack.author_id != Some(user.id) {
        return Err(AppError::Forbidden("not_pack_author"));
    }

    if payload.name.trim().is_empty() || payload.file_url.trim().is_empty() {
        return Err(AppError::BadRequest("invalid_sticker_data"));
    }
    if !payload.file_url.trim().starts_with("/uploads/stickers/") {
        return Err(AppError::BadRequest("invalid_sticker_url"));
    }

    let sticker_id = Uuid::new_v4();
    let is_anim = payload.is_animated.unwrap_or(false);
    let row = sqlx::query!(
        r#"
        INSERT INTO stickers (id, pack_id, name, file_url, is_animated)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, pack_id, name, file_url, is_animated, created_at
        "#,
        sticker_id,
        pack_id,
        payload.name.trim(),
        payload.file_url.trim(),
        is_anim
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(StickerItem {
        id: row.id,
        pack_id: row.pack_id,
        name: row.name,
        file_url: row.file_url,
        is_animated: row.is_animated,
        created_at: row.created_at,
    }))
}

