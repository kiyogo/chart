use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use axum::http::HeaderMap;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::{AppState, google::verify_google_id_token},
    error::{AppError, AppResult},
    security::{check_second_factor, log_event},
};

const MAX_REAUTH_FAILS: i32 = 5;
const LOCKOUT_SECS: i64 = 900;
const EXPORT_TOKEN_TTL_SECS: i64 = 300; // 5 minutes

#[derive(Clone, Debug, Serialize)]
pub struct ReauthMethodsResponse {
    pub has_export_password: bool,
    pub totp_enabled: bool,
    pub backup_codes_remaining: i64,
    pub passkeys_count: i64,
    pub has_google_account: bool,
    pub locked_until: Option<String>,
}

fn argon2_instance() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, Some(32)).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash_export_token(raw_token: &str) -> Vec<u8> {
    Sha256::digest(raw_token.as_bytes()).to_vec()
}

pub async fn get_reauth_methods(pool: &PgPool, user_id: Uuid) -> AppResult<ReauthMethodsResponse> {
    let has_export_password: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_export_credentials WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let urow = sqlx::query("SELECT totp_enabled_at, google_sub FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let totp_enabled: Option<chrono::DateTime<chrono::Utc>> = urow.try_get("totp_enabled_at")?;
    let google_sub: String = urow.try_get("google_sub")?;
    let has_google_account = !google_sub.starts_with("dev:") && !google_sub.starts_with("local:");

    let backup_codes_remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backup_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let passkeys_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passkeys WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let locked_until: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT locked_until FROM reauth_attempts WHERE user_id = $1 AND locked_until > now()",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(ReauthMethodsResponse {
        has_export_password,
        totp_enabled: totp_enabled.is_some(),
        backup_codes_remaining,
        passkeys_count,
        has_google_account,
        locked_until: locked_until.map(|t| t.to_rfc3339()),
    })
}

async fn check_rate_limit(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    let row = sqlx::query(
        "SELECT failed_count, window_started_at, locked_until FROM reauth_attempts WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let locked_until: Option<chrono::DateTime<chrono::Utc>> = r.try_get("locked_until")?;
        if let Some(lu) = locked_until {
            if lu > chrono::Utc::now() {
                return Err(AppError::TooManyRequests);
            }
        }
    }
    Ok(())
}

async fn record_reauth_failure(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO reauth_attempts (user_id, failed_count, window_started_at, locked_until)
         VALUES ($1, 1, now(), NULL)
         ON CONFLICT (user_id) DO UPDATE SET
           failed_count = CASE
             WHEN reauth_attempts.window_started_at < now() - interval '15 minutes' THEN 1
             ELSE reauth_attempts.failed_count + 1
           END,
           window_started_at = CASE
             WHEN reauth_attempts.window_started_at < now() - interval '15 minutes' THEN now()
             ELSE reauth_attempts.window_started_at
           END,
           locked_until = CASE
             WHEN (CASE WHEN reauth_attempts.window_started_at < now() - interval '15 minutes' THEN 1 ELSE reauth_attempts.failed_count + 1 END) >= $2
             THEN now() + ($3 || ' seconds')::interval
             ELSE NULL
           END",
    )
    .bind(user_id)
    .bind(MAX_REAUTH_FAILS)
    .bind(LOCKOUT_SECS)
    .execute(pool)
    .await?;
    Ok(())
}

async fn clear_reauth_failures(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM reauth_attempts WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_export_password(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
    current_password: Option<&str>,
    totp_or_backup: Option<&str>,
    new_password: &str,
) -> AppResult<()> {
    if new_password.chars().count() < 6 || new_password.len() > 128 {
        return Err(AppError::BadRequest("invalid_password_length"));
    }
    check_rate_limit(&state.db, user_id).await?;

    let existing_phc: Option<String> = sqlx::query_scalar(
        "SELECT password_phc FROM user_export_credentials WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(phc) = existing_phc {
        let mut verified = false;
        if let Some(cur) = current_password {
            if verify_argon2_phc(&phc, cur) {
                verified = true;
            }
        }
        if !verified {
            if let Some(code) = totp_or_backup {
                if check_second_factor(state, user_id, code).await?.is_some() {
                    verified = true;
                }
            }
        }
        if !verified {
            record_reauth_failure(&state.db, user_id).await?;
            log_event(state, user_id, "backup_reauth_fail", headers).await;
            return Err(AppError::Unauthorized);
        }
    } else {
        // First-time export password setup must still enforce 2FA if TOTP is enabled on the account.
        crate::security::require_second_factor(state, user_id, totp_or_backup, headers).await?;
    }

    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(|_| AppError::Internal)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AppError::Internal)?;
    let phc = argon2_instance()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal)?
        .to_string();

    sqlx::query(
        "INSERT INTO user_export_credentials (user_id, password_phc, updated_at)
         VALUES ($1, $2, now())
         ON CONFLICT (user_id) DO UPDATE SET password_phc = EXCLUDED.password_phc, updated_at = now()",
    )
    .bind(user_id)
    .bind(phc)
    .execute(&state.db)
    .await?;

    clear_reauth_failures(&state.db, user_id).await?;
    log_event(state, user_id, "export_pw_updated", headers).await;
    Ok(())
}

fn verify_argon2_phc(phc: &str, candidate: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    argon2_instance().verify_password(candidate.as_bytes(), &parsed).is_ok()
}

pub async fn issue_passkey_challenge(
    pool: &PgPool,
    user_id: Uuid,
    purpose: &str,
) -> AppResult<(Uuid, String)> {
    if purpose != "register" && purpose != "reauth" {
        return Err(AppError::BadRequest("invalid_purpose"));
    }
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).map_err(|_| AppError::Internal)?;
    let b64 = URL_SAFE_NO_PAD.encode(raw);
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO passkey_challenges (id, user_id, challenge_b64, purpose, expires_at)
         VALUES ($1, $2, $3, $4, now() + interval '5 minutes')",
    )
    .bind(id)
    .bind(user_id)
    .bind(&b64)
    .bind(purpose)
    .execute(pool)
    .await?;
    Ok((id, b64))
}

pub async fn register_device_passkey(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
    credential_id: &str,
    public_key_spki_b64: &str,
    label: &str,
) -> AppResult<Uuid> {
    if credential_id.len() < 8 || credential_id.len() > 256 {
        return Err(AppError::BadRequest("invalid_credential_id"));
    }
    let spki = URL_SAFE_NO_PAD
        .decode(public_key_spki_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(public_key_spki_b64))
        .map_err(|_| AppError::BadRequest("invalid_public_key"))?;
    if spki.len() < 32 || spki.len() > 256 {
        return Err(AppError::BadRequest("invalid_public_key"));
    }
    let id = Uuid::new_v4();
    let clean_label: String = label.trim().chars().take(64).collect();
    sqlx::query(
        "INSERT INTO passkeys (id, user_id, credential_id, public_key_spki, label)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(user_id)
    .bind(credential_id)
    .bind(spki)
    .bind(if clean_label.is_empty() { "Khóa bảo mật" } else { &clean_label })
    .execute(&state.db)
    .await
    .map_err(|_| AppError::Conflict("passkey_exists"))?;
    log_event(state, user_id, "passkey_registered", headers).await;
    Ok(id)
}

#[derive(Deserialize)]
pub struct ReauthRequest {
    pub scope: String, // "kdata" | "zip_encrypted" | "zip_unencrypted"
    pub method: String, // "password" | "totp" | "backup_code" | "passkey" | "google_id_token"
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub challenge_id: Option<Uuid>,
    #[serde(default)]
    pub credential_id: Option<String>,
    #[serde(default)]
    pub signature_b64: Option<String>,
}

#[derive(Serialize)]
pub struct ReauthResponse {
    pub authorization_id: Uuid,
    pub export_token: String,
    pub scope: String,
    pub expires_in_secs: i64,
    pub requires_multi_step: bool,
}

/// Verifies strong account re-authentication and mints a short-lived, single-scope export token.
/// A regular session cookie alone NEVER authorizes any export.
pub async fn verify_and_issue_export_token(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
    req: &ReauthRequest,
) -> AppResult<ReauthResponse> {
    if !matches!(req.scope.as_str(), "kdata" | "zip_encrypted" | "zip_unencrypted") {
        return Err(AppError::BadRequest("invalid_export_scope"));
    }
    check_rate_limit(&state.db, user_id).await?;

    let method_used: &'static str = match req.method.as_str() {
        "password" => {
            let phc: Option<String> = sqlx::query_scalar(
                "SELECT password_phc FROM user_export_credentials WHERE user_id = $1",
            )
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
            let ok = phc.as_deref().is_some_and(|p| !req.secret.is_empty() && verify_argon2_phc(p, &req.secret));
            if !ok {
                record_reauth_failure(&state.db, user_id).await?;
                log_event(state, user_id, "backup_reauth_fail", headers).await;
                return Err(AppError::Unauthorized);
            }
            "password"
        }
        "totp" | "backup_code" => {
            match check_second_factor(state, user_id, &req.secret).await? {
                Some(m) => m,
                None => {
                    record_reauth_failure(&state.db, user_id).await?;
                    log_event(state, user_id, "backup_reauth_fail", headers).await;
                    return Err(AppError::Unauthorized);
                }
            }
        }
        "google_id_token" => {
            let expected_sub: String = sqlx::query_scalar("SELECT google_sub FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&state.db)
                .await?;
            if expected_sub.starts_with("dev:") || expected_sub.starts_with("local:") {
                return Err(AppError::BadRequest("google_reauth_unavailable"));
            }
            let cid = req.challenge_id.ok_or(AppError::BadRequest("missing_challenge"))?;
            let nonce: Option<String> = sqlx::query_scalar(
                "DELETE FROM passkey_challenges WHERE id = $1 AND user_id = $2 AND purpose = 'reauth' AND expires_at > now() RETURNING challenge_b64",
            )
            .bind(cid)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
            let Some(expected_nonce) = nonce else {
                record_reauth_failure(&state.db, user_id).await?;
                return Err(AppError::Unauthorized);
            };
            let claims = verify_google_id_token(
                &req.secret,
                &state.config.google_client_id,
                &expected_nonce,
                &state.http_client,
                &state.jwks_cache,
            )
            .await
            .map_err(|_| AppError::Unauthorized)?;
            if claims.sub != expected_sub {
                record_reauth_failure(&state.db, user_id).await?;
                log_event(state, user_id, "backup_reauth_fail", headers).await;
                return Err(AppError::Unauthorized);
            }
            "google_reauth"
        }
        "passkey" => {
            let cid = req.challenge_id.ok_or(AppError::BadRequest("missing_challenge"))?;
            let cred_id = req.credential_id.as_deref().ok_or(AppError::BadRequest("missing_credential"))?;
            let sig_b64 = req.signature_b64.as_deref().ok_or(AppError::BadRequest("missing_signature"))?;
            let ch_row = sqlx::query(
                "DELETE FROM passkey_challenges WHERE id = $1 AND user_id = $2 AND purpose = 'reauth' AND expires_at > now() RETURNING challenge_b64",
            )
            .bind(cid)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
            let Some(ch) = ch_row else {
                record_reauth_failure(&state.db, user_id).await?;
                log_event(state, user_id, "backup_reauth_fail", headers).await;
                return Err(AppError::Unauthorized);
            };
            let challenge_b64: String = ch.try_get("challenge_b64")?;
            let pk_bytes: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT public_key_spki FROM passkeys WHERE user_id = $1 AND credential_id = $2",
            )
            .bind(user_id)
            .bind(cred_id)
            .fetch_optional(&state.db)
            .await?;
            let Some(raw_pk) = pk_bytes else {
                record_reauth_failure(&state.db, user_id).await?;
                return Err(AppError::Unauthorized);
            };
            let sig_bytes = URL_SAFE_NO_PAD
                .decode(sig_b64)
                .or_else(|_| base64::engine::general_purpose::STANDARD.decode(sig_b64))
                .map_err(|_| AppError::Unauthorized)?;
            let vk = VerifyingKey::from_sec1_bytes(&raw_pk).map_err(|_| AppError::Unauthorized)?;
            let sig = Signature::from_slice(&sig_bytes).map_err(|_| AppError::Unauthorized)?;
            if vk.verify(challenge_b64.as_bytes(), &sig).is_err() {
                record_reauth_failure(&state.db, user_id).await?;
                log_event(state, user_id, "backup_reauth_fail", headers).await;
                return Err(AppError::Unauthorized);
            }
            "passkey"
        }
        _ => return Err(AppError::BadRequest("unsupported_reauth_method")),
    };

    clear_reauth_failures(&state.db, user_id).await?;

    let mut tok_bytes = [0u8; 32];
    getrandom::fill(&mut tok_bytes).map_err(|_| AppError::Internal)?;
    let raw_token = URL_SAFE_NO_PAD.encode(tok_bytes);
    let token_hash = hash_export_token(&raw_token);
    let auth_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO export_authorizations (id, user_id, token_hash, scope, reauth_method, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + ($6 || ' seconds')::interval)",
    )
    .bind(auth_id)
    .bind(user_id)
    .bind(&token_hash)
    .bind(&req.scope)
    .bind(method_used)
    .bind(EXPORT_TOKEN_TTL_SECS)
    .execute(&state.db)
    .await?;

    log_event(state, user_id, "backup_reauth_ok", headers).await;

    Ok(ReauthResponse {
        authorization_id: auth_id,
        export_token: raw_token,
        scope: req.scope.clone(),
        expires_in_secs: EXPORT_TOKEN_TTL_SECS,
        requires_multi_step: req.scope == "zip_unencrypted",
    })
}

/// Multi-step explicit confirmation required ONLY for locked unencrypted ZIP exports.
pub async fn advance_unencrypted_zip_step(
    pool: &PgPool,
    user_id: Uuid,
    raw_token: &str,
    step: &str,
) -> AppResult<()> {
    let hash = hash_export_token(raw_token);
    let updated = match step {
        "ack_risk" => {
            sqlx::query(
                "UPDATE export_authorizations SET risk_acknowledged = TRUE, preview_viewed = TRUE
                 WHERE user_id = $1 AND token_hash = $2 AND scope = 'zip_unencrypted'
                   AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(user_id)
            .bind(&hash)
            .execute(pool)
            .await?
            .rows_affected()
        }
        "confirm_final" => {
            sqlx::query(
                "UPDATE export_authorizations SET step_confirmed = TRUE
                 WHERE user_id = $1 AND token_hash = $2 AND scope = 'zip_unencrypted'
                   AND risk_acknowledged = TRUE AND preview_viewed = TRUE
                   AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL",
            )
            .bind(user_id)
            .bind(&hash)
            .execute(pool)
            .await?
            .rows_affected()
        }
        _ => return Err(AppError::BadRequest("invalid_zip_step")),
    };
    if updated == 0 {
        return Err(AppError::Forbidden("export_authorization_invalid_or_expired"));
    }
    Ok(())
}

/// Atomically consumes a short-lived single-use export authorization token.
/// Refuses expired, wrong-scope, already-used, revoked, or unconfirmed unencrypted-ZIP tokens.
pub async fn consume_export_token(
    pool: &PgPool,
    user_id: Uuid,
    raw_token: &str,
    expected_scope: &str,
) -> AppResult<Uuid> {
    if raw_token.is_empty() {
        return Err(AppError::Forbidden("export_reauth_required"));
    }
    let hash = hash_export_token(raw_token);
    let auth_id: Option<Uuid> = if expected_scope == "zip_unencrypted" {
        sqlx::query_scalar(
            "UPDATE export_authorizations
             SET used_at = now(), revoked_at = now()
             WHERE user_id = $1 AND token_hash = $2 AND scope = 'zip_unencrypted'
               AND risk_acknowledged = TRUE AND preview_viewed = TRUE AND step_confirmed = TRUE
               AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL
             RETURNING id",
        )
        .bind(user_id)
        .bind(&hash)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "UPDATE export_authorizations
             SET used_at = now()
             WHERE user_id = $1 AND token_hash = $2 AND scope = $3
               AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL
             RETURNING id",
        )
        .bind(user_id)
        .bind(&hash)
        .bind(expected_scope)
        .fetch_optional(pool)
        .await?
    };

    auth_id.ok_or(AppError::Forbidden("export_reauth_required"))
}

pub async fn revoke_export_token(pool: &PgPool, user_id: Uuid, raw_token: &str) -> AppResult<()> {
    let hash = hash_export_token(raw_token);
    sqlx::query(
        "UPDATE export_authorizations SET revoked_at = now() WHERE user_id = $1 AND token_hash = $2",
    )
    .bind(user_id)
    .bind(&hash)
    .execute(pool)
    .await?;
    Ok(())
}
