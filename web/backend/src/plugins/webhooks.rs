//! Discord-Style Incoming & Outgoing Webhooks for ChatW Servers, Groups, Channels & Plugins.
//!
//! - Incoming: `POST /api/webhooks/{webhook_id}/{token}` (no cookie required; verifies constant-time SHA-256 token hash,
//!   validates Discord Embed JSON schema, enforces rate limit + idempotency deduplication, inserts real message into
//!   `channel_messages` or `group_messages`, broadcasts over `WsHub`, and records delivery logs).
//! - Outgoing: Subscribed event delivery with HTTPS + SSRF protection, HMAC-SHA256 signature (`X-ChatW-Signature-256`),
//!   timestamp + replay protection, Delivery ID, exponential backoff retry worker, dead-letter queue, and delivery logs.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    models::{ChannelMessageItem, GroupMessageItem},
    plugins::{
        schema::{IncomingWebhookPayload, is_safe_http_url, validate_incoming_webhook_payload},
        security::{
            PLUGIN_RATE_LIMITER, build_pinned_client, constant_time_eq_bytes, decrypt_secret,
            encrypt_secret, require_scope_manage_access, require_scope_read_access,
            sign_webhook_payload, validate_outbound_url,
        },
    },
    privacy::auth_user,
    websocket::WsEvent,
};

#[derive(Deserialize)]
pub struct CreateWebhookReq {
    #[serde(default = "default_incoming")]
    pub direction: String, // "incoming" | "outgoing"
    pub scope_type: String, // "server" | "group" | "channel" | "user"
    pub scope_id: Uuid,
    #[serde(default)]
    pub channel_id: Option<Uuid>,
    #[serde(default)]
    pub installation_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub target_url: Option<String>,
    #[serde(default)]
    pub subscribed_events: Vec<String>,
    #[serde(default)]
    pub rate_limit_per_min: Option<i32>,
}

fn default_incoming() -> String {
    "incoming".into()
}

#[derive(Deserialize)]
pub struct UpdateWebhookReq {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub channel_id: Option<Uuid>,
    pub target_url: Option<String>,
    pub subscribed_events: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub rate_limit_per_min: Option<i32>,
}

#[derive(Deserialize)]
pub struct ListWebhooksQuery {
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
    pub direction: Option<String>,
}

fn generate_random_secret(prefix: &str) -> AppResult<String> {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|_| AppError::Internal)?;
    Ok(format!("{prefix}{}", URL_SAFE_NO_PAD.encode(buf)))
}

fn sha256_bytes(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

pub async fn create_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateWebhookReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    if !matches!(req.direction.as_str(), "incoming" | "outgoing") {
        return Err(AppError::BadRequest("invalid_webhook_direction"));
    }
    let name = req.name.trim();
    if name.len() < 2 || name.len() > 80 {
        return Err(AppError::BadRequest("invalid_webhook_name"));
    }
    if let Some(ref av) = req.avatar_url {
        if !av.is_empty() && !av.starts_with('/') && !is_safe_http_url(av) {
            return Err(AppError::BadRequest("invalid_avatar_url"));
        }
    }
    require_scope_manage_access(&state, uid, &req.scope_type, req.scope_id).await?;

    // Resolve channel_id for incoming server/channel webhooks
    let resolved_channel_id = if req.direction == "incoming" {
        match req.scope_type.as_str() {
            "channel" => Some(req.scope_id),
            "server" => {
                let cid = req.channel_id.ok_or(AppError::BadRequest("channel_id_required"))?;
                let belongs: Option<Uuid> =
                    sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
                        .bind(cid)
                        .fetch_optional(&state.db)
                        .await?;
                if belongs != Some(req.scope_id) {
                    return Err(AppError::BadRequest("channel_not_in_server"));
                }
                Some(cid)
            }
            "group" => req.channel_id,
            _ => return Err(AppError::BadRequest("incoming_webhook_requires_channel_or_group")),
        }
    } else {
        req.channel_id
    };

    let rate_limit = req.rate_limit_per_min.unwrap_or(30).clamp(1, 120);
    let wh_id = Uuid::new_v4();
    let mut returned_token: Option<String> = None;
    let mut returned_secret: Option<String> = None;
    let mut token_hash: Option<Vec<u8>> = None;
    let token_hint: String;
    let mut target_url: Option<String> = None;
    let mut secret_ct: Option<Vec<u8>> = None;

    if req.direction == "incoming" {
        let tok = generate_random_secret("cw_wh_")?;
        token_hint = format!("{}***", &tok[..12]);
        token_hash = Some(sha256_bytes(&tok));
        returned_token = Some(tok);
    } else {
        let raw_url = req.target_url.as_deref().ok_or(AppError::BadRequest("target_url_required"))?;
        let validated = validate_outbound_url(raw_url).await?;
        target_url = Some(validated.url);
        let sec = generate_random_secret("whsec_")?;
        token_hint = format!("{}***", &sec[..10]);
        secret_ct = Some(encrypt_secret(&state.config.session_secret, sec.as_bytes())?);
        returned_secret = Some(sec);
    }

    sqlx::query(
        "INSERT INTO plugin_webhooks (id, direction, scope_type, scope_id, channel_id, installation_id, created_by, name, avatar_url, token_hash, token_hint, target_url, secret_ciphertext, subscribed_events, enabled, rate_limit_per_min)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, true, $15)",
    )
    .bind(wh_id)
    .bind(&req.direction)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .bind(resolved_channel_id)
    .bind(req.installation_id)
    .bind(uid)
    .bind(name)
    .bind(req.avatar_url.as_deref())
    .bind(token_hash.as_deref())
    .bind(&token_hint)
    .bind(target_url.as_deref())
    .bind(secret_ct.as_deref())
    .bind(serde_json::to_value(&req.subscribed_events).unwrap_or_else(|_| json!([])))
    .bind(rate_limit)
    .execute(&state.db)
    .await?;

    let execute_url = returned_token
        .as_ref()
        .map(|t| format!("/api/webhooks/{wh_id}/{t}"));

    Ok(Json(json!({
        "id": wh_id,
        "direction": req.direction,
        "scope_type": req.scope_type,
        "scope_id": req.scope_id,
        "channel_id": resolved_channel_id,
        "name": name,
        "avatar_url": req.avatar_url,
        "token_hint": token_hint,
        "token": returned_token,
        "execute_url": execute_url,
        "target_url": target_url,
        "signing_secret": returned_secret,
        "subscribed_events": req.subscribed_events,
        "enabled": true,
        "rate_limit_per_min": rate_limit,
    })))
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListWebhooksQuery>,
) -> AppResult<Json<Vec<Value>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = if let (Some(st), Some(sid)) = (q.scope_type.as_deref(), q.scope_id) {
        require_scope_read_access(&state, uid, st, sid).await?;
        sqlx::query(
            "SELECT id, direction, scope_type, scope_id, channel_id, installation_id, created_by, name, avatar_url, token_hint, target_url, subscribed_events, enabled, rate_limit_per_min, last_used_at, created_at
             FROM plugin_webhooks WHERE scope_type = $1 AND scope_id = $2 ORDER BY created_at DESC",
        )
        .bind(st)
        .bind(sid)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id, direction, scope_type, scope_id, channel_id, installation_id, created_by, name, avatar_url, token_hint, target_url, subscribed_events, enabled, rate_limit_per_min, last_used_at, created_at
               FROM plugin_webhooks
               WHERE created_by = $1
                  OR (scope_type = 'server' AND scope_id IN (SELECT server_id FROM server_members WHERE user_id = $1))
                  OR (scope_type = 'group' AND scope_id IN (SELECT group_id FROM group_members WHERE user_id = $1))
               ORDER BY created_at DESC LIMIT 100"#,
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await?
    };

    let dir_filter = q.direction.as_deref();
    Ok(Json(
        rows.into_iter()
            .filter(|r| dir_filter.map(|d| r.get::<String, _>("direction") == d).unwrap_or(true))
            .map(|r| {
                // Never return secret tokens or HMAC keys in list responses!
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "direction": r.get::<String, _>("direction"),
                    "scope_type": r.get::<String, _>("scope_type"),
                    "scope_id": r.get::<Uuid, _>("scope_id"),
                    "channel_id": r.get::<Option<Uuid>, _>("channel_id"),
                    "installation_id": r.get::<Option<Uuid>, _>("installation_id"),
                    "name": r.get::<String, _>("name"),
                    "avatar_url": r.get::<Option<String>, _>("avatar_url"),
                    "token_hint": r.get::<String, _>("token_hint"),
                    "target_url": r.get::<Option<String>, _>("target_url"),
                    "subscribed_events": r.get::<Value, _>("subscribed_events"),
                    "enabled": r.get::<bool, _>("enabled"),
                    "rate_limit_per_min": r.get::<i32, _>("rate_limit_per_min"),
                    "last_used_at": r.get::<Option<DateTime<Utc>>, _>("last_used_at"),
                    "created_at": r.get::<DateTime<Utc>, _>("created_at"),
                })
            })
            .collect(),
    ))
}

pub async fn update_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wh_id): Path<Uuid>,
    Json(req): Json<UpdateWebhookReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query(
        "SELECT direction, scope_type, scope_id, channel_id, name, avatar_url, target_url, subscribed_events, enabled, rate_limit_per_min
         FROM plugin_webhooks WHERE id = $1",
    )
    .bind(wh_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("webhook_not_found"))?;

    let scope_type: String = cur.get("scope_type");
    let scope_id: Uuid = cur.get("scope_id");
    require_scope_manage_access(&state, uid, &scope_type, scope_id).await?;

    let direction: String = cur.get("direction");
    let target_url = if let Some(ref u) = req.target_url {
        if direction != "outgoing" {
            return Err(AppError::BadRequest("not_outgoing_webhook"));
        }
        Some(validate_outbound_url(u).await?.url)
    } else {
        cur.get("target_url")
    };

    let name = req.name.unwrap_or_else(|| cur.get("name"));
    if name.trim().len() < 2 || name.trim().len() > 80 {
        return Err(AppError::BadRequest("invalid_webhook_name"));
    }
    let avatar_url = req.avatar_url.or_else(|| cur.get("avatar_url"));
    let channel_id = req.channel_id.or_else(|| cur.get("channel_id"));
    let subscribed_events = req
        .subscribed_events
        .map(|v| serde_json::to_value(v).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| cur.get("subscribed_events"));
    let enabled = req.enabled.unwrap_or_else(|| cur.get("enabled"));
    let rate_limit = req
        .rate_limit_per_min
        .unwrap_or_else(|| cur.get("rate_limit_per_min"))
        .clamp(1, 120);

    sqlx::query(
        "UPDATE plugin_webhooks
         SET name = $1, avatar_url = $2, channel_id = $3, target_url = $4,
             subscribed_events = $5, enabled = $6, rate_limit_per_min = $7, updated_at = now()
         WHERE id = $8",
    )
    .bind(name.trim())
    .bind(avatar_url.as_deref())
    .bind(channel_id)
    .bind(target_url.as_deref())
    .bind(&subscribed_events)
    .bind(enabled)
    .bind(rate_limit)
    .bind(wh_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "id": wh_id,
        "name": name.trim(),
        "enabled": enabled,
        "channel_id": channel_id,
        "target_url": target_url,
        "rate_limit_per_min": rate_limit,
    })))
}

pub async fn rotate_webhook_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wh_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query("SELECT direction, scope_type, scope_id FROM plugin_webhooks WHERE id = $1")
        .bind(wh_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("webhook_not_found"))?;
    require_scope_manage_access(&state, uid, &cur.get::<String, _>("scope_type"), cur.get("scope_id")).await?;

    let dir: String = cur.get("direction");
    if dir == "incoming" {
        let tok = generate_random_secret("cw_wh_")?;
        let hint = format!("{}***", &tok[..12]);
        let hash = sha256_bytes(&tok);
        sqlx::query("UPDATE plugin_webhooks SET token_hash = $1, token_hint = $2, updated_at = now() WHERE id = $3")
            .bind(&hash)
            .bind(&hint)
            .bind(wh_id)
            .execute(&state.db)
            .await?;
        Ok(Json(json!({
            "id": wh_id,
            "token": tok,
            "token_hint": hint,
            "execute_url": format!("/api/webhooks/{wh_id}/{tok}"),
        })))
    } else {
        let sec = generate_random_secret("whsec_")?;
        let hint = format!("{}***", &sec[..10]);
        let ct = encrypt_secret(&state.config.session_secret, sec.as_bytes())?;
        sqlx::query("UPDATE plugin_webhooks SET secret_ciphertext = $1, token_hint = $2, updated_at = now() WHERE id = $3")
            .bind(&ct)
            .bind(&hint)
            .bind(wh_id)
            .execute(&state.db)
            .await?;
        Ok(Json(json!({
            "id": wh_id,
            "signing_secret": sec,
            "token_hint": hint,
        })))
    }
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wh_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query("SELECT scope_type, scope_id FROM plugin_webhooks WHERE id = $1")
        .bind(wh_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("webhook_not_found"))?;
    require_scope_manage_access(&state, uid, &cur.get::<String, _>("scope_type"), cur.get("scope_id")).await?;

    sqlx::query("DELETE FROM plugin_webhooks WHERE id = $1")
        .bind(wh_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "deleted": true, "id": wh_id })))
}

pub async fn list_webhook_deliveries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wh_id): Path<Uuid>,
) -> AppResult<Json<Vec<Value>>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query("SELECT scope_type, scope_id FROM plugin_webhooks WHERE id = $1")
        .bind(wh_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("webhook_not_found"))?;
    require_scope_read_access(&state, uid, &cur.get::<String, _>("scope_type"), cur.get("scope_id")).await?;

    let rows = sqlx::query(
        "SELECT id, event_type, event_id, status, http_status, attempt_count, request_bytes, response_preview, error_message, next_retry_at, created_at, completed_at
         FROM plugin_webhook_deliveries WHERE webhook_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(wh_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "event_type": r.get::<String, _>("event_type"),
                    "event_id": r.get::<String, _>("event_id"),
                    "status": r.get::<String, _>("status"),
                    "http_status": r.get::<Option<i32>, _>("http_status"),
                    "attempt_count": r.get::<i32, _>("attempt_count"),
                    "request_bytes": r.get::<i32, _>("request_bytes"),
                    "response_preview": r.get::<String, _>("response_preview"),
                    "error_message": r.get::<String, _>("error_message"),
                    "next_retry_at": r.get::<Option<DateTime<Utc>>, _>("next_retry_at"),
                    "created_at": r.get::<DateTime<Utc>, _>("created_at"),
                    "completed_at": r.get::<Option<DateTime<Utc>>, _>("completed_at"),
                })
            })
            .collect(),
    ))
}

/// Public Discord-compatible Incoming Webhook execution endpoint:
/// `POST /api/webhooks/{webhook_id}/{token}`
pub async fn execute_incoming_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((wh_id, token)): Path<(Uuid, String)>,
    Json(payload): Json<IncomingWebhookPayload>,
) -> AppResult<Json<Value>> {
    validate_incoming_webhook_payload(&payload).map_err(AppError::BadRequest)?;

    let row = sqlx::query(
        "SELECT direction, scope_type, scope_id, channel_id, created_by, name, avatar_url, token_hash, enabled, rate_limit_per_min
         FROM plugin_webhooks WHERE id = $1",
    )
    .bind(wh_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("webhook_not_found"))?;

    let dir: String = row.get("direction");
    let enabled: bool = row.get("enabled");
    if dir != "incoming" {
        return Err(AppError::BadRequest("not_incoming_webhook"));
    }
    if !enabled {
        return Err(AppError::Forbidden("webhook_disabled"));
    }

    let expected_hash: Option<Vec<u8>> = row.get("token_hash");
    let supplied_hash = sha256_bytes(&token);
    let Some(exp) = expected_hash else {
        return Err(AppError::Unauthorized);
    };
    if !constant_time_eq_bytes(&exp, &supplied_hash) {
        return Err(AppError::Unauthorized);
    }

    let rpm = row.get::<i32, _>("rate_limit_per_min").clamp(1, 120) as usize;
    if !PLUGIN_RATE_LIMITER.check_and_record(&format!("wh_in:{wh_id}"), rpm, Duration::from_secs(60)) {
        return Err(AppError::TooManyRequests);
    }

    // Idempotency deduplication (header `Idempotency-Key` or `payload.idempotency_key`)
    let idem_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| payload.idempotency_key.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let deliv_id = Uuid::new_v4();
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO plugin_webhook_deliveries (id, webhook_id, event_type, event_id, status, attempt_count, request_bytes)
         VALUES ($1, $2, 'incoming.message', $3, 'pending', 1, $4)
         ON CONFLICT (webhook_id, event_id) DO NOTHING
         RETURNING id",
    )
    .bind(deliv_id)
    .bind(wh_id)
    .bind(&idem_key)
    .bind(serde_json::to_vec(&payload).map(|v| v.len()).unwrap_or(0) as i32)
    .fetch_optional(&state.db)
    .await?;

    if inserted.is_none() {
        return Ok(Json(json!({
            "ok": true,
            "deduplicated": true,
            "idempotency_key": idem_key,
        })));
    }

    let scope_type: String = row.get("scope_type");
    let scope_id: Uuid = row.get("scope_id");
    let channel_id: Option<Uuid> = row.get("channel_id");
    let created_by: Uuid = row.get("created_by");
    let wh_name = payload
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| row.get::<&str, _>("name"))
        .to_string();
    let wh_avatar = payload
        .avatar_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| row.get("avatar_url"));

    // Encode as structured rich `embed:{json}` so Discord embeds, buttons, and Webhook identity render natively
    let rich_obj = json!({
        "webhook_id": wh_id,
        "username": wh_name,
        "avatar_url": wh_avatar,
        "content": payload.content.clone().unwrap_or_default(),
        "embeds": payload.embeds,
        "components": payload.components,
    });
    let encoded_content = format!("embed:{}", serde_json::to_string(&rich_obj).unwrap_or_default());
    let msg_id = Uuid::new_v4();

    if scope_type == "group" {
        let grp_row = sqlx::query(
            "INSERT INTO group_messages (id, group_id, channel_id, sender_id, content)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING seq, created_at",
        )
        .bind(msg_id)
        .bind(scope_id)
        .bind(channel_id)
        .bind(created_by)
        .bind(&encoded_content)
        .fetch_one(&state.db)
        .await?;

        let item = GroupMessageItem {
            id: msg_id,
            seq: grp_row.get("seq"),
            group_id: scope_id,
            channel_id,
            sender_id: created_by,
            sender_name: wh_name.clone(),
            sender_avatar_url: wh_avatar.clone(),
            content: encoded_content,
            reply_to_id: None,
            is_pinned: false,
            is_edited: false,
            reactions: Vec::new(),
            is_bot: true,
            created_at: grp_row.get("created_at"),
        };

        let member_ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
                .bind(scope_id)
                .fetch_all(&state.db)
                .await?;
        state
            .ws_hub
            .send_to_users(&member_ids, WsEvent::GroupMessageNew { message: item })
            .await;
    } else {
        let target_cid = channel_id.ok_or(AppError::BadRequest("channel_id_missing"))?;
        let srv_id: Uuid = sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
            .bind(target_cid)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound("channel_not_found"))?;

        let ch_row = sqlx::query(
            "INSERT INTO channel_messages (id, channel_id, sender_id, content)
             VALUES ($1, $2, $3, $4)
             RETURNING seq, created_at, is_pinned, is_edited",
        )
        .bind(msg_id)
        .bind(target_cid)
        .bind(created_by)
        .bind(&encoded_content)
        .fetch_one(&state.db)
        .await?;

        let item = ChannelMessageItem {
            id: msg_id,
            seq: ch_row.get("seq"),
            channel_id: target_cid,
            sender_id: created_by,
            sender_name: wh_name.clone(),
            sender_avatar_url: wh_avatar.clone(),
            content: encoded_content,
            reply_to_id: None,
            is_pinned: ch_row.get("is_pinned"),
            is_edited: ch_row.get("is_edited"),
            created_at: ch_row.get("created_at"),
        };

        let member_ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM server_members WHERE server_id = $1")
                .bind(srv_id)
                .fetch_all(&state.db)
                .await?;
        state
            .ws_hub
            .send_to_users(&member_ids, WsEvent::ChannelMessageNew { message: item })
            .await;
    }

    sqlx::query("UPDATE plugin_webhooks SET last_used_at = now() WHERE id = $1")
        .bind(wh_id)
        .execute(&state.db)
        .await?;
    sqlx::query(
        "UPDATE plugin_webhook_deliveries SET status = 'delivered', http_status = 200, completed_at = now() WHERE id = $1",
    )
    .bind(deliv_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "ok": true,
        "message_id": msg_id,
        "delivery_id": deliv_id,
        "webhook_id": wh_id,
        "username": wh_name,
    })))
}

/// Authenticated Test Endpoint: triggers a sample Discord embed (incoming) or signed ping (outgoing).
pub async fn test_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wh_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query(
        "SELECT direction, scope_type, scope_id, channel_id, created_by, name, avatar_url, target_url, secret_ciphertext
         FROM plugin_webhooks WHERE id = $1",
    )
    .bind(wh_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("webhook_not_found"))?;
    let scope_type: String = cur.get("scope_type");
    let scope_id: Uuid = cur.get("scope_id");
    require_scope_manage_access(&state, uid, &scope_type, scope_id).await?;

    let dir: String = cur.get("direction");
    if dir == "outgoing" {
        let res = deliver_outgoing_single(
            &state,
            wh_id,
            "webhook.test",
            &Uuid::new_v4().to_string(),
            json!({
                "event": "webhook.test",
                "webhook_id": wh_id,
                "scope_type": scope_type,
                "scope_id": scope_id,
                "triggered_by": uid,
            }),
        )
        .await;
        return Ok(Json(res));
    }

    // Incoming test: insert a live Discord-style test embed directly
    let wh_name: String = cur.get("name");
    let wh_avatar: Option<String> = cur.get("avatar_url");
    let channel_id: Option<Uuid> = cur.get("channel_id");
    let created_by: Uuid = cur.get("created_by");
    let rich_obj = json!({
        "webhook_id": wh_id,
        "username": wh_name,
        "avatar_url": wh_avatar,
        "content": "Test messages from ChatW Webhook",
        "embeds": [{
            "title": "Webhook connection works",
            "description": "Full support for Discord-style Embeds, SHA-256 token authentication, deduplication, and delivery logs.",
            "color": 2450411,
            "fields": [
                { "name": "Status", "value": "Online", "inline": true },
                { "name": "Scope", "value": scope_type, "inline": true }
            ],
            "footer": { "text": "ChatW Webhook Engine" },
            "timestamp": Utc::now().to_rfc3339()
        }],
        "components": []
    });
    let encoded_content = format!("embed:{}", serde_json::to_string(&rich_obj).unwrap_or_default());
    let msg_id = Uuid::new_v4();

    if scope_type == "group" {
        let grp_row = sqlx::query(
            "INSERT INTO group_messages (id, group_id, channel_id, sender_id, content) VALUES ($1, $2, $3, $4, $5) RETURNING seq, created_at",
        )
        .bind(msg_id)
        .bind(scope_id)
        .bind(channel_id)
        .bind(created_by)
        .bind(&encoded_content)
        .fetch_one(&state.db)
        .await?;
        let item = GroupMessageItem {
            id: msg_id,
            seq: grp_row.get("seq"),
            group_id: scope_id,
            channel_id,
            sender_id: created_by,
            sender_name: wh_name,
            sender_avatar_url: wh_avatar,
            content: encoded_content,
            reply_to_id: None,
            is_pinned: false,
            is_edited: false,
            reactions: Vec::new(),
            is_bot: true,
            created_at: grp_row.get("created_at"),
        };
        let members: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(scope_id)
            .fetch_all(&state.db)
            .await?;
        state.ws_hub.send_to_users(&members, WsEvent::GroupMessageNew { message: item }).await;
    } else if let Some(cid) = channel_id {
        let srv_id: Uuid = sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
            .bind(cid)
            .fetch_one(&state.db)
            .await?;
        let ch_row = sqlx::query(
            "INSERT INTO channel_messages (id, channel_id, sender_id, content) VALUES ($1, $2, $3, $4) RETURNING seq, created_at, is_pinned, is_edited",
        )
        .bind(msg_id)
        .bind(cid)
        .bind(created_by)
        .bind(&encoded_content)
        .fetch_one(&state.db)
        .await?;
        let item = ChannelMessageItem {
            id: msg_id,
            seq: ch_row.get("seq"),
            channel_id: cid,
            sender_id: created_by,
            sender_name: wh_name,
            sender_avatar_url: wh_avatar,
            content: encoded_content,
            reply_to_id: None,
            is_pinned: ch_row.get("is_pinned"),
            is_edited: ch_row.get("is_edited"),
            created_at: ch_row.get("created_at"),
        };
        let members: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM server_members WHERE server_id = $1")
            .bind(srv_id)
            .fetch_all(&state.db)
            .await?;
        state.ws_hub.send_to_users(&members, WsEvent::ChannelMessageNew { message: item }).await;
    }

    Ok(Json(json!({ "ok": true, "message_id": msg_id })))
}

// ============================================================================
// Outgoing Event Dispatcher, HMAC Signing & Exponential Backoff Retry Worker
// ============================================================================

pub async fn dispatch_outgoing_event(
    state: AppState,
    scope_type: &'static str,
    scope_id: Uuid,
    channel_id: Option<Uuid>,
    event_type: &'static str,
    data: Value,
) {
    tokio::spawn(async move {
        let hooks = sqlx::query(
            "SELECT id, channel_id, subscribed_events
             FROM plugin_webhooks
             WHERE direction = 'outgoing' AND enabled = true
               AND ((scope_type = $1 AND scope_id = $2)
                    OR ($3::uuid IS NOT NULL AND scope_type = 'channel' AND scope_id = $3))",
        )
        .bind(scope_type)
        .bind(scope_id)
        .bind(channel_id)
        .fetch_all(&state.db)
        .await;

        let Ok(rows) = hooks else { return };
        let event_id = format!("evt_{}", Uuid::new_v4().simple());
        for r in rows {
            let wh_id: Uuid = r.get("id");
            let wh_ch: Option<Uuid> = r.get("channel_id");
            if let (Some(filter_ch), Some(ev_ch)) = (wh_ch, channel_id) {
                if filter_ch != ev_ch {
                    continue;
                }
            }
            let subs: Vec<String> = serde_json::from_value(r.get("subscribed_events")).unwrap_or_default();
            if !subs.is_empty() && !subs.iter().any(|s| s == "*" || s == event_type) {
                continue;
            }
            let envelope = json!({
                "id": event_id,
                "event": event_type,
                "scope_type": scope_type,
                "scope_id": scope_id,
                "channel_id": channel_id,
                "timestamp": Utc::now().to_rfc3339(),
                "data": data,
            });
            let _ = deliver_outgoing_single(&state, wh_id, event_type, &event_id, envelope).await;
        }
    });
}

async fn deliver_outgoing_single(
    state: &AppState,
    wh_id: Uuid,
    event_type: &str,
    event_id: &str,
    payload: Value,
) -> Value {
    let deliv_id = Uuid::new_v4();
    let body_bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let ins: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO plugin_webhook_deliveries (id, webhook_id, event_type, event_id, status, attempt_count, request_bytes, payload_json)
         VALUES ($1, $2, $3, $4, 'pending', 0, $5, $6)
         ON CONFLICT (webhook_id, event_id) DO NOTHING
         RETURNING id",
    )
    .bind(deliv_id)
    .bind(wh_id)
    .bind(event_type)
    .bind(event_id)
    .bind(body_bytes.len() as i32)
    .bind(&payload)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let Some(did) = ins else {
        return json!({ "ok": true, "deduplicated": true });
    };
    attempt_outgoing_delivery(state, did, wh_id, event_type, &body_bytes, 1).await
}

async fn attempt_outgoing_delivery(
    state: &AppState,
    deliv_id: Uuid,
    wh_id: Uuid,
    event_type: &str,
    body_bytes: &[u8],
    attempt: i32,
) -> Value {
    let wh = match sqlx::query("SELECT target_url, secret_ciphertext, enabled FROM plugin_webhooks WHERE id = $1")
        .bind(wh_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(r)) => r,
        _ => return json!({ "ok": false, "error": "webhook_not_found" }),
    };

    if !wh.get::<bool, _>("enabled") {
        let _ = sqlx::query("UPDATE plugin_webhook_deliveries SET status = 'dead_letter', error_message = 'webhook_disabled', completed_at = now() WHERE id = $1")
            .bind(deliv_id)
            .execute(&state.db)
            .await;
        return json!({ "ok": false, "error": "webhook_disabled" });
    }

    let raw_url: String = wh.get::<Option<String>, _>("target_url").unwrap_or_default();
    let target = match validate_outbound_url(&raw_url).await {
        Ok(t) => t,
        Err(_) => {
            let _ = sqlx::query("UPDATE plugin_webhook_deliveries SET status = 'dead_letter', attempt_count = $1, error_message = 'ssrf_or_url_blocked', completed_at = now() WHERE id = $2")
                .bind(attempt)
                .bind(deliv_id)
                .execute(&state.db)
                .await;
            return json!({ "ok": false, "status": "dead_letter", "error": "ssrf_blocked" });
        }
    };

    let secret = wh
        .get::<Option<Vec<u8>>, _>("secret_ciphertext")
        .and_then(|ct| decrypt_secret(&state.config.session_secret, &ct).ok())
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();

    let ts = Utc::now().timestamp();
    let sig = sign_webhook_payload(&secret, ts, body_bytes);
    let client = match build_pinned_client(&target, 6) {
        Ok(c) => c,
        Err(_) => return json!({ "ok": false, "error": "client_init_failed" }),
    };

    let resp = client
        .post(&target.url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "ChatW-Webhooks/1.0")
        .header("X-ChatW-Event", event_type)
        .header("X-ChatW-Delivery-Id", deliv_id.to_string())
        .header("X-ChatW-Timestamp", ts.to_string())
        .header("X-ChatW-Signature-256", &sig)
        .body(body_bytes.to_vec())
        .send()
        .await;

    match resp {
        Ok(r) => {
            let code = r.status().as_u16() as i32;
            let preview = r
                .bytes()
                .await
                .map(|b| String::from_utf8_lossy(&b[..b.len().min(512)]).into_owned())
                .unwrap_or_default();
            if (200..300).contains(&code) {
                let _ = sqlx::query("UPDATE plugin_webhooks SET last_used_at = now() WHERE id = $1")
                    .bind(wh_id)
                    .execute(&state.db)
                    .await;
                let _ = sqlx::query(
                    "UPDATE plugin_webhook_deliveries SET status = 'delivered', http_status = $1, attempt_count = $2, response_preview = $3, next_retry_at = NULL, completed_at = now() WHERE id = $4",
                )
                .bind(code)
                .bind(attempt)
                .bind(&preview)
                .bind(deliv_id)
                .execute(&state.db)
                .await;
                json!({ "ok": true, "delivery_id": deliv_id, "http_status": code, "status": "delivered" })
            } else {
                schedule_retry_or_dlq(state, deliv_id, attempt, Some(code), &preview, "non_2xx_response").await
            }
        }
        Err(e) => {
            schedule_retry_or_dlq(state, deliv_id, attempt, None, "", &e.to_string()).await
        }
    }
}

async fn schedule_retry_or_dlq(
    state: &AppState,
    deliv_id: Uuid,
    attempt: i32,
    http_status: Option<i32>,
    preview: &str,
    err: &str,
) -> Value {
    let max_attempts = 4;
    if attempt >= max_attempts {
        let _ = sqlx::query(
            "UPDATE plugin_webhook_deliveries SET status = 'dead_letter', http_status = $1, attempt_count = $2, response_preview = $3, error_message = $4, next_retry_at = NULL, completed_at = now() WHERE id = $5",
        )
        .bind(http_status)
        .bind(attempt)
        .bind(preview)
        .bind(&err[..err.len().min(300)])
        .bind(deliv_id)
        .execute(&state.db)
        .await;
        json!({ "ok": false, "delivery_id": deliv_id, "http_status": http_status, "status": "dead_letter", "attempt": attempt })
    } else {
        // Exponential backoff: 2s, 8s, 32s
        let backoff_secs = 2i64.pow((attempt * 2).min(6) as u32);
        let _ = sqlx::query(
            "UPDATE plugin_webhook_deliveries SET status = 'failed', http_status = $1, attempt_count = $2, response_preview = $3, error_message = $4, next_retry_at = now() + ($5 || ' seconds')::interval WHERE id = $6",
        )
        .bind(http_status)
        .bind(attempt)
        .bind(preview)
        .bind(&err[..err.len().min(300)])
        .bind(backoff_secs.to_string())
        .bind(deliv_id)
        .execute(&state.db)
        .await;
        json!({ "ok": false, "delivery_id": deliv_id, "http_status": http_status, "status": "failed", "attempt": attempt, "retry_in_secs": backoff_secs })
    }
}

/// Background worker retrying failed outgoing webhook deliveries with exponential backoff -> dead-letter queue.
pub fn spawn_webhook_retry_worker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.tick().await;
        loop {
            interval.tick().await;
            let due = sqlx::query(
                "SELECT id, webhook_id, event_type, attempt_count, payload_json
                 FROM plugin_webhook_deliveries
                 WHERE status = 'failed' AND next_retry_at IS NOT NULL AND next_retry_at <= now()
                 ORDER BY next_retry_at ASC LIMIT 20",
            )
            .fetch_all(&state.db)
            .await;

            if let Ok(rows) = due {
                for r in rows {
                    let did: Uuid = r.get("id");
                    let wh_id: Uuid = r.get("webhook_id");
                    let ev: String = r.get("event_type");
                    let att: i32 = r.get("attempt_count");
                    let payload: Value = r.get("payload_json");
                    let body = serde_json::to_vec(&payload).unwrap_or_default();
                    let _ = attempt_outgoing_delivery(&state, did, wh_id, &ev, &body, att + 1).await;
                }
            }
        }
    });
}
