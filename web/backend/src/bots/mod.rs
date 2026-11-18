//! Bots (Telegram-style).
//!
//! * Management API (cookie auth): `/api/bots/...` — create, edit, delete, rotate token,
//!   chat with a bot, add a bot to a group.
//! * Bot API (token auth): `/api/bot/{token}/{method}` — `getMe`, `getUpdates`
//!   (long polling) and `sendMessage`.
//!
//! Chats with bots are NOT end-to-end encrypted: the bot (and therefore the server)
//! must read them. The client labels these conversations clearly.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    groups::{get_member_role, manage::is_admin_role},
    models::GroupMessageItem,
    privacy::auth_user,
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_my_bots).post(create_bot))
        .route("/public", get(list_public_bots))
        .route("/chats", get(list_my_bot_chats))
        .route("/by-name/{username}", get(get_bot_by_name))
        .route("/{id}", get(get_bot).put(update_bot).delete(delete_bot))
        .route("/{id}/token", post(rotate_token))
        .route("/{id}/messages", get(list_chat).post(send_to_bot))
        .route("/{id}/groups/{group_id}", post(add_to_group).delete(remove_from_group))
}

pub fn api_routes() -> Router<AppState> {
    Router::new().route("/{token}/{method}", get(bot_api).post(bot_api))
}

fn norm(name: &str) -> String {
    name.trim().to_lowercase().nfkc().collect()
}

fn valid_username(n: &str) -> bool {
    (2..=32).contains(&n.chars().count()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn new_token(bot_id: Uuid) -> String {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("CSPRNG failure");
    format!("{}:{}", &bot_id.simple().to_string()[..10], URL_SAFE_NO_PAD.encode(buf))
}

#[derive(Serialize)]
pub struct BotItem {
    id: Uuid,
    owner_id: Uuid,
    username: String,
    display_name: String,
    description: Option<String>,
    avatar_url: Option<String>,
    is_public: bool,
    verification_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn fetch_bot(db: &PgPool, id: Uuid) -> AppResult<BotItem> {
    let r = sqlx::query!(
        "SELECT id, owner_id, username, display_name, description, avatar_url, is_public, verification_status, token_hint, created_at FROM bots WHERE id = $1",
        id
    )
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound("bot_not_found"))?;
    Ok(BotItem {
        id: r.id,
        owner_id: r.owner_id,
        username: r.username,
        display_name: r.display_name,
        description: r.description,
        avatar_url: r.avatar_url,
        is_public: r.is_public,
        verification_status: r.verification_status,
        token_hint: Some(r.token_hint),
        token: None,
        created_at: r.created_at,
    })
}

async fn owned_bot(db: &PgPool, id: Uuid, owner: Uuid) -> AppResult<BotItem> {
    let b = fetch_bot(db, id).await?;
    if b.owner_id != owner {
        return Err(AppError::Forbidden("not_bot_owner"));
    }
    Ok(b)
}

async fn list_my_bots(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<BotItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar!("SELECT id FROM bots WHERE owner_id = $1 ORDER BY created_at", uid)
        .fetch_all(&state.db)
        .await?;
    let mut out = Vec::new();
    for id in ids {
        out.push(fetch_bot(&state.db, id).await?);
    }
    Ok(Json(out))
}

async fn list_public_bots(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<BotItem>>> {
    let _ = auth_user(&state, &headers).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM bots WHERE is_public AND verification_status = 'verified' ORDER BY created_at DESC LIMIT 50"
    )
    .fetch_all(&state.db)
    .await?;
    let mut out = Vec::new();
    for id in ids {
        let mut b = fetch_bot(&state.db, id).await?;
        b.token_hint = None;
        out.push(b);
    }
    Ok(Json(out))
}

/// Bots the caller has a conversation with (most recent first).
async fn list_my_bot_chats(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Vec<serde_json::Value>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        r#"SELECT b.id, b.username, b.display_name, b.avatar_url, b.description,
                  (SELECT text FROM bot_messages m WHERE m.bot_id = b.id AND m.user_id = $1 ORDER BY seq DESC LIMIT 1) AS "last_text?",
                  (SELECT MAX(seq) FROM bot_messages m WHERE m.bot_id = b.id AND m.user_id = $1) AS "last_seq?"
           FROM bots b
           WHERE EXISTS (SELECT 1 FROM bot_messages m WHERE m.bot_id = b.id AND m.user_id = $1) OR b.owner_id = $1
           ORDER BY 7 DESC NULLS LAST"#,
        uid
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| serde_json::json!({ "id": r.id, "username": r.username, "display_name": r.display_name, "avatar_url": r.avatar_url, "description": r.description, "last_text": r.last_text }))
            .collect(),
    ))
}

async fn get_bot_by_name(State(state): State<AppState>, headers: HeaderMap, Path(username): Path<String>) -> AppResult<Json<BotItem>> {
    let _ = auth_user(&state, &headers).await?;
    let id = sqlx::query_scalar!("SELECT id FROM bots WHERE username_norm = $1", norm(&username))
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("bot_not_found"))?;
    let mut b = fetch_bot(&state.db, id).await?;
    b.token_hint = None;
    Ok(Json(b))
}

async fn get_bot(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<BotItem>> {
    let uid = auth_user(&state, &headers).await?;
    let mut b = fetch_bot(&state.db, id).await?;
    if b.owner_id != uid {
        b.token_hint = None;
    }
    Ok(Json(b))
}

#[derive(Deserialize)]
struct CreateBot {
    username: String,
    display_name: String,
    description: Option<String>,
}

/// Bot names share the namespace with people: a bot may not take another user's
/// username, but it may reuse its owner's own username.
async fn check_bot_name(db: &PgPool, owner: Uuid, username: &str) -> AppResult<()> {
    if !valid_username(username) {
        return Err(AppError::BadRequest("invalid_bot_username"));
    }
    let n = norm(username);
    let taken_by_other = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM users WHERE username_norm = $1 AND id <> $2"#, n, owner)
        .fetch_optional(db)
        .await?;
    if taken_by_other.is_some() {
        return Err(AppError::Conflict("username_taken_by_user"));
    }
    let taken_by_bot = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM bots WHERE username_norm = $1"#, n)
        .fetch_optional(db)
        .await?;
    if taken_by_bot.is_some() {
        return Err(AppError::Conflict("bot_username_taken"));
    }
    Ok(())
}

async fn create_bot(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<CreateBot>) -> AppResult<Json<BotItem>> {
    let uid = auth_user(&state, &headers).await?;
    let username = p.username.trim().to_string();
    check_bot_name(&state.db, uid, &username).await?;
    let display = p.display_name.trim().to_string();
    if display.is_empty() || display.chars().count() > 64 {
        return Err(AppError::BadRequest("invalid_display_name"));
    }
    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "c!" FROM bots WHERE owner_id = $1"#, uid).fetch_one(&state.db).await?;
    if count >= 20 {
        return Err(AppError::BadRequest("bot_limit_reached"));
    }
    let id = Uuid::new_v4();
    let token = new_token(id);
    let hint = format!("{}…", &token[..14]);
    sqlx::query!(
        "INSERT INTO bots (id, owner_id, username, username_norm, display_name, description, token_hash, token_hint) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        id,
        uid,
        username,
        norm(&username),
        display,
        p.description.as_deref().map(str::trim),
        hash_token(&token),
        hint
    )
    .execute(&state.db)
    .await?;
    let _ = crate::audit::log_audit_event(&state.db, uid, "bot_created", Some("bot"), Some(&id.to_string()), None, None, None).await;
    let mut b = fetch_bot(&state.db, id).await?;
    b.token = Some(token);
    Ok(Json(b))
}

#[derive(Deserialize)]
struct UpdateBot {
    display_name: Option<String>,
    description: Option<String>,
    avatar_url: Option<String>,
    is_public: Option<bool>,
}

async fn update_bot(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(p): Json<UpdateBot>) -> AppResult<Json<BotItem>> {
    let uid = auth_user(&state, &headers).await?;
    owned_bot(&state.db, id, uid).await?;
    if let Some(ref d) = p.display_name {
        if d.trim().is_empty() || d.chars().count() > 64 {
            return Err(AppError::BadRequest("invalid_display_name"));
        }
    }
    if let Some(ref d) = p.description {
        if d.chars().count() > 512 {
            return Err(AppError::BadRequest("description_too_long"));
        }
    }
    if let Some(ref a) = p.avatar_url {
        if !a.is_empty() && (!a.starts_with("/uploads/media/") || a.contains("..")) {
            return Err(AppError::BadRequest("invalid_avatar_url"));
        }
    }
    sqlx::query!(
        r#"UPDATE bots SET
            display_name = COALESCE($1, display_name),
            description = COALESCE($2, description),
            avatar_url = CASE WHEN $3::text IS NULL THEN avatar_url WHEN $3 = '' THEN NULL ELSE $3 END,
            is_public = COALESCE($4, is_public),
            verification_status = CASE
                WHEN $4 IS TRUE AND verification_status IN ('none', 'rejected') THEN 'pending'
                WHEN $4 IS FALSE THEN 'none'
                ELSE verification_status END
        WHERE id = $5"#,
        p.display_name.as_deref().map(str::trim),
        p.description.as_deref().map(str::trim),
        p.avatar_url,
        p.is_public,
        id
    )
    .execute(&state.db)
    .await?;
    Ok(Json(fetch_bot(&state.db, id).await?))
}

async fn delete_bot(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    owned_bot(&state.db, id, uid).await?;
    sqlx::query!("DELETE FROM bots WHERE id = $1", id).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn rotate_token(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<BotItem>> {
    let uid = auth_user(&state, &headers).await?;
    owned_bot(&state.db, id, uid).await?;
    let token = new_token(id);
    let hint = format!("{}…", &token[..14]);
    sqlx::query!("UPDATE bots SET token_hash = $1, token_hint = $2 WHERE id = $3", hash_token(&token), hint, id)
        .execute(&state.db)
        .await?;
    let _ = crate::audit::log_audit_event(&state.db, uid, "bot_token_rotated", Some("bot"), Some(&id.to_string()), None, None, None).await;
    let mut b = fetch_bot(&state.db, id).await?;
    b.token = Some(token);
    Ok(Json(b))
}

#[derive(Serialize)]
struct ChatMessage {
    id: Uuid,
    seq: i64,
    from_bot: bool,
    text: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_chat(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Vec<ChatMessage>>> {
    let uid = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        "SELECT id, seq, from_bot, text, created_at FROM bot_messages WHERE bot_id = $1 AND user_id = $2 ORDER BY seq DESC LIMIT 100",
        id,
        uid
    )
    .fetch_all(&state.db)
    .await?;
    let mut out: Vec<ChatMessage> = rows
        .into_iter()
        .map(|r| ChatMessage { id: r.id, seq: r.seq, from_bot: r.from_bot, text: r.text, created_at: r.created_at })
        .collect();
    out.reverse();
    Ok(Json(out))
}

#[derive(Deserialize)]
struct TextReq {
    text: String,
}

async fn send_to_bot(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(p): Json<TextReq>) -> AppResult<Json<ChatMessage>> {
    let uid = auth_user(&state, &headers).await?;
    let text = p.text.trim().to_string();
    if text.is_empty() || text.chars().count() > 4096 {
        return Err(AppError::BadRequest("invalid_text"));
    }
    let bot = fetch_bot(&state.db, id).await?;
    // Private bots only talk to their owner until they are public and verified.
    if bot.owner_id != uid && !(bot.is_public && bot.verification_status == "verified") {
        return Err(AppError::Forbidden("bot_not_public"));
    }
    let r = sqlx::query!(
        "INSERT INTO bot_messages (bot_id, user_id, from_bot, text) VALUES ($1, $2, FALSE, $3) RETURNING id, seq, created_at",
        id,
        uid,
        text
    )
    .fetch_one(&state.db)
    .await?;
    let username = sqlx::query_scalar!("SELECT username FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    enqueue(
        &state.db,
        id,
        serde_json::json!({
            "message": {
                "message_id": r.id, "date": r.created_at.timestamp(), "text": text,
                "from": { "id": uid, "username": username, "is_bot": false },
                "chat": { "id": uid, "type": "private" }
            }
        }),
    )
    .await?;
    Ok(Json(ChatMessage { id: r.id, seq: r.seq, from_bot: false, text, created_at: r.created_at }))
}

async fn enqueue(db: &PgPool, bot_id: Uuid, payload: serde_json::Value) -> AppResult<()> {
    sqlx::query!("INSERT INTO bot_updates (bot_id, payload) VALUES ($1, $2)", bot_id, payload).execute(db).await?;
    // Keep the queue bounded per bot.
    sqlx::query!(
        "DELETE FROM bot_updates WHERE bot_id = $1 AND update_id < (SELECT COALESCE(MAX(update_id), 0) - 1000 FROM bot_updates WHERE bot_id = $1)",
        bot_id
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn add_to_group(State(state): State<AppState>, headers: HeaderMap, Path((id, group_id)): Path<(Uuid, Uuid)>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    let bot = fetch_bot(&state.db, id).await?;
    if bot.owner_id != uid && !(bot.is_public && bot.verification_status == "verified") {
        return Err(AppError::Forbidden("bot_not_public"));
    }
    sqlx::query!(
        "INSERT INTO bot_group_members (bot_id, group_id, added_by) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        id,
        group_id,
        uid
    )
    .execute(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "added": true })))
}

async fn remove_from_group(State(state): State<AppState>, headers: HeaderMap, Path((id, group_id)): Path<(Uuid, Uuid)>) -> AppResult<Json<serde_json::Value>> {
    let uid = auth_user(&state, &headers).await?;
    let role = get_member_role(&state.db, group_id, uid).await?.ok_or(AppError::Forbidden("not_a_group_member"))?;
    if !is_admin_role(&role) {
        return Err(AppError::Forbidden("insufficient_permissions"));
    }
    sqlx::query!("DELETE FROM bot_group_members WHERE bot_id = $1 AND group_id = $2", id, group_id).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "removed": true })))
}

/// Called after a group message is stored: bots in the group receive commands
/// ("/start") and messages that mention them ("@mybot"), like Telegram's privacy mode.
pub async fn on_group_message(db: &PgPool, item: &GroupMessageItem) {
    let bots = match sqlx::query!(
        "SELECT b.id, b.username_norm FROM bot_group_members m JOIN bots b ON b.id = m.bot_id WHERE m.group_id = $1",
        item.group_id
    )
    .fetch_all(db)
    .await
    {
        Ok(b) => b,
        Err(_) => return,
    };
    let lower = item.content.to_lowercase();
    for b in bots {
        let mentioned = lower.contains(&format!("@{}", b.username_norm));
        if item.content.starts_with('/') || mentioned {
            let _ = enqueue(
                db,
                b.id,
                serde_json::json!({
                    "message": {
                        "message_id": item.id, "date": item.created_at.timestamp(), "text": item.content,
                        "from": { "id": item.sender_id, "username": item.sender_name, "is_bot": false },
                        "chat": { "id": item.group_id, "type": "group" }
                    }
                }),
            )
            .await;
        }
    }
}

#[derive(Deserialize, Default)]
struct BotApiParams {
    offset: Option<i64>,
    limit: Option<i64>,
    timeout: Option<u64>,
    chat_id: Option<Uuid>,
    text: Option<String>,
}

fn ok(result: serde_json::Value) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "result": result }))
}

fn fail(code: u16, description: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": false, "error_code": code, "description": description }))
}

/// Token-authenticated Bot API. Parameters may come as JSON body or query string.
async fn bot_api(
    State(state): State<AppState>,
    Path((token, method)): Path<(String, String)>,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<Json<serde_json::Value>> {
    let bot = sqlx::query!("SELECT id, username, display_name FROM bots WHERE token_hash = $1", hash_token(&token))
        .fetch_optional(&state.db)
        .await?;
    let Some(bot) = bot else { return Ok(fail(401, "Unauthorized")) };
    let mut merged = serde_json::Map::new();
    for (k, v) in query.0.iter() {
        merged.insert(k.clone(), serde_json::from_str(v).unwrap_or(serde_json::Value::String(v.clone())));
    }
    if let Some(Json(serde_json::Value::Object(m))) = body {
        merged.extend(m);
    }
    let params: BotApiParams = serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default();

    match method.as_str() {
        "getMe" => Ok(ok(serde_json::json!({ "id": bot.id, "is_bot": true, "username": bot.username, "first_name": bot.display_name }))),
        "getUpdates" => {
            let offset = params.offset.unwrap_or(0);
            if offset > 0 {
                sqlx::query!("DELETE FROM bot_updates WHERE bot_id = $1 AND update_id < $2", bot.id, offset).execute(&state.db).await?;
            }
            let limit = params.limit.unwrap_or(100).clamp(1, 100);
            let wait = params.timeout.unwrap_or(0).min(25);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait);
            loop {
                let rows = sqlx::query!(
                    "SELECT update_id, payload FROM bot_updates WHERE bot_id = $1 AND update_id >= $2 ORDER BY update_id LIMIT $3",
                    bot.id,
                    offset,
                    limit
                )
                .fetch_all(&state.db)
                .await?;
                if !rows.is_empty() || std::time::Instant::now() >= deadline {
                    let list: Vec<serde_json::Value> = rows
                        .into_iter()
                        .map(|r| {
                            let mut v = r.payload;
                            if let Some(o) = v.as_object_mut() {
                                o.insert("update_id".into(), serde_json::json!(r.update_id));
                            }
                            v
                        })
                        .collect();
                    return Ok(ok(serde_json::Value::Array(list)));
                }
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            }
        }
        "sendMessage" => {
            let (Some(chat_id), Some(text)) = (params.chat_id, params.text.map(|t| t.trim().to_string())) else {
                return Ok(fail(400, "chat_id and text are required"));
            };
            if text.is_empty() || text.chars().count() > 4096 {
                return Ok(fail(400, "text must be 1-4096 characters"));
            }
            // Private chat: only with users who started the conversation.
            let started = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM bot_messages WHERE bot_id = $1 AND user_id = $2 LIMIT 1"#, bot.id, chat_id)
                .fetch_optional(&state.db)
                .await?;
            if started.is_some() {
                let r = sqlx::query!(
                    "INSERT INTO bot_messages (bot_id, user_id, from_bot, text) VALUES ($1, $2, TRUE, $3) RETURNING id, seq, created_at",
                    bot.id,
                    chat_id,
                    text
                )
                .fetch_one(&state.db)
                .await?;
                let msg = serde_json::json!({ "bot_id": bot.id, "id": r.id, "seq": r.seq, "from_bot": true, "text": text, "created_at": r.created_at });
                state.ws_hub.send_to_user(&chat_id, WsEvent::BotMessage(msg)).await;
                return Ok(ok(serde_json::json!({ "message_id": r.id, "chat": { "id": chat_id, "type": "private" }, "text": text })));
            }
            // Group chat: the bot must have been added to the group.
            let in_group = sqlx::query!(
                "SELECT g.owner_id FROM bot_group_members m JOIN groups g ON g.id = m.group_id WHERE m.bot_id = $1 AND m.group_id = $2",
                bot.id,
                chat_id
            )
            .fetch_optional(&state.db)
            .await?;
            let Some(g) = in_group else { return Ok(fail(403, "Forbidden: chat not found or bot not a member")) };
            let channel_id = sqlx::query_scalar!(
                "SELECT id FROM group_channels WHERE group_id = $1 ORDER BY position, created_at LIMIT 1",
                chat_id
            )
            .fetch_optional(&state.db)
            .await?;
            let owner = sqlx::query_scalar!(
                "SELECT COALESCE(added_by, $2) AS \"u!\" FROM bot_group_members WHERE bot_id = $1 AND group_id = $3",
                bot.id,
                g.owner_id,
                chat_id
            )
            .fetch_one(&state.db)
            .await?;
            let mid = Uuid::new_v4();
            let r = sqlx::query!(
                "INSERT INTO group_messages (id, group_id, channel_id, sender_id, content, bot_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING seq, created_at",
                mid,
                chat_id,
                channel_id,
                owner,
                text,
                bot.id
            )
            .fetch_one(&state.db)
            .await?;
            let item = GroupMessageItem {
                id: mid,
                seq: r.seq,
                group_id: chat_id,
                channel_id,
                sender_id: owner,
                sender_name: bot.display_name.clone(),
                sender_avatar_url: None,
                content: text.clone(),
                reply_to_id: None,
                is_pinned: false,
                is_edited: false,
                reactions: vec![],
                created_at: r.created_at,
                is_bot: true,
            };
            let members: Vec<Uuid> = sqlx::query_scalar!("SELECT user_id FROM group_members WHERE group_id = $1", chat_id)
                .fetch_all(&state.db)
                .await?;
            state.ws_hub.send_to_users(&members, WsEvent::GroupMessageNew { message: item }).await;
            Ok(ok(serde_json::json!({ "message_id": mid, "chat": { "id": chat_id, "type": "group" }, "text": text })))
        }
        _ => Ok(fail(404, "Not Found: method not found")),
    }
}
