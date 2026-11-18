//! "Kinh doanh": a business profile with a public page (`/k/{handle}`), product
//! catalogue, quick replies and greeting / away messages.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::auth_user,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_mine).put(upsert_mine).delete(delete_mine))
        .route("/products", post(create_product))
        .route("/products/{id}", put(update_product).delete(delete_product))
        .route("/quick-replies", post(create_quick_reply))
        .route("/quick-replies/{id}", delete(delete_quick_reply))
        .route("/public/{handle}", get(public_json))
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BusinessProfile {
    pub handle: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub address: Option<String>,
    pub hours: Option<String>,
    pub website: Option<String>,
    pub contact: Option<String>,
    pub cover_url: Option<String>,
    pub enabled: bool,
    pub greeting: Option<String>,
    pub away_message: Option<String>,
    #[serde(default)]
    pub verification_status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub position: i32,
}

#[derive(Serialize)]
struct QuickReply {
    id: Uuid,
    shortcut: String,
    text: String,
}

pub async fn load_profile(db: &sqlx::PgPool, user_id: Uuid) -> AppResult<Option<BusinessProfile>> {
    let r = sqlx::query!(
        "SELECT handle, name, category, description, address, hours, website, contact, cover_url, enabled, greeting, away_message, verification_status FROM business_profiles WHERE user_id = $1",
        user_id
    )
    .fetch_optional(db)
    .await?;
    Ok(r.map(|r| BusinessProfile {
        handle: r.handle,
        name: r.name,
        category: r.category,
        description: r.description,
        address: r.address,
        hours: r.hours,
        website: r.website,
        contact: r.contact,
        cover_url: r.cover_url,
        enabled: r.enabled,
        greeting: r.greeting,
        away_message: r.away_message,
        verification_status: r.verification_status,
    }))
}

pub async fn load_products(db: &sqlx::PgPool, user_id: Uuid) -> AppResult<Vec<Product>> {
    let rows = sqlx::query!(
        "SELECT id, name, price, description, image_url, position FROM business_products WHERE user_id = $1 ORDER BY position, created_at",
        user_id
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Product { id: r.id, name: r.name, price: r.price, description: r.description, image_url: r.image_url, position: r.position })
        .collect())
}

async fn get_mine(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let profile = load_profile(&state.db, uid).await?;
    let products = load_products(&state.db, uid).await?;
    let replies: Vec<QuickReply> = sqlx::query!("SELECT id, shortcut, text FROM business_quick_replies WHERE user_id = $1 ORDER BY shortcut", uid)
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|r| QuickReply { id: r.id, shortcut: r.shortcut, text: r.text })
        .collect();
    Ok(Json(serde_json::json!({ "profile": profile, "products": products, "quick_replies": replies })))
}

fn clean(v: &Option<String>, max: usize) -> AppResult<Option<String>> {
    match v.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() <= max => Ok(Some(s.to_string())),
        _ => Err(AppError::BadRequest("field_too_long")),
    }
}

#[derive(Deserialize)]
struct UpsertReq {
    handle: String,
    name: String,
    category: Option<String>,
    description: Option<String>,
    address: Option<String>,
    hours: Option<String>,
    website: Option<String>,
    contact: Option<String>,
    cover_url: Option<String>,
    enabled: Option<bool>,
    greeting: Option<String>,
    away_message: Option<String>,
    request_verification: Option<bool>,
}

async fn upsert_mine(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<UpsertReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let handle = p.handle.trim().to_lowercase();
    let ok_handle = (3..=32).contains(&handle.len())
        && handle.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && handle.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c));
    if !ok_handle {
        return Err(AppError::BadRequest("invalid_handle"));
    }
    let name = p.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::BadRequest("invalid_name"));
    }
    let website = clean(&p.website, 200)?;
    if let Some(ref w) = website {
        if !(w.starts_with("https://") || w.starts_with("http://")) {
            return Err(AppError::BadRequest("invalid_website"));
        }
    }
    let cover = clean(&p.cover_url, 300)?;
    if let Some(ref c) = cover {
        if !c.starts_with("/uploads/media/") || c.contains("..") {
            return Err(AppError::BadRequest("invalid_cover_url"));
        }
    }
    let res = sqlx::query!(
        r#"INSERT INTO business_profiles (user_id, handle, name, category, description, address, hours, website, contact, cover_url, enabled, greeting, away_message, verification_status)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, COALESCE($11, TRUE), $12, $13, CASE WHEN $14 THEN 'pending' ELSE 'none' END)
           ON CONFLICT (user_id) DO UPDATE SET
             handle = EXCLUDED.handle, name = EXCLUDED.name, category = EXCLUDED.category, description = EXCLUDED.description,
             address = EXCLUDED.address, hours = EXCLUDED.hours, website = EXCLUDED.website, contact = EXCLUDED.contact,
             cover_url = EXCLUDED.cover_url, enabled = COALESCE($11, business_profiles.enabled),
             greeting = EXCLUDED.greeting, away_message = EXCLUDED.away_message,
             verification_status = CASE WHEN $14 AND business_profiles.verification_status IN ('none', 'rejected') THEN 'pending' ELSE business_profiles.verification_status END,
             updated_at = now()"#,
        uid,
        handle,
        name,
        clean(&p.category, 60)?,
        clean(&p.description, 2000)?,
        clean(&p.address, 200)?,
        clean(&p.hours, 200)?,
        website,
        clean(&p.contact, 80)?,
        cover,
        p.enabled,
        clean(&p.greeting, 500)?,
        clean(&p.away_message, 500)?,
        p.request_verification.unwrap_or(false)
    )
    .execute(&state.db)
    .await;
    match res {
        Ok(_) => {}
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => return Err(AppError::Conflict("handle_taken")),
        Err(e) => return Err(e.into()),
    }
    get_mine(State(state), headers).await
}

async fn delete_mine(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM business_products WHERE user_id = $1", uid).execute(&mut *tx).await?;
    sqlx::query!("DELETE FROM business_quick_replies WHERE user_id = $1", uid).execute(&mut *tx).await?;
    sqlx::query!("DELETE FROM business_profiles WHERE user_id = $1", uid).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct ProductReq {
    name: String,
    price: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
    position: Option<i32>,
}

fn check_product(p: &ProductReq) -> AppResult<()> {
    if p.name.trim().is_empty() || p.name.chars().count() > 120 {
        return Err(AppError::BadRequest("invalid_product_name"));
    }
    if let Some(ref u) = p.image_url {
        if !u.is_empty() && (!u.starts_with("/uploads/media/") || u.contains("..")) {
            return Err(AppError::BadRequest("invalid_image_url"));
        }
    }
    Ok(())
}

async fn create_product(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<ProductReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    check_product(&p)?;
    if load_profile(&state.db, uid).await?.is_none() {
        return Err(AppError::BadRequest("business_profile_required"));
    }
    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM business_products WHERE user_id = $1"#, uid).fetch_one(&state.db).await?;
    if count >= 200 {
        return Err(AppError::BadRequest("product_limit_reached"));
    }
    sqlx::query!(
        "INSERT INTO business_products (user_id, name, price, description, image_url, position) VALUES ($1, $2, $3, $4, $5, $6)",
        uid,
        p.name.trim(),
        clean(&p.price, 40)?,
        clean(&p.description, 1000)?,
        p.image_url.filter(|u| !u.is_empty()),
        p.position.unwrap_or(count as i32)
    )
    .execute(&state.db)
    .await?;
    get_mine(State(state), headers).await
}

async fn update_product(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(p): Json<ProductReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    check_product(&p)?;
    let res = sqlx::query!(
        "UPDATE business_products SET name = $1, price = $2, description = $3, image_url = $4, position = COALESCE($5, position) WHERE id = $6 AND user_id = $7",
        p.name.trim(),
        clean(&p.price, 40)?,
        clean(&p.description, 1000)?,
        p.image_url.filter(|u| !u.is_empty()),
        p.position,
        id,
        uid
    )
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("product_not_found"));
    }
    get_mine(State(state), headers).await
}

async fn delete_product(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query!("DELETE FROM business_products WHERE id = $1 AND user_id = $2", id, uid).execute(&state.db).await?;
    get_mine(State(state), headers).await
}

#[derive(Deserialize)]
struct QuickReplyReq {
    shortcut: String,
    text: String,
}

async fn create_quick_reply(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<QuickReplyReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let sc = p.shortcut.trim().trim_start_matches('/').to_lowercase();
    if sc.is_empty() || sc.len() > 24 || !sc.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest("invalid_shortcut"));
    }
    let text = p.text.trim();
    if text.is_empty() || text.chars().count() > 1000 {
        return Err(AppError::BadRequest("invalid_text"));
    }
    let res = sqlx::query!("INSERT INTO business_quick_replies (user_id, shortcut, text) VALUES ($1, $2, $3)", uid, sc, text)
        .execute(&state.db)
        .await;
    match res {
        Ok(_) => get_mine(State(state), headers).await,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AppError::Conflict("shortcut_taken")),
        Err(e) => Err(e.into()),
    }
}

async fn delete_quick_reply(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query!("DELETE FROM business_quick_replies WHERE id = $1 AND user_id = $2", id, uid).execute(&state.db).await?;
    get_mine(State(state), headers).await
}

/// Public business card (no login needed): only enabled profiles.
pub async fn public_profile(db: &sqlx::PgPool, handle: &str) -> AppResult<Option<(Uuid, String, BusinessProfile, Vec<Product>)>> {
    let row = sqlx::query!(
        r#"SELECT b.user_id, u.username AS "username!" FROM business_profiles b JOIN users u ON u.id = b.user_id
           WHERE b.handle = $1 AND b.enabled AND u.suspended_at IS NULL"#,
        handle.to_lowercase()
    )
    .fetch_optional(db)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let profile = load_profile(db, row.user_id).await?.ok_or(AppError::NotFound("business_not_found"))?;
    let products = load_products(db, row.user_id).await?;
    Ok(Some((row.user_id, row.username, profile, products)))
}

async fn public_json(State(state): State<AppState>, Path(handle): Path<String>) -> AppResult<Json<serde_json::Value>> {
    let (uid, username, mut profile, products) = public_profile(&state.db, &handle).await?.ok_or(AppError::NotFound("business_not_found"))?;
    profile.greeting = None;
    profile.away_message = None;
    Ok(Json(serde_json::json!({ "user_id": uid, "username": username, "profile": profile, "products": products })))
}
