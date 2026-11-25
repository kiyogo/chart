//! Admin API (separate admin web at `/admin`): platform stats, verification queue
//! for public communities / groups / bots / business pages, account suspension
//! and the audit trail. Every handler requires `users.is_admin`.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::auth_user,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/stats", get(stats))
        .route("/verifications", get(list_verifications))
        .route("/verifications/{kind}/{id}/{decision}", post(decide))
        .route("/users", get(list_users))
        .route("/users/{id}/{action}", post(user_action))
        .route("/audit", get(audit))
}

async fn require_admin(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let uid = auth_user(state, headers).await?;
    let is_admin = sqlx::query_scalar!("SELECT is_admin FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    if !is_admin {
        return Err(AppError::Forbidden("admin_only"));
    }
    Ok(uid)
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let r = sqlx::query!("SELECT username, is_admin FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    Ok(Json(serde_json::json!({ "id": uid, "username": r.username, "is_admin": r.is_admin })))
}

async fn stats(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    require_admin(&state, &headers).await?;
    let r = sqlx::query!(
        r#"SELECT
            (SELECT COUNT(*) FROM users) AS "users!",
            (SELECT COUNT(*) FROM users WHERE suspended_at IS NOT NULL) AS "suspended!",
            (SELECT COUNT(*) FROM sessions WHERE expires_at > now()) AS "sessions!",
            (SELECT COUNT(*) FROM messages) AS "dm_messages!",
            (SELECT COUNT(*) FROM group_messages) AS "group_messages!",
            (SELECT COUNT(*) FROM groups) AS "groups!",
            (SELECT COUNT(*) FROM servers) AS "servers!",
            (SELECT COUNT(*) FROM bots) AS "bots!",
            (SELECT COUNT(*) FROM business_profiles) AS "businesses!",
            ((SELECT COUNT(*) FROM servers WHERE verification_status = 'pending')
             + (SELECT COUNT(*) FROM groups WHERE verification_status = 'pending')
             + (SELECT COUNT(*) FROM bots WHERE verification_status = 'pending')
             + (SELECT COUNT(*) FROM business_profiles WHERE verification_status = 'pending')) AS "pending!""#
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(serde_json::json!({
        "users": r.users, "suspended": r.suspended, "active_sessions": r.sessions, "dm_messages": r.dm_messages,
        "group_messages": r.group_messages, "groups": r.groups, "servers": r.servers, "bots": r.bots,
        "businesses": r.businesses, "pending_verifications": r.pending,
    })))
}

async fn list_verifications(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<serde_json::Value>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query!(
        r#"SELECT 'server' AS "kind!", s.id::text AS "id!", s.name AS "name!", s.description, u.username AS "owner?", s.created_at AS "created_at!"
             FROM servers s JOIN users u ON u.id = s.owner_id WHERE s.verification_status = 'pending'
           UNION ALL
           SELECT 'group', g.id::text, g.name, g.description, u.username, g.created_at
             FROM groups g JOIN users u ON u.id = g.owner_id WHERE g.verification_status = 'pending'
           UNION ALL
           SELECT 'bot', b.id::text, b.display_name || ' (@' || b.username || ')', b.description, u.username, b.created_at
             FROM bots b JOIN users u ON u.id = b.owner_id WHERE b.verification_status = 'pending'
           UNION ALL
           SELECT 'business', p.user_id::text, p.name || ' (/' || p.handle || ')', p.description, u.username, p.created_at
             FROM business_profiles p JOIN users u ON u.id = p.user_id WHERE p.verification_status = 'pending'
           ORDER BY 6"#
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| serde_json::json!({ "kind": r.kind, "id": r.id, "name": r.name, "description": r.description, "owner": r.owner, "created_at": r.created_at }))
            .collect(),
    ))
}

async fn decide(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((kind, id, decision)): Path<(String, Uuid, String)>,
) -> AppResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &headers).await?;
    let status = match decision.as_str() {
        "verify" => "verified",
        "reject" => "rejected",
        _ => return Err(AppError::BadRequest("invalid_decision")),
    };
    let affected = match kind.as_str() {
        "server" => sqlx::query!("UPDATE servers SET verification_status = $1 WHERE id = $2", status, id).execute(&state.db).await?,
        "group" => sqlx::query!("UPDATE groups SET verification_status = $1 WHERE id = $2", status, id).execute(&state.db).await?,
        "bot" => sqlx::query!("UPDATE bots SET verification_status = $1 WHERE id = $2", status, id).execute(&state.db).await?,
        "business" => sqlx::query!("UPDATE business_profiles SET verification_status = $1 WHERE user_id = $2", status, id).execute(&state.db).await?,
        _ => return Err(AppError::BadRequest("invalid_kind")),
    }
    .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound("item_not_found"));
    }
    let _ = crate::audit::log_audit_event(
        &state.db,
        admin,
        &format!("admin_{status}"),
        Some(&kind),
        Some(&id.to_string()),
        None,
        None,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "status": status })))
}

#[derive(Deserialize)]
struct UsersQ {
    #[serde(default)]
    q: String,
}

async fn list_users(State(state): State<AppState>, headers: HeaderMap, Query(p): Query<UsersQ>) -> AppResult<Json<Vec<serde_json::Value>>> {
    require_admin(&state, &headers).await?;
    let like = format!("{}%", p.q.trim().to_lowercase().replace('%', ""));
    let rows = sqlx::query!(
        r#"SELECT id, username, created_at, suspended_at, is_admin,
                  (SELECT COUNT(*) FROM sessions s WHERE s.user_id = u.id AND s.expires_at > now()) AS "sessions!"
           FROM users u WHERE COALESCE(username_norm, '') LIKE $1 ORDER BY created_at DESC LIMIT 50"#,
        like
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| serde_json::json!({ "id": r.id, "username": r.username, "created_at": r.created_at, "suspended_at": r.suspended_at, "is_admin": r.is_admin, "sessions": r.sessions }))
            .collect(),
    ))
}

async fn user_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, action)): Path<(Uuid, String)>,
) -> AppResult<Json<serde_json::Value>> {
    let admin = require_admin(&state, &headers).await?;
    if id == admin {
        return Err(AppError::BadRequest("cannot_moderate_self"));
    }
    let target_admin = sqlx::query_scalar!("SELECT is_admin FROM users WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user_not_found"))?;
    if target_admin {
        return Err(AppError::Forbidden("cannot_moderate_admin"));
    }
    match action.as_str() {
        "suspend" => {
            let mut tx = state.db.begin().await?;
            sqlx::query!("UPDATE users SET suspended_at = now() WHERE id = $1", id).execute(&mut *tx).await?;
            sqlx::query!("DELETE FROM sessions WHERE user_id = $1", id).execute(&mut *tx).await?;
            tx.commit().await?;
            state.ws_hub.disconnect_user(&id).await;
        }
        "unsuspend" => {
            sqlx::query!("UPDATE users SET suspended_at = NULL WHERE id = $1", id).execute(&state.db).await?;
        }
        _ => return Err(AppError::BadRequest("invalid_action")),
    }
    let _ = crate::audit::log_audit_event(&state.db, admin, &format!("admin_user_{action}"), Some("user"), Some(&id.to_string()), None, None, None).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn audit(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<serde_json::Value>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query!(
        r#"SELECT a.action, a.target_type, a.target_id, a.created_at, u.username
           FROM audit_logs a LEFT JOIN users u ON u.id = a.user_id ORDER BY a.created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| serde_json::json!({ "action": r.action, "target_type": r.target_type, "target_id": r.target_id, "created_at": r.created_at, "username": r.username }))
            .collect(),
    ))
}

/// Promote the usernames listed in `ADMIN_USERNAMES` (comma separated) at startup.
pub async fn bootstrap_admins(db: &sqlx::PgPool) {
    let Ok(list) = std::env::var("ADMIN_USERNAMES") else { return };
    let names: Vec<String> = list.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
    if names.is_empty() {
        return;
    }
    match sqlx::query!("UPDATE users SET is_admin = TRUE WHERE username_norm = ANY($1)", &names).execute(db).await {
        Ok(r) => tracing::info!(count = r.rows_affected(), "admin accounts promoted from ADMIN_USERNAMES"),
        Err(e) => tracing::warn!(error = %e, "could not promote admins"),
    }
}
