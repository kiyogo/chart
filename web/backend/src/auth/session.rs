use axum::http::header;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{AppError, AppResult},
};

type HmacSha256 = Hmac<Sha256>;

pub const SESSION_COOKIE_NAME: &str = "chatw_session";
pub const OAUTH_COOKIE_NAME: &str = "chatw_oauth";

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub session_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct SessionRecord {
    pub token_hash: Vec<u8>,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Compute HMAC-SHA256 of the raw 32-byte session token using server session_secret.
pub fn hash_session_token(secret: &[u8], token_bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take any size secret");
    mac.update(token_bytes);
    mac.finalize().into_bytes().into()
}

/// Generate a new 32-byte cryptographically secure random session token.
pub fn generate_session_token() -> [u8; 32] {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("CSPRNG failure");
    buf
}

/// Create a new session in PostgreSQL and return both the raw hex token and the Set-Cookie header value.
/// Operating-system family shown in device labels ("Windows 1", "Linux 2").
pub fn device_family(user_agent: Option<&str>) -> &'static str {
    let ua = user_agent.unwrap_or("");
    if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") {
        "iOS"
    } else if ua.contains("Mac OS") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Linux") || ua.contains("X11") {
        "Linux"
    } else {
        "Devices"
    }
}

pub async fn create_session_raw(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    user_agent: Option<&str>,
) -> AppResult<(String, String)> {
    let token = generate_session_token();
    let token_hash = hash_session_token(&config.session_secret, &token);
    let expires_at = Utc::now() + chrono::Duration::from_std(config.session_ttl).unwrap();

    // Number devices per OS family: the next free "Windows N".
    let family = device_family(user_agent);
    let existing: Vec<Option<String>> = sqlx::query_scalar!(
        "SELECT device_label FROM sessions WHERE user_id = $1 AND device_label LIKE $2",
        user_id,
        format!("{family} %")
    )
    .fetch_all(pool)
    .await?;
    let next = existing
        .iter()
        .flatten()
        .filter_map(|l| l.rsplit(' ').next().and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0)
        + 1;
    let label = format!("{family} {next}");
    let ua_trimmed: Option<String> = user_agent.map(|u| u.chars().take(300).collect());

    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at, device_name, device_label) VALUES ($1, $2, $3, $4, $5)",
        &token_hash[..],
        user_id,
        expires_at,
        ua_trimmed,
        label
    )
    .execute(pool)
    .await?;

    let token_hex = hex::encode(token);
    let max_age = config.session_ttl.as_secs();
    let secure_flag = if config.cookie_secure { "; Secure" } else { "" };

    let cookie_str = format!(
        "{SESSION_COOKIE_NAME}={token_hex}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_flag}"
    );

    Ok((token_hex, cookie_str))
}

/// Create a new session in PostgreSQL and return the Set-Cookie header value.
pub async fn create_session(pool: &PgPool, config: &Config, user_id: Uuid, user_agent: Option<&str>) -> AppResult<String> {
    let (_, cookie_str) = create_session_raw(pool, config, user_id, user_agent).await?;
    Ok(cookie_str)
}

/// Invalidate a session in the database and return a cookie-clearing header.
pub async fn destroy_session(pool: &PgPool, config: &Config, token_hex: &str) -> AppResult<String> {
    if let Some(token_bytes) = hex::decode(token_hex).ok().filter(|b| b.len() == 32) {
        let token_hash = hash_session_token(&config.session_secret, &token_bytes);
        let _ = sqlx::query!(
            "DELETE FROM sessions WHERE token_hash = $1",
            &token_hash[..]
        )
        .execute(pool)
        .await;
    }

    let secure_flag = if config.cookie_secure { "; Secure" } else { "" };
    Ok(format!(
        "{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}"
    ))
}

/// Look up user by session token extracted from cookies.
pub async fn authenticate_session(
    pool: &PgPool,
    config: &Config,
    token_hex: &str,
) -> AppResult<AuthUser> {
    let token_bytes = hex::decode(token_hex).map_err(|_| AppError::Unauthorized)?;
    if token_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }

    let token_hash = hash_session_token(&config.session_secret, &token_bytes);

    let row = sqlx::query(
        r#"
        SELECT s.id AS session_id, s.device_id, s.user_id, u.username, u.avatar_url
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        LEFT JOIN devices d ON d.id = s.device_id
        WHERE s.token_hash = $1 AND s.expires_at > now() AND u.suspended_at IS NULL
          AND (d.trust_state IS NULL OR d.trust_state <> 'revoked')
        "#,
    )
    .bind(&token_hash[..])
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    use sqlx::Row;
    Ok(AuthUser {
        id: row.try_get("user_id")?,
        username: row.try_get("username")?,
        avatar_url: row.try_get("avatar_url")?,
        session_id: row.try_get("session_id")?,
        device_id: row.try_get("device_id")?,
    })
}

/// Helper to parse a named cookie from the `Cookie` header.
pub fn parse_cookie<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let trimmed = pair.trim();
        if let Some((_, v)) = trimmed.split_once('=').filter(|(k, _)| *k == name) {
            return Some(v);
        }
    }
    None
}
