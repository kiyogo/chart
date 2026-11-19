pub mod permissions;
pub mod roles;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    websocket::WsEvent,
};
use crate::models::{
    ChannelItem, ChannelMessageItem, CreateCategoryRequest, CreateChannelRequest,
    CreateRoleRequest, CreateServerRequest, PostChannelMessageRequest, ServerCategoryItem,
    ServerItem, ServerMemberItem, ServerRoleItem, permissions as perm_flags,
};

#[derive(Deserialize)]
pub struct ChannelMessagesQuery {
    pub before_seq: Option<i64>,
    pub limit: Option<i64>,
}

pub fn server_routes() -> Router<AppState> {
    Router::new()
        .route("/invite-preview/{code}", get(handle_invite_preview))
        .route("/{id}/settings", get(handle_get_settings).put(handle_update_settings))
        .route("/", post(handle_create_server).get(handle_list_servers))
        .route("/discover", get(handle_discover_servers))
        .route("/join/{code}", post(handle_join_server_by_invite))
        .route("/{id}", get(handle_get_server).delete(handle_delete_server))
        .route("/{id}/join", post(handle_join_server_direct))
        .route(
            "/{id}/categories",
            get(handle_list_categories).post(handle_create_category),
        )
        .route(
            "/{id}/channels",
            get(handle_list_channels).post(handle_create_channel),
        )
        .route("/{id}/members", get(handle_list_members))
        .route(
            "/{id}/roles",
            get(handle_list_roles).post(handle_create_role),
        )
        .route("/{id}/roles/{role_id}", put(roles::update_role).delete(roles::delete_role))
        .route("/{id}/members/{user_id}/roles/{role_id}", put(roles::assign_role).delete(roles::unassign_role))
        .route("/{id}/members/{user_id}", delete(roles::kick_member))
        .route("/{id}/my-permissions", get(roles::my_permissions))
        .route("/{id}/invites", post(handle_create_invite))
}

pub fn channel_routes() -> Router<AppState> {
    Router::new().route(
        "/{id}/messages",
        get(handle_get_channel_messages).post(handle_post_channel_message),
    )
}

// -----------------------------------------------------------------------------
// Server Handlers
// -----------------------------------------------------------------------------

async fn handle_create_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateServerRequest>,
) -> AppResult<Json<ServerItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let name = payload.name.trim();
    if name.len() < 2 || name.len() > 64 {
        return Err(AppError::BadRequest("invalid_server_name_length"));
    }

    let mut tx = state.db.begin().await?;

    let server_id = Uuid::new_v4();
    let desc = payload.description.as_deref();

    let server_row = sqlx::query!(
        r#"
        INSERT INTO servers (id, name, description, owner_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, icon_url, owner_id, created_at
        "#,
        server_id,
        name,
        desc,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Default @everyone role
    let role_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO server_roles (id, server_id, name, color, position, permissions)
        VALUES ($1, $2, '@everyone', '#99AAB5', 0, $3)
        "#,
        role_id,
        server_id,
        perm_flags::DEFAULT_EVERYONE
    )
    .execute(&mut *tx)
    .await?;

    // Default General category
    let category_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO server_categories (id, server_id, name, position)
        VALUES ($1, $2, 'General', 0)
        "#,
        category_id,
        server_id
    )
    .execute(&mut *tx)
    .await?;

    // Default #general text channel
    let channel_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, category_id, name, channel_type, position, is_private)
        VALUES ($1, $2, $3, 'general', 'text', 0, FALSE)
        "#,
        channel_id,
        server_id,
        category_id
    )
    .execute(&mut *tx)
    .await?;

    // Default Voice category & voice channel
    let voice_cat_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO server_categories (id, server_id, name, position)
        VALUES ($1, $2, 'Giọng nói', 1)
        "#,
        voice_cat_id,
        server_id
    )
    .execute(&mut *tx)
    .await?;

    let voice_ch_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, category_id, name, channel_type, position, is_private)
        VALUES ($1, $2, $3, 'Tổng quát', 'voice', 0, FALSE)
        "#,
        voice_ch_id,
        server_id,
        voice_cat_id
    )
    .execute(&mut *tx)
    .await?;

    // Add owner as server member
    sqlx::query!(
        r#"
        INSERT INTO server_members (server_id, user_id)
        VALUES ($1, $2)
        "#,
        server_id,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ServerItem {
        id: server_row.id,
        name: server_row.name,
        description: server_row.description,
        icon_url: server_row.icon_url,
        owner_id: server_row.owner_id,
        created_at: server_row.created_at,
        member_count: None,
        is_owner: None,
    }))
}

async fn handle_list_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<ServerItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT s.id, s.name, s.description, s.icon_url, s.owner_id, s.created_at,
               (SELECT COUNT(*) FROM server_members sm WHERE sm.server_id = s.id) AS "member_count!"
        FROM servers s
        JOIN server_members m ON s.id = m.server_id
        WHERE m.user_id = $1
        ORDER BY s.created_at ASC
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let servers = rows
        .into_iter()
        .map(|r| ServerItem {
            id: r.id,
            name: r.name,
            description: r.description,
            icon_url: r.icon_url,
            is_owner: Some(r.owner_id == user.id),
            owner_id: r.owner_id,
            created_at: r.created_at,
            member_count: Some(r.member_count),
        })
        .collect();

    Ok(Json(servers))
}

async fn handle_discover_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<ServerItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT s.id, s.name, s.description, s.icon_url, s.owner_id, s.created_at
        FROM servers s
        WHERE s.is_public AND s.verification_status = 'verified'
        ORDER BY s.created_at DESC
        LIMIT 25
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let servers = rows
        .into_iter()
        .map(|r| ServerItem {
            id: r.id,
            name: r.name,
            description: r.description,
            icon_url: r.icon_url,
            owner_id: r.owner_id,
            created_at: r.created_at,
            member_count: None,
            is_owner: None,
        })
        .collect();

    Ok(Json(servers))
}

async fn handle_join_server_direct(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<ServerItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // Direct join (without an invite) is only possible for verified public communities.
    let srv = sqlx::query!(
        "SELECT id, name, description, icon_url, owner_id, created_at FROM servers WHERE id = $1 AND is_public AND verification_status = 'verified'",
        server_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("server_not_found"))?;

    sqlx::query!(
        "INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        server_id,
        user.id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(ServerItem {
        id: srv.id,
        name: srv.name,
        description: srv.description,
        icon_url: srv.icon_url,
        owner_id: srv.owner_id,
        created_at: srv.created_at,
        member_count: None,
        is_owner: None,
    }))
}

async fn handle_get_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<ServerItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // Verify member
    let is_member = sqlx::query_scalar!(
        r#"SELECT 1 as "one!" FROM server_members WHERE server_id = $1 AND user_id = $2"#,
        server_id,
        user.id
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();

    if !is_member {
        return Err(AppError::Forbidden("not_server_member"));
    }

    let r = sqlx::query!(
        "SELECT id, name, description, icon_url, owner_id, created_at FROM servers WHERE id = $1",
        server_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("server_not_found"))?;

    Ok(Json(ServerItem {
        id: r.id,
        name: r.name,
        description: r.description,
        icon_url: r.icon_url,
        owner_id: r.owner_id,
        created_at: r.created_at,
        member_count: None,
        is_owner: None,
    }))
}

async fn handle_delete_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM servers WHERE id = $1",
        server_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("server_not_found"))?;

    if owner_id != user.id {
        return Err(AppError::Forbidden("only_owner_can_delete_server"));
    }

    sqlx::query!("DELETE FROM servers WHERE id = $1", server_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// -----------------------------------------------------------------------------
// Category Handlers
// -----------------------------------------------------------------------------

async fn handle_list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ServerCategoryItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, server_id, name, position, created_at
        FROM server_categories
        WHERE server_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
        server_id
    )
    .fetch_all(&state.db)
    .await?;

    let categories = rows
        .into_iter()
        .map(|r| ServerCategoryItem {
            id: r.id,
            server_id: r.server_id,
            name: r.name,
            position: r.position,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(categories))
}

async fn handle_create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<CreateCategoryRequest>,
) -> AppResult<Json<ServerCategoryItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::MANAGE_CHANNELS,
    )
    .await?;

    let name = payload.name.trim();
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::BadRequest("invalid_category_name"));
    }

    let pos = payload.position.unwrap_or(0);
    let category_id = Uuid::new_v4();

    let row = sqlx::query!(
        r#"
        INSERT INTO server_categories (id, server_id, name, position)
        VALUES ($1, $2, $3, $4)
        RETURNING id, server_id, name, position, created_at
        "#,
        category_id,
        server_id,
        name,
        pos
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ServerCategoryItem {
        id: row.id,
        server_id: row.server_id,
        name: row.name,
        position: row.position,
        created_at: row.created_at,
    }))
}

// -----------------------------------------------------------------------------
// Channel Handlers
// -----------------------------------------------------------------------------

async fn handle_list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ChannelItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, server_id, category_id, name, channel_type, position, is_private, created_at
        FROM channels
        WHERE server_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
        server_id
    )
    .fetch_all(&state.db)
    .await?;

    let channels = rows
        .into_iter()
        .map(|r| ChannelItem {
            id: r.id,
            server_id: r.server_id,
            category_id: r.category_id,
            name: r.name,
            channel_type: r.channel_type,
            position: r.position,
            is_private: r.is_private,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(channels))
}

async fn handle_create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<CreateChannelRequest>,
) -> AppResult<Json<ChannelItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::MANAGE_CHANNELS,
    )
    .await?;

    let name = payload.name.trim();
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::BadRequest("invalid_channel_name"));
    }

    let ch_type = payload.channel_type.as_deref().unwrap_or("text");
    if !["text", "voice", "announcement", "forum", "event"].contains(&ch_type) {
        return Err(AppError::BadRequest("invalid_channel_type"));
    }

    let pos = payload.position.unwrap_or(0);
    let is_priv = payload.is_private.unwrap_or(false);
    let channel_id = Uuid::new_v4();

    let row = sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, category_id, name, channel_type, position, is_private)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, server_id, category_id, name, channel_type, position, is_private, created_at
        "#,
        channel_id,
        server_id,
        payload.category_id,
        name,
        ch_type,
        pos,
        is_priv
    )
    .fetch_one(&state.db)
    .await?;

    crate::community::audit(&state, server_id, user.id, "channel_created", name).await;
    Ok(Json(ChannelItem {
        id: row.id,
        server_id: row.server_id,
        category_id: row.category_id,
        name: row.name,
        channel_type: row.channel_type,
        position: row.position,
        is_private: row.is_private,
        created_at: row.created_at,
    }))
}

// -----------------------------------------------------------------------------
// Members & Roles
// -----------------------------------------------------------------------------

async fn handle_list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ServerMemberItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let rows = sqlx::query!(
        r#"
        SELECT m.server_id, m.user_id, u.username, m.nickname, u.avatar_url, m.joined_at
        FROM server_members m
        JOIN users u ON m.user_id = u.id
        WHERE m.server_id = $1
        ORDER BY m.joined_at ASC
        "#,
        server_id
    )
    .fetch_all(&state.db)
    .await?;

    let role_rows = sqlx::query!(
        r#"
        SELECT user_id, role_id
        FROM server_member_roles
        WHERE server_id = $1
        "#,
        server_id
    )
    .fetch_all(&state.db)
    .await?;

    let mut user_roles: std::collections::HashMap<Uuid, Vec<Uuid>> =
        std::collections::HashMap::new();
    for r in role_rows {
        user_roles.entry(r.user_id).or_default().push(r.role_id);
    }

    let members = rows
        .into_iter()
        .map(|r| {
            let roles = user_roles.remove(&r.user_id).unwrap_or_default();
            ServerMemberItem {
                server_id: r.server_id,
                user_id: r.user_id,
                username: r.username.unwrap_or_default(),
                nickname: r.nickname,
                avatar_url: r.avatar_url,
                roles,
                joined_at: r.joined_at,
            }
        })
        .collect();

    Ok(Json(members))
}

async fn handle_list_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<Vec<ServerRoleItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, server_id, name, color, position, permissions, created_at
        FROM server_roles
        WHERE server_id = $1
        ORDER BY position ASC
        "#,
        server_id
    )
    .fetch_all(&state.db)
    .await?;

    let roles = rows
        .into_iter()
        .map(|r| ServerRoleItem {
            id: r.id,
            server_id: r.server_id,
            name: r.name,
            color: r.color,
            position: r.position,
            permissions: r.permissions,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(roles))
}

async fn handle_create_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<CreateRoleRequest>,
) -> AppResult<Json<ServerRoleItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::MANAGE_ROLES,
    )
    .await?;

    let name = payload.name.trim();
    if name.is_empty() || name.len() > 32 {
        return Err(AppError::BadRequest("invalid_role_name"));
    }

    let color = payload.color.as_deref().unwrap_or("#99AAB5");
    let pos = payload.position.unwrap_or(0);
    let perms = payload.permissions.unwrap_or(0);
    let role_id = Uuid::new_v4();

    let row = sqlx::query!(
        r#"
        INSERT INTO server_roles (id, server_id, name, color, position, permissions)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, server_id, name, color, position, permissions, created_at
        "#,
        role_id,
        server_id,
        name,
        color,
        pos,
        perms
    )
    .fetch_one(&state.db)
    .await?;

    crate::community::audit(&state, server_id, user.id, "role_created", payload.name.trim()).await;
    Ok(Json(ServerRoleItem {
        id: row.id,
        server_id: row.server_id,
        name: row.name,
        color: row.color,
        position: row.position,
        permissions: row.permissions,
        created_at: row.created_at,
    }))
}

// -----------------------------------------------------------------------------
// Invites Handlers
// -----------------------------------------------------------------------------

async fn handle_create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    permissions::require_server_permission(
        &state.db,
        server_id,
        user.id,
        perm_flags::CREATE_INVITES,
    )
    .await?;

    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).map_err(|_| AppError::Internal)?;
    let code = hex::encode(bytes);

    sqlx::query!(
        r#"
        INSERT INTO server_invites (code, server_id, inviter_id)
        VALUES ($1, $2, $3)
        "#,
        code,
        server_id,
        user.id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "code": code })))
}

async fn handle_join_server_by_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> AppResult<Json<ServerItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let invite = sqlx::query!(
        r#"
        SELECT server_id, max_uses, uses, expires_at
        FROM server_invites
        WHERE code = $1
        "#,
        code
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("invite_not_found"))?;

    if let Some(exp) = invite.expires_at {
        if exp <= Utc::now() {
            return Err(AppError::BadRequest("invite_expired"));
        }
    }
    if let Some(max_u) = invite.max_uses {
        if invite.uses >= max_u {
            return Err(AppError::BadRequest("invite_max_uses_reached"));
        }
    }

    let mut tx = state.db.begin().await?;

    // Increment invite usage
    sqlx::query!(
        "UPDATE server_invites SET uses = uses + 1 WHERE code = $1",
        code
    )
    .execute(&mut *tx)
    .await?;

    // Insert user into server members
    sqlx::query!(
        r#"
        INSERT INTO server_members (server_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (server_id, user_id) DO NOTHING
        "#,
        invite.server_id,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    let server_row = sqlx::query!(
        "SELECT id, name, description, icon_url, owner_id, created_at FROM servers WHERE id = $1",
        invite.server_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ServerItem {
        id: server_row.id,
        name: server_row.name,
        description: server_row.description,
        icon_url: server_row.icon_url,
        owner_id: server_row.owner_id,
        created_at: server_row.created_at,
        member_count: None,
        is_owner: None,
    }))
}

// -----------------------------------------------------------------------------
// Channel Messages
// -----------------------------------------------------------------------------

async fn handle_get_channel_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ChannelMessagesQuery>,
) -> AppResult<Json<Vec<ChannelMessageItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // Fetch channel to find its server_id
    let ch = sqlx::query!(
        "SELECT server_id FROM channels WHERE id = $1",
        channel_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("channel_not_found"))?;

    permissions::require_server_permission(
        &state.db,
        ch.server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let before_seq = query.before_seq.unwrap_or(i64::MAX);

    let rows = sqlx::query!(
        r#"
        SELECT m.id, m.seq, m.channel_id, m.sender_id,
               COALESCE(sm.nickname, u.username) AS "username?", COALESCE(sm.avatar_url, u.avatar_url) AS "avatar_url?",
               m.content, m.reply_to_id, m.is_pinned, m.is_edited, m.created_at
        FROM channel_messages m
        JOIN users u ON m.sender_id = u.id
        JOIN channels c ON c.id = m.channel_id
        LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = m.sender_id
        WHERE m.channel_id = $1 AND m.seq < $2 AND m.thread_id IS NULL
        ORDER BY m.seq DESC
        LIMIT $3
        "#,
        channel_id,
        before_seq,
        limit
    )
    .fetch_all(&state.db)
    .await?;

    let messages = rows
        .into_iter()
        .map(|r| ChannelMessageItem {
            id: r.id,
            seq: r.seq,
            channel_id: r.channel_id,
            sender_id: r.sender_id,
            sender_name: r.username.unwrap_or_default(),
            sender_avatar_url: r.avatar_url,
            content: r.content,
            reply_to_id: r.reply_to_id,
            is_pinned: r.is_pinned,
            is_edited: r.is_edited,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(messages))
}

async fn handle_post_channel_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<PostChannelMessageRequest>,
) -> AppResult<Json<ChannelMessageItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let content = payload.content.trim();
    if content.is_empty() || content.len() > 4000 {
        return Err(AppError::BadRequest("invalid_message_content"));
    }

    let ch = sqlx::query!(
        "SELECT server_id FROM channels WHERE id = $1",
        channel_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("channel_not_found"))?;

    permissions::require_server_permission(
        &state.db,
        ch.server_id,
        user.id,
        perm_flags::SEND_MESSAGES,
    )
    .await?;
    let lower = content.to_lowercase();
    if (lower.contains("@everyone") || lower.contains("@here"))
        && !permissions::has_server_permission(&state.db, ch.server_id, user.id, perm_flags::MENTION_EVERYONE).await?
    {
        return Err(AppError::Forbidden("mention_everyone_forbidden"));
    }
    // Spec 013: members must accept the rules first when the community requires it.
    crate::community::rules_gate(&state, ch.server_id, user.id).await?;
    let local = sqlx::query!("SELECT nickname, avatar_url FROM server_members WHERE server_id = $1 AND user_id = $2", ch.server_id, user.id)
        .fetch_optional(&state.db)
        .await?;

    let message_id = Uuid::new_v4();

    let row = sqlx::query!(
        r#"
        INSERT INTO channel_messages (id, channel_id, sender_id, content, reply_to_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING seq, created_at, is_pinned, is_edited
        "#,
        message_id,
        channel_id,
        user.id,
        content,
        payload.reply_to_id
    )
    .fetch_one(&state.db)
    .await?;

    let item = ChannelMessageItem {
        id: message_id,
        seq: row.seq,
        channel_id,
        sender_id: user.id,
        sender_name: local.as_ref().and_then(|l| l.nickname.clone()).or(user.username.clone()).unwrap_or_default(),
        sender_avatar_url: local.as_ref().and_then(|l| l.avatar_url.clone()).or(user.avatar_url.clone()),
        content: content.to_string(),
        reply_to_id: payload.reply_to_id,
        is_pinned: row.is_pinned,
        is_edited: row.is_edited,
        created_at: row.created_at,
    };

    // Broadcast to all server members via WebSocket
    let member_ids = sqlx::query_scalar!(
        "SELECT user_id FROM server_members WHERE server_id = $1",
        ch.server_id
    )
    .fetch_all(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_users(
            &member_ids,
            WsEvent::ChannelMessageNew {
                message: item.clone(),
            },
        )
        .await;

    Ok(Json(item))
}


#[derive(serde::Serialize)]
struct ServerSettings {
    is_public: bool,
    public_index: bool,
    verification_status: String,
    banner_url: Option<String>,
    is_owner: bool,
}

async fn handle_get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
) -> AppResult<Json<ServerSettings>> {
    let uid = crate::privacy::auth_user(&state, &headers).await?;
    let r = sqlx::query!(
        r#"SELECT s.owner_id, s.is_public, s.public_index, s.verification_status, s.banner_url
           FROM servers s JOIN server_members m ON m.server_id = s.id AND m.user_id = $2
           WHERE s.id = $1"#,
        server_id,
        uid
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Forbidden("not_a_member"))?;
    Ok(Json(ServerSettings {
        is_public: r.is_public,
        public_index: r.public_index,
        verification_status: r.verification_status,
        banner_url: r.banner_url,
        is_owner: r.owner_id == uid,
    }))
}

#[derive(serde::Deserialize)]
struct UpdateServerSettings {
    is_public: Option<bool>,
    public_index: Option<bool>,
    banner_url: Option<String>,
}

async fn handle_update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<Uuid>,
    Json(p): Json<UpdateServerSettings>,
) -> AppResult<Json<ServerSettings>> {
    let uid = crate::privacy::auth_user(&state, &headers).await?;
    let owner = sqlx::query_scalar!("SELECT owner_id FROM servers WHERE id = $1", server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("server_not_found"))?;
    if owner != uid {
        return Err(AppError::Forbidden("owner_only"));
    }
    if let Some(ref b) = p.banner_url {
        if !b.is_empty() && (!b.starts_with("/uploads/media/") || b.contains("..")) {
            return Err(AppError::BadRequest("invalid_banner_url"));
        }
    }
    sqlx::query!(
        r#"UPDATE servers SET
            is_public = COALESCE($1, is_public),
            verification_status = CASE
                WHEN $1 IS TRUE AND verification_status IN ('none', 'rejected') THEN 'pending'
                WHEN $1 IS FALSE THEN 'none'
                ELSE verification_status END,
            public_index = COALESCE($2, public_index),
            banner_url = CASE WHEN $3::text IS NULL THEN banner_url WHEN $3 = '' THEN NULL ELSE $3 END
        WHERE id = $4"#,
        p.is_public,
        p.public_index,
        p.banner_url,
        server_id
    )
    .execute(&state.db)
    .await?;
    if let Ok(t) = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized) {
        if let Ok(u) = authenticate_session(&state.db, &state.config, t).await {
            crate::community::audit(&state, server_id, u.id, "settings_updated", "").await;
        }
    }
    handle_get_settings(State(state), headers, Path(server_id)).await
}

/// Card shown before accepting a community invite link.
async fn handle_invite_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = crate::privacy::auth_user(&state, &headers).await?;
    let r = sqlx::query!(
        r#"SELECT s.id, s.name, s.description, s.icon_url, s.banner_url,
                  (SELECT COUNT(*) FROM server_members WHERE server_id = s.id) AS "members!",
                  EXISTS (SELECT 1 FROM server_members WHERE server_id = s.id AND user_id = $2) AS "is_member!"
           FROM server_invites i JOIN servers s ON s.id = i.server_id
           WHERE i.code = $1"#,
        code,
        uid
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("invite_not_found"))?;
    Ok(Json(serde_json::json!({
        "kind": "server", "id": r.id, "name": r.name, "description": r.description, "avatar_url": r.icon_url,
        "banner_url": r.banner_url, "member_count": r.members, "require_approval": false, "is_member": r.is_member,
    })))
}
