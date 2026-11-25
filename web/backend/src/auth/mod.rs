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
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    auth::{
        google::{
            CallbackQuery, exchange_code_for_token, google_login_handler, verify_google_id_token,
        },
        session::{
            OAUTH_COOKIE_NAME, SESSION_COOKIE_NAME, authenticate_session,
            destroy_session, hash_session_token, parse_cookie,
        },
    },
    config::Config,
    error::{AppError, AppResult},
};

pub const OAUTH_TICKET_TTL: std::time::Duration = std::time::Duration::from_secs(300);
pub const OAUTH_TICKET_MAX_CAPACITY: usize = 1024;
const OAUTH_CONFIRM_COOKIE_NAME: &str = "chatw_oauth_confirm";

#[derive(Clone, Debug)]
pub struct OAuthTicketEntry {
    pub secret_hash: [u8; 32],
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
    pub rpc_router: Arc<std::sync::OnceLock<axum::Router>>,
    pub valkey: Arc<crate::cache::ValkeyCoordinator>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub has_keys: bool,
    pub device_id: Option<Uuid>,
    pub has_password: bool,
}

#[derive(serde::Deserialize)]
pub struct DevLoginRequest {
    pub username: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct PasswordAuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateAccountPasswordRequest {
    #[serde(default)]
    pub current_password: Option<String>,
    pub new_password: String,
    #[serde(default)]
    pub totp_or_backup_code: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GoogleLoginQuery {
    pub ticket: Option<Uuid>,
    pub redirect_uri: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTicketResponse {
    pub ticket: Uuid,
    pub ticket_secret: String,
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
        .route("/oauth-ticket/confirm", post(handle_confirm_oauth_ticket))
        .route("/oauth-ticket/{ticket_id}", get(handle_check_oauth_ticket))
        .route("/dev-login", post(handle_dev_login))
        .route("/register", post(handle_register_password))
        .route("/login", post(handle_login_password))
        .route("/password", post(handle_update_account_password))
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

async fn handle_create_oauth_ticket(State(state): State<AppState>) -> AppResult<Json<CreateTicketResponse>> {
    let ticket = Uuid::new_v4();
    let mut secret_raw = [0u8; 32];
    getrandom::fill(&mut secret_raw).map_err(|_| AppError::Internal)?;
    let ticket_secret = hex::encode(secret_raw);
    let secret_hash: [u8; 32] = Sha256::digest(ticket_secret.as_bytes()).into();
    let now = std::time::Instant::now();
    let mut tickets = state.oauth_tickets.write().await;
    prune_expired_oauth_tickets(&mut tickets, now);
    tickets.insert(
        ticket,
        OAuthTicketEntry {
            secret_hash,
            token: None,
            created_at: now,
        },
    );
    Ok(Json(CreateTicketResponse { ticket, ticket_secret }))
}

async fn handle_check_oauth_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(ticket_id): axum::extract::Path<Uuid>,
) -> AppResult<Json<CheckTicketResponse>> {
    let provided_secret = headers
        .get("x-ticket-secret")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .unwrap_or("");
    if provided_secret.is_empty() {
        return Err(AppError::Forbidden("missing_ticket_secret"));
    }
    let provided_hash: [u8; 32] = Sha256::digest(provided_secret.as_bytes()).into();

    let now = std::time::Instant::now();
    let mut tickets = state.oauth_tickets.write().await;
    prune_expired_oauth_tickets(&mut tickets, now);
    if let Some(entry) = tickets.get(&ticket_id) {
        if entry.secret_hash.ct_eq(&provided_hash).unwrap_u8() != 1 {
            return Err(AppError::Forbidden("invalid_ticket_secret"));
        }
        if let Some(ref token) = entry.token {
            let tok = token.clone();
            // Atomic one-time consumption: ticket is destroyed immediately once token is claimed
            tickets.remove(&ticket_id);
            return Ok(Json(CheckTicketResponse {
                ready: true,
                token: Some(tok),
            }));
        }
        return Ok(Json(CheckTicketResponse {
            ready: false,
            token: None,
        }));
    }
    Ok(Json(CheckTicketResponse {
        ready: false,
        token: None,
    }))
}

#[derive(serde::Deserialize)]
struct ConfirmOAuthTicketForm {
    ticket: Uuid,
    csrf: String,
}

async fn handle_confirm_oauth_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<ConfirmOAuthTicketForm>,
) -> AppResult<Response> {
    let raw_confirm = parse_cookie(&headers, OAUTH_CONFIRM_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let (ticket_str, rest) = raw_confirm.split_once('|').ok_or(AppError::Unauthorized)?;
    let (user_str, rest2) = rest.split_once('|').ok_or(AppError::Unauthorized)?;
    let (exp_str, rest3) = rest2.split_once('|').ok_or(AppError::Unauthorized)?;
    let (csrf_saved, sig_hex) = rest3.split_once('|').ok_or(AppError::Unauthorized)?;
    let unsigned = format!("{ticket_str}|{user_str}|{exp_str}|{csrf_saved}");
    let expected_sig = hash_session_token(&state.config.session_secret, unsigned.as_bytes());
    let given_sig = hex::decode(sig_hex).map_err(|_| AppError::Unauthorized)?;
    if given_sig.len() != 32 || expected_sig.ct_eq(&given_sig).unwrap_u8() != 1 {
        return Err(AppError::Unauthorized);
    }
    let ticket_id = Uuid::parse_str(ticket_str).map_err(|_| AppError::Unauthorized)?;
    let user_id = Uuid::parse_str(user_str).map_err(|_| AppError::Unauthorized)?;
    let exp: i64 = exp_str.parse().map_err(|_| AppError::Unauthorized)?;
    if Utc::now().timestamp() > exp || ticket_id != form.ticket || csrf_saved.as_bytes().ct_eq(form.csrf.as_bytes()).unwrap_u8() != 1 {
        return Err(AppError::Unauthorized);
    }

    let (raw_token, _session_cookie) = match crate::security::complete_login(&state, user_id, &headers).await? {
        crate::security::LoginOutcome::Session(raw, cookie) => (raw, cookie),
        crate::security::LoginOutcome::Challenge(_, _) => return Err(AppError::Forbidden("twofa_required")),
    };

    let now = std::time::Instant::now();
    let mut tickets = state.oauth_tickets.write().await;
    prune_expired_oauth_tickets(&mut tickets, now);
    if let Some(entry) = tickets.get_mut(&ticket_id) {
        entry.token = Some(raw_token);
    } else {
        return Err(AppError::BadRequest("ticket_expired"));
    }

    let secure_flag = if state.config.cookie_secure { "; Secure" } else { "" };
    let clear_confirm = format!("{OAUTH_CONFIRM_COOKIE_NAME}=; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}");
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="utf-8"><title>ChatW - Xác nhận thành công</title><style>body{background:#09090b;color:#fafafa;font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}.box{background:#18181b;border:1px solid #27272a;border-radius:12px;padding:32px;text-align:center;max-width:420px}</style></head><body><div class="box"><h1>Đã xác nhận đăng nhập ứng dụng</h1><p>Bạn có thể đóng tab này và quay lại ChatW.</p></div><script>setTimeout(function(){window.close();},1500);</script></body></html>"#;
    let mut resp = axum::response::Html(html).into_response();
    resp.headers_mut().append(header::SET_COOKIE, clear_confirm.parse().map_err(|_| AppError::Internal)?);
    Ok(resp)
}

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub google: bool,
    pub dev: bool,
    pub password: bool,
}

async fn handle_providers(State(state): State<AppState>) -> Json<ProvidersResponse> {
    Json(ProvidersResponse {
        google: state.config.google_configured,
        dev: state.config.dev_login_enabled,
        password: true,
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
            if state_param.starts_with("local") && state.config.bind_addr.ip().is_loopback() {
                let port = state.config.bind_addr.port();
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
                        "UPDATE users SET avatar_url = $1 WHERE id = $2 AND (avatar_url IS NULL OR avatar_url LIKE 'https://%googleusercontent.com%')",
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

    // Clear oauth cookie
    let secure_flag = if state.config.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let clear_oauth = format!(
        "{OAUTH_COOKIE_NAME}=; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}"
    );

    if let Some(ticket_id) = saved.ticket {
        let now = std::time::Instant::now();
        let ticket_exists = {
            let mut tickets = state.oauth_tickets.write().await;
            prune_expired_oauth_tickets(&mut tickets, now);
            tickets.contains_key(&ticket_id)
        };
        if !ticket_exists {
            return Err(AppError::BadRequest("ticket_expired"));
        }
        let _ = raw_token;
        let mut csrf_bytes = [0u8; 16];
        getrandom::fill(&mut csrf_bytes).map_err(|_| AppError::Internal)?;
        let csrf = hex::encode(csrf_bytes);
        let exp = Utc::now().timestamp() + 180;
        let unsigned = format!("{ticket_id}|{user_id}|{exp}|{csrf}");
        let sig = hex::encode(hash_session_token(&state.config.session_secret, unsigned.as_bytes()));
        let confirm_val = format!("{unsigned}|{sig}");
        let confirm_cookie = format!(
            "{OAUTH_CONFIRM_COOKIE_NAME}={confirm_val}; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=180{secure_flag}"
        );
        let short_ticket: String = ticket_id.to_string().chars().take(8).collect();
        let confirm_html = format!(
            r#"<!DOCTYPE html><html lang="vi"><head><meta charset="utf-8"><title>Xác nhận đăng nhập ứng dụng ChatW</title><style>body{{background:#09090b;color:#fafafa;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}.box{{background:#18181b;border:1px solid #27272a;border-radius:12px;padding:32px;text-align:center;max-width:420px}}h1{{font-size:20px;margin:0 0 10px}}p{{font-size:14px;color:#a1a1aa;margin:0 0 20px;line-height:1.5}}.btn{{background:#2563eb;color:#fff;border:none;padding:10px 20px;border-radius:6px;font-size:14px;font-weight:500;cursor:pointer}}</style></head><body><div class="box"><h1>Xác nhận chuyển phiên đăng nhập</h1><p>Bạn có muốn đăng nhập ứng dụng ChatW Desktop (mã phiên {short_ticket}) bằng tài khoản này không?</p><form method="POST" action="/api/auth/oauth-ticket/confirm"><input type="hidden" name="ticket" value="{ticket_id}" /><input type="hidden" name="csrf" value="{csrf}" /><button class="btn" type="submit">Xác nhận đăng nhập ứng dụng</button></form></div></body></html>"#
        );
        let mut response = axum::response::Html(confirm_html).into_response();
        response.headers_mut().append(header::SET_COOKIE, session_cookie.parse().map_err(|_| AppError::Internal)?);
        response.headers_mut().append(header::SET_COOKIE, confirm_cookie.parse().map_err(|_| AppError::Internal)?);
        response.headers_mut().append(header::SET_COOKIE, clear_oauth.parse().map_err(|_| AppError::Internal)?);
        return Ok(response);
    }

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

    let has_password: bool = sqlx::query_scalar(
        "SELECT password_hash IS NOT NULL FROM users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        avatar_url: user.avatar_url,
        has_keys: key_count > 0,
        device_id: user.device_id,
        has_password,
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

    finish_login_response(&state, user_id, &headers).await
}

async fn finish_login_response(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
) -> AppResult<Response> {
    match crate::security::complete_login(state, user_id, headers).await? {
        crate::security::LoginOutcome::Session(_, session_cookie) => {
            let mut response = StatusCode::OK.into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                session_cookie.parse().map_err(|_| AppError::Internal)?,
            );
            Ok(response)
        }
        crate::security::LoginOutcome::Challenge(_, challenge_cookie) => {
            let mut response = Json(serde_json::json!({ "requires_2fa": true })).into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                challenge_cookie.parse().map_err(|_| AppError::Internal)?,
            );
            Ok(response)
        }
    }
}

fn validate_account_password(password: &str) -> AppResult<()> {
    let chars = password.chars().count();
    if chars < 6 || password.len() > 128 {
        return Err(AppError::BadRequest("invalid_password_length"));
    }
    Ok(())
}

async fn handle_register_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    crate::error::ApiJson(req): crate::error::ApiJson<PasswordAuthRequest>,
) -> AppResult<Response> {
    let ip_key = format!("reg:ip:{}", crate::security::ip_masked_with_trust(&headers, state.config.trust_proxy));
    state
        .valkey
        .check_rate_limit(&[ip_key], 20, std::time::Duration::from_secs(900))
        .await
        .map_err(|_| AppError::TooManyRequests)?;
    let (display_name, username_norm) =
        crate::users::validate_and_normalize_username(&req.username)?;
    validate_account_password(&req.password)?;
    let pw = req.password.clone();
    let phc = tokio::task::spawn_blocking(move || crate::backup::reauth::hash_argon2_phc(&pw))
        .await
        .map_err(|_| AppError::Internal)??;

    let user_id = Uuid::new_v4();
    let local_sub = format!("local:{user_id}");
    let res = sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm, password_hash)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(&local_sub)
    .bind(&display_name)
    .bind(&username_norm)
    .bind(&phc)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => {
            crate::security::log_event(&state, user_id, "account_registered_password", &headers)
                .await;
            finish_login_response(&state, user_id, &headers).await
        }
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            Err(AppError::Conflict("username_taken"))
        }
        Err(e) => Err(e.into()),
    }
}

async fn handle_login_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    crate::error::ApiJson(req): crate::error::ApiJson<PasswordAuthRequest>,
) -> AppResult<Response> {
    const DUMMY_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let (_, username_norm) = crate::users::validate_and_normalize_username(&req.username)?;
    let ip_key = format!("login:ip:{}", crate::security::ip_masked_with_trust(&headers, state.config.trust_proxy));
    let user_key = format!("login:u:{username_norm}");
    state
        .valkey
        .check_rate_limit(&[ip_key, user_key], 15, std::time::Duration::from_secs(900))
        .await
        .map_err(|_| AppError::TooManyRequests)?;

    if req.password.is_empty() || req.password.len() > 128 {
        return Err(AppError::BadRequest("invalid_credentials"));
    }

    let row = sqlx::query(
        "SELECT id, password_hash, suspended_at FROM users WHERE username_norm = $1",
    )
    .bind(&username_norm)
    .fetch_optional(&state.db)
    .await?;

    let pw = req.password.clone();
    let Some(row) = row else {
        let _ = tokio::task::spawn_blocking(move || crate::backup::reauth::verify_argon2_phc(DUMMY_PHC, &pw)).await;
        return Err(AppError::BadRequest("invalid_credentials"));
    };

    use sqlx::Row;
    let user_id: Uuid = row.try_get("id")?;
    let suspended: Option<DateTime<Utc>> = row.try_get("suspended_at")?;
    if suspended.is_some() {
        let _ = tokio::task::spawn_blocking(move || crate::backup::reauth::verify_argon2_phc(DUMMY_PHC, &pw)).await;
        return Err(AppError::Forbidden("account_suspended"));
    }
    let phc: Option<String> = row.try_get("password_hash")?;
    let Some(phc) = phc else {
        let _ = tokio::task::spawn_blocking(move || crate::backup::reauth::verify_argon2_phc(DUMMY_PHC, &pw)).await;
        return Err(AppError::BadRequest("password_not_set_for_account"));
    };
    let ok = tokio::task::spawn_blocking(move || crate::backup::reauth::verify_argon2_phc(&phc, &pw))
        .await
        .map_err(|_| AppError::Internal)?;
    if !ok {
        crate::security::log_event(&state, user_id, "password_login_failed", &headers).await;
        return Err(AppError::BadRequest("invalid_credentials"));
    }

    finish_login_response(&state, user_id, &headers).await
}

async fn handle_update_account_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    crate::error::ApiJson(req): crate::error::ApiJson<UpdateAccountPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;
    validate_account_password(&req.new_password)?;
    crate::security::require_second_factor(
        &state,
        user.id,
        req.totp_or_backup_code.as_deref(),
        &headers,
    )
    .await?;

    let existing_phc: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;

    if let Some(cur_phc) = existing_phc {
        let cur = req.current_password.clone().unwrap_or_default();
        let ok = tokio::task::spawn_blocking(move || crate::backup::reauth::verify_argon2_phc(&cur_phc, &cur))
            .await
            .map_err(|_| AppError::Internal)?;
        if !ok {
            return Err(AppError::BadRequest("wrong_current_password"));
        }
    }

    let next_pw = req.new_password.clone();
    let next_phc = tokio::task::spawn_blocking(move || crate::backup::reauth::hash_argon2_phc(&next_pw))
        .await
        .map_err(|_| AppError::Internal)??;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&next_phc)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    if let Ok(token_bytes) = hex::decode(session_token)
        && token_bytes.len() == 32
    {
        let token_hash = hash_session_token(&state.config.session_secret, &token_bytes);
        if let Ok(revoked_ids) = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2 RETURNING id",
        )
        .bind(user.id)
        .bind(&token_hash[..])
        .fetch_all(&state.db)
        .await
        {
            for sid in revoked_ids {
                state.ws_hub.disconnect_session(&user.id, sid).await;
            }
        }
    }

    crate::security::log_event(&state, user.id, "account_password_updated", &headers).await;
    Ok(Json(serde_json::json!({ "ok": true, "has_password": true })))
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
            id: r.id,
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
pub fn is_allowed_cors_origin_for_config(
    origin_raw: &str,
    configured_public_origin: &str,
    dev_login_enabled: bool,
) -> bool {
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
    let pub_parsed = reqwest::Url::parse(configured_public_origin.trim()).ok();
    let pub_is_loopback = pub_parsed
        .as_ref()
        .and_then(|u| u.host_str())
        .is_some_and(|h| h.eq_ignore_ascii_case("localhost") || h == "127.0.0.1");
    if (dev_login_enabled || pub_is_loopback)
        && (host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1")
    {
        return true;
    }
    if let Some(pub_url) = pub_parsed
        && pub_url.scheme() == parsed.scheme()
        && pub_url.host_str() == parsed.host_str()
        && pub_url.port_or_known_default() == parsed.port_or_known_default()
    {
        return true;
    }
    false
}

/// Strict CORS origin check: parses the `Origin` header as a URL and permits only exact
/// `localhost` / `127.0.0.1` loopback hosts (when configured/dev) or the configured `public_origin`.
pub fn is_allowed_cors_origin(origin_raw: &str, configured_public_origin: &str) -> bool {
    is_allowed_cors_origin_for_config(origin_raw, configured_public_origin, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_cors_origin_rejects_subdomain_prefix_bypass() {
        assert!(is_allowed_cors_origin("http://localhost:8085", "http://localhost:8085"));
        assert!(is_allowed_cors_origin("http://127.0.0.1:5173", "http://localhost:8085"));
        assert!(is_allowed_cors_origin("https://chat.example.com", "https://chat.example.com"));
        // Production non-loopback MUST reject arbitrary localhost origins
        assert!(!is_allowed_cors_origin("http://localhost:5173", "https://chat.example.com"));
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
                secret_hash: [1u8; 32],
                token: Some("old".into()),
                created_at: base,
            },
        );
        let fresh_id = Uuid::new_v4();
        let later = base + OAUTH_TICKET_TTL + std::time::Duration::from_secs(1);
        store.insert(
            fresh_id,
            OAuthTicketEntry {
                secret_hash: [2u8; 32],
                token: Some("fresh".into()),
                created_at: later,
            },
        );
        prune_expired_oauth_tickets(&mut store, later);
        assert!(!store.contains_key(&expired_id), "expired ticket must be evicted");
        assert!(store.contains_key(&fresh_id), "fresh ticket must remain");
    }
}
