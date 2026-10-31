use crate::{
    auth::session::OAUTH_COOKIE_NAME,
    config::Config,
    error::{AppError, AppResult},
};
use axum::{
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

type HmacSha256 = Hmac<Sha256>;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

#[derive(Serialize, Deserialize)]
pub struct OAuthStatePayload {
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
    pub exp: i64,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    id_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleIdTokenClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: usize,
    pub nonce: Option<String>,
    pub picture: Option<String>,
}

pub struct GoogleJwksCache {
    pub jwks: Option<JwkSet>,
    pub fetched_at: Option<chrono::DateTime<Utc>>,
}

impl GoogleJwksCache {
    pub fn new() -> Self {
        Self {
            jwks: None,
            fetched_at: None,
        }
    }
}

impl Default for GoogleJwksCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn generate_pkce() -> (String, String) {
    let mut verifier_bytes = [0u8; 32];
    getrandom::fill(&mut verifier_bytes).expect("CSPRNG");
    let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    (code_verifier, challenge)
}

pub fn generate_random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("CSPRNG");
    hex::encode(bytes)
}

fn sign_oauth_cookie(secret: &[u8], payload: &OAuthStatePayload) -> String {
    let json = serde_json::to_vec(payload).unwrap();
    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(&json);
    let sig = mac.finalize().into_bytes();
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(&json),
        URL_SAFE_NO_PAD.encode(sig)
    )
}

pub fn verify_oauth_cookie(secret: &[u8], cookie_val: &str) -> Option<OAuthStatePayload> {
    let (data_b64, sig_b64) = cookie_val.split_once('.')?;
    let data = URL_SAFE_NO_PAD.decode(data_b64).ok()?;
    let sig = URL_SAFE_NO_PAD.decode(sig_b64).ok()?;

    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
    mac.update(&data);
    if mac.verify_slice(&sig).is_err() {
        return None;
    }

    let payload: OAuthStatePayload = serde_json::from_slice(&data).ok()?;
    if Utc::now().timestamp() > payload.exp {
        return None;
    }
    Some(payload)
}

/// GET /api/auth/google
pub async fn google_login_handler(config: &Config) -> AppResult<Response> {
    let state = generate_random_token();
    let nonce = generate_random_token();
    let (code_verifier, code_challenge) = generate_pkce();

    let oauth_payload = OAuthStatePayload {
        state: state.clone(),
        nonce: nonce.clone(),
        code_verifier,
        exp: Utc::now().timestamp() + 600, // 10 minutes
    };

    let cookie_val = sign_oauth_cookie(&config.session_secret, &oauth_payload);
    let secure_flag = if config.cookie_secure { "; Secure" } else { "" };
    let cookie_header = format!(
        "{OAUTH_COOKIE_NAME}={cookie_val}; Path=/api/auth; HttpOnly; SameSite=Lax; Max-Age=600{secure_flag}"
    );

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
        GOOGLE_AUTH_URL,
        urlencoding::encode(&config.google_client_id),
        urlencoding::encode(&config.google_redirect_url),
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

/// Fetch Google JWKS or return cached version (refreshed every hour).
pub async fn get_google_jwks(
    client: &reqwest::Client,
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

    let res = client.get(GOOGLE_CERTS_URL).send().await.map_err(|e| {
        tracing::error!(error = %e, "failed to fetch google jwks");
        AppError::Unavailable
    })?;

    let jwks: JwkSet = res.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse google jwks");
        AppError::Unavailable
    })?;

    let mut write = cache.write().await;
    write.jwks = Some(jwks.clone());
    write.fetched_at = Some(Utc::now());

    Ok(jwks)
}

/// Verify Google ID token and return claims.
pub async fn verify_google_id_token(
    id_token: &str,
    client_id: &str,
    expected_nonce: &str,
    http_client: &reqwest::Client,
    jwks_cache: &RwLock<GoogleJwksCache>,
) -> AppResult<GoogleIdTokenClaims> {
    let header =
        decode_header(id_token).map_err(|_| AppError::BadRequest("invalid_token_header"))?;
    let kid = header
        .kid
        .ok_or(AppError::BadRequest("missing_token_kid"))?;

    let jwks = get_google_jwks(http_client, jwks_cache).await?;
    let jwk = jwks
        .find(&kid)
        .ok_or(AppError::BadRequest("unknown_token_kid"))?;

    let decoding_key =
        DecodingKey::from_jwk(jwk).map_err(|_| AppError::BadRequest("invalid_jwk"))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
    validation.validate_exp = true;

    let token_data =
        decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation).map_err(|e| {
            tracing::warn!(error = %e, "google id_token validation failed");
            AppError::Unauthorized
        })?;

    let claims = token_data.claims;
    if claims.nonce.as_deref() != Some(expected_nonce) {
        tracing::warn!("google id_token nonce mismatch");
        return Err(AppError::Unauthorized);
    }

    Ok(claims)
}

/// Exchange authorization code with Google for tokens.
pub async fn exchange_code_for_token(
    http_client: &reqwest::Client,
    config: &Config,
    code: &str,
    code_verifier: &str,
) -> AppResult<String> {
    let params = [
        ("code", code),
        ("client_id", &config.google_client_id),
        ("client_secret", &config.google_client_secret),
        ("redirect_uri", &config.google_redirect_url),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];

    let res = http_client
        .post(GOOGLE_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "google token exchange request failed");
            AppError::Unavailable
        })?;

    if !res.status().is_success() {
        tracing::warn!(status = %res.status(), "google token exchange rejected");
        return Err(AppError::Unauthorized);
    }

    let token_resp: GoogleTokenResponse = res.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse google token response");
        AppError::Unavailable
    })?;

    token_resp.id_token.ok_or(AppError::Unauthorized)
}
