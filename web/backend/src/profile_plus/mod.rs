//! Spec 012: profile layout (sections, order, visibility, size, template),
//! saved layout presets, showcase, guestbook, achievements computed from real
//! activity, mutual friends and shared communities.

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
    privacy::{are_friends, auth_user, is_blocked_either},
};

pub const SECTIONS: [&str; 9] = ["about", "details", "badges", "showcase", "achievements", "guestbook", "communities", "mutual", "links"];
pub const TEMPLATES: [&str; 21] = [
    "minimal", "modern", "anime", "gaming", "developer", "creator", "artist", "music", "retro", "pixel", "neon", "glass",
    "elegant", "dark", "light", "pastel", "nature", "cyber", "vintage", "professional", "community",
];
const MAX_SHOWCASE: i64 = 12;
const MAX_PRESETS: i64 = 10;

/// (id, goal, hidden until unlocked)
pub const ACHIEVEMENTS: [(&str, i64, bool); 15] = [
    ("first_message", 1, false),
    ("chatterbox", 100, false),
    ("first_friend", 1, false),
    ("social", 10, false),
    ("group_founder", 1, false),
    ("community_member", 1, false),
    ("sticker_maker", 1, false),
    ("artist", 1, false),
    ("animator", 1, false),
    ("secure", 1, false),
    ("stylist", 1, false),
    ("storyteller", 4, false),
    ("bot_builder", 1, false),
    ("merchant", 1, false),
    ("night_owl", 1, true),
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me/layout", put(save_layout))
        .route("/me/presets", get(list_presets).post(create_preset))
        .route("/me/presets/{id}", delete(delete_preset))
        .route("/me/showcase", post(add_showcase))
        .route("/me/showcase/order", put(order_showcase))
        .route("/me/showcase/{id}", delete(delete_showcase))
        .route("/guestbook/{id}", delete(delete_entry))
        .route("/{user_id}", get(get_plus))
        .route("/{user_id}/guestbook", post(sign_guestbook))
}

/// Validate and normalise a section list: known ids, no duplicates, every section present once.
pub fn clean_sections(v: &Value) -> Result<Value, &'static str> {
    let arr = v.as_array().ok_or("invalid_sections")?;
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for s in arr {
        let id = s.get("id").and_then(|x| x.as_str()).ok_or("invalid_sections")?;
        if !SECTIONS.contains(&id) || seen.contains(&id) {
            return Err("invalid_sections");
        }
        let size = match s.get("size").and_then(|x| x.as_str()).unwrap_or("normal") {
            "wide" => "wide",
            "normal" => "normal",
            _ => return Err("invalid_sections"),
        };
        seen.push(id);
        out.push(json!({ "id": id, "visible": s.get("visible").and_then(|x| x.as_bool()).unwrap_or(true), "size": size }));
    }
    for id in SECTIONS {
        if !seen.contains(&id) {
            out.push(json!({ "id": id, "visible": true, "size": "normal" }));
        }
    }
    Ok(Value::Array(out))
}

fn default_sections() -> Value {
    clean_sections(&json!([])).unwrap_or(Value::Null)
}

fn valid_ref(kind: &str, url: &str) -> bool {
    if kind == "link" {
        return (url.starts_with("https://") || url.starts_with("http://"))
            && url.len() <= 300
            && !url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '\\'));
    }
    (url.starts_with("/stickers/") || url.starts_with("/uploads/"))
        && !url.contains("..")
        && url.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
}

/// Progress for every achievement, from real activity.
async fn progress(state: &AppState, uid: Uuid) -> AppResult<Vec<(&'static str, i64)>> {
    let r = sqlx::query!(
        r#"SELECT
            (SELECT count(*) FROM messages WHERE sender_id = $1) + (SELECT count(*) FROM group_messages WHERE sender_id = $1) + (SELECT count(*) FROM channel_messages WHERE sender_id = $1) AS "msgs!",
            (SELECT count(*) FROM friendships WHERE user_a = $1 OR user_b = $1) AS "friends!",
            (SELECT count(*) FROM groups WHERE owner_id = $1) AS "groups!",
            (SELECT count(*) FROM server_members WHERE user_id = $1) AS "servers!",
            (SELECT count(*) FROM sticker_packs p WHERE p.author_id = $1 AND EXISTS (SELECT 1 FROM stickers s WHERE s.pack_id = p.id)) AS "packs!",
            (SELECT count(*) FROM drawing_projects WHERE owner_id = $1) AS "drawings!",
            (SELECT count(*) FROM drawing_projects WHERE owner_id = $1 AND frame_count >= 8) AS "animations!",
            (SELECT count(*) FROM bots WHERE owner_id = $1) AS "bots!",
            (SELECT count(*) FROM business_profiles WHERE user_id = $1 AND enabled) AS "biz!",
            (SELECT count(*) FROM security_events WHERE user_id = $1 AND kind IN ('login', 'login_2fa') AND extract(hour FROM created_at AT TIME ZONE 'Asia/Ho_Chi_Minh') < 4) AS "night!",
            u.totp_enabled_at IS NOT NULL AS "secure!",
            (u.avatar_motion IS NOT NULL OR u.name_effect <> 'none' OR u.nameplate <> 'none') AS "style!",
            ((u.bio IS NOT NULL AND u.bio <> '')::int + (u.avatar_url IS NOT NULL)::int + (u.pronouns IS NOT NULL)::int
              + (u.occupation IS NOT NULL)::int + (u.location IS NOT NULL)::int + (cardinality(u.interests) > 0)::int)::bigint AS "story!"
           FROM users u WHERE u.id = $1"#,
        uid
    )
    .fetch_one(&state.db)
    .await?;
    Ok(vec![
        ("first_message", r.msgs),
        ("chatterbox", r.msgs),
        ("first_friend", r.friends),
        ("social", r.friends),
        ("group_founder", r.groups),
        ("community_member", r.servers),
        ("sticker_maker", r.packs),
        ("artist", r.drawings),
        ("animator", r.animations),
        ("secure", r.secure as i64),
        ("stylist", r.style as i64),
        ("storyteller", r.story),
        ("bot_builder", r.bots),
        ("merchant", r.biz),
        ("night_owl", r.night),
    ])
}

/// Compute achievements and remember first unlock times.
async fn achievements(state: &AppState, uid: Uuid) -> AppResult<Vec<Value>> {
    let prog = progress(state, uid).await?;
    let mut out = Vec::new();
    for (id, goal, hidden) in ACHIEVEMENTS {
        let p = prog.iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or(0);
        let unlocked = p >= goal;
        if unlocked {
            sqlx::query!("INSERT INTO user_achievements (user_id, achievement) VALUES ($1, $2) ON CONFLICT DO NOTHING", uid, id)
                .execute(&state.db)
                .await?;
        }
        out.push(json!({ "id": id, "progress": p.min(goal), "goal": goal, "hidden": hidden, "unlocked": unlocked }));
    }
    let times = sqlx::query!("SELECT achievement, unlocked_at FROM user_achievements WHERE user_id = $1", uid).fetch_all(&state.db).await?;
    for a in out.iter_mut() {
        if let Some(t) = times.iter().find(|t| Some(t.achievement.as_str()) == a["id"].as_str()) {
            a["unlocked_at"] = json!(t.unlocked_at);
        }
    }
    Ok(out)
}

async fn get_plus(State(state): State<AppState>, headers: HeaderMap, Path(target): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let is_me = me == target;
    if !is_me && is_blocked_either(&state.db, me, target).await? {
        return Err(AppError::NotFound("user_not_found"));
    }
    let exists = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM users WHERE id = $1"#, target).fetch_optional(&state.db).await?;
    if exists.is_none() {
        return Err(AppError::NotFound("user_not_found"));
    }
    let layout = sqlx::query!("SELECT template, sections, guestbook_mode FROM profile_layouts WHERE user_id = $1", target)
        .fetch_optional(&state.db)
        .await?;
    let (template, sections, gb_mode) = match layout {
        Some(l) => (l.template, clean_sections(&l.sections).unwrap_or_else(|_| default_sections()), l.guestbook_mode),
        None => ("minimal".to_string(), default_sections(), "friends".to_string()),
    };
    let visible = |id: &str| is_me || sections.as_array().is_some_and(|a| a.iter().any(|s| s["id"] == id && s["visible"] == true));
    let friends = !is_me && are_friends(&state.db, me, target).await?;

    let showcase = if visible("showcase") {
        let rows = sqlx::query!("SELECT id, kind, ref_url, title, pinned FROM profile_showcase WHERE user_id = $1 ORDER BY pinned DESC, position, created_at", target)
            .fetch_all(&state.db)
            .await?;
        json!(rows.into_iter().map(|r| json!({ "id": r.id, "kind": r.kind, "url": r.ref_url, "title": r.title, "pinned": r.pinned })).collect::<Vec<_>>())
    } else {
        json!([])
    };
    let ach = if visible("achievements") {
        let all = achievements(&state, target).await?;
        // Others only see unlocked achievements; hidden ones stay secret until unlocked.
        json!(all.into_iter().filter(|a| is_me || a["unlocked"] == true).collect::<Vec<_>>())
    } else {
        json!([])
    };
    let can_sign = !is_me && match gb_mode.as_str() { "everyone" => true, "friends" => friends, _ => false };
    let guestbook = if visible("guestbook") && gb_mode != "off" {
        let rows = sqlx::query!(
            r#"SELECT g.id, g.author_id, g.body, g.created_at, u.username, u.avatar_url
               FROM profile_guestbook g JOIN users u ON u.id = g.author_id
               WHERE g.owner_id = $1 AND NOT EXISTS (SELECT 1 FROM user_blocks b WHERE (b.blocker_id = $2 AND b.blocked_id = g.author_id) OR (b.blocker_id = g.author_id AND b.blocked_id = $2))
               ORDER BY g.created_at DESC LIMIT 30"#,
            target,
            me
        )
        .fetch_all(&state.db)
        .await?;
        json!(rows.into_iter().map(|r| json!({
            "id": r.id, "author_id": r.author_id, "author": r.username, "avatar_url": r.avatar_url, "body": r.body, "at": r.created_at,
            "can_delete": is_me || r.author_id == me
        })).collect::<Vec<_>>())
    } else {
        json!([])
    };
    let mutual = if !is_me && visible("mutual") {
        let rows = sqlx::query!(
            r#"SELECT u.username AS "username?" FROM users u WHERE u.id IN (
                 SELECT CASE WHEN user_a = $1 THEN user_b ELSE user_a END FROM friendships WHERE user_a = $1 OR user_b = $1
                 INTERSECT
                 SELECT CASE WHEN user_a = $2 THEN user_b ELSE user_a END FROM friendships WHERE user_a = $2 OR user_b = $2
               ) ORDER BY u.username LIMIT 50"#,
            me,
            target
        )
        .fetch_all(&state.db)
        .await?;
        json!(rows.into_iter().filter_map(|r| r.username).collect::<Vec<_>>())
    } else {
        json!([])
    };
    let communities = if visible("communities") {
        // Only communities both people are in (no one learns about others' private memberships).
        let rows = sqlx::query!(
            "SELECT s.id, s.name, s.icon_url FROM servers s WHERE EXISTS (SELECT 1 FROM server_members m WHERE m.server_id = s.id AND m.user_id = $1) AND EXISTS (SELECT 1 FROM server_members m WHERE m.server_id = s.id AND m.user_id = $2) ORDER BY s.name LIMIT 30",
            me,
            target
        )
        .fetch_all(&state.db)
        .await?;
        json!(rows.into_iter().map(|r| json!({ "id": r.id, "name": r.name, "icon_url": r.icon_url })).collect::<Vec<_>>())
    } else {
        json!([])
    };
    Ok(Json(json!({
        "user_id": target, "template": template, "sections": sections, "guestbook_mode": gb_mode,
        "showcase": showcase, "achievements": ach, "guestbook": guestbook, "can_sign_guestbook": can_sign && visible("guestbook"),
        "mutual_friends": mutual, "shared_communities": communities,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutBody {
    template: String,
    sections: Value,
    #[serde(default)]
    guestbook_mode: Option<String>,
}

fn check_layout(b: &LayoutBody) -> AppResult<Value> {
    if !TEMPLATES.contains(&b.template.as_str()) {
        return Err(AppError::BadRequest("invalid_template"));
    }
    if b.guestbook_mode.as_deref().is_some_and(|m| !["off", "friends", "everyone"].contains(&m)) {
        return Err(AppError::BadRequest("invalid_guestbook_mode"));
    }
    clean_sections(&b.sections).map_err(AppError::BadRequest)
}

async fn save_layout(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<LayoutBody>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let sections = check_layout(&b)?;
    sqlx::query!(
        r#"INSERT INTO profile_layouts (user_id, template, sections, guestbook_mode) VALUES ($1, $2, $3, COALESCE($4, 'friends'))
           ON CONFLICT (user_id) DO UPDATE SET template = EXCLUDED.template, sections = EXCLUDED.sections,
             guestbook_mode = COALESCE($4, profile_layouts.guestbook_mode), updated_at = now()"#,
        me,
        b.template,
        sections,
        b.guestbook_mode
    )
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "template": b.template, "sections": sections })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetBody {
    name: String,
    template: String,
    sections: Value,
}

async fn list_presets(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let rows = sqlx::query!("SELECT id, name, template, sections FROM profile_layout_presets WHERE user_id = $1 ORDER BY created_at", me)
        .fetch_all(&state.db)
        .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({ "id": r.id, "name": r.name, "template": r.template, "sections": r.sections })).collect::<Vec<_>>())))
}

async fn create_preset(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<PresetBody>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let name = b.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 40 || name.chars().any(|c| c.is_control()) {
        return Err(AppError::BadRequest("invalid_preset_name"));
    }
    let sections = check_layout(&LayoutBody { template: b.template.clone(), sections: b.sections, guestbook_mode: None })?;
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM profile_layout_presets WHERE user_id = $1"#, me).fetch_one(&state.db).await?;
    if n >= MAX_PRESETS {
        return Err(AppError::BadRequest("preset_limit_reached"));
    }
    let id = sqlx::query_scalar!(
        "INSERT INTO profile_layout_presets (user_id, name, template, sections) VALUES ($1, $2, $3, $4) RETURNING id",
        me,
        name,
        b.template,
        sections
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "id": id })))
}

async fn delete_preset(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("DELETE FROM profile_layout_presets WHERE id = $1 AND user_id = $2", id, me).execute(&state.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("preset_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShowcaseBody {
    kind: String,
    url: String,
    #[serde(default)]
    title: Option<String>,
}

async fn add_showcase(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<ShowcaseBody>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    if !["sticker", "emoji", "image", "gif", "link"].contains(&b.kind.as_str()) || !valid_ref(&b.kind, b.url.trim()) {
        return Err(AppError::BadRequest("invalid_showcase_item"));
    }
    let title = b.title.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
    if title.as_ref().is_some_and(|t| t.chars().count() > 40 || t.chars().any(|c| c.is_control())) {
        return Err(AppError::BadRequest("invalid_showcase_title"));
    }
    let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM profile_showcase WHERE user_id = $1"#, me).fetch_one(&state.db).await?;
    if n >= MAX_SHOWCASE {
        return Err(AppError::BadRequest("showcase_full"));
    }
    let id = sqlx::query_scalar!(
        "INSERT INTO profile_showcase (user_id, kind, ref_url, title, position) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        me,
        b.kind,
        b.url.trim(),
        title,
        n as i32
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderBody {
    ids: Vec<Uuid>,
    #[serde(default)]
    pinned: Vec<Uuid>,
}

async fn order_showcase(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<OrderBody>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    if b.ids.len() > MAX_SHOWCASE as usize {
        return Err(AppError::BadRequest("invalid_order"));
    }
    let mut tx = state.db.begin().await?;
    for (i, id) in b.ids.iter().enumerate() {
        // Only my own items are touched (user_id in the WHERE clause).
        sqlx::query!(
            "UPDATE profile_showcase SET position = $1, pinned = $2 WHERE id = $3 AND user_id = $4",
            i as i32,
            b.pinned.contains(id),
            id,
            me
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_showcase(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    let r = sqlx::query!("DELETE FROM profile_showcase WHERE id = $1 AND user_id = $2", id, me).execute(&state.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("item_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryBody {
    body: String,
}

async fn sign_guestbook(State(state): State<AppState>, headers: HeaderMap, Path(owner): Path<Uuid>, Json(b): Json<EntryBody>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    if me == owner {
        return Err(AppError::BadRequest("cannot_sign_own_guestbook"));
    }
    if is_blocked_either(&state.db, me, owner).await? {
        return Err(AppError::Forbidden("guestbook_closed"));
    }
    let body = b.body.trim().to_string();
    if body.is_empty() || body.chars().count() > 300 || body.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::BadRequest("invalid_entry"));
    }
    let layout = sqlx::query!("SELECT guestbook_mode, sections FROM profile_layouts WHERE user_id = $1", owner).fetch_optional(&state.db).await?;
    let (mode, shown) = match layout {
        Some(l) => {
            let s = clean_sections(&l.sections).unwrap_or_else(|_| default_sections());
            let shown = s.as_array().is_some_and(|a| a.iter().any(|x| x["id"] == "guestbook" && x["visible"] == true));
            (l.guestbook_mode, shown)
        }
        None => ("friends".to_string(), true),
    };
    let allowed = shown && match mode.as_str() { "everyone" => true, "friends" => are_friends(&state.db, me, owner).await?, _ => false };
    if !allowed {
        return Err(AppError::Forbidden("guestbook_closed"));
    }
    let recent = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM profile_guestbook WHERE owner_id = $1 AND author_id = $2 AND created_at > now() - interval '1 hour'"#,
        owner,
        me
    )
    .fetch_one(&state.db)
    .await?;
    if recent >= 5 {
        return Err(AppError::TooManyRequests);
    }
    let id = sqlx::query_scalar!("INSERT INTO profile_guestbook (owner_id, author_id, body) VALUES ($1, $2, $3) RETURNING id", owner, me, body)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(json!({ "id": id })))
}

async fn delete_entry(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    let me = auth_user(&state, &headers).await?;
    // The profile owner or the author may remove an entry.
    let r = sqlx::query!("DELETE FROM profile_guestbook WHERE id = $1 AND (owner_id = $2 OR author_id = $2)", id, me).execute(&state.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("entry_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_are_normalised() {
        let s = clean_sections(&json!([{ "id": "guestbook", "visible": false, "size": "wide" }])).unwrap();
        let a = s.as_array().unwrap();
        assert_eq!(a.len(), SECTIONS.len());
        assert_eq!(a[0]["id"], "guestbook");
        assert_eq!(a[0]["visible"], false);
        assert!(clean_sections(&json!([{ "id": "evil" }])).is_err());
        assert!(clean_sections(&json!([{ "id": "about" }, { "id": "about" }])).is_err());
        assert!(clean_sections(&json!([{ "id": "about", "size": "huge" }])).is_err());
    }

    #[test]
    fn showcase_refs() {
        assert!(valid_ref("sticker", "/stickers/openmoji/1F600.svg"));
        assert!(valid_ref("image", "/uploads/media/a-b_c.png"));
        assert!(!valid_ref("image", "/uploads/../secret"));
        assert!(!valid_ref("image", "https://evil.example/x.png"));
        assert!(valid_ref("link", "https://example.com/me"));
        assert!(!valid_ref("link", "javascript:alert(1)"));
        assert_eq!(TEMPLATES.len(), 21);
        assert_eq!(ACHIEVEMENTS.len(), 15);
    }
}
