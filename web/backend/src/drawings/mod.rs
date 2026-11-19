//! Spec 010: private drawing / animation projects. A project keeps every layer
//! and frame (PNG data URLs inside a small JSON document) so it can be opened
//! and edited again. Only the owner can list, read, change or delete it.

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::auth_user,
};

const MAX_PROJECT_BYTES: usize = 6 * 1024 * 1024;
const MAX_THUMB_BYTES: usize = 256 * 1024;
const MAX_PROJECTS: i64 = 100;
const MAX_LAYERS: usize = 12;
const MAX_FRAMES: usize = 60;
const KINDS: [&str; 8] = ["free", "avatar", "cover", "sticker", "emoji", "decoration", "nameplate", "effect"];
const PNG_PREFIX: &str = "data:image/png;base64,";
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(fetch).put(update).delete(remove))
        .route("/{id}/thumb", get(thumb))
        .layer(DefaultBodyLimit::max(MAX_PROJECT_BYTES + 512 * 1024))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveProject {
    name: String,
    #[serde(default)]
    kind: Option<String>,
    /// The project document (JSON text).
    data: String,
    /// Base64 PNG thumbnail (optional).
    #[serde(default)]
    thumb: Option<String>,
}

struct Checked {
    name: String,
    kind: String,
    width: i32,
    height: i32,
    frames: i32,
    layers: i32,
    data: Vec<u8>,
    thumb: Option<Vec<u8>>,
}

/// Validate the document shape: dimensions, counts and that every cell is a PNG data URL.
pub fn check_document(doc: &Value) -> Result<(i32, i32, i32, i32), &'static str> {
    if doc.get("v").and_then(|v| v.as_i64()) != Some(1) {
        return Err("invalid_project_version");
    }
    let dim = |k: &str| doc.get(k).and_then(|v| v.as_i64()).filter(|v| (16..=1024).contains(v));
    let (Some(w), Some(h)) = (dim("w"), dim("h")) else {
        return Err("invalid_project_size");
    };
    let layers = doc.get("layers").and_then(|l| l.as_array()).ok_or("invalid_project_layers")?;
    let frames = doc.get("frames").and_then(|f| f.as_array()).ok_or("invalid_project_frames")?;
    if layers.is_empty() || layers.len() > MAX_LAYERS {
        return Err("too_many_layers");
    }
    if frames.is_empty() || frames.len() > MAX_FRAMES {
        return Err("too_many_frames");
    }
    for l in layers {
        let name_ok = l.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.chars().count() <= 40);
        if !name_ok {
            return Err("invalid_layer");
        }
    }
    for f in frames {
        let cells = f.get("cells").and_then(|c| c.as_array()).ok_or("invalid_frame")?;
        if cells.len() != layers.len() {
            return Err("invalid_frame");
        }
        for c in cells {
            match c {
                Value::Null => {}
                Value::String(s) if s.starts_with(PNG_PREFIX) => {
                    let raw = base64::engine::general_purpose::STANDARD.decode(&s[PNG_PREFIX.len()..]).map_err(|_| "invalid_cell")?;
                    if !raw.starts_with(&PNG_MAGIC) {
                        return Err("invalid_cell");
                    }
                }
                _ => return Err("invalid_cell"),
            }
        }
    }
    Ok((w as i32, h as i32, frames.len() as i32, layers.len() as i32))
}

fn check(p: SaveProject) -> AppResult<Checked> {
    let name = p.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(|c| c.is_control()) {
        return Err(AppError::BadRequest("invalid_project_name"));
    }
    let kind = p.kind.unwrap_or_else(|| "free".into());
    if !KINDS.contains(&kind.as_str()) {
        return Err(AppError::BadRequest("invalid_project_kind"));
    }
    if p.data.len() > MAX_PROJECT_BYTES {
        return Err(AppError::BadRequest("project_too_large"));
    }
    let doc: Value = serde_json::from_str(&p.data).map_err(|_| AppError::BadRequest("invalid_project"))?;
    let (width, height, frames, layers) = check_document(&doc).map_err(AppError::BadRequest)?;
    let thumb = match p.thumb.as_deref().filter(|t| !t.is_empty()) {
        None => None,
        Some(t) => {
            let raw = base64::engine::general_purpose::STANDARD.decode(t.trim_start_matches(PNG_PREFIX)).map_err(|_| AppError::BadRequest("invalid_thumb"))?;
            if raw.len() > MAX_THUMB_BYTES || !raw.starts_with(&PNG_MAGIC) {
                return Err(AppError::BadRequest("invalid_thumb"));
            }
            Some(raw)
        }
    };
    Ok(Checked { name, kind, width, height, frames, layers, data: p.data.into_bytes(), thumb })
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        r#"SELECT id, name, kind, width, height, frame_count, layer_count, octet_length(data) AS "size!", thumb IS NOT NULL AS "has_thumb!", updated_at
           FROM drawing_projects WHERE owner_id = $1 ORDER BY updated_at DESC LIMIT 200"#,
        me
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|r| json!({
            "id": r.id, "name": r.name, "kind": r.kind, "width": r.width, "height": r.height,
            "frames": r.frame_count, "layers": r.layer_count, "size": r.size, "has_thumb": r.has_thumb, "updated_at": r.updated_at
        }))
        .collect::<Vec<_>>())))
}

async fn create(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<SaveProject>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let c = check(p)?;
    let count = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM drawing_projects WHERE owner_id = $1"#, me).fetch_one(&state.db).await?;
    if count >= MAX_PROJECTS {
        return Err(AppError::BadRequest("project_limit_reached"));
    }
    let r = sqlx::query!(
        r#"INSERT INTO drawing_projects (owner_id, name, kind, width, height, frame_count, layer_count, data, thumb)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id, updated_at"#,
        me, c.name, c.kind, c.width, c.height, c.frames, c.layers, c.data, c.thumb
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "id": r.id, "updated_at": r.updated_at })))
}

async fn update(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(p): Json<SaveProject>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let c = check(p)?;
    let r = sqlx::query!(
        r#"UPDATE drawing_projects SET name = $3, kind = $4, width = $5, height = $6, frame_count = $7, layer_count = $8,
                  data = $9, thumb = COALESCE($10, thumb), updated_at = now()
           WHERE id = $1 AND owner_id = $2 RETURNING updated_at"#,
        id, me, c.name, c.kind, c.width, c.height, c.frames, c.layers, c.data, c.thumb
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("project_not_found"))?;
    Ok(Json(json!({ "id": id, "updated_at": r.updated_at })))
}

async fn fetch(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("SELECT name, kind, data, updated_at FROM drawing_projects WHERE id = $1 AND owner_id = $2", id, me)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("project_not_found"))?;
    let doc: Value = serde_json::from_slice(&r.data).map_err(|_| AppError::Internal)?;
    Ok(Json(json!({ "id": id, "name": r.name, "kind": r.kind, "updated_at": r.updated_at, "data": doc })))
}

async fn remove(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let res = sqlx::query!("DELETE FROM drawing_projects WHERE id = $1 AND owner_id = $2", id, me).execute(&state.db).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("project_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

async fn thumb(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Response> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("SELECT thumb FROM drawing_projects WHERE id = $1 AND owner_id = $2", id, me)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("project_not_found"))?;
    let bytes = r.thumb.ok_or(AppError::NotFound("thumb_not_found"))?;
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PX: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    #[test]
    fn document_validation() {
        let ok = json!({ "v": 1, "w": 64, "h": 64, "layers": [{ "name": "Layer 1" }], "frames": [{ "cells": [PX] }, { "cells": [null] }] });
        assert_eq!(check_document(&ok), Ok((64, 64, 2, 1)));
        let html = json!({ "v": 1, "w": 64, "h": 64, "layers": [{ "name": "L" }], "frames": [{ "cells": ["data:text/html;base64,PHNjcmlwdD4="] }] });
        assert!(check_document(&html).is_err());
        let fake_png = json!({ "v": 1, "w": 64, "h": 64, "layers": [{ "name": "L" }], "frames": [{ "cells": ["data:image/png;base64,PHNjcmlwdD4="] }] });
        assert!(check_document(&fake_png).is_err());
        let big = json!({ "v": 1, "w": 4096, "h": 64, "layers": [{ "name": "L" }], "frames": [{ "cells": [null] }] });
        assert!(check_document(&big).is_err());
        let mismatch = json!({ "v": 1, "w": 64, "h": 64, "layers": [{ "name": "A" }, { "name": "B" }], "frames": [{ "cells": [null] }] });
        assert!(check_document(&mismatch).is_err());
    }
}
