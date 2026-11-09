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
        .route("/search", get(handle_search))
        .route(
            "/avatar",
            post(handle_upload_avatar).layer(DefaultBodyLimit::max(5 * 1024 * 1024)),
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
