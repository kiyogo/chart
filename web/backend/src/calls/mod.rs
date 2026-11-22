pub mod sfu;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    models::{
        CallParticipantItem, CallSessionItem, CallSignalPayload, CreateMeetRequest,
        StartCallRequest, UpdateCallStateRequest,
    },
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/start", post(start_call))
        .route("/active", get(get_active_call))
        .route("/history", get(get_call_history))
        .route("/meet", post(create_meet))
        .route("/meet/list", get(list_meets))
        .route("/meet/{code}/join", post(join_meet))
        .route("/{id}/accept", post(accept_call))
        .route("/{id}/reject", post(reject_call))
        .route("/{id}/end", post(end_call))
        .route("/{id}/leave", post(leave_call))
        .route("/{id}/signal", post(relay_signal))
        .route("/{id}/state", post(update_state))
        .route("/{id}/sfu/join", post(sfu_join))
        .route("/{id}/sfu/leave", post(sfu_leave))
}

async fn ensure_meet_columns(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        ALTER TABLE call_sessions
        ADD COLUMN IF NOT EXISTS meet_code VARCHAR(32) UNIQUE,
        ADD COLUMN IF NOT EXISTS meet_title VARCHAR(120)
        "#,
    )
    .execute(db)
    .await?;
    Ok(())
}

fn generate_meet_code() -> String {
    let bytes = *Uuid::new_v4().as_bytes();
    let ch = |b: u8| ((b % 26) + b'a') as char;
    format!(
        "{}{}{}-{}{}{}{}-{}{}{}",
        ch(bytes[0]),
        ch(bytes[1]),
        ch(bytes[2]),
        ch(bytes[3]),
        ch(bytes[4]),
        ch(bytes[5]),
        ch(bytes[6]),
        ch(bytes[7]),
        ch(bytes[8]),
        ch(bytes[9]),
    )
}

fn normalize_meet_code(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    if let Some(idx) = s.find("meet=") {
        s = s[idx + 5..].to_string();
    } else if let Some(idx) = s.find("/meet/") {
        s = s[idx + 6..].to_string();
    } else if let Some(idx) = s.rfind('/') {
        s = s[idx + 1..].to_string();
    }
    if let Some(idx) = s.find(['?', '#', '&', ' ']) {
        s = s[..idx].to_string();
    }
    s.trim().to_string()
}

async fn create_meet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateMeetRequest>,
) -> AppResult<Json<CallSessionItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let _ = ensure_meet_columns(&state.db).await;

    let call_type = match payload.call_type.as_deref().unwrap_or("video") {
        "audio" => "audio".to_string(),
        _ => "video".to_string(),
    };
    let uname = user.username.clone().unwrap_or_else(|| "You".to_string());
    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(110).collect::<String>())
        .unwrap_or_else(|| format!("{uname}'s meeting"));

    let cid = Uuid::new_v4();
    let code = generate_meet_code();

    sqlx::query(
        r#"
        INSERT INTO call_sessions (id, call_type, caller_id, status, started_at, meet_code, meet_title)
        VALUES ($1, $2, $3, 'active', now(), $4, $5)
        "#,
    )
    .bind(cid)
    .bind(&call_type)
    .bind(user.id)
    .bind(&code)
    .bind(&title)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO call_participants (call_id, user_id, joined_at, muted, camera_off)
        VALUES ($1, $2, now(), false, false)
        ON CONFLICT (call_id, user_id)
        DO UPDATE SET left_at = NULL, joined_at = now()
        "#,
    )
    .bind(cid)
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let item = fetch_call_session(&state.db, cid).await?;
    Ok(Json(item))
}

async fn join_meet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_code): Path<String>,
) -> AppResult<Json<CallSessionItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let _ = ensure_meet_columns(&state.db).await;

    let code = normalize_meet_code(&raw_code);
    if code.is_empty() {
        return Err(AppError::BadRequest("invalid_meet_code"));
    }

    let found_id: Option<Uuid> = if let Ok(uuid_val) = Uuid::parse_str(&code) {
        sqlx::query_scalar("SELECT id FROM call_sessions WHERE id = $1")
            .bind(uuid_val)
            .fetch_optional(&state.db)
            .await?
    } else {
        sqlx::query_scalar("SELECT id FROM call_sessions WHERE LOWER(meet_code) = $1")
            .bind(&code)
            .fetch_optional(&state.db)
            .await?
    };

    let call_id = found_id.ok_or(AppError::NotFound("meet_not_found"))?;

    sqlx::query(
        r#"
        UPDATE call_sessions
        SET status = 'active',
            started_at = COALESCE(started_at, now()),
            ended_at = NULL
        WHERE id = $1
        "#,
    )
    .bind(call_id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO call_participants (call_id, user_id, joined_at, muted, camera_off)
        VALUES ($1, $2, now(), false, false)
        ON CONFLICT (call_id, user_id)
        DO UPDATE SET left_at = NULL, joined_at = now()
        "#,
    )
    .bind(call_id)
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let session_item = fetch_call_session(&state.db, call_id).await?;

    let participant_item = CallParticipantItem {
        call_id,
        user_id: user.id,
        username: user.username.unwrap_or_default(),
        avatar_url: user.avatar_url,
        joined_at: Utc::now(),
        left_at: None,
        muted: false,
        camera_off: false,
    };

    let target_users = get_other_participants(&state.db, call_id, user.id).await?;
    if !target_users.is_empty() {
        state
            .ws_hub
            .send_to_users(
                &target_users,
                WsEvent::CallParticipantJoined {
                    call_id,
                    participant: participant_item,
                },
            )
            .await;
    }

    Ok(Json(session_item))
}

async fn list_meets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<CallSessionItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let _ = ensure_meet_columns(&state.db).await;

    let ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM call_sessions c
        WHERE c.meet_code IS NOT NULL
          AND (
              c.caller_id = $1
              OR EXISTS (SELECT 1 FROM call_participants p WHERE p.call_id = c.id AND p.user_id = $1)
          )
        ORDER BY c.created_at DESC
        LIMIT 20
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let mut list = Vec::new();
    for id in ids {
        if let Ok(item) = fetch_call_session(&state.db, id).await {
            list.push(item);
        }
    }
    Ok(Json(list))
}

// 1. Start a call (1:1 or Group)
async fn start_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StartCallRequest>,
) -> AppResult<Json<CallSessionItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let call_type = match payload.call_type.as_str() {
        "audio" | "video" => payload.call_type.clone(),
        _ => return Err(AppError::BadRequest("invalid_call_type")),
    };

    if let Some(group_id) = payload.group_id {
        // Group Call
        // Check group membership
        let is_member = sqlx::query_scalar!(
            "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
            group_id,
            user.id
        )
        .fetch_optional(&state.db)
        .await?
        .is_some();

        if !is_member {
            return Err(AppError::Forbidden("not_a_group_member"));
        }

        // Check if there is already an active or ringing call for this group
        let existing = sqlx::query!(
            r#"
            SELECT id FROM call_sessions
            WHERE group_id = $1 AND status IN ('ringing', 'active')
            LIMIT 1
            "#,
            group_id
        )
        .fetch_optional(&state.db)
        .await?;

        let call_id = if let Some(ex) = existing {
            ex.id
        } else {
            let cid = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO call_sessions (id, call_type, caller_id, group_id, status, started_at)
                VALUES ($1, $2, $3, $4, 'active', now())
                "#,
                cid,
                call_type,
                user.id,
                group_id
            )
            .execute(&state.db)
            .await?;
            cid
        };

        // Add caller to participants
        sqlx::query!(
            r#"
            INSERT INTO call_participants (call_id, user_id, joined_at, muted, camera_off)
            VALUES ($1, $2, now(), false, false)
            ON CONFLICT (call_id, user_id)
            DO UPDATE SET left_at = NULL, joined_at = now()
            "#,
            call_id,
            user.id
        )
        .execute(&state.db)
        .await?;

        let session_item = fetch_call_session(&state.db, call_id).await?;

        // Notify other group members
        let other_members = sqlx::query_scalar!(
            "SELECT user_id FROM group_members WHERE group_id = $1 AND user_id != $2",
            group_id,
            user.id
        )
        .fetch_all(&state.db)
        .await?;

        state
            .ws_hub
            .send_to_users(
                &other_members,
                WsEvent::CallIncoming {
                    session: session_item.clone(),
                },
            )
            .await;

        return Ok(Json(session_item));
    }

    if let Some(receiver_id) = payload.receiver_id {
        // 1:1 Call
        if receiver_id == user.id {
            return Err(AppError::BadRequest("cannot_call_self"));
        }
        // Only friends can ring each other; blocks and "nobody can call me" are enforced here.
        if !crate::privacy::are_friends(&state.db, user.id, receiver_id).await?
            || crate::privacy::is_blocked_either(&state.db, user.id, receiver_id).await?
        {
            return Err(AppError::Forbidden("cannot_call_user"));
        }
        let accepts = sqlx::query_scalar!("SELECT privacy_calls FROM users WHERE id = $1", receiver_id)
            .fetch_one(&state.db)
            .await?;
        if accepts == "nobody" {
            return Err(AppError::Forbidden("calls_disabled"));
        }

        let cid = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO call_sessions (id, call_type, caller_id, receiver_id, status)
            VALUES ($1, $2, $3, $4, 'ringing')
            "#,
            cid,
            call_type,
            user.id,
            receiver_id
        )
        .execute(&state.db)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO call_participants (call_id, user_id, joined_at, muted, camera_off)
            VALUES ($1, $2, now(), false, false)
            "#,
            cid,
            user.id
        )
        .execute(&state.db)
        .await?;

        let session_item = fetch_call_session(&state.db, cid).await?;

        // Send incoming call event to receiver
        state
            .ws_hub
            .send_to_user(
                &receiver_id,
                WsEvent::CallIncoming {
                    session: session_item.clone(),
                },
            )
            .await;

        return Ok(Json(session_item));
    }

    Err(AppError::BadRequest("missing_receiver_or_group"))
}

// 2. Accept call
async fn accept_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
) -> AppResult<Json<CallSessionItem>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let session = sqlx::query!(
        "SELECT id, caller_id, receiver_id, group_id, status FROM call_sessions WHERE id = $1",
        call_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("call_not_found"))?;

    if session.status != "ringing" && session.status != "active" {
        return Err(AppError::BadRequest("call_not_active"));
    }

    // Verify authorized user
    if let Some(recv_id) = session.receiver_id {
        if recv_id != user.id && session.caller_id != user.id {
            return Err(AppError::Forbidden("not_authorized_for_call"));
        }
    } else if let Some(gid) = session.group_id {
        let is_member = sqlx::query_scalar!(
            "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
            gid,
            user.id
        )
        .fetch_optional(&state.db)
        .await?
        .is_some();
        if !is_member {
            return Err(AppError::Forbidden("not_a_group_member"));
        }
    }

    // Update status to active
    sqlx::query!(
        r#"
        UPDATE call_sessions
        SET status = 'active',
            started_at = COALESCE(started_at, now())
        WHERE id = $1
        "#,
        call_id
    )
    .execute(&state.db)
    .await?;

    // Add to participants
    sqlx::query!(
        r#"
        INSERT INTO call_participants (call_id, user_id, joined_at, muted, camera_off)
        VALUES ($1, $2, now(), false, false)
        ON CONFLICT (call_id, user_id)
        DO UPDATE SET left_at = NULL, joined_at = now()
        "#,
        call_id,
        user.id
    )
    .execute(&state.db)
    .await?;

    let session_item = fetch_call_session(&state.db, call_id).await?;

    // Notify caller or other participants
    let participant_item = CallParticipantItem {
        call_id,
        user_id: user.id,
        username: user.username.unwrap_or_default(),
        avatar_url: user.avatar_url,
        joined_at: Utc::now(),
        left_at: None,
        muted: false,
        camera_off: false,
    };

    let target_users = get_other_participants(&state.db, call_id, user.id).await?;
    state
        .ws_hub
        .send_to_users(
            &target_users,
            WsEvent::CallAccepted {
                call_id,
                user_id: user.id,
            },
        )
        .await;

    state
        .ws_hub
        .send_to_users(
            &target_users,
            WsEvent::CallParticipantJoined {
                call_id,
                participant: participant_item,
            },
        )
        .await;

    Ok(Json(session_item))
}

// 3. Reject call
async fn reject_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let session = sqlx::query!(
        "SELECT caller_id, receiver_id, group_id, status FROM call_sessions WHERE id = $1",
        call_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("call_not_found"))?;

    // Update status to rejected if 1:1
    if session.receiver_id.is_some() {
        sqlx::query!(
            "UPDATE call_sessions SET status = 'rejected', ended_at = now() WHERE id = $1",
            call_id
        )
        .execute(&state.db)
        .await?;

        state
            .ws_hub
            .send_to_user(
                &session.caller_id,
                WsEvent::CallRejected {
                    call_id,
                    user_id: user.id,
                    reason: "rejected".to_string(),
                },
            )
            .await;
    }

    Ok(Json(serde_json::json!({ "rejected": true })))
}

// 4. End call (Entire session ends)
async fn end_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let targets = get_other_participants(&state.db, call_id, user.id).await?;

    sqlx::query!(
        "UPDATE call_sessions SET status = 'ended', ended_at = now() WHERE id = $1",
        call_id
    )
    .execute(&state.db)
    .await?;

    sqlx::query!(
        "UPDATE call_participants SET left_at = now() WHERE call_id = $1 AND left_at IS NULL",
        call_id
    )
    .execute(&state.db)
    .await?;

    state.sfu.end_call(call_id);

    state
        .ws_hub
        .send_to_users(
            &targets,
            WsEvent::CallEnded {
                call_id,
                reason: "ended".to_string(),
            },
        )
        .await;

    Ok(Json(serde_json::json!({ "ended": true })))
}

// 5. Leave call (Participant leaves)
async fn leave_call(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    sqlx::query!(
        "UPDATE call_participants SET left_at = now() WHERE call_id = $1 AND user_id = $2",
        call_id,
        user.id
    )
    .execute(&state.db)
    .await?;

    state.sfu.leave_peer(call_id, user.id);

    let targets = get_other_participants(&state.db, call_id, user.id).await?;

    state
        .ws_hub
        .send_to_users(
            &targets,
            WsEvent::CallParticipantLeft {
                call_id,
                user_id: user.id,
            },
        )
        .await;

    // Check if anyone is still in the call
    let remaining_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM call_participants WHERE call_id = $1 AND left_at IS NULL",
        call_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    if remaining_count == 0 {
        sqlx::query!(
            "UPDATE call_sessions SET status = 'ended', ended_at = now() WHERE id = $1",
            call_id
        )
        .execute(&state.db)
        .await?;
    }

    Ok(Json(serde_json::json!({ "left": true })))
}

// 6. Relay WebRTC Signal (SDP offer/answer, ICE candidate)
async fn relay_signal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
    Json(payload): Json<CallSignalPayload>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let event = WsEvent::CallSignal {
        call_id,
        sender_id: user.id,
        target_user_id: payload.target_user_id,
        signal_type: payload.signal_type,
        data: payload.data,
    };

    if let Some(target_uid) = payload.target_user_id {
        state.ws_hub.send_to_user(&target_uid, event).await;
    } else {
        // Send to all other active participants
        let targets = get_other_participants(&state.db, call_id, user.id).await?;
        state.ws_hub.send_to_users(&targets, event).await;
    }

    Ok(Json(serde_json::json!({ "relayed": true })))
}

// 7. Update Call State (Mute / Camera)
async fn update_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
    Json(payload): Json<UpdateCallStateRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query!(
        r#"
        UPDATE call_participants
        SET muted = COALESCE($3, muted),
            camera_off = COALESCE($4, camera_off)
        WHERE call_id = $1 AND user_id = $2
        RETURNING muted, camera_off
        "#,
        call_id,
        user.id,
        payload.muted,
        payload.camera_off
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("not_in_call"))?;

    let targets = get_other_participants(&state.db, call_id, user.id).await?;
    state
        .ws_hub
        .send_to_users(
            &targets,
            WsEvent::CallStateUpdate {
                call_id,
                user_id: user.id,
                muted: row.muted,
                camera_off: row.camera_off,
            },
        )
        .await;

    Ok(Json(serde_json::json!({ "muted": row.muted, "camera_off": row.camera_off })))
}

// 8. Get active call for current user
async fn get_active_call(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Option<CallSessionItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let session = sqlx::query!(
        r#"
        SELECT c.id FROM call_sessions c
        JOIN call_participants p ON c.id = p.call_id
        WHERE p.user_id = $1 AND p.left_at IS NULL AND c.status IN ('ringing', 'active')
        ORDER BY c.created_at DESC
        LIMIT 1
        "#,
        user.id
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(s) = session {
        let item = fetch_call_session(&state.db, s.id).await?;
        Ok(Json(Some(item)))
    } else {
        Ok(Json(None))
    }
}

// 9. Get call history
async fn get_call_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<CallSessionItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT c.id, c.created_at FROM call_sessions c
        JOIN call_participants p ON c.id = p.call_id
        WHERE p.user_id = $1
        ORDER BY c.created_at DESC
        LIMIT 30
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = fetch_call_session(&state.db, r.id).await {
            list.push(item);
        }
    }

    Ok(Json(list))
}

// Helper: fetch full session details
async fn fetch_call_session(db: &sqlx::PgPool, call_id: Uuid) -> AppResult<CallSessionItem> {
    let s = sqlx::query!(
        r#"
        SELECT c.id as "id!", c.call_type as "call_type!", c.caller_id as "caller_id!", c.receiver_id, c.group_id, c.status as "status!",
               c.started_at, c.ended_at, c.created_at as "created_at!",
               u_caller.username AS caller_name, u_caller.avatar_url AS caller_avatar,
               u_recv.username AS "receiver_name?",
               g.name AS "group_name?"
        FROM call_sessions c
        JOIN users u_caller ON c.caller_id = u_caller.id
        LEFT JOIN users u_recv ON c.receiver_id = u_recv.id
        LEFT JOIN groups g ON c.group_id = g.id
        WHERE c.id = $1
        "#,
        call_id
    )
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound("call_not_found"))?;

    let part_rows = sqlx::query!(
        r#"
        SELECT p.call_id, p.user_id, p.joined_at, p.left_at, p.muted, p.camera_off,
               u.username, u.avatar_url
        FROM call_participants p
        JOIN users u ON p.user_id = u.id
        WHERE p.call_id = $1
        ORDER BY p.joined_at ASC
        "#,
        call_id
    )
    .fetch_all(db)
    .await?;

    let participants = part_rows
        .into_iter()
        .map(|p| CallParticipantItem {
            call_id: p.call_id,
            user_id: p.user_id,
            username: p.username.unwrap_or_default(),
            avatar_url: p.avatar_url,
            joined_at: p.joined_at,
            left_at: p.left_at,
            muted: p.muted,
            camera_off: p.camera_off,
        })
        .collect();

    let (meet_code, meet_title) = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT meet_code, meet_title FROM call_sessions WHERE id = $1",
    )
    .bind(call_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or((None, None));

    Ok(CallSessionItem {
        id: s.id,
        call_type: s.call_type,
        caller_id: s.caller_id,
        caller_name: s.caller_name.unwrap_or_default(),
        caller_avatar_url: s.caller_avatar,
        receiver_id: s.receiver_id,
        receiver_name: s.receiver_name,
        group_id: s.group_id,
        group_name: s.group_name,
        meet_code,
        meet_title,
        status: s.status,
        started_at: s.started_at,
        ended_at: s.ended_at,
        created_at: s.created_at,
        participants,
    })
}

async fn get_other_participants(
    db: &sqlx::PgPool,
    call_id: Uuid,
    current_user_id: Uuid,
) -> AppResult<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        r#"
        SELECT user_id FROM call_participants
        WHERE call_id = $1 AND user_id != $2 AND left_at IS NULL
        "#,
        call_id,
        current_user_id
    )
    .fetch_all(db)
    .await?;

    Ok(ids)
}

#[derive(serde::Deserialize)]
struct SfuJoinRequest {
    sdp: String,
    #[serde(default)]
    meta: std::collections::HashMap<String, String>,
}

async fn sfu_join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
    Json(payload): Json<SfuJoinRequest>,
) -> AppResult<Json<sfu::SfuJoinReply>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let is_participant = sqlx::query_scalar!(
        "SELECT 1 FROM call_participants WHERE call_id = $1 AND user_id = $2",
        call_id,
        user.id
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();

    if !is_participant {
        let allowed_session = sqlx::query!(
            "SELECT caller_id, receiver_id, group_id, meet_code FROM call_sessions WHERE id = $1",
            call_id
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("call_not_found"))?;

        let mut allowed = allowed_session.caller_id == user.id
            || allowed_session.receiver_id == Some(user.id)
            || allowed_session.meet_code.is_some();

        if !allowed {
            if let Some(gid) = allowed_session.group_id {
                allowed = sqlx::query_scalar!(
                    "SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2",
                    gid,
                    user.id
                )
                .fetch_optional(&state.db)
                .await?
                .is_some();
            }
        }
        if !allowed {
            return Err(AppError::Forbidden("not_authorized_for_call"));
        }
    }

    let reply = state
        .sfu
        .join_peer(call_id, user.id, payload.sdp, payload.meta)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "SFU join failed");
            AppError::BadRequest("sfu_negotiation_failed")
        })?;

    Ok(Json(reply))
}

async fn sfu_leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(call_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    state.sfu.leave_peer(call_id, user.id);
    Ok(Json(serde_json::json!({ "left_sfu": true })))
}
