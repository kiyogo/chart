//! Plugin Platform Security Primitives:
//! - SSRF URL & DNS IP validation + redirect-disabled pinned HTTP client
//! - AES-256-GCM encryption at rest for secrets & webhook HMAC signing keys
//! - Constant-time HMAC-SHA256 signing & replay-protected verification
//! - Sliding-window rate limiter
//! - Server / Group / Channel RBAC scope enforcement

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    groups::{get_member_role, manage::is_admin_role},
    models::permissions as perm_flags,
    servers::permissions::has_server_permission,
};

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// 1. Secret Encryption at Rest (AES-256-GCM with HKDF-SHA256 key derivation)
// ============================================================================

fn derive_secret_key(master_secret: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"chatw-plugin-secrets-v1"), master_secret);
    let mut okm = [0u8; 32];
    hk.expand(b"aes-256-gcm-key", &mut okm)
        .expect("HKDF expand 32 bytes");
    okm
}

#[allow(deprecated)]
pub fn encrypt_secret(master_secret: &[u8], plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let key = derive_secret_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal)?;
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|_| AppError::Internal)?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| AppError::Internal)?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[allow(deprecated)]
pub fn decrypt_secret(master_secret: &[u8], blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() < 28 {
        return Err(AppError::Internal);
    }
    let key = derive_secret_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal)?;
    cipher
        .decrypt(Nonce::from_slice(&blob[..12]), &blob[12..])
        .map_err(|_| AppError::Internal)
}

// ============================================================================
// 2. Constant-Time HMAC-SHA256 Webhook Signing & Replay Verification
// ============================================================================

pub fn sign_webhook_payload(secret: &str, timestamp: i64, body: &[u8]) -> String {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

pub fn verify_webhook_signature(
    secret: &str,
    timestamp: i64,
    now_ts: i64,
    max_skew_secs: i64,
    body: &[u8],
    header_sig: &str,
) -> bool {
    if (now_ts - timestamp).abs() > max_skew_secs {
        return false;
    }
    let expected = sign_webhook_payload(secret, timestamp, body);
    let a = expected.as_bytes();
    let b = header_sig.trim().as_bytes();
    if a.len() != b.len() {
        return false;
    }
    bool::from(a.ct_eq(b))
}

pub fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    bool::from(a.ct_eq(b))
}

// ============================================================================
// 3. SSRF Protection & Redirect-Disabled Outbound HTTP Client
// ============================================================================

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                // CGNAT 100.64.0.0/10
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                // Cloud metadata / link-local 169.254.0.0/16
                || (o[0] == 169 && o[1] == 254)
                // Benchmarking / reserved
                || (o[0] == 0)
                || (o[0] >= 240)
        }
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // Unique Local Address fc00::/7
                || (segs[0] & 0xfe00) == 0xfc00
                // Link-local fe80::/10
                || (segs[0] & 0xffc0) == 0xfe80
                // IPv4-mapped ::ffff:a.b.c.d
                || v6.to_ipv4_mapped().map(|v4| is_blocked_ip(IpAddr::V4(v4))).unwrap_or(false)
        }
    }
}

fn allow_local_webhooks() -> bool {
    std::env::var("CHATW_ALLOW_LOCAL_WEBHOOKS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Clone, Debug)]
pub struct ValidatedOutboundTarget {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub resolved: SocketAddr,
}

/// Validates URL scheme (HTTPS required in production), rejects credentials/control chars,
/// resolves DNS, blocks private/loopback/metadata IPs (SSRF), and pins the resolved IP.
pub async fn validate_outbound_url(raw_url: &str) -> AppResult<ValidatedOutboundTarget> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 || trimmed.contains('\r') || trimmed.contains('\n') {
        return Err(AppError::BadRequest("invalid_target_url"));
    }
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| AppError::BadRequest("invalid_target_url"))?;
    let scheme = parsed.scheme();
    let local_ok = allow_local_webhooks();
    let host = parsed
        .host_str()
        .ok_or(AppError::BadRequest("invalid_target_host"))?
        .to_string();

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AppError::BadRequest("url_credentials_forbidden"));
    }

    let is_local_host = host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]";

    if scheme != "https" {
        if !(scheme == "http" && local_ok && is_local_host) {
            return Err(AppError::BadRequest("https_required"));
        }
    }

    let port = parsed
        .port_or_known_default()
        .ok_or(AppError::BadRequest("invalid_target_port"))?;

    // Check literal IP or resolve DNS
    let host_unbracketed = host.trim_start_matches('[').trim_end_matches(']');
    let addrs: Vec<SocketAddr> = if let Ok(ip) = host_unbracketed.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        tokio::net::lookup_host((host_unbracketed, port))
            .await
            .map_err(|_| AppError::BadRequest("dns_resolution_failed"))?
            .collect()
    };

    if addrs.is_empty() {
        return Err(AppError::BadRequest("dns_resolution_failed"));
    }

    for addr in &addrs {
        if is_blocked_ip(addr.ip()) && !(local_ok && addr.ip().is_loopback()) {
            return Err(AppError::Forbidden("ssrf_blocked"));
        }
    }

    Ok(ValidatedOutboundTarget {
        url: parsed.to_string(),
        host: host_unbracketed.to_string(),
        port,
        resolved: addrs[0],
    })
}

/// Builds a pinned, redirect-disabled HTTP client bound strictly to the validated IP (blocks DNS rebinding & redirect SSRF).
pub fn build_pinned_client(target: &ValidatedOutboundTarget, timeout_secs: u64) -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(timeout_secs.clamp(1, 15)))
        .resolve(&target.host, target.resolved)
        .build()
        .map_err(|_| AppError::Internal)
}

// ============================================================================
// 4. Sliding-Window Rate Limiter
// ============================================================================

pub struct SlidingRateLimiter {
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl Default for SlidingRateLimiter {
    fn default() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
        }
    }
}

impl SlidingRateLimiter {
    pub fn check_and_record(&self, key: &str, max_per_window: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut guard = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let q = guard.entry(key.to_string()).or_default();
        while let Some(&front) = q.front() {
            if now.duration_since(front) > window {
                q.pop_front();
            } else {
                break;
            }
        }
        if q.len() >= max_per_window {
            return false;
        }
        q.push_back(now);
        true
    }
}

pub static PLUGIN_RATE_LIMITER: std::sync::LazyLock<SlidingRateLimiter> =
    std::sync::LazyLock::new(SlidingRateLimiter::default);

// ============================================================================
// 5. Scope RBAC Helpers (User / Server / Group / Channel)
// ============================================================================

pub async fn require_scope_manage_access(
    state: &AppState,
    actor: Uuid,
    scope_type: &str,
    scope_id: Uuid,
) -> AppResult<()> {
    match scope_type {
        "user" | "device" | "conversation" => {
            if scope_id != actor {
                return Err(AppError::Forbidden("scope_access_denied"));
            }
            Ok(())
        }
        "server" => {
            if !has_server_permission(&state.db, scope_id, actor, perm_flags::MANAGE_SERVER).await? {
                return Err(AppError::Forbidden("insufficient_server_permissions"));
            }
            Ok(())
        }
        "channel" => {
            let server_id: Option<Uuid> =
                sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
                    .bind(scope_id)
                    .fetch_optional(&state.db)
                    .await?;
            let sid = server_id.ok_or(AppError::NotFound("channel_not_found"))?;
            if !has_server_permission(&state.db, sid, actor, perm_flags::MANAGE_CHANNELS).await?
                && !has_server_permission(&state.db, sid, actor, perm_flags::MANAGE_SERVER).await?
            {
                return Err(AppError::Forbidden("insufficient_channel_permissions"));
            }
            Ok(())
        }
        "group" => {
            let role = get_member_role(&state.db, scope_id, actor).await?;
            if !role.as_deref().is_some_and(is_admin_role) {
                return Err(AppError::Forbidden("insufficient_group_permissions"));
            }
            Ok(())
        }
        _ => Err(AppError::BadRequest("invalid_scope_type")),
    }
}

pub async fn require_scope_read_access(
    state: &AppState,
    actor: Uuid,
    scope_type: &str,
    scope_id: Uuid,
) -> AppResult<()> {
    match scope_type {
        "user" | "device" | "conversation" => {
            if scope_id != actor {
                return Err(AppError::Forbidden("scope_access_denied"));
            }
            Ok(())
        }
        "server" => {
            let is_member: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
            )
            .bind(scope_id)
            .bind(actor)
            .fetch_optional(&state.db)
            .await?;
            if is_member.is_none() {
                return Err(AppError::Forbidden("not_server_member"));
            }
            Ok(())
        }
        "channel" => {
            let server_id: Option<Uuid> =
                sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
                    .bind(scope_id)
                    .fetch_optional(&state.db)
                    .await?;
            let sid = server_id.ok_or(AppError::NotFound("channel_not_found"))?;
            let is_member: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
            )
            .bind(sid)
            .bind(actor)
            .fetch_optional(&state.db)
            .await?;
            if is_member.is_none() {
                return Err(AppError::Forbidden("not_server_member"));
            }
            Ok(())
        }
        "group" => {
            let role = get_member_role(&state.db, scope_id, actor).await?;
            if role.is_none() {
                return Err(AppError::Forbidden("not_group_member"));
            }
            Ok(())
        }
        _ => Err(AppError::BadRequest("invalid_scope_type")),
    }
}

pub async fn is_platform_admin(state: &AppState, user_id: Uuid) -> AppResult<bool> {
    let admin: Option<bool> = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;
    Ok(admin.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_encryption_roundtrip_and_tamper_rejection() {
        let ct = encrypt_secret(b"super-secret-master-key", b"whsec_123456").unwrap();
        let pt = decrypt_secret(b"super-secret-master-key", &ct).unwrap();
        assert_eq!(pt, b"whsec_123456");
        let mut tampered = ct.clone();
        tampered[15] ^= 0xff;
        assert!(decrypt_secret(b"super-secret-master-key", &tampered).is_err());
        assert!(decrypt_secret(b"wrong-key", &ct).is_err());
    }

    #[test]
    fn webhook_hmac_and_replay_protection() {
        let body = br#"{"event":"message.new","id":"evt_1"}"#;
        let ts = 1_700_000_000;
        let sig = sign_webhook_payload("sec_abc", ts, body);
        assert!(verify_webhook_signature("sec_abc", ts, ts + 30, 300, body, &sig));
        // Wrong secret
        assert!(!verify_webhook_signature("sec_wrong", ts, ts + 30, 300, body, &sig));
        // Replay outside window
        assert!(!verify_webhook_signature("sec_abc", ts, ts + 600, 300, body, &sig));
        // Tampered body
        assert!(!verify_webhook_signature("sec_abc", ts, ts + 30, 300, b"{}", &sig));
    }

    #[test]
    fn ssrf_blocks_private_loopback_and_metadata_ips() {
        assert!(is_blocked_ip("127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("10.0.0.5".parse().unwrap()));
        assert!(is_blocked_ip("172.16.1.1".parse().unwrap()));
        assert!(is_blocked_ip("192.168.1.1".parse().unwrap()));
        assert!(is_blocked_ip("169.254.169.254".parse().unwrap()));
        assert!(is_blocked_ip("100.64.0.1".parse().unwrap()));
        assert!(is_blocked_ip("::1".parse().unwrap()));
        assert!(is_blocked_ip("fc00::1".parse().unwrap()));
        assert!(is_blocked_ip("fe80::1".parse().unwrap()));
        assert!(!is_blocked_ip("93.184.216.34".parse().unwrap()));
        assert!(!is_blocked_ip("1.1.1.1".parse().unwrap()));
    }
}
