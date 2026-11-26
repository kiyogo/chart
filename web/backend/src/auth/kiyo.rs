use axum::{
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    auth::google::{
        GoogleIdTokenClaims, GoogleJwksCache, OAuthStatePayload, generate_pkce,
        generate_random_token, sign_oauth_cookie, verify_oauth_cookie,
    },
    config::Config,
    error::{AppError, AppResult},
};

pub const KIYO_OAUTH_COOKIE_NAME: &str = "chatw_kiyo_oauth";

#[derive(Deserialize)]
struct KiyoTokenResponse {
    id_token: Option<String>,
}

/// GET /api/auth/kiyo
pub async fn kiyo_login_handler(
    config: &Config,
    ticket: Option<Uuid>,
    custom_redirect: Option<String>,
) -> AppResult<Response> {
    if !config.kiyo_configured {
        return Err(AppError::BadRequest("kiyo_not_configured"));
    }
    let token = generate_random_token();
    let state = if config.bind_addr.ip().is_loopback() {
        format!("local{}_{token}", config.bind_addr.port())
    } else {
        token
    };
    let nonce = generate_random_token();
    let (code_verifier, code_challenge) = generate_pkce();

    let default_port = config.bind_addr.port();
    let allowed_local_cb_1 = format!("http://127.0.0.1:{default_port}/api/auth/kiyo/callback");
    let allowed_local_cb_2 = format!("http://localhost:{default_port}/api/auth/kiyo/callback");
    let redirect_uri = custom_redirect
        .map(|u| u.trim().to_string())
        .filter(|u| {
            u == &config.kiyo_redirect_url
                || (config.bind_addr.ip().is_loopback()
                    && (u == &allowed_local_cb_1 || u == &allowed_local_cb_2))
        })
        .unwrap_or_else(|| config.kiyo_redirect_url.clone());

    let oauth_payload = OAuthStatePayload {
        state: state.clone(),
        nonce: nonce.clone(),
        code_verifier,
        exp: Utc::now().timestamp() + 600,
        ticket,
        redirect_uri: Some(redirect_uri.clone()),
    };

    let cookie_val = sign_oauth_cookie(&config.session_secret, &oauth_payload);
    let secure_flag = if config.cookie_secure { "; Secure" } else { "" };
    let cookie_header = format!(
        "{KIYO_OAUTH_COOKIE_NAME}={cookie_val}; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=600{secure_flag}"
    );

    let auth_url = format!(
        "{}/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
        config.kiyo_issuer,
        urlencoding::encode(&config.kiyo_client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
        urlencoding::encode(&nonce),
        urlencoding::encode(&code_challenge),
    );

    let mut response = Redirect::temporary(&auth_url).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_header.parse().map_err(|_| AppError::Internal)?,
    );
    Ok(response)
}

pub fn verify_kiyo_cookie(secret: &[u8], cookie_val: &str) -> Option<OAuthStatePayload> {
    verify_oauth_cookie(secret, cookie_val)
}

async fn get_kiyo_jwks(
    client: &reqwest::Client,
    issuer: &str,
    cache: &RwLock<GoogleJwksCache>,
) -> AppResult<JwkSet> {
    {
        let read = cache.read().await;
        if let (Some(jwks), Some(fetched)) = (&read.jwks, read.fetched_at)
            && Utc::now() - fetched < chrono::Duration::hours(1)
        {
            return Ok(jwks.clone());
        }
    }

    let jwks_url = format!("{issuer}/.well-known/jwks.json");
    let res = client.get(&jwks_url).send().await.map_err(|e| {
        tracing::error!(error = %e, "failed to fetch kiyo jwks");
        AppError::Unavailable
    })?;

    let jwks: JwkSet = res.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse kiyo jwks");
        AppError::Unavailable
    })?;

    let mut write = cache.write().await;
    write.jwks = Some(jwks.clone());
    write.fetched_at = Some(Utc::now());
    Ok(jwks)
}

pub async fn exchange_kiyo_code_for_token(
    http_client: &reqwest::Client,
    config: &Config,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> AppResult<String> {
    let mut params = vec![
        ("code", code),
        ("client_id", config.kiyo_client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];
    if !config.kiyo_client_secret.is_empty() {
        params.push(("client_secret", config.kiyo_client_secret.as_str()));
    }

    let token_url = format!("{}/oauth2/token", config.kiyo_issuer);
    let res = http_client
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "kiyo token exchange request failed");
            AppError::Unavailable
        })?;

    if !res.status().is_success() {
        tracing::warn!(status = %res.status(), "kiyo token exchange rejected");
        return Err(AppError::Unauthorized);
    }

    let token_resp: KiyoTokenResponse = res.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse kiyo token response");
        AppError::Unavailable
    })?;

    token_resp.id_token.ok_or(AppError::Unauthorized)
}

pub async fn verify_kiyo_id_token(
    id_token: &str,
    config: &Config,
    expected_nonce: &str,
    http_client: &reqwest::Client,
    jwks_cache: &RwLock<GoogleJwksCache>,
) -> AppResult<GoogleIdTokenClaims> {
    let header =
        decode_header(id_token).map_err(|_| AppError::BadRequest("invalid_token_header"))?;
    let kid = header
        .kid
        .ok_or(AppError::BadRequest("missing_token_kid"))?;

    let jwks = get_kiyo_jwks(http_client, &config.kiyo_issuer, jwks_cache).await?;
    let jwk = jwks
        .find(&kid)
        .ok_or(AppError::BadRequest("unknown_token_kid"))?;

    let decoding_key =
        DecodingKey::from_jwk(jwk).map_err(|_| AppError::BadRequest("invalid_jwk"))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[config.kiyo_client_id.as_str()]);
    validation.set_issuer(&[config.kiyo_issuer.as_str()]);
    validation.validate_exp = true;

    let token_data =
        decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation).map_err(|e| {
            tracing::warn!(error = %e, "kiyo id_token validation failed");
            AppError::Unauthorized
        })?;

    let claims = token_data.claims;
    if claims.nonce.as_deref() != Some(expected_nonce) {
        tracing::warn!("kiyo id_token nonce mismatch");
        return Err(AppError::Unauthorized);
    }

    Ok(claims)
}
