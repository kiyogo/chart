//! Spec 009: avatar motion, richer profile details with per-field visibility,
//! custom-status expiry, per-account settings sync and private contact notes.
//!
//! Every value from the client is validated here; nothing is trusted. Avatar
//! motion is a small declarative object (effect id + bounded numbers), never
//! code, so a profile can't run anything in another user's app.

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::{are_friends, auth_user, is_blocked_either},
    websocket::WsEvent,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profile-ext/me", put(update_me))
        .route("/profile-ext/{user_id}", get(get_profile_ext))
        .route("/settings-sync", get(get_settings).put(put_settings))
        .route("/contacts/notes", get(list_notes))
        .route("/contacts/{user_id}/note", put(put_note))
}

/// Avatar effects, in the order the studio shows them. Names live in the client (English).
pub const MOTION_EFFECTS: [&str; 40] = [
    "spin", "pulse", "breathe", "bounce", "float", "shake", "nod", "tilt", "jitter", "wobble",
    "zoom", "swing", "flip", "glow", "flash", "ring-cycle", "hue", "rainbow", "gradient-ring", "halo",
    "orbit", "sparkle", "stars", "hearts", "snow", "rain", "fire", "electric", "smoke", "bubbles",
    "ripple", "wave", "heartbeat", "hologram", "glitch", "pixel", "scanline", "shine", "confetti", "morph",
];
const TRIGGERS: [&str; 3] = ["always", "hover", "click"];
const EASINGS: [&str; 5] = ["linear", "ease", "ease-in-out", "ease-out", "bounce"];
const DETAIL_KEYS: [&str; 5] = ["pronouns", "location", "website", "occupation", "interests"];
const VISIBILITY: [&str; 3] = ["everyone", "friends", "nobody"];
const MAX_SETTINGS_BYTES: usize = 32 * 1024;

fn default_speed() -> f64 {
    1.0
}
fn default_intensity() -> u8 {
    60
}
fn default_trigger() -> String {
    "always".into()
}
fn default_easing() -> String {
    "linear".into()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvatarMotion {
    pub effect: String,
    /// Playback speed multiplier, 0.25 – 4.
    #[serde(default = "default_speed")]
    pub speed: f64,
    /// 1 – 100.
    #[serde(default = "default_intensity")]
    pub intensity: u8,
    /// `#RRGGBB` or none for the profile accent.
    #[serde(default)]
    pub color: Option<String>,
    /// Counter-clockwise / reversed direction.
    #[serde(default)]
    pub reverse: bool,
    #[serde(default = "default_trigger")]
    pub trigger: String,
    #[serde(default = "default_easing")]
    pub easing: String,
    /// 0 = forever, otherwise 1 – 10 runs per trigger.
    #[serde(default)]
    pub repeat: u8,
}

fn is_hex_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn validate_motion(m: &AvatarMotion) -> Result<(), &'static str> {
    if !MOTION_EFFECTS.contains(&m.effect.as_str()) {
        return Err("invalid_motion_effect");
    }
    if !(0.25..=4.0).contains(&m.speed) || m.speed.is_nan() {
        return Err("invalid_motion_speed");
    }
    if !(1..=100).contains(&m.intensity) {
        return Err("invalid_motion_intensity");
    }
    if m.color.as_deref().is_some_and(|c| !is_hex_color(c)) {
        return Err("invalid_motion_color");
    }
    if !TRIGGERS.contains(&m.trigger.as_str()) {
        return Err("invalid_motion_trigger");
    }
    if !EASINGS.contains(&m.easing.as_str()) {
        return Err("invalid_motion_easing");
    }
    if m.repeat > 10 {
        return Err("invalid_motion_repeat");
    }
    Ok(())
}

/// Parse a stored or submitted motion; anything invalid becomes `None`.
pub fn parse_motion(v: Option<&Value>) -> Option<AvatarMotion> {
    let m: AvatarMotion = serde_json::from_value(v?.clone()).ok()?;
    validate_motion(&m).ok().map(|_| m)
}

fn clean_text(s: &str, max: usize, code: &'static str) -> AppResult<Option<String>> {
    let t = s.trim();
    if t.chars().any(|c| c.is_control()) || t.chars().count() > max {
        return Err(AppError::BadRequest(code));
    }
    Ok(if t.is_empty() { None } else { Some(t.to_string()) })
}

fn valid_website(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 200
        && !url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
        && url.splitn(4, '/').nth(2).is_some_and(|host| !host.is_empty())
}

fn visibility_of(map: &Value, key: &str) -> String {
    let default = if key == "location" { "friends" } else { "everyone" };
    map.get(key).and_then(|v| v.as_str()).filter(|v| VISIBILITY.contains(v)).unwrap_or(default).to_string()
}

async fn get_profile_ext(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    if me != target && is_blocked_either(&state.db, me, target).await? {
        return Err(AppError::NotFound("user_not_found"));
    }
    let row = sqlx::query!(
        r#"SELECT avatar_motion, status_expires_at, pronouns, location, website, occupation, interests, detail_visibility
           FROM users WHERE id = $1"#,
        target
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("user_not_found"))?;
    let is_me = me == target;
    let friends = !is_me && are_friends(&state.db, me, target).await?;
    let can_see = |key: &str| -> bool {
        is_me
            || match visibility_of(&row.detail_visibility, key).as_str() {
                "everyone" => true,
                "friends" => friends,
                _ => false,
            }
    };
    let mut details = serde_json::Map::new();
    if can_see("pronouns") {
        details.insert("pronouns".into(), json!(row.pronouns));
    }
    if can_see("location") {
        details.insert("location".into(), json!(row.location));
    }
    if can_see("website") {
        details.insert("website".into(), json!(row.website));
    }
    if can_see("occupation") {
        details.insert("occupation".into(), json!(row.occupation));
    }
    if can_see("interests") {
        details.insert("interests".into(), json!(row.interests));
    }
    let note = if is_me {
        None
    } else {
        sqlx::query!("SELECT nickname, note FROM contact_notes WHERE owner_id = $1 AND target_id = $2", me, target)
            .fetch_optional(&state.db)
            .await?
    };
    let mut out = json!({
        "user_id": target,
        "avatar_motion": parse_motion(row.avatar_motion.as_ref()),
        "details": details,
        "nickname": note.as_ref().and_then(|n| n.nickname.clone()),
        "note": note.as_ref().and_then(|n| n.note.clone()),
    });
    if is_me {
        let mut vis = serde_json::Map::new();
        for k in DETAIL_KEYS {
            vis.insert(k.into(), json!(visibility_of(&row.detail_visibility, k)));
        }
        out["detail_visibility"] = Value::Object(vis);
        out["status_expires_at"] = json!(row.status_expires_at);
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateMe {
    /// Absent = unchanged, `null` = no motion, object = new motion.
    #[serde(default)]
    avatar_motion: Option<Value>,
    #[serde(default)]
    pronouns: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    occupation: Option<String>,
    #[serde(default)]
    interests: Option<Vec<String>>,
    #[serde(default)]
    detail_visibility: Option<serde_json::Map<String, Value>>,
    /// Minutes until the custom status clears; 0 = never.
    #[serde(default)]
    status_expires_in_minutes: Option<i64>,
}

async fn update_me(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<UpdateMe>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let mut tx = state.db.begin().await?;

    if let Some(v) = p.avatar_motion.as_ref() {
        let stored = if v.is_null() {
            None
        } else {
            let m: AvatarMotion = serde_json::from_value(v.clone()).map_err(|_| AppError::BadRequest("invalid_motion"))?;
            validate_motion(&m).map_err(AppError::BadRequest)?;
            Some(serde_json::to_value(&m).map_err(|_| AppError::Internal)?)
        };
        // Keep the legacy one-word column in step for older clients.
        let legacy = stored.as_ref().and_then(|s| s.get("effect")).and_then(|e| e.as_str()).map(|e| {
            if ["spin", "pulse", "float", "wobble", "rainbow"].contains(&e) { e.to_string() } else { "none".to_string() }
        });
        sqlx::query!(
            "UPDATE users SET avatar_motion = $1, avatar_effect = COALESCE($2, 'none') WHERE id = $3",
            stored,
            legacy,
            me
        )
        .execute(&mut *tx)
        .await?;
    }
    let pronouns = p.pronouns.as_deref().map(|s| clean_text(s, 40, "invalid_pronouns")).transpose()?;
    let location = p.location.as_deref().map(|s| clean_text(s, 64, "invalid_location")).transpose()?;
    let occupation = p.occupation.as_deref().map(|s| clean_text(s, 64, "invalid_occupation")).transpose()?;
    let website = match p.website.as_deref().map(str::trim) {
        None => None,
        Some("") => Some(None),
        Some(w) if valid_website(w) => Some(Some(w.to_string())),
        Some(_) => return Err(AppError::BadRequest("invalid_website")),
    };
    // `Some(None)` clears a field, `None` leaves it unchanged.
    sqlx::query!(
        r#"UPDATE users SET
             pronouns = CASE WHEN $1 THEN $2 ELSE pronouns END,
             location = CASE WHEN $3 THEN $4 ELSE location END,
             occupation = CASE WHEN $5 THEN $6 ELSE occupation END,
             website = CASE WHEN $7 THEN $8 ELSE website END
           WHERE id = $9"#,
        pronouns.is_some(),
        pronouns.clone().flatten(),
        location.is_some(),
        location.clone().flatten(),
        occupation.is_some(),
        occupation.clone().flatten(),
        website.is_some(),
        website.clone().flatten(),
        me
    )
    .execute(&mut *tx)
    .await?;
    if let Some(list) = p.interests.as_ref() {
        if list.len() > 12 {
            return Err(AppError::BadRequest("too_many_interests"));
        }
        let mut clean: Vec<String> = Vec::new();
        for i in list {
            if let Some(t) = clean_text(i, 32, "invalid_interest")? {
                if !clean.iter().any(|c| c.to_lowercase() == t.to_lowercase()) {
                    clean.push(t);
                }
            }
        }
        sqlx::query!("UPDATE users SET interests = $1 WHERE id = $2", &clean, me).execute(&mut *tx).await?;
    }
    if let Some(vis) = p.detail_visibility.as_ref() {
        let mut clean = serde_json::Map::new();
        for (k, v) in vis {
            let val = v.as_str().unwrap_or_default();
            if !DETAIL_KEYS.contains(&k.as_str()) || !VISIBILITY.contains(&val) {
                return Err(AppError::BadRequest("invalid_visibility"));
            }
            clean.insert(k.clone(), json!(val));
        }
        sqlx::query!(
            "UPDATE users SET detail_visibility = detail_visibility || $1 WHERE id = $2",
            Value::Object(clean),
            me
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(minutes) = p.status_expires_in_minutes {
        if !(0..=10_080).contains(&minutes) {
            return Err(AppError::BadRequest("invalid_status_expiry"));
        }
        sqlx::query!(
            "UPDATE users SET status_expires_at = CASE WHEN $1::bigint = 0 THEN NULL ELSE now() + make_interval(mins => $1::int) END WHERE id = $2",
            minutes,
            me
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Friends (and my other devices) refresh the avatar / details they show.
    let mut targets = friend_ids(&state, me).await?;
    targets.push(me);
    state.ws_hub.send_to_users(&targets, WsEvent::ProfileUpdated(json!({ "user_id": me }))).await;
    get_profile_ext(State(state), headers, Path(me)).await
}

async fn friend_ids(state: &AppState, me: Uuid) -> AppResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT CASE WHEN user_a = $1 THEN user_b ELSE user_a END AS "fid!" FROM friendships WHERE user_a = $1 OR user_b = $1"#,
        me
    )
    .fetch_all(&state.db)
    .await?)
}

/// Clears custom statuses whose time is up and tells friends. Runs every minute.
pub fn spawn_status_expiry(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            let rows = sqlx::query!(
                r#"UPDATE users SET status_text = NULL, status_emoji = NULL, status_expires_at = NULL
                   WHERE status_expires_at IS NOT NULL AND status_expires_at <= now()
                   RETURNING id, presence"#
            )
            .fetch_all(&state.db)
            .await;
            let Ok(rows) = rows else { continue };
            for r in rows {
                let mut targets = friend_ids(&state, r.id).await.unwrap_or_default();
                targets.push(r.id);
                state
                    .ws_hub
                    .send_to_users(
                        &targets,
                        WsEvent::UserPresenceUpdate { user_id: r.id, presence: r.presence, status_text: None, status_emoji: None },
                    )
                    .await;
            }
        }
    });
}

// ---------- Settings sync ----------

async fn get_settings(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let row = sqlx::query!("SELECT data, version, updated_at FROM user_settings WHERE user_id = $1", me)
        .fetch_optional(&state.db)
        .await?;
    Ok(Json(match row {
        Some(r) => json!({ "data": r.data, "version": r.version, "updated_at": r.updated_at }),
        None => json!({ "data": {}, "version": 0, "updated_at": null }),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PutSettings {
    data: serde_json::Map<String, Value>,
    /// The version this write is based on; a stale base is rejected with 409.
    base_version: i64,
}

/// Keys that must never leave the device (PIN lock material).
const DEVICE_ONLY_KEYS: [&str; 5] = ["pin_salt", "pin_hash", "lock_enabled", "lock_minutes", "lock_on_start"];

async fn put_settings(State(state): State<AppState>, headers: HeaderMap, Json(p): Json<PutSettings>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let mut data = p.data;
    for k in DEVICE_ONLY_KEYS {
        data.remove(k);
    }
    let blob = Value::Object(data);
    if serde_json::to_vec(&blob).map(|b| b.len()).unwrap_or(usize::MAX) > MAX_SETTINGS_BYTES {
        return Err(AppError::BadRequest("settings_too_large"));
    }
    // Insert the first version, or bump only when the client saw the latest one.
    let updated = sqlx::query!(
        r#"INSERT INTO user_settings (user_id, data, version) VALUES ($1, $2, 1)
           ON CONFLICT (user_id) DO UPDATE SET data = EXCLUDED.data, version = user_settings.version + 1, updated_at = now()
           WHERE user_settings.version = $3
           RETURNING version"#,
        me,
        blob,
        p.base_version
    )
    .fetch_optional(&state.db)
    .await?;
    match updated {
        Some(r) => {
            state.ws_hub.send_to_user(&me, WsEvent::PrefsUpdated(json!({ "kind": "settings", "version": r.version }))).await;
            Ok(Json(json!({ "version": r.version })))
        }
        None => Err(AppError::Conflict("settings_version_conflict")),
    }
}

// ---------- Contact nickname + private note ----------

async fn list_notes(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let rows = sqlx::query!(
        "SELECT target_id, nickname, note FROM contact_notes WHERE owner_id = $1 ORDER BY updated_at DESC LIMIT 2000",
        me
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(
        rows.into_iter()
            .map(|r| json!({ "user_id": r.target_id, "nickname": r.nickname, "note": r.note }))
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PutNote {
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

async fn put_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target): Path<Uuid>,
    Json(p): Json<PutNote>,
) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    if me == target {
        return Err(AppError::BadRequest("cannot_note_self"));
    }
    let exists = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM users WHERE id = $1"#, target).fetch_optional(&state.db).await?;
    if exists.is_none() {
        return Err(AppError::NotFound("user_not_found"));
    }
    let nickname = p.nickname.as_deref().map(|s| clean_text(s, 40, "invalid_nickname")).transpose()?.flatten();
    let note = match p.note.as_deref() {
        Some(n) if n.chars().count() > 500 || n.chars().any(|c| c.is_control() && c != '\n') => {
            return Err(AppError::BadRequest("invalid_note"));
        }
        Some(n) if !n.trim().is_empty() => Some(n.trim().to_string()),
        _ => None,
    };
    if nickname.is_none() && note.is_none() {
        sqlx::query!("DELETE FROM contact_notes WHERE owner_id = $1 AND target_id = $2", me, target).execute(&state.db).await?;
    } else {
        sqlx::query!(
            r#"INSERT INTO contact_notes (owner_id, target_id, nickname, note) VALUES ($1, $2, $3, $4)
               ON CONFLICT (owner_id, target_id) DO UPDATE SET nickname = EXCLUDED.nickname, note = EXCLUDED.note, updated_at = now()"#,
            me,
            target,
            nickname,
            note
        )
        .execute(&state.db)
        .await?;
    }
    state.ws_hub.send_to_user(&me, WsEvent::PrefsUpdated(json!({ "kind": "contacts" }))).await;
    Ok(Json(json!({ "user_id": target, "nickname": nickname, "note": note })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(effect: &str) -> AvatarMotion {
        serde_json::from_value(json!({ "effect": effect })).unwrap()
    }

    #[test]
    fn motion_validation() {
        assert!(validate_motion(&m("spin")).is_ok());
        assert_eq!(MOTION_EFFECTS.len(), 40);
        assert!(validate_motion(&m("evil")).is_err());
        let mut x = m("orbit");
        x.speed = 5.0;
        assert!(validate_motion(&x).is_err());
        x.speed = 2.0;
        x.color = Some("red;}".into());
        assert!(validate_motion(&x).is_err());
        x.color = Some("#12AB34".into());
        x.trigger = "hover".into();
        assert!(validate_motion(&x).is_ok());
        // Unknown fields (e.g. script-like payloads) are rejected outright.
        assert!(serde_json::from_value::<AvatarMotion>(json!({ "effect": "spin", "css": "x" })).is_err());
        assert!(parse_motion(Some(&json!({ "effect": "spin", "speed": 9 }))).is_none());
    }

    #[test]
    fn website_and_text_checks() {
        assert!(valid_website("https://example.com/me"));
        assert!(!valid_website("javascript:alert(1)"));
        assert!(!valid_website("https://"));
        assert!(!valid_website("https://a.com/\"x"));
        assert!(clean_text("ok", 5, "x").unwrap().is_some());
        assert!(clean_text("bad\u{0007}", 10, "x").is_err());
        assert_eq!(visibility_of(&json!({}), "location"), "friends");
        assert_eq!(visibility_of(&json!({ "website": "nobody" }), "website"), "nobody");
    }
}
