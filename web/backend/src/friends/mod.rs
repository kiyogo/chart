use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    websocket::WsEvent,
};

#[derive(Deserialize)]
pub struct SendRequestPayload {
    pub receiver_id: Option<Uuid>,
    pub username: Option<String>,
}

#[derive(Serialize)]
pub struct RequestResponse {
    pub id: Uuid,
    pub status: String,
}

#[derive(Serialize)]
pub struct PendingRequestItem {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub sender_display_name: Option<String>,
    pub sender_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct SentRequestItem {
    pub id: Uuid,
    pub receiver_id: Uuid,
    pub receiver_username: String,
    pub receiver_display_name: Option<String>,
    pub receiver_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateTargetInvitePayload {
    pub kind: String,
    pub target_id: Uuid,
    pub code: String,
    pub invitee_id: Uuid,
}

#[derive(Serialize)]
pub struct DirectTargetInviteItem {
    pub id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub code: String,
    pub target_name: String,
    pub target_avatar_url: Option<String>,
    pub member_count: i64,
    pub peer_id: Uuid,
    pub peer_username: String,
    pub peer_display_name: Option<String>,
    pub peer_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct DirectTargetInvitesResponse {
    pub received: Vec<DirectTargetInviteItem>,
    pub sent: Vec<DirectTargetInviteItem>,
}

#[derive(Serialize)]
pub struct FriendItem {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    /// Public profile flair shown in lists (nameplate, name effect, status).
    pub display_name: Option<String>,
    pub nameplate: String,
    pub custom_nameplate_url: Option<String>,
    pub name_effect: String,
    pub profile_color: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub presence: String,
    /// Avatar decoration and validated avatar motion (spec 009).
    pub avatar_decoration: Option<String>,
    pub avatar_motion: Option<crate::profile_ext::AvatarMotion>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/requests",
            post(handle_send_request).get(handle_list_requests),
        )
        .route("/requests/sent", get(handle_list_sent_requests))
        .route("/requests/{id}", delete(handle_delete_request))
        .route("/requests/{id}/accept", post(handle_accept_request))
        .route(
            "/target-invites",
            get(handle_list_target_invites).post(handle_create_target_invite),
        )
        .route(
            "/target-invites/{id}/accept",
            post(handle_accept_target_invite),
        )
        .route("/target-invites/{id}", delete(handle_delete_target_invite))
        .route("/{friend_id}", delete(handle_remove_friend))
        .route("/", get(handle_list_friends))
}

async fn handle_send_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendRequestPayload>,
) -> AppResult<Json<RequestResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let sender = authenticate_session(&state.db, &state.config, session_token).await?;

    let (receiver_id, receiver_username) = if let Some(raw_uname) = payload.username {
        let trimmed = raw_uname.trim();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("empty_username"));
        }
        let norm: String = trimmed.to_lowercase().nfkc().collect();
        let rec = sqlx::query!(
            "SELECT id, username FROM users WHERE username_norm = $1 AND username IS NOT NULL",
            norm
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("receiver_not_found"))?;
        (rec.id, rec.username)
    } else if let Some(rid) = payload.receiver_id {
        let rec = sqlx::query!(
            "SELECT id, username FROM users WHERE id = $1 AND username IS NOT NULL",
            rid
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("receiver_not_found"))?;
        (rec.id, rec.username)
    } else {
        return Err(AppError::BadRequest("missing_receiver"));
    };

    if sender.id == receiver_id {
        return Err(AppError::BadRequest("cannot_friend_self"));
    }
    if crate::privacy::is_blocked_either(&state.db, sender.id, receiver_id).await? {
        // Same answer as "not found" so a block is not revealed.
        return Err(AppError::NotFound("receiver_not_found"));
    }
    let accepts = sqlx::query_scalar!("SELECT privacy_friend_requests FROM users WHERE id = $1", receiver_id)
        .fetch_one(&state.db)
        .await?;
    if accepts == "nobody" {
        return Err(AppError::Forbidden("friend_requests_disabled"));
    }

    let u_a = sender.id.min(receiver_id);
    let u_b = sender.id.max(receiver_id);

    // 1. Check if already friends
    let already_friends = sqlx::query_scalar!(
        r#"SELECT 1 as "one!" FROM friendships WHERE user_a = $1 AND user_b = $2"#,
        u_a,
        u_b
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();

    if already_friends {
        return Err(AppError::Conflict("already_friends"));
    }

    let mut tx = state.db.begin().await?;

    // 2. Check if receiver already sent a pending request to sender -> mutual auto-accept!
    let reciprocal = sqlx::query!(
        "SELECT id FROM friend_requests WHERE sender_id = $1 AND receiver_id = $2 AND status = 'pending'",
        receiver_id,
        sender.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(req) = reciprocal {
        sqlx::query!(
            "UPDATE friend_requests SET status = 'accepted' WHERE id = $1",
            req.id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            u_a,
            u_b
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Notify both parties via WebSocket
        let sender_username = sender.username.clone().unwrap_or_default();
        let receiver_username_str = receiver_username.unwrap_or_default();

        state
            .ws_hub
            .send_to_user(
                &sender.id,
                WsEvent::FriendAccepted {
                    user_id: receiver_id,
                    username: receiver_username_str.clone(),
                },
            )
            .await;

        state
            .ws_hub
            .send_to_user(
                &receiver_id,
                WsEvent::FriendAccepted {
                    user_id: sender.id,
                    username: sender_username,
                },
            )
            .await;

        return Ok(Json(RequestResponse {
            id: req.id,
            status: "accepted".into(),
        }));
    }

    // 3. Otherwise create new request
    let request_id = Uuid::new_v4();
    let res = sqlx::query!(
        r#"
        INSERT INTO friend_requests (id, sender_id, receiver_id, status)
        VALUES ($1, $2, $3, 'pending')
        ON CONFLICT (sender_id, receiver_id) DO NOTHING
        "#,
        request_id,
        sender.id,
        receiver_id
    )
    .execute(&mut *tx)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::Conflict("request_already_sent"));
    }

    tx.commit().await?;

    // Send WebSocket event to receiver
    state
        .ws_hub
        .send_to_user(
            &receiver_id,
            WsEvent::FriendRequest {
                id: request_id,
                sender_id: sender.id,
                sender_username: sender.username.unwrap_or_default(),
                created_at: Utc::now(),
            },
        )
        .await;

    Ok(Json(RequestResponse {
        id: request_id,
        status: "pending".into(),
    }))
}

async fn handle_list_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<PendingRequestItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query(
        r#"
        SELECT fr.id, fr.sender_id, u.username, u.display_name, u.avatar_url, fr.created_at
        FROM friend_requests fr
        JOIN users u ON u.id = fr.sender_id
        WHERE fr.receiver_id = $1 AND fr.status = 'pending'
        ORDER BY fr.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| PendingRequestItem {
            id: r.get("id"),
            sender_id: r.get("sender_id"),
            sender_username: r.get::<Option<String>, _>("username").unwrap_or_default(),
            sender_display_name: r.get("display_name"),
            sender_avatar_url: r.get("avatar_url"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(items))
}

async fn handle_list_sent_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SentRequestItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query(
        r#"
        SELECT fr.id, fr.receiver_id, u.username, u.display_name, u.avatar_url, fr.created_at
        FROM friend_requests fr
        JOIN users u ON u.id = fr.receiver_id
        WHERE fr.sender_id = $1 AND fr.status = 'pending'
        ORDER BY fr.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| SentRequestItem {
            id: r.get("id"),
            receiver_id: r.get("receiver_id"),
            receiver_username: r.get::<Option<String>, _>("username").unwrap_or_default(),
            receiver_display_name: r.get("display_name"),
            receiver_avatar_url: r.get("avatar_url"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(items))
}

/// Sender withdraws a sent friend request OR receiver declines a received friend request.
async fn handle_delete_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> AppResult<Json<RequestResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let res = sqlx::query(
        r#"
        DELETE FROM friend_requests
        WHERE id = $1 AND status = 'pending' AND (sender_id = $2 OR receiver_id = $2)
        "#,
    )
    .bind(request_id)
    .bind(user.id)
    .execute(&state.db)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("request_not_found"));
    }

    Ok(Json(RequestResponse {
        id: request_id,
        status: "removed".into(),
    }))
}

/// Remove an existing friend from both users' contact lists.
async fn handle_remove_friend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friend_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let u_a = user.id.min(friend_id);
    let u_b = user.id.max(friend_id);

    let mut tx = state.db.begin().await?;
    let del = sqlx::query("DELETE FROM friendships WHERE user_a = $1 AND user_b = $2")
        .bind(u_a)
        .bind(u_b)
        .execute(&mut *tx)
        .await?;

    if del.rows_affected() == 0 {
        return Err(AppError::NotFound("friendship_not_found"));
    }

    sqlx::query(
        "DELETE FROM friend_requests WHERE (sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1)",
    )
    .bind(user.id)
    .bind(friend_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true, "friend_id": friend_id })))
}

async fn handle_create_target_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTargetInvitePayload>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let kind = payload.kind.trim();
    if kind != "group" && kind != "server" {
        return Err(AppError::BadRequest("invalid_invite_kind"));
    }
    let code = payload.code.trim();
    if code.is_empty() || code.len() > 64 {
        return Err(AppError::BadRequest("invalid_invite_code"));
    }
    if payload.invitee_id == user.id {
        return Err(AppError::BadRequest("cannot_invite_self"));
    }
    if crate::privacy::is_blocked_either(&state.db, user.id, payload.invitee_id).await? {
        return Err(AppError::NotFound("invitee_not_found"));
    }

    // Verify inviter membership and invitee is not already a member
    if kind == "group" {
        let is_mem: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
        )
        .bind(payload.target_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?;
        if is_mem.is_none() {
            return Err(AppError::Forbidden("not_a_group_member"));
        }
        let already: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
        )
        .bind(payload.target_id)
        .bind(payload.invitee_id)
        .fetch_optional(&state.db)
        .await?;
        if already.is_some() {
            return Ok(Json(serde_json::json!({ "ok": true, "status": "already_member" })));
        }
    } else {
        let is_mem: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
        )
        .bind(payload.target_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?;
        if is_mem.is_none() {
            return Err(AppError::Forbidden("not_a_server_member"));
        }
        let already: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
        )
        .bind(payload.target_id)
        .bind(payload.invitee_id)
        .fetch_optional(&state.db)
        .await?;
        if already.is_some() {
            return Ok(Json(serde_json::json!({ "ok": true, "status": "already_member" })));
        }
    }

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO direct_target_invites (id, kind, target_id, code, inviter_id, invitee_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending')
        ON CONFLICT (kind, target_id, invitee_id)
        DO UPDATE SET
            code = EXCLUDED.code,
            inviter_id = EXCLUDED.inviter_id,
            status = 'pending',
            created_at = now()
        "#,
    )
    .bind(id)
    .bind(kind)
    .bind(payload.target_id)
    .bind(code)
    .bind(user.id)
    .bind(payload.invitee_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true, "id": id, "status": "pending" })))
}

async fn handle_list_target_invites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<DirectTargetInvitesResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let recv_rows = sqlx::query(
        r#"
        SELECT
            d.id, d.kind, d.target_id, d.code, d.created_at,
            CASE WHEN d.kind = 'group' THEN g.name ELSE s.name END AS target_name,
            CASE WHEN d.kind = 'group' THEN g.avatar_url ELSE s.icon_url END AS target_avatar_url,
            CASE
                WHEN d.kind = 'group' THEN (SELECT COUNT(*) FROM group_members gm WHERE gm.group_id = d.target_id)
                ELSE (SELECT COUNT(*) FROM server_members sm WHERE sm.server_id = d.target_id)
            END AS member_count,
            u.id AS peer_id,
            u.username AS peer_username,
            u.display_name AS peer_display_name,
            u.avatar_url AS peer_avatar_url
        FROM direct_target_invites d
        JOIN users u ON u.id = d.inviter_id
        LEFT JOIN groups g ON d.kind = 'group' AND g.id = d.target_id
        LEFT JOIN servers s ON d.kind = 'server' AND s.id = d.target_id
        WHERE d.invitee_id = $1
          AND d.status = 'pending'
          AND ((d.kind = 'group' AND g.id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM group_members gm WHERE gm.group_id = d.target_id AND gm.user_id = $1))
            OR (d.kind = 'server' AND s.id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM server_members sm WHERE sm.server_id = d.target_id AND sm.user_id = $1)))
        ORDER BY d.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let sent_rows = sqlx::query(
        r#"
        SELECT
            d.id, d.kind, d.target_id, d.code, d.created_at,
            CASE WHEN d.kind = 'group' THEN g.name ELSE s.name END AS target_name,
            CASE WHEN d.kind = 'group' THEN g.avatar_url ELSE s.icon_url END AS target_avatar_url,
            CASE
                WHEN d.kind = 'group' THEN (SELECT COUNT(*) FROM group_members gm WHERE gm.group_id = d.target_id)
                ELSE (SELECT COUNT(*) FROM server_members sm WHERE sm.server_id = d.target_id)
            END AS member_count,
            u.id AS peer_id,
            u.username AS peer_username,
            u.display_name AS peer_display_name,
            u.avatar_url AS peer_avatar_url
        FROM direct_target_invites d
        JOIN users u ON u.id = d.invitee_id
        LEFT JOIN groups g ON d.kind = 'group' AND g.id = d.target_id
        LEFT JOIN servers s ON d.kind = 'server' AND s.id = d.target_id
        WHERE d.inviter_id = $1
          AND d.status = 'pending'
          AND ((d.kind = 'group' AND g.id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM group_members gm WHERE gm.group_id = d.target_id AND gm.user_id = d.invitee_id))
            OR (d.kind = 'server' AND s.id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM server_members sm WHERE sm.server_id = d.target_id AND sm.user_id = d.invitee_id)))
        ORDER BY d.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let map_row = |r: sqlx::postgres::PgRow| DirectTargetInviteItem {
        id: r.get("id"),
        kind: r.get("kind"),
        target_id: r.get("target_id"),
        code: r.get("code"),
        target_name: r.get::<Option<String>, _>("target_name").unwrap_or_default(),
        target_avatar_url: r.get("target_avatar_url"),
        member_count: r.get::<Option<i64>, _>("member_count").unwrap_or(0),
        peer_id: r.get("peer_id"),
        peer_username: r.get::<Option<String>, _>("peer_username").unwrap_or_default(),
        peer_display_name: r.get("peer_display_name"),
        peer_avatar_url: r.get("peer_avatar_url"),
        created_at: r.get("created_at"),
    };

    Ok(Json(DirectTargetInvitesResponse {
        received: recv_rows.into_iter().map(map_row).collect(),
        sent: sent_rows.into_iter().map(map_row).collect(),
    }))
}

async fn handle_accept_target_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(invite_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query(
        r#"
        SELECT kind, target_id, code, invitee_id, status
        FROM direct_target_invites
        WHERE id = $1
        "#,
    )
    .bind(invite_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("invite_not_found"))?;

    let invitee_id: Uuid = row.get("invitee_id");
    let status: String = row.get("status");
    let kind: String = row.get("kind");
    let target_id: Uuid = row.get("target_id");

    if invitee_id != user.id {
        return Err(AppError::Forbidden("not_invitee"));
    }
    if status != "pending" {
        return Err(AppError::Conflict("invite_not_pending"));
    }

    if kind == "group" {
        let blocked: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM group_blocks WHERE user_id = $1 AND group_id = $2",
        )
        .bind(user.id)
        .bind(target_id)
        .fetch_optional(&state.db)
        .await?;
        if blocked.is_some() {
            return Err(AppError::Forbidden("blocked_from_group"));
        }
        sqlx::query(
            "INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
        )
        .bind(target_id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(target_id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    }

    sqlx::query("UPDATE direct_target_invites SET status = 'accepted' WHERE id = $1")
        .bind(invite_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "status": "accepted",
        "kind": kind,
        "target_id": target_id
    })))
}

async fn handle_delete_target_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(invite_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let res = sqlx::query(
        r#"
        DELETE FROM direct_target_invites
        WHERE id = $1 AND status = 'pending' AND (inviter_id = $2 OR invitee_id = $2)
        "#,
    )
    .bind(invite_id)
    .bind(user.id)
    .execute(&state.db)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("invite_not_found"));
    }

    Ok(Json(serde_json::json!({ "ok": true, "id": invite_id, "status": "removed" })))
}

async fn handle_accept_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> AppResult<Json<RequestResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let mut tx = state.db.begin().await?;

    let req = sqlx::query!(
        r#"
        SELECT fr.sender_id, fr.receiver_id, fr.status, u.username as sender_username
        FROM friend_requests fr
        JOIN users u ON u.id = fr.sender_id
        WHERE fr.id = $1
        FOR UPDATE
        "#,
        request_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("request_not_found"))?;

    // Authorization: only the intended receiver can accept
    if req.receiver_id != user.id {
        return Err(AppError::Forbidden("not_recipient"));
    }

    if req.status != "pending" {
        return Err(AppError::Conflict("request_not_pending"));
    }

    // Mark as accepted
    sqlx::query!(
        "UPDATE friend_requests SET status = 'accepted' WHERE id = $1",
        request_id
    )
    .execute(&mut *tx)
    .await?;

    // Insert friendship
    let u_a = req.sender_id.min(user.id);
    let u_b = req.sender_id.max(user.id);

    sqlx::query!(
        "INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        u_a,
        u_b
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let sender_username = req.sender_username.unwrap_or_default();
    let current_username = user.username.unwrap_or_default();

    // Broadcast WebSocket events to both users
    state
        .ws_hub
        .send_to_user(
            &user.id,
            WsEvent::FriendAccepted {
                user_id: req.sender_id,
                username: sender_username.clone(),
            },
        )
        .await;

    state
        .ws_hub
        .send_to_user(
            &req.sender_id,
            WsEvent::FriendAccepted {
                user_id: user.id,
                username: current_username,
            },
        )
        .await;

    Ok(Json(RequestResponse {
        id: request_id,
        status: "accepted".into(),
    }))
}

async fn handle_list_friends(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<FriendItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT
            CASE WHEN f.user_a = $1 THEN f.user_b ELSE f.user_a END AS "friend_id!",
            u.username as "username!",
            u.avatar_url as "avatar_url",
            u.display_name, u.nameplate, u.custom_nameplate_url, u.name_effect, u.profile_color,
            u.status_text, u.status_emoji, u.presence, u.avatar_decoration, u.avatar_motion
        FROM friendships f
        JOIN users u ON u.id = (CASE WHEN f.user_a = $1 THEN f.user_b ELSE f.user_a END)
        WHERE f.user_a = $1 OR f.user_b = $1
        ORDER BY u.username_norm ASC
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let friends = rows
        .into_iter()
        .map(|r| FriendItem {
            id: r.friend_id,
            username: r.username,
            avatar_url: r.avatar_url,
            display_name: r.display_name,
            nameplate: r.nameplate,
            custom_nameplate_url: r.custom_nameplate_url,
            name_effect: r.name_effect,
            profile_color: r.profile_color,
            status_text: r.status_text,
            status_emoji: r.status_emoji,
            presence: r.presence,
            avatar_decoration: Some(r.avatar_decoration),
            avatar_motion: crate::profile_ext::parse_motion(r.avatar_motion.as_ref()),
        })
        .collect();

    Ok(Json(friends))
}
