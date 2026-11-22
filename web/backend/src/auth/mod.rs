pub mod google;
pub mod session;

use chrono::{DateTime, Utc};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::{
        google::{
            CallbackQuery, exchange_code_for_token, google_login_handler, verify_google_id_token,
        },
        session::{
            OAUTH_COOKIE_NAME, SESSION_COOKIE_NAME, authenticate_session,
            destroy_session, parse_cookie,
        },
    },
    config::Config,
    error::{AppError, AppResult},
};

pub const OAUTH_TICKET_TTL: std::time::Duration = std::time::Duration::from_secs(300);
pub const OAUTH_TICKET_MAX_CAPACITY: usize = 1024;

#[derive(Clone, Debug)]
pub struct OAuthTicketEntry {
    pub token: Option<String>,
    pub created_at: std::time::Instant,
}

pub type OAuthTicketStore =
    Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, OAuthTicketEntry>>>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: sqlx::PgPool,
    pub http_client: reqwest::Client,
    pub jwks_cache: Arc<tokio::sync::RwLock<google::GoogleJwksCache>>,
    pub ws_hub: Arc<crate::websocket::WsHub>,
    pub oauth_tickets: OAuthTicketStore,
    pub storage: Arc<crate::storage::StorageRegistry>,
    pub sfu: Arc<crate::calls::sfu::SfuHandle>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub has_keys: bool,
    pub device_id: Option<Uuid>,
}

#[derive(serde::Deserialize)]
pub struct DevLoginRequest {
    pub username: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GoogleLoginQuery {
    pub ticket: Option<Uuid>,
    pub redirect_uri: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTicketResponse {
    pub ticket: Uuid,
}

#[derive(Serialize)]
pub struct CheckTicketResponse {
    pub ready: bool,
    pub token: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/google", get(handle_google_login))
        .route("/google/callback", get(handle_google_callback))
        .route("/oauth-ticket", post(handle_create_oauth_ticket))
        .route("/oauth-ticket/{ticket_id}", get(handle_check_oauth_ticket))
        .route("/dev-login", post(handle_dev_login))
        .route("/providers", get(handle_providers))
        .route("/me", get(handle_me))
        .route("/sessions", get(handle_list_sessions).delete(handle_delete_other_sessions))
        .route("/sessions/{session_id}", axum::routing::delete(handle_delete_session).patch(handle_rename_session))
        .route("/logout", post(handle_logout))
        .merge(crate::security::auth_routes())
}

pub fn prune_expired_oauth_tickets(
    tickets: &mut std::collections::HashMap<Uuid, OAuthTicketEntry>,
    now: std::time::Instant,
) {
    tickets.retain(|_, entry| now.duration_since(entry.created_at) < OAUTH_TICKET_TTL);
    if tickets.len() >= OAUTH_TICKET_MAX_CAPACITY {
        if let Some(oldest_key) = tickets
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(k, _)| *k)
        {
            tickets.remove(&oldest_key);
        }
    }
}

async fn handle_create_oauth_ticket(State(state): State<AppState>) -> Json<CreateTicketResponse> {
    let ticket = Uuid::new_v4();
    let now = std::time::Instant::now();
    let mut tickets = state.oauth_tickets.write().await;
    prune_expired_oauth_tickets(&mut tickets, now);
    tickets.insert(
        ticket,
        OAuthTicketEntry {
            token: None,
            created_at: now,
        },
    );
    Json(CreateTicketResponse { ticket })
}

async fn handle_check_oauth_ticket(
    State(state): State<AppState>,
    axum::extract::Path(ticket_id): axum::extract::Path<Uuid>,
) -> Json<CheckTicketResponse> {
    let now = std::time::Instant::now();
    let mut tickets = state.oauth_tickets.write().await;
    prune_expired_oauth_tickets(&mut tickets, now);
    if let Some(entry) = tickets.get(&ticket_id) {
        if let Some(ref token) = entry.token {
            let tok = token.clone();
            // Atomic one-time consumption: ticket is destroyed immediately once token is claimed
            tickets.remove(&ticket_id);
            return Json(CheckTicketResponse {
                ready: true,
                token: Some(tok),
            });
        }
    }
    Json(CheckTicketResponse {
        ready: false,
        token: None,
    })
}

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub google: bool,
    pub dev: bool,
}

async fn handle_providers(State(state): State<AppState>) -> Json<ProvidersResponse> {
    Json(ProvidersResponse {
        google: state.config.google_configured,
        dev: state.config.dev_login_enabled,
    })
}

async fn handle_google_login(
    State(state): State<AppState>,
    Query(query): Query<GoogleLoginQuery>,
) -> AppResult<Response> {
    google_login_handler(&state.config, query.ticket, query.redirect_uri).await
}

async fn handle_google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> AppResult<Response> {
    if let Some(err) = query.error {
        tracing::warn!(error = %err, "google oauth error callback");
        return Err(AppError::Unauthorized);
    }

    let code = query.code.ok_or(AppError::BadRequest("missing_code"))?;
    let state_param = query.state.ok_or(AppError::BadRequest("missing_state"))?;

    let oauth_cookie = match parse_cookie(&headers, OAUTH_COOKIE_NAME) {
        Some(c) => c,
        None => {
            if state_param.starts_with("local") {
                let port: u16 = state_param
                    .strip_prefix("local")
                    .and_then(|s| s.split('_').next())
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(state.config.bind_addr.port());
                let local_url = format!(
                    "http://127.0.0.1:{port}/api/auth/google/callback?code={}&state={}",
                    urlencoding::encode(&code),
                    urlencoding::encode(&state_param)
                );
                return Ok(Redirect::temporary(&local_url).into_response());
            }
            return Err(AppError::Unauthorized);
        }
    };

    let saved = google::verify_oauth_cookie(&state.config.session_secret, oauth_cookie)
        .ok_or(AppError::Unauthorized)?;

    if subtle::ConstantTimeEq::ct_eq(state_param.as_bytes(), saved.state.as_bytes()).unwrap_u8()
        != 1
    {
        return Err(AppError::Unauthorized);
    }

    let redirect_uri = saved
        .redirect_uri
        .as_deref()
        .unwrap_or(&state.config.google_redirect_url);

    // Exchange authorization code for ID token
    let id_token = exchange_code_for_token(
        &state.http_client,
        &state.config,
        &code,
        &saved.code_verifier,
        redirect_uri,
    )
    .await?;

    // Verify ID token claims
    let claims = verify_google_id_token(
        &id_token,
        &state.config.google_client_id,
        &saved.nonce,
        &state.http_client,
        &state.jwks_cache,
    )
    .await?;

    // Upsert or fetch internal user record by google_sub
    let user_id = {
        let mut tx = state.db.begin().await?;
        let existing = sqlx::query!("SELECT id FROM users WHERE google_sub = $1", claims.sub)
            .fetch_optional(&mut *tx)
            .await?;

        let id = match existing {
            Some(row) => {
                if let Some(ref pic) = claims.picture {
                    sqlx::query!(
                        "UPDATE users SET avatar_url = $1 WHERE id = $2",
                        pic,
                        row.id
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                row.id
            }
            None => {
                let new_id = Uuid::new_v4();
                sqlx::query!(
                    "INSERT INTO users (id, google_sub, username, username_norm, avatar_url) VALUES ($1, $2, NULL, NULL, $3)",
                    new_id,
                    claims.sub,
                    claims.picture
                )
                .execute(&mut *tx)
                .await?;
                new_id
            }
        };
        tx.commit().await?;
        id
    };

    // Create the session, or ask for the second step first (spec 011).
    let (raw_token, session_cookie) = match crate::security::complete_login(&state, user_id, &headers).await? {
        crate::security::LoginOutcome::Session(raw, cookie) => (raw, cookie),
        crate::security::LoginOutcome::Challenge(_, challenge_cookie) => {
            let secure_flag = if state.config.cookie_secure { "; Secure" } else { "" };
            let clear_oauth = format!("{OAUTH_COOKIE_NAME}=; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}");
            let mut response = Redirect::temporary("/?twofa=1").into_response();
            response.headers_mut().append(header::SET_COOKIE, challenge_cookie.parse().map_err(|_| AppError::Internal)?);
            response.headers_mut().append(header::SET_COOKIE, clear_oauth.parse().map_err(|_| AppError::Internal)?);
            return Ok(response);
        }
    };

    if let Some(ticket_id) = saved.ticket {
        let now = std::time::Instant::now();
        let mut tickets = state.oauth_tickets.write().await;
        prune_expired_oauth_tickets(&mut tickets, now);
        if let Some(entry) = tickets.get_mut(&ticket_id) {
            entry.token = Some(raw_token);
        }
    }

    // Clear oauth cookie
    let secure_flag = if state.config.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let clear_oauth = format!(
        "{OAUTH_COOKIE_NAME}=; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}"
    );

    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>ChatW - Google Sign-In Successful</title>
    <style>
        body { background: #09090b; color: #fafafa; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
        .box { background: #18181b; border: 1px solid #27272a; border-radius: 12px; padding: 36px; text-align: center; max-width: 420px; box-shadow: 0 4px 24px rgba(0,0,0,0.4); }
        .icon { font-size: 44px; margin-bottom: 16px; }
        h1 { font-size: 20px; margin: 0 0 10px; color: #ffffff; }
        p { font-size: 14px; color: #a1a1aa; margin: 0 0 20px; line-height: 1.5; }
        .btn { display: inline-block; background: #2563eb; color: #ffffff; padding: 10px 20px; border-radius: 6px; text-decoration: none; font-size: 13px; font-weight: 500; }
    </style>
</head>
<body>
    <div class="box">
        <div class="icon">✅</div>
        <h1>Đăng nhập Google thành công</h1>
        <p>Phiên đăng nhập đã được tạo thành công cho tài khoản chatw của bạn. Bạn có thể đóng tab này và quay lại ứng dụng.</p>
        <a href="javascript:window.close()" class="btn">Đóng cửa sổ</a>
    </div>
    <script>setTimeout(function(){ window.close(); }, 2500);</script>
</body>
</html>"#;

    let mut response = axum::response::Html(html).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        session_cookie.parse().map_err(|_| AppError::Internal)?,
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        clear_oauth.parse().map_err(|_| AppError::Internal)?,
    );

    Ok(response)
}

async fn handle_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<MeResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;

    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let key_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM device_keys WHERE user_id = $1",
        user.id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        avatar_url: user.avatar_url,
        has_keys: key_count > 0,
        device_id: user.device_id,
    }))
}

async fn handle_logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let session_token = parse_cookie(&headers, SESSION_COOKIE_NAME).unwrap_or("");
    let clear_cookie = destroy_session(&state.db, &state.config, session_token).await?;

    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        clear_cookie.parse().map_err(|_| AppError::Internal)?,
    );
    Ok(response)
}

async fn handle_dev_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<DevLoginRequest>>,
) -> AppResult<Response> {
    if !state.config.dev_login_enabled {
        return Err(AppError::NotFound("not_found"));
    }

    let username = body
        .and_then(|b| b.0.username)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let sub = match &username {
        Some(u) => format!("dev_{}", u.to_lowercase()),
        None => format!("dev_user_{}", Uuid::new_v4().simple()),
    };

    let mut tx = state.db.begin().await?;
    let existing = sqlx::query!("SELECT id FROM users WHERE google_sub = $1", sub)
        .fetch_optional(&mut *tx)
        .await?;

    let user_id = match existing {
        Some(row) => {
            if let Some(ref u) = username {
                let norm = u.to_lowercase();
                sqlx::query!(
                    "UPDATE users SET username = COALESCE(username, $1), username_norm = COALESCE(username_norm, $2) WHERE id = $3",
                    u,
                    norm,
                    row.id
                )
                .execute(&mut *tx)
                .await?;
            }
            row.id
        }
        None => {
            let new_id = Uuid::new_v4();
            let norm = username.as_ref().map(|u| u.to_lowercase());
            sqlx::query!(
                "INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)",
                new_id,
                sub,
                username,
                norm
            )
            .execute(&mut *tx)
            .await?;
            new_id
        }
    };
    tx.commit().await?;

    // Spec 011: two-step verification and login alerts apply to every login.
    match crate::security::complete_login(&state, user_id, &headers).await? {
        crate::security::LoginOutcome::Session(_, session_cookie) => {
            let mut response = StatusCode::OK.into_response();
            response.headers_mut().insert(header::SET_COOKIE, session_cookie.parse().map_err(|_| AppError::Internal)?);
            Ok(response)
        }
        crate::security::LoginOutcome::Challenge(_, challenge_cookie) => {
            let mut response = Json(serde_json::json!({ "requires_2fa": true })).into_response();
            response.headers_mut().insert(header::SET_COOKIE, challenge_cookie.parse().map_err(|_| AppError::Internal)?);
            Ok(response)
        }
    }
}

#[derive(Serialize)]
pub struct SessionResponseItem {
    pub id: Uuid,
    pub device_name: Option<String>,
    pub device_label: Option<String>,
    pub ip_address: Option<String>,
    pub last_active: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

async fn handle_list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SessionResponseItem>>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let token_bytes = hex::decode(session_token).map_err(|_| AppError::Unauthorized)?;
    if token_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }
    let token_hash = session::hash_session_token(&state.config.session_secret, &token_bytes);

    let rows = sqlx::query!(
        r#"
        SELECT id, device_name, device_label, ip_address, last_active, created_at, (token_hash = $2) as "is_current!"
        FROM sessions
        WHERE user_id = $1
        ORDER BY last_active DESC
        "#,
        user.id,
        &token_hash[..]
    )
    .fetch_all(&state.db)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|r| SessionResponseItem {
            id: r.id.unwrap_or_else(Uuid::new_v4),
            device_name: r.device_name,
            device_label: r.device_label,
            ip_address: r.ip_address,
            last_active: r.last_active,
            created_at: r.created_at,
            is_current: r.is_current,
        })
        .collect();

    Ok(Json(sessions))
}

#[derive(serde::Deserialize)]
struct RenameSessionReq {
    label: String,
}

/// Rename one of the caller's devices (shown in "Chính mình" and the device list).
async fn handle_rename_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<Uuid>,
    Json(p): Json<RenameSessionReq>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    let label = p.label.trim().to_string();
    if label.is_empty() || label.chars().count() > 40 {
        return Err(AppError::BadRequest("invalid_device_label"));
    }
    let res = sqlx::query!(
        "UPDATE sessions SET device_label = $1 WHERE id = $2 AND user_id = $3",
        label,
        session_id,
        user.id
    )
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("session_not_found"));
    }
    Ok(Json(serde_json::json!({ "label": label })))
}

/// Sign out every other device of the current user (keeps this session).
async fn handle_delete_other_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    // Signing out every other device is sensitive: ask for the second step when it is on.
    crate::security::require_second_factor(&state, user.id, q.get("code").map(String::as_str), &headers).await?;
    crate::security::log_event(&state, user.id, "sessions_revoked_others", &headers).await;
    let token_bytes = hex::decode(session_token).map_err(|_| AppError::Unauthorized)?;
    if token_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }
    let token_hash = session::hash_session_token(&state.config.session_secret, &token_bytes);
    let revoked_ids: Vec<Uuid> = sqlx::query_scalar(
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2 RETURNING id",
    )
    .bind(user.id)
    .bind(&token_hash[..])
    .fetch_all(&state.db)
    .await?;
    for sid in &revoked_ids {
        state.ws_hub.disconnect_session(&user.id, *sid).await;
    }
    let _ = crate::audit::log_audit_event(
        &state.db,
        user.id,
        "sessions_revoked_others",
        Some("session"),
        None,
        None,
        None,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "revoked": revoked_ids.len() })))
}

async fn handle_delete_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    sqlx::query!(
        "DELETE FROM sessions WHERE id = $1 AND user_id = $2",
        session_id,
        user.id
    )
    .execute(&state.db)
    .await?;
    state.ws_hub.disconnect_session(&user.id, session_id).await;

    let _ = crate::audit::log_audit_event(
        &state.db,
        user.id,
        "session_revoked",
        Some("session"),
        Some(&session_id.to_string()),
        None,
        None,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Strict CORS origin check: parses the `Origin` header as a URL and permits only exact
/// `localhost` / `127.0.0.1` loopback hosts (any port) or the configured `public_origin`.
/// Rejects prefix-bypass origins like `http://localhost.attacker.evil` or `http://127.0.0.1.evil.com`.
pub fn is_allowed_cors_origin(origin_raw: &str, configured_public_origin: &str) -> bool {
    let trimmed = origin_raw.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return false;
    }
    let Ok(parsed) = reqwest::Url::parse(trimmed) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        return true;
    }
    if let Ok(pub_url) = reqwest::Url::parse(configured_public_origin.trim()) {
        if pub_url.scheme() == parsed.scheme()
            && pub_url.host_str() == parsed.host_str()
            && pub_url.port_or_known_default() == parsed.port_or_known_default()
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_cors_origin_rejects_subdomain_prefix_bypass() {
        assert!(is_allowed_cors_origin("http://localhost:8085", "http://localhost:8085"));
        assert!(is_allowed_cors_origin("http://127.0.0.1:5173", "http://localhost:8085"));
        assert!(is_allowed_cors_origin("https://chat.example.com", "https://chat.example.com"));
        // Prefix-spoofed attacker domains MUST be rejected
        assert!(!is_allowed_cors_origin("http://localhost.attacker.evil", "http://localhost:8085"));
        assert!(!is_allowed_cors_origin("http://localhost.evil.com:8085", "http://localhost:8085"));
        assert!(!is_allowed_cors_origin("http://127.0.0.1.evil.com", "http://localhost:8085"));
        assert!(!is_allowed_cors_origin("null", "http://localhost:8085"));
        assert!(!is_allowed_cors_origin("javascript:alert(1)", "http://localhost:8085"));
    }

    #[test]
    fn test_oauth_ticket_ttl_and_capacity_eviction() {
        let mut store = std::collections::HashMap::new();
        let base = std::time::Instant::now();
        let expired_id = Uuid::new_v4();
        store.insert(
            expired_id,
            OAuthTicketEntry {
                token: Some("old".into()),
                created_at: base,
            },
        );
        let fresh_id = Uuid::new_v4();
        let later = base + OAUTH_TICKET_TTL + std::time::Duration::from_secs(1);
        store.insert(
            fresh_id,
            OAuthTicketEntry {
                token: Some("fresh".into()),
                created_at: later,
            },
        );
        prune_expired_oauth_tickets(&mut store, later);
        assert!(!store.contains_key(&expired_id), "expired ticket must be evicted");
        assert!(store.contains_key(&fresh_id), "fresh ticket must remain");
    }
}
