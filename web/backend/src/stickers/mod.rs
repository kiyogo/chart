use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
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
        .route("/packs/{pack_id}", get(get_pack))
        .route("/packs/{pack_id}/stickers", post(add_sticker))
}

// 1. List all sticker packs with stickers
async fn list_packs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<StickerPackItem>>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let packs = sqlx::query!(
        r#"
        SELECT id, name, description, author_id, cover_url, created_at
        FROM sticker_packs
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let all_stickers = sqlx::query!(
        r#"
        SELECT id, pack_id, name, file_url, is_animated, created_at
        FROM stickers
        ORDER BY created_at ASC
        "#
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
