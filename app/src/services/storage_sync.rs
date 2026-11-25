use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{models::ApiError, platform};

async fn req_json<T: serde::de::DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

fn default_media_quality_mode() -> String {
    "ultra_optimized".into()
}
fn default_storage_mode() -> String {
    "auto".into()
}
fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoragePolicy {
    pub cache_limit_mb: i32,
    pub auto_download_mode: String,
    pub dm_relay_ttl_hours: i32,
    pub dm_encrypted_backup: bool,
    pub require_device_approval: bool,
    pub prefer_original_upload: bool,
    #[serde(default = "default_media_quality_mode")]
    pub media_quality_mode: String,
    #[serde(default = "default_true")]
    pub ai_enhancement_enabled: bool,
    #[serde(default)]
    pub data_saver_mode: bool,
    #[serde(default = "default_true")]
    pub auto_cache_cleanup: bool,
    #[serde(default = "default_true")]
    pub server_durable_dm: bool,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: String,
    #[serde(default = "default_true")]
    pub compression_rollout_enabled: bool,
}

impl Default for StoragePolicy {
    fn default() -> Self {
        Self {
            cache_limit_mb: 1024,
            auto_download_mode: "wifi".into(),
            dm_relay_ttl_hours: 168,
            dm_encrypted_backup: false,
            require_device_approval: false,
            prefer_original_upload: false,
            media_quality_mode: default_media_quality_mode(),
            ai_enhancement_enabled: true,
            data_saver_mode: false,
            auto_cache_cleanup: true,
            server_durable_dm: true,
            storage_mode: default_storage_mode(),
            compression_rollout_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConvStoragePolicy {
    pub allow_peer_recovery: bool,
    pub dm_encrypted_backup: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub id: String,
    pub is_current: bool,
    #[serde(default)]
    pub client_device_id: Option<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub device_label: Option<String>,
    pub platform: String,
    pub trust_state: String,
    pub has_key: bool,
    #[serde(default)]
    pub last_seen_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceApprovalPending {
    pub id: String,
    pub requesting_device_id: String,
    #[serde(default)]
    pub device_label: Option<String>,
    pub platform: String,
    pub verification_code: String,
    #[serde(default)]
    pub ecdh_public_b64: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DevicesListResponse {
    pub devices: Vec<DeviceEntry>,
    pub pending_requests: Vec<DeviceApprovalPending>,
    #[serde(default)]
    pub require_device_approval: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecoveryRequestEntry {
    pub id: String,
    pub ref_id: String,
    pub requester_user_id: String,
    pub target_user_id: String,
    pub is_incoming: bool,
    pub transport_mode: String,
    pub status: String,
    #[serde(default)]
    pub relay_ref_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    pub file_name: String,
    pub mime_type: String,
    pub byte_size: i64,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecoveryListResponse {
    pub requests: Vec<RecoveryRequestEntry>,
}

pub async fn get_capabilities() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/storage/capabilities", None).await
}

pub async fn get_policy() -> Result<StoragePolicy, ApiError> {
    req_json("GET", "/api/storage/policy", None).await
}

pub async fn update_policy(policy: &StoragePolicy) -> Result<StoragePolicy, ApiError> {
    req_json("PUT", "/api/storage/policy", Some(serde_json::to_value(policy).unwrap_or_default())).await
}

pub async fn get_conv_policy(peer_id: &str) -> Result<ConvStoragePolicy, ApiError> {
    req_json("GET", &format!("/api/storage/conversations/{}", urlencoding::encode(peer_id)), None).await
}

pub async fn update_conv_policy(peer_id: &str, policy: &ConvStoragePolicy) -> Result<ConvStoragePolicy, ApiError> {
    req_json(
        "PUT",
        &format!("/api/storage/conversations/{}", urlencoding::encode(peer_id)),
        Some(serde_json::to_value(policy).unwrap_or_default()),
    )
    .await
}

pub async fn get_metrics() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/storage/metrics", None).await
}

pub async fn trigger_cleanup() -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/cleanup", None).await
}

pub async fn trigger_gc_incomplete() -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/gc/incomplete", None).await
}

pub async fn check_dedup(payload: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/dedup/check", Some(payload)).await
}

pub async fn claim_dedup(payload: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/dedup/claim", Some(payload)).await
}

pub async fn cdc_plan(payload: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/dedup/cdc/plan", Some(payload)).await
}

pub async fn cdc_put_chunk(blake3_hex: &str, bytes: Vec<u8>) -> Result<serde_json::Value, ApiError> {
    let path = format!("/api/storage/dedup/cdc/chunks/{}", urlencoding::encode(blake3_hex));
    let (_, body) = platform::http_request("PUT", &path, None, Some((bytes, "application/octet-stream"))).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

pub async fn cdc_commit(payload: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/dedup/cdc/commit", Some(payload)).await
}

pub async fn get_manifest(ref_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("GET", &format!("/api/storage/references/{}/manifest", urlencoding::encode(ref_id)), None).await
}

pub async fn put_manifest(ref_id: &str, payload: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    req_json("POST", &format!("/api/storage/references/{}/manifest", urlencoding::encode(ref_id)), Some(payload)).await
}

pub async fn run_migration_job(dry_run: bool) -> Result<serde_json::Value, ApiError> {
    req_json("POST", "/api/storage/migration/run", Some(json!({ "dry_run": dry_run }))).await
}

pub async fn get_ref_meta(ref_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("GET", &format!("/api/storage/references/{}/meta", urlencoding::encode(ref_id)), None).await
}

pub async fn ack_relay_copy(ref_id: &str, local_tier: &str, sha256_hex: &str, has_original: bool) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/storage/relay/{}/ack", urlencoding::encode(ref_id)),
        Some(json!({
            "local_tier": local_tier,
            "sha256_hex": sha256_hex,
            "has_original": has_original,
        })),
    )
    .await
}

pub async fn delete_reference(ref_id: &str, scope: &str, force: bool) -> Result<serde_json::Value, ApiError> {
    req_json(
        "DELETE",
        &format!(
            "/api/storage/references/{}?scope={}&force={}",
            urlencoding::encode(ref_id),
            urlencoding::encode(scope),
            force
        ),
        None,
    )
    .await
}

pub async fn forward_reference(
    from_ref_id: &str,
    target_scope_type: &str,
    target_scope_id: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/storage/forward",
        Some(json!({
            "from_ref_id": from_ref_id,
            "target_scope_type": target_scope_type,
            "target_scope_id": target_scope_id,
        })),
    )
    .await
}

#[derive(Clone, Default)]
pub struct DedupUploadMeta<'a> {
    pub orig_blake3: Option<&'a str>,
    pub pdq_hex: Option<&'a str>,
    pub pdq_quality: Option<u8>,
    pub caption_band_hash: Option<u64>,
    pub color_moment_hash: Option<u64>,
    pub aspect_ratio_milli: Option<u16>,
    pub orig_profile_json: Option<&'a str>,
    pub orig_width: Option<u32>,
    pub orig_height: Option<u32>,
    pub ai_recommended: bool,
    pub video_fingerprint_json: Option<&'a str>,
    pub audio_fingerprint_json: Option<&'a str>,
}

pub async fn upload_attachment_scoped(
    bytes: Vec<u8>,
    content_type: &str,
    scope_type: &str,
    scope_id: Option<&str>,
    file_name: Option<&str>,
    mime_type: Option<&str>,
    sha256_plain: Option<&str>,
    is_e2ee: bool,
    preserve_original: bool,
) -> Result<serde_json::Value, ApiError> {
    upload_attachment_with_dedup(
        bytes,
        content_type,
        scope_type,
        scope_id,
        file_name,
        mime_type,
        sha256_plain,
        is_e2ee,
        preserve_original,
        &DedupUploadMeta::default(),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn upload_attachment_with_dedup(
    bytes: Vec<u8>,
    content_type: &str,
    scope_type: &str,
    scope_id: Option<&str>,
    file_name: Option<&str>,
    mime_type: Option<&str>,
    sha256_plain: Option<&str>,
    is_e2ee: bool,
    preserve_original: bool,
    dedup: &DedupUploadMeta<'_>,
) -> Result<serde_json::Value, ApiError> {
    let mut qs = vec![
        format!("scope_type={}", urlencoding::encode(scope_type)),
        format!("is_e2ee={is_e2ee}"),
        format!("preserve_original={preserve_original}"),
    ];
    if let Some(sid) = scope_id {
        qs.push(format!("scope_id={}", urlencoding::encode(sid)));
    }
    if let Some(fnm) = file_name {
        qs.push(format!("file_name={}", urlencoding::encode(fnm)));
    }
    if let Some(mt) = mime_type {
        qs.push(format!("mime_type={}", urlencoding::encode(mt)));
    }
    if let Some(sp) = sha256_plain {
        qs.push(format!("sha256_plain={}", urlencoding::encode(sp)));
    }
    if let Some(ob3) = dedup.orig_blake3 {
        qs.push(format!("orig_blake3={}", urlencoding::encode(ob3)));
    }
    if let Some(ph) = dedup.pdq_hex {
        qs.push(format!("pdq_hex={}", urlencoding::encode(ph)));
    }
    if let Some(pq) = dedup.pdq_quality {
        qs.push(format!("pdq_quality={pq}"));
    }
    if let Some(ch) = dedup.caption_band_hash {
        qs.push(format!("caption_band_hash={ch}"));
    }
    if let Some(cm) = dedup.color_moment_hash {
        qs.push(format!("color_moment_hash={cm}"));
    }
    if let Some(am) = dedup.aspect_ratio_milli {
        qs.push(format!("aspect_ratio_milli={am}"));
    }
    if let Some(qp) = dedup.orig_profile_json {
        qs.push(format!("orig_profile_json={}", urlencoding::encode(qp)));
    }
    if let Some(ow) = dedup.orig_width {
        qs.push(format!("orig_width={ow}"));
    }
    if let Some(oh) = dedup.orig_height {
        qs.push(format!("orig_height={oh}"));
    }
    if dedup.ai_recommended {
        qs.push("ai_recommended=true".into());
    }
    if let Some(vf) = dedup.video_fingerprint_json {
        if vf.len() <= 2400 {
            qs.push(format!("video_fingerprint_json={}", urlencoding::encode(vf)));
        }
    }
    if let Some(af) = dedup.audio_fingerprint_json {
        if af.len() <= 600 {
            qs.push(format!("audio_fingerprint_json={}", urlencoding::encode(af)));
        }
    }
    let path = format!("/api/attachments?{}", qs.join("&"));
    let (_, body) = platform::http_request("POST", &path, None, Some((bytes, content_type))).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

pub struct SmartUploadOutcome {
    pub download_url: String,
    pub ref_id: String,
    pub key_b64: String,
    pub nonce_b64: String,
    pub availability_state: String,
    pub deduplicated: bool,
    pub dedup_match_kind: String,
    pub needs_sender_ai: bool,
}

fn register_sent_media_in_local_dedup(
    ref_id: &str,
    download_url: &str,
    key_b64: &str,
    nonce_b64: &str,
    payload: &crate::storage::media_opt::OptimizedMediaPayload,
    is_sender_orig_local: bool,
) {
    if ref_id.is_empty() {
        return;
    }
    let entry = json!({
        "refId": ref_id,
        "url": download_url,
        "keyB64": key_b64,
        "nonceB64": nonce_b64,
        "mime": payload.mime,
        "fileName": payload.file_name,
        "mediaKind": payload.media_kind,
        "sha256": payload.sha256_plain,
        "origBlake3": payload.orig_blake3.as_deref().unwrap_or(""),
        "wireBlake3": if !payload.bytes.is_empty() {
            chatw_media_core::dedup::hash_bytes_exact(&payload.bytes).blake3_hex
        } else {
            payload.orig_blake3.clone().unwrap_or_default()
        },
        "pdqHex": payload.pdq_hex.as_deref().unwrap_or(""),
        "pdqQuality": payload.pdq_quality.unwrap_or(0),
        "captionBandHash": payload.caption_band_hash.map(|v| v.to_string()).unwrap_or_else(|| "0".into()),
        "colorMomentHash": payload.color_moment_hash.map(|v| v.to_string()).unwrap_or_else(|| "0".into()),
        "aspectRatioMilli": payload.aspect_ratio_milli.unwrap_or(0),
        "width": payload.width.unwrap_or(0),
        "height": payload.height.unwrap_or(0),
        "origW": payload.target_width.or(payload.width).unwrap_or(0),
        "origH": payload.target_height.or(payload.height).unwrap_or(0),
        "qp": payload.orig_profile_json.as_deref().unwrap_or(""),
        "isAiRecovery": payload.ai_recommended || payload.orig_profile_json.is_some(),
        "isSenderOrigLocal": is_sender_orig_local,
    });
    if let Ok(js_obj) = serde_json::to_string(&entry) {
        let _ = dioxus::document::eval(&format!(
            "(() => {{ if (window.__chatwDedupLocal && window.__chatwDedupLocal.registerMedia) window.__chatwDedupLocal.registerMedia({js_obj}); }})()"
        ));
    }
}

/// Unified 4-Tier Hyper-Dedup Pre-Flight Claim + FastCDC + Convergent MLE Upload Engine (Spec 111).
/// - Never creates duplicate physical files when copying/pasting across chats OR when multiple users send identical
///   files/memes/videos/audio!
/// - Preserves canonical `orig_profile_json` and triggers sender+receiver AI sharpening on copied blurry images!
pub async fn upload_or_claim_media_for_scope(
    scope_type: &str,
    scope_id: Option<&str>,
    is_e2ee: bool,
    payload: &mut crate::storage::media_opt::OptimizedMediaPayload,
) -> Result<SmartUploadOutcome, String> {
    // STEP 1: Zero-upload Authorized Cross-Chat Copy Claim (`from_ref_id`, DM↔DM / DM↔Group / Group↔Group, even blurry copies!)
    if let Some(src_ref_id) = payload.copied_from_ref_id.clone() {
        let claim_body = json!({
            "mode": "from_ref_id",
            "from_ref_id": src_ref_id,
            "scope_type": scope_type,
            "scope_id": scope_id,
            "file_name": payload.file_name,
            "mime_type": payload.mime,
            "sha256_plain": payload.sha256_plain,
        });
        if let Ok(resp) = claim_dedup(claim_body).await {
            if resp["claimed"].as_bool() == Some(true) {
                if let Some(qp) = resp["orig_profile_json"].as_str().filter(|s| !s.is_empty()) {
                    payload.orig_profile_json = Some(qp.to_string());
                }
                if let Some(ow) = resp["orig_width"].as_u64().filter(|&n| n > 0) {
                    payload.target_width = Some(ow as u32);
                }
                if let Some(oh) = resp["orig_height"].as_u64().filter(|&n| n > 0) {
                    payload.target_height = Some(oh as u32);
                }
                if let Some(sz) = resp["file_size"].as_u64().filter(|&n| n > 0) {
                    payload.optimized_size = sz as usize;
                }
                let url = resp["download_url"].as_str().unwrap_or_default().to_string();
                let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
                let key_b64 = payload.copied_key_b64.clone().unwrap_or_default();
                let nonce_b64 = payload.copied_nonce_b64.clone().unwrap_or_default();
                let ai_rec = resp["ai_recommended"].as_bool().unwrap_or(false) || payload.orig_profile_json.is_some();
                let needs_sender_ai = payload.media_kind == "image" && (payload.needs_sender_ai || ai_rec);
                register_sent_media_in_local_dedup(&rid, &url, &key_b64, &nonce_b64, payload, !needs_sender_ai);
                return Ok(SmartUploadOutcome {
                    download_url: url,
                    ref_id: rid,
                    key_b64,
                    nonce_b64,
                    availability_state: resp["availability_state"].as_str().unwrap_or("server_durable").to_string(),
                    deduplicated: true,
                    dedup_match_kind: resp["dedup_match_kind"].as_str().unwrap_or("authorized_cross_chat_ref").to_string(),
                    needs_sender_ai,
                });
            }
        }
    }

    if payload.bytes.is_empty() {
        if let Some(ref stale_ref_id) = payload.copied_from_ref_id {
            crate::storage::media_opt::evict_local_dedup_entry(stale_ref_id);
        }
        let mut recovered = None;
        if let Some(ref preview_url) = payload.instant_preview_url {
            match crate::storage::media_opt::reencode_from_preview_url(preview_url).await {
                Ok(fresh) if !fresh.bytes.is_empty() => {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::warn_1(
                        &format!(
                            "[chatw] stale local dedup claim miss ({:?}); re-encoded {} bytes for upload",
                            payload.copied_from_ref_id,
                            fresh.bytes.len()
                        )
                        .into(),
                    );
                    recovered = Some(fresh);
                }
                Ok(_) => {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::warn_1(
                        &"[chatw] reencode_from_preview_url returned empty bytes".into(),
                    );
                }
                Err(e) => {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::warn_1(
                        &format!("[chatw] reencode_from_preview_url failed: {e}").into(),
                    );
                    let _ = e;
                }
            }
        }
        match recovered {
            Some(fresh) => {
                *payload = fresh;
            }
            None => {
                if let Some(ref stale_ref_id) = payload.copied_from_ref_id {
                    return Err(format!("empty_payload_after_claim_miss:{stale_ref_id}"));
                }
                return Err("empty_payload_after_claim_miss".into());
            }
        }
    }

    // STEP 2A: E2EE DM Path -> Zero-Knowledge Convergent MLE + Proof-of-Ownership Claim (`0 bytes` upload across users!)
    if is_e2ee {
        let sealed = crate::crypto::blob::seal_convergent(&payload.bytes)?;
        let cipher_hashes = chatw_media_core::dedup::hash_bytes_exact(&sealed.cipher);
        let pow_hex = chatw_media_core::dedup::compute_pow_digest_hex(&sealed.cipher);

        let claim_body = json!({
            "mode": "convergent_cipher_pow",
            "scope_type": scope_type,
            "scope_id": scope_id,
            "blake3_hex": cipher_hashes.blake3_hex,
            "sha256_hex": cipher_hashes.sha256_hex,
            "sha256_plain": payload.sha256_plain,
            "pow_digest_hex": pow_hex,
            "byte_size": sealed.cipher.len() as i64,
            "file_name": payload.file_name,
            "mime_type": payload.mime,
        });
        if let Ok(resp) = claim_dedup(claim_body).await {
            if resp["claimed"].as_bool() == Some(true) {
                if let Some(qp) = resp["orig_profile_json"].as_str().filter(|s| !s.is_empty()) {
                    payload.orig_profile_json = Some(qp.to_string());
                }
                if let Some(ow) = resp["orig_width"].as_u64().filter(|&n| n > 0) {
                    payload.target_width = Some(ow as u32);
                }
                if let Some(oh) = resp["orig_height"].as_u64().filter(|&n| n > 0) {
                    payload.target_height = Some(oh as u32);
                }
                let url = resp["download_url"].as_str().unwrap_or_default().to_string();
                let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
                let srv_orig_w = resp["orig_width"].as_u64().unwrap_or(0) as u32;
                let incoming_was_blurry = payload.media_kind == "image"
                    && (payload.needs_sender_ai
                        || (srv_orig_w > 0 && payload.width.unwrap_or(srv_orig_w) < srv_orig_w));
                register_sent_media_in_local_dedup(&rid, &url, &sealed.key_b64, &sealed.nonce_b64, payload, !incoming_was_blurry);
                return Ok(SmartUploadOutcome {
                    download_url: url,
                    ref_id: rid,
                    key_b64: sealed.key_b64,
                    nonce_b64: sealed.nonce_b64,
                    availability_state: resp["availability_state"].as_str().unwrap_or("server_durable").to_string(),
                    deduplicated: true,
                    dedup_match_kind: "convergent_mle_pow".into(),
                    needs_sender_ai: incoming_was_blurry,
                });
            }
        }

        // Fresh blind ciphertext upload (server stores `pow_digest` + `orig_profile_json` for future convergent/cross-chat reuse!)
        let dmeta = DedupUploadMeta {
            orig_blake3: payload.orig_blake3.as_deref(),
            orig_profile_json: payload.orig_profile_json.as_deref(),
            orig_width: payload.target_width.or(payload.width),
            orig_height: payload.target_height.or(payload.height),
            ai_recommended: payload.ai_recommended || payload.orig_profile_json.is_some(),
            ..Default::default()
        };
        let resp = upload_attachment_with_dedup(
            sealed.cipher,
            "application/octet-stream",
            scope_type,
            scope_id,
            Some(&payload.file_name),
            Some(&payload.mime),
            Some(&payload.sha256_plain),
            true,
            true,
            &dmeta,
        )
        .await
        .map_err(|e| e.code)?;

        let url = resp["download_url"].as_str().unwrap_or_default().to_string();
        let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
        let deduped = resp["deduplicated"].as_bool().unwrap_or(false);
        register_sent_media_in_local_dedup(&rid, &url, &sealed.key_b64, &sealed.nonce_b64, payload, !payload.needs_sender_ai);
        return Ok(SmartUploadOutcome {
            download_url: url,
            ref_id: rid,
            key_b64: sealed.key_b64,
            nonce_b64: sealed.nonce_b64,
            availability_state: resp["availability_state"].as_str().unwrap_or("server_durable").to_string(),
            deduplicated: deduped,
            dedup_match_kind: resp["dedup_match_kind"].as_str().unwrap_or("none").to_string(),
            needs_sender_ai: payload.needs_sender_ai,
        });
    }

    // STEP 2B: Shared Non-E2EE Path (Group / Channel) -> 4-Tier Zero-Upload Claim!
    let wire_hashes = chatw_media_core::dedup::hash_bytes_exact(&payload.bytes);
    let vf_val = payload.video_fingerprint_json.as_deref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let af_val = payload.audio_fingerprint_json.as_deref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let claim_body = json!({
        "mode": "shared_cas_or_perceptual",
        "scope_type": scope_type,
        "scope_id": scope_id,
        "blake3_hex": wire_hashes.blake3_hex,
        "orig_blake3_hex": payload.orig_blake3,
        "sha256_hex": wire_hashes.sha256_hex,
        "sha256_plain": payload.sha256_plain,
        "perceptual_hash_hex": payload.pdq_hex,
        "perceptual_quality": payload.pdq_quality,
        "caption_band_hash": payload.caption_band_hash,
        "color_moment_hash": payload.color_moment_hash,
        "aspect_ratio_milli": payload.aspect_ratio_milli,
        "width": payload.target_width.or(payload.width),
        "height": payload.target_height.or(payload.height),
        "video_fingerprint": vf_val,
        "audio_fingerprint": af_val,
        "file_name": payload.file_name,
        "mime_type": payload.mime,
    });
    if let Ok(resp) = claim_dedup(claim_body).await {
        if resp["claimed"].as_bool() == Some(true) {
            if let Some(qp) = resp["orig_profile_json"].as_str().filter(|s| !s.is_empty()) {
                payload.orig_profile_json = Some(qp.to_string());
            }
            if let Some(ow) = resp["orig_width"].as_u64().filter(|&n| n > 0) {
                payload.target_width = Some(ow as u32);
            }
            if let Some(oh) = resp["orig_height"].as_u64().filter(|&n| n > 0) {
                payload.target_height = Some(oh as u32);
            }
            let url = resp["download_url"].as_str().unwrap_or_default().to_string();
            let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
            let srv_orig_w = resp["orig_width"].as_u64().unwrap_or(0) as u32;
            let srv_ai = resp["ai_recommended"].as_bool().unwrap_or(false) || payload.orig_profile_json.is_some();
            // If user sent a blurry/downscaled copy (smaller than canonical orig_width) -> auto-sharpen sender TOO!
            let needs_sender_ai = payload.media_kind == "image"
                && srv_ai
                && (payload.needs_sender_ai || (srv_orig_w > 0 && payload.width.unwrap_or(srv_orig_w) < srv_orig_w));
            register_sent_media_in_local_dedup(&rid, &url, "", "", payload, !needs_sender_ai);
            return Ok(SmartUploadOutcome {
                download_url: url,
                ref_id: rid,
                key_b64: String::new(),
                nonce_b64: String::new(),
                availability_state: "server_durable".into(),
                deduplicated: true,
                dedup_match_kind: resp["dedup_match_kind"].as_str().unwrap_or("exact_shared_cas").to_string(),
                needs_sender_ai,
            });
        }
    }

    // STEP 3: Tier 2 FastCDC v2020 Sub-File Content-Defined Chunking for large non-E2EE files/docs/videos (>= 256 KiB)
    if payload.bytes.len() >= 256 * 1024 && payload.media_kind != "image" {
        let chunks = chatw_media_core::dedup::fastcdc_chunk_bytes(&payload.bytes);
        if chunks.len() >= 2 {
            let chunk_specs: Vec<serde_json::Value> = chunks
                .iter()
                .enumerate()
                .map(|(i, c)| json!({ "index": i as i32, "offset": c.offset as i64, "length": c.length as i32, "blake3_hex": c.blake3_hex }))
                .collect();
            if let Ok(plan) = cdc_plan(json!({ "scope_type": scope_type, "scope_id": scope_id, "chunks": chunk_specs })).await {
                if let Some(missing_arr) = plan["missing_indices"].as_array() {
                    let mut all_ok = true;
                    for idx_val in missing_arr {
                        if let Some(idx) = idx_val.as_u64().map(|n| n as usize) {
                            if let Some(c) = chunks.get(idx) {
                                let s = c.offset as usize;
                                let e = (s + c.length as usize).min(payload.bytes.len());
                                if cdc_put_chunk(&c.blake3_hex, payload.bytes[s..e].to_vec()).await.is_err() {
                                    all_ok = false;
                                    break;
                                }
                            }
                        }
                    }
                    if all_ok {
                        let pow_hex = chatw_media_core::dedup::compute_pow_digest_hex(&payload.bytes);
                        if let Ok(resp) = cdc_commit(json!({
                            "scope_type": scope_type,
                            "scope_id": scope_id,
                            "file_name": payload.file_name,
                            "mime_type": payload.mime,
                            "is_e2ee": false,
                            "total_blake3_hex": wire_hashes.blake3_hex,
                            "total_sha256_hex": wire_hashes.sha256_hex,
                            "sha256_plain": payload.sha256_plain,
                            "pow_digest_hex": pow_hex,
                            "chunks": chunk_specs,
                            "video_fingerprint": vf_val,
                            "audio_fingerprint": af_val,
                        }))
                        .await
                        {
                            let url = resp["download_url"].as_str().unwrap_or_default().to_string();
                            let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
                            register_sent_media_in_local_dedup(&rid, &url, "", "", payload, true);
                            return Ok(SmartUploadOutcome {
                                download_url: url,
                                ref_id: rid,
                                key_b64: String::new(),
                                nonce_b64: String::new(),
                                availability_state: "server_durable".into(),
                                deduplicated: resp["deduplicated"].as_bool().unwrap_or(false),
                                dedup_match_kind: resp["dedup_match_kind"].as_str().unwrap_or("fastcdc_subfile_chunks").to_string(),
                                needs_sender_ai: false,
                            });
                        }
                    }
                }
            }
        }
    }

    // STEP 4: Fresh Upload with Full Hyper-Dedup Metadata (indexes MIH 16-band PDQ + orig_blake3 + PoW + orig_profile_json!)
    let dmeta = DedupUploadMeta {
        orig_blake3: payload.orig_blake3.as_deref(),
        pdq_hex: payload.pdq_hex.as_deref(),
        pdq_quality: payload.pdq_quality,
        caption_band_hash: payload.caption_band_hash,
        color_moment_hash: payload.color_moment_hash,
        aspect_ratio_milli: payload.aspect_ratio_milli,
        orig_profile_json: payload.orig_profile_json.as_deref(),
        orig_width: payload.target_width.or(payload.width),
        orig_height: payload.target_height.or(payload.height),
        ai_recommended: payload.ai_recommended || payload.orig_profile_json.is_some(),
        video_fingerprint_json: payload.video_fingerprint_json.as_deref(),
        audio_fingerprint_json: payload.audio_fingerprint_json.as_deref(),
    };
    let resp = upload_attachment_with_dedup(
        payload.bytes.clone(),
        &payload.mime,
        scope_type,
        scope_id,
        Some(&payload.file_name),
        Some(&payload.mime),
        Some(&payload.sha256_plain),
        false,
        false,
        &dmeta,
    )
    .await
    .map_err(|e| e.code)?;

    let url = resp["download_url"].as_str().unwrap_or_default().to_string();
    let rid = resp["ref_id"].as_str().unwrap_or_default().to_string();
    register_sent_media_in_local_dedup(&rid, &url, "", "", payload, !payload.needs_sender_ai);
    Ok(SmartUploadOutcome {
        download_url: url,
        ref_id: rid,
        key_b64: String::new(),
        nonce_b64: String::new(),
        availability_state: "server_durable".into(),
        deduplicated: resp["deduplicated"].as_bool().unwrap_or(false),
        dedup_match_kind: resp["dedup_match_kind"].as_str().unwrap_or("none").to_string(),
        needs_sender_ai: payload.needs_sender_ai,
    })
}

pub async fn register_device(
    client_device_id: &str,
    platform_kind: &str,
    device_name: &str,
    ecdh_pub_b64: Option<&str>,
    ecdsa_pub_b64: Option<&str>,
    fingerprint_b64: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/devices/register",
        Some(json!({
            "client_device_id": client_device_id,
            "platform": platform_kind,
            "device_name": device_name,
            "ecdh_public_b64": ecdh_pub_b64,
            "ecdsa_public_b64": ecdsa_pub_b64,
            "key_fingerprint_b64": fingerprint_b64,
        })),
    )
    .await
}

pub async fn current_device_status() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/devices/current", None).await
}

pub async fn list_devices() -> Result<DevicesListResponse, ApiError> {
    req_json("GET", "/api/devices", None).await
}

pub async fn approve_device_request(
    request_id: &str,
    verification_code: Option<&str>,
    wrapped_sync_key: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/devices/requests/{}/approve", urlencoding::encode(request_id)),
        Some(json!({
            "verification_code": verification_code,
            "wrapped_sync_key": wrapped_sync_key,
        })),
    )
    .await
}

pub async fn reject_device_request(request_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/devices/requests/{}/reject", urlencoding::encode(request_id)),
        Some(json!({})),
    )
    .await
}

pub async fn revoke_device(device_id: &str) -> Result<serde_json::Value, ApiError> {
    req_json("DELETE", &format!("/api/devices/{}", urlencoding::encode(device_id)), None).await
}

pub async fn recover_lost_devices(code: Option<&str>, revoke_others: bool) -> Result<serde_json::Value, ApiError> {
    recover_lost_devices_with_proof(code, revoke_others, None, false).await
}

pub async fn recover_lost_devices_with_proof(
    code: Option<&str>,
    revoke_others: bool,
    recovery_proof_b64: Option<&str>,
    confirm_emergency_reset: bool,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/devices/recover-lost",
        Some(json!({
            "totp_or_backup_code": code,
            "revoke_unreachable_devices": revoke_others,
            "recovery_proof_b64": recovery_proof_b64,
            "confirm_emergency_reset": confirm_emergency_reset,
        })),
    )
    .await
}

pub async fn put_key_backup(
    wrapped_keys_b64: &str,
    kdf_salt_b64: &str,
    kdf_iterations: i32,
    key_fingerprint_b64: &str,
) -> Result<serde_json::Value, ApiError> {
    put_key_backup_v2(
        wrapped_keys_b64,
        kdf_salt_b64,
        "argon2id-v1",
        19_456,
        2,
        1,
        kdf_iterations,
        key_fingerprint_b64,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn put_key_backup_v2(
    wrapped_keys_b64: &str,
    kdf_salt_b64: &str,
    kdf_alg: &str,
    kdf_mem_kib: i32,
    kdf_time_cost: i32,
    kdf_parallelism: i32,
    kdf_iterations: i32,
    key_fingerprint_b64: &str,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "PUT",
        "/api/keys/backup",
        Some(json!({
            "wrapped_keys_b64": wrapped_keys_b64,
            "kdf_salt_b64": kdf_salt_b64,
            "kdf_alg": kdf_alg,
            "kdf_mem_kib": kdf_mem_kib,
            "kdf_time_cost": kdf_time_cost,
            "kdf_parallelism": kdf_parallelism,
            "kdf_iterations": kdf_iterations,
            "key_fingerprint_b64": key_fingerprint_b64,
        })),
    )
    .await
}

pub async fn get_key_backup() -> Result<serde_json::Value, ApiError> {
    req_json("GET", "/api/keys/backup", None).await
}

pub async fn delta_sync(dm_since: i64, group_since: i64, channel_since: i64) -> Result<serde_json::Value, ApiError> {
    delta_sync_with_limit(dm_since, group_since, channel_since, 250).await
}

pub async fn delta_sync_with_limit(
    dm_since: i64,
    group_since: i64,
    channel_since: i64,
    limit: i64,
) -> Result<serde_json::Value, ApiError> {
    let lim = limit.clamp(1, 500);
    req_json(
        "GET",
        &format!("/api/sync/delta?dm_since={dm_since}&group_since={group_since}&channel_since={channel_since}&limit={lim}"),
        None,
    )
    .await
}

pub async fn report_locations(items: Vec<serde_json::Value>) -> Result<serde_json::Value, ApiError> {
    req_json("PUT", "/api/storage/locations/report", Some(json!({ "items": items }))).await
}

pub async fn list_recovery_requests() -> Result<RecoveryListResponse, ApiError> {
    req_json("GET", "/api/recovery/requests", None).await
}

pub async fn create_recovery_request(ref_id: &str, preferred_transport: Option<&str>) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        "/api/recovery/requests",
        Some(json!({
            "ref_id": ref_id,
            "preferred_transport": preferred_transport,
        })),
    )
    .await
}

pub async fn respond_recovery_request(
    request_id: &str,
    action: &str,
    transport_mode: Option<&str>,
    relay_ref_id: Option<&str>,
    reason: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/recovery/requests/{}/respond", urlencoding::encode(request_id)),
        Some(json!({
            "action": action,
            "transport_mode": transport_mode,
            "relay_ref_id": relay_ref_id,
            "reason": reason,
        })),
    )
    .await
}

pub async fn complete_recovery_request(
    request_id: &str,
    sha256_hex: Option<&str>,
    local_tier: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    req_json(
        "POST",
        &format!("/api/recovery/requests/{}/complete", urlencoding::encode(request_id)),
        Some(json!({
            "sha256_hex": sha256_hex,
            "local_tier": local_tier,
        })),
    )
    .await
}
