//! Group management: member permissions, join approval, invite links, group blocking.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::get_member_role;
use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::auth_user,
    websocket::WsEvent,
};

#[derive(Serialize, Clone)]
pub struct GroupSettings {
    pub perm_send: bool,
    pub perm_poll: bool,
    pub perm_note: bool,
    pub perm_reminder: bool,
    pub perm_pin: bool,
    pub perm_rename: bool,
    pub perm_avatar: bool,
    pub require_approval: bool,
    pub highlight_admins: bool,
    pub allow_link_join: bool,
    pub is_public: bool,
    pub verification_status: String,
    pub banner_url: Option<String>,
    pub my_role: Option<String>,
    pub pending_requests: i64,
}

pub async fn load_settings(db: &PgPool, group_id: Uuid) -> AppResult<GroupSettings> {
    let r = sqlx::query!(
        r#"SELECT perm_send, perm_poll, perm_note, perm_reminder, perm_pin, perm_rename, perm_avatar,
                  require_approval, highlight_admins, allow_link_join, is_public, verification_status, banner_url,
                  (SELECT COUNT(*) FROM group_join_requests WHERE group_id = g.id) AS "pending!"
           FROM groups g WHERE id = $1"#,
        group_id
    )
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound("group_not_found"))?;
    Ok(GroupSettings {
        perm_send: r.perm_send,
        perm_poll: r.perm_poll,
        perm_note: r.perm_note,
        perm_reminder: r.perm_reminder,
        perm_pin: r.perm_pin,
        perm_rename: r.perm_rename,
        perm_avatar: r.perm_avatar,
        require_approval: r.require_approval,
        highlight_admins: r.highlight_admins,
        allow_link_join: r.allow_link_join,
        is_public: r.is_public,
        verification_status: r.verification_status,
        banner_url: r.banner_url,
        my_role: None,
        pending_requests: r.pending,
    })
}

pub fn is_admin_role(role: &str) -> bool {
    role == "owner" || role == "admin"
}

pub async fn get_settings(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<GroupSettings>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    let mut s = load_settings(&state.db, group_id).await?;
    if !is_admin_role(&role) {
        s.pending_requests = 0;
    }
    s.my_role = Some(role);
    Ok(Json(s))
}

#[derive(Deserialize)]
pub struct UpdateSettings {
    perm_send: Option<bool>,
    perm_poll: Option<bool>,
    perm_note: Option<bool>,
    perm_reminder: Option<bool>,
    perm_pin: Option<bool>,
    perm_rename: Option<bool>,
    perm_avatar: Option<bool>,
    require_approval: Option<bool>,
    highlight_admins: Option<bool>,
    allow_link_join: Option<bool>,
    is_public: Option<bool>,
    banner_url: Option<String>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(p): Json<UpdateSettings>,
) -> AppResult<Json<GroupSettings>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    if p.is_public.is_some() && role != "owner" {
        return Err(AppError::Forbidden("owner_only"));
    }
    if let Some(ref b) = p.banner_url {
        if !b.is_empty() && (!b.starts_with("/uploads/media/") || b.contains("..")) {
            return Err(AppError::BadRequest("invalid_banner_url"));
        }
    }
    sqlx::query!(
        r#"UPDATE groups SET
            perm_send = COALESCE($1, perm_send), perm_poll = COALESCE($2, perm_poll), perm_note = COALESCE($3, perm_note),
            perm_reminder = COALESCE($4, perm_reminder), perm_pin = COALESCE($5, perm_pin), perm_rename = COALESCE($6, perm_rename),
            perm_avatar = COALESCE($7, perm_avatar), require_approval = COALESCE($8, require_approval),
            highlight_admins = COALESCE($9, highlight_admins), allow_link_join = COALESCE($10, allow_link_join),
            is_public = COALESCE($11, is_public),
            verification_status = CASE
                WHEN $11 IS TRUE AND verification_status IN ('none', 'rejected') THEN 'pending'
                WHEN $11 IS FALSE THEN 'none'
                ELSE verification_status END,
            banner_url = CASE WHEN $12::text IS NULL THEN banner_url WHEN $12 = '' THEN NULL ELSE $12 END
        WHERE id = $13"#,
        p.perm_send,
        p.perm_poll,
        p.perm_note,
        p.perm_reminder,
        p.perm_pin,
        p.perm_rename,
        p.perm_avatar,
        p.require_approval,
        p.highlight_admins,
        p.allow_link_join,
        p.is_public,
        p.banner_url,
        group_id
    )
    .execute(&state.db)
    .await?;
    let mut s = load_settings(&state.db, group_id).await?;
    let members: Vec<Uuid> = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
        .fetch_all(&state.db)
        .await?;
    state
        .ws_hub
        .send_to_users(&members, WsEvent::GroupSettings(serde_json::json!({ "group_id": group_id })))
        .await;
    s.my_role = Some(role);
    Ok(Json(s))
}

#[derive(Serialize)]
pub struct JoinRequest {
    user_id: Uuid,
    username: String,
    avatar_url: Option<String>,
    note: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_join_requests(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<Vec<JoinRequest>>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let rows = sqlx::query!(
        r#"SELECT r.user_id, u.username AS "username!", u.avatar_url, r.note, r.created_at
           FROM group_join_requests r JOIN users u ON u.id = r.user_id
           WHERE r.group_id = $1 ORDER BY r.created_at"#,
        group_id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| JoinRequest { user_id: r.user_id, username: r.username, avatar_url: r.avatar_url, note: r.note, created_at: r.created_at })
            .collect(),
    ))
}

pub async fn decide_join_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, user_id, decision)): Path<(Uuid, Uuid, String)>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let mut tx = state.db.begin().await?;
    let removed = sqlx::query!("DELETE FROM group_join_requests WHERE group_id = $1 AND user_id = $2", group_id, user_id)
        .execute(&mut *tx)
        .await?;
    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound("join_request_not_found"));
    }
    let approved = decision == "approve";
    if approved {
        sqlx::query!(
            "INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
            group_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;
    } else if decision != "reject" {
        return Err(AppError::BadRequest("invalid_decision"));
    }
    tx.commit().await?;
    if approved {
        let members: Vec<Uuid> = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
            .fetch_all(&state.db)
            .await?;
        state
            .ws_hub
            .send_to_users(&members, WsEvent::GroupSettings(serde_json::json!({ "group_id": group_id, "member_added": user_id })))
            .await;
    }
    Ok(Json(serde_json::json!({ "approved": approved })))
}

/// Leave the group and refuse future additions to it.
pub async fn block_group(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let owner = sqlx::query_scalar!("SELECT owner_id FROM groups WHERE id = $1", group_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("group_not_found"))?;
    if owner == uid {
        return Err(AppError::BadRequest("owner_cannot_block_own_group"));
    }
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2", group_id, uid).execute(&mut *tx).await?;
    sqlx::query!("DELETE FROM group_join_requests WHERE group_id = $1 AND user_id = $2", group_id, uid).execute(&mut *tx).await?;
    sqlx::query!("INSERT INTO group_blocks (user_id, group_id) VALUES ($1, $2) ON CONFLICT DO NOTHING", uid, group_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "blocked": true })))
}

pub async fn unblock_group(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query!("DELETE FROM group_blocks WHERE user_id = $1 AND group_id = $2", uid, group_id).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "blocked": false })))
}

async fn active_link(db: &PgPool, group_id: Uuid) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT code FROM group_invites WHERE group_id = $1 AND max_uses IS NULL AND expires_at IS NULL ORDER BY created_at DESC LIMIT 1",
        group_id
    )
    .fetch_optional(db)
    .await?)
}

fn new_code() -> String {
    Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// The group's reusable invite link (created on first request).
pub async fn get_invite_link(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    let settings = load_settings(&state.db, group_id).await?;
    if !settings.allow_link_join && !is_admin_role(&role) {
        return Err(AppError::Forbidden("link_join_disabled"));
    }
    let code = match active_link(&state.db, group_id).await? {
        Some(c) => c,
        None => {
            let c = new_code();
            sqlx::query!("INSERT INTO group_invites (code, group_id, inviter_id) VALUES ($1, $2, $3)", c, group_id, uid)
                .execute(&state.db)
                .await?;
            c
        }
    };
    Ok(Json(serde_json::json!({ "code": code, "enabled": settings.allow_link_join })))
}

pub async fn regenerate_invite_link(State(state): State<AppState>, headers: HeaderMap, Path(group_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM group_invites WHERE group_id = $1 AND max_uses IS NULL AND expires_at IS NULL", group_id)
        .execute(&mut *tx)
        .await?;
    let c = new_code();
    sqlx::query!("INSERT INTO group_invites (code, group_id, inviter_id) VALUES ($1, $2, $3)", c, group_id, uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "code": c })))
}

/// Public card for an invite (shown before joining).
pub async fn invite_preview(State(state): State<AppState>, headers: HeaderMap, Path(code): Path<String>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let r = sqlx::query!(
        r#"SELECT g.id, g.name, g.description, g.avatar_url, g.banner_url, g.require_approval, g.allow_link_join,
                  (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) AS "members!",
                  EXISTS (SELECT 1 FROM group_members WHERE group_id = g.id AND user_id = $2) AS "is_member!"
           FROM group_invites i JOIN groups g ON g.id = i.group_id
           WHERE i.code = $1 AND (i.expires_at IS NULL OR i.expires_at > now()) AND (i.max_uses IS NULL OR i.uses < i.max_uses)"#,
        code,
        uid
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("invite_not_found"))?;
    if !r.allow_link_join {
        return Err(AppError::Forbidden("link_join_disabled"));
    }
    Ok(Json(serde_json::json!({
        "kind": "group", "id": r.id, "name": r.name, "description": r.description, "avatar_url": r.avatar_url,
        "banner_url": r.banner_url, "member_count": r.members, "require_approval": r.require_approval, "is_member": r.is_member,
    })))
}

/// Permission checks for members (owners/admins are never restricted).
pub async fn check_member_may(db: &PgPool, group_id: Uuid, role: &str, action: &str) -> AppResult<()> {
    if is_admin_role(role) {
        return Ok(());
    }
    let s = load_settings(db, group_id).await?;
    let allowed = match action {
        "send" => s.perm_send,
        "poll" => s.perm_poll,
        "note" => s.perm_note,
        "reminder" => s.perm_reminder,
        "pin" => s.perm_pin,
        "rename" => s.perm_rename,
        "avatar" => s.perm_avatar,
        _ => false,
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden("not_allowed_by_group_settings")) }
}
