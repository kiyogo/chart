use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    crypto::{compute_fingerprint, validate_p256_public_key},
    error::{AppError, AppResult},
};

#[derive(Deserialize)]
pub struct RegisterKeysRequest {
    /// 65-byte uncompressed P-256 public key (base64url)
    pub ecdh_public: String,
    /// 65-byte uncompressed P-256 public key (base64url)
    pub ecdsa_public: String,
}

#[derive(Serialize)]
pub struct PublicKeysResponse {
    pub user_id: Uuid,
    pub ecdh_public: String,
    pub ecdsa_public: String,
    pub fingerprint: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_register_keys))
        .route("/{userId}", get(handle_get_keys))
}

async fn handle_register_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterKeysRequest>,
) -> AppResult<Json<PublicKeysResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let ecdh_bytes = URL_SAFE_NO_PAD
        .decode(&payload.ecdh_public)
        .map_err(|_| AppError::BadRequest("invalid_ecdh_encoding"))?;
    let ecdsa_bytes = URL_SAFE_NO_PAD
        .decode(&payload.ecdsa_public)
        .map_err(|_| AppError::BadRequest("invalid_ecdsa_encoding"))?;

    validate_p256_public_key(&ecdh_bytes)?;
    validate_p256_public_key(&ecdsa_bytes)?;

    let fingerprint = compute_fingerprint(&ecdh_bytes);

    let res = sqlx::query!(
        r#"
        INSERT INTO device_keys (user_id, ecdh_public, ecdsa_public, fingerprint)
        VALUES ($1, $2, $3, $4)
        "#,
        user.id,
        &ecdh_bytes[..],
        &ecdsa_bytes[..],
        &fingerprint[..]
    )
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => Ok(Json(PublicKeysResponse {
            user_id: user.id,
            ecdh_public: payload.ecdh_public,
            ecdsa_public: payload.ecdsa_public,
            fingerprint: URL_SAFE_NO_PAD.encode(fingerprint),
        })),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(AppError::Conflict("keys_already_registered"))
        }
        Err(e) => Err(e.into()),
    }
}

async fn handle_get_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<Uuid>,
) -> AppResult<Json<PublicKeysResponse>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query!(
        r#"
        SELECT ecdh_public, ecdsa_public, fingerprint
        FROM device_keys
        WHERE user_id = $1
        "#,
        target_user_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("keys_not_found"))?;

    Ok(Json(PublicKeysResponse {
        user_id: target_user_id,
        ecdh_public: URL_SAFE_NO_PAD.encode(&row.ecdh_public),
        ecdsa_public: URL_SAFE_NO_PAD.encode(&row.ecdsa_public),
        fingerprint: URL_SAFE_NO_PAD.encode(&row.fingerprint),
    }))
}
