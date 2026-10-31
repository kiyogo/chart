use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    pub sender_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct FriendItem {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/requests",
            post(handle_send_request).get(handle_list_requests),
        )
        .route("/requests/{id}/accept", post(handle_accept_request))
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

    let rows = sqlx::query!(
        r#"
        SELECT fr.id, fr.sender_id, u.username as "sender_username!", u.avatar_url as "sender_avatar_url", fr.created_at
        FROM friend_requests fr
        JOIN users u ON u.id = fr.sender_id
        WHERE fr.receiver_id = $1 AND fr.status = 'pending'
        ORDER BY fr.created_at DESC
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| PendingRequestItem {
            id: r.id,
            sender_id: r.sender_id,
            sender_username: r.sender_username,
            sender_avatar_url: r.sender_avatar_url,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(items))
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
            u.avatar_url as "avatar_url"
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
        })
        .collect();

    Ok(Json(friends))
}
