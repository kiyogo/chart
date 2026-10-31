use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
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

#[derive(Deserialize)]
pub struct TypingPayload {
    pub friend_id: Uuid,
    pub is_typing: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/", post(handle_typing))
}

async fn handle_typing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TypingPayload>,
) -> AppResult<StatusCode> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let friend_id = payload.friend_id;
    if user.id == friend_id {
        return Err(AppError::BadRequest("cannot_type_to_self"));
    }

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

    let event = if payload.is_typing {
        WsEvent::TypingStart { user_id: user.id }
    } else {
        WsEvent::TypingStop { user_id: user.id }
    };

    state.ws_hub.send_to_user(&friend_id, event).await;

    Ok(StatusCode::NO_CONTENT)
}
