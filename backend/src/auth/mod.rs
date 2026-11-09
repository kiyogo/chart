pub mod google;
pub mod session;

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
            OAUTH_COOKIE_NAME, SESSION_COOKIE_NAME, authenticate_session, create_session,
            create_session_raw, destroy_session, parse_cookie,
        },
    },
    config::Config,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: sqlx::PgPool,
    pub http_client: reqwest::Client,
    pub jwks_cache: Arc<tokio::sync::RwLock<google::GoogleJwksCache>>,
    pub ws_hub: Arc<crate::websocket::WsHub>,
    pub oauth_tickets: Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, Option<String>>>>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub has_keys: bool,
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
        .route("/logout", post(handle_logout))
}

async fn handle_create_oauth_ticket(State(state): State<AppState>) -> Json<CreateTicketResponse> {
    let ticket = Uuid::new_v4();
    state.oauth_tickets.write().await.insert(ticket, None);
    Json(CreateTicketResponse { ticket })
}

async fn handle_check_oauth_ticket(
    State(state): State<AppState>,
    axum::extract::Path(ticket_id): axum::extract::Path<Uuid>,
) -> Json<CheckTicketResponse> {
    let mut tickets = state.oauth_tickets.write().await;
    if let Some(token_opt) = tickets.get(&ticket_id) {
        if let Some(token) = token_opt {
            let tok = token.clone();
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
            if state_param.starts_with("local8082_") {
                let local_url = format!(
                    "http://127.0.0.1:8082/api/auth/google/callback?code={}&state={}",
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

    // Create session cookie
    let (raw_token, session_cookie) = create_session_raw(&state.db, &state.config, user_id).await?;

    if let Some(ticket_id) = saved.ticket {
        state
            .oauth_tickets
            .write()
            .await
            .insert(ticket_id, Some(raw_token));
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
<html lang="vi">
<head>
    <meta charset="utf-8">
    <title>chatw - Đăng nhập Google thành công</title>
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
        Some(row) => row.id,
        None => {
            let new_id = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, NULL, NULL)",
                new_id,
                sub
            )
            .execute(&mut *tx)
            .await?;
            new_id
        }
    };
    tx.commit().await?;

    let session_cookie = create_session(&state.db, &state.config, user_id).await?;
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie.parse().map_err(|_| AppError::Internal)?,
    );

    Ok(response)
}
