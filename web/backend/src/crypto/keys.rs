use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    crypto::{compute_fingerprint, compute_identity_fingerprint, validate_p256_public_key},
    error::{AppError, AppResult},
};

#[derive(Deserialize)]
pub struct RegisterKeysRequest {
    /// 65-byte uncompressed P-256 public key (base64url)
    pub ecdh_public: String,
    /// 65-byte uncompressed P-256 public key (base64url)
    pub ecdsa_public: String,
    /// Explicit flag required when rotating/resetting an existing identity key from a trusted device.
    #[serde(default)]
    pub allow_rotation: bool,
    /// Optional 64-byte ECDSA signature by the previous `ecdsa_public` over `b"chatw-key-rotate-v1:" || ecdh_bytes || ecdsa_bytes`.
    #[serde(default)]
    pub previous_key_signature: Option<String>,
}

#[derive(Serialize)]
pub struct PublicKeysResponse {
    pub user_id: Uuid,
    pub ecdh_public: String,
    pub ecdsa_public: String,
    pub fingerprint: String,
    pub identity_fingerprint: String,
}

#[derive(Deserialize)]
pub struct UploadPrekeysRequest {
    pub signed_prekey_id: Option<i32>,
    /// 65-byte uncompressed P-256 ECDH public key (base64url)
    pub signed_prekey_pub: String,
    /// 64-byte ECDSA signature over `b"chatw-spk-v2:" || signed_prekey_pub_bytes` (base64url)
    pub signed_prekey_sig: String,
    /// Optional batch of 65-byte uncompressed P-256 one-time prekey public keys (base64url)
    #[serde(default)]
    pub one_time_prekeys: Vec<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_register_keys))
        .route("/prekeys", put(handle_put_prekeys))
        .route("/{userId}", get(handle_get_keys))
        .route("/{userId}/history", get(handle_get_key_history))
        .route("/{userId}/bundle", get(handle_get_prekey_bundle))
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
    let identity_fp = compute_identity_fingerprint(&ecdh_bytes, &ecdsa_bytes);

    // Check if this user already has registered identity keys.
    let existing = sqlx::query(
        "SELECT ecdh_public, ecdsa_public, fingerprint FROM device_keys WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    let mut action = "initial_register";
    if let Some(ex) = existing {
        let ex_ecdh: Vec<u8> = ex.try_get("ecdh_public")?;
        let ex_ecdsa: Vec<u8> = ex.try_get("ecdsa_public")?;
        let ex_fp: Vec<u8> = ex.try_get("fingerprint")?;

        if ex_ecdh == ecdh_bytes && ex_ecdsa == ecdsa_bytes {
            // Idempotent re-sync of the exact same identity keypair.
            return Ok(Json(PublicKeysResponse {
                user_id: user.id,
                ecdh_public: payload.ecdh_public,
                ecdsa_public: payload.ecdsa_public,
                fingerprint: URL_SAFE_NO_PAD.encode(&ex_fp),
                identity_fingerprint: URL_SAFE_NO_PAD.encode(identity_fp),
            }));
        }

        // Identity keys differ! Enforce strict device trust & rotation authorization.
        if let Some(did) = user.device_id {
            let trust: Option<String> = sqlx::query_scalar(
                "SELECT trust_state FROM devices WHERE id = $1 AND user_id = $2",
            )
            .bind(did)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
            if matches!(trust.as_deref(), Some("pending") | Some("revoked")) {
                return Err(AppError::Forbidden(
                    "cannot_overwrite_identity_key_from_untrusted_device",
                ));
            }
        } else {
            let trusted_devices_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM devices WHERE user_id = $1 AND trust_state = 'trusted'",
            )
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
            if trusted_devices_count > 0 && !payload.allow_rotation && payload.previous_key_signature.is_none() {
                return Err(AppError::Conflict(
                    "identity_key_already_exists_require_recovery_or_rotation",
                ));
            }
        }

        // Verify signed rotation proof if provided, otherwise require explicit `allow_rotation: true`.
        if let Some(ref sig_b64) = payload.previous_key_signature {
            let sig_bytes = URL_SAFE_NO_PAD
                .decode(sig_b64)
                .map_err(|_| AppError::BadRequest("invalid_rotation_signature_encoding"))?;
            let vk = VerifyingKey::from_sec1_bytes(&ex_ecdsa)
                .map_err(|_| AppError::BadRequest("invalid_existing_verifying_key"))?;
            let sig = Signature::from_slice(&sig_bytes)
                .map_err(|_| AppError::BadRequest("invalid_rotation_signature_format"))?;
            let mut msg = Vec::with_capacity(20 + ecdh_bytes.len() + ecdsa_bytes.len());
            msg.extend_from_slice(b"chatw-key-rotate-v1:");
            msg.extend_from_slice(&ecdh_bytes);
            msg.extend_from_slice(&ecdsa_bytes);
            vk.verify(&msg, &sig)
                .map_err(|_| AppError::Forbidden("invalid_previous_key_rotation_signature"))?;
            action = "signed_rotate";
        } else if payload.allow_rotation {
            action = "explicit_rotate";
        } else {
            return Err(AppError::Conflict(
                "identity_key_already_exists_require_recovery_or_rotation",
            ));
        }
    }

    sqlx::query!(
        r#"
        INSERT INTO device_keys (user_id, ecdh_public, ecdsa_public, fingerprint)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id) DO UPDATE
        SET ecdh_public = EXCLUDED.ecdh_public,
            ecdsa_public = EXCLUDED.ecdsa_public,
            fingerprint = EXCLUDED.fingerprint,
            created_at = now()
        "#,
        user.id,
        &ecdh_bytes[..],
        &ecdsa_bytes[..],
        &fingerprint[..]
    )
    .execute(&state.db)
    .await?;

    let _ = sqlx::query(
        r#"
        INSERT INTO identity_key_history (user_id, device_id, ecdh_public, ecdsa_public, fingerprint, action)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user.id)
    .bind(user.device_id)
    .bind(&ecdh_bytes[..])
    .bind(&ecdsa_bytes[..])
    .bind(&identity_fp[..])
    .bind(action)
    .execute(&state.db)
    .await;

    if action != "initial_register" {
        crate::security::log_event(&state, user.id, "e2ee_key_rotated", &headers).await;
    }

    Ok(Json(PublicKeysResponse {
        user_id: user.id,
        ecdh_public: payload.ecdh_public,
        ecdsa_public: payload.ecdsa_public,
        fingerprint: URL_SAFE_NO_PAD.encode(fingerprint),
        identity_fingerprint: URL_SAFE_NO_PAD.encode(identity_fp),
    }))
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

    let identity_fp = compute_identity_fingerprint(&row.ecdh_public, &row.ecdsa_public);

    Ok(Json(PublicKeysResponse {
        user_id: target_user_id,
        ecdh_public: URL_SAFE_NO_PAD.encode(&row.ecdh_public),
        ecdsa_public: URL_SAFE_NO_PAD.encode(&row.ecdsa_public),
        fingerprint: URL_SAFE_NO_PAD.encode(&row.fingerprint),
        identity_fingerprint: URL_SAFE_NO_PAD.encode(identity_fp),
    }))
}

async fn handle_get_key_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let rows = sqlx::query(
        r#"
        SELECT id, device_id, fingerprint, action, created_at
        FROM identity_key_history
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 25
        "#,
    )
    .bind(target_user_id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let fp: Vec<u8> = r.try_get("fingerprint").unwrap_or_default();
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok(),
                "device_id": r.try_get::<Option<Uuid>, _>("device_id").ok().flatten(),
                "identity_fingerprint": URL_SAFE_NO_PAD.encode(fp),
                "action": r.try_get::<String, _>("action").unwrap_or_default(),
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "user_id": target_user_id,
        "history": items,
    })))
}

async fn handle_put_prekeys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UploadPrekeysRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    let spk_pub = URL_SAFE_NO_PAD
        .decode(&payload.signed_prekey_pub)
        .map_err(|_| AppError::BadRequest("invalid_signed_prekey_encoding"))?;
    validate_p256_public_key(&spk_pub)?;

    let spk_sig = URL_SAFE_NO_PAD
        .decode(&payload.signed_prekey_sig)
        .map_err(|_| AppError::BadRequest("invalid_signed_prekey_sig_encoding"))?;
    if spk_sig.len() != 64 {
        return Err(AppError::BadRequest("invalid_signed_prekey_sig_length"));
    }

    // Verify signed prekey against the user's registered ECDSA identity key.
    let id_row = sqlx::query("SELECT ecdsa_public FROM device_keys WHERE user_id = $1")
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::BadRequest("identity_key_not_registered"))?;
    let ecdsa_pub: Vec<u8> = id_row.try_get("ecdsa_public")?;
    let vk = VerifyingKey::from_sec1_bytes(&ecdsa_pub)
        .map_err(|_| AppError::BadRequest("invalid_identity_ecdsa_key"))?;
    let sig = Signature::from_slice(&spk_sig)
        .map_err(|_| AppError::BadRequest("invalid_signed_prekey_signature"))?;
    let mut signed_msg = Vec::with_capacity(13 + spk_pub.len());
    signed_msg.extend_from_slice(b"chatw-spk-v2:");
    signed_msg.extend_from_slice(&spk_pub);
    vk.verify(&signed_msg, &sig)
        .map_err(|_| AppError::BadRequest("signed_prekey_signature_verification_failed"))?;

    let mut clean_otks = Vec::new();
    for otk_b64 in payload.one_time_prekeys.iter().take(64) {
        let otk_bytes = URL_SAFE_NO_PAD
            .decode(otk_b64)
            .map_err(|_| AppError::BadRequest("invalid_one_time_prekey_encoding"))?;
        validate_p256_public_key(&otk_bytes)?;
        clean_otks.push(otk_b64.clone());
    }

    let spk_id = payload.signed_prekey_id.unwrap_or(1);
    let otks_json = serde_json::to_value(&clean_otks).unwrap_or_else(|_| serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO device_prekeys (user_id, device_id, signed_prekey_id, signed_prekey_pub, signed_prekey_sig, one_time_prekeys, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, now())
        ON CONFLICT (user_id) DO UPDATE SET
            device_id = EXCLUDED.device_id,
            signed_prekey_id = EXCLUDED.signed_prekey_id,
            signed_prekey_pub = EXCLUDED.signed_prekey_pub,
            signed_prekey_sig = EXCLUDED.signed_prekey_sig,
            one_time_prekeys = EXCLUDED.one_time_prekeys,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(user.device_id)
    .bind(spk_id)
    .bind(&spk_pub[..])
    .bind(&spk_sig[..])
    .bind(&otks_json)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "saved": true,
        "signed_prekey_id": spk_id,
        "one_time_prekey_count": clean_otks.len(),
    })))
}

async fn handle_get_prekey_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let _user = authenticate_session(&state.db, &state.config, session_token).await?;

    let row = sqlx::query(
        "SELECT ecdh_public, ecdsa_public, fingerprint FROM device_keys WHERE user_id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("keys_not_found"))?;

    let ecdh_pub: Vec<u8> = row.try_get("ecdh_public")?;
    let ecdsa_pub: Vec<u8> = row.try_get("ecdsa_public")?;
    let fp: Vec<u8> = row.try_get("fingerprint")?;
    let identity_fp = compute_identity_fingerprint(&ecdh_pub, &ecdsa_pub);

    let prekey_row = sqlx::query(
        "SELECT signed_prekey_id, signed_prekey_pub, signed_prekey_sig, one_time_prekeys FROM device_prekeys WHERE user_id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(&state.db)
    .await?;

    let mut spk_id: Option<i32> = None;
    let mut spk_pub_b64: Option<String> = None;
    let mut spk_sig_b64: Option<String> = None;
    let mut popped_otk: Option<String> = None;

    if let Some(pk) = prekey_row {
        spk_id = pk.try_get("signed_prekey_id").ok();
        spk_pub_b64 = pk
            .try_get::<Vec<u8>, _>("signed_prekey_pub")
            .ok()
            .map(|b| URL_SAFE_NO_PAD.encode(b));
        spk_sig_b64 = pk
            .try_get::<Vec<u8>, _>("signed_prekey_sig")
            .ok()
            .map(|b| URL_SAFE_NO_PAD.encode(b));

        if let Ok(serde_json::Value::Array(mut arr)) = pk.try_get::<serde_json::Value, _>("one_time_prekeys")
            && !arr.is_empty()
        {
            if let Some(serde_json::Value::String(s)) = arr.pop() {
                popped_otk = Some(s);
                let _ = sqlx::query("UPDATE device_prekeys SET one_time_prekeys = $2 WHERE user_id = $1")
                    .bind(target_user_id)
                    .bind(serde_json::Value::Array(arr))
                    .execute(&state.db)
                    .await;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "user_id": target_user_id,
        "ecdh_public": URL_SAFE_NO_PAD.encode(ecdh_pub),
        "ecdsa_public": URL_SAFE_NO_PAD.encode(ecdsa_pub),
        "fingerprint": URL_SAFE_NO_PAD.encode(fp),
        "identity_fingerprint": URL_SAFE_NO_PAD.encode(identity_fp),
        "signed_prekey_id": spk_id,
        "signed_prekey_pub": spk_pub_b64,
        "signed_prekey_sig": spk_sig_b64,
        "one_time_prekey": popped_otk,
    })))
}
