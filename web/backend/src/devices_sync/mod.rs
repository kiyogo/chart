use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use getrandom::fill as random_fill;
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{AuthUser, SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    storage::migration,
    websocket::WsEvent,
};

pub fn device_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handle_list_devices))
        .route("/register", post(handle_register_device))
        .route("/current", get(handle_current_device_status))
        .route("/requests/{id}/approve", post(handle_approve_request))
        .route("/requests/{id}/reject", post(handle_reject_request))
        .route("/recover-lost", post(handle_recover_lost_devices))
        .route("/{id}", delete(handle_revoke_device))
}

pub fn sync_routes() -> Router<AppState> {
    Router::new()
        .route("/delta", get(handle_delta_sync))
        .route("/events", get(handle_sync_events))
        .route("/phase-status", post(handle_update_phase_status))
}

pub fn recovery_routes() -> Router<AppState> {
    Router::new()
        .route("/requests", get(handle_list_recovery_requests).post(handle_create_recovery_request))
        .route("/requests/{id}/respond", post(handle_respond_recovery_request))
        .route("/requests/{id}/signal", post(handle_recovery_p2p_signal))
        .route("/requests/{id}/complete", post(handle_complete_recovery_request))
}

pub fn key_backup_routes() -> Router<AppState> {
    Router::new().route("/backup", get(handle_get_key_backup).put(handle_put_key_backup))
}

async fn caller(state: &AppState, headers: &HeaderMap) -> AppResult<AuthUser> {
    let tok = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    authenticate_session(&state.db, &state.config, tok).await
}

fn generate_6_digit_code() -> String {
    let mut b = [0u8; 4];
    let _ = random_fill(&mut b);
    let num = u32::from_be_bytes(b) % 1_000_000;
    format!("{num:06}")
}

#[derive(Deserialize)]
struct RegisterDeviceReq {
    pub client_device_id: String,
    pub platform: Option<String>,
    pub device_name: Option<String>,
    pub device_label: Option<String>,
    pub ecdh_public_b64: Option<String>,
    pub ecdsa_public_b64: Option<String>,
    pub key_fingerprint_b64: Option<String>,
}

async fn handle_register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterDeviceReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let cid = req.client_device_id.trim();
    if cid.len() < 8 || cid.len() > 128 {
        return Err(AppError::BadRequest("invalid_client_device_id"));
    }
    let platform = req
        .platform
        .as_deref()
        .filter(|p| matches!(*p, "web" | "desktop" | "mobile" | "cli"))
        .unwrap_or("web");

    if let Some(ref pk_b64) = req.ecdh_public_b64 {
        crate::crypto::validate_p256_public_key_b64(pk_b64)
            .map_err(|_| AppError::BadRequest("invalid_ecdh_public_key"))?;
    }
    if let Some(ref pk_b64) = req.ecdsa_public_b64 {
        crate::crypto::validate_p256_public_key_b64(pk_b64)
            .map_err(|_| AppError::BadRequest("invalid_ecdsa_public_key"))?;
    }
    let ecdh = decode_b64_opt(req.ecdh_public_b64.as_deref(), 65);
    let ecdsa = decode_b64_opt(req.ecdsa_public_b64.as_deref(), 65);
    let fp = decode_b64_opt(req.key_fingerprint_b64.as_deref(), 32);

    // Check if this client_device_id already exists for user.
    let existing = sqlx::query(
        "SELECT id, trust_state, ecdh_public FROM devices WHERE user_id = $1 AND client_device_id = $2",
    )
    .bind(user.id)
    .bind(cid)
    .fetch_optional(&state.db)
    .await?;

    let other_trusted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM devices WHERE user_id = $1 AND trust_state = 'trusted' AND client_device_id <> $2",
    )
    .bind(user.id)
    .bind(cid)
    .fetch_one(&state.db)
    .await?;

    let has_account_keys: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM device_keys WHERE user_id = $1)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let require_approval: bool = sqlx::query_scalar(
        "SELECT require_device_approval FROM device_storage_policies WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    let needs_key_sync = ecdsa.is_none() || fp.is_none();
    let mut ecdh_changed = false;

    let (device_id, trust_state) = if let Some(row) = existing {
        let did: Uuid = row.try_get("id")?;
        let state_str: String = row.try_get("trust_state")?;
        let prev_ecdh: Option<Vec<u8>> = row.try_get("ecdh_public").ok().flatten();
        if state_str == "revoked" {
            return Err(AppError::Forbidden("device_revoked"));
        }
        if let Some(ref next_ecdh) = ecdh {
            ecdh_changed = prev_ecdh.as_deref() != Some(next_ecdh.as_slice());
        }
        sqlx::query(
            r#"
            UPDATE devices
            SET last_seen_at = now(),
                platform = $2,
                device_name = COALESCE($3, device_name),
                ecdh_public = COALESCE($4, ecdh_public),
                ecdsa_public = COALESCE($5, ecdsa_public),
                key_fingerprint = COALESCE($6, key_fingerprint)
            WHERE id = $1
            "#,
        )
        .bind(did)
        .bind(platform)
        .bind(req.device_name.as_deref())
        .bind(ecdh.as_deref())
        .bind(ecdsa.as_deref())
        .bind(fp.as_deref())
        .execute(&state.db)
        .await?;
        (did, state_str)
    } else {
        let did = Uuid::new_v4();
        // When other trusted devices already exist, require explicit approval or cryptographic
        // key-ownership proof (submitted immediately via `/api/devices/recover-lost` if keys were unwrapped).
        let initial_trust = if other_trusted_count > 0 && (require_approval || needs_key_sync || has_account_keys) {
            "pending"
        } else {
            "trusted"
        };
        let label = req
            .device_label
            .clone()
            .or_else(|| req.device_name.clone())
            .unwrap_or_else(|| format!("{platform} device"));
        sqlx::query(
            r#"
            INSERT INTO devices
                (id, user_id, client_device_id, device_name, device_label, platform,
                 ecdh_public, ecdsa_public, key_fingerprint, trust_state, approved_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CASE WHEN $10 = 'trusted' THEN now() ELSE NULL END)
            "#,
        )
        .bind(did)
        .bind(user.id)
        .bind(cid)
        .bind(req.device_name.as_deref())
        .bind(&label)
        .bind(platform)
        .bind(ecdh.as_deref())
        .bind(ecdsa.as_deref())
        .bind(fp.as_deref())
        .bind(initial_trust)
        .execute(&state.db)
        .await?;
        (did, initial_trust.to_string())
    };

    // Bind current session cleanly to this device_id.
    if let Some(sid) = user.session_id {
        sqlx::query("UPDATE sessions SET device_id = $1 WHERE id = $2 AND user_id = $3")
            .bind(device_id)
            .bind(sid)
            .bind(user.id)
            .execute(&state.db)
            .await?;
    }

    let mut challenge_id: Option<Uuid> = None;
    let mut verification_code: Option<String> = None;
    let should_request_sync = trust_state == "pending"
        || (needs_key_sync
            && has_account_keys
            && other_trusted_count > 0
            && req.ecdh_public_b64.is_some());

    if should_request_sync {
        if ecdh_changed {
            let _ = sqlx::query(
                "UPDATE device_login_requests SET status = 'expired' WHERE requesting_device_id = $1 AND status = 'pending'",
            )
            .bind(device_id)
            .execute(&state.db)
            .await;
        }
        let existing_req = sqlx::query(
            "SELECT id, verification_code FROM device_login_requests WHERE requesting_device_id = $1 AND status = 'pending' AND expires_at > now() ORDER BY created_at DESC LIMIT 1",
        )
        .bind(device_id)
        .fetch_optional(&state.db)
        .await?;
        let (rid, code) = if let Some(er) = existing_req {
            (er.try_get::<Uuid, _>("id")?, er.try_get::<String, _>("verification_code")?)
        } else {
            let rid = Uuid::new_v4();
            let code = generate_6_digit_code();
            let ua = headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.chars().take(200).collect::<String>());
            sqlx::query(
                r#"
                INSERT INTO device_login_requests
                    (id, user_id, requesting_device_id, requesting_session_id, verification_code, user_agent, status, expires_at)
                VALUES ($1, $2, $3, $4, $5, $6, 'pending', now() + interval '15 minutes')
                "#,
            )
            .bind(rid)
            .bind(user.id)
            .bind(device_id)
            .bind(user.session_id)
            .bind(&code)
            .bind(&ua)
            .execute(&state.db)
            .await?;
            (rid, code)
        };
        challenge_id = Some(rid);
        verification_code = Some(code.clone());

        // Always notify online trusted devices (including on page reload) so auto-sync fires immediately.
        state
            .ws_hub
            .send_to_user(
                &user.id,
                WsEvent::DeviceApprovalRequest(serde_json::json!({
                    "request_id": rid,
                    "requesting_device_id": device_id,
                    "client_device_id": cid,
                    "device_name": req.device_name.as_deref().unwrap_or(platform),
                    "platform": platform,
                    "verification_code": code,
                    "ecdh_public_b64": req.ecdh_public_b64,
                    "auto_approve": !require_approval,
                    "needs_key_sync": needs_key_sync,
                })),
            )
            .await;
    }

    Ok(Json(serde_json::json!({
        "device_id": device_id,
        "trust_state": trust_state,
        "requires_approval": trust_state == "pending",
        "auto_approve": !require_approval,
        "needs_key_sync": needs_key_sync,
        "request_id": challenge_id,
        "verification_code": verification_code,
    })))
}

async fn handle_current_device_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let Some(did) = user.device_id else {
        return Ok(Json(serde_json::json!({
            "registered": false,
            "trust_state": "unregistered",
        })));
    };
    let row = sqlx::query(
        "SELECT trust_state, platform, device_label FROM devices WHERE id = $1 AND user_id = $2",
    )
    .bind(did)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("device_not_found"))?;
    let trust_state: String = row.try_get("trust_state")?;
    let pending_req = sqlx::query(
        "SELECT id, status, verification_code, wrapped_sync_key, expires_at FROM device_login_requests WHERE requesting_device_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(did)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "registered": true,
        "device_id": did,
        "trust_state": trust_state,
        "platform": row.try_get::<String, _>("platform")?,
        "device_label": row.try_get::<Option<String>, _>("device_label")?,
        "latest_request": pending_req.map(|r| serde_json::json!({
            "id": r.try_get::<Uuid, _>("id").ok(),
            "status": r.try_get::<String, _>("status").ok(),
            "verification_code": r.try_get::<String, _>("verification_code").ok(),
            "wrapped_sync_key": r.try_get::<Option<String>, _>("wrapped_sync_key").ok().flatten(),
            "expires_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("expires_at").ok(),
        })),
    })))
}

async fn handle_list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let devs = sqlx::query(
        r#"
        SELECT id, client_device_id, device_name, device_label, platform, trust_state, last_seen_at, created_at,
               ecdh_public IS NOT NULL AS has_key
        FROM devices
        WHERE user_id = $1 AND trust_state <> 'revoked'
        ORDER BY last_seen_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let devices_json: Vec<serde_json::Value> = devs
        .into_iter()
        .map(|d| {
            let id: Uuid = d.try_get("id").unwrap_or_default();
            serde_json::json!({
                "id": id,
                "is_current": user.device_id == Some(id),
                "client_device_id": d.try_get::<Option<String>, _>("client_device_id").ok().flatten(),
                "device_name": d.try_get::<Option<String>, _>("device_name").ok().flatten(),
                "device_label": d.try_get::<Option<String>, _>("device_label").ok().flatten(),
                "platform": d.try_get::<String, _>("platform").unwrap_or_default(),
                "trust_state": d.try_get::<String, _>("trust_state").unwrap_or_default(),
                "has_key": d.try_get::<bool, _>("has_key").unwrap_or(false),
                "last_seen_at": d.try_get::<chrono::DateTime<chrono::Utc>, _>("last_seen_at").ok(),
                "created_at": d.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    let caller_is_trusted = if let Some(did) = user.device_id {
        sqlx::query_scalar::<_, String>(
            "SELECT trust_state FROM devices WHERE id = $1 AND user_id = $2",
        )
        .bind(did)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .as_deref()
            == Some("trusted")
    } else {
        false
    };

    let require_approval: bool = sqlx::query_scalar(
        "SELECT require_device_approval FROM device_storage_policies WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    let reqs_json: Vec<serde_json::Value> = if caller_is_trusted {
        let reqs = sqlx::query(
            r#"
            SELECT r.id, r.requesting_device_id, r.verification_code, r.user_agent, r.status, r.expires_at, r.created_at,
                   d.device_label, d.platform, d.ecdh_public
            FROM device_login_requests r
            JOIN devices d ON d.id = r.requesting_device_id
            WHERE r.user_id = $1
              AND r.status = 'pending'
              AND r.expires_at > now()
              AND ($2::uuid IS NULL OR r.requesting_device_id <> $2)
            ORDER BY r.created_at DESC
            "#,
        )
        .bind(user.id)
        .bind(user.device_id)
        .fetch_all(&state.db)
        .await?;

        reqs.into_iter()
            .map(|r| {
                let ecdh_b64 = r
                    .try_get::<Option<Vec<u8>>, _>("ecdh_public")
                    .ok()
                    .flatten()
                    .map(|b| URL_SAFE_NO_PAD.encode(b));
                serde_json::json!({
                    "id": r.try_get::<Uuid, _>("id").ok(),
                    "requesting_device_id": r.try_get::<Uuid, _>("requesting_device_id").ok(),
                    "device_label": r.try_get::<Option<String>, _>("device_label").ok().flatten(),
                    "platform": r.try_get::<String, _>("platform").unwrap_or_default(),
                    "verification_code": r.try_get::<String, _>("verification_code").unwrap_or_default(),
                    "ecdh_public_b64": ecdh_b64,
                    "user_agent": r.try_get::<Option<String>, _>("user_agent").ok().flatten(),
                    "expires_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("expires_at").ok(),
                    "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                })
            })
            .collect()
    } else {
        // Untrusted/pending sessions must never harvest verification codes or pending requests of other sessions.
        Vec::new()
    };

    Ok(Json(serde_json::json!({
        "devices": devices_json,
        "pending_requests": reqs_json,
        "require_device_approval": require_approval,
    })))
}

#[derive(Deserialize, Default)]
struct ApproveDeviceReq {
    pub verification_code: Option<String>,
    pub wrapped_sync_key: Option<String>,
}

async fn require_trusted_device(pool: &PgPool, user_id: Uuid, device_id: Option<Uuid>) -> AppResult<Uuid> {
    let did = device_id.ok_or(AppError::Forbidden("trusted_device_required"))?;
    let trust: Option<String> = sqlx::query_scalar(
        "SELECT trust_state FROM devices WHERE id = $1 AND user_id = $2",
    )
    .bind(did)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if trust.as_deref() == Some("trusted") {
        Ok(did)
    } else {
        Err(AppError::Forbidden("only_trusted_device_may_approve"))
    }
}

async fn handle_approve_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    body: Option<Json<ApproveDeviceReq>>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let approver_did = require_trusted_device(&state.db, user.id, user.device_id).await?;
    let p = body.map(|j| j.0).unwrap_or_default();

    let req_row = sqlx::query(
        "SELECT requesting_device_id, verification_code, status, expires_at FROM device_login_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(request_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("approval_request_not_found"))?;

    let status: String = req_row.try_get("status")?;
    let expected_code: String = req_row.try_get("verification_code")?;
    let req_did: Uuid = req_row.try_get("requesting_device_id")?;
    let exp: chrono::DateTime<chrono::Utc> = req_row.try_get("expires_at")?;
    if status != "pending" || exp <= chrono::Utc::now() {
        return Err(AppError::BadRequest("approval_request_expired_or_resolved"));
    }
    if req_did == approver_did {
        return Err(AppError::BadRequest("device_cannot_self_approve"));
    }
    let supplied_code = p
        .verification_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(AppError::BadRequest("verification_code_required"))?;
    if !bool::from(supplied_code.as_bytes().ct_eq(expected_code.as_bytes())) {
        return Err(AppError::BadRequest("invalid_verification_code"));
    }
    if let Some(ref wsk) = p.wrapped_sync_key
        && wsk.len() > 32768
    {
        return Err(AppError::BadRequest("wrapped_sync_key_too_large"));
    }

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE device_login_requests SET status = 'approved', wrapped_sync_key = $2, approver_device_id = $3, decided_at = now() WHERE id = $1",
    )
    .bind(request_id)
    .bind(p.wrapped_sync_key.as_deref())
    .bind(approver_did)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE devices SET trust_state = 'trusted', approved_by_device = $2, approved_at = now() WHERE id = $1",
    )
    .bind(req_did)
    .bind(approver_did)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    crate::security::log_event(&state, user.id, "device_approved", &headers).await;

    state
        .ws_hub
        .send_to_user(
            &user.id,
            WsEvent::DeviceApprovalResolved(serde_json::json!({
                "request_id": request_id,
                "requesting_device_id": req_did,
                "approved": true,
                "wrapped_sync_key": p.wrapped_sync_key,
            })),
        )
        .await;

    Ok(Json(serde_json::json!({ "approved": true, "device_id": req_did })))
}

async fn handle_reject_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let approver_did = require_trusted_device(&state.db, user.id, user.device_id).await?;

    let req_row = sqlx::query(
        "SELECT requesting_device_id, requesting_session_id, status FROM device_login_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(request_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("approval_request_not_found"))?;

    let req_did: Uuid = req_row.try_get("requesting_device_id")?;
    let req_sid: Option<Uuid> = req_row.try_get("requesting_session_id")?;
    if req_row.try_get::<String, _>("status")? != "pending" {
        return Err(AppError::BadRequest("already_resolved"));
    }

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE device_login_requests SET status = 'rejected', approver_device_id = $2, decided_at = now() WHERE id = $1",
    )
    .bind(request_id)
    .bind(approver_did)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE devices SET trust_state = 'revoked', revoked_at = now() WHERE id = $1")
        .bind(req_did)
        .execute(&mut *tx)
        .await?;
    if let Some(sid) = req_sid {
        sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
            .bind(sid)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM sessions WHERE device_id = $1 AND user_id = $2")
        .bind(req_did)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    state
        .ws_hub
        .send_to_user(
            &user.id,
            WsEvent::DeviceApprovalResolved(serde_json::json!({
                "request_id": request_id,
                "requesting_device_id": req_did,
                "approved": false,
            })),
        )
        .await;
    state
        .ws_hub
        .send_to_user(
            &user.id,
            WsEvent::DeviceRevoked(serde_json::json!({ "device_id": req_did })),
        )
        .await;
    state.ws_hub.disconnect_device(&user.id, req_did).await;
    crate::security::log_event(&state, user.id, "device_rejected", &headers).await;

    Ok(Json(serde_json::json!({ "rejected": true, "device_id": req_did })))
}

#[derive(Deserialize)]
struct RecoverLostReq {
    pub totp_or_backup_code: Option<String>,
    pub revoke_unreachable_devices: Option<bool>,
    /// Cryptographic proof of E2EE identity key ownership: 64-byte ECDSA signature (base64url)
    /// over `b"chatw-recover-device-v1:" || current_device_id.as_bytes()`.
    #[serde(default)]
    pub recovery_proof_b64: Option<String>,
    /// Legacy field kept for JSON backward compatibility; emergency bypass is no longer permitted when trusted devices exist.
    #[allow(dead_code)]
    #[serde(default)]
    pub confirm_emergency_reset: bool,
}

async fn handle_recover_lost_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RecoverLostReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let current_did = user.device_id.ok_or(AppError::BadRequest("device_not_registered"))?;

    let current_trust: Option<String> = sqlx::query_scalar(
        "SELECT trust_state FROM devices WHERE id = $1 AND user_id = $2",
    )
    .bind(current_did)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    if current_trust.as_deref() == Some("revoked") {
        return Err(AppError::Forbidden("device_revoked"));
    }

    // Require 2FA code if TOTP is enabled for this account.
    crate::security::require_second_factor(
        &state,
        user.id,
        req.totp_or_backup_code.as_deref(),
        &headers,
    )
    .await?;

    let other_trusted_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM devices WHERE user_id = $1 AND trust_state = 'trusted' AND id <> $2",
    )
    .bind(user.id)
    .bind(current_did)
    .fetch_all(&state.db)
    .await?;

    let totp_enabled: bool = sqlx::query_scalar(
        "SELECT totp_enabled_at IS NOT NULL FROM users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    // Prevent zero-credential self-promotion of a pending device when 2FA is disabled and other trusted devices exist.
    if !other_trusted_ids.is_empty() && current_trust.as_deref() != Some("trusted") && !totp_enabled {
        let mut proof_verified = false;
        if let Some(ref sig_b64) = req.recovery_proof_b64 {
            let sig_bytes = URL_SAFE_NO_PAD
                .decode(sig_b64)
                .map_err(|_| AppError::BadRequest("invalid_recovery_proof_encoding"))?;
            let id_ecdsa: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT ecdsa_public FROM device_keys WHERE user_id = $1",
            )
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
            if let Some(ecdsa_pub) = id_ecdsa {
                let vk = VerifyingKey::from_sec1_bytes(&ecdsa_pub)
                    .map_err(|_| AppError::BadRequest("invalid_identity_ecdsa_key"))?;
                let sig = Signature::from_slice(&sig_bytes)
                    .map_err(|_| AppError::BadRequest("invalid_recovery_proof_format"))?;
                let mut msg = Vec::with_capacity(24 + 16);
                msg.extend_from_slice(b"chatw-recover-device-v1:");
                msg.extend_from_slice(current_did.as_bytes());
                vk.verify(&msg, &sig)
                    .map_err(|_| AppError::Forbidden("invalid_recovery_proof_signature"))?;
                proof_verified = true;
            }
        }

        if !proof_verified {
            return Err(AppError::Forbidden("recovery_proof_or_2fa_required"));
        }
    }

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE devices SET trust_state = 'trusted', approved_at = now() WHERE id = $1 AND user_id = $2 AND trust_state <> 'revoked'",
    )
    .bind(current_did)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE device_login_requests SET status = 'approved', decided_at = now() WHERE requesting_device_id = $1 AND status = 'pending'",
    )
    .bind(current_did)
    .execute(&mut *tx)
    .await?;

    if req.revoke_unreachable_devices.unwrap_or(false) {
        sqlx::query(
            "UPDATE devices SET trust_state = 'revoked', revoked_at = now() WHERE user_id = $1 AND id <> $2",
        )
        .bind(user.id)
        .bind(current_did)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND (device_id IS NULL OR device_id <> $2)")
            .bind(user.id)
            .bind(current_did)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    if req.revoke_unreachable_devices.unwrap_or(false) {
        for revoked_did in other_trusted_ids {
            state
                .ws_hub
                .send_to_user(
                    &user.id,
                    WsEvent::DeviceRevoked(serde_json::json!({ "device_id": revoked_did })),
                )
                .await;
            state.ws_hub.disconnect_device(&user.id, revoked_did).await;
        }
    }
    crate::security::log_event(&state, user.id, "device_recovered", &headers).await;

    let backup = sqlx::query(
        "SELECT wrapped_keys_b64, kdf_salt_b64, kdf_iterations, kdf_alg, kdf_mem_kib, kdf_time_cost, kdf_parallelism, root_key_wrapped_b64, encode(key_fingerprint, 'base64') AS fp_b64 FROM user_key_backups WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "recovered": true,
        "device_id": current_did,
        "trust_state": "trusted",
        "key_backup": backup.map(|b| serde_json::json!({
            "wrapped_keys_b64": b.try_get::<String, _>("wrapped_keys_b64").ok(),
            "kdf_salt_b64": b.try_get::<String, _>("kdf_salt_b64").ok(),
            "kdf_iterations": b.try_get::<i32, _>("kdf_iterations").ok(),
            "kdf_alg": b.try_get::<String, _>("kdf_alg").ok(),
            "kdf_mem_kib": b.try_get::<i32, _>("kdf_mem_kib").ok(),
            "kdf_time_cost": b.try_get::<i32, _>("kdf_time_cost").ok(),
            "kdf_parallelism": b.try_get::<i32, _>("kdf_parallelism").ok(),
            "root_key_wrapped_b64": b.try_get::<Option<String>, _>("root_key_wrapped_b64").ok().flatten(),
        })),
        "honest_notice": "Server text messages, group and server history remain intact. Private chat files that have not been backed up will be checked from the other person's device if they are online."
    })))
}

async fn handle_revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    // Only a trusted device may revoke another device (prevents pending device from revoking trusted ones).
    let _trusted_did = require_trusted_device(&state.db, user.id, user.device_id).await?;

    let res = sqlx::query(
        "UPDATE devices SET trust_state = 'revoked', revoked_at = now() WHERE id = $1 AND user_id = $2",
    )
    .bind(device_id)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("device_not_found"));
    }

    sqlx::query("DELETE FROM sessions WHERE device_id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM media_device_locations WHERE device_id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    state
        .ws_hub
        .send_to_user(
            &user.id,
            WsEvent::DeviceRevoked(serde_json::json!({ "device_id": device_id })),
        )
        .await;
    state.ws_hub.disconnect_device(&user.id, device_id).await;
    crate::security::log_event(&state, user.id, "device_revoked", &headers).await;

    Ok(Json(serde_json::json!({ "revoked": true, "device_id": device_id })))
}

#[derive(Deserialize)]
struct PutKeyBackupReq {
    pub wrapped_keys_b64: String,
    pub kdf_salt_b64: String,
    pub kdf_iterations: Option<i32>,
    pub key_fingerprint_b64: String,
    #[serde(default)]
    pub kdf_alg: Option<String>,
    #[serde(default)]
    pub kdf_mem_kib: Option<i32>,
    #[serde(default)]
    pub kdf_time_cost: Option<i32>,
    #[serde(default)]
    pub kdf_parallelism: Option<i32>,
    #[serde(default)]
    pub root_key_wrapped_b64: Option<String>,
}

async fn handle_put_key_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PutKeyBackupReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    // Prevent an untrusted/pending device from overwriting the user's encrypted E2EE key backup.
    let has_any_trusted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM devices WHERE user_id = $1 AND trust_state = 'trusted')",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if has_any_trusted || user.device_id.is_some() {
        let _ = require_trusted_device(&state.db, user.id, user.device_id).await?;
    }

    let fp = decode_b64_opt(Some(&req.key_fingerprint_b64), 32)
        .ok_or(AppError::BadRequest("invalid_key_fingerprint"))?;
    let iters = req.kdf_iterations.unwrap_or(210_000).clamp(10_000, 2_000_000);
    let kdf_alg = req
        .kdf_alg
        .as_deref()
        .filter(|a| matches!(*a, "argon2id-v1" | "pbkdf2-sha256-v1"))
        .unwrap_or("argon2id-v1");
    let kdf_mem_kib = req.kdf_mem_kib.unwrap_or(19456).clamp(8192, 262144);
    let kdf_time_cost = req.kdf_time_cost.unwrap_or(2).clamp(1, 16);
    let kdf_parallelism = req.kdf_parallelism.unwrap_or(1).clamp(1, 8);

    if req.wrapped_keys_b64.len() > 32768 || req.kdf_salt_b64.len() > 256 {
        return Err(AppError::BadRequest("invalid_backup_payload"));
    }
    if let Some(ref rk) = req.root_key_wrapped_b64
        && rk.len() > 4096
    {
        return Err(AppError::BadRequest("invalid_root_key_payload"));
    }

    sqlx::query(
        r#"
        INSERT INTO user_key_backups
            (user_id, wrapped_keys_b64, kdf_salt_b64, kdf_iterations, key_fingerprint,
             kdf_alg, kdf_mem_kib, kdf_time_cost, kdf_parallelism, root_key_wrapped_b64, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
        ON CONFLICT (user_id) DO UPDATE SET
            wrapped_keys_b64 = EXCLUDED.wrapped_keys_b64,
            kdf_salt_b64 = EXCLUDED.kdf_salt_b64,
            kdf_iterations = EXCLUDED.kdf_iterations,
            key_fingerprint = EXCLUDED.key_fingerprint,
            kdf_alg = EXCLUDED.kdf_alg,
            kdf_mem_kib = EXCLUDED.kdf_mem_kib,
            kdf_time_cost = EXCLUDED.kdf_time_cost,
            kdf_parallelism = EXCLUDED.kdf_parallelism,
            root_key_wrapped_b64 = EXCLUDED.root_key_wrapped_b64,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(&req.wrapped_keys_b64)
    .bind(&req.kdf_salt_b64)
    .bind(iters)
    .bind(&fp[..])
    .bind(kdf_alg)
    .bind(kdf_mem_kib)
    .bind(kdf_time_cost)
    .bind(kdf_parallelism)
    .bind(req.root_key_wrapped_b64.as_deref())
    .execute(&state.db)
    .await?;

    crate::security::log_event(&state, user.id, "e2ee_key_backup_saved", &headers).await;
    Ok(Json(serde_json::json!({ "saved": true, "kdf_alg": kdf_alg })))
}

async fn handle_get_key_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT wrapped_keys_b64, kdf_salt_b64, kdf_iterations, kdf_alg, kdf_mem_kib, kdf_time_cost, kdf_parallelism, root_key_wrapped_b64, updated_at FROM user_key_backups WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "exists": row.is_some(),
        "backup": row.map(|r| serde_json::json!({
            "wrapped_keys_b64": r.try_get::<String, _>("wrapped_keys_b64").ok(),
            "kdf_salt_b64": r.try_get::<String, _>("kdf_salt_b64").ok(),
            "kdf_iterations": r.try_get::<i32, _>("kdf_iterations").ok(),
            "kdf_alg": r.try_get::<String, _>("kdf_alg").ok(),
            "kdf_mem_kib": r.try_get::<i32, _>("kdf_mem_kib").ok(),
            "kdf_time_cost": r.try_get::<i32, _>("kdf_time_cost").ok(),
            "kdf_parallelism": r.try_get::<i32, _>("kdf_parallelism").ok(),
            "root_key_wrapped_b64": r.try_get::<Option<String>, _>("root_key_wrapped_b64").ok().flatten(),
            "updated_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok(),
        })),
    })))
}

// -----------------------------------------------------------------------------
// Incremental Cursor Delta Sync (Phase A / Phase B)
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct EventsSyncQuery {
    pub after_event_id: Option<i64>,
    pub limit: Option<i64>,
}

/// Spec 165 (Section 39): Ordered, deduplicated HTTP gap-recovery endpoint after WebSocket
/// disconnect or Valkey restart. Returns durable events strictly newer than `after_event_id`.
async fn handle_sync_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<EventsSyncQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let after_id = q.after_event_id.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(200).clamp(1, 500);

    let rows = sqlx::query(
        r#"
        SELECT event_id, event_type, scope_type, scope_id, message_id, payload, created_at
        FROM ws_event_log
        WHERE user_id = $1 AND event_id > $2
        ORDER BY event_id ASC
        LIMIT $3
        "#,
    )
    .bind(user.id)
    .bind(after_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = (rows.len() as i64) > limit;
    let mut latest_event_id = after_id;
    let mut events = Vec::with_capacity(rows.len().min(limit as usize));
    for r in rows.into_iter().take(limit as usize) {
        let eid: i64 = r.try_get("event_id")?;
        latest_event_id = latest_event_id.max(eid);
        let ts: chrono::DateTime<chrono::Utc> =
            r.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now());
        events.push(serde_json::json!({
            "event_id": eid,
            "event_type": r.try_get::<String, _>("event_type").unwrap_or_default(),
            "scope_type": r.try_get::<Option<String>, _>("scope_type").ok().flatten(),
            "scope_id": r.try_get::<Option<String>, _>("scope_id").ok().flatten(),
            "message_id": r.try_get::<Option<Uuid>, _>("message_id").ok().flatten(),
            "payload": r.try_get::<serde_json::Value, _>("payload").unwrap_or(serde_json::Value::Null),
            "created_at_ms": ts.timestamp_millis(),
        }));
    }

    Ok(Json(serde_json::json!({
        "after_event_id": after_id,
        "latest_event_id": latest_event_id,
        "has_more": has_more,
        "events": events,
    })))
}

#[derive(Deserialize)]
struct DeltaQuery {
    pub dm_since: Option<i64>,
    pub group_since: Option<i64>,
    pub channel_since: Option<i64>,
    pub limit: Option<i64>,
}

async fn handle_delta_sync(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<DeltaQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;

    if let Some(did) = user.device_id {
        let trust: Option<String> = sqlx::query_scalar(
            "SELECT trust_state FROM devices WHERE id = $1 AND user_id = $2",
        )
        .bind(did)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?;
        if trust.as_deref() == Some("pending") {
            return Err(AppError::Forbidden("device_approval_required"));
        }
        if trust.as_deref() == Some("revoked") {
            return Err(AppError::Unauthorized);
        }
    }

    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let dm_since = q.dm_since.unwrap_or(0);
    let group_since = q.group_since.unwrap_or(0);
    let channel_since = q.channel_since.unwrap_or(0);

    // 1. DM incremental deltas (emit cleared tombstones for deleted_for_all so offline devices remove them)
    let dm_rows = sqlx::query(
        r#"
        SELECT id, seq, sender_id, receiver_id, reply_to_id, is_pinned, is_edited, deleted_for_all, created_at,
               CASE
                   WHEN deleted_for_all THEN ''::bytea
                   WHEN sender_id = $1 THEN sender_envelope
                   ELSE recipient_envelope
               END AS env
        FROM messages
        WHERE (sender_id = $1 OR receiver_id = $1)
          AND seq > $2
        ORDER BY seq ASC
        LIMIT $3
        "#,
    )
    .bind(user.id)
    .bind(dm_since)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let mut max_dm_seq = dm_since;
    let dm_messages: Vec<serde_json::Value> = dm_rows
        .into_iter()
        .map(|r| {
            let seq: i64 = r.try_get("seq").unwrap_or(0);
            max_dm_seq = max_dm_seq.max(seq);
            let env: Vec<u8> = r.try_get("env").unwrap_or_default();
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok(),
                "seq": seq,
                "sender_id": r.try_get::<Uuid, _>("sender_id").ok(),
                "recipient_id": r.try_get::<Uuid, _>("receiver_id").ok(),
                "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten(),
                "is_pinned": r.try_get::<bool, _>("is_pinned").unwrap_or(false),
                "is_edited": r.try_get::<bool, _>("is_edited").unwrap_or(false),
                "deleted_for_all": r.try_get::<bool, _>("deleted_for_all").unwrap_or(false),
                "envelope": base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, env),
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    // 2. Group incremental deltas
    let group_rows = sqlx::query(
        r#"
        SELECT gm.id, gm.seq, gm.group_id, gm.channel_id, gm.sender_id, gm.content, gm.reply_to_id, gm.is_pinned, gm.is_edited, gm.created_at
        FROM group_messages gm
        JOIN group_members m ON m.group_id = gm.group_id AND m.user_id = $1
        WHERE gm.seq > $2
        ORDER BY gm.seq ASC
        LIMIT $3
        "#,
    )
    .bind(user.id)
    .bind(group_since)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let mut max_group_seq = group_since;
    let group_messages: Vec<serde_json::Value> = group_rows
        .into_iter()
        .map(|r| {
            let seq: i64 = r.try_get("seq").unwrap_or(0);
            max_group_seq = max_group_seq.max(seq);
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok(),
                "seq": seq,
                "group_id": r.try_get::<Uuid, _>("group_id").ok(),
                "channel_id": r.try_get::<Option<Uuid>, _>("channel_id").ok().flatten(),
                "sender_id": r.try_get::<Uuid, _>("sender_id").ok(),
                "content": r.try_get::<String, _>("content").unwrap_or_default(),
                "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten(),
                "is_pinned": r.try_get::<bool, _>("is_pinned").unwrap_or(false),
                "is_edited": r.try_get::<bool, _>("is_edited").unwrap_or(false),
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    // 3. Server channel incremental deltas
    let channel_rows = sqlx::query(
        r#"
        SELECT cm.id, cm.seq, cm.channel_id, cm.sender_id, cm.content, cm.reply_to_id, cm.is_pinned, cm.is_edited, cm.created_at
        FROM channel_messages cm
        JOIN channels c ON c.id = cm.channel_id
        JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $1
        WHERE cm.seq > $2
        ORDER BY cm.seq ASC
        LIMIT $3
        "#,
    )
    .bind(user.id)
    .bind(channel_since)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let mut max_channel_seq = channel_since;
    let mut chan_perm_cache: std::collections::HashMap<Uuid, bool> = std::collections::HashMap::new();
    let mut channel_messages: Vec<serde_json::Value> = Vec::with_capacity(channel_rows.len());
    for r in channel_rows {
        let seq: i64 = r.try_get("seq").unwrap_or(0);
        max_channel_seq = max_channel_seq.max(seq);
        let Ok(cid) = r.try_get::<Uuid, _>("channel_id") else {
            continue;
        };
        let allowed = if let Some(&cached) = chan_perm_cache.get(&cid) {
            cached
        } else {
            let ok = crate::storage::permissions::authorize_channel_access(
                &state.db,
                user.id,
                cid,
                crate::models::permissions::VIEW_CHANNELS,
            )
            .await
            .is_ok();
            chan_perm_cache.insert(cid, ok);
            ok
        };
        if !allowed {
            continue;
        }
        channel_messages.push(serde_json::json!({
            "id": r.try_get::<Uuid, _>("id").ok(),
            "seq": seq,
            "channel_id": cid,
            "sender_id": r.try_get::<Uuid, _>("sender_id").ok(),
            "content": r.try_get::<String, _>("content").unwrap_or_default(),
            "reply_to_id": r.try_get::<Option<Uuid>, _>("reply_to_id").ok().flatten(),
            "is_pinned": r.try_get::<bool, _>("is_pinned").unwrap_or(false),
            "is_edited": r.try_get::<bool, _>("is_edited").unwrap_or(false),
            "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
        }));
    }

    if let Some(did) = user.device_id {
        let _ = sqlx::query(
            r#"
            INSERT INTO device_sync_cursors (device_id, user_id, dm_seq, group_seq, channel_seq, phase_a_done, last_sync_at)
            VALUES ($1, $2, $3, $4, $5, TRUE, now())
            ON CONFLICT (device_id) DO UPDATE SET
                dm_seq = GREATEST(device_sync_cursors.dm_seq, EXCLUDED.dm_seq),
                group_seq = GREATEST(device_sync_cursors.group_seq, EXCLUDED.group_seq),
                channel_seq = GREATEST(device_sync_cursors.channel_seq, EXCLUDED.channel_seq),
                phase_a_done = TRUE,
                last_sync_at = now()
            "#,
        )
        .bind(did)
        .bind(user.id)
        .bind(max_dm_seq)
        .bind(max_group_seq)
        .bind(max_channel_seq)
        .execute(&state.db)
        .await;
    }

    let has_more = (dm_messages.len() as i64) >= limit
        || (group_messages.len() as i64) >= limit
        || (channel_messages.len() as i64) >= limit;

    Ok(Json(serde_json::json!({
        "phase": "a_delta",
        "has_more": has_more,
        "dm_messages": dm_messages,
        "group_messages": group_messages,
        "channel_messages": channel_messages,
        "next_cursors": {
            "dm_seq": max_dm_seq,
            "group_seq": max_group_seq,
            "channel_seq": max_channel_seq,
        }
    })))
}

#[derive(Deserialize)]
struct UpdatePhaseReq {
    pub phase_a_done: Option<bool>,
    pub phase_b_done: Option<bool>,
}

async fn handle_update_phase_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdatePhaseReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    if let Some(did) = user.device_id {
        sqlx::query(
            r#"
            INSERT INTO device_sync_cursors (device_id, user_id, phase_a_done, phase_b_done, last_sync_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (device_id) DO UPDATE SET
                phase_a_done = COALESCE($3, device_sync_cursors.phase_a_done),
                phase_b_done = COALESCE($4, device_sync_cursors.phase_b_done),
                last_sync_at = now()
            "#,
        )
        .bind(did)
        .bind(user.id)
        .bind(req.phase_a_done.unwrap_or(true))
        .bind(req.phase_b_done.unwrap_or(false))
        .execute(&state.db)
        .await?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// -----------------------------------------------------------------------------
// Two-Way DM Peer Recovery Orchestrator
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateRecoveryReq {
    pub ref_id: Uuid,
    pub preferred_transport: Option<String>,
}

async fn handle_create_recovery_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateRecoveryReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let r = sqlx::query(
        "SELECT owner_id, scope_type, scope_id, deleted_for_all, object_id, availability_state FROM media_references WHERE id = $1",
    )
    .bind(req.ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("reference_not_found"))?;

    let owner_id: Uuid = r.try_get("owner_id")?;
    let scope_type: String = r.try_get("scope_type")?;
    let scope_id: Option<Uuid> = r.try_get("scope_id")?;
    let deleted_for_all: bool = r.try_get("deleted_for_all")?;
    let server_obj: Option<Uuid> = r.try_get("object_id")?;

    crate::storage::permissions::authorize_read(&state.db, user.id, &scope_type, scope_id, owner_id).await?;
    if deleted_for_all {
        return Err(AppError::BadRequest("media_deleted_for_everyone"));
    }

    // If server already holds a live object (e.g. group/channel/backup/live relay), direct restore is possible immediately.
    if server_obj.is_some() {
        return Ok(Json(serde_json::json!({
            "source": "server_object",
            "status": "ready_for_download",
            "download_url": format!("/api/attachments/{}", req.ref_id),
        })));
    }

    // Step 1: Check same-account other trusted devices first!
    let own_other_device: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT mdl.device_id
        FROM media_device_locations mdl
        JOIN devices d ON d.id = mdl.device_id
        WHERE mdl.ref_id = $1 AND mdl.user_id = $2 AND d.trust_state = 'trusted'
          AND ($3::uuid IS NULL OR mdl.device_id <> $3)
          AND mdl.local_tier <> 'server_relay'
        ORDER BY mdl.last_verified_at DESC LIMIT 1
        "#,
    )
    .bind(req.ref_id)
    .bind(user.id)
    .bind(user.device_id)
    .fetch_optional(&state.db)
    .await?;

    // Determine DM peer user_id (either the other person or self if same-account device sync).
    let peer_user_id: Option<Uuid> = if own_other_device.is_some() {
        Some(user.id)
    } else if scope_type == "dm" {
        if user.id != owner_id { Some(owner_id) } else { scope_id.filter(|&pid| pid != user.id) }
    } else {
        None
    };

    let Some(target_user_id) = peer_user_id else {
        let _ = sqlx::query(
            "UPDATE media_references SET availability_state = 'no_remaining_copies', updated_at = now() WHERE id = $1",
        )
        .bind(req.ref_id)
        .execute(&state.db)
        .await;
        return Ok(Json(serde_json::json!({
            "status": "no_remaining_copies",
            "honest_reason": "There are no valid copies left on the server or other device."
        })));
    };

    // If requesting from the DM peer (another user), strictly check per-DM allow_peer_recovery preference!
    if target_user_id != user.id {
        let peer_allows: bool = sqlx::query_scalar(
            "SELECT allow_peer_recovery FROM conversation_prefs WHERE user_id = $1 AND conv_type = 'dm' AND conv_id = $2",
        )
        .bind(target_user_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(true);

        if !peer_allows {
            let denied_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO recovery_requests
                    (id, ref_id, requester_user_id, requester_device_id, target_user_id, transport_mode, status, reason, expires_at)
                VALUES ($1, $2, $3, $4, $5, 'webrtc_p2p', 'denied', 'peer_recovery_disabled', now() + interval '7 days')
                "#,
            )
            .bind(denied_id)
            .bind(req.ref_id)
            .bind(user.id)
            .bind(user.device_id)
            .bind(target_user_id)
            .execute(&state.db)
            .await?;

            sqlx::query("UPDATE media_references SET availability_state = 'denied_by_peer', updated_at = now() WHERE id = $1")
                .bind(req.ref_id)
                .execute(&state.db)
                .await?;

            return Ok(Json(serde_json::json!({
                "request_id": denied_id,
                "status": "denied",
                "reason": "peer_recovery_disabled",
            })));
        }
    }

    // Check if target user has ANY verified device copy reported (if locations exist & count == 0 AND target has registered devices that didn't keep it -> check honest!).
    let target_has_copy: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM media_device_locations WHERE ref_id = $1 AND user_id = $2 AND local_tier <> 'server_relay')",
    )
    .bind(req.ref_id)
    .bind(target_user_id)
    .fetch_one(&state.db)
    .await?;

    let target_online = state.ws_hub.online_user_ids().await.contains(&target_user_id);
    let total_copies_anywhere: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM media_device_locations WHERE ref_id = $1 AND local_tier <> 'server_relay'",
    )
    .bind(req.ref_id)
    .fetch_one(&state.db)
    .await?;

    // If neither user has a copy recorded AND target is currently offline/lost -> queue cleanly or honestly state unknown vs no copies.
    let mode = if own_other_device.is_some() {
        "device_sync"
    } else {
        req.preferred_transport
            .as_deref()
            .filter(|m| matches!(*m, "webrtc_p2p" | "encrypted_relay"))
            .unwrap_or("webrtc_p2p")
    };

    let (status, avail) = if !target_has_copy && total_copies_anywhere == 0 && !target_online {
        ("no_remaining_copies", "no_remaining_copies")
    } else if target_online {
        ("pending", "recovery_pending")
    } else {
        ("queued_offline_peer", "queued_offline_peer")
    };

    let rec_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO recovery_requests
            (id, ref_id, requester_user_id, requester_device_id, target_user_id, target_device_id, transport_mode, status, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now() + interval '7 days')
        "#,
    )
    .bind(rec_id)
    .bind(req.ref_id)
    .bind(user.id)
    .bind(user.device_id)
    .bind(target_user_id)
    .bind(own_other_device)
    .bind(mode)
    .bind(status)
    .execute(&state.db)
    .await?;

    sqlx::query("UPDATE media_references SET availability_state = $2, updated_at = now() WHERE id = $1")
        .bind(req.ref_id)
        .bind(avail)
        .execute(&state.db)
        .await?;

    if target_online && status == "pending" {
        state
            .ws_hub
            .send_to_user(
                &target_user_id,
                WsEvent::RecoveryRequestIncoming(serde_json::json!({
                    "request_id": rec_id,
                    "ref_id": req.ref_id,
                    "requester_user_id": user.id,
                    "transport_mode": mode,
                })),
            )
            .await;
    }

    Ok(Json(serde_json::json!({
        "request_id": rec_id,
        "status": status,
        "availability_state": avail,
        "transport_mode": mode,
        "target_online": target_online,
        "target_has_verified_copy": target_has_copy,
    })))
}

async fn handle_list_recovery_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT rr.id, rr.ref_id, rr.requester_user_id, rr.target_user_id, rr.transport_mode,
               rr.status, rr.relay_ref_id, rr.reason, rr.expires_at, rr.created_at,
               mr.original_filename, mr.mime_type, mr.byte_size
        FROM recovery_requests rr
        JOIN media_references mr ON mr.id = rr.ref_id
        WHERE rr.requester_user_id = $1 OR rr.target_user_id = $1
        ORDER BY rr.created_at DESC
        LIMIT 100
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok(),
                "ref_id": r.try_get::<Uuid, _>("ref_id").ok(),
                "requester_user_id": r.try_get::<Uuid, _>("requester_user_id").ok(),
                "target_user_id": r.try_get::<Uuid, _>("target_user_id").ok(),
                "is_incoming": r.try_get::<Uuid, _>("target_user_id").ok() == Some(user.id),
                "transport_mode": r.try_get::<String, _>("transport_mode").unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "relay_ref_id": r.try_get::<Option<Uuid>, _>("relay_ref_id").ok().flatten(),
                "reason": r.try_get::<Option<String>, _>("reason").ok().flatten(),
                "file_name": r.try_get::<String, _>("original_filename").unwrap_or_default(),
                "mime_type": r.try_get::<String, _>("mime_type").unwrap_or_default(),
                "byte_size": r.try_get::<i64, _>("byte_size").unwrap_or(0),
                "expires_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("expires_at").ok(),
                "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "requests": items })))
}

#[derive(Deserialize)]
struct RespondRecoveryReq {
    pub action: String, // "accept" | "deny" | "no_copy"
    pub transport_mode: Option<String>,
    pub relay_ref_id: Option<Uuid>,
    pub reason: Option<String>,
}

async fn handle_respond_recovery_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    Json(req): Json<RespondRecoveryReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT ref_id, requester_user_id, target_user_id FROM recovery_requests WHERE id = $1",
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("recovery_request_not_found"))?;

    let target_uid: Uuid = row.try_get("target_user_id")?;
    let req_uid: Uuid = row.try_get("requester_user_id")?;
    let ref_id: Uuid = row.try_get("ref_id")?;
    if target_uid != user.id {
        return Err(AppError::Forbidden("not_recovery_target"));
    }

    let (new_status, new_avail) = match req.action.as_str() {
        "accept" => ("transferring", "recovery_pending"),
        "no_copy" => ("no_remaining_copies", "no_remaining_copies"),
        "deny" => ("denied", "denied_by_peer"),
        _ => return Err(AppError::BadRequest("invalid_recovery_action")),
    };

    if let Some(relay_ref_id) = req.relay_ref_id {
        let owns_relay: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM media_references WHERE id = $1 AND owner_id = $2)",
        )
        .bind(relay_ref_id)
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
        if !owns_relay {
            return Err(AppError::Forbidden("invalid_relay_ref_id"));
        }
    }

    let mode = req
        .transport_mode
        .as_deref()
        .filter(|m| matches!(*m, "webrtc_p2p" | "encrypted_relay" | "device_sync"))
        .unwrap_or(if req.relay_ref_id.is_some() { "encrypted_relay" } else { "webrtc_p2p" });

    sqlx::query(
        r#"
        UPDATE recovery_requests
        SET status = $2, transport_mode = $3, relay_ref_id = COALESCE($4, relay_ref_id),
            reason = COALESCE($5, reason), target_device_id = COALESCE($6, target_device_id), updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(new_status)
    .bind(mode)
    .bind(req.relay_ref_id)
    .bind(req.reason.as_deref())
    .bind(user.device_id)
    .execute(&state.db)
    .await?;

    sqlx::query("UPDATE media_references SET availability_state = $2, updated_at = now() WHERE id = $1")
        .bind(ref_id)
        .bind(new_avail)
        .execute(&state.db)
        .await?;

    state
        .ws_hub
        .send_to_users(
            &[req_uid, target_uid],
            WsEvent::RecoveryRequestUpdated(serde_json::json!({
                "request_id": request_id,
                "ref_id": ref_id,
                "status": new_status,
                "availability_state": new_avail,
                "transport_mode": mode,
                "relay_ref_id": req.relay_ref_id,
            })),
        )
        .await;

    Ok(Json(serde_json::json!({
        "request_id": request_id,
        "status": new_status,
        "transport_mode": mode,
        "relay_ref_id": req.relay_ref_id,
    })))
}

#[derive(Deserialize)]
struct P2pSignalReq {
    pub signal_type: String,
    pub data: serde_json::Value,
}

async fn handle_recovery_p2p_signal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    Json(req): Json<P2pSignalReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT requester_user_id, target_user_id FROM recovery_requests WHERE id = $1",
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("recovery_request_not_found"))?;
    let r_uid: Uuid = row.try_get("requester_user_id")?;
    let t_uid: Uuid = row.try_get("target_user_id")?;
    let target = if user.id == r_uid {
        t_uid
    } else if user.id == t_uid {
        r_uid
    } else {
        return Err(AppError::Forbidden("not_recovery_participant"));
    };

    state
        .ws_hub
        .send_to_user(
            &target,
            WsEvent::RecoveryP2pSignal(serde_json::json!({
                "request_id": request_id,
                "sender_id": user.id,
                "signal_type": req.signal_type,
                "data": req.data,
            })),
        )
        .await;

    Ok(Json(serde_json::json!({ "relayed": true })))
}

#[derive(Deserialize, Default)]
struct CompleteRecoveryReq {
    pub sha256_hex: Option<String>,
    pub local_tier: Option<String>,
}

async fn handle_complete_recovery_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    body: Option<Json<CompleteRecoveryReq>>,
) -> AppResult<Json<serde_json::Value>> {
    let user = caller(&state, &headers).await?;
    let row = sqlx::query(
        r#"
        SELECT rr.ref_id, rr.requester_user_id, rr.target_user_id, rr.relay_ref_id,
               mr.sha256_plain, mr.sha256_cipher
        FROM recovery_requests rr
        JOIN media_references mr ON mr.id = rr.ref_id
        WHERE rr.id = $1
        "#,
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("recovery_request_not_found"))?;

    let req_uid: Uuid = row.try_get("requester_user_id")?;
    let target_uid: Uuid = row.try_get("target_user_id")?;
    if req_uid != user.id {
        return Err(AppError::Forbidden("not_requester"));
    }

    let b = body.map(|j| j.0).unwrap_or_default();
    let ref_id: Uuid = row.try_get("ref_id")?;
    let relay_ref: Option<Uuid> = row.try_get("relay_ref_id")?;

    // Optional integrity verification against recorded reference digest if provided.
    if let Some(ref hex_str) = b.sha256_hex {
        let supplied = hex::decode(hex_str.trim()).map_err(|_| AppError::BadRequest("invalid_sha256"))?;
        let plain: Option<Vec<u8>> = row.try_get("sha256_plain")?;
        let cipher: Option<Vec<u8>> = row.try_get("sha256_cipher")?;
        let matches = plain.as_ref().is_some_and(|p| p == &supplied) || cipher.as_ref().is_some_and(|c| c == &supplied);
        if (plain.is_some() || cipher.is_some()) && !matches {
            return Err(AppError::BadRequest("recovered_integrity_mismatch"));
        }
    }

    sqlx::query("UPDATE recovery_requests SET status = 'completed', updated_at = now() WHERE id = $1")
        .bind(request_id)
        .execute(&state.db)
        .await?;
    sqlx::query(
        "UPDATE media_references SET availability_state = 'restored_from_peer', updated_at = now() WHERE id = $1",
    )
    .bind(ref_id)
    .execute(&state.db)
    .await?;

    if let Some(did) = user.device_id {
        let tier = b.local_tier.as_deref().filter(|t| matches!(*t, "opfs" | "indexeddb" | "cache_api" | "native_fs")).unwrap_or("opfs");
        let _ = sqlx::query(
            r#"
            INSERT INTO media_device_locations (ref_id, device_id, user_id, has_original, local_tier, last_verified_at)
            VALUES ($1, $2, $3, TRUE, $4, now())
            ON CONFLICT (ref_id, device_id) DO UPDATE SET has_original = TRUE, local_tier = EXCLUDED.local_tier, last_verified_at = now()
            "#,
        )
        .bind(ref_id)
        .bind(did)
        .bind(user.id)
        .bind(tier)
        .execute(&state.db)
        .await;
    }

    // Safely release temporary recovery relay blob if one was created for this request by the target peer.
    if let Some(r_id) = relay_ref {
        if let Ok(Some(oid)) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT object_id FROM media_references WHERE id = $1 AND owner_id = $2",
        )
            .bind(r_id)
            .bind(target_uid)
            .fetch_optional(&state.db)
            .await
            .map(|opt| opt.flatten())
        {
            let _ = migration::release_object_reference(&state.db, &state.storage, oid).await;
            let _ = sqlx::query(
                "UPDATE media_references SET object_id = NULL, deleted_for_all = TRUE WHERE id = $1 AND owner_id = $2",
            )
                .bind(r_id)
                .bind(target_uid)
                .execute(&state.db)
                .await;
        }
    }

    state
        .ws_hub
        .send_to_users(
            &[req_uid, target_uid],
            WsEvent::RecoveryRequestUpdated(serde_json::json!({
                "request_id": request_id,
                "ref_id": ref_id,
                "status": "completed",
                "availability_state": "restored_from_peer",
            })),
        )
        .await;

    Ok(Json(serde_json::json!({
        "request_id": request_id,
        "status": "completed",
        "availability_state": "restored_from_peer",
    })))
}

fn decode_b64_opt(s_opt: Option<&str>, expected_len: usize) -> Option<Vec<u8>> {
    let s = s_opt?.trim();
    if s.is_empty() {
        return None;
    }
    let v = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, s)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s))
        .ok()?;
    (v.len() == expected_len).then_some(v)
}
