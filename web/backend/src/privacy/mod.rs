//! Privacy settings and user blocking, plus helpers used by other modules to
//! enforce them server-side (messages, friend requests, calls, group invites).

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(get_settings).put(update_settings))
        .route("/blocks", get(list_blocks).post(block_user))
        .route("/blocks/{user_id}", delete(unblock_user))
}

pub async fn auth_user(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let token = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    Ok(authenticate_session(&state.db, &state.config, token).await?.id)
}

/// True when either user blocked the other.
pub async fn is_blocked_either(db: &PgPool, a: Uuid, b: Uuid) -> AppResult<bool> {
    let hit = sqlx::query_scalar!(
        r#"SELECT 1 AS "one!" FROM user_blocks WHERE (blocker_id = $1 AND blocked_id = $2) OR (blocker_id = $2 AND blocked_id = $1) LIMIT 1"#,
        a,
        b
    )
    .fetch_optional(db)
    .await?;
    Ok(hit.is_some())
}

pub async fn are_friends(db: &PgPool, a: Uuid, b: Uuid) -> AppResult<bool> {
    let (ua, ub) = (a.min(b), a.max(b));
    let hit = sqlx::query_scalar!(
        r#"SELECT 1 AS "one!" FROM friendships WHERE user_a = $1 AND user_b = $2"#,
        ua,
        ub
    )
    .fetch_optional(db)
    .await?;
    Ok(hit.is_some())
}

/// Can `adder` put `target` into a group? Respects blocks, group blocks and the
/// target's "who can add me to groups" setting.
pub async fn can_add_to_group(db: &PgPool, adder: Uuid, target: Uuid, group_id: Option<Uuid>) -> AppResult<bool> {
    if is_blocked_either(db, adder, target).await? {
        return Ok(false);
    }
    if let Some(gid) = group_id {
        let blocked = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM group_blocks WHERE user_id = $1 AND group_id = $2"#, target, gid)
            .fetch_optional(db)
            .await?;
        if blocked.is_some() {
            return Ok(false);
        }
    }
    let pref = sqlx::query_scalar!("SELECT privacy_group_invites FROM users WHERE id = $1", target)
        .fetch_optional(db)
        .await?;
    Ok(match pref.as_deref() {
        Some("everyone") => true,
        Some("friends") => are_friends(db, adder, target).await?,
        _ => false,
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PrivacySettings {
    pub friend_requests: String,
    pub calls: String,
    pub group_invites: String,
    pub discoverable: bool,
    pub read_receipts: bool,
    pub typing: bool,
    pub self_chat: bool,
}

pub async fn load_settings(db: &PgPool, user_id: Uuid) -> AppResult<PrivacySettings> {
    let r = sqlx::query!(
        "SELECT privacy_friend_requests, privacy_calls, privacy_group_invites, privacy_discoverable, privacy_read_receipts, privacy_typing, self_chat FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(db)
    .await?;
    Ok(PrivacySettings {
        friend_requests: r.privacy_friend_requests,
        calls: r.privacy_calls,
        group_invites: r.privacy_group_invites,
        discoverable: r.privacy_discoverable,
        read_receipts: r.privacy_read_receipts,
        typing: r.privacy_typing,
        self_chat: r.self_chat,
    })
}

async fn get_settings(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<PrivacySettings>> {
    let uid = auth_user(&state, &headers).await?;
    Ok(Json(load_settings(&state.db, uid).await?))
}

#[derive(Deserialize)]
struct UpdateSettings {
    friend_requests: Option<String>,
    calls: Option<String>,
    group_invites: Option<String>,
    discoverable: Option<bool>,
    read_receipts: Option<bool>,
    typing: Option<bool>,
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(p): Json<UpdateSettings>,
) -> AppResult<Json<PrivacySettings>> {
    let uid = auth_user(&state, &headers).await?;
    let check = |v: &Option<String>, allowed: &[&str], code: &'static str| -> AppResult<()> {
        match v {
            Some(x) if !allowed.contains(&x.as_str()) => Err(AppError::BadRequest(code)),
            _ => Ok(()),
        }
    };
    check(&p.friend_requests, &["everyone", "nobody"], "invalid_friend_requests")?;
    check(&p.calls, &["friends", "nobody"], "invalid_calls")?;
    check(&p.group_invites, &["everyone", "friends", "nobody"], "invalid_group_invites")?;
    sqlx::query!(
        r#"UPDATE users SET
            privacy_friend_requests = COALESCE($1, privacy_friend_requests),
            privacy_calls = COALESCE($2, privacy_calls),
            privacy_group_invites = COALESCE($3, privacy_group_invites),
            privacy_discoverable = COALESCE($4, privacy_discoverable),
            privacy_read_receipts = COALESCE($5, privacy_read_receipts),
            privacy_typing = COALESCE($6, privacy_typing)
        WHERE id = $7"#,
        p.friend_requests,
        p.calls,
        p.group_invites,
        p.discoverable,
        p.read_receipts,
        p.typing,
        uid
    )
    .execute(&state.db)
    .await?;
    Ok(Json(load_settings(&state.db, uid).await?))
}

#[derive(Serialize)]
struct BlockedUser {
    id: Uuid,
    username: String,
    avatar_url: Option<String>,
    blocked_at: chrono::DateTime<chrono::Utc>,
}

async fn list_blocks(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<BlockedUser>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        r#"SELECT u.id, u.username AS "username!", u.avatar_url, b.created_at
           FROM user_blocks b JOIN users u ON u.id = b.blocked_id
           WHERE b.blocker_id = $1 ORDER BY b.created_at DESC"#,
        uid
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| BlockedUser { id: r.id, username: r.username, avatar_url: r.avatar_url, blocked_at: r.created_at })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct BlockReq {
    user_id: Uuid,
}

async fn block_user(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<BlockReq>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    if p.user_id == uid {
        return Err(AppError::BadRequest("cannot_block_self"));
    }
    let exists = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM users WHERE id = $1"#, p.user_id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound("user_not_found"));
    }
    sqlx::query!(
        "INSERT INTO user_blocks (blocker_id, blocked_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        uid,
        p.user_id
    )
    .execute(&state.db)
    .await?;
    // Pending friend requests between the two are dropped.
    let _ = sqlx::query!(
        "DELETE FROM friend_requests WHERE status = 'pending' AND ((sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1))",
        uid,
        p.user_id
    )
    .execute(&state.db)
    .await;
    let _ = crate::audit::log_audit_event(&state.db, uid, "user_blocked", Some("user"), Some(&p.user_id.to_string()), None, None, None).await;
    Ok(Json(serde_json::json!({ "blocked": true })))
}

async fn unblock_user(State(state): State<AppState>, headers: HeaderMap, Path(user_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query!("DELETE FROM user_blocks WHERE blocker_id = $1 AND blocked_id = $2", uid, user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "blocked": false })))
}
