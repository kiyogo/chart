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
    pub reply_to_id: Option<Uuid>,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ReactionSummaryItem {
    pub emoji: String,
    pub count: i64,
    pub users: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct MessageItem {
    pub id: Uuid,
    pub seq: i64,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub envelope: String, // base64url
    pub reply_to_id: Option<Uuid>,
    pub is_pinned: bool,
    pub is_edited: bool,
    pub deleted_for_all: bool,
    pub reactions: Vec<ReactionSummaryItem>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ReactionPayload {
    pub emoji: String,
}

#[derive(Deserialize)]
pub struct EditMessagePayload {
    pub recipient_envelope: String, // base64url
    pub sender_envelope: String,    // base64url
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_send_message))
        .route("/pinned/{id}", get(handle_get_pinned_messages))
        .route(
            "/{id}",
            get(handle_get_messages)
                .delete(handle_delete_message)
                .put(handle_edit_message),
        )
        .route("/{id}/reactions", post(handle_toggle_reaction).get(handle_get_reactions))
        .route("/{id}/pin", post(handle_pin_message).delete(handle_unpin_message))
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
        INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope, reply_to_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE SET id = EXCLUDED.id
        RETURNING seq, created_at
        "#,
        payload.message_id,
        sender.id,
        recipient_id,
        &recipient_env_raw[..],
        &sender_env_raw[..],
        payload.reply_to_id
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
                reply_to_id: payload.reply_to_id,
                created_at: row.created_at,
            },
        )
        .await;

    // Send real-time event to sender (for other active sessions / tabs of sender)
    state
        .ws_hub
        .send_to_user(
            &sender.id,
            WsEvent::MessageNew {
                id: payload.message_id,
                seq: row.seq,
                sender_id: sender.id,
                recipient_id,
                envelope: payload.sender_envelope,
                reply_to_id: payload.reply_to_id,
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
        SELECT id, seq, sender_id, receiver_id, recipient_envelope, sender_envelope, reply_to_id, is_pinned, is_edited, deleted_for_all, created_at
        FROM messages
        WHERE ((sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1))
          AND seq < $3
          AND deleted_for_all = FALSE
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

    let msg_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let reaction_rows = sqlx::query!(
        r#"
        SELECT message_id, emoji, user_id
        FROM message_reactions
        WHERE message_id = ANY($1)
        ORDER BY created_at ASC
        "#,
        &msg_ids[..]
    )
    .fetch_all(&state.db)
    .await?;

    let mut reactions_map: std::collections::HashMap<Uuid, Vec<ReactionSummaryItem>> =
        std::collections::HashMap::new();
    let mut intermediate: std::collections::HashMap<(Uuid, String), Vec<Uuid>> =
        std::collections::HashMap::new();
    for r in reaction_rows {
        intermediate
            .entry((r.message_id, r.emoji))
            .or_default()
            .push(r.user_id);
    }
    for ((msg_id, emoji), users) in intermediate {
        let count = users.len() as i64;
        reactions_map
            .entry(msg_id)
            .or_default()
            .push(ReactionSummaryItem {
                emoji,
                count,
                users,
            });
    }

    let messages = rows
        .into_iter()
        .map(|r| {
            let raw_env = if r.sender_id == user.id {
                r.sender_envelope
            } else {
                r.recipient_envelope
            };
            let reactions = reactions_map.remove(&r.id).unwrap_or_default();
            MessageItem {
                id: r.id,
                seq: r.seq,
                sender_id: r.sender_id,
                recipient_id: r.receiver_id,
                envelope: URL_SAFE_NO_PAD.encode(raw_env),
                reply_to_id: r.reply_to_id,
                is_pinned: r.is_pinned,
                is_edited: r.is_edited,
                deleted_for_all: r.deleted_for_all,
                reactions,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(messages))
}

async fn handle_toggle_reaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<ReactionPayload>,
) -> AppResult<Json<Vec<ReactionSummaryItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let emoji = payload.emoji.trim();
    if emoji.is_empty() || emoji.len() > 32 {
        return Err(AppError::BadRequest("invalid_emoji"));
    }

    let msg = sqlx::query!(
        "SELECT sender_id, receiver_id FROM messages WHERE id = $1",
        message_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id && msg.receiver_id != user.id {
        return Err(AppError::Forbidden("not_conversation_participant"));
    }

    let deleted = sqlx::query!(
        "DELETE FROM message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
        message_id,
        user.id,
        emoji
    )
    .execute(&state.db)
    .await?;

    let action = if deleted.rows_affected() > 0 {
        "removed"
    } else {
        sqlx::query!(
            "INSERT INTO message_reactions (message_id, user_id, emoji) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            message_id,
            user.id,
            emoji
        )
        .execute(&state.db)
        .await?;
        "added"
    };

    let event = WsEvent::ReactionUpdate {
        message_id,
        user_id: user.id,
        emoji: emoji.to_string(),
        action: action.to_string(),
    };
    state.ws_hub.send_to_user(&msg.sender_id, event.clone()).await;
    state.ws_hub.send_to_user(&msg.receiver_id, event).await;

    let current = fetch_reactions_for_message(&state.db, message_id).await?;
    Ok(Json(current))
}

async fn handle_get_reactions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> AppResult<Json<Vec<ReactionSummaryItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let reactions = fetch_reactions_for_message(&state.db, message_id).await?;
    Ok(Json(reactions))
}

async fn fetch_reactions_for_message(
    db: &sqlx::PgPool,
    message_id: Uuid,
) -> AppResult<Vec<ReactionSummaryItem>> {
    let rows = sqlx::query!(
        "SELECT emoji, user_id FROM message_reactions WHERE message_id = $1 ORDER BY created_at ASC",
        message_id
    )
    .fetch_all(db)
    .await?;

    let mut map: std::collections::HashMap<String, Vec<Uuid>> = std::collections::HashMap::new();
    for r in rows {
        map.entry(r.emoji).or_default().push(r.user_id);
    }
    let list = map
        .into_iter()
        .map(|(emoji, users)| ReactionSummaryItem {
            count: users.len() as i64,
            emoji,
            users,
        })
        .collect();
    Ok(list)
}

async fn handle_pin_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let msg = sqlx::query!(
        "SELECT sender_id, receiver_id FROM messages WHERE id = $1",
        message_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id && msg.receiver_id != user.id {
        return Err(AppError::Forbidden("not_conversation_participant"));
    }

    sqlx::query!(
        "UPDATE messages SET is_pinned = TRUE WHERE id = $1",
        message_id
    )
    .execute(&state.db)
    .await?;

    let event = WsEvent::MessagePinned {
        message_id,
        is_pinned: true,
    };
    state.ws_hub.send_to_user(&msg.sender_id, event.clone()).await;
    state.ws_hub.send_to_user(&msg.receiver_id, event).await;

    Ok(Json(serde_json::json!({ "success": true, "is_pinned": true })))
}

async fn handle_unpin_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let msg = sqlx::query!(
        "SELECT sender_id, receiver_id FROM messages WHERE id = $1",
        message_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id && msg.receiver_id != user.id {
        return Err(AppError::Forbidden("not_conversation_participant"));
    }

    sqlx::query!(
        "UPDATE messages SET is_pinned = FALSE WHERE id = $1",
        message_id
    )
    .execute(&state.db)
    .await?;

    let event = WsEvent::MessagePinned {
        message_id,
        is_pinned: false,
    };
    state.ws_hub.send_to_user(&msg.sender_id, event.clone()).await;
    state.ws_hub.send_to_user(&msg.receiver_id, event).await;

    Ok(Json(serde_json::json!({ "success": true, "is_pinned": false })))
}

async fn handle_delete_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let msg = sqlx::query!(
        "SELECT sender_id, receiver_id FROM messages WHERE id = $1",
        message_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id {
        return Err(AppError::Forbidden("only_sender_can_delete_for_all"));
    }

    sqlx::query!(
        "UPDATE messages SET deleted_for_all = TRUE WHERE id = $1",
        message_id
    )
    .execute(&state.db)
    .await?;

    let event = WsEvent::MessageDeleted { message_id };
    state.ws_hub.send_to_user(&msg.sender_id, event.clone()).await;
    state.ws_hub.send_to_user(&msg.receiver_id, event).await;

    Ok(Json(serde_json::json!({ "success": true, "deleted": true })))
}

async fn handle_edit_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<EditMessagePayload>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let msg = sqlx::query!(
        "SELECT sender_id, receiver_id, deleted_for_all FROM messages WHERE id = $1",
        message_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("message_not_found"))?;

    if msg.sender_id != user.id {
        return Err(AppError::Forbidden("only_sender_can_edit"));
    }
    if msg.deleted_for_all {
        return Err(AppError::BadRequest("cannot_edit_deleted_message"));
    }

    let recipient_env_raw = URL_SAFE_NO_PAD
        .decode(&payload.recipient_envelope)
        .map_err(|_| AppError::BadRequest("invalid_recipient_envelope_base64"))?;
    let sender_env_raw = URL_SAFE_NO_PAD
        .decode(&payload.sender_envelope)
        .map_err(|_| AppError::BadRequest("invalid_sender_envelope_base64"))?;

    let recipient_env = Envelope::parse(&recipient_env_raw)?;
    let sender_env = Envelope::parse(&sender_env_raw)?;

    if recipient_env.message_id != message_id || sender_env.message_id != message_id {
        return Err(AppError::BadRequest("message_id_mismatch"));
    }
    if recipient_env.sender_id != user.id || sender_env.sender_id != user.id {
        return Err(AppError::BadRequest("sender_id_mismatch"));
    }

    let sender_keys = sqlx::query!(
        "SELECT ecdsa_public FROM device_keys WHERE user_id = $1",
        user.id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("sender_has_no_keys"))?;

    recipient_env.verify_signature(&sender_keys.ecdsa_public)?;
    sender_env.verify_signature(&sender_keys.ecdsa_public)?;

    sqlx::query!(
        "UPDATE messages SET recipient_envelope = $1, sender_envelope = $2, is_edited = TRUE WHERE id = $3",
        recipient_env_raw,
        sender_env_raw,
        message_id
    )
    .execute(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_user(
            &msg.receiver_id,
            WsEvent::MessageEdited {
                message_id,
                envelope: payload.recipient_envelope,
            },
        )
        .await;
    state
        .ws_hub
        .send_to_user(
            &msg.sender_id,
            WsEvent::MessageEdited {
                message_id,
                envelope: payload.sender_envelope,
            },
        )
        .await;

    Ok(Json(serde_json::json!({ "success": true, "is_edited": true })))
}

async fn handle_get_pinned_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friend_id): Path<Uuid>,
) -> AppResult<Json<Vec<MessageItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, seq, sender_id, receiver_id, recipient_envelope, sender_envelope, reply_to_id, is_pinned, is_edited, deleted_for_all, created_at
        FROM messages
        WHERE ((sender_id = $1 AND receiver_id = $2) OR (sender_id = $2 AND receiver_id = $1))
          AND is_pinned = TRUE AND deleted_for_all = FALSE
        ORDER BY seq DESC
        LIMIT 50
        "#,
        user.id,
        friend_id
    )
    .fetch_all(&state.db)
    .await?;

    let msg_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let reaction_rows = sqlx::query!(
        r#"
        SELECT message_id, emoji, user_id
        FROM message_reactions
        WHERE message_id = ANY($1)
        ORDER BY created_at ASC
        "#,
        &msg_ids[..]
    )
    .fetch_all(&state.db)
    .await?;

    let mut reactions_map: std::collections::HashMap<Uuid, Vec<ReactionSummaryItem>> =
        std::collections::HashMap::new();
    let mut intermediate: std::collections::HashMap<(Uuid, String), Vec<Uuid>> =
        std::collections::HashMap::new();
    for r in reaction_rows {
        intermediate
            .entry((r.message_id, r.emoji))
            .or_default()
            .push(r.user_id);
    }
    for ((msg_id, emoji), users) in intermediate {
        let count = users.len() as i64;
        reactions_map
            .entry(msg_id)
            .or_default()
            .push(ReactionSummaryItem {
                emoji,
                count,
                users,
            });
    }

    let messages = rows
        .into_iter()
        .map(|r| {
            let raw_env = if r.sender_id == user.id {
                r.sender_envelope
            } else {
                r.recipient_envelope
            };
            let reactions = reactions_map.remove(&r.id).unwrap_or_default();
            MessageItem {
                id: r.id,
                seq: r.seq,
                sender_id: r.sender_id,
                recipient_id: r.receiver_id,
                envelope: URL_SAFE_NO_PAD.encode(raw_env),
                reply_to_id: r.reply_to_id,
                is_pinned: r.is_pinned,
                is_edited: r.is_edited,
                deleted_for_all: r.deleted_for_all,
                reactions,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(messages))
}
