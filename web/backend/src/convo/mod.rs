//! Per-user conversation organisation: labels, pins, "show in" placement,
//! read receipts, shared wallpapers and the "Chính mình" self chat switch.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::{auth_user, are_friends},
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/labels", get(list_labels).post(create_label))
        .route("/labels/{id}", put(update_label).delete(delete_label))
        .route("/prefs", get(list_prefs))
        .route("/prefs/{conv_type}/{conv_id}", put(update_pref))
        .route("/reads", post(mark_read))
        .route("/reads/{partner_id}", get(get_partner_read))
        .route("/wallpaper/{conv_type}/{conv_id}", get(get_wallpaper).put(set_wallpaper))
        .route("/self-chat", post(enable_self_chat).delete(delete_self_chat))
}

fn hex_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

#[derive(Serialize)]
struct Label {
    id: Uuid,
    name: String,
    color: String,
    position: i32,
}

#[derive(Deserialize)]
struct LabelReq {
    name: String,
    color: String,
    #[serde(default)]
    position: Option<i32>,
}

async fn list_labels(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<Label>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = sqlx::query!("SELECT id, name, color, position FROM user_labels WHERE user_id = $1 ORDER BY position, created_at", uid)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(|r| Label { id: r.id, name: r.name, color: r.color, position: r.position }).collect()))
}

async fn create_label(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<LabelReq>) -> AppResult<Json<Label>> {
    let uid = auth_user(&state, &headers).await?;
    let name = p.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 32 || !hex_color(&p.color) {
        return Err(AppError::BadRequest("invalid_label"));
    }
    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM user_labels WHERE user_id = $1"#, uid).fetch_one(&state.db).await?;
    if count >= 30 {
        return Err(AppError::BadRequest("label_limit_reached"));
    }
    let r = sqlx::query!(
        "INSERT INTO user_labels (user_id, name, color, position) VALUES ($1, $2, $3, $4) RETURNING id, name, color, position",
        uid,
        name,
        p.color,
        p.position.unwrap_or(count as i32)
    )
    .fetch_one(&state.db)
    .await?;
    notify_prefs(&state, uid).await;
    Ok(Json(Label { id: r.id, name: r.name, color: r.color, position: r.position }))
}

async fn update_label(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(p): Json<LabelReq>) -> AppResult<Json<Label>> {
    let uid = auth_user(&state, &headers).await?;
    let name = p.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 32 || !hex_color(&p.color) {
        return Err(AppError::BadRequest("invalid_label"));
    }
    let r = sqlx::query!(
        "UPDATE user_labels SET name = $1, color = $2, position = COALESCE($3, position) WHERE id = $4 AND user_id = $5 RETURNING id, name, color, position",
        name,
        p.color,
        p.position,
        id,
        uid
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("label_not_found"))?;
    notify_prefs(&state, uid).await;
    Ok(Json(Label { id: r.id, name: r.name, color: r.color, position: r.position }))
}

async fn delete_label(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM user_labels WHERE id = $1 AND user_id = $2", id, uid).execute(&mut *tx).await?;
    sqlx::query!("UPDATE conversation_prefs SET label_ids = array_remove(label_ids, $1) WHERE user_id = $2", id, uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    notify_prefs(&state, uid).await;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Serialize)]
struct Pref {
    conv_type: String,
    conv_id: Uuid,
    pinned_at: Option<chrono::DateTime<chrono::Utc>>,
    label_ids: Vec<Uuid>,
    in_messages: bool,
    in_communities: bool,
    muted: bool,
}

async fn list_prefs(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<Pref>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        "SELECT conv_type, conv_id, pinned_at, label_ids, in_messages, in_communities, muted FROM conversation_prefs WHERE user_id = $1",
        uid
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Pref {
                conv_type: r.conv_type,
                conv_id: r.conv_id,
                pinned_at: r.pinned_at,
                label_ids: r.label_ids,
                in_messages: r.in_messages,
                in_communities: r.in_communities,
                muted: r.muted,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct PrefReq {
    pinned: Option<bool>,
    label_ids: Option<Vec<Uuid>>,
    in_messages: Option<bool>,
    in_communities: Option<bool>,
    muted: Option<bool>,
}

async fn update_pref(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conv_type, conv_id)): Path<(String, Uuid)>,
    Json(p): Json<PrefReq>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    if !["dm", "group", "channel", "self", "bot"].contains(&conv_type.as_str()) {
        return Err(AppError::BadRequest("invalid_conversation_type"));
    }
    let deduped_labels: Option<Vec<Uuid>> = p.label_ids.as_ref().map(|ids| {
        let mut uniq = Vec::with_capacity(ids.len());
        for id in ids {
            if !uniq.contains(id) {
                uniq.push(*id);
            }
        }
        uniq
    });
    if let Some(ids) = &deduped_labels {
        if ids.len() > 10 {
            return Err(AppError::BadRequest("too_many_labels"));
        }
        // Only the caller's own labels can be attached.
        let owned = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM user_labels WHERE user_id = $1 AND id = ANY($2)"#, uid, ids)
            .fetch_one(&state.db)
            .await?;
        if owned as usize != ids.len() {
            return Err(AppError::BadRequest("unknown_label"));
        }
    }
    let pinned_at = p.pinned.map(|on| if on { Some(chrono::Utc::now()) } else { None });
    sqlx::query!(
        r#"INSERT INTO conversation_prefs (user_id, conv_type, conv_id, pinned_at, label_ids, in_messages, in_communities, muted)
           VALUES ($1, $2, $3, $4, COALESCE($5, '{}'::uuid[]), COALESCE($6, FALSE), COALESCE($7, FALSE), COALESCE($8, FALSE))
           ON CONFLICT (user_id, conv_type, conv_id) DO UPDATE SET
             pinned_at = CASE WHEN $9 THEN EXCLUDED.pinned_at ELSE conversation_prefs.pinned_at END,
             label_ids = COALESCE($5, conversation_prefs.label_ids),
             in_messages = COALESCE($6, conversation_prefs.in_messages),
             in_communities = COALESCE($7, conversation_prefs.in_communities),
             muted = COALESCE($8, conversation_prefs.muted),
             updated_at = now()"#,
        uid,
        conv_type,
        conv_id,
        pinned_at.flatten(),
        deduped_labels.as_deref(),
        p.in_messages,
        p.in_communities,
        p.muted,
        p.pinned.is_some()
    )
    .execute(&state.db)
    .await?;
    notify_prefs(&state, uid).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn notify_prefs(state: &AppState, uid: Uuid) {
    state.ws_hub.send_to_user(&uid, WsEvent::PrefsUpdated(serde_json::json!({}))).await;
}

#[derive(Deserialize)]
struct ReadReq {
    partner_id: Uuid,
    seq: i64,
}

/// Record that the caller read DMs from `partner_id` up to `seq`.
async fn mark_read(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<ReadReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    if p.partner_id == uid || !are_friends(&state.db, uid, p.partner_id).await? {
        return Err(AppError::Forbidden("not_friends"));
    }
    sqlx::query!(
        r#"INSERT INTO dm_reads (reader_id, partner_id, last_read_seq) VALUES ($1, $2, $3)
           ON CONFLICT (reader_id, partner_id) DO UPDATE SET last_read_seq = GREATEST(dm_reads.last_read_seq, EXCLUDED.last_read_seq), updated_at = now()"#,
        uid,
        p.partner_id,
        p.seq
    )
    .execute(&state.db)
    .await?;
    let shares = sqlx::query_scalar!("SELECT privacy_read_receipts FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    if shares {
        state
            .ws_hub
            .send_to_user(&p.partner_id, WsEvent::MessageRead(serde_json::json!({ "reader_id": uid, "seq": p.seq })))
            .await;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// How far `partner_id` has read the caller's messages (hidden if the partner turned receipts off).
async fn get_partner_read(State(state): State<AppState>, headers: HeaderMap, Path(partner_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let row = sqlx::query!(
        r#"SELECT r.last_read_seq, u.privacy_read_receipts FROM dm_reads r JOIN users u ON u.id = r.reader_id
           WHERE r.reader_id = $1 AND r.partner_id = $2"#,
        partner_id,
        uid
    )
    .fetch_optional(&state.db)
    .await?;
    let seq = row.filter(|r| r.privacy_read_receipts).map(|r| r.last_read_seq);
    Ok(Json(serde_json::json!({ "seq": seq })))
}

/// Conversation key + participant ids for a shared wallpaper, after checking access.
async fn wallpaper_scope(state: &AppState, uid: Uuid, conv_type: &str, conv_id: Uuid) -> AppResult<(String, Vec<Uuid>)> {
    match conv_type {
        "dm" => {
            if !are_friends(&state.db, uid, conv_id).await? {
                return Err(AppError::Forbidden("not_friends"));
            }
            let (a, b) = (uid.min(conv_id), uid.max(conv_id));
            Ok((format!("dm:{a}:{b}"), vec![uid, conv_id]))
        }
        "group" => {
            let members: Vec<Uuid> = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", conv_id)
                .fetch_all(&state.db)
                .await?;
            if !members.contains(&uid) {
                return Err(AppError::Forbidden("not_a_group_member"));
            }
            Ok((format!("group:{conv_id}"), members))
        }
        _ => Err(AppError::BadRequest("invalid_conversation_type")),
    }
}

async fn get_wallpaper(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conv_type, conv_id)): Path<(String, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let (key, _) = wallpaper_scope(&state, uid, &conv_type, conv_id).await?;
    let v = sqlx::query_scalar!("SELECT value FROM shared_wallpapers WHERE conv_key = $1", key).fetch_optional(&state.db).await?;
    Ok(Json(serde_json::json!({ "value": v })))
}

#[derive(Deserialize)]
struct WallpaperReq {
    value: String,
}

fn valid_wallpaper(v: &str) -> bool {
    const PRESETS: [&str; 9] = ["none", "dots", "bubbles", "aurora", "paper", "sunset", "ocean", "mint", "night"];
    PRESETS.contains(&v)
        || ((v.starts_with("/uploads/media/") || v.starts_with("/wallpapers/"))
            && !v.contains("..")
            && v.len() <= 300
            && !v.chars().any(|c| matches!(c, '"' | '\'' | '(' | ')' | '\\' | '\n' | '\r' | '<' | '>')))
        || (v.starts_with("color:#") && v.len() == 13 && v[7..].chars().all(|c| c.is_ascii_hexdigit()))
}

async fn set_wallpaper(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conv_type, conv_id)): Path<(String, Uuid)>,
    Json(p): Json<WallpaperReq>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    if !valid_wallpaper(&p.value) {
        return Err(AppError::BadRequest("invalid_wallpaper"));
    }
    let (key, members) = wallpaper_scope(&state, uid, &conv_type, conv_id).await?;
    sqlx::query!(
        r#"INSERT INTO shared_wallpapers (conv_key, value, set_by) VALUES ($1, $2, $3)
           ON CONFLICT (conv_key) DO UPDATE SET value = EXCLUDED.value, set_by = EXCLUDED.set_by, updated_at = now()"#,
        key,
        p.value,
        uid
    )
    .execute(&state.db)
    .await?;
    state
        .ws_hub
        .send_to_users(
            &members,
            WsEvent::WallpaperUpdated(serde_json::json!({ "conv_type": conv_type, "conv_id": conv_id, "value": p.value, "set_by": uid })),
        )
        .await;
    Ok(Json(serde_json::json!({ "value": p.value })))
}

async fn enable_self_chat(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query!("UPDATE users SET self_chat = TRUE WHERE id = $1", uid).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "self_chat": true })))
}

/// Delete the self chat and every note in it (can be re-created later).
async fn delete_self_chat(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM messages WHERE sender_id = $1 AND receiver_id = $1", uid).execute(&mut *tx).await?;
    sqlx::query!("UPDATE users SET self_chat = FALSE WHERE id = $1", uid).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "self_chat": false })))
}
