//! Spec 011: two-step verification (TOTP, RFC 6238) with one-time backup codes,
//! login challenges, login alerts, a per-account security log, and the
//! multi-account "vault" that lets one browser keep several signed-in accounts.
//!
//! TOTP secrets are encrypted at rest (AES-256-GCM, key derived from the server
//! session secret). Backup codes are stored as SHA-256 hashes. Other accounts'
//! session tokens live only in an HttpOnly cookie scoped to /api/auth.

use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::Aead,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, create_session_raw, parse_cookie},
    },
    error::{AppError, AppResult},
    websocket::WsEvent,
};

pub const VAULT_COOKIE: &str = "chatw_accounts";
pub const TWOFA_COOKIE: &str = "chatw_2fa";
const MAX_ACCOUNTS: usize = 5;
const TOTP_STEP: u64 = 30;
const CHALLENGE_MINUTES: i64 = 5;
const MAX_CHALLENGE_ATTEMPTS: i32 = 5;

/// Routes mounted under /api/security.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/events", get(events))
        .route("/totp/setup", post(totp_setup))
        .route("/totp/enable", post(totp_enable))
        .route("/totp/disable", post(totp_disable))
        .route("/backup-codes/regenerate", post(regenerate_codes))
}

/// Routes mounted under /api/auth (they need the /api/auth-scoped cookies).
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/2fa/pending", get(twofa_pending))
        .route("/2fa/verify", post(twofa_verify))
        .route("/accounts", get(accounts_list))
        .route("/accounts/add", post(accounts_add))
        .route("/accounts/switch", post(accounts_switch))
        .route("/accounts/{user_id}/logout", post(accounts_logout_one))
        .route("/logout-all", post(logout_all))
}

// ---------------------------------------------------------------- helpers

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn rand_bytes<const N: usize>() -> AppResult<[u8; N]> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|_| AppError::Internal)?;
    Ok(b)
}

/// RFC 4648 base32 without padding (what authenticator apps expect).
pub fn base32(bytes: &[u8]) -> String {
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for &b in bytes {
        buf = (buf << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            out.push(A[((buf >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(A[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// RFC 6238 TOTP, HMAC-SHA1, 6 digits.
pub fn totp_at(secret: &[u8], step: u64) -> u32 {
    let mut mac = <Hmac<sha1::Sha1> as Mac>::new_from_slice(secret).expect("any key length");
    mac.update(&step.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let off = (h[19] & 0x0f) as usize;
    let bin = ((h[off] as u32 & 0x7f) << 24) | ((h[off + 1] as u32) << 16) | ((h[off + 2] as u32) << 8) | h[off + 3] as u32;
    bin % 1_000_000
}

/// The step that matches `code` within ±1 step (clock drift), if any.
pub fn totp_match(secret: &[u8], code: &str, now: u64) -> Option<u64> {
    let code = code.trim().replace(' ', "");
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let want: u32 = code.parse().ok()?;
    let cur = now / TOTP_STEP;
    [cur.saturating_sub(1), cur, cur + 1].into_iter().find(|s| {
        let got = totp_at(secret, *s);
        got.to_be_bytes().ct_eq(&want.to_be_bytes()).into()
    })
}

fn cipher(state: &AppState) -> Aes256Gcm {
    let mut h = Sha256::new();
    h.update(b"chatw-totp-secret-v1");
    h.update(state.config.session_secret.as_slice());
    Aes256Gcm::new_from_slice(&h.finalize()).expect("32-byte key")
}

fn seal(state: &AppState, plain: &[u8]) -> AppResult<Vec<u8>> {
    let nonce = rand_bytes::<12>()?;
    let mut out = nonce.to_vec();
    out.extend(cipher(state).encrypt(&Nonce::from(nonce), plain).map_err(|_| AppError::Internal)?);
    Ok(out)
}

fn open(state: &AppState, sealed: &[u8]) -> AppResult<Vec<u8>> {
    if sealed.len() < 13 {
        return Err(AppError::Internal);
    }
    let n: [u8; 12] = sealed[..12].try_into().map_err(|_| AppError::Internal)?;
    cipher(state).decrypt(&Nonce::from(n), &sealed[12..]).map_err(|_| AppError::Internal)
}

fn hash_code(code: &str) -> Vec<u8> {
    let norm: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    Sha256::digest(format!("chatw-backup-v1:{norm}").as_bytes()).to_vec()
}

fn new_backup_codes() -> AppResult<Vec<String>> {
    const A: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut out = Vec::new();
    for _ in 0..10 {
        let r = rand_bytes::<8>()?;
        let s: String = r.iter().map(|b| A[*b as usize % A.len()] as char).collect();
        out.push(format!("{}-{}", &s[..4], &s[4..]));
    }
    Ok(out)
}

pub fn ip_masked_with_trust(headers: &HeaderMap, trust_proxy: bool) -> String {
    if !trust_proxy {
        return "Direct".into();
    }
    let raw = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    // Keep only the network part: enough to notice "a new place", not to track anyone.
    if let Some((a, rest)) = raw.split_once('.')
        && let Some((b, _)) = rest.split_once('.')
    {
        return format!("{a}.{b}.x.x");
    }
    if raw.contains(':') {
        return raw.split(':').take(3).collect::<Vec<_>>().join(":") + "::";
    }
    "Unknown".into()
}

pub fn ip_masked(headers: &HeaderMap) -> String {
    ip_masked_with_trust(headers, true)
}

fn ua(headers: &HeaderMap) -> Option<String> {
    headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(|s| s.chars().take(300).collect())
}

fn device_of(ua: Option<&str>) -> String {
    let u = ua.unwrap_or_default();
    let os = if u.contains("Windows") {
        "Windows"
    } else if u.contains("Android") {
        "Android"
    } else if u.contains("iPhone") || u.contains("iPad") {
        "iOS"
    } else if u.contains("Mac OS") {
        "macOS"
    } else if u.contains("Linux") {
        "Linux"
    } else {
        "Devices"
    };
    let browser = if u.contains("Edg/") {
        "Edge"
    } else if u.contains("Firefox/") {
        "Firefox"
    } else if u.contains("Chrome/") {
        "Chrome"
    } else if u.contains("Safari/") {
        "Safari"
    } else {
        "application"
    };
    format!("{browser} on {os}")
}

pub async fn log_event(state: &AppState, user_id: Uuid, kind: &str, headers: &HeaderMap) {
    let u = ua(headers);
    let _ = sqlx::query!(
        "INSERT INTO security_events (user_id, kind, device_label, user_agent, ip_masked) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        kind,
        device_of(u.as_deref()),
        u,
        ip_masked_with_trust(headers, state.config.trust_proxy)
    )
    .execute(&state.db)
    .await;
}

async fn me(state: &AppState, headers: &HeaderMap) -> AppResult<Uuid> {
    let t = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    Ok(authenticate_session(&state.db, &state.config, t).await?.id)
}

fn secure(state: &AppState) -> &'static str {
    if state.config.cookie_secure { "; Secure" } else { "" }
}

fn cookie(state: &AppState, name: &str, value: &str, path: &str, max_age: i64) -> AppResult<HeaderValue> {
    format!("{name}={value}; Path={path}; HttpOnly; SameSite=Lax; Max-Age={max_age}{}", secure(state))
        .parse()
        .map_err(|_| AppError::Internal)
}

fn session_cookie(state: &AppState, token_hex: &str) -> AppResult<HeaderValue> {
    let age = if token_hex.is_empty() { 0 } else { state.config.session_ttl.as_secs() as i64 };
    cookie(state, SESSION_COOKIE_NAME, token_hex, "/", age)
}

fn vault_cookie(state: &AppState, tokens: &[String]) -> AppResult<HeaderValue> {
    let v = tokens.join(".");
    cookie(state, VAULT_COOKIE, &v, "/api/auth", if v.is_empty() { 0 } else { state.config.session_ttl.as_secs() as i64 })
}

fn vault_tokens(headers: &HeaderMap) -> Vec<String> {
    parse_cookie(headers, VAULT_COOKIE)
        .map(|v| v.split('.').filter(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit())).map(str::to_string).take(MAX_ACCOUNTS).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------- login completion

pub enum LoginOutcome {
    /// (raw token hex, Set-Cookie value)
    Session(String, String),
    /// Two-step verification needed: Set-Cookie value for the challenge.
    Challenge(Uuid, String),
}

/// Every login (Google, dev) goes through here: ask for the second step when
/// TOTP is on, otherwise create the session, log it and alert other devices
/// about a sign-in from a device that was never seen before.
pub async fn complete_login(state: &AppState, user_id: Uuid, headers: &HeaderMap) -> AppResult<LoginOutcome> {
    let enabled = sqlx::query_scalar!("SELECT totp_enabled_at IS NOT NULL AS \"on!\" FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await?;
    if enabled {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO twofa_challenges (id, user_id, user_agent, ip_masked, expires_at) VALUES ($1, $2, $3, $4, now() + make_interval(mins => $5::int))",
            id,
            user_id,
            ua(headers),
            ip_masked_with_trust(headers, state.config.trust_proxy),
            CHALLENGE_MINUTES as i32
        )
        .execute(&state.db)
        .await?;
        let c = cookie(state, TWOFA_COOKIE, &id.to_string(), "/api/auth", CHALLENGE_MINUTES * 60)?;
        return Ok(LoginOutcome::Challenge(id, c.to_str().unwrap_or_default().to_string()));
    }
    finish_session(state, user_id, headers, "login").await
}

async fn finish_session(state: &AppState, user_id: Uuid, headers: &HeaderMap, kind: &str) -> AppResult<LoginOutcome> {
    let u = ua(headers);
    let device = device_of(u.as_deref());
    let seen_before = sqlx::query_scalar!(
        r#"SELECT 1 AS "one!" FROM security_events WHERE user_id = $1 AND kind IN ('login', 'login_2fa') AND device_label = $2 LIMIT 1"#,
        user_id,
        device
    )
    .fetch_optional(&state.db)
    .await?
    .is_some();
    let has_history = sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM security_events WHERE user_id = $1 LIMIT 1"#, user_id)
        .fetch_optional(&state.db)
        .await?
        .is_some();
    let (raw, set_cookie) = create_session_raw(&state.db, &state.config, user_id, u.as_deref()).await?;
    log_event(state, user_id, kind, headers).await;
    if has_history && !seen_before {
        log_event(state, user_id, "new_device", headers).await;
        state
            .ws_hub
            .send_to_user(&user_id, WsEvent::SecurityAlert(json!({ "kind": "new_device", "device": device, "ip": ip_masked_with_trust(headers, state.config.trust_proxy) })))
            .await;
    }
    Ok(LoginOutcome::Session(raw, set_cookie))
}

async fn twofa_pending(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let pending = match parse_cookie(&headers, TWOFA_COOKIE).and_then(|v| Uuid::parse_str(v).ok()) {
        Some(id) => sqlx::query_scalar!(r#"SELECT 1 AS "one!" FROM twofa_challenges WHERE id = $1 AND expires_at > now()"#, id)
            .fetch_optional(&state.db)
            .await?
            .is_some(),
        None => false,
    };
    Ok(Json(json!({ "pending": pending })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyBody {
    code: String,
    /// Desktop clients that can't hold the cookie pass the challenge id instead.
    #[serde(default)]
    challenge: Option<Uuid>,
}

/// TOTP code or one backup code for `user_id`; returns the method used.
pub async fn check_second_factor(state: &AppState, user_id: Uuid, code: &str) -> AppResult<Option<&'static str>> {
    let row = sqlx::query!("SELECT totp_secret_enc FROM users WHERE id = $1", user_id).fetch_one(&state.db).await?;
    if let Some(enc) = row.totp_secret_enc {
        let secret = open(state, &enc)?;
        if let Some(step) = totp_match(&secret, code, now_secs()) {
            let claimed = sqlx::query_scalar!(
                "UPDATE users SET totp_last_step = $1 WHERE id = $2 AND COALESCE(totp_last_step, -1) < $1 RETURNING id",
                step as i64,
                user_id
            )
            .fetch_optional(&state.db)
            .await?;
            if claimed.is_none() {
                return Ok(None); // replayed or concurrent code
            }
            return Ok(Some("totp"));
        }
    }
    let h = hash_code(code);
    let used = sqlx::query_scalar!(
        "UPDATE backup_codes SET used_at = now() WHERE id = (SELECT id FROM backup_codes WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL LIMIT 1) AND used_at IS NULL RETURNING id",
        user_id,
        h
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(used.map(|_| "backup_code"))
}

async fn twofa_verify(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<VerifyBody>) -> AppResult<Response> {
    let id = b
        .challenge
        .or_else(|| parse_cookie(&headers, TWOFA_COOKIE).and_then(|v| Uuid::parse_str(v).ok()))
        .ok_or(AppError::BadRequest("no_challenge"))?;
    let ch = sqlx::query!(
        "UPDATE twofa_challenges SET attempts = attempts + 1 WHERE id = $1 AND expires_at > now() RETURNING user_id, attempts",
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("challenge_expired"))?;
    if ch.attempts > MAX_CHALLENGE_ATTEMPTS {
        sqlx::query!("DELETE FROM twofa_challenges WHERE id = $1", id).execute(&state.db).await?;
        return Err(AppError::TooManyRequests);
    }
    guard_attempts(&state, ch.user_id).await?;
    let Some(method) = check_second_factor(&state, ch.user_id, &b.code).await? else {
        log_event(&state, ch.user_id, "failed_2fa", &headers).await;
        return Err(AppError::BadRequest("invalid_code"));
    };
    sqlx::query!("DELETE FROM twofa_challenges WHERE id = $1", id).execute(&state.db).await?;
    if method == "backup_code" {
        log_event(&state, ch.user_id, "backup_code_used", &headers).await;
    }
    let LoginOutcome::Session(_, set_cookie) = finish_session(&state, ch.user_id, &headers, "login_2fa").await? else {
        return Err(AppError::Internal);
    };
    let mut resp = Json(json!({ "ok": true, "method": method })).into_response();
    resp.headers_mut().append(header::SET_COOKIE, set_cookie.parse().map_err(|_| AppError::Internal)?);
    resp.headers_mut().append(header::SET_COOKIE, cookie(&state, TWOFA_COOKIE, "", "/api/auth", 0)?);
    Ok(resp)
}

// ---------------------------------------------------------------- TOTP management

async fn status(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    let r = sqlx::query!(
        r#"SELECT totp_enabled_at, (SELECT count(*) FROM backup_codes WHERE user_id = $1 AND used_at IS NULL) AS "left!" FROM users WHERE id = $1"#,
        uid
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "totp_enabled": r.totp_enabled_at.is_some(), "totp_enabled_at": r.totp_enabled_at, "backup_codes_left": r.left })))
}

async fn events(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    let rows = sqlx::query!(
        "SELECT kind, device_label, ip_masked, created_at FROM security_events WHERE user_id = $1 ORDER BY created_at DESC LIMIT 100",
        uid
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|r| json!({ "kind": r.kind, "device": r.device_label, "ip": r.ip_masked, "at": r.created_at }))
        .collect::<Vec<_>>())))
}

/// Too many wrong codes in 15 minutes → 429 (protects the 6-digit space).
async fn guard_attempts(state: &AppState, uid: Uuid) -> AppResult<()> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM security_events WHERE user_id = $1 AND kind = 'failed_2fa' AND created_at > now() - interval '15 minutes'"#,
        uid
    )
    .fetch_one(&state.db)
    .await?;
    if n >= 10 { Err(AppError::TooManyRequests) } else { Ok(()) }
}

async fn totp_setup(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    let r = sqlx::query!("SELECT username, totp_enabled_at FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    if r.totp_enabled_at.is_some() {
        return Err(AppError::Conflict("totp_already_enabled"));
    }
    let secret = rand_bytes::<20>()?;
    sqlx::query!("UPDATE users SET totp_pending_enc = $1 WHERE id = $2", seal(&state, &secret)?, uid).execute(&state.db).await?;
    let b32 = base32(&secret);
    let label = urlencoding::encode(&format!("ChatW:{}", r.username.unwrap_or_else(|| "account".into()))).into_owned();
    Ok(Json(json!({
        "secret": b32,
        "otpauth_uri": format!("otpauth://totp/{label}?secret={b32}&issuer=ChatW&algorithm=SHA1&digits=6&period=30"),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodeBody {
    code: String,
}

async fn store_backup_codes(state: &AppState, uid: Uuid) -> AppResult<Vec<String>> {
    let codes = new_backup_codes()?;
    let mut tx = state.db.begin().await?;
    sqlx::query!("DELETE FROM backup_codes WHERE user_id = $1", uid).execute(&mut *tx).await?;
    for c in &codes {
        sqlx::query!("INSERT INTO backup_codes (user_id, code_hash) VALUES ($1, $2)", uid, hash_code(c)).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(codes)
}

async fn totp_enable(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<CodeBody>) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    guard_attempts(&state, uid).await?;
    let r = sqlx::query!("SELECT totp_pending_enc, totp_enabled_at FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    if r.totp_enabled_at.is_some() {
        return Err(AppError::Conflict("totp_already_enabled"));
    }
    let pending = r.totp_pending_enc.ok_or(AppError::BadRequest("setup_first"))?;
    let secret = open(&state, &pending)?;
    let Some(step) = totp_match(&secret, &b.code, now_secs()) else {
        log_event(&state, uid, "failed_2fa", &headers).await;
        return Err(AppError::BadRequest("invalid_code"));
    };
    sqlx::query!(
        "UPDATE users SET totp_secret_enc = $1, totp_pending_enc = NULL, totp_enabled_at = now(), totp_last_step = $2 WHERE id = $3",
        pending,
        step as i64,
        uid
    )
    .execute(&state.db)
    .await?;
    let codes = store_backup_codes(&state, uid).await?;
    log_event(&state, uid, "totp_enabled", &headers).await;
    Ok(Json(json!({ "enabled": true, "backup_codes": codes })))
}

async fn totp_disable(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<CodeBody>) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    guard_attempts(&state, uid).await?;
    if check_second_factor(&state, uid, &b.code).await?.is_none() {
        log_event(&state, uid, "failed_2fa", &headers).await;
        return Err(AppError::BadRequest("invalid_code"));
    }
    sqlx::query!("UPDATE users SET totp_secret_enc = NULL, totp_pending_enc = NULL, totp_enabled_at = NULL, totp_last_step = NULL WHERE id = $1", uid)
        .execute(&state.db)
        .await?;
    sqlx::query!("DELETE FROM backup_codes WHERE user_id = $1", uid).execute(&state.db).await?;
    log_event(&state, uid, "totp_disabled", &headers).await;
    Ok(Json(json!({ "enabled": false })))
}

async fn regenerate_codes(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<CodeBody>) -> AppResult<Json<Value>> {
    let uid = me(&state, &headers).await?;
    guard_attempts(&state, uid).await?;
    if check_second_factor(&state, uid, &b.code).await?.is_none() {
        log_event(&state, uid, "failed_2fa", &headers).await;
        return Err(AppError::BadRequest("invalid_code"));
    }
    let codes = store_backup_codes(&state, uid).await?;
    log_event(&state, uid, "backup_codes_regenerated", &headers).await;
    Ok(Json(json!({ "backup_codes": codes })))
}

/// Sensitive actions (sign out every other device) need the second step when it is on.
pub async fn require_second_factor(state: &AppState, uid: Uuid, code: Option<&str>, headers: &HeaderMap) -> AppResult<()> {
    let on = sqlx::query_scalar!("SELECT totp_enabled_at IS NOT NULL AS \"on!\" FROM users WHERE id = $1", uid).fetch_one(&state.db).await?;
    if !on {
        return Ok(());
    }
    guard_attempts(state, uid).await?;
    let code = code.ok_or(AppError::Forbidden("second_factor_required"))?;
    if check_second_factor(state, uid, code).await?.is_none() {
        log_event(state, uid, "failed_2fa", headers).await;
        return Err(AppError::Forbidden("invalid_code"));
    }
    Ok(())
}

// ---------------------------------------------------------------- multiple accounts

async fn accounts_list(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let current = parse_cookie(&headers, SESSION_COOKIE_NAME).unwrap_or_default().to_string();
    let mut tokens = vec![current.clone()];
    tokens.extend(vault_tokens(&headers));
    let mut out = Vec::new();
    let mut keep = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, t) in tokens.iter().enumerate() {
        if t.is_empty() {
            continue;
        }
        let Ok(u) = authenticate_session(&state.db, &state.config, t).await else { continue };
        if !seen.insert(u.id) {
            continue;
        }
        let info = sqlx::query!(
            r#"SELECT u.display_name,
                      (SELECT count(*) FROM messages m LEFT JOIN dm_reads r ON r.reader_id = m.receiver_id AND r.partner_id = m.sender_id
                       WHERE m.receiver_id = u.id AND m.sender_id <> u.id AND m.seq > COALESCE(r.last_read_seq, 0)) AS "unread!"
               FROM users u WHERE u.id = $1"#,
            u.id
        )
        .fetch_one(&state.db)
        .await?;
        out.push(json!({ "user_id": u.id, "username": u.username, "display_name": info.display_name, "avatar_url": u.avatar_url, "unread": info.unread, "active": i == 0 }));
        if i > 0 {
            keep.push(t.clone());
        }
    }
    let mut resp = Json(json!(out)).into_response();
    // Drop expired / duplicate accounts from the vault.
    if keep.len() != vault_tokens(&headers).len() {
        resp.headers_mut().append(header::SET_COOKIE, vault_cookie(&state, &keep)?);
    }
    Ok(resp)
}

/// Park the current account in the vault and go to the sign-in screen for another one.
async fn accounts_add(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let current = parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?.to_string();
    let uid = authenticate_session(&state.db, &state.config, &current).await?.id;
    let mut vault = vault_tokens(&headers);
    vault.retain(|t| t != &current);
    if vault.len() >= MAX_ACCOUNTS {
        return Err(AppError::BadRequest("account_limit_reached"));
    }
    vault.insert(0, current);
    log_event(&state, uid, "account_added", &headers).await;
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut().append(header::SET_COOKIE, vault_cookie(&state, &vault)?);
    resp.headers_mut().append(header::SET_COOKIE, session_cookie(&state, "")?);
    Ok(resp)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SwitchBody {
    user_id: Uuid,
}

async fn accounts_switch(State(state): State<AppState>, headers: HeaderMap, Json(b): Json<SwitchBody>) -> AppResult<Response> {
    let current = parse_cookie(&headers, SESSION_COOKIE_NAME).unwrap_or_default().to_string();
    let mut vault = vault_tokens(&headers);
    let mut found = None;
    for t in &vault {
        if let Ok(u) = authenticate_session(&state.db, &state.config, t).await {
            if u.id == b.user_id {
                found = Some(t.clone());
                break;
            }
        }
    }
    let target = found.ok_or(AppError::NotFound("account_not_signed_in"))?;
    vault.retain(|t| t != &target);
    if !current.is_empty() && authenticate_session(&state.db, &state.config, &current).await.is_ok() {
        vault.insert(0, current);
    }
    log_event(&state, b.user_id, "account_switched", &headers).await;
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut().append(header::SET_COOKIE, vault_cookie(&state, &vault)?);
    resp.headers_mut().append(header::SET_COOKIE, session_cookie(&state, &target)?);
    Ok(resp)
}

async fn destroy(state: &AppState, token: &str) {
    if let Ok(bytes) = hex::decode(token) {
        if bytes.len() == 32 {
            let h = crate::auth::session::hash_session_token(&state.config.session_secret, &bytes);
            let _ = sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", &h[..]).execute(&state.db).await;
        }
    }
}

async fn accounts_logout_one(State(state): State<AppState>, headers: HeaderMap, Path(user_id): Path<Uuid>) -> AppResult<Response> {
    let mut vault = vault_tokens(&headers);
    let mut removed = false;
    let mut keep = Vec::new();
    for t in vault.drain(..) {
        match authenticate_session(&state.db, &state.config, &t).await {
            Ok(u) if u.id == user_id => {
                destroy(&state, &t).await;
                log_event(&state, user_id, "logout", &headers).await;
                removed = true;
            }
            _ => keep.push(t),
        }
    }
    let mut cleared_current = false;
    if let Some(cur) = parse_cookie(&headers, SESSION_COOKIE_NAME)
        && let Ok(u) = authenticate_session(&state.db, &state.config, cur).await
        && u.id == user_id
    {
        destroy(&state, cur).await;
        log_event(&state, user_id, "logout", &headers).await;
        removed = true;
        cleared_current = true;
    }
    if !removed {
        return Err(AppError::NotFound("account_not_signed_in"));
    }
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut().append(header::SET_COOKIE, vault_cookie(&state, &keep)?);
    if cleared_current {
        resp.headers_mut().append(header::SET_COOKIE, session_cookie(&state, "")?);
    }
    Ok(resp)
}

async fn logout_all(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let mut all = vault_tokens(&headers);
    if let Some(c) = parse_cookie(&headers, SESSION_COOKIE_NAME) {
        all.push(c.to_string());
    }
    for t in &all {
        if let Ok(u) = authenticate_session(&state.db, &state.config, t).await {
            log_event(&state, u.id, "logout", &headers).await;
        }
        destroy(&state, t).await;
    }
    let mut resp = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
    resp.headers_mut().append(header::SET_COOKIE, vault_cookie(&state, &[])?);
    resp.headers_mut().append(header::SET_COOKIE, session_cookie(&state, "")?);
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_test_vectors() {
        // RFC 6238 appendix B, SHA1 seed "12345678901234567890" (8 digits there; 6 = last six).
        let seed = b"12345678901234567890";
        assert_eq!(totp_at(seed, 59 / 30), 287082);
        assert_eq!(totp_at(seed, 1111111109 / 30), 81804);
        assert_eq!(totp_at(seed, 1234567890 / 30), 5924);
        assert_eq!(totp_at(seed, 2000000000 / 30), 279037);
        assert_eq!(totp_match(seed, "287082", 59), Some(1));
        assert_eq!(totp_match(seed, "287 082", 59 + 30), Some(1)); // one step of drift
        assert_eq!(totp_match(seed, "287082", 59 + 90), None);
        assert_eq!(totp_match(seed, "28708a", 59), None);
    }

    #[test]
    fn base32_and_codes() {
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
        assert_eq!(base32(b""), "");
        let codes = new_backup_codes().unwrap();
        assert_eq!(codes.len(), 10);
        assert!(codes.iter().all(|c| c.len() == 9 && c.as_bytes()[4] == b'-'));
        assert_eq!(hash_code("ABCD-efgh"), hash_code("abcdefgh"));
    }

    #[test]
    fn ip_is_masked() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.45, 10.0.0.1".parse().unwrap());
        assert_eq!(ip_masked(&h), "203.0.x.x");
        assert_eq!(ip_masked(&HeaderMap::new()), "Unknown");
    }
}
