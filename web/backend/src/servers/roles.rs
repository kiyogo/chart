//! Discord-style role management for community servers: edit and delete roles,
//! assign roles to members, kick members, and report the caller's effective
//! permissions. Every permission bit exposed here is enforced server-side.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AppState,
    enterprise::models::ent_perm,
    error::{AppError, AppResult},
    models::{ServerRoleItem, permissions as perm},
    privacy::auth_user,
    websocket::WsEvent,
};

/// (effective permission bits, effective enterprise bits, is_owner). Owners and administrators get every bit.
pub async fn effective_all(db: &sqlx::PgPool, server_id: Uuid, user_id: Uuid) -> AppResult<(i64, i64, bool)> {
    let owner = sqlx::query_scalar!("SELECT owner_id FROM servers WHERE id = $1", server_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound("server_not_found"))?;
    if owner == user_id {
        return Ok((perm::ALL, ent_perm::ALL, true));
    }
    let member = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM server_members WHERE server_id = $1 AND user_id = $2"#, server_id, user_id)
        .fetch_optional(db)
        .await?
        .is_some();
    if !member {
        return Err(AppError::Forbidden("not_a_member"));
    }
    let row = sqlx::query!(
        r#"SELECT COALESCE(BIT_OR(r.permissions), 0) AS "p!",
                  COALESCE(BIT_OR(r.enterprise_permissions), 0) AS "e!"
           FROM server_roles r
           WHERE r.server_id = $1 AND (r.name = '@everyone' OR r.id IN (SELECT role_id FROM server_member_roles WHERE server_id = $1 AND user_id = $2))"#,
        server_id,
        user_id
    )
    .fetch_one(db)
    .await?;
    if row.p & perm::ADMINISTRATOR != 0 {
        Ok((perm::ALL, ent_perm::ALL, false))
    } else {
        Ok((row.p, row.e, false))
    }
}

/// (effective permission bits, is_owner). Owners and administrators get every bit.
pub async fn effective(db: &sqlx::PgPool, server_id: Uuid, user_id: Uuid) -> AppResult<(i64, bool)> {
    let (bits, _, owner) = effective_all(db, server_id, user_id).await?;
    Ok((bits, owner))
}

async fn require(db: &sqlx::PgPool, server_id: Uuid, user_id: Uuid, bit: i64) -> AppResult<(i64, bool)> {
    let (bits, owner) = effective(db, server_id, user_id).await?;
    if bits & bit == 0 {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    Ok((bits, owner))
}

pub async fn my_permissions(State(state): State<AppState>, headers: HeaderMap, Path(server_id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let (bits, ent_bits, owner) = effective_all(&state.db, server_id, uid).await?;
    Ok(Json(serde_json::json!({
        "permissions": bits,
        "enterprise_permissions": ent_bits,
        "is_owner": owner
    })))
}

#[derive(Deserialize)]
pub struct UpdateRole {
    name: Option<String>,
    color: Option<String>,
    permissions: Option<i64>,
    enterprise_permissions: Option<i64>,
    position: Option<i32>,
}

fn hex_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit())
}

pub async fn update_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, role_id)): Path<(Uuid, Uuid)>,
    Json(p): Json<UpdateRole>,
) -> AppResult<Json<ServerRoleItem>> {
    let uid = auth_user(&state, &headers).await?;
    let (mine, my_ent, _) = effective_all(&state.db, server_id, uid).await?;
    if mine & perm::MANAGE_ROLES == 0 {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let role = sqlx::query!(
        "SELECT name, permissions, enterprise_permissions FROM server_roles WHERE id = $1 AND server_id = $2",
        role_id,
        server_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("role_not_found"))?;
    if let Some(n) = &p.name {
        let n = n.trim();
        if role.name == "@everyone" && n != "@everyone" {
            return Err(AppError::BadRequest("cannot_rename_everyone"));
        }
        if n.is_empty() || n.chars().count() > 32 {
            return Err(AppError::BadRequest("invalid_role_name"));
        }
    }
    if let Some(c) = &p.color {
        if !hex_color(c) {
            return Err(AppError::BadRequest("invalid_color"));
        }
    }
    if let Some(bits) = p.permissions {
        if bits < 0 || bits & !perm::ALL != 0 {
            return Err(AppError::BadRequest("invalid_permissions"));
        }
        // You cannot grant (or remove) bits you do not hold yourself.
        if (bits ^ role.permissions) & !mine != 0 {
            return Err(AppError::Forbidden("cannot_grant_missing_permissions"));
        }
    }
    if let Some(ebits) = p.enterprise_permissions {
        if ebits < 0 || ebits & !ent_perm::ALL != 0 {
            return Err(AppError::BadRequest("invalid_enterprise_permissions"));
        }
        if (ebits ^ role.enterprise_permissions) & !my_ent != 0 {
            return Err(AppError::Forbidden("cannot_grant_missing_enterprise_permissions"));
        }
    }
    let r = sqlx::query!(
        r#"UPDATE server_roles SET name = COALESCE($3, name), color = COALESCE($4, color),
             permissions = COALESCE($5, permissions),
             enterprise_permissions = COALESCE($6, enterprise_permissions),
             position = COALESCE($7, position)
           WHERE id = $1 AND server_id = $2
           RETURNING id, server_id, name, color, position, permissions, enterprise_permissions, created_at"#,
        role_id,
        server_id,
        p.name.as_deref().map(str::trim),
        p.color,
        p.permissions,
        p.enterprise_permissions,
        p.position
    )
    .fetch_one(&state.db)
    .await?;
    crate::community::audit(&state, server_id, uid, "role_updated", &r.name).await;
    Ok(Json(ServerRoleItem {
        id: r.id,
        server_id: r.server_id,
        name: r.name,
        color: r.color,
        position: r.position,
        permissions: r.permissions,
        enterprise_permissions: r.enterprise_permissions,
        created_at: r.created_at,
    }))
}

pub async fn delete_role(State(state): State<AppState>, headers: HeaderMap, Path((server_id, role_id)): Path<(Uuid, Uuid)>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let (mine, _) = require(&state.db, server_id, uid, perm::MANAGE_ROLES).await?;
    let role = sqlx::query!("SELECT name, permissions FROM server_roles WHERE id = $1 AND server_id = $2", role_id, server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("role_not_found"))?;
    if role.name == "@everyone" {
        return Err(AppError::BadRequest("cannot_delete_everyone"));
    }
    if role.permissions & !mine != 0 {
        return Err(AppError::Forbidden("cannot_grant_missing_permissions"));
    }
    sqlx::query!("DELETE FROM server_roles WHERE id = $1 AND server_id = $2", role_id, server_id).execute(&state.db).await?;
    crate::community::audit(&state, server_id, uid, "role_deleted", &role_id.to_string()).await;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn assign_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, user_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    set_member_role(state, headers, server_id, user_id, role_id, true).await
}

pub async fn unassign_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, user_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    set_member_role(state, headers, server_id, user_id, role_id, false).await
}

async fn set_member_role(state: AppState, headers: HeaderMap, server_id: Uuid, user_id: Uuid, role_id: Uuid, on: bool) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let (mine, _) = require(&state.db, server_id, uid, perm::MANAGE_ROLES).await?;
    let role = sqlx::query!("SELECT name, permissions FROM server_roles WHERE id = $1 AND server_id = $2", role_id, server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("role_not_found"))?;
    if role.name == "@everyone" {
        return Err(AppError::BadRequest("everyone_is_implicit"));
    }
    if role.permissions & !mine != 0 {
        return Err(AppError::Forbidden("cannot_grant_missing_permissions"));
    }
    let member = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM server_members WHERE server_id = $1 AND user_id = $2"#, server_id, user_id)
        .fetch_optional(&state.db)
        .await?
        .is_some();
    if !member {
        return Err(AppError::NotFound("member_not_found"));
    }
    if on {
        sqlx::query!("INSERT INTO server_member_roles (server_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING", server_id, user_id, role_id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query!("DELETE FROM server_member_roles WHERE server_id = $1 AND user_id = $2 AND role_id = $3", server_id, user_id, role_id)
            .execute(&state.db)
            .await?;
    }
    crate::community::audit(&state, server_id, uid, if on { "role_assigned" } else { "role_removed" }, &user_id.to_string()).await;
    Ok(Json(serde_json::json!({ "assigned": on })))
}

pub async fn kick_member(State(state): State<AppState>, headers: HeaderMap, Path((server_id, user_id)): Path<(Uuid, Uuid)>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    require(&state.db, server_id, uid, perm::KICK_MEMBERS).await?;
    if uid == user_id {
        return Err(AppError::BadRequest("cannot_kick_self"));
    }
    let owner = sqlx::query_scalar!("SELECT owner_id FROM servers WHERE id = $1", server_id).fetch_one(&state.db).await?;
    if owner == user_id {
        return Err(AppError::Forbidden("cannot_kick_owner"));
    }
    // Members holding administrator can only be removed by the owner.
    let (target_bits, _) = effective(&state.db, server_id, user_id).await?;
    if target_bits == perm::ALL && owner != uid {
        return Err(AppError::Forbidden("cannot_kick_administrator"));
    }
    sqlx::query!("DELETE FROM server_member_roles WHERE server_id = $1 AND user_id = $2", server_id, user_id).execute(&state.db).await?;
    sqlx::query!("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2", server_id, user_id).execute(&state.db).await?;
    state
        .ws_hub
        .send_to_user(&user_id, WsEvent::GroupSettings(serde_json::json!({ "server_id": server_id, "removed": true })))
        .await;
    crate::community::audit(&state, server_id, uid, "member_kicked", &user_id.to_string()).await;
    Ok(Json(serde_json::json!({ "kicked": true })))
}
