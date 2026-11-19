use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};

pub const MIN_USERNAME_LEN: usize = 2;
pub const MAX_USERNAME_LEN: usize = 32;

#[derive(Deserialize)]
pub struct SetNameRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
}

#[derive(Serialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Serialize)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub relationship: &'static str,
}

/// Normalize username: trim, NFKC, check length and characters, produce display & search versions.
pub fn validate_and_normalize_username(raw: &str) -> AppResult<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("empty_username"));
    }

    // NFKC normalization
    let display_name: String = trimmed.nfkc().collect();
    let char_count = display_name.chars().count();
    if !(MIN_USERNAME_LEN..=MAX_USERNAME_LEN).contains(&char_count) {
        return Err(AppError::BadRequest("invalid_username_length"));
    }

    // Disallow control characters, line breaks, zero-width chars
    for c in display_name.chars() {
        if c.is_control() || ('\u{200B}'..='\u{200F}').contains(&c) || c == '\u{FEFF}' {
            return Err(AppError::BadRequest("illegal_character"));
        }
    }

    let search_norm: String = display_name.to_lowercase().nfkc().collect();
    Ok((display_name, search_norm))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/name", post(handle_set_name))
        .route("/username", axum::routing::put(handle_change_username))
        .route("/search", get(handle_search))
        .route(
            "/profile",
            axum::routing::put(handle_update_profile).post(handle_update_profile),
        )
        .route(
            "/me/profile",
            axum::routing::put(handle_update_profile).post(handle_update_profile),
        )
        .route("/{id}/profile", get(handle_get_profile))
        .route(
            "/avatar",
            post(handle_upload_avatar).layer(DefaultBodyLimit::max(5 * 1024 * 1024)),
        )
        .route(
            "/banner",
            post(handle_upload_banner).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
}

async fn handle_set_name(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetNameRequest>,
) -> AppResult<Json<UserResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let (display_name, search_norm) = validate_and_normalize_username(&payload.name)?;

    // Check if user already set their name
    if user.username.is_some() {
        return Err(AppError::Forbidden("username_already_set"));
    }

    // Update with uniqueness check
    let res = sqlx::query!(
        r#"
        UPDATE users
        SET username = $1, username_norm = $2
        WHERE id = $3 AND username IS NULL
        "#,
        display_name,
        search_norm,
        user.id
    )
    .execute(&state.db)
    .await;

    match res {
        Ok(pg_res) => {
            if pg_res.rows_affected() == 0 {
                return Err(AppError::Forbidden("username_already_set"));
            }
            Ok(Json(UserResponse {
                id: user.id,
                username: display_name,
            }))
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(AppError::Conflict("username_taken"))
        }
        Err(e) => Err(e.into()),
    }
}

/// Change an existing username (same rules as the first-time name).
async fn handle_change_username(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetNameRequest>,
) -> AppResult<Json<UserResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    let (display_name, search_norm) = validate_and_normalize_username(&payload.name)?;
    let res = sqlx::query!(
        "UPDATE users SET username = $1, username_norm = $2 WHERE id = $3",
        display_name,
        search_norm,
        user.id
    )
    .execute(&state.db)
    .await;
    match res {
        Ok(_) => Ok(Json(UserResponse { id: user.id, username: display_name })),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Err(AppError::Conflict("username_taken")),
        Err(e) => Err(e.into()),
    }
}

async fn handle_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<UserSearchResult>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let q_raw = query.q.unwrap_or_default();
    let q_trimmed = q_raw.trim();
    if q_trimmed.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let search_prefix: String = q_trimmed.to_lowercase().nfkc().collect();

    let rows = sqlx::query!(
        r#"
        SELECT
            u.id,
            u.username,
            u.avatar_url,
            CASE
                WHEN EXISTS (
                    SELECT 1 FROM friendships f
                    WHERE (f.user_a = LEAST($1, u.id) AND f.user_b = GREATEST($1, u.id))
                ) THEN 'friend'
                WHEN EXISTS (
                    SELECT 1 FROM friend_requests fr
                    WHERE fr.sender_id = $1 AND fr.receiver_id = u.id AND fr.status = 'pending'
                ) THEN 'pending_sent'
                WHEN EXISTS (
                    SELECT 1 FROM friend_requests fr
                    WHERE fr.sender_id = u.id AND fr.receiver_id = $1 AND fr.status = 'pending'
                ) THEN 'pending_received'
                ELSE 'none'
            END AS "rel!"
        FROM users u
        WHERE u.id <> $1
          AND u.username IS NOT NULL
          AND u.username_norm LIKE $2 || '%'
          AND u.suspended_at IS NULL
          AND (u.privacy_discoverable OR EXISTS (
                SELECT 1 FROM friendships f WHERE f.user_a = LEAST($1, u.id) AND f.user_b = GREATEST($1, u.id)))
          AND NOT EXISTS (
                SELECT 1 FROM user_blocks b
                WHERE (b.blocker_id = u.id AND b.blocked_id = $1) OR (b.blocker_id = $1 AND b.blocked_id = u.id))
        ORDER BY u.username_norm ASC
        LIMIT 20
        "#,
        user.id,
        search_prefix
    )
    .fetch_all(&state.db)
    .await?;

    let results = rows
        .into_iter()
        .map(|r| {
            let rel: &'static str = match r.rel.as_str() {
                "friend" => "friend",
                "pending_sent" => "pending_sent",
                "pending_received" => "pending_received",
                _ => "none",
            };
            UserSearchResult {
                id: r.id,
                username: r.username.unwrap_or_default(),
                avatar_url: r.avatar_url,
                relationship: rel,
            }
        })
        .collect();

    Ok(Json(results))
}

/// Real image type from magic bytes (the Content-Type header is not trusted).
pub fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && matches!(&bytes[8..12], b"avif" | b"avis" | b"mif1" | b"msf1") {
        Some("avif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else {
        None
    }
}

async fn handle_upload_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<AvatarUploadResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if body.is_empty() {
        return Err(AppError::BadRequest("empty_avatar_payload"));
    }

    if body.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("avatar_too_large"));
    }
    // The client re-encodes avatars; anything that is not a still image is refused.
    if !matches!(sniff_image(&body), Some("avif" | "webp" | "png" | "jpg")) {
        return Err(AppError::BadRequest("unsupported_avatar_format"));
    }

    let avatar_dir = state.config.uploads_dir.join("avatars");
    tokio::fs::create_dir_all(&avatar_dir).await.map_err(|e| {
        tracing::error!(error = %e, "failed to create avatar directory");
        AppError::Internal
    })?;

    let filename = format!("{}.avif", user.id);
    let filepath = avatar_dir.join(&filename);

    tokio::fs::write(&filepath, &body).await.map_err(|e| {
        tracing::error!(error = %e, "failed to write avatar file");
        AppError::Internal
    })?;

    let now_ts = chrono::Utc::now().timestamp_millis();
    let avatar_url = format!("/uploads/avatars/{}?t={}", filename, now_ts);

    sqlx::query!(
        r#"
        UPDATE users
        SET avatar_url = $1
        WHERE id = $2
        "#,
        avatar_url,
        user.id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(AvatarUploadResponse { avatar_url }))
}

async fn handle_upload_banner(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if body.is_empty() {
        return Err(AppError::BadRequest("empty_banner_payload"));
    }
    if body.len() > 8 * 1024 * 1024 {
        return Err(AppError::BadRequest("banner_too_large"));
    }

    let banner_dir = state.config.uploads_dir.join("banners");
    tokio::fs::create_dir_all(&banner_dir).await.map_err(|e| {
        tracing::error!(error = %e, "failed to create banner directory");
        AppError::Internal
    })?;

    // Still covers arrive re-encoded (AVIF / WebP / JPEG); GIF and animated WebP covers are
    // uploaded as-is so they keep moving. The stored extension follows the real type.
    let ext = sniff_image(&body).ok_or(AppError::BadRequest("unsupported_banner_format"))?;
    for old in ["avif", "webp", "gif", "png", "jpg"] {
        if old != ext {
            let _ = tokio::fs::remove_file(banner_dir.join(format!("{}.{old}", user.id))).await;
        }
    }
    let filename = format!("{}.{ext}", user.id);
    let filepath = banner_dir.join(&filename);
    tokio::fs::write(&filepath, &body).await.map_err(|e| {
        tracing::error!(error = %e, "failed to write banner file");
        AppError::Internal
    })?;

    let now_ts = chrono::Utc::now().timestamp_millis();
    let banner_url = format!("/uploads/banners/{}?t={}", filename, now_ts);

    sqlx::query!(
        "UPDATE users SET custom_banner_url = $1 WHERE id = $2",
        banner_url,
        user.id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "banner_url": banner_url })))
}

async fn handle_update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::UpdateProfileRequest>,
) -> AppResult<Json<crate::models::UserProfileResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    if let Some(ref bio) = payload.bio {
        if bio.len() > 256 {
            return Err(AppError::BadRequest("bio_too_long"));
        }
    }
    if let Some(ref st) = payload.status_text {
        if st.len() > 128 {
            return Err(AppError::BadRequest("status_text_too_long"));
        }
    }
    if let Some(ref se) = payload.status_emoji {
        if se.len() > 32 {
            return Err(AppError::BadRequest("status_emoji_too_long"));
        }
    }
    if let Some(ref p) = payload.presence {
        if !["online", "idle", "dnd", "offline"].contains(&p.as_str()) {
            return Err(AppError::BadRequest("invalid_presence"));
        }
    }
    if let Some(ref dn) = payload.display_name {
        if dn.chars().count() > 64 {
            return Err(AppError::BadRequest("display_name_too_long"));
        }
    }
    // Banners must come from our own upload endpoint (no arbitrary external URLs).
    if let Some(ref url) = payload.custom_banner_url {
        if !url.is_empty() && !url.starts_with("/uploads/banners/") {
            return Err(AppError::BadRequest("invalid_banner_url"));
        }
    }
    let is_hex = |c: &str| c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    for color in [&payload.profile_color, &payload.profile_accent].into_iter().flatten() {
        if !color.is_empty() && !is_hex(color) {
            return Err(AppError::BadRequest("invalid_profile_color"));
        }
    }
    if let Some(ref d) = payload.avatar_decoration {
        if !["none", "halo", "crown", "sprout", "stars", "cat", "custom"].contains(&d.as_str()) {
            return Err(AppError::BadRequest("invalid_avatar_decoration"));
        }
    }
    if let Some(ref e) = payload.profile_effect {
        if !["none", "sparkle", "snow", "aurora", "bubbles", "confetti", "hearts", "custom"].contains(&e.as_str()) {
            return Err(AppError::BadRequest("invalid_profile_effect"));
        }
    }
    let one_of = |v: &Option<String>, allowed: &[&str], code: &'static str| -> AppResult<()> {
        match v {
            Some(x) if !allowed.contains(&x.as_str()) => Err(AppError::BadRequest(code)),
            _ => Ok(()),
        }
    };
    one_of(&payload.avatar_effect, &["none", "spin", "pulse", "float", "wobble", "rainbow"], "invalid_avatar_effect")?;
    one_of(
        &payload.name_effect,
        &["none", "gradient", "shimmer", "rainbow", "neon", "wave", "glow", "sparkle", "glitch", "pulse", "typing", "reveal", "cycle", "outline", "holo", "breathe"],
        "invalid_name_effect",
    )?;
    one_of(&payload.nameplate, &["none", "aurora", "sunset", "ocean", "galaxy", "sakura", "forest", "custom"], "invalid_nameplate")?;
    one_of(&payload.custom_effect_anim, &["none", "float", "spin", "pulse", "drift", "sway"], "invalid_effect_animation")?;
    // Custom artwork must be our own uploads; empty string clears it.
    for url in [&payload.custom_decoration_url, &payload.custom_nameplate_url, &payload.custom_effect_url].into_iter().flatten() {
        if !url.is_empty() && (!url.starts_with("/uploads/media/") || url.contains("..")) {
            return Err(AppError::BadRequest("invalid_custom_media_url"));
        }
    }
    // Empty string clears an optional color.
    let color_param = |c: &Option<String>| c.as_ref().map(|v| v.trim().to_string());
    let profile_color = color_param(&payload.profile_color);
    let profile_accent = color_param(&payload.profile_accent);

    let row = sqlx::query!(
        r#"
        UPDATE users
        SET bio = COALESCE($1, bio),
            status_text = CASE WHEN $2::text IS NULL THEN status_text WHEN $2 = '' THEN NULL ELSE $2 END,
            status_emoji = CASE WHEN $3::text IS NULL THEN status_emoji WHEN $3 = '' THEN NULL ELSE $3 END,
            presence = COALESCE($4, presence),
            display_name = COALESCE($5, display_name),
            custom_banner_url = COALESCE($6, custom_banner_url),
            social_links = COALESCE($7, social_links),
            profile_color = CASE WHEN $8::text IS NULL THEN profile_color WHEN $8 = '' THEN NULL ELSE $8 END,
            profile_accent = CASE WHEN $9::text IS NULL THEN profile_accent WHEN $9 = '' THEN NULL ELSE $9 END,
            avatar_decoration = COALESCE($10, avatar_decoration),
            profile_effect = COALESCE($11, profile_effect),
            avatar_effect = COALESCE($13, avatar_effect),
            name_effect = COALESCE($14, name_effect),
            nameplate = COALESCE($15, nameplate),
            custom_decoration_url = CASE WHEN $16::text IS NULL THEN custom_decoration_url WHEN $16 = '' THEN NULL ELSE $16 END,
            custom_nameplate_url = CASE WHEN $17::text IS NULL THEN custom_nameplate_url WHEN $17 = '' THEN NULL ELSE $17 END,
            custom_effect_url = CASE WHEN $18::text IS NULL THEN custom_effect_url WHEN $18 = '' THEN NULL ELSE $18 END,
            custom_effect_anim = COALESCE($19, custom_effect_anim)
        WHERE id = $12
        RETURNING id, username, display_name, avatar_url, custom_banner_url, bio, status_text, status_emoji, presence, badges, social_links,
                  profile_color, profile_accent, avatar_decoration, profile_effect, created_at,
                  avatar_effect, name_effect, nameplate, custom_decoration_url, custom_nameplate_url, custom_effect_url, custom_effect_anim
        "#,
        payload.bio,
        payload.status_text,
        payload.status_emoji,
        payload.presence,
        payload.display_name,
        payload.custom_banner_url,
        payload.social_links,
        profile_color,
        profile_accent,
        payload.avatar_decoration,
        payload.profile_effect,
        user.id,
        payload.avatar_effect,
        payload.name_effect,
        payload.nameplate,
        payload.custom_decoration_url,
        payload.custom_nameplate_url,
        payload.custom_effect_url,
        payload.custom_effect_anim
    )
    .fetch_one(&state.db)
    .await?;

    // Broadcast presence update to friends
    let friend_ids: Vec<Uuid> = sqlx::query_scalar!(
        r#"
        SELECT CASE WHEN user_a = $1 THEN user_b ELSE user_a END as "fid!"
        FROM friendships
        WHERE user_a = $1 OR user_b = $1
        "#,
        user.id
    )
    .fetch_all(&state.db)
    .await?;

    let mut recipients = friend_ids;
    recipients.push(user.id);
    state
        .ws_hub
        .send_to_users(
            &recipients,
            crate::websocket::WsEvent::UserPresenceUpdate {
                user_id: user.id,
                presence: row.presence.clone(),
                status_text: row.status_text.clone(),
                status_emoji: row.status_emoji.clone(),
            },
        )
        .await;

    Ok(Json(crate::models::UserProfileResponse {
        id: row.id,
        username: row.username.unwrap_or_default(),
        display_name: row.display_name,
        avatar_url: row.avatar_url,
        custom_banner_url: row.custom_banner_url,
        bio: row.bio,
        status_text: row.status_text,
        status_emoji: row.status_emoji,
        presence: row.presence,
        badges: row.badges,
        social_links: row.social_links,
        profile_color: row.profile_color,
        profile_accent: row.profile_accent,
        avatar_decoration: row.avatar_decoration,
        profile_effect: row.profile_effect,
        member_since: row.created_at,
        avatar_effect: row.avatar_effect,
        name_effect: row.name_effect,
        nameplate: row.nameplate,
        custom_decoration_url: row.custom_decoration_url,
        custom_nameplate_url: row.custom_nameplate_url,
        custom_effect_url: row.custom_effect_url,
        custom_effect_anim: row.custom_effect_anim,
    }))
}

async fn handle_get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(target_id): axum::extract::Path<Uuid>,
) -> AppResult<Json<crate::models::UserProfileResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query!(
        "SELECT id, username, display_name, avatar_url, custom_banner_url, bio, status_text, status_emoji, presence, badges, social_links, profile_color, profile_accent, avatar_decoration, profile_effect, created_at, avatar_effect, name_effect, nameplate, custom_decoration_url, custom_nameplate_url, custom_effect_url, custom_effect_anim FROM users WHERE id = $1",
        target_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("user_not_found"))?;

    Ok(Json(crate::models::UserProfileResponse {
        id: row.id,
        username: row.username.unwrap_or_default(),
        display_name: row.display_name,
        avatar_url: row.avatar_url,
        custom_banner_url: row.custom_banner_url,
        bio: row.bio,
        status_text: row.status_text,
        status_emoji: row.status_emoji,
        presence: row.presence,
        badges: row.badges,
        social_links: row.social_links,
        profile_color: row.profile_color,
        profile_accent: row.profile_accent,
        avatar_decoration: row.avatar_decoration,
        profile_effect: row.profile_effect,
        member_since: row.created_at,
        avatar_effect: row.avatar_effect,
        name_effect: row.name_effect,
        nameplate: row.nameplate,
        custom_decoration_url: row.custom_decoration_url,
        custom_nameplate_url: row.custom_nameplate_url,
        custom_effect_url: row.custom_effect_url,
        custom_effect_anim: row.custom_effect_anim,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_validation_and_normalization() {
        // Valid usernames
        let (display, norm) = validate_and_normalize_username("  Nguyễn Văn A  ").unwrap();
        assert_eq!(display, "Nguyễn Văn A");
        assert_eq!(norm, "nguyễn văn a");

        // Unicode with combining accents normalized
        let (_d1, n1) = validate_and_normalize_username("Hoà").unwrap();
        let (_d2, n2) = validate_and_normalize_username("Hoà").unwrap();
        assert_eq!(n1, n2);

        // Reject too short
        assert!(validate_and_normalize_username("a").is_err());
        assert!(validate_and_normalize_username("   ").is_err());

        // Reject too long (>32 chars)
        let long_name = "a".repeat(33);
        assert!(validate_and_normalize_username(&long_name).is_err());

        // Reject control chars
        assert!(validate_and_normalize_username("user\nname").is_err());
        assert!(validate_and_normalize_username("user\u{0000}").is_err());
        assert!(validate_and_normalize_username("user\u{200B}name").is_err()); // zero width space
    }
}
