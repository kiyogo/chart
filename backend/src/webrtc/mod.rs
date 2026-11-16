use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    servers::permissions,
    websocket::WsEvent,
};
use chatw_core::{
    UpdateVoiceStateRequest, VoiceStateItem, WebRtcSignalPayload, permissions as perm_flags,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/signal", post(handle_webrtc_signal))
        .route(
            "/voice/{channel_id}/join",
            post(handle_join_voice_channel),
        )
        .route(
            "/voice/{channel_id}/leave",
            post(handle_leave_voice_channel),
        )
        .route(
            "/voice/{channel_id}/participants",
            get(handle_get_voice_participants),
        )
        .route(
            "/voice/{channel_id}/state",
            post(handle_update_voice_state),
        )
}

async fn handle_webrtc_signal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<WebRtcSignalPayload>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let event = WsEvent::WebRtcSignal {
        sender_id: user.id,
        channel_id: payload.channel_id,
        signal_type: payload.signal_type.clone(),
        data: payload.data,
    };

    if let Some(target_uid) = payload.target_user_id {
        // Direct P2P WebRTC Signal
        state.ws_hub.send_to_user(&target_uid, event).await;
    } else if let Some(ch_id) = payload.channel_id {
        // Voice Channel WebRTC Signal -> relay to all participants in voice channel
        let participants = sqlx::query_scalar!(
            "SELECT user_id FROM voice_states WHERE channel_id = $1 AND user_id != $2",
            ch_id,
            user.id
        )
        .fetch_all(&state.db)
        .await?;

        state.ws_hub.send_to_users(&participants, event).await;
    } else {
        return Err(AppError::BadRequest("missing_target_or_channel"));
    }

    Ok(Json(serde_json::json!({ "relayed": true })))
}

async fn handle_join_voice_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
) -> AppResult<Json<Vec<VoiceStateItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let ch = sqlx::query!(
        "SELECT server_id, channel_type FROM channels WHERE id = $1",
        channel_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("channel_not_found"))?;

    if ch.channel_type != "voice" {
        return Err(AppError::BadRequest("not_a_voice_channel"));
    }

    permissions::require_server_permission(
        &state.db,
        ch.server_id,
        user.id,
        perm_flags::VIEW_CHANNELS,
    )
    .await?;

    let session_id = format!("v_{}_{}", user.id, chrono::Utc::now().timestamp_millis());

    // Insert or update voice state
    sqlx::query!(
        r#"
        INSERT INTO voice_states (channel_id, user_id, session_id, self_mute, self_deaf)
        VALUES ($1, $2, $3, FALSE, FALSE)
        ON CONFLICT (channel_id, user_id)
        DO UPDATE SET session_id = EXCLUDED.session_id, joined_at = now()
        "#,
        channel_id,
        user.id,
        session_id
    )
    .execute(&state.db)
    .await?;

    let current_state = VoiceStateItem {
        channel_id,
        user_id: user.id,
        username: user.username.clone().unwrap_or_default(),
        avatar_url: user.avatar_url.clone(),
        self_mute: false,
        self_deaf: false,
    };

    // Broadcast voice state update to all server members
    let member_ids = sqlx::query_scalar!(
        "SELECT user_id FROM server_members WHERE server_id = $1",
        ch.server_id
    )
    .fetch_all(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_users(&member_ids, WsEvent::VoiceStateUpdate(current_state))
        .await;

    // Fetch all current participants in the channel
    let participants = fetch_channel_participants(&state.db, channel_id).await?;
    Ok(Json(participants))
}

async fn handle_leave_voice_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    sqlx::query!(
        "DELETE FROM voice_states WHERE channel_id = $1 AND user_id = $2",
        channel_id,
        user.id
    )
    .execute(&state.db)
    .await?;

    let ch = sqlx::query!("SELECT server_id FROM channels WHERE id = $1", channel_id)
        .fetch_optional(&state.db)
        .await?;

    if let Some(c) = ch {
        let member_ids = sqlx::query_scalar!(
            "SELECT user_id FROM server_members WHERE server_id = $1",
            c.server_id
        )
        .fetch_all(&state.db)
        .await?;

        // Broadcast leaving state (self_mute = true & self_deaf = true indicates removal or client handles empty)
        let leave_state = VoiceStateItem {
            channel_id,
            user_id: user.id,
            username: user.username.unwrap_or_default(),
            avatar_url: user.avatar_url,
            self_mute: false,
            self_deaf: false,
        };
        state
            .ws_hub
            .send_to_users(&member_ids, WsEvent::VoiceStateUpdate(leave_state))
            .await;
    }

    Ok(Json(serde_json::json!({ "left": true })))
}

async fn handle_get_voice_participants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
) -> AppResult<Json<Vec<VoiceStateItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let ch = sqlx::query!("SELECT server_id FROM channels WHERE id = $1", channel_id)
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

    let participants = fetch_channel_participants(&state.db, channel_id).await?;
    Ok(Json(participants))
}

async fn handle_update_voice_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<UpdateVoiceStateRequest>,
) -> AppResult<Json<VoiceStateItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let ch = sqlx::query!("SELECT server_id FROM channels WHERE id = $1", channel_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("channel_not_found"))?;

    let row = sqlx::query!(
        r#"
        UPDATE voice_states
        SET self_mute = COALESCE($3, self_mute),
            self_deaf = COALESCE($4, self_deaf)
        WHERE channel_id = $1 AND user_id = $2
        RETURNING self_mute, self_deaf
        "#,
        channel_id,
        user.id,
        payload.self_mute,
        payload.self_deaf
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("not_in_voice_channel"))?;

    let updated = VoiceStateItem {
        channel_id,
        user_id: user.id,
        username: user.username.unwrap_or_default(),
        avatar_url: user.avatar_url,
        self_mute: row.self_mute,
        self_deaf: row.self_deaf,
    };

    let member_ids = sqlx::query_scalar!(
        "SELECT user_id FROM server_members WHERE server_id = $1",
        ch.server_id
    )
    .fetch_all(&state.db)
    .await?;

    state
        .ws_hub
        .send_to_users(&member_ids, WsEvent::VoiceStateUpdate(updated.clone()))
        .await;

    Ok(Json(updated))
}

async fn fetch_channel_participants(
    db: &sqlx::PgPool,
    channel_id: Uuid,
) -> AppResult<Vec<VoiceStateItem>> {
    let rows = sqlx::query!(
        r#"
        SELECT v.channel_id, v.user_id, u.username, u.avatar_url, v.self_mute, v.self_deaf
        FROM voice_states v
        JOIN users u ON v.user_id = u.id
        WHERE v.channel_id = $1
        ORDER BY v.joined_at ASC
        "#,
        channel_id
    )
    .fetch_all(db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| VoiceStateItem {
            channel_id: r.channel_id,
            user_id: r.user_id,
            username: r.username.unwrap_or_default(),
            avatar_url: r.avatar_url,
            self_mute: r.self_mute,
            self_deaf: r.self_deaf,
        })
        .collect();

    Ok(items)
}

