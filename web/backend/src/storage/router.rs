use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
    media_opt::{MediaKind, optimize_chat_image, probe_image_dimensions, sniff_media_bytes},
    storage::{
        StorageAdapter, StorageRegistry, local_cas::LocalFileStorageAdapter,
        permissions::{authorize_read, authorize_write},
        sha256_bytes,
    },
};
use chatw_media_core::dedup::{
    AudioFingerprint, PdqHash256, VideoFingerprint, compute_pdq_hash_rgba, compute_pow_digest,
    evaluate_dedup_policy, hash_bytes_exact,
};
use crate::storage::dedup_engine;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/capabilities", get(handle_capabilities))
        .route("/policy", get(handle_get_policy).put(handle_update_policy))
        .route("/conversations/{peer_id}", get(handle_get_conv_policy).put(handle_update_conv_policy))
        .route("/uploads/init", post(handle_init_upload))
        .route("/uploads/{id}", get(handle_get_upload_status).delete(handle_abort_upload))
        .route(
            "/uploads/{id}/chunks/{index}",
            put(handle_upload_chunk).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        .route("/uploads/{id}/complete", post(handle_complete_upload))
        .route("/dedup/check", post(handle_dedup_check))
        .route("/dedup/claim", post(dedup_engine::handle_dedup_claim))
        .route("/dedup/cdc/plan", post(dedup_engine::handle_cdc_plan))
        .route(
            "/dedup/cdc/chunks/{blake3}",
            put(dedup_engine::handle_cdc_chunk_put).layer(DefaultBodyLimit::max(4 * 1024 * 1024)),
        )
        .route("/dedup/cdc/commit", post(dedup_engine::handle_cdc_commit))
        .route("/forward", post(handle_forward_reference))
        .route("/references/{id}/meta", get(handle_get_ref_meta))
        .route("/references/{id}/manifest", get(handle_get_manifest).post(handle_put_manifest))
        .route("/references/{id}/presign", post(handle_presign_ref))
        .route("/references/{id}", delete(handle_delete_reference))
        .route("/relay/{id}/ack", post(handle_relay_ack))
        .route("/locations/report", put(handle_report_locations))
        .route("/metrics", get(handle_storage_metrics))
        .route("/cleanup", post(handle_cleanup_relays))
        .route("/gc/incomplete", post(handle_gc_incomplete))
        .route("/migration/run", post(handle_run_migration))
}

#[derive(Deserialize, Default)]
pub struct UploadMetaQuery {
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub sha256_plain: Option<String>,
    pub sha256_cipher: Option<String>,
    pub is_e2ee: Option<bool>,
    pub preserve_original: Option<bool>,
    pub backup_dm: Option<bool>,
    pub forward_from_ref_id: Option<Uuid>,
    pub tier: Option<String>,
    pub orig_blake3: Option<String>,
    pub pdq_hex: Option<String>,
    pub pdq_quality: Option<u8>,
    pub caption_band_hash: Option<u64>,
    pub color_moment_hash: Option<u64>,
    pub aspect_ratio_milli: Option<u16>,
    pub orig_profile_json: Option<String>,
    pub orig_width: Option<i32>,
    pub orig_height: Option<i32>,
    pub ai_recommended: Option<bool>,
    pub video_fingerprint_json: Option<String>,
    pub audio_fingerprint_json: Option<String>,
}

/// Spec 115: Strictly block `.svg` / `.svgz` / `image/svg+xml` / XML-SVG payloads across all upload & dedup paths.
/// Only inspects raw bytes as SVG when trimmed content starts with `<` (after optional UTF-8 BOM/whitespace)
/// so binary raster images, video, archives, and E2EE ciphertext never suffer false positives.
pub fn is_blocked_svg_upload(
    file_name: Option<&str>,
    mime_type: Option<&str>,
    content_type: Option<&str>,
    bytes: Option<&[u8]>,
) -> bool {
    if let Some(name) = file_name {
        let lower = name.trim().to_ascii_lowercase();
        if lower.ends_with(".svg") || lower.ends_with(".svgz") {
            return true;
        }
    }
    for m in [mime_type, content_type].into_iter().flatten() {
        let lower = m.trim().to_ascii_lowercase();
        if lower.contains("svg") {
            return true;
        }
    }
    if let Some(raw) = bytes {
        let mut slice = raw;
        if slice.starts_with(&[0xEF, 0xBB, 0xBF]) {
            slice = &slice[3..];
        }
        while let Some((&first, rest)) = slice.split_first() {
            if first.is_ascii_whitespace() || first == 0 {
                slice = rest;
            } else {
                break;
            }
        }
        if slice.first() == Some(&b'<') {
            let inspect_len = slice.len().min(4096);
            let head = String::from_utf8_lossy(&slice[..inspect_len]).to_ascii_lowercase();
            let has_svg_tag = head.starts_with("<svg")
                && matches!(head.as_bytes().get(4), None | Some(b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/' | b':'));
            if has_svg_tag
                || head.contains("<svg ")
                || head.contains("<svg>")
                || head.contains("<svg\t")
                || head.contains("<svg\n")
                || head.contains("<svg\r")
                || head.contains("<svg/")
                || head.contains("<!doctype svg")
                || head.contains("http://www.w3.org/2000/svg")
                || head.contains("https://www.w3.org/2000/svg")
            {
                return true;
            }
        }
    }
    false
}

/// Shared ingestion helper used by BOTH legacy `/api/attachments` and new resumable chunked uploads.
pub async fn ingest_media_bytes(
    pool: &PgPool,
    storage: &StorageRegistry,
    owner_id: Uuid,
    device_id: Option<Uuid>,
    bytes: Vec<u8>,
    content_type_header: &str,
    meta: UploadMetaQuery,
) -> AppResult<serde_json::Value> {
    if bytes.is_empty() {
        return Err(AppError::BadRequest("empty_attachment"));
    }
    if is_blocked_svg_upload(
        meta.file_name.as_deref(),
        meta.mime_type.as_deref(),
        Some(content_type_header),
        Some(&bytes),
    ) {
        return Err(AppError::BadRequest("svg_upload_not_allowed"));
    }

    let scope_type = meta.scope_type.as_deref().unwrap_or("dm");
    let scope_id = meta.scope_id;
    authorize_write(pool, owner_id, scope_type, scope_id).await?;

    let is_e2ee = meta.is_e2ee.unwrap_or(scope_type == "dm" && content_type_header == "application/octet-stream");
    let preserve_original = meta.preserve_original.unwrap_or(true);
    let declared_name = meta
        .file_name
        .as_deref()
        .map(sanitize_filename)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "attachment.bin".to_string());

    let plain_sha = decode_hex_32(meta.sha256_plain.as_deref());
    let orig_b3_opt = decode_hex_32(meta.orig_blake3.as_deref());
    let exact_hashes = hash_bytes_exact(&bytes);
    let stored_sha_arr = sha256_bytes(&bytes);
    let stored_blake3_arr: [u8; 32] = blake3::hash(&bytes).into();
    let pow_digest_arr: [u8; 32] = compute_pow_digest(&bytes);
    let client_orig_profile = meta
        .orig_profile_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 512)
        .map(str::to_string);
    let client_ai_recommended = meta.ai_recommended.unwrap_or(client_orig_profile.is_some());
    let client_vf: Option<VideoFingerprint> = meta
        .video_fingerprint_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let client_af: Option<AudioFingerprint> = meta
        .audio_fingerprint_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    if let Some(expected_hex) = meta.sha256_cipher.as_deref() {
        let expected = decode_hex_32(Some(expected_hex)).ok_or(AppError::BadRequest("invalid_sha256_cipher"))?;
        if expected != stored_sha_arr {
            return Err(AppError::BadRequest("integrity_mismatch"));
        }
    }

    let sniffed = sniff_media_bytes(&bytes);
    let actual_mime = if is_e2ee {
        meta.mime_type
            .as_deref()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| if !content_type_header.is_empty() { content_type_header } else { "application/octet-stream" })
            .to_string()
    } else if !content_type_header.is_empty() && content_type_header != "application/octet-stream" {
        content_type_header.to_string()
    } else if sniffed.mime != "application/octet-stream" {
        sniffed.mime.to_string()
    } else {
        "application/octet-stream".into()
    };
    let actual_ext = if is_e2ee { "enc" } else { sniffed.ext };

    let policy_row = sqlx::query(
        "SELECT dm_relay_ttl_hours, dm_encrypted_backup, COALESCE(server_durable_dm, TRUE) AS server_durable_dm FROM device_storage_policies WHERE user_id = $1",
    )
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;
    let relay_ttl_hours: i32 = policy_row.as_ref().and_then(|r| r.try_get("dm_relay_ttl_hours").ok()).unwrap_or(168);
    let default_dm_backup: bool = policy_row.as_ref().and_then(|r| r.try_get("dm_encrypted_backup").ok()).unwrap_or(false);
    let server_durable_dm: bool = policy_row.as_ref().and_then(|r| r.try_get("server_durable_dm").ok()).unwrap_or(true);

    // Server-First storage routing:
    // - When S3/MinIO is configured and ready, durable objects route directly to `"s3_ready"`.
    // - Otherwise durable objects use local CAS `"local_cas"` / `"encrypted_backup"`.
    // - DMs default to durable server storage of E2EE ciphertext (`server_durable_dm = TRUE`) so users
    //   never lose history across devices while preserving zero-knowledge privacy.
    let s3_ready = storage.s3.config().is_ready();
    let durable_backend = if s3_ready { "s3_ready" } else { "local_cas" };
    let wants_backup = meta.backup_dm.unwrap_or(default_dm_backup);
    let explicit_tier = meta.tier.as_deref();
    let (storage_tier, availability_state, relay_expires_at, backend_kind) = match scope_type {
        _ if explicit_tier == Some("temp_relay") => (
            "temp_relay",
            "pending_delivery",
            Some(chrono::Utc::now() + chrono::Duration::hours(relay_ttl_hours as i64)),
            "temp_relay",
        ),
        "group" | "channel" | "avatar" | "banner" | "library" | "sticker" => (
            "server_durable",
            "server_durable",
            None,
            durable_backend,
        ),
        "dm" if wants_backup => (
            "encrypted_backup",
            "backed_up_encrypted",
            None,
            if s3_ready { "s3_ready" } else { "encrypted_backup" },
        ),
        "dm" if server_durable_dm => (
            "server_durable",
            "server_durable",
            None,
            durable_backend,
        ),
        _ => (
            "temp_relay",
            "pending_delivery",
            Some(chrono::Utc::now() + chrono::Duration::hours(relay_ttl_hours as i64)),
            "temp_relay",
        ),
    };

    // Privacy-safe deduplication policy (Section XIX.6):
    // - Non-E2EE shared scopes (`group`/`channel`/`library`/`sticker`): shared CAS + PDQ perceptual index allowed.
    // - Private E2EE content: STRICTLY isolated per owner/scope; ZERO cross-user or global plaintext lookup!
    let sha_hex = exact_hashes.sha256_hex.clone();
    let blake3_hex = exact_hashes.blake3_hex.clone();
    let dedup_decision = evaluate_dedup_policy(
        scope_type,
        &owner_id.to_string(),
        scope_id.map(|u| u.to_string()).as_deref(),
        is_e2ee,
        meta.forward_from_ref_id.is_some(),
        &blake3_hex,
    );
    let cas_scope = if storage_tier == "temp_relay" {
        format!("relay:{}:{}", owner_id, Uuid::new_v4())
    } else if storage_tier == "encrypted_backup" {
        format!("backup:{}:{}", owner_id, blake3_hex)
    } else {
        dedup_decision.cas_scope_key.clone()
    };

    let client_pdq = meta.pdq_hex.as_deref().and_then(|hex| {
        PdqHash256::from_hex_full(
            hex,
            meta.pdq_quality.unwrap_or(50),
            meta.caption_band_hash.unwrap_or(0),
            meta.color_moment_hash.unwrap_or(0),
            meta.aspect_ratio_milli.unwrap_or(0),
        )
    });

    let (dims_w, dims_h, pdq_opt) = if !is_e2ee && matches!(sniffed.kind, MediaKind::Image) {
        if let Some(cpdq) = client_pdq {
            let (w, h) = probe_image_dimensions(&bytes)
                .map(|(w, h)| (Some(w as i32), Some(h as i32)))
                .unwrap_or((meta.orig_width, meta.orig_height));
            (w, h, Some(cpdq))
        } else if dedup_decision.allow_perceptual_lookup {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pdq = compute_pdq_hash_rgba(rgba.as_raw(), w, h);
                (Some(w as i32), Some(h as i32), Some(pdq))
            } else {
                let (w, h) = probe_image_dimensions(&bytes).map(|(w, h)| (Some(w as i32), Some(h as i32))).unwrap_or((None, None));
                (w, h, None)
            }
        } else {
            let (w, h) = probe_image_dimensions(&bytes).map(|(w, h)| (Some(w as i32), Some(h as i32))).unwrap_or((None, None));
            (w, h, None)
        }
    } else {
        (meta.orig_width, meta.orig_height, None)
    };

    let storage_key = match backend_kind {
        "temp_relay" => format!("{}.enc", Uuid::new_v4()),
        "encrypted_backup" => format!("{}/{}.enc", owner_id, sha_hex),
        "s3_ready" if is_e2ee => format!("e2ee/{}/{}/{}.enc", &blake3_hex[0..2], &blake3_hex[2..4], Uuid::new_v4()),
        "s3_ready" => format!("cas/{}", LocalFileStorageAdapter::cas_key_for_sha(&stored_blake3_arr, actual_ext)),
        _ => LocalFileStorageAdapter::cas_key_for_sha(&stored_sha_arr, actual_ext),
    };

    let mut tx = pool.begin().await?;
    let existing_obj = sqlx::query(
        "SELECT id, storage_key, storage_backend FROM media_objects WHERE cas_scope = $1 AND variant_kind = 'original'",
    )
    .bind(&cas_scope)
    .fetch_optional(&mut *tx)
    .await?;

    let mut dedup_match_kind: &'static str = "none";
    let mut matched_existing: Option<(Uuid, String, String, &'static str)> = None;

    if let Some(existing_row) = existing_obj {
        let oid: Uuid = existing_row.try_get("id")?;
        let ex_key: String = existing_row.try_get("storage_key")?;
        let ex_backend: String = existing_row.try_get("storage_backend")?;
        matched_existing = Some((oid, ex_key, ex_backend, "exact_cas_scope"));
    }

    // Fallback 1: If not found by cas_scope, check exact wire/original BLAKE3 or SHA-256 in shared dedup index
    if matched_existing.is_none() && !is_e2ee && dedup_decision.allow_exact_global_lookup {
        if let Some(row) = sqlx::query(
            r#"
            SELECT d.object_id, o.storage_key, o.storage_backend
            FROM media_dedup_indexes d
            JOIN media_objects o ON o.id = d.object_id
            WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE
              AND (d.blake3 = $1 OR d.sha256 = $2 OR ($3::bytea IS NOT NULL AND (d.orig_blake3 = $3 OR d.blake3 = $3)))
            LIMIT 1
            "#,
        )
        .bind(&stored_blake3_arr[..])
        .bind(&stored_sha_arr[..])
        .bind(orig_b3_opt.as_ref().map(|x| &x[..]))
        .fetch_optional(&mut *tx)
        .await?
        {
            let oid: Uuid = row.try_get("object_id")?;
            let ex_key: String = row.try_get("storage_key")?;
            let ex_backend: String = row.try_get("storage_backend")?;
            matched_existing = Some((oid, ex_key, ex_backend, "exact_shared_hash"));
        }
    }

    // Fallback 1B: Convergent MLE Ciphertext Match for E2EE DMs (blind zero-knowledge dedup by exact ciphertext + PoW!)
    if matched_existing.is_none() && is_e2ee && storage_tier != "temp_relay" {
        if let Some(row) = sqlx::query(
            r#"
            SELECT id, storage_key, storage_backend, is_cdc_assembled
            FROM media_objects
            WHERE blake3 = $1 AND pow_digest = $2 AND variant_kind = 'original'
            ORDER BY ref_count DESC
            LIMIT 1
            "#,
        )
        .bind(&stored_blake3_arr[..])
        .bind(&pow_digest_arr[..])
        .fetch_optional(&mut *tx)
        .await?
        {
            let oid: Uuid = row.try_get("id")?;
            let ex_key: String = row.try_get("storage_key")?;
            let ex_backend: String = row.try_get("storage_backend")?;
            let is_cdc: bool = row.try_get("is_cdc_assembled").unwrap_or(false);
            if dedup_engine::physical_object_available(storage, &ex_backend, &ex_key, is_cdc).await {
                matched_existing = Some((oid, ex_key, ex_backend, "convergent_mle_pow"));
            }
        }
    }

    // Fallback 2: O(1) 16-Band MIH Meta PDQ 256-bit DCT match for memes & re-encoded/blurry images (Triple Guard!)
    if matched_existing.is_none() && !is_e2ee && dedup_decision.allow_perceptual_lookup {
        if let Some(ref incoming_pdq) = pdq_opt {
            let incoming_px = (dims_w.unwrap_or(0).max(0) as u64) * (dims_h.unwrap_or(0).max(0) as u64);
            if let Some(hit) = dedup_engine::find_pdq_mih_match(
                pool,
                storage,
                "shared_non_e2ee",
                incoming_pdq,
                incoming_px,
            )
            .await?
            {
                matched_existing = Some((hit.object_id, hit.storage_key, hit.storage_backend, "perceptual_pdq_mih_match"));
            }
        }
    }

    let (original_obj_id, deduplicated) = if let Some((oid, ex_key, ex_backend, kind)) = matched_existing {
        dedup_match_kind = kind;
        if ex_backend == "local_cas" && !storage.local_cas.object_exists(&ex_key).await.unwrap_or(false) {
            storage.local_cas.put_object(&ex_key, bytes.clone(), &actual_mime, None).await
                .map_err(|_| AppError::Internal)?;
        } else if ex_backend == "s3_ready" && !storage.s3.object_exists(&ex_key).await.unwrap_or(false) {
            storage.s3.put_object(&ex_key, bytes.clone(), &actual_mime, None).await
                .map_err(|_| AppError::Internal)?;
        }
        sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
            .bind(oid)
            .execute(&mut *tx)
            .await?;
        (oid, true)
    } else {
        match backend_kind {
            "s3_ready" => {
                storage.s3.put_object(&storage_key, bytes.clone(), &actual_mime, None).await
                    .map_err(|_| AppError::Internal)?;
            }
            "temp_relay" => {
                storage.temp_relay.put_object(&storage_key, bytes.clone(), &actual_mime, Some((relay_ttl_hours as u64) * 3600)).await
                    .map_err(|_| AppError::Internal)?;
            }
            "encrypted_backup" => {
                storage.encrypted_backup.put_object(&storage_key, bytes.clone(), &actual_mime, None).await
                    .map_err(|_| AppError::Internal)?;
            }
            _ => {
                storage.local_cas.put_object(&storage_key, bytes.clone(), &actual_mime, None).await
                    .map_err(|_| AppError::Internal)?;
            }
        }
        let oid = Uuid::new_v4();
        let orig_codec = if is_e2ee { "e2ee_ciphertext" } else { actual_ext };
        sqlx::query(
            r#"
            INSERT INTO media_objects
                (id, cas_scope, sha256, plain_sha256, blake3, pow_digest,
                 perceptual_hash, perceptual_quality, caption_band_hash, color_moment_hash, aspect_ratio_milli,
                 orig_profile_json, orig_width, orig_height, ai_recommended,
                 storage_backend, storage_key, mime_type, file_ext, compression_codec, compression_ratio,
                 byte_size, original_byte_size, width, height, is_animated, ref_count, variant_kind, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, 1.0, $21, $21, $22, $23, $24, 1, 'original', $25)
            "#,
        )
        .bind(oid)
        .bind(&cas_scope)
        .bind(&stored_sha_arr[..])
        .bind(plain_sha.as_ref().map(|p| &p[..]))
        .bind(&stored_blake3_arr[..])
        .bind(&pow_digest_arr[..])
        .bind(pdq_opt.as_ref().map(|p| &p.bytes[..]))
        .bind(pdq_opt.as_ref().map(|p| p.quality as i16))
        .bind(pdq_opt.as_ref().map(|p| p.caption_band_hash as i64))
        .bind(pdq_opt.as_ref().map(|p| p.color_moment_hash as i64))
        .bind(pdq_opt.as_ref().map(|p| p.aspect_ratio_milli as i16))
        .bind(&client_orig_profile)
        .bind(meta.orig_width)
        .bind(meta.orig_height)
        .bind(client_ai_recommended)
        .bind(backend_kind)
        .bind(&storage_key)
        .bind(&actual_mime)
        .bind(actual_ext)
        .bind(orig_codec)
        .bind(bytes.len() as i64)
        .bind(dims_w)
        .bind(dims_h)
        .bind(matches!(sniffed.kind, MediaKind::AnimatedImage))
        .bind(relay_expires_at)
        .execute(&mut *tx)
        .await?;

        // Strictly populate shared dedup index + O(1) 16-Band MIH buckets ONLY for non-E2EE shared scopes (`is_e2ee = FALSE`)
        if !is_e2ee && dedup_decision.allow_exact_global_lookup {
            let vf_json = client_vf.as_ref().and_then(|v| serde_json::to_value(v).ok());
            let af_json = client_af.as_ref().and_then(|a| serde_json::to_value(a).ok());
            let _ = sqlx::query(
                r#"
                INSERT INTO media_dedup_indexes
                    (object_id, scope_class, is_e2ee, blake3, orig_blake3, sha256,
                     perceptual_hash, perceptual_quality, caption_band_hash, color_moment_hash, aspect_ratio_milli,
                     video_fingerprint, audio_fingerprint,
                     width, height, orig_width, orig_height, orig_profile_json, ai_recommended, byte_size)
                VALUES ($1, 'shared_non_e2ee', FALSE, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                ON CONFLICT (scope_class, blake3) DO NOTHING
                "#,
            )
            .bind(oid)
            .bind(&stored_blake3_arr[..])
            .bind(orig_b3_opt.as_ref().map(|x| &x[..]))
            .bind(&stored_sha_arr[..])
            .bind(pdq_opt.as_ref().map(|p| &p.bytes[..]))
            .bind(pdq_opt.as_ref().map(|p| p.quality as i16))
            .bind(pdq_opt.as_ref().map(|p| p.caption_band_hash as i64))
            .bind(pdq_opt.as_ref().map(|p| p.color_moment_hash as i64))
            .bind(pdq_opt.as_ref().map(|p| p.aspect_ratio_milli as i16))
            .bind(&vf_json)
            .bind(&af_json)
            .bind(dims_w)
            .bind(dims_h)
            .bind(meta.orig_width)
            .bind(meta.orig_height)
            .bind(&client_orig_profile)
            .bind(client_ai_recommended)
            .bind(bytes.len() as i64)
            .execute(&mut *tx)
            .await;

            if let Some(ref pdq) = pdq_opt {
                let _ = dedup_engine::insert_pdq_mih_buckets(&mut tx, oid, "shared_non_e2ee", pdq).await;
            }
            if let Some(anchor) = client_vf.as_ref().and_then(|v| v.anchor_pdq()) {
                let _ = dedup_engine::insert_pdq_mih_buckets(&mut tx, oid, "video_shared", &anchor).await;
            }
        }
        (oid, false)
    };

    // For non-E2EE images in durable scopes (group/channel/library), also produce genuine Adaptive Ultra AVIF + AVIF thumbnail.
    let mut opt_obj_id: Option<Uuid> = None;
    let mut thumb_obj_id: Option<Uuid> = None;

    if deduplicated {
        let variants = sqlx::query(
            "SELECT id, variant_kind FROM media_objects WHERE variant_of = $1",
        )
        .bind(original_obj_id)
        .fetch_all(&mut *tx)
        .await?;
        for v in variants {
            let vid: Uuid = v.try_get("id")?;
            let vkind: String = v.try_get("variant_kind")?;
            if vkind == "optimized" && opt_obj_id.is_none() {
                opt_obj_id = Some(vid);
                let _ = sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                    .bind(vid)
                    .execute(&mut *tx)
                    .await;
            } else if (vkind == "thumb" || vkind == "thumbnail") && thumb_obj_id.is_none() {
                thumb_obj_id = Some(vid);
                let _ = sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                    .bind(vid)
                    .execute(&mut *tx)
                    .await;
            }
        }
    } else if false
        && !is_e2ee
        && !preserve_original
        && matches!(backend_kind, "local_cas" | "s3_ready")
        && matches!(sniffed.kind, MediaKind::Image | MediaKind::AnimatedImage)
    {
        // Spec 158: Active images stay untouched on day 0 and compress automatically after 15 days idle via `compress_inactive_images_15d`.
        if let Ok(opt) = optimize_chat_image(&bytes) {
            if opt.bytes.len() < bytes.len() {
                let opt_sha = sha256_bytes(&opt.bytes);
                let opt_b3: [u8; 32] = blake3::hash(&opt.bytes).into();
                let opt_key = if backend_kind == "s3_ready" {
                    format!("cas/{}", LocalFileStorageAdapter::cas_key_for_sha(&opt_b3, opt.ext))
                } else {
                    LocalFileStorageAdapter::cas_key_for_sha(&opt_sha, opt.ext)
                };
                let opt_scope = format!("{cas_scope}:opt");
                let put_ok = if backend_kind == "s3_ready" {
                    storage.s3.put_object(&opt_key, opt.bytes.clone(), opt.mime, None).await.is_ok()
                } else {
                    storage.local_cas.put_object(&opt_key, opt.bytes.clone(), opt.mime, None).await.is_ok()
                };
                if put_ok {
                    let oid = sqlx::query_scalar::<_, Uuid>(
                        r#"
                        INSERT INTO media_objects
                            (id, cas_scope, sha256, blake3, storage_backend, storage_key, mime_type, file_ext,
                             compression_codec, compression_ratio, quality_ssim, media_class, ai_recommended,
                             byte_size, original_byte_size, width, height, is_animated, ref_count, variant_of, variant_kind)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, 1, $19, 'optimized')
                        ON CONFLICT (cas_scope, variant_kind) DO UPDATE SET ref_count = media_objects.ref_count + 1
                        RETURNING id
                        "#,
                    )
                    .bind(Uuid::new_v4())
                    .bind(&opt_scope)
                    .bind(&opt_sha[..])
                    .bind(&opt_b3[..])
                    .bind(backend_kind)
                    .bind(&opt_key)
                    .bind(opt.mime)
                    .bind(opt.ext)
                    .bind(opt.codec)
                    .bind(opt.compression_ratio)
                    .bind(opt.estimated_ssim)
                    .bind(&opt.media_class)
                    .bind(opt.ai_recommended)
                    .bind(opt.bytes.len() as i64)
                    .bind(bytes.len() as i64)
                    .bind(opt.width as i32)
                    .bind(opt.height as i32)
                    .bind(matches!(sniffed.kind, MediaKind::AnimatedImage))
                    .bind(original_obj_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    opt_obj_id = Some(oid);
                }
            }
            if let Some(tb) = opt.thumb_bytes {
                let tb_sha = sha256_bytes(&tb);
                let tb_b3: [u8; 32] = blake3::hash(&tb).into();
                let tb_key = if backend_kind == "s3_ready" {
                    format!("cas/{}", LocalFileStorageAdapter::cas_key_for_sha(&tb_b3, opt.thumb_ext))
                } else {
                    LocalFileStorageAdapter::cas_key_for_sha(&tb_sha, opt.thumb_ext)
                };
                let tb_scope = format!("{cas_scope}:thumb");
                let put_ok = if backend_kind == "s3_ready" {
                    storage.s3.put_object(&tb_key, tb.clone(), opt.thumb_mime, None).await.is_ok()
                } else {
                    storage.local_cas.put_object(&tb_key, tb.clone(), opt.thumb_mime, None).await.is_ok()
                };
                if put_ok {
                    let tid = sqlx::query_scalar::<_, Uuid>(
                        r#"
                        INSERT INTO media_objects
                            (id, cas_scope, sha256, blake3, storage_backend, storage_key, mime_type, file_ext,
                             compression_codec, byte_size, original_byte_size, width, height, is_animated, ref_count, variant_of, variant_kind)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'avif_thumb', $9, $10, 320, 320, FALSE, 1, $11, 'thumb')
                        ON CONFLICT (cas_scope, variant_kind) DO UPDATE SET ref_count = media_objects.ref_count + 1
                        RETURNING id
                        "#,
                    )
                    .bind(Uuid::new_v4())
                    .bind(&tb_scope)
                    .bind(&tb_sha[..])
                    .bind(&tb_b3[..])
                    .bind(backend_kind)
                    .bind(&tb_key)
                    .bind(opt.thumb_mime)
                    .bind(opt.thumb_ext)
                    .bind(tb.len() as i64)
                    .bind(bytes.len() as i64)
                    .bind(original_obj_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    thumb_obj_id = Some(tid);
                }
            }
        }
    }

    let ref_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO media_references
            (id, owner_id, scope_type, scope_id, message_id, object_id, optimized_object_id, thumb_object_id,
             original_filename, mime_type, byte_size, sha256_plain, sha256_cipher, forward_from_ref_id,
             storage_tier, availability_state, relay_expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(ref_id)
    .bind(owner_id)
    .bind(scope_type)
    .bind(scope_id)
    .bind(meta.message_id)
    .bind(original_obj_id)
    .bind(opt_obj_id)
    .bind(thumb_obj_id)
    .bind(&declared_name)
    .bind(&actual_mime)
    .bind(bytes.len() as i64)
    .bind(plain_sha.as_ref().map(|p| &p[..]))
    .bind(&stored_sha_arr[..])
    .bind(meta.forward_from_ref_id)
    .bind(storage_tier)
    .bind(availability_state)
    .bind(relay_expires_at)
    .execute(&mut *tx)
    .await?;

    // Maintain backwards-compatible row in `attachments` linking to `ref_id`.
    let compat_path = format!("{}::{}", backend_kind, storage_key);
    sqlx::query(
        r#"
        INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type, ref_id, scope_type, scope_id)
        VALUES ($1, $2, $3, $4, $5, $6, $1, $7, $8)
        "#,
    )
    .bind(ref_id)
    .bind(owner_id)
    .bind(&compat_path)
    .bind(&declared_name)
    .bind(bytes.len() as i64)
    .bind(&actual_mime)
    .bind(scope_type)
    .bind(scope_id)
    .execute(&mut *tx)
    .await?;

    if let Some(did) = device_id {
        let _ = sqlx::query(
            r#"
            INSERT INTO media_device_locations (ref_id, device_id, user_id, has_original, has_optimized, local_tier, verified_sha256)
            VALUES ($1, $2, $3, TRUE, $4, 'server_relay', $5)
            ON CONFLICT (ref_id, device_id) DO UPDATE SET last_verified_at = now()
            "#,
        )
        .bind(ref_id)
        .bind(did)
        .bind(owner_id)
        .bind(opt_obj_id.is_some())
        .bind(&stored_sha_arr[..])
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await?;

    Ok(serde_json::json!({
        "id": ref_id,
        "ref_id": ref_id,
        "object_id": original_obj_id,
        "uploader_id": owner_id,
        "file_name": declared_name,
        "file_size": bytes.len() as i64,
        "mime_type": actual_mime,
        "sha256": sha_hex,
        "blake3": blake3_hex,
        "orig_profile_json": client_orig_profile,
        "orig_width": meta.orig_width,
        "orig_height": meta.orig_height,
        "ai_recommended": client_ai_recommended,
        "storage_tier": storage_tier,
        "storage_backend": backend_kind,
        "availability_state": availability_state,
        "deduplicated": deduplicated,
        "dedup_match_kind": dedup_match_kind,
        "has_optimized": opt_obj_id.is_some(),
        "has_thumb": thumb_obj_id.is_some(),
        "relay_expires_at": relay_expires_at,
        "download_url": format!("/api/attachments/{ref_id}"),
        "created_at": chrono::Utc::now(),
    }))
}

pub async fn serve_media_reference(
    pool: &PgPool,
    storage: &StorageRegistry,
    caller_id: Uuid,
    ref_id: Uuid,
    variant: Option<&str>,
    range_header: Option<&str>,
) -> AppResult<Response> {
    let row = sqlx::query(
        r#"
        SELECT r.owner_id, r.scope_type, r.scope_id, r.original_filename, r.mime_type, r.deleted_for_all,
               r.object_id, r.optimized_object_id, r.thumb_object_id,
               o.storage_backend AS orig_backend, o.storage_key AS orig_key, o.mime_type AS orig_mime, o.sha256 AS orig_sha, o.byte_size AS orig_size, COALESCE(o.is_cdc_assembled, FALSE) AS orig_is_cdc,
               opt.storage_backend AS opt_backend, opt.storage_key AS opt_key, opt.mime_type AS opt_mime, opt.sha256 AS opt_sha, opt.byte_size AS opt_size,
               th.storage_backend AS th_backend, th.storage_key AS th_key, th.mime_type AS th_mime, th.sha256 AS th_sha, th.byte_size AS th_size
        FROM media_references r
        LEFT JOIN media_objects o ON o.id = r.object_id
        LEFT JOIN media_objects opt ON opt.id = r.optimized_object_id
        LEFT JOIN media_objects th ON th.id = r.thumb_object_id
        WHERE r.id = $1
        "#,
    )
    .bind(ref_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("attachment_not_found"))?;

    let owner_id: Uuid = row.try_get("owner_id")?;
    let scope_type: String = row.try_get("scope_type")?;
    let scope_id: Option<Uuid> = row.try_get("scope_id")?;
    authorize_read(pool, caller_id, &scope_type, scope_id, owner_id).await?;

    let deleted_for_all: bool = row.try_get("deleted_for_all")?;
    if deleted_for_all {
        return Err(AppError::NotFound("deleted_for_everyone"));
    }

    let wanted = variant.unwrap_or("original");
    let orig_is_cdc: bool = row.try_get("orig_is_cdc").unwrap_or(false);
    let orig_object_id: Option<Uuid> = row.try_get("object_id")?;
    let (backend, key, mime, sha, total_size, use_cdc) = if wanted == "thumb" && row.try_get::<Option<String>, _>("th_key")?.is_some() {
        (
            row.try_get::<Option<String>, _>("th_backend")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("th_key")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("th_mime")?.unwrap_or_else(|| "image/avif".into()),
            row.try_get::<Option<Vec<u8>>, _>("th_sha")?.unwrap_or_default(),
            row.try_get::<Option<i64>, _>("th_size")?.unwrap_or(0) as u64,
            false,
        )
    } else if wanted == "optimized" && row.try_get::<Option<String>, _>("opt_key")?.is_some() {
        (
            row.try_get::<Option<String>, _>("opt_backend")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("opt_key")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("opt_mime")?.unwrap_or_else(|| "image/avif".into()),
            row.try_get::<Option<Vec<u8>>, _>("opt_sha")?.unwrap_or_default(),
            row.try_get::<Option<i64>, _>("opt_size")?.unwrap_or(0) as u64,
            false,
        )
    } else {
        let Some(k) = row.try_get::<Option<String>, _>("orig_key")? else {
            return Err(AppError::NotFound("no_server_copy_available"));
        };
        (
            row.try_get::<Option<String>, _>("orig_backend")?.unwrap_or_default(),
            k,
            row.try_get::<Option<String>, _>("orig_mime")?.unwrap_or_else(|| "application/octet-stream".into()),
            row.try_get::<Option<Vec<u8>>, _>("orig_sha")?.unwrap_or_default(),
            row.try_get::<Option<i64>, _>("orig_size")?.unwrap_or(0) as u64,
            orig_is_cdc,
        )
    };

    let filename: String = row.try_get("original_filename")?;
    let safe_disp = sanitize_filename(&filename);
    let sha_hex = hex::encode(sha);

    // Spec 158: Touch `last_accessed_at` on real media reads (throttled to 1h windows to avoid write amplification)
    // so actively viewed images stay untouched indefinitely and only 15-day unused images compress.
    if wanted != "thumb" {
        let touch_pool = pool.clone();
        let touch_oid = orig_object_id;
        tokio::spawn(async move {
            let _ = sqlx::query(
                "UPDATE media_references SET last_accessed_at = now() WHERE id = $1 AND last_accessed_at < now() - interval '1 hour'",
            )
            .bind(ref_id)
            .execute(&touch_pool)
            .await;
            if let Some(oid) = touch_oid {
                let _ = sqlx::query(
                    "UPDATE media_objects SET last_accessed_at = now() WHERE id = $1 AND last_accessed_at < now() - interval '1 hour'",
                )
                .bind(oid)
                .execute(&touch_pool)
                .await;
            }
        });
    }

    if use_cdc {
        let oid = orig_object_id.ok_or(AppError::NotFound("cdc_object_missing"))?;
        if let Some((start, end_opt)) = range_header.and_then(parse_byte_range) {
            let end = end_opt.unwrap_or(total_size.saturating_sub(1)).min(total_size.saturating_sub(1));
            let (chunk, full_len) = dedup_engine::read_cdc_object_range(pool, storage, oid, Some((start, end))).await?;
            let end_actual = start + (chunk.len() as u64).saturating_sub(1);
            return Ok((
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CONTENT_RANGE, format!("bytes {start}-{end_actual}/{full_len}")),
                    (header::ACCEPT_RANGES, "bytes".into()),
                    (header::CONTENT_DISPOSITION, format!("inline; filename=\"{safe_disp}\"")),
                    (header::HeaderName::from_static("x-content-sha256"), sha_hex),
                ],
                chunk,
            )
                .into_response());
        }
        let (full_bytes, _) = dedup_engine::read_cdc_object_range(pool, storage, oid, None).await?;
        return Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime),
                (header::ACCEPT_RANGES, "bytes".into()),
                (header::CONTENT_DISPOSITION, format!("inline; filename=\"{safe_disp}\"")),
                (header::HeaderName::from_static("x-content-sha256"), sha_hex),
            ],
            full_bytes,
        )
            .into_response());
    }

    if let Some((start, end_opt)) = range_header.and_then(parse_byte_range) {
        let end = end_opt.unwrap_or(total_size.saturating_sub(1)).min(total_size.saturating_sub(1));
        let (chunk, full_len) = match backend.as_str() {
            "s3_ready" => storage.s3.get_object_range(&key, start, end).await,
            "local_cas" => storage.local_cas.get_object_range(&key, start, end).await,
            "temp_relay" => storage.temp_relay.get_object_range(&key, start, end).await,
            "encrypted_backup" => storage.encrypted_backup.get_object_range(&key, start, end).await,
            _ => storage.existing.get_object_range(&key, start, end).await,
        }
        .map_err(|_| AppError::NotFound("object_file_missing"))?;
        let end_actual = start + (chunk.len() as u64).saturating_sub(1);
        return Ok((
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_TYPE, mime),
                (header::CONTENT_RANGE, format!("bytes {start}-{end_actual}/{full_len}")),
                (header::ACCEPT_RANGES, "bytes".into()),
                (header::CONTENT_DISPOSITION, format!("inline; filename=\"{safe_disp}\"")),
                (header::HeaderName::from_static("x-content-sha256"), sha_hex),
            ],
            chunk,
        )
            .into_response());
    }

    let body = match backend.as_str() {
        "s3_ready" => storage.s3.get_object_stream(&key).await,
        "local_cas" => storage.local_cas.get_object_stream(&key).await,
        "temp_relay" => storage.temp_relay.get_object_stream(&key).await,
        "encrypted_backup" => storage.encrypted_backup.get_object_stream(&key).await,
        _ => storage.existing.get_object_stream(&key).await,
    }
    .map_err(|_| AppError::NotFound("object_file_missing"))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::ACCEPT_RANGES, "bytes".into()),
            (header::CONTENT_DISPOSITION, format!("inline; filename=\"{safe_disp}\"")),
            (header::HeaderName::from_static("x-content-sha256"), sha_hex),
        ],
        body,
    )
        .into_response())
}

// -----------------------------------------------------------------------------
// Route Handlers
// -----------------------------------------------------------------------------

pub async fn auth_caller(state: &AppState, headers: &HeaderMap) -> AppResult<crate::auth::session::AuthUser> {
    let tok = parse_cookie(headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    authenticate_session(&state.db, &state.config, tok).await
}

async fn handle_capabilities(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    let _user = auth_caller(&state, &headers).await?;
    let s3_health = state.storage.s3.health_check().await;
    Ok(Json(serde_json::json!({
        "adapters": state.storage.capabilities_list(),
        "s3": s3_health,
        "s3_active_for_durable": state.storage.s3.config().is_ready(),
        "s3_lifecycle_preview": state.storage.s3.lifecycle_policy_descriptor(7),
        "default_cache_limit_mb": 1024,
        "e2ee_chunk_protocol": "CWCH3",
        "supported_codecs": ["avif", "avis", "av1_fmp4", "opus", "flac", "zstd", "brotli", "lzma2"],
    })))
}

fn default_media_quality_mode() -> String {
    "ultra_optimized".into()
}
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct PolicyPayload {
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
}

async fn handle_get_policy(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<PolicyPayload>> {
    let user = auth_caller(&state, &headers).await?;
    sqlx::query(
        "INSERT INTO device_storage_policies (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;
    let r = sqlx::query(
        r#"
        SELECT cache_limit_mb, auto_download_mode, dm_relay_ttl_hours, dm_encrypted_backup,
               require_device_approval, prefer_original_upload,
               COALESCE(media_quality_mode, 'ultra_optimized') AS media_quality_mode,
               COALESCE(ai_enhancement_enabled, TRUE) AS ai_enhancement_enabled,
               COALESCE(data_saver_mode, FALSE) AS data_saver_mode,
               COALESCE(auto_cache_cleanup, TRUE) AS auto_cache_cleanup,
               COALESCE(server_durable_dm, TRUE) AS server_durable_dm
        FROM device_storage_policies WHERE user_id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(PolicyPayload {
        cache_limit_mb: r.try_get("cache_limit_mb")?,
        auto_download_mode: r.try_get("auto_download_mode")?,
        dm_relay_ttl_hours: r.try_get("dm_relay_ttl_hours")?,
        dm_encrypted_backup: r.try_get("dm_encrypted_backup")?,
        require_device_approval: r.try_get("require_device_approval")?,
        prefer_original_upload: r.try_get("prefer_original_upload")?,
        media_quality_mode: r.try_get("media_quality_mode")?,
        ai_enhancement_enabled: r.try_get("ai_enhancement_enabled")?,
        data_saver_mode: r.try_get("data_saver_mode")?,
        auto_cache_cleanup: r.try_get("auto_cache_cleanup")?,
        server_durable_dm: r.try_get("server_durable_dm")?,
    }))
}

async fn handle_update_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(p): Json<PolicyPayload>,
) -> AppResult<Json<PolicyPayload>> {
    let user = auth_caller(&state, &headers).await?;
    if !matches!(p.auto_download_mode.as_str(), "always" | "wifi" | "manual") {
        return Err(AppError::BadRequest("invalid_auto_download_mode"));
    }
    if !matches!(p.media_quality_mode.as_str(), "ultra_optimized" | "data_saver" | "original") {
        return Err(AppError::BadRequest("invalid_media_quality_mode"));
    }
    if !(1..=720).contains(&p.dm_relay_ttl_hours) || p.cache_limit_mb < 0 {
        return Err(AppError::BadRequest("invalid_policy_bounds"));
    }
    sqlx::query(
        r#"
        INSERT INTO device_storage_policies
            (user_id, cache_limit_mb, auto_download_mode, dm_relay_ttl_hours, dm_encrypted_backup,
             require_device_approval, prefer_original_upload, media_quality_mode, ai_enhancement_enabled,
             data_saver_mode, auto_cache_cleanup, server_durable_dm, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())
        ON CONFLICT (user_id) DO UPDATE SET
            cache_limit_mb = EXCLUDED.cache_limit_mb,
            auto_download_mode = EXCLUDED.auto_download_mode,
            dm_relay_ttl_hours = EXCLUDED.dm_relay_ttl_hours,
            dm_encrypted_backup = EXCLUDED.dm_encrypted_backup,
            require_device_approval = EXCLUDED.require_device_approval,
            prefer_original_upload = EXCLUDED.prefer_original_upload,
            media_quality_mode = EXCLUDED.media_quality_mode,
            ai_enhancement_enabled = EXCLUDED.ai_enhancement_enabled,
            data_saver_mode = EXCLUDED.data_saver_mode,
            auto_cache_cleanup = EXCLUDED.auto_cache_cleanup,
            server_durable_dm = EXCLUDED.server_durable_dm,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(p.cache_limit_mb)
    .bind(&p.auto_download_mode)
    .bind(p.dm_relay_ttl_hours)
    .bind(p.dm_encrypted_backup)
    .bind(p.require_device_approval)
    .bind(p.prefer_original_upload)
    .bind(&p.media_quality_mode)
    .bind(p.ai_enhancement_enabled)
    .bind(p.data_saver_mode)
    .bind(p.auto_cache_cleanup)
    .bind(p.server_durable_dm)
    .execute(&state.db)
    .await?;
    Ok(Json(p))
}

#[derive(Serialize, Deserialize)]
struct ConvStoragePolicyPayload {
    pub allow_peer_recovery: bool,
    pub dm_encrypted_backup: bool,
}

async fn handle_get_conv_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(peer_id): Path<Uuid>,
) -> AppResult<Json<ConvStoragePolicyPayload>> {
    let user = auth_caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT allow_peer_recovery, dm_encrypted_backup FROM conversation_prefs WHERE user_id = $1 AND conv_type = 'dm' AND conv_id = $2",
    )
    .bind(user.id)
    .bind(peer_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(ConvStoragePolicyPayload {
        allow_peer_recovery: row.as_ref().and_then(|r| r.try_get("allow_peer_recovery").ok()).unwrap_or(true),
        dm_encrypted_backup: row.as_ref().and_then(|r| r.try_get("dm_encrypted_backup").ok()).unwrap_or(false),
    }))
}

async fn handle_update_conv_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(peer_id): Path<Uuid>,
    Json(p): Json<ConvStoragePolicyPayload>,
) -> AppResult<Json<ConvStoragePolicyPayload>> {
    let user = auth_caller(&state, &headers).await?;
    sqlx::query(
        r#"
        INSERT INTO conversation_prefs (user_id, conv_type, conv_id, allow_peer_recovery, dm_encrypted_backup)
        VALUES ($1, 'dm', $2, $3, $4)
        ON CONFLICT (user_id, conv_type, conv_id) DO UPDATE SET
            allow_peer_recovery = EXCLUDED.allow_peer_recovery,
            dm_encrypted_backup = EXCLUDED.dm_encrypted_backup,
            updated_at = now()
        "#,
    )
    .bind(user.id)
    .bind(peer_id)
    .bind(p.allow_peer_recovery)
    .bind(p.dm_encrypted_backup)
    .execute(&state.db)
    .await?;
    Ok(Json(p))
}

#[derive(Deserialize)]
struct InitUploadReq {
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub file_name: String,
    pub mime_type: String,
    pub total_size: i64,
    pub chunk_size: Option<i32>,
    pub sha256_cipher: Option<String>,
    pub sha256_plain: Option<String>,
    pub blake3_hex: Option<String>,
    pub perceptual_hash_hex: Option<String>,
    pub perceptual_quality: Option<u8>,
    pub caption_band_hash: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_e2ee: Option<bool>,
    pub preserve_original: Option<bool>,
}

async fn handle_init_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InitUploadReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.scope_type, req.scope_id).await?;
    if is_blocked_svg_upload(Some(&req.file_name), Some(&req.mime_type), None, None) {
        return Err(AppError::BadRequest("svg_upload_not_allowed"));
    }
    if req.total_size <= 0 || req.total_size > 200 * 1024 * 1024 {
        return Err(AppError::BadRequest("invalid_total_size"));
    }

    let chunk_size = req.chunk_size.unwrap_or(1024 * 1024).clamp(4096, 8 * 1024 * 1024);
    let total_chunks = ((req.total_size + chunk_size as i64 - 1) / chunk_size as i64) as i32;
    let transfer_id = Uuid::new_v4();
    let expected_sha = decode_hex_32(req.sha256_cipher.as_deref());
    let plain_sha = decode_hex_32(req.sha256_plain.as_deref());

    // Instant deduplication fast-path check STRICTLY for non-E2EE shared scopes (`group`/`channel`/`library`/`sticker`).
    let is_e2ee = req.is_e2ee.unwrap_or(false);
    if !is_e2ee && matches!(req.scope_type.as_str(), "group" | "channel" | "library" | "sticker") {
        let mut matched_instant: Option<(Uuid, &'static str)> = None;

        // 1. Exact match by cas_scope
        if let Some(ref hex_sha) = req.sha256_cipher {
            let cas_scope = format!("shared:{}", hex_sha.to_lowercase());
            if let Some(ex_row) = sqlx::query(
                "SELECT id, storage_key, storage_backend FROM media_objects WHERE cas_scope = $1 AND variant_kind = 'original'",
            )
            .bind(&cas_scope)
            .fetch_optional(&state.db)
            .await?
            {
                let obj_id: Uuid = ex_row.try_get("id")?;
                let ex_key: String = ex_row.try_get("storage_key")?;
                let ex_backend: String = ex_row.try_get("storage_backend")?;
                let exists = match ex_backend.as_str() {
                    "s3_ready" => state.storage.s3.object_exists(&ex_key).await.unwrap_or(false),
                    _ => state.storage.local_cas.object_exists(&ex_key).await.unwrap_or(false),
                };
                if exists {
                    matched_instant = Some((obj_id, "exact_cas_scope"));
                }
            }
        }

        // 2. Exact match by BLAKE3 or SHA-256 in shared index
        if matched_instant.is_none() {
            let b3_arr = decode_hex_32(req.blake3_hex.as_deref());
            let sha_arr = decode_hex_32(req.sha256_cipher.as_deref()).or_else(|| decode_hex_32(req.sha256_plain.as_deref()));
            if b3_arr.is_some() || sha_arr.is_some() {
                if let Some(row) = sqlx::query(
                    r#"
                    SELECT d.object_id, o.storage_key, o.storage_backend
                    FROM media_dedup_indexes d
                    JOIN media_objects o ON o.id = d.object_id
                    WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND (d.blake3 = $1 OR d.sha256 = $2)
                    LIMIT 1
                    "#,
                )
                .bind(b3_arr.as_ref().map(|b| &b[..]))
                .bind(sha_arr.as_ref().map(|s| &s[..]))
                .fetch_optional(&state.db)
                .await?
                {
                    let obj_id: Uuid = row.try_get("object_id")?;
                    let ex_key: String = row.try_get("storage_key")?;
                    let ex_backend: String = row.try_get("storage_backend")?;
                    let exists = match ex_backend.as_str() {
                        "s3_ready" => state.storage.s3.object_exists(&ex_key).await.unwrap_or(false),
                        _ => state.storage.local_cas.object_exists(&ex_key).await.unwrap_or(false),
                    };
                    if exists {
                        matched_instant = Some((obj_id, "exact_shared_index"));
                    }
                }
            }
        }

        // 3. O(1) 16-Band MIH PDQ meme match with Triple Guard (caption + color + aspect)
        if matched_instant.is_none() {
            if let Some(pdq_hex) = req.perceptual_hash_hex.as_deref() {
                if let Some(incoming_pdq) = PdqHash256::from_hex(
                    pdq_hex,
                    req.perceptual_quality.unwrap_or(0),
                    req.caption_band_hash.unwrap_or(0),
                ) {
                    let incoming_px = (req.width.unwrap_or(0) as u64) * (req.height.unwrap_or(0) as u64);
                    if let Some(hit) = dedup_engine::find_pdq_mih_match(
                        &state.db,
                        &state.storage,
                        "shared_non_e2ee",
                        &incoming_pdq,
                        incoming_px,
                    )
                    .await?
                    {
                        matched_instant = Some((hit.object_id, "perceptual_pdq_mih_match"));
                    }
                }
            }
        }

        if let Some((obj_id, match_kind)) = matched_instant {
            let ref_id = Uuid::new_v4();
            let mut tx = state.db.begin().await?;

            // Reuse variants if present
            let mut opt_obj_id: Option<Uuid> = None;
            let mut thumb_obj_id: Option<Uuid> = None;
            let variants = sqlx::query(
                "SELECT id, variant_kind FROM media_objects WHERE variant_of = $1",
            )
            .bind(obj_id)
            .fetch_all(&mut *tx)
            .await?;
            for v in variants {
                let vid: Uuid = v.try_get("id")?;
                let vkind: String = v.try_get("variant_kind")?;
                if vkind == "optimized" && opt_obj_id.is_none() {
                    opt_obj_id = Some(vid);
                    let _ = sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                        .bind(vid)
                        .execute(&mut *tx)
                        .await;
                } else if (vkind == "thumb" || vkind == "thumbnail") && thumb_obj_id.is_none() {
                    thumb_obj_id = Some(vid);
                    let _ = sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                        .bind(vid)
                        .execute(&mut *tx)
                        .await;
                }
            }

            sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
                .bind(obj_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                r#"
                INSERT INTO media_references
                    (id, owner_id, scope_type, scope_id, object_id, optimized_object_id, thumb_object_id,
                     original_filename, mime_type, byte_size, sha256_plain, sha256_cipher,
                     storage_tier, availability_state)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'server_durable', 'server_durable')
                "#,
            )
            .bind(ref_id)
            .bind(user.id)
            .bind(&req.scope_type)
            .bind(req.scope_id)
            .bind(obj_id)
            .bind(opt_obj_id)
            .bind(thumb_obj_id)
            .bind(sanitize_filename(&req.file_name))
            .bind(&req.mime_type)
            .bind(req.total_size)
            .bind(plain_sha.as_ref().map(|x| &x[..]))
            .bind(expected_sha.as_ref().map(|x| &x[..]))
            .execute(&mut *tx)
            .await?;

            if let Some(sid) = req.scope_id {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO media_permissions (id, reference_id, grantee_type, grantee_id, can_read, can_write, can_forward)
                    VALUES ($1, $2, $3, $4, TRUE, TRUE, TRUE)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(ref_id)
                .bind(&req.scope_type)
                .bind(sid)
                .execute(&mut *tx)
                .await;
            }

            let _ = sqlx::query(
                r#"
                INSERT INTO media_access_audit_logs (id, reference_id, actor_id, action, client_platform, ip_prefix)
                VALUES ($1, $2, $3, 'dedup_reference_granted', 'client', '127.0.0.0/24')
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ref_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await;

            tx.commit().await?;

            return Ok(Json(serde_json::json!({
                "dedup_instant": true,
                "dedup_kind": match_kind,
                "transfer_id": transfer_id,
                "ref_id": ref_id,
                "download_url": format!("/api/attachments/{ref_id}"),
            })));
        }
    }

    // Use real S3 Multipart Upload for >= 5 MiB transfers when S3 is configured; otherwise local staging (which uploads to S3 on complete if ready).
    let use_s3_multipart = state.storage.s3.config().is_ready() && req.total_size >= 5 * 1024 * 1024 && chunk_size >= 5 * 1024 * 1024;
    let multipart_token = if use_s3_multipart {
        let s3_key = if is_e2ee {
            format!("e2ee/multipart/{transfer_id}.enc")
        } else {
            format!("cas/multipart/{transfer_id}.bin")
        };
        let init = state
            .storage
            .s3
            .initiate_multipart_upload(&s3_key, &req.mime_type)
            .await
            .map_err(|_| AppError::Internal)?;
        format!("s3::{}::{}", s3_key, init.upload_id)
    } else {
        let init = state
            .storage
            .local_cas
            .begin_multipart_transfer(&format!("transfer/{transfer_id}"), &req.mime_type)
            .await
            .map_err(|_| AppError::Internal)?;
        init.upload_id
    };

    sqlx::query(
        r#"
        INSERT INTO transfer_sessions
            (id, user_id, device_id, scope_type, scope_id, original_filename, mime_type, total_size,
             chunk_size, total_chunks, expected_sha256, plain_sha256, is_e2ee, preserve_original, s3_upload_id, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, now() + interval '24 hours')
        "#,
    )
    .bind(transfer_id)
    .bind(user.id)
    .bind(user.device_id)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .bind(sanitize_filename(&req.file_name))
    .bind(&req.mime_type)
    .bind(req.total_size)
    .bind(chunk_size)
    .bind(total_chunks)
    .bind(expected_sha.as_ref().map(|x| &x[..]))
    .bind(plain_sha.as_ref().map(|x| &x[..]))
    .bind(is_e2ee)
    .bind(req.preserve_original.unwrap_or(true))
    .bind(&multipart_token)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "dedup_instant": false,
        "transfer_id": transfer_id,
        "multipart_id": multipart_token,
        "s3_multipart": use_s3_multipart,
        "chunk_size": chunk_size,
        "total_chunks": total_chunks,
    })))
}

async fn handle_abort_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(transfer_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT s3_upload_id, status FROM transfer_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(transfer_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("transfer_not_found"))?;

    let status: String = row.try_get("status")?;
    if status == "completed" {
        return Err(AppError::BadRequest("transfer_already_completed"));
    }

    let token: String = row.try_get::<Option<String>, _>("s3_upload_id")?.unwrap_or_default();
    if let Some(rest) = token.strip_prefix("s3::") {
        if let Some((s3_key, upload_id)) = rest.split_once("::") {
            let _ = state.storage.s3.abort_multipart_upload(s3_key, upload_id).await;
        }
    } else if !token.is_empty() {
        let _ = state.storage.local_cas.abort_chunks(&token).await;
    }

    sqlx::query("UPDATE transfer_sessions SET status = 'aborted' WHERE id = $1")
        .bind(transfer_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "aborted": true,
        "transfer_id": transfer_id,
    })))
}

async fn handle_get_upload_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(transfer_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT status, total_chunks, total_size, resulting_ref_id FROM transfer_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(transfer_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("transfer_not_found"))?;
    let uploaded: Vec<i32> = sqlx::query_scalar(
        "SELECT chunk_index FROM transfer_chunks WHERE transfer_id = $1 ORDER BY chunk_index ASC",
    )
    .bind(transfer_id)
    .fetch_all(&state.db)
    .await?;
    let total_chunks: i32 = row.try_get("total_chunks")?;
    let missing: Vec<i32> = (0..total_chunks).filter(|i| !uploaded.contains(i)).collect();
    Ok(Json(serde_json::json!({
        "transfer_id": transfer_id,
        "status": row.try_get::<String, _>("status")?,
        "total_chunks": total_chunks,
        "total_size": row.try_get::<i64, _>("total_size")?,
        "uploaded_chunks": uploaded,
        "missing_chunks": missing,
        "resulting_ref_id": row.try_get::<Option<Uuid>, _>("resulting_ref_id")?,
    })))
}

async fn handle_upload_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((transfer_id, chunk_index)): Path<(Uuid, i32)>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    if body.is_empty() {
        return Err(AppError::BadRequest("empty_chunk"));
    }
    let row = sqlx::query(
        "SELECT total_chunks, s3_upload_id, status FROM transfer_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(transfer_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("transfer_not_found"))?;
    let status: String = row.try_get("status")?;
    let total_chunks: i32 = row.try_get("total_chunks")?;
    let multipart_id: String = row.try_get::<Option<String>, _>("s3_upload_id")?.unwrap_or_default();
    if status != "active" || chunk_index < 0 || chunk_index >= total_chunks {
        return Err(AppError::BadRequest("invalid_chunk_index"));
    }

    let chunk_sha = sha256_bytes(&body);
    if let Some(expected_hdr) = headers.get("x-chunk-sha256").and_then(|v| v.to_str().ok()) {
        let expected = decode_hex_32(Some(expected_hdr)).ok_or(AppError::BadRequest("invalid_chunk_sha256"))?;
        if expected != chunk_sha {
            return Err(AppError::BadRequest("chunk_integrity_mismatch"));
        }
    }

    let s3_etag: Option<String> = if let Some(rest) = multipart_id.strip_prefix("s3::") {
        let (s3_key, upload_id) = rest.split_once("::").ok_or(AppError::Internal)?;
        let etag = state
            .storage
            .s3
            .upload_multipart_part(s3_key, upload_id, (chunk_index + 1) as u32, body.to_vec())
            .await
            .map_err(|_| AppError::Internal)?;
        Some(etag)
    } else {
        state
            .storage
            .local_cas
            .upload_chunk(&multipart_id, chunk_index as u32, body.to_vec())
            .await
            .map_err(|_| AppError::Internal)?;
        None
    };

    sqlx::query(
        r#"
        INSERT INTO transfer_chunks (transfer_id, chunk_index, byte_size, chunk_sha256, s3_etag)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (transfer_id, chunk_index) DO UPDATE SET byte_size = EXCLUDED.byte_size, chunk_sha256 = EXCLUDED.chunk_sha256, s3_etag = EXCLUDED.s3_etag, uploaded_at = now()
        "#,
    )
    .bind(transfer_id)
    .bind(chunk_index)
    .bind(body.len() as i32)
    .bind(&chunk_sha[..])
    .bind(&s3_etag)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "chunk_index": chunk_index,
        "chunk_sha256": hex::encode(chunk_sha),
        "s3_etag": s3_etag,
    })))
}

#[derive(Deserialize, Default)]
struct CompleteUploadBody {
    pub message_id: Option<Uuid>,
    pub sha256_cipher: Option<String>,
}

async fn handle_complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(transfer_id): Path<Uuid>,
    body: Option<Json<CompleteUploadBody>>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let row = sqlx::query(
        r#"
        SELECT scope_type, scope_id, original_filename, mime_type, total_size, total_chunks,
               expected_sha256, plain_sha256, is_e2ee, preserve_original, s3_upload_id, status, resulting_ref_id
        FROM transfer_sessions WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(transfer_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("transfer_not_found"))?;

    if let Some(ref_id) = row.try_get::<Option<Uuid>, _>("resulting_ref_id")? {
        return Ok(Json(serde_json::json!({
            "id": ref_id,
            "ref_id": ref_id,
            "download_url": format!("/api/attachments/{ref_id}"),
        })));
    }
    if row.try_get::<String, _>("status")? != "active" {
        return Err(AppError::BadRequest("transfer_not_active"));
    }

    let total_chunks: i32 = row.try_get("total_chunks")?;
    let uploaded_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transfer_chunks WHERE transfer_id = $1",
    )
    .bind(transfer_id)
    .fetch_one(&state.db)
    .await?;
    if uploaded_count != total_chunks as i64 {
        return Err(AppError::BadRequest("missing_chunks"));
    }

    let multipart_id: String = row.try_get::<Option<String>, _>("s3_upload_id")?.unwrap_or_default();
    let expected_size: i64 = row.try_get("total_size")?;
    let expected_sha: Option<Vec<u8>> = row.try_get("expected_sha256")?;
    let scope_type: String = row.try_get("scope_type")?;
    let scope_id: Option<Uuid> = row.try_get("scope_id")?;
    let original_filename: String = row.try_get("original_filename")?;
    let mime_type: String = row.try_get("mime_type")?;
    let plain_sha: Option<Vec<u8>> = row.try_get("plain_sha256")?;
    let is_e2ee: bool = row.try_get("is_e2ee")?;
    let preserve_original: bool = row.try_get("preserve_original")?;
    let b = body.map(|j| j.0).unwrap_or_default();

    if let Some(rest) = multipart_id.strip_prefix("s3::") {
        let (s3_key, upload_id) = rest.split_once("::").ok_or(AppError::Internal)?;
        let part_rows = sqlx::query(
            "SELECT chunk_index, s3_etag FROM transfer_chunks WHERE transfer_id = $1 ORDER BY chunk_index ASC",
        )
        .bind(transfer_id)
        .fetch_all(&state.db)
        .await?;
        let mut parts = Vec::with_capacity(part_rows.len());
        for pr in part_rows {
            let idx: i32 = pr.try_get("chunk_index")?;
            let etag: String = pr.try_get::<Option<String>, _>("s3_etag")?.unwrap_or_default();
            parts.push(((idx + 1) as u32, etag));
        }
        state
            .storage
            .s3
            .complete_multipart_upload(s3_key, upload_id, &parts)
            .await
            .map_err(|_| AppError::Internal)?;

        let sha_arr = b
            .sha256_cipher
            .as_deref()
            .and_then(|h| decode_hex_32(Some(h)))
            .or_else(|| {
                expected_sha.as_ref().and_then(|v| {
                    if v.len() == 32 {
                        let mut a = [0u8; 32];
                        a.copy_from_slice(v);
                        Some(a)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| sha256_bytes(s3_key.as_bytes()));
        let sha_hex = hex::encode(sha_arr);
        let oid = Uuid::new_v4();
        let ref_id = Uuid::new_v4();
        let cas_scope = format!("s3mp:{}:{}", user.id, transfer_id);
        let actual_ext = if is_e2ee { "enc" } else { "bin" };
        let ingest_scope = if scope_type == "recovery_relay" { "dm" } else { scope_type.as_str() };

        let mut tx = state.db.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO media_objects
                (id, cas_scope, sha256, plain_sha256, storage_backend, storage_key, mime_type, file_ext,
                 compression_codec, byte_size, original_byte_size, is_animated, ref_count, variant_kind)
            VALUES ($1, $2, $3, $4, 's3_ready', $5, $6, $7, $8, $9, $9, FALSE, 1, 'original')
            "#,
        )
        .bind(oid)
        .bind(&cas_scope)
        .bind(&sha_arr[..])
        .bind(plain_sha.as_ref().map(|p| &p[..]))
        .bind(s3_key)
        .bind(&mime_type)
        .bind(actual_ext)
        .bind(if is_e2ee { "e2ee_ciphertext" } else { "s3_multipart" })
        .bind(expected_size)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO media_references
                (id, owner_id, scope_type, scope_id, message_id, object_id, original_filename, mime_type,
                 byte_size, sha256_plain, sha256_cipher, storage_tier, availability_state)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'server_durable', 'server_durable')
            "#,
        )
        .bind(ref_id)
        .bind(user.id)
        .bind(ingest_scope)
        .bind(scope_id)
        .bind(b.message_id)
        .bind(oid)
        .bind(&original_filename)
        .bind(&mime_type)
        .bind(expected_size)
        .bind(plain_sha.as_ref().map(|p| &p[..]))
        .bind(&sha_arr[..])
        .execute(&mut *tx)
        .await?;

        let compat_path = format!("s3_ready::{s3_key}");
        sqlx::query(
            r#"
            INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type, ref_id, scope_type, scope_id)
            VALUES ($1, $2, $3, $4, $5, $6, $1, $7, $8)
            "#,
        )
        .bind(ref_id)
        .bind(user.id)
        .bind(&compat_path)
        .bind(&original_filename)
        .bind(expected_size)
        .bind(&mime_type)
        .bind(ingest_scope)
        .bind(scope_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE transfer_sessions SET status = 'completed', resulting_ref_id = $2 WHERE id = $1")
            .bind(transfer_id)
            .bind(ref_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        return Ok(Json(serde_json::json!({
            "id": ref_id,
            "ref_id": ref_id,
            "uploader_id": user.id,
            "file_name": original_filename,
            "file_size": expected_size,
            "mime_type": mime_type,
            "sha256": sha_hex,
            "storage_tier": "server_durable",
            "storage_backend": "s3_ready",
            "availability_state": "server_durable",
            "deduplicated": false,
            "download_url": format!("/api/attachments/{ref_id}"),
            "created_at": chrono::Utc::now(),
        })));
    }

    let assembled = state
        .storage
        .local_cas
        .assemble_chunks(&multipart_id, total_chunks as u32)
        .await
        .map_err(|_| AppError::Internal)?;

    if assembled.len() as i64 != expected_size {
        return Err(AppError::BadRequest("size_mismatch"));
    }
    if let Some(exp) = expected_sha.filter(|v| v.len() == 32) {
        if sha256_bytes(&assembled).as_slice() != exp.as_slice() {
            return Err(AppError::BadRequest("integrity_mismatch"));
        }
    }

    let ingest_scope = if scope_type == "recovery_relay" { "dm" } else { scope_type.as_str() };
    let res = ingest_media_bytes(
        &state.db,
        &state.storage,
        user.id,
        user.device_id,
        assembled,
        &mime_type.clone(),
        UploadMetaQuery {
            scope_type: Some(ingest_scope.into()),
            scope_id,
            message_id: b.message_id,
            file_name: Some(original_filename),
            mime_type: Some(mime_type),
            sha256_plain: plain_sha.map(hex::encode),
            sha256_cipher: b.sha256_cipher,
            is_e2ee: Some(is_e2ee),
            preserve_original: Some(preserve_original),
            backup_dm: Some(scope_type == "encrypted_backup"),
            forward_from_ref_id: None,
            tier: None,
            ..Default::default()
        },
    )
    .await?;

    let resulting_ref_id = res.get("ref_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());
    sqlx::query(
        "UPDATE transfer_sessions SET status = 'completed', resulting_ref_id = $2 WHERE id = $1",
    )
    .bind(transfer_id)
    .bind(resulting_ref_id)
    .execute(&state.db)
    .await?;

    Ok(Json(res))
}

#[derive(Deserialize)]
struct ForwardRefReq {
    pub from_ref_id: Uuid,
    pub target_scope_type: String,
    pub target_scope_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
}

async fn handle_forward_reference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ForwardRefReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.target_scope_type, req.target_scope_id).await?;

    let src = sqlx::query(
        r#"
        SELECT owner_id, scope_type, scope_id, object_id, optimized_object_id, thumb_object_id,
               original_filename, mime_type, byte_size, sha256_plain, sha256_cipher, deleted_for_all, storage_tier, availability_state
        FROM media_references WHERE id = $1
        "#,
    )
    .bind(req.from_ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("source_reference_not_found"))?;

    let src_owner: Uuid = src.try_get("owner_id")?;
    let src_scope_type: String = src.try_get("scope_type")?;
    let src_scope_id: Option<Uuid> = src.try_get("scope_id")?;
    authorize_read(&state.db, user.id, &src_scope_type, src_scope_id, src_owner).await?;
    if src.try_get::<bool, _>("deleted_for_all")? {
        return Err(AppError::BadRequest("cannot_forward_deleted_media"));
    }

    let obj_id: Option<Uuid> = src.try_get("object_id")?;
    let opt_id: Option<Uuid> = src.try_get("optimized_object_id")?;
    let thumb_id: Option<Uuid> = src.try_get("thumb_object_id")?;
    let new_ref_id = Uuid::new_v4();

    let mut tx = state.db.begin().await?;
    for id_opt in [obj_id, opt_id, thumb_id] {
        if let Some(oid) = id_opt {
            sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
                .bind(oid)
                .execute(&mut *tx)
                .await?;
        }
    }

    let obj_meta = if let Some(oid) = obj_id {
        sqlx::query(
            "SELECT storage_backend, storage_key, orig_profile_json, orig_width, orig_height, ai_recommended FROM media_objects WHERE id = $1",
        )
        .bind(oid)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        None
    };

    let target_tier = if obj_id.is_some() {
        "server_durable"
    } else {
        "dm_client_preferred"
    };
    let target_avail = if obj_id.is_some() {
        "server_durable"
    } else {
        "replicated_multi_device"
    };

    let src_fname: String = src.try_get("original_filename")?;
    let src_mime: String = src.try_get("mime_type")?;
    let src_size: i64 = src.try_get("byte_size")?;
    if is_blocked_svg_upload(Some(&src_fname), Some(&src_mime), None, None) {
        return Err(AppError::BadRequest("svg_upload_not_allowed"));
    }

    sqlx::query(
        r#"
        INSERT INTO media_references
            (id, owner_id, scope_type, scope_id, message_id, object_id, optimized_object_id, thumb_object_id,
             original_filename, mime_type, byte_size, sha256_plain, sha256_cipher, forward_from_ref_id,
             storage_tier, availability_state)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        "#,
    )
    .bind(new_ref_id)
    .bind(user.id)
    .bind(&req.target_scope_type)
    .bind(req.target_scope_id)
    .bind(req.message_id)
    .bind(obj_id)
    .bind(opt_id)
    .bind(thumb_id)
    .bind(&src_fname)
    .bind(&src_mime)
    .bind(src_size)
    .bind(src.try_get::<Option<Vec<u8>>, _>("sha256_plain")?)
    .bind(src.try_get::<Option<Vec<u8>>, _>("sha256_cipher")?)
    .bind(req.from_ref_id)
    .bind(target_tier)
    .bind(target_avail)
    .execute(&mut *tx)
    .await?;

    // Maintain backwards-compatible row in `attachments` linking to `new_ref_id`
    let compat_path = obj_meta
        .as_ref()
        .and_then(|m| Some(format!("{}::{}", m.try_get::<String, _>("storage_backend").ok()?, m.try_get::<String, _>("storage_key").ok()?)))
        .unwrap_or_else(|| format!("forwarded::{new_ref_id}"));
    sqlx::query(
        r#"
        INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type, ref_id, scope_type, scope_id)
        VALUES ($1, $2, $3, $4, $5, $6, $1, $7, $8)
        "#,
    )
    .bind(new_ref_id)
    .bind(user.id)
    .bind(&compat_path)
    .bind(&src_fname)
    .bind(src_size)
    .bind(&src_mime)
    .bind(&req.target_scope_type)
    .bind(req.target_scope_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    let orig_profile_json = obj_meta.as_ref().and_then(|m| m.try_get::<Option<String>, _>("orig_profile_json").ok().flatten());
    let orig_width = obj_meta.as_ref().and_then(|m| m.try_get::<Option<i32>, _>("orig_width").ok().flatten());
    let orig_height = obj_meta.as_ref().and_then(|m| m.try_get::<Option<i32>, _>("orig_height").ok().flatten());
    let ai_recommended = obj_meta.as_ref().and_then(|m| m.try_get::<Option<bool>, _>("ai_recommended").ok().flatten()).unwrap_or(orig_profile_json.is_some());

    Ok(Json(serde_json::json!({
        "id": new_ref_id,
        "ref_id": new_ref_id,
        "object_id": obj_id,
        "reused_object": true,
        "deduplicated": true,
        "dedup_match_kind": "authorized_cross_chat_ref",
        "file_name": src_fname,
        "file_size": src_size,
        "mime_type": src_mime,
        "orig_profile_json": orig_profile_json,
        "orig_width": orig_width,
        "orig_height": orig_height,
        "ai_recommended": ai_recommended,
        "download_url": format!("/api/attachments/{new_ref_id}"),
    })))
}

pub use crate::storage::references::{
    BatchLocationsReq, DedupCheckReq, DeleteRefQuery, LocationItem, PutManifestChunk,
    PutManifestReq, RelayAckBody, RunMigrationReq, handle_cleanup_relays, handle_dedup_check,
    handle_delete_reference, handle_gc_incomplete, handle_get_manifest, handle_get_ref_meta,
    handle_presign_ref, handle_put_manifest, handle_relay_ack, handle_report_locations,
    handle_run_migration, handle_storage_metrics,
};

pub(crate) fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\' && *c != '"')
        .take(120)
        .collect();
    cleaned.trim().to_string()
}

pub(crate) fn decode_hex_32(hex_opt: Option<&str>) -> Option<[u8; 32]> {
    let h = hex_opt?.trim();
    if h.len() != 64 {
        return None;
    }
    let vec = hex::decode(h).ok()?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&vec);
    Some(arr)
}

pub(crate) fn parse_byte_range(hdr: &str) -> Option<(u64, Option<u64>)> {
    let spec = hdr.trim().strip_prefix("bytes=")?;
    let (s, e) = spec.split_once('-')?;
    let start = s.trim().parse::<u64>().ok()?;
    let end = if e.trim().is_empty() {
        None
    } else {
        Some(e.trim().parse::<u64>().ok()?)
    };
    Some((start, end))
}

#[cfg(test)]
mod spec115_svg_block_tests {
    use super::is_blocked_svg_upload;

    #[test]
    fn blocks_svg_by_extension_mime_or_xml_payload() {
        assert!(is_blocked_svg_upload(Some("icon.svg"), None, None, None));
        assert!(is_blocked_svg_upload(Some("VECTOR.SVGZ"), None, None, None));
        assert!(is_blocked_svg_upload(Some("photo.png"), Some("image/svg+xml"), None, None));
        assert!(is_blocked_svg_upload(Some("photo.png"), None, Some("image/svg+xml; charset=utf-8"), None));
        assert!(is_blocked_svg_upload(
            Some("renamed.png"),
            Some("image/png"),
            Some("image/png"),
            Some(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"10\" height=\"10\"/></svg>"),
        ));
        assert!(is_blocked_svg_upload(
            Some("renamed.bin"),
            Some("application/octet-stream"),
            None,
            Some(b"\xEF\xBB\xBF  <?xml version=\"1.0\"?><!-- c --><SVG viewBox=\"0 0 1 1\"></SVG>"),
        ));
        assert!(is_blocked_svg_upload(
            Some("doctype.txt"),
            Some("text/plain"),
            None,
            Some(b"<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">"),
        ));
    }

    #[test]
    fn allows_valid_raster_images_and_normal_files() {
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert!(!is_blocked_svg_upload(Some("photo.png"), Some("image/png"), Some("image/png"), Some(png)));
        assert!(!is_blocked_svg_upload(Some("notes.txt"), Some("text/plain"), None, Some(b"hello world")));
        assert!(!is_blocked_svg_upload(Some("cipher.bin"), Some("application/octet-stream"), None, Some(&[0xde, 0xad, 0xbe, 0xef])));
    }
}
