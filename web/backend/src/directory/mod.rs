//! Public directory and global search: verified public communities, groups,
//! bots and business pages, plus joining a public group without an invite.

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
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/groups/{id}/join", post(join_public_group))
}

#[derive(Deserialize)]
struct SearchQ {
    #[serde(default)]
    q: String,
}

async fn search(State(state): State<AppState>, headers: HeaderMap, Query(p): Query<SearchQ>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let q = p.q.trim().to_lowercase();
    if q.chars().count() > 64 {
        return Err(AppError::BadRequest("query_too_long"));
    }
    let like = format!("%{}%", q.replace('%', "").replace('_', ""));
    let servers = sqlx::query!(
        r#"SELECT s.id, s.name, s.description, s.icon_url, s.banner_url,
                  (SELECT COUNT(*) FROM server_members WHERE server_id = s.id) AS "members!",
                  EXISTS (SELECT 1 FROM server_members WHERE server_id = s.id AND user_id = $2) AS "joined!"
           FROM servers s WHERE s.is_public AND s.verification_status = 'verified' AND (lower(s.name) LIKE $1 OR lower(COALESCE(s.description, '')) LIKE $1)
           ORDER BY 6 DESC LIMIT 20"#,
        like,
        uid
    )
    .fetch_all(&state.db)
    .await?;
    let groups = sqlx::query!(
        r#"SELECT g.id, g.name, g.description, g.avatar_url, g.banner_url, g.require_approval,
                  (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) AS "members!",
                  EXISTS (SELECT 1 FROM group_members WHERE group_id = g.id AND user_id = $2) AS "joined!"
           FROM groups g WHERE g.is_public AND g.verification_status = 'verified' AND (lower(g.name) LIKE $1 OR lower(COALESCE(g.description, '')) LIKE $1)
           ORDER BY 7 DESC LIMIT 20"#,
        like,
        uid
    )
    .fetch_all(&state.db)
    .await?;
    let bots = sqlx::query!(
        r#"SELECT id, username, display_name, description, avatar_url FROM bots
           WHERE is_public AND verification_status = 'verified' AND (username_norm LIKE $1 OR lower(display_name) LIKE $1)
           ORDER BY created_at DESC LIMIT 20"#,
        like
    )
    .fetch_all(&state.db)
    .await?;
    let businesses = sqlx::query!(
        r#"SELECT b.handle, b.name, b.category, b.description, b.cover_url, b.verification_status, u.username AS "username!", u.avatar_url
           FROM business_profiles b JOIN users u ON u.id = b.user_id
           WHERE b.enabled AND u.suspended_at IS NULL AND b.verification_status = 'verified'
             AND (b.handle LIKE $1 OR lower(b.name) LIKE $1 OR lower(COALESCE(b.category, '')) LIKE $1)
           ORDER BY b.updated_at DESC LIMIT 20"#,
        like
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({
        "servers": servers.into_iter().map(|r| serde_json::json!({"id": r.id, "name": r.name, "description": r.description, "avatar_url": r.icon_url, "banner_url": r.banner_url, "member_count": r.members, "joined": r.joined})).collect::<Vec<_>>(),
        "groups": groups.into_iter().map(|r| serde_json::json!({"id": r.id, "name": r.name, "description": r.description, "avatar_url": r.avatar_url, "banner_url": r.banner_url, "member_count": r.members, "joined": r.joined, "require_approval": r.require_approval})).collect::<Vec<_>>(),
        "bots": bots.into_iter().map(|r| serde_json::json!({"id": r.id, "username": r.username, "display_name": r.display_name, "description": r.description, "avatar_url": r.avatar_url})).collect::<Vec<_>>(),
        "businesses": businesses.into_iter().map(|r| serde_json::json!({"handle": r.handle, "name": r.name, "category": r.category, "description": r.description, "cover_url": r.cover_url, "username": r.username, "avatar_url": r.avatar_url, "verified": r.verification_status == "verified"})).collect::<Vec<_>>(),
    })))
}

/// Join a verified public group straight from the directory (approval rules still apply).
async fn join_public_group(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let g = sqlx::query!(
        "SELECT require_approval FROM groups WHERE id = $1 AND is_public AND verification_status = 'verified'",
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("group_not_found"))?;
    let blocked = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM group_blocks WHERE user_id = $1 AND group_id = $2"#, uid, id)
        .fetch_optional(&state.db)
        .await?;
    if blocked.is_some() {
        return Err(AppError::Forbidden("group_blocked"));
    }
    if g.require_approval {
        sqlx::query!("INSERT INTO group_join_requests (group_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING", id, uid)
            .execute(&state.db)
            .await?;
        let admins: Vec<Uuid> = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1 AND role IN ('owner', 'admin')", id)
            .fetch_all(&state.db)
            .await?;
        state.ws_hub.send_to_users(&admins, WsEvent::GroupJoinRequest(serde_json::json!({ "group_id": id, "user_id": uid }))).await;
        return Ok(Json(serde_json::json!({ "status": "pending" })));
    }
    sqlx::query!("INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING", id, uid)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "status": "joined" })))
}
