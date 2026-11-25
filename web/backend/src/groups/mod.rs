pub mod manage;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    models::{
        AddGroupMemberRequest, CreateGroupCategoryRequest, CreateGroupChannelRequest,
        CreateGroupInviteRequest, CreateGroupRequest, GroupCategoryItem, GroupChannelItem,
        GroupInviteItem, GroupItem, GroupMemberItem, GroupMessageItem, GroupReactionItem,
        PostGroupMessageRequest, ToggleReactionRequest, UpdateGroupMemberRoleRequest,
        UpdateGroupRequest,
    },
    websocket::WsEvent,
};

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub channel_id: Option<Uuid>,
    pub before_seq: Option<i64>,
    pub limit: Option<i64>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/invite-preview/{code}", get(manage::invite_preview))
        .route("/{group_id}/settings", get(manage::get_settings).put(manage::update_settings))
        .route("/{group_id}/join-requests", get(manage::list_join_requests))
        .route("/{group_id}/join-requests/{user_id}/{decision}", post(manage::decide_join_request))
        .route("/{group_id}/block", post(manage::block_group).delete(manage::unblock_group))
        .route("/{group_id}/invite-link", get(manage::get_invite_link))
        .route("/{group_id}/invite-link/regenerate", post(manage::regenerate_invite_link))
        .route("/{group_id}/avatar", post(upload_group_avatar).layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)))
        .route("/{group_id}/banner", post(upload_group_banner).layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)))
        .route("/", post(create_group).get(list_my_groups))
        .route("/join/{code}", post(join_by_invite))
        .route(
            "/{group_id}",
            get(get_group_details)
                .put(update_group)
                .delete(delete_group),
        )
        .route(
            "/{group_id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/{group_id}/members/{user_id}",
            delete(remove_member),
        )
        .route(
            "/{group_id}/members/{user_id}/role",
            put(update_member_role),
        )
        .route("/{group_id}/invites", post(create_invite))
        .route(
            "/{group_id}/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/{group_id}/categories/{category_id}",
            delete(delete_category),
        )
        .route(
            "/{group_id}/channels",
            get(list_channels).post(create_channel),
        )
        .route(
            "/{group_id}/channels/{channel_id}",
            delete(delete_channel),
        )
        .route(
            "/{group_id}/messages",
            get(list_messages).post(send_message),
        )
        .route(
            "/{group_id}/messages/{message_id}",
            put(edit_message).delete(delete_message),
        )
        .route(
            "/{group_id}/messages/{message_id}/pin",
            post(pin_message),
        )
        .route(
            "/{group_id}/messages/{message_id}/reactions",
            post(toggle_reaction),
        )
}

// Helper: check group membership and return user role
pub(crate) async fn get_member_role(
    db: &sqlx::PgPool,
    group_id: Uuid,
    user_id: Uuid,
) -> AppResult<Option<String>> {
    let row = sqlx::query!(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2",
        group_id,
        user_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| r.role))
}

// 1. Create Group
async fn create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateGroupRequest>,
) -> AppResult<Json<GroupItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("group_name_cannot_be_empty"));
    }

    let group_id = Uuid::new_v4();
    let mut tx = state.db.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO groups (id, name, description, avatar_url, banner_url, owner_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        group_id,
        payload.name.trim(),
        payload.description,
        payload.avatar_url,
        payload.banner_url,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    // Add owner as owner member
    sqlx::query!(
        r#"
        INSERT INTO group_members (group_id, user_id, role)
        VALUES ($1, $2, 'owner')
        "#,
        group_id,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    // Add additional members
    for mid in payload.member_ids {
        if mid != user.id && crate::privacy::can_add_to_group(&state.db, user.id, mid, None).await? {
            sqlx::query!(
                r#"
                INSERT INTO group_members (group_id, user_id, role)
                VALUES ($1, $2, 'member')
                ON CONFLICT DO NOTHING
                "#,
                group_id,
                mid
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Default category and general channel
    let cat_id = Uuid::new_v4();
    let chan_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO group_categories (id, group_id, name, position)
        VALUES ($1, $2, 'Text Channels', 0)
        "#,
        cat_id,
        group_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO group_channels (id, group_id, category_id, name, channel_type, position)
        VALUES ($1, $2, $3, 'general', 'text', 0)
        "#,
        chan_id,
        group_id,
        cat_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(1);

    let group = GroupItem {
        id: group_id,
        name: payload.name.trim().to_string(),
        description: payload.description,
        avatar_url: payload.avatar_url,
        banner_url: payload.banner_url,
        owner_id: user.id,
        member_count: count,
        role: Some("owner".to_string()),
        created_at: Utc::now(),
    };

    Ok(Json(group))
}

// 2. List user's groups
async fn list_my_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<GroupItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT g.id, g.name, g.description, g.avatar_url, g.banner_url, g.owner_id, g.created_at,
               gm.role,
               (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) AS member_count
        FROM groups g
        JOIN group_members gm ON g.id = gm.group_id
        WHERE gm.user_id = $1
        ORDER BY g.created_at DESC
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let groups = rows
        .into_iter()
        .map(|r| GroupItem {
            id: r.id,
            name: r.name,
            description: r.description,
            avatar_url: r.avatar_url,
            banner_url: r.banner_url,
            owner_id: r.owner_id,
            member_count: r.member_count.unwrap_or(1),
            role: Some(r.role),
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(groups))
}

// 3. Get group details
async fn get_group_details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<GroupItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id).await?;
    if role.is_none() {
        return Err(AppError::Forbidden("not_a_group_member"));
    }

    let g = sqlx::query!(
        r#"
        SELECT g.id, g.name, g.description, g.avatar_url, g.banner_url, g.owner_id, g.created_at,
               (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) AS member_count
        FROM groups g
        WHERE g.id = $1
        "#,
        group_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("group_not_found"))?;

    Ok(Json(GroupItem {
        id: g.id,
        name: g.name,
        description: g.description,
        avatar_url: g.avatar_url,
        banner_url: g.banner_url,
        owner_id: g.owner_id,
        member_count: g.member_count.unwrap_or(1),
        role,
        created_at: g.created_at,
    }))
}

// 4. Update group (Owner or Admin)
async fn update_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<UpdateGroupRequest>,
) -> AppResult<Json<GroupItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if !manage::is_admin_role(&role) {
        if payload.description.is_some() {
            return Err(AppError::Forbidden("insufficient_permissions"));
        }
        if payload.name.is_some() {
            manage::check_member_may(&state.db, group_id, &role, "rename").await?;
        }
        if payload.avatar_url.is_some() {
            manage::check_member_may(&state.db, group_id, &role, "avatar").await?;
        }
    }

    let row = sqlx::query!(
        r#"
        UPDATE groups
        SET name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_url = COALESCE($4, avatar_url),
            banner_url = CASE WHEN $5::text IS NULL THEN banner_url WHEN $5 = '' THEN NULL ELSE $5 END
        WHERE id = $1
        RETURNING id, name, description, avatar_url, banner_url, owner_id, created_at
        "#,
        group_id,
        payload.name.as_deref().map(str::trim),
        payload.description,
        payload.avatar_url,
        payload.banner_url
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("group_not_found"))?;

    let member_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(1);

    let updated = GroupItem {
        id: row.id,
        name: row.name,
        description: row.description,
        avatar_url: row.avatar_url,
        banner_url: row.banner_url,
        owner_id: row.owner_id,
        member_count,
        role: Some(role),
        created_at: row.created_at,
    };

    // Broadcast update
    let members = sqlx::query_scalar!(
        "SELECT user_id FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_users(&members, WsEvent::GroupUpdated { group: updated.clone() })
        .await;

    Ok(Json(updated))
}

// 5. Delete group (Owner only)
async fn delete_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let group = sqlx::query!("SELECT owner_id FROM groups WHERE id = $1", group_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("group_not_found"))?;

    if group.owner_id != user.id {
        return Err(AppError::Forbidden("only_owner_can_delete_group"));
    }

    sqlx::query!("DELETE FROM groups WHERE id = $1", group_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// 6. List group members
async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<Vec<GroupMemberItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id).await?;
    if role.is_none() {
        return Err(AppError::Forbidden("not_a_group_member"));
    }

    let rows = sqlx::query!(
        r#"
        SELECT gm.group_id, gm.user_id, gm.role, gm.joined_at,
               u.username, u.avatar_url
        FROM group_members gm
        JOIN users u ON gm.user_id = u.id
        WHERE gm.group_id = $1
        ORDER BY
            CASE gm.role
                WHEN 'owner' THEN 1
                WHEN 'admin' THEN 2
                ELSE 3
            END,
            gm.joined_at ASC
        "#,
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    let members = rows
        .into_iter()
        .map(|r| GroupMemberItem {
            group_id: r.group_id,
            user_id: r.user_id,
            username: r.username.unwrap_or_default(),
            avatar_url: r.avatar_url,
            role: r.role,
            joined_at: r.joined_at,
        })
        .collect();

    Ok(Json(members))
}

// 7. Add member (Owner or Admin)
async fn add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<AddGroupMemberRequest>,
) -> AppResult<Json<GroupMemberItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    let target_user_id = if let Some(uid) = payload.user_id {
        uid
    } else if let Some(ref uname) = payload.username {
        sqlx::query_scalar!(
            "SELECT id FROM users WHERE username_norm = $1",
            uname.to_lowercase()
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user_not_found"))?
    } else {
        return Err(AppError::BadRequest("missing_user_id_or_username"));
    };
    if !crate::privacy::can_add_to_group(&state.db, user.id, target_user_id, Some(group_id)).await? {
        return Err(AppError::Forbidden("cannot_add_user"));
    }

    sqlx::query!(
        r#"
        INSERT INTO group_members (group_id, user_id, role)
        VALUES ($1, $2, 'member')
        ON CONFLICT (group_id, user_id) DO NOTHING
        "#,
        group_id,
        target_user_id
    )
    .execute(&state.db)
    .await?;

    let target_user = sqlx::query!(
        "SELECT username, avatar_url FROM users WHERE id = $1",
        target_user_id
    )
    .fetch_one(&state.db)
    .await?;

    let member_item = GroupMemberItem {
        group_id,
        user_id: target_user_id,
        username: target_user.username.unwrap_or_default(),
        avatar_url: target_user.avatar_url,
        role: "member".to_string(),
        joined_at: Utc::now(),
    };

    let members = sqlx::query_scalar!(
        "SELECT user_id FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    state.ws_hub.send_to_users(
        &members,
        WsEvent::GroupMemberJoined {
            group_id,
            member: member_item.clone(),
        },
    )
    .await;

    Ok(Json(member_item))
}

// 8. Remove member or Leave group
async fn remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let my_role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    let target_role = get_member_role(&state.db, group_id, target_user_id)
        .await?
        .ok_or(AppError::NotFound("member_not_found"))?;

    // Self leaving
    if user.id == target_user_id {
        if my_role == "owner" {
            return Err(AppError::BadRequest("owner_cannot_leave_must_transfer_or_delete"));
        }
    } else {
        // Kick logic
        if my_role == "owner" {
            // Owner can kick anyone
        } else if my_role == "admin" {
            if target_role == "owner" || target_role == "admin" {
                return Err(AppError::Forbidden("admin_cannot_kick_owner_or_admin"));
            }
        } else {
            return Err(AppError::Forbidden("insufficient_permissions"));
        }
    }

    sqlx::query!(
        "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2",
        group_id,
        target_user_id
    )
    .execute(&state.db)
    .await?;

    let members = sqlx::query_scalar!(
        "SELECT user_id FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    state.ws_hub.send_to_users(
        &members,
        WsEvent::GroupMemberLeft {
            group_id,
            user_id: target_user_id,
        },
    )
    .await;

    // Also notify target user
    state.ws_hub.send_to_user(
        &target_user_id,
        WsEvent::GroupMemberLeft {
            group_id,
            user_id: target_user_id,
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "removed": true })))
}

// 9. Update member role (Owner only)
async fn update_member_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, target_user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateGroupMemberRoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let my_role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if my_role != "owner" {
        return Err(AppError::Forbidden("only_owner_can_manage_roles"));
    }

    if payload.role == "owner" {
        if target_user_id == user.id {
            return Ok(Json(serde_json::json!({ "updated": true, "role": "owner" })));
        }
        let target_role = get_member_role(&state.db, group_id, target_user_id).await?;
        if target_role.is_none() {
            return Err(AppError::NotFound("member_not_found"));
        }
        // Transfer ownership
        let mut tx = state.db.begin().await?;
        sqlx::query!(
            "UPDATE groups SET owner_id = $1 WHERE id = $2",
            target_user_id,
            group_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE group_members SET role = 'owner' WHERE group_id = $1 AND user_id = $2",
            group_id,
            target_user_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE group_members SET role = 'admin' WHERE group_id = $1 AND user_id = $2",
            group_id,
            user.id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    } else if payload.role == "admin" || payload.role == "member" {
        if target_user_id == user.id {
            return Err(AppError::BadRequest("owner_cannot_demote_self"));
        }
        let res = sqlx::query!(
            "UPDATE group_members SET role = $1 WHERE group_id = $2 AND user_id = $3",
            payload.role,
            group_id,
            target_user_id
        )
        .execute(&state.db)
        .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("member_not_found"));
        }
    } else {
        return Err(AppError::BadRequest("invalid_role"));
    }

    Ok(Json(serde_json::json!({ "updated": true, "role": payload.role })))
}

// 10. Create group invite link
async fn create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<CreateGroupInviteRequest>,
) -> AppResult<Json<GroupInviteItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    let code = Uuid::new_v4().simple().to_string()[..12].to_string();

    let expires_at = payload
        .expires_in_seconds
        .map(|sec| Utc::now() + Duration::seconds(sec));

    sqlx::query!(
        r#"
        INSERT INTO group_invites (code, group_id, inviter_id, max_uses, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        code,
        group_id,
        user.id,
        payload.max_uses,
        expires_at
    )
    .execute(&state.db)
    .await?;

    Ok(Json(GroupInviteItem {
        code,
        group_id,
        inviter_id: user.id,
        max_uses: payload.max_uses,
        uses: 0,
        expires_at,
        created_at: Utc::now(),
    }))
}

// 11. Join group via invite code
async fn join_by_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> AppResult<Json<GroupItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let invite = sqlx::query!(
        "SELECT group_id, max_uses, uses, expires_at FROM group_invites WHERE code = $1",
        code
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("invite_not_found"))?;

    if let Some(exp) = invite.expires_at {
        if exp < Utc::now() {
            return Err(AppError::BadRequest("invite_expired"));
        }
    }

    if let Some(max_u) = invite.max_uses {
        if invite.uses >= max_u {
            return Err(AppError::BadRequest("invite_limit_reached"));
        }
    }

    let settings = manage::load_settings(&state.db, invite.group_id).await?;
    if !settings.allow_link_join {
        return Err(AppError::Forbidden("link_join_disabled"));
    }
    let blocked = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM group_blocks WHERE user_id = $1 AND group_id = $2"#, user.id, invite.group_id)
        .fetch_optional(&state.db)
        .await?;
    if blocked.is_some() {
        return Err(AppError::Forbidden("group_blocked"));
    }
    let already = get_member_role(&state.db, invite.group_id, user.id).await?.is_some();
    if settings.require_approval && !already {
        sqlx::query!(
            "INSERT INTO group_join_requests (group_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            invite.group_id,
            user.id
        )
        .execute(&state.db)
        .await?;
        let admins: Vec<Uuid> = sqlx::query_scalar!(
            "SELECT user_id FROM group_members WHERE group_id = $1 AND role IN ('owner', 'admin')",
            invite.group_id
        )
        .fetch_all(&state.db)
        .await?;
        state
            .ws_hub
            .send_to_users(
                &admins,
                WsEvent::GroupJoinRequest(serde_json::json!({ "group_id": invite.group_id, "user_id": user.id, "username": user.username })),
            )
            .await;
        return Err(AppError::Conflict("join_request_pending"));
    }
    let mut tx = state.db.begin().await?;

    // Increment invite usage
    sqlx::query!(
        "UPDATE group_invites SET uses = uses + 1 WHERE code = $1",
        code
    )
    .execute(&mut *tx)
    .await?;

    // Add user as member
    sqlx::query!(
        r#"
        INSERT INTO group_members (group_id, user_id, role)
        VALUES ($1, $2, 'member')
        ON CONFLICT (group_id, user_id) DO NOTHING
        "#,
        invite.group_id,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let g = sqlx::query!(
        r#"
        SELECT g.id, g.name, g.description, g.avatar_url, g.banner_url, g.owner_id, g.created_at,
               (SELECT COUNT(*) FROM group_members WHERE group_id = g.id) AS member_count
        FROM groups g
        WHERE g.id = $1
        "#,
        invite.group_id
    )
    .fetch_one(&state.db)
    .await?;

    let member_item = GroupMemberItem {
        group_id: invite.group_id,
        user_id: user.id,
        username: user.username.unwrap_or_default(),
        avatar_url: user.avatar_url,
        role: "member".to_string(),
        joined_at: Utc::now(),
    };

    let members = sqlx::query_scalar!(
        "SELECT user_id FROM group_members WHERE group_id = $1",
        invite.group_id
    )
    .fetch_all(&state.db)
    .await?;

    state.ws_hub.send_to_users(
        &members,
        WsEvent::GroupMemberJoined {
            group_id: invite.group_id,
            member: member_item,
        },
    )
    .await;

    Ok(Json(GroupItem {
        id: g.id,
        name: g.name,
        description: g.description,
        avatar_url: g.avatar_url,
        banner_url: g.banner_url,
        owner_id: g.owner_id,
        member_count: g.member_count.unwrap_or(1),
        role: Some("member".to_string()),
        created_at: g.created_at,
    }))
}

// 12. List messages
async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<Vec<GroupMessageItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if let Some(ch_id) = query.channel_id {
        let is_priv: Option<bool> = sqlx::query_scalar(
            "SELECT is_private FROM group_channels WHERE id = $1 AND group_id = $2",
        )
        .bind(ch_id)
        .bind(group_id)
        .fetch_optional(&state.db)
        .await?;
        match is_priv {
            None => return Err(AppError::NotFound("channel_not_found")),
            Some(true) if !matches!(role.as_str(), "owner" | "admin") => {
                return Err(AppError::Forbidden("private_channel_admin_only"));
            }
            _ => {}
        }
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let before_seq = query.before_seq.unwrap_or(i64::MAX);
    let is_group_admin = matches!(role.as_str(), "owner" | "admin");

    let rows = sqlx::query!(
        r#"
        SELECT m.id, m.seq, m.group_id, m.channel_id, m.sender_id, m.content, m.reply_to_id,
               m.is_pinned, m.is_edited, m.created_at,
               u.username, u.avatar_url,
               b.display_name AS "bot_name?", b.avatar_url AS "bot_avatar?"
        FROM group_messages m
        JOIN users u ON m.sender_id = u.id
        LEFT JOIN bots b ON b.id = m.bot_id
        LEFT JOIN group_channels gc ON gc.id = m.channel_id
        WHERE m.group_id = $1
          AND ($2::uuid IS NULL OR m.channel_id = $2)
          AND ($5::bool OR m.channel_id IS NULL OR COALESCE(gc.is_private, FALSE) = FALSE)
          AND m.seq < $3
        ORDER BY m.seq DESC
        LIMIT $4
        "#,
        group_id,
        query.channel_id,
        before_seq,
        limit,
        is_group_admin
    )
    .fetch_all(&state.db)
    .await?;

    let msg_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let reaction_rows = if msg_ids.is_empty() {
        vec![]
    } else {
        sqlx::query!(
            r#"
            SELECT r.id, r.message_id, r.user_id, r.emoji, r.created_at, u.username
            FROM group_message_reactions r
            JOIN users u ON r.user_id = u.id
            WHERE r.message_id = ANY($1)
            ORDER BY r.created_at ASC
            "#,
            &msg_ids
        )
        .fetch_all(&state.db)
        .await?
    };

    let mut reactions_map: std::collections::HashMap<Uuid, Vec<GroupReactionItem>> = std::collections::HashMap::new();
    for r in reaction_rows {
        reactions_map.entry(r.message_id).or_default().push(GroupReactionItem {
            id: r.id,
            message_id: r.message_id,
            user_id: r.user_id,
            username: r.username.unwrap_or_default(),
            emoji: r.emoji,
            created_at: r.created_at,
        });
    }

    let mut messages: Vec<GroupMessageItem> = rows
        .into_iter()
        .map(|r| {
            let reactions = reactions_map.remove(&r.id).unwrap_or_default();
            GroupMessageItem {
                id: r.id,
                seq: r.seq,
                group_id: r.group_id,
                channel_id: r.channel_id,
                sender_id: r.sender_id,
                is_bot: r.bot_name.is_some(),
                sender_name: r.bot_name.clone().unwrap_or_else(|| r.username.clone().unwrap_or_default()),
                sender_avatar_url: if r.bot_name.is_some() { r.bot_avatar.clone() } else { r.avatar_url.clone() },
                content: r.content,
                reply_to_id: r.reply_to_id,
                is_pinned: r.is_pinned,
                is_edited: r.is_edited,
                reactions,
                created_at: r.created_at,
            }
        })
        .collect();

    messages.reverse();
    Ok(Json(messages))
}

// 13. Send group message
async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<PostGroupMessageRequest>,
) -> AppResult<Json<GroupMessageItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    manage::check_member_may(&state.db, group_id, &role, "send").await?;
    let body = payload.content.trim_start();
    for (prefix, action) in [("poll:", "poll"), ("note:", "note"), ("reminder:", "reminder")] {
        if body.starts_with(prefix) {
            manage::check_member_may(&state.db, group_id, &role, action).await?;
        }
    }

    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("empty_message"));
    }
    if let Some(ch_id) = payload.channel_id {
        let is_priv: Option<bool> = sqlx::query_scalar(
            "SELECT is_private FROM group_channels WHERE id = $1 AND group_id = $2",
        )
        .bind(ch_id)
        .bind(group_id)
        .fetch_optional(&state.db)
        .await?;
        match is_priv {
            None => return Err(AppError::NotFound("channel_not_found")),
            Some(true) if !matches!(role.as_str(), "owner" | "admin") => {
                return Err(AppError::Forbidden("private_channel_admin_only"));
            }
            _ => {}
        }
    }

    let msg_id = Uuid::new_v4();
    let row = sqlx::query!(
        r#"
        INSERT INTO group_messages (id, group_id, channel_id, sender_id, content, reply_to_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING seq, created_at
        "#,
        msg_id,
        group_id,
        payload.channel_id,
        user.id,
        payload.content.trim(),
        payload.reply_to_id
    )
    .fetch_one(&state.db)
    .await?;

    let item = GroupMessageItem {
        id: msg_id,
        seq: row.seq,
        group_id,
        channel_id: payload.channel_id,
        sender_id: user.id,
        sender_name: user.username.unwrap_or_default(),
        sender_avatar_url: user.avatar_url,
        content: payload.content.trim().to_string(),
        reply_to_id: payload.reply_to_id,
        is_pinned: false,
        is_edited: false,
        reactions: Vec::new(),
        is_bot: false,
        created_at: row.created_at,
    };

    // Broadcast to all group members
    let member_ids = sqlx::query_scalar!(
        "SELECT user_id FROM group_members WHERE group_id = $1",
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_users(&member_ids, WsEvent::GroupMessageNew { message: item.clone() })
        .await;
    crate::bots::on_group_message(&state.db, &item).await;
    crate::plugins::webhooks::dispatch_outgoing_event(
        state.clone(),
        "group",
        group_id,
        payload.channel_id,
        "onGroupMessage",
        serde_json::to_value(&item).unwrap_or_default(),
    )
    .await;

    Ok(Json(item))
}

// 14. Edit message
async fn edit_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<PostGroupMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let _role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    let content = payload.content.trim();
    if content.is_empty() || content.len() > 4000 {
        return Err(AppError::BadRequest("invalid_message_content"));
    }

    let msg = sqlx::query!(
        "SELECT sender_id FROM group_messages WHERE id = $1 AND group_id = $2",
        message_id,
        group_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id {
        return Err(AppError::Forbidden("cannot_edit_other_user_message"));
    }

    sqlx::query!(
        "UPDATE group_messages SET content = $1, is_edited = true WHERE id = $2",
        content,
        message_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "edited": true })))
}

// 15. Delete message (Sender or Owner/Admin)
async fn delete_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, message_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    let msg = sqlx::query!(
        "SELECT sender_id FROM group_messages WHERE id = $1 AND group_id = $2",
        message_id,
        group_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id && role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    sqlx::query!(
        "DELETE FROM group_messages WHERE id = $1",
        message_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// 16. Pin / Unpin message (Owner or Admin)
async fn pin_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, message_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    manage::check_member_may(&state.db, group_id, &role, "pin").await?;

    let updated = sqlx::query!(
        r#"
        UPDATE group_messages
        SET is_pinned = NOT is_pinned
        WHERE id = $1 AND group_id = $2
        RETURNING is_pinned
        "#,
        message_id,
        group_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    Ok(Json(serde_json::json!({ "pinned": updated.is_pinned })))
}

// 17. List categories
async fn list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<Vec<GroupCategoryItem>>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id).await?;
    if role.is_none() {
        return Err(AppError::Forbidden("not_a_group_member"));
    }

    let rows = sqlx::query_as!(
        GroupCategoryItem,
        "SELECT id, group_id, name, position, created_at FROM group_categories WHERE group_id = $1 ORDER BY position ASC, created_at ASC",
        group_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

// 18. Create category
async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<CreateGroupCategoryRequest>,
) -> AppResult<Json<GroupCategoryItem>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("category_name_cannot_be_empty"));
    }

    let cat_id = Uuid::new_v4();
    let pos = payload.position.unwrap_or(0);
    let row = sqlx::query_as!(
        GroupCategoryItem,
        r#"
        INSERT INTO group_categories (id, group_id, name, position)
        VALUES ($1, $2, $3, $4)
        RETURNING id, group_id, name, position, created_at
        "#,
        cat_id,
        group_id,
        payload.name.trim(),
        pos
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row))
}

// 19. Delete category
async fn delete_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, category_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    sqlx::query!("DELETE FROM group_categories WHERE id = $1 AND group_id = $2", category_id, group_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// 20. List channels
async fn list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<Vec<GroupChannelItem>>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    let is_group_admin = matches!(role.as_str(), "owner" | "admin");

    let rows = sqlx::query_as!(
        GroupChannelItem,
        r#"
        SELECT id, group_id, category_id, name, description, channel_type, position, is_private, created_at
        FROM group_channels
        WHERE group_id = $1 AND ($2::bool OR NOT is_private)
        ORDER BY position ASC, created_at ASC
        "#,
        group_id,
        is_group_admin
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

// 21. Create channel
async fn create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<CreateGroupChannelRequest>,
) -> AppResult<Json<GroupChannelItem>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("channel_name_cannot_be_empty"));
    }

    let chan_id = Uuid::new_v4();
    let chan_type = payload.channel_type.unwrap_or_else(|| "text".to_string());
    let pos = payload.position.unwrap_or(0);
    let is_priv = payload.is_private.unwrap_or(false);

    let row = sqlx::query_as!(
        GroupChannelItem,
        r#"
        INSERT INTO group_channels (id, group_id, category_id, name, description, channel_type, position, is_private)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, group_id, category_id, name, description, channel_type, position, is_private, created_at
        "#,
        chan_id,
        group_id,
        payload.category_id,
        payload.name.trim(),
        payload.description,
        chan_type,
        pos,
        is_priv
    )
    .fetch_one(&state.db)
    .await?;

    // Broadcast to members
    let member_ids = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
        .fetch_all(&state.db)
        .await?;

    state
        .ws_hub
        .send_to_users(
            &member_ids,
            WsEvent::GroupChannelNew {
                group_id,
                channel: row.clone(),
            },
        )
        .await;

    Ok(Json(row))
}

// 22. Delete channel
async fn delete_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, channel_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;
    if role != "owner" && role != "admin" {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    sqlx::query!("DELETE FROM group_channels WHERE id = $1 AND group_id = $2", channel_id, group_id)
        .execute(&state.db)
        .await?;

    let member_ids = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
        .fetch_all(&state.db)
        .await?;

    state
        .ws_hub
        .send_to_users(
            &member_ids,
            WsEvent::GroupChannelDeleted {
                group_id,
                channel_id,
            },
        )
        .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// 23. Toggle Reaction
async fn toggle_reaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ToggleReactionRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id).await?;
    if role.is_none() {
        return Err(AppError::Forbidden("not_a_group_member"));
    }

    let belongs_to_group = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM group_messages WHERE id = $1 AND group_id = $2)",
        message_id,
        group_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(false);
    if !belongs_to_group {
        return Err(AppError::NotFound("message_not_found"));
    }

    let emoji = payload.emoji.trim();
    if emoji.is_empty() || emoji.len() > 32 {
        return Err(AppError::BadRequest("invalid_emoji"));
    }

    let existing = sqlx::query!(
        "SELECT id FROM group_message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
        message_id,
        user.id,
        emoji
    )
    .fetch_optional(&state.db)
    .await?;

    let action = if let Some(r) = existing {
        sqlx::query!("DELETE FROM group_message_reactions WHERE id = $1", r.id)
            .execute(&state.db)
            .await?;
        "removed"
    } else {
        sqlx::query!(
            "INSERT INTO group_message_reactions (message_id, user_id, emoji) VALUES ($1, $2, $3)",
            message_id,
            user.id,
            emoji
        )
        .execute(&state.db)
        .await?;
        "added"
    };

    let member_ids = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", group_id)
        .fetch_all(&state.db)
        .await?;

    state
        .ws_hub
        .send_to_users(
            &member_ids,
            WsEvent::GroupReactionUpdate {
                group_id,
                message_id,
                user_id: user.id,
                emoji: emoji.to_string(),
                action: action.to_string(),
            },
        )
        .await;

    Ok(Json(serde_json::json!({ "action": action, "emoji": emoji })))
}

// Upload group avatar (honest .avif)
async fn upload_group_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if !manage::is_admin_role(&role) {
        manage::check_member_may(&state.db, group_id, &role, "avatar").await?;
    }

    if body.is_empty() {
        return Err(AppError::BadRequest("empty_avatar_payload"));
    }
    if body.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("avatar_too_large"));
    }

    let body_vec = body.to_vec();
    let processed = tokio::task::spawn_blocking(move || crate::media_opt::process_avatar_honest(&body_vec))
        .await
        .map_err(|_| AppError::Internal)?
        .map_err(AppError::BadRequest)?;

    let avatar_dir = state.config.uploads_dir.join("avatars");
    tokio::fs::create_dir_all(&avatar_dir).await.map_err(|e| {
        tracing::error!(error = %e, "failed to create avatar directory");
        AppError::Internal
    })?;

    let filename = format!("group_{}.{}", group_id, processed.ext);
    let filepath = avatar_dir.join(&filename);

    tokio::fs::write(&filepath, &processed.main_bytes).await.map_err(|e| {
        tracing::error!(error = %e, "failed to write group avatar file");
        AppError::Internal
    })?;
    if !processed.thumb_bytes.is_empty() {
        let thumb_name = format!("group_{}_thumb.{}", group_id, processed.ext);
        let _ = tokio::fs::write(avatar_dir.join(&thumb_name), &processed.thumb_bytes).await;
    }

    let now_ts = chrono::Utc::now().timestamp_millis();
    let avatar_url = format!("/uploads/avatars/{}?t={}", filename, now_ts);

    sqlx::query!(
        "UPDATE groups SET avatar_url = $1 WHERE id = $2",
        avatar_url,
        group_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "avatar_url": avatar_url })))
}

// Upload group cover banner
async fn upload_group_banner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let role = get_member_role(&state.db, group_id, user.id)
        .await?
        .ok_or(AppError::Forbidden("not_a_group_member"))?;

    if !manage::is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }

    if body.is_empty() {
        return Err(AppError::BadRequest("empty_banner_payload"));
    }
    if body.len() > 8 * 1024 * 1024 {
        return Err(AppError::BadRequest("banner_too_large"));
    }

    let banner_dir = state.config.uploads_dir.join("banners");
    tokio::fs::create_dir_all(&banner_dir).await.map_err(|e| {
        tracing::error!(error = %e, "failed to create banner directory");
        AppError::Internal
    })?;

    let ext = crate::users::sniff_image(&body).ok_or(AppError::BadRequest("unsupported_banner_format"))?;
    for old in ["avif", "webp", "gif", "png", "jpg"] {
        if old != ext {
            let _ = tokio::fs::remove_file(banner_dir.join(format!("group_{}.{old}", group_id))).await;
        }
    }
    let filename = format!("group_{}.{ext}", group_id);
    let filepath = banner_dir.join(&filename);
    tokio::fs::write(&filepath, &body).await.map_err(|e| {
        tracing::error!(error = %e, "failed to write group banner file");
        AppError::Internal
    })?;

    let now_ts = chrono::Utc::now().timestamp_millis();
    let banner_url = format!("/uploads/banners/{}?t={}", filename, now_ts);

    sqlx::query!(
        "UPDATE groups SET banner_url = $1 WHERE id = $2",
        banner_url,
        group_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "banner_url": banner_url })))
}
