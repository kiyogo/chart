use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    crypto::Envelope,
    error::{AppError, AppResult},
    websocket::WsEvent,
};

#[derive(Deserialize)]
pub struct SendMessagePayload {
    pub message_id: Uuid,
    pub recipient_id: Uuid,
    pub recipient_envelope: String, // base64url
    pub sender_envelope: String,    // base64url
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    pub id: Uuid,
    pub seq: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub before_seq: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct MessageItem {
    pub id: Uuid,
    pub seq: i64,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub envelope: String, // base64url
    pub created_at: DateTime<Utc>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_send_message))
        .route("/{friendId}", get(handle_get_messages))
}

async fn handle_send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendMessagePayload>,
) -> AppResult<Json<SendMessageResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let sender = authenticate_session(&state.db, &state.config, session_token).await?;

    let recipient_id = payload.recipient_id;
    if sender.id == recipient_id {
        return Err(AppError::BadRequest("cannot_message_self"));
    }

    // 1. Verify friendship
    let u_a = sender.id.min(recipient_id);
    let u_b = sender.id.max(recipient_id);
    let is_friend = sqlx::query_scalar!(
        r#"SELECT 1 as "one!" FROM friendships WHERE user_a = $1 AND user_b = $2"#,
        u_a,
        u_b
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();

    if !is_friend {
        return Err(AppError::Forbidden("not_friends"));
    }

    // 2. Decode raw envelope bytes
    let recipient_env_raw = URL_SAFE_NO_PAD
        .decode(&payload.recipient_envelope)
        .map_err(|_| AppError::BadRequest("invalid_recipient_envelope_base64"))?;
    let sender_env_raw = URL_SAFE_NO_PAD
        .decode(&payload.sender_envelope)
        .map_err(|_| AppError::BadRequest("invalid_sender_envelope_base64"))?;

    // 3. Parse envelopes
    let recipient_env = Envelope::parse(&recipient_env_raw)?;
    let sender_env = Envelope::parse(&sender_env_raw)?;

    // 4. Verify message_id consistency
    if recipient_env.message_id != payload.message_id || sender_env.message_id != payload.message_id
    {
        return Err(AppError::BadRequest("message_id_mismatch"));
    }

    // 5. Verify sender consistency
    if recipient_env.sender_id != sender.id || sender_env.sender_id != sender.id {
        return Err(AppError::BadRequest("sender_id_mismatch"));
    }

    // 6. Verify recipients
    if recipient_env.recipient_id != recipient_id {
        return Err(AppError::BadRequest("recipient_id_mismatch"));
    }
    if sender_env.recipient_id != sender.id {
        return Err(AppError::BadRequest(
            "sender_envelope_recipient_must_be_sender",
        ));
    }

    // 7. Look up recipient's registered device key
    let recipient_keys = sqlx::query!(
        "SELECT fingerprint FROM device_keys WHERE user_id = $1",
        recipient_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("recipient_has_no_keys"))?;

    if recipient_env.recipient_key_fingerprint.as_slice() != recipient_keys.fingerprint.as_slice() {
        return Err(AppError::BadRequest("recipient_fingerprint_mismatch"));
    }

    // 8. Look up sender's registered device key
    let sender_keys = sqlx::query!(
        "SELECT ecdsa_public, fingerprint FROM device_keys WHERE user_id = $1",
        sender.id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("sender_has_no_keys"))?;

    if sender_env.recipient_key_fingerprint.as_slice() != sender_keys.fingerprint.as_slice() {
        return Err(AppError::BadRequest("sender_fingerprint_mismatch"));
    }

    // 9. Verify sender signatures on BOTH envelopes
    recipient_env.verify_signature(&sender_keys.ecdsa_public)?;
    sender_env.verify_signature(&sender_keys.ecdsa_public)?;

    // 10. Insert message into PostgreSQL (idempotent on message_id)
    let row = sqlx::query!(
        r#"
        INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (id) DO UPDATE SET id = EXCLUDED.id
        RETURNING seq, created_at
        "#,
        payload.message_id,
        sender.id,
        recipient_id,
        &recipient_env_raw[..],
        &sender_env_raw[..]
    )
    .fetch_one(&state.db)
    .await?;

    // 11. Send real-time event to recipient via WebSocket
    state
        .ws_hub
        .send_to_user(
            &recipient_id,
            WsEvent::MessageNew {
                id: payload.message_id,
                seq: row.seq,
                sender_id: sender.id,
                recipient_id,
                envelope: payload.recipient_envelope,
                created_at: row.created_at,
            },
        )
        .await;

    Ok(Json(SendMessageResponse {
        id: payload.message_id,
        seq: row.seq,
        created_at: row.created_at,
    }))
}

async fn handle_get_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friend_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<Vec<MessageItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // Verify friendship
    let u_a = user.id.min(friend_id);
    let u_b = user.id.max(friend_id);
    let is_friend = sqlx::query_scalar!(
        r#"SELECT 1 as "one!" FROM friendships WHERE user_a = $1 AND user_b = $2"#,
        u_a,
        u_b
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();

    if !is_friend {
        return Err(AppError::Forbidden("not_friends"));
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let before_seq = query.before_seq.unwrap_or(i64::MAX);

    let rows = sqlx::query!(
        r#"
        SELECT id, seq, sender_id, receiver_id, recipient_envelope, sender_envelope, created_at
        FROM messages
        WHERE ((sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1))
          AND seq < $3
        ORDER BY seq DESC
        LIMIT $4
        "#,
        user.id,
        friend_id,
        before_seq,
        limit
    )
    .fetch_all(&state.db)
    .await?;

    let messages = rows
        .into_iter()
        .map(|r| {
            // Return appropriate envelope for current user:
            // If current user is sender -> sender_envelope
            // If current user is receiver -> recipient_envelope
            let raw_env = if r.sender_id == user.id {
                r.sender_envelope
            } else {
                r.recipient_envelope
            };
            MessageItem {
                id: r.id,
                seq: r.seq,
                sender_id: r.sender_id,
                recipient_id: r.receiver_id,
                envelope: URL_SAFE_NO_PAD.encode(raw_env),
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(messages))
}
