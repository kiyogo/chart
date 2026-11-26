use axum::http::{HeaderMap, header};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::AppState,
    cache::valkey::RedisSession,
    config::Config,
    error::{AppError, AppResult},
};

type HmacSha256 = Hmac<Sha256>;
use sha2::Sha256;

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

#[derive(Clone, Debug)]
pub struct SessionSummary {
    pub id: Uuid,
    pub device_name: Option<String>,
    pub device_label: Option<String>,
    pub ip_address: Option<String>,
    pub last_active: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
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

fn next_device_label(family: &str, existing_labels: &[Option<String>]) -> String {
    let next = existing_labels
        .iter()
        .flatten()
        .filter_map(|l| l.rsplit(' ').next().and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0)
        + 1;
    format!("{family} {next}")
}

fn format_session_cookie(config: &Config, token_hex: &str) -> String {
    let max_age = config.session_ttl.as_secs();
    let secure_flag = if config.cookie_secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE_NAME}={token_hex}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_flag}"
    )
}

/// Create a new session in Redis (when Valkey is configured) or PG fallback (tests without Valkey).
pub async fn create_session_for_state(
    state: &AppState,
    user_id: Uuid,
    user_agent: Option<&str>,
) -> AppResult<(String, String)> {
    let token = generate_session_token();
    let token_hash = hash_session_token(&state.config.session_secret, &token);
    let token_hash_hex = hex::encode(token_hash);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::from_std(state.config.session_ttl).unwrap();
    let family = device_family(user_agent);
    let ua_trimmed: Option<String> = user_agent.map(|u| u.chars().take(300).collect());

    if state.valkey.is_configured() {
        let existing_labels: Vec<Option<String>> = state
            .valkey
            .session_list_for_user(user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.device_label)
            .filter(|l| l.as_deref().is_some_and(|v| v.starts_with(&format!("{family} "))))
            .collect();
        let label = next_device_label(family, &existing_labels);
        let sess = RedisSession {
            id: Uuid::new_v4(),
            token_hash_hex,
            user_id,
            device_id: None,
            device_name: ua_trimmed,
            device_label: Some(label),
            ip_address: None,
            created_at_unix: now.timestamp(),
            last_active_unix: now.timestamp(),
            expires_at_unix: expires_at.timestamp(),
        };
        state
            .valkey
            .session_store(&sess, state.config.session_ttl.as_secs())
            .await
            .map_err(|_| AppError::Internal)?;
        let token_hex = hex::encode(token);
        let cookie_str = format_session_cookie(&state.config, &token_hex);
        return Ok((token_hex, cookie_str));
    }

    create_session_raw_pg(&state.db, &state.config, user_id, user_agent).await
}

/// Legacy/test PostgreSQL session creation (used when Valkey is disabled in tests).
pub async fn create_session_raw(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    user_agent: Option<&str>,
) -> AppResult<(String, String)> {
    create_session_raw_pg(pool, config, user_id, user_agent).await
}

async fn create_session_raw_pg(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    user_agent: Option<&str>,
) -> AppResult<(String, String)> {
    let token = generate_session_token();
    let token_hash = hash_session_token(&config.session_secret, &token);
    let expires_at = Utc::now() + chrono::Duration::from_std(config.session_ttl).unwrap();

    let family = device_family(user_agent);
    let existing: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT device_label FROM sessions WHERE user_id = $1 AND device_label LIKE $2",
    )
    .bind(user_id)
    .bind(format!("{family} %"))
    .fetch_all(pool)
    .await?;
    let label = next_device_label(family, &existing);
    let ua_trimmed: Option<String> = user_agent.map(|u| u.chars().take(300).collect());

    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at, device_name, device_label) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&token_hash[..])
    .bind(user_id)
    .bind(expires_at)
    .bind(ua_trimmed)
    .bind(label)
    .execute(pool)
    .await?;

    let token_hex = hex::encode(token);
    let cookie_str = format_session_cookie(config, &token_hex);
    Ok((token_hex, cookie_str))
}

/// Create a new session in PostgreSQL (used by test suites) and return the Set-Cookie header value.
pub async fn create_session(pool: &PgPool, config: &Config, user_id: Uuid, user_agent: Option<&str>) -> AppResult<String> {
    let (_, cookie_str) = create_session_raw_pg(pool, config, user_id, user_agent).await?;
    Ok(cookie_str)
}

pub async fn destroy_session_for_state(state: &AppState, token_hex: &str) -> AppResult<String> {
    if let Some(token_bytes) = hex::decode(token_hex).ok().filter(|b| b.len() == 32) {
        let token_hash = hash_session_token(&state.config.session_secret, &token_bytes);
        let token_hash_hex = hex::encode(token_hash);
        if state.valkey.is_configured() {
            let _ = state.valkey.session_delete_by_hash(&token_hash_hex).await;
        } else {
            let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
                .bind(&token_hash[..])
                .execute(&state.db)
                .await;
        }
    }

    let secure_flag = if state.config.cookie_secure { "; Secure" } else { "" };
    Ok(format!(
        "{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}"
    ))
}

/// Validate user status and bound device_id in PostgreSQL on every authenticated request.
async fn verify_user_and_device_binding(
    pool: &PgPool,
    user_id: Uuid,
    session_id: Option<Uuid>,
    device_id: Option<Uuid>,
    presented_device_header: Option<&str>,
) -> AppResult<AuthUser> {
    let user_row = sqlx::query(
        "SELECT username, avatar_url, suspended_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let suspended_at: Option<DateTime<Utc>> = user_row.try_get("suspended_at")?;
    if suspended_at.is_some() {
        return Err(AppError::Unauthorized);
    }

    if let Some(did) = device_id {
        let dev_row = sqlx::query(
            "SELECT trust_state, client_device_id FROM devices WHERE id = $1 AND user_id = $2",
        )
        .bind(did)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

        let trust_state: String = dev_row.try_get("trust_state")?;
        if trust_state == "revoked" {
            return Err(AppError::Unauthorized);
        }

        if let Some(hdr) = presented_device_header.map(str::trim).filter(|s| !s.is_empty()) {
            let client_did: Option<String> = dev_row.try_get("client_device_id")?;
            let matches_uuid = hdr == did.to_string();
            let matches_client = client_did.as_deref() == Some(hdr);
            if !matches_uuid && !matches_client {
                return Err(AppError::Forbidden("device_mismatch"));
            }
        }
    }

    Ok(AuthUser {
        id: user_id,
        username: user_row.try_get("username")?,
        avatar_url: user_row.try_get("avatar_url")?,
        session_id,
        device_id,
    })
}

/// Authenticate a session using `AppState` (Redis primary store, with strict PG user+device re-check on every request).
pub async fn authenticate_session_state(
    state: &AppState,
    token_hex: &str,
) -> AppResult<AuthUser> {
    authenticate_session_state_with_headers(state, token_hex, None).await
}

pub async fn authenticate_session_state_with_headers(
    state: &AppState,
    token_hex: &str,
    headers: Option<&HeaderMap>,
) -> AppResult<AuthUser> {
    let token_bytes = hex::decode(token_hex).map_err(|_| AppError::Unauthorized)?;
    if token_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }

    let token_hash = hash_session_token(&state.config.session_secret, &token_bytes);
    let token_hash_hex = hex::encode(token_hash);
    let presented_dev = headers
        .and_then(|h| h.get("x-chatw-device-id").or_else(|| h.get("x-device-id")))
        .and_then(|v| v.to_str().ok());

    if state.valkey.is_configured() {
        if let Ok(Some(rs)) = state.valkey.session_get_by_hash(&token_hash_hex).await {
            if rs.expires_at_unix <= Utc::now().timestamp() {
                let _ = state.valkey.session_delete_by_hash(&token_hash_hex).await;
                return Err(AppError::Unauthorized);
            }
            match verify_user_and_device_binding(
                &state.db,
                rs.user_id,
                Some(rs.id),
                rs.device_id,
                presented_dev,
            )
            .await
            {
                Ok(auth_user) => return Ok(auth_user),
                Err(AppError::Unauthorized) => {
                    let _ = state.valkey.session_delete_by_hash(&token_hash_hex).await;
                    return Err(AppError::Unauthorized);
                }
                Err(e) => return Err(e),
            }
        }
        // Fallback only succeeds when connected as table owner (integration tests); `chatw_app` gets permission denied -> Unauthorized cleanly without error-level log noise.
        let row = sqlx::query(
            r#"
            SELECT s.id AS session_id, s.device_id, s.user_id
            FROM sessions s
            WHERE s.token_hash = $1 AND s.expires_at > now()
            "#,
        )
        .bind(&token_hash[..])
        .fetch_optional(&state.db)
        .await
        .map_err(|_| AppError::Unauthorized)?
        .ok_or(AppError::Unauthorized)?;
        let user_id: Uuid = row.try_get("user_id")?;
        let session_id: Option<Uuid> = row.try_get("session_id")?;
        let device_id: Option<Uuid> = row.try_get("device_id")?;
        return verify_user_and_device_binding(&state.db, user_id, session_id, device_id, presented_dev).await;
    }

    authenticate_session_pg(&state.db, &token_hash, presented_dev).await
}

/// Legacy PG lookup helper for tests without Valkey.
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
    authenticate_session_pg(pool, &token_hash, None).await
}

async fn authenticate_session_pg(
    pool: &PgPool,
    token_hash: &[u8; 32],
    presented_dev: Option<&str>,
) -> AppResult<AuthUser> {
    let row = sqlx::query(
        r#"
        SELECT s.id AS session_id, s.device_id, s.user_id
        FROM sessions s
        WHERE s.token_hash = $1 AND s.expires_at > now()
        "#,
    )
    .bind(&token_hash[..])
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let user_id: Uuid = row.try_get("user_id")?;
    let session_id: Option<Uuid> = row.try_get("session_id")?;
    let device_id: Option<Uuid> = row.try_get("device_id")?;
    verify_user_and_device_binding(pool, user_id, session_id, device_id, presented_dev).await
}

pub async fn list_user_sessions_for_state(
    state: &AppState,
    user_id: Uuid,
    current_token_hash: &[u8; 32],
) -> AppResult<Vec<SessionSummary>> {
    let current_hex = hex::encode(current_token_hash);
    if state.valkey.is_configured() {
        let list = state
            .valkey
            .session_list_for_user(user_id)
            .await
            .map_err(|_| AppError::Internal)?;
        return Ok(list
            .into_iter()
            .map(|s| SessionSummary {
                id: s.id,
                device_name: s.device_name,
                device_label: s.device_label,
                ip_address: s.ip_address,
                last_active: DateTime::from_timestamp(s.last_active_unix, 0).unwrap_or_else(Utc::now),
                created_at: DateTime::from_timestamp(s.created_at_unix, 0).unwrap_or_else(Utc::now),
                is_current: s.token_hash_hex == current_hex,
            })
            .collect());
    }

    let rows = sqlx::query(
        r#"
        SELECT id, device_name, device_label, ip_address, last_active, created_at, (token_hash = $2) AS is_current
        FROM sessions
        WHERE user_id = $1
        ORDER BY last_active DESC
        "#,
    )
    .bind(user_id)
    .bind(&current_token_hash[..])
    .fetch_all(&state.db)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(SessionSummary {
            id: r.try_get("id")?,
            device_name: r.try_get("device_name")?,
            device_label: r.try_get("device_label")?,
            ip_address: r.try_get("ip_address")?,
            last_active: r.try_get("last_active")?,
            created_at: r.try_get("created_at")?,
            is_current: r.try_get("is_current")?,
        });
    }
    Ok(out)
}

pub async fn rename_session_for_state(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    label: &str,
) -> AppResult<bool> {
    if state.valkey.is_configured() {
        return state
            .valkey
            .session_rename(user_id, session_id, label)
            .await
            .map_err(|_| AppError::Internal);
    }
    let res = sqlx::query(
        "UPDATE sessions SET device_label = $1 WHERE id = $2 AND user_id = $3",
    )
    .bind(label)
    .bind(session_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn bind_session_device_for_state(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    device_id: Uuid,
) -> AppResult<()> {
    if state.valkey.is_configured() {
        let _ = state
            .valkey
            .session_bind_device(user_id, session_id, device_id)
            .await;
        return Ok(());
    }
    sqlx::query("UPDATE sessions SET device_id = $1 WHERE id = $2 AND user_id = $3")
        .bind(device_id)
        .bind(session_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn revoke_session_by_id_for_state(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
) -> AppResult<()> {
    if state.valkey.is_configured() {
        let _ = state.valkey.session_delete_by_id(user_id, session_id).await;
    } else {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(user_id)
            .execute(&state.db)
            .await;
    }
    Ok(())
}

pub async fn revoke_other_sessions_for_state(
    state: &AppState,
    user_id: Uuid,
    keep_token_hash: &[u8; 32],
) -> AppResult<Vec<Uuid>> {
    if state.valkey.is_configured() {
        return state
            .valkey
            .session_delete_others(user_id, &hex::encode(keep_token_hash))
            .await
            .map_err(|_| AppError::Internal);
    }
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2 RETURNING id",
    )
    .bind(user_id)
    .bind(&keep_token_hash[..])
    .fetch_all(&state.db)
    .await?;
    Ok(ids)
}

pub async fn revoke_all_user_sessions_for_state(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<()> {
    if state.valkey.is_configured() {
        let _ = state.valkey.session_delete_all_for_user(user_id).await;
    } else {
        let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&state.db)
            .await;
    }
    Ok(())
}

pub async fn revoke_device_sessions_for_state(
    state: &AppState,
    user_id: Uuid,
    device_id: Uuid,
    keep_only_this_device: bool,
) -> AppResult<()> {
    if state.valkey.is_configured() {
        let _ = state
            .valkey
            .session_delete_by_device(user_id, device_id, keep_only_this_device)
            .await;
    } else if keep_only_this_device {
        let _ = sqlx::query(
            "DELETE FROM sessions WHERE user_id = $1 AND (device_id IS NULL OR device_id <> $2)",
        )
        .bind(user_id)
        .bind(device_id)
        .execute(&state.db)
        .await;
    } else {
        let _ = sqlx::query("DELETE FROM sessions WHERE device_id = $1 AND user_id = $2")
            .bind(device_id)
            .bind(user_id)
            .execute(&state.db)
            .await;
    }
    Ok(())
}

pub async fn count_active_sessions_for_state(state: &AppState) -> AppResult<i64> {
    if state.valkey.is_configured() {
        return Ok(state.valkey.session_count_total().await.unwrap_or(0));
    }
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE expires_at > now()")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    Ok(n)
}

pub async fn count_user_sessions_for_state(state: &AppState, user_id: Uuid) -> AppResult<i64> {
    if state.valkey.is_configured() {
        return Ok(state
            .valkey
            .session_list_for_user(user_id)
            .await
            .map(|l| l.len() as i64)
            .unwrap_or(0));
    }
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND expires_at > now()",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    Ok(n)
}

/// Helper to parse a named cookie from the `Cookie` header.
pub fn parse_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let trimmed = pair.trim();
        if let Some((_, v)) = trimmed.split_once('=').filter(|(k, _)| *k == name) {
            return Some(v);
        }
    }
    None
}
