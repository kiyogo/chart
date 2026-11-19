//! Spec 013: per-community profiles, community emoji / stickers, forum threads,
//! events with RSVP, moderation audit log, welcome screen and rules.
//! Every write checks membership or the right permission bit server-side.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    models::permissions as perm,
    privacy::auth_user,
    servers::permissions::has_server_permission,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/servers/{id}/me/profile", get(get_my_profile).put(put_my_profile))
        .route("/servers/{id}/expressions", get(list_expressions).post(add_expression))
        .route("/servers/{id}/expressions/{eid}", delete(delete_expression))
        .route("/servers/{id}/events", get(list_events).post(create_event))
        .route("/servers/{id}/events/{eid}", delete(delete_event))
        .route("/servers/{id}/events/{eid}/rsvp", put(rsvp))
        .route("/servers/{id}/audit-log", get(audit_log))
        .route("/servers/{id}/welcome", get(get_welcome).put(put_welcome))
        .route("/servers/{id}/accept-rules", post(accept_rules))
        .route("/channels/{id}/threads", get(list_threads).post(create_thread))
        .route("/channels/{id}/threads/{tid}", put(update_thread))
        .route("/channels/{id}/threads/{tid}/messages", get(thread_messages).post(post_thread_message))
}

pub async fn audit(state: &AppState, server_id: Uuid, actor: Uuid, action: &str, target: &str) {
    let t: String = target.chars().take(120).collect();
    let _ = sqlx::query!("INSERT INTO server_audit_log (server_id, actor_id, action, target) VALUES ($1, $2, $3, $4)", server_id, actor, action, t)
        .execute(&state.db)
        .await;
}

async fn member(state: &AppState, server_id: Uuid, uid: Uuid) -> AppResult<()> {
    if has_server_permission(&state.db, server_id, uid, perm::VIEW_CHANNELS).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden("not_a_member"))
    }
}

async fn need(state: &AppState, server_id: Uuid, uid: Uuid, bit: i64) -> AppResult<()> {
    if has_server_permission(&state.db, server_id, uid, bit).await? {
        Ok(())
    } else {
        Err(AppError::Forbidden("missing_permission"))
    }
}

fn clean(s: &str, max: usize, code: &'static str) -> AppResult<Option<String>> {
    let t = s.trim();
    if t.chars().count() > max || t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::BadRequest(code));
    }
    Ok(if t.is_empty() { None } else { Some(t.to_string()) })
}

fn safe_upload(u: &str) -> bool {
    u.starts_with("/uploads/") && !u.contains("..") && u.len() <= 200 && u.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

// ---------------------------------------------------------------- community profile

async fn get_my_profile(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("SELECT nickname, avatar_url, bio FROM server_members WHERE server_id = $1 AND user_id = $2", id, me)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Forbidden("not_a_member"))?;
    Ok(Json(json!({ "nickname": r.nickname, "avatar_url": r.avatar_url, "bio": r.bio })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommunityProfile {
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    bio: Option<String>,
}

/// Only changes how I appear in this community; the global profile is untouched.
async fn put_my_profile(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(b): Json<CommunityProfile>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    member(&state, id, me).await?;
    let nickname = clean(b.nickname.as_deref().unwrap_or_default(), 32, "invalid_nickname")?;
    let bio = clean(b.bio.as_deref().unwrap_or_default(), 190, "invalid_bio")?;
    let avatar = b.avatar_url.as_deref().map(str::trim).filter(|u| !u.is_empty()).map(str::to_string);
    if avatar.as_deref().is_some_and(|u| !safe_upload(u)) {
        return Err(AppError::BadRequest("invalid_avatar_url"));
    }
    sqlx::query!("UPDATE server_members SET nickname = $1, avatar_url = $2, bio = $3 WHERE server_id = $4 AND user_id = $5", nickname, avatar, bio, id, me)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "nickname": nickname, "avatar_url": avatar, "bio": bio })))
}

// ---------------------------------------------------------------- emoji / stickers

async fn list_expressions(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    member(&state, id, me).await?;
    let rows = sqlx::query!("SELECT id, kind, name, file_url, is_animated FROM server_expressions WHERE server_id = $1 ORDER BY kind, name", id)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({ "id": r.id, "kind": r.kind, "name": r.name, "file_url": r.file_url, "is_animated": r.is_animated })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewExpression {
    kind: String,
    name: String,
    file_url: String,
    #[serde(default)]
    is_animated: bool,
}

async fn add_expression(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(b): Json<NewExpression>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_EXPRESSIONS).await?;
    if !["emoji", "sticker"].contains(&b.kind.as_str()) {
        return Err(AppError::BadRequest("invalid_kind"));
    }
    let name = b.name.trim().to_lowercase();
    if !(2..=32).contains(&name.len()) || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest("invalid_name"));
    }
    if !safe_upload(&b.file_url) {
        return Err(AppError::BadRequest("invalid_file_url"));
    }
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM server_expressions WHERE server_id = $1 AND kind = $2"#, id, b.kind).fetch_one(&state.db).await?;
    if n >= 100 {
        return Err(AppError::BadRequest("expression_limit_reached"));
    }
    let r = sqlx::query_scalar!(
        "INSERT INTO server_expressions (server_id, kind, name, file_url, is_animated, created_by) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING RETURNING id",
        id,
        b.kind,
        name,
        b.file_url,
        b.is_animated,
        me
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Conflict("name_taken"))?;
    audit(&state, id, me, &format!("{}_added", b.kind), &name).await;
    Ok(Json(json!({ "id": r })))
}

async fn delete_expression(State(state): State<AppState>, headers: HeaderMap, Path((id, eid)): Path<(Uuid, Uuid)>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_EXPRESSIONS).await?;
    let r = sqlx::query!("DELETE FROM server_expressions WHERE id = $1 AND server_id = $2 RETURNING kind, name", eid, id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("expression_not_found"))?;
    audit(&state, id, me, &format!("{}_deleted", r.kind), &r.name).await;
    Ok(Json(json!({ "deleted": true })))
}

// ---------------------------------------------------------------- events

async fn list_events(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    member(&state, id, me).await?;
    let rows = sqlx::query!(
        r#"SELECT e.id, e.title, e.description, e.location, e.starts_at, e.ends_at, e.created_by,
                  (SELECT count(*) FROM server_event_rsvps r WHERE r.event_id = e.id AND r.status = 'going') AS "going!",
                  (SELECT count(*) FROM server_event_rsvps r WHERE r.event_id = e.id AND r.status = 'interested') AS "interested!",
                  (SELECT status FROM server_event_rsvps r WHERE r.event_id = e.id AND r.user_id = $2) AS "mine?"
           FROM server_events e WHERE e.server_id = $1 AND COALESCE(e.ends_at, e.starts_at + interval '3 hours') > now() - interval '1 day'
           ORDER BY e.starts_at LIMIT 100"#,
        id,
        me
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({
        "id": r.id, "title": r.title, "description": r.description, "location": r.location, "starts_at": r.starts_at, "ends_at": r.ends_at,
        "created_by": r.created_by, "going": r.going, "interested": r.interested, "mine": r.mine
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewEvent {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    location: Option<String>,
    starts_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    ends_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn create_event(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(b): Json<NewEvent>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_CHANNELS).await?;
    let title = clean(&b.title, 100, "invalid_title")?.ok_or(AppError::BadRequest("invalid_title"))?;
    let description = clean(b.description.as_deref().unwrap_or_default(), 1000, "invalid_description")?;
    let location = clean(b.location.as_deref().unwrap_or_default(), 120, "invalid_location")?;
    if b.ends_at.is_some_and(|e| e <= b.starts_at) {
        return Err(AppError::BadRequest("invalid_event_time"));
    }
    let eid = sqlx::query_scalar!(
        "INSERT INTO server_events (server_id, title, description, location, starts_at, ends_at, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        id,
        title,
        description,
        location,
        b.starts_at,
        b.ends_at,
        me
    )
    .fetch_one(&state.db)
    .await?;
    audit(&state, id, me, "event_created", &title).await;
    Ok(Json(json!({ "id": eid })))
}

async fn delete_event(State(state): State<AppState>, headers: HeaderMap, Path((id, eid)): Path<(Uuid, Uuid)>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_CHANNELS).await?;
    let t = sqlx::query_scalar!("DELETE FROM server_events WHERE id = $1 AND server_id = $2 RETURNING title", eid, id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("event_not_found"))?;
    audit(&state, id, me, "event_deleted", &t).await;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rsvp {
    /// "going" | "interested" | "none"
    status: String,
}

async fn rsvp(State(state): State<AppState>, headers: HeaderMap, Path((id, eid)): Path<(Uuid, Uuid)>, Json(b): Json<Rsvp>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    member(&state, id, me).await?;
    let ok = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM server_events WHERE id = $1 AND server_id = $2"#, eid, id).fetch_optional(&state.db).await?;
    if ok.is_none() {
        return Err(AppError::NotFound("event_not_found"));
    }
    match b.status.as_str() {
        "none" => {
            sqlx::query!("DELETE FROM server_event_rsvps WHERE event_id = $1 AND user_id = $2", eid, me).execute(&state.db).await?;
        }
        "going" | "interested" => {
            sqlx::query!(
                "INSERT INTO server_event_rsvps (event_id, user_id, status) VALUES ($1, $2, $3) ON CONFLICT (event_id, user_id) DO UPDATE SET status = EXCLUDED.status, updated_at = now()",
                eid,
                me,
                b.status
            )
            .execute(&state.db)
            .await?;
        }
        _ => return Err(AppError::BadRequest("invalid_status")),
    }
    Ok(Json(json!({ "status": b.status })))
}

// ---------------------------------------------------------------- audit log

async fn audit_log(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_SERVER).await?;
    let rows = sqlx::query!(
        r#"SELECT a.action, a.target, a.created_at, u.username AS "actor?" FROM server_audit_log a LEFT JOIN users u ON u.id = a.actor_id
           WHERE a.server_id = $1 ORDER BY a.created_at DESC LIMIT 200"#,
        id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({ "action": r.action, "target": r.target, "at": r.created_at, "actor": r.actor })).collect::<Vec<_>>())))
}

// ---------------------------------------------------------------- welcome + rules

async fn get_welcome(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    member(&state, id, me).await?;
    let s = sqlx::query!("SELECT name, welcome_message, rules, require_rules FROM servers WHERE id = $1", id).fetch_one(&state.db).await?;
    let accepted = sqlx::query_scalar!("SELECT rules_accepted_at FROM server_members WHERE server_id = $1 AND user_id = $2", id, me)
        .fetch_optional(&state.db)
        .await?
        .flatten();
    let channels = sqlx::query!("SELECT id, name, channel_type FROM channels WHERE server_id = $1 AND NOT is_private ORDER BY position, created_at LIMIT 5", id)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(json!({
        "name": s.name, "welcome_message": s.welcome_message, "rules": s.rules, "require_rules": s.require_rules,
        "accepted_at": accepted,
        "channels": channels.into_iter().map(|c| json!({ "id": c.id, "name": c.name, "type": c.channel_type })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Welcome {
    #[serde(default)]
    welcome_message: Option<String>,
    #[serde(default)]
    rules: Option<String>,
    #[serde(default)]
    require_rules: bool,
}

async fn put_welcome(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>, Json(b): Json<Welcome>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    need(&state, id, me, perm::MANAGE_SERVER).await?;
    let w = clean(b.welcome_message.as_deref().unwrap_or_default(), 500, "invalid_welcome")?;
    let r = clean(b.rules.as_deref().unwrap_or_default(), 2000, "invalid_rules")?;
    let require = b.require_rules && r.is_some();
    sqlx::query!("UPDATE servers SET welcome_message = $1, rules = $2, require_rules = $3 WHERE id = $4", w, r, require, id).execute(&state.db).await?;
    audit(&state, id, me, "welcome_updated", if require { "rules required" } else { "" }).await;
    Ok(Json(json!({ "ok": true, "require_rules": require })))
}

async fn accept_rules(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("UPDATE server_members SET rules_accepted_at = now() WHERE server_id = $1 AND user_id = $2", id, me).execute(&state.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::Forbidden("not_a_member"));
    }
    Ok(Json(json!({ "accepted": true })))
}

/// Posting is blocked until the member accepts the rules (when the community requires it).
pub async fn rules_gate(state: &AppState, server_id: Uuid, uid: Uuid) -> AppResult<()> {
    let r = sqlx::query!(
        r#"SELECT s.require_rules, s.owner_id, m.rules_accepted_at AS "accepted?" FROM servers s LEFT JOIN server_members m ON m.server_id = s.id AND m.user_id = $2 WHERE s.id = $1"#,
        server_id,
        uid
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("server_not_found"))?;
    if r.require_rules && r.owner_id != uid && r.accepted.is_none() {
        return Err(AppError::Forbidden("rules_not_accepted"));
    }
    Ok(())
}

// ---------------------------------------------------------------- forum threads

async fn forum_channel(state: &AppState, channel: Uuid) -> AppResult<Uuid> {
    let r = sqlx::query!("SELECT server_id, channel_type FROM channels WHERE id = $1", channel)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("channel_not_found"))?;
    if r.channel_type != "forum" {
        return Err(AppError::BadRequest("not_a_forum"));
    }
    Ok(r.server_id)
}

async fn list_threads(State(state): State<AppState>, headers: HeaderMap, Path(ch): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sid = forum_channel(&state, ch).await?;
    member(&state, sid, me).await?;
    let rows = sqlx::query!(
        r#"SELECT t.id, t.title, t.pinned, t.locked, t.created_at, t.last_activity, u.username AS "author?",
                  (SELECT count(*) FROM channel_messages m WHERE m.thread_id = t.id) AS "replies!"
           FROM forum_threads t LEFT JOIN users u ON u.id = t.author_id WHERE t.channel_id = $1
           ORDER BY t.pinned DESC, t.last_activity DESC LIMIT 200"#,
        ch
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({
        "id": r.id, "title": r.title, "pinned": r.pinned, "locked": r.locked, "created_at": r.created_at,
        "last_activity": r.last_activity, "author": r.author, "replies": r.replies
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewThread {
    title: String,
    body: String,
}

async fn create_thread(State(state): State<AppState>, headers: HeaderMap, Path(ch): Path<Uuid>, Json(b): Json<NewThread>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sid = forum_channel(&state, ch).await?;
    need(&state, sid, me, perm::SEND_MESSAGES).await?;
    rules_gate(&state, sid, me).await?;
    let title = clean(&b.title, 120, "invalid_title")?.ok_or(AppError::BadRequest("invalid_title"))?;
    let body = clean(&b.body, 4000, "invalid_body")?.ok_or(AppError::BadRequest("invalid_body"))?;
    let mut tx = state.db.begin().await?;
    let tid = sqlx::query_scalar!("INSERT INTO forum_threads (channel_id, author_id, title) VALUES ($1, $2, $3) RETURNING id", ch, me, title)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query!("INSERT INTO channel_messages (id, channel_id, sender_id, content, thread_id) VALUES ($1, $2, $3, $4, $5)", Uuid::new_v4(), ch, me, body, tid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "id": tid })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThreadUpdate {
    #[serde(default)]
    pinned: Option<bool>,
    #[serde(default)]
    locked: Option<bool>,
}

async fn update_thread(State(state): State<AppState>, headers: HeaderMap, Path((ch, tid)): Path<(Uuid, Uuid)>, Json(b): Json<ThreadUpdate>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sid = forum_channel(&state, ch).await?;
    need(&state, sid, me, perm::MANAGE_CHANNELS).await?;
    let t = sqlx::query_scalar!(
        "UPDATE forum_threads SET pinned = COALESCE($1, pinned), locked = COALESCE($2, locked) WHERE id = $3 AND channel_id = $4 RETURNING title",
        b.pinned,
        b.locked,
        tid,
        ch
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("thread_not_found"))?;
    if b.locked == Some(true) {
        audit(&state, sid, me, "thread_locked", &t).await;
    }
    Ok(Json(json!({ "ok": true })))
}

async fn thread_messages(State(state): State<AppState>, headers: HeaderMap, Path((ch, tid)): Path<(Uuid, Uuid)>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sid = forum_channel(&state, ch).await?;
    member(&state, sid, me).await?;
    let rows = sqlx::query!(
        r#"SELECT m.id, m.sender_id, m.content, m.created_at, COALESCE(sm.nickname, u.username) AS "name?", COALESCE(sm.avatar_url, u.avatar_url) AS "avatar?"
           FROM channel_messages m JOIN users u ON u.id = m.sender_id
           LEFT JOIN server_members sm ON sm.server_id = $3 AND sm.user_id = m.sender_id
           WHERE m.channel_id = $1 AND m.thread_id = $2 ORDER BY m.seq LIMIT 500"#,
        ch,
        tid,
        sid
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({
        "id": r.id, "sender_id": r.sender_id, "sender_name": r.name, "sender_avatar_url": r.avatar, "content": r.content, "created_at": r.created_at
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    content: String,
}

async fn post_thread_message(State(state): State<AppState>, headers: HeaderMap, Path((ch, tid)): Path<(Uuid, Uuid)>, Json(b): Json<Reply>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sid = forum_channel(&state, ch).await?;
    need(&state, sid, me, perm::SEND_MESSAGES).await?;
    rules_gate(&state, sid, me).await?;
    let locked = sqlx::query_scalar!("SELECT locked FROM forum_threads WHERE id = $1 AND channel_id = $2", tid, ch)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("thread_not_found"))?;
    if locked && !has_server_permission(&state.db, sid, me, perm::MANAGE_CHANNELS).await? {
        return Err(AppError::Forbidden("thread_locked"));
    }
    let content = clean(&b.content, 4000, "invalid_body")?.ok_or(AppError::BadRequest("invalid_body"))?;
    let id = Uuid::new_v4();
    sqlx::query!("INSERT INTO channel_messages (id, channel_id, sender_id, content, thread_id) VALUES ($1, $2, $3, $4, $5)", id, ch, me, content, tid)
        .execute(&state.db)
        .await?;
    sqlx::query!("UPDATE forum_threads SET last_activity = now() WHERE id = $1", tid).execute(&state.db).await?;
    Ok(Json(json!({ "id": id })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_paths() {
        assert!(safe_upload("/uploads/media/a.png"));
        assert!(!safe_upload("/uploads/../etc/passwd"));
        assert!(!safe_upload("https://evil.example/a.png"));
        assert!(clean("  hi  ", 5, "x").unwrap() == Some("hi".into()));
        assert!(clean("bad\u{7}", 10, "x").is_err());
        assert_eq!(perm::MANAGE_EXPRESSIONS, 1024);
    }
}
