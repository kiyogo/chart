//! Hyper-Deduplication Engine Backend (`Spec 111`)
//!
//! 4-Tier Server-Side Deduplication:
//! 1. **Exact CAS + Proof-of-Ownership (`BLAKE3` + `SHA-256` + `pow_digest`)**: zero-upload instant claim for
//!    both shared scopes AND Convergent Message-Locked E2EE DMs (server stays 100% blind to plaintext & keys!).
//! 2. **FastCDC v2020 Sub-File Content-Defined Chunking (`media_cdc_chunks` + `media_cdc_recipes`)**: stores
//!    only changed delta chunks for large files, PDFs, ZIPs, docs & videos with transparent HTTP `Range` serving!
//! 3. **Meta PDQ 256-bit DCT + 16-Band Multi-Index Hashing (MIH) O(1)** (`media_pdq_mih_buckets`): sub-millisecond
//!    Hamming lookup with Triple Guard (texture + meme caption band + 4×4 color moment + aspect ratio) —
//!    deduplicates memes, clipboard copies, format conversions, and blurry un-enhanced AVIFs vs originals!
//! 4. **vPDQ Multi-Keyframe Video + 128-bit Audio Spectral Fingerprint** + **Authorized Cross-Chat Reference Clone**
//!    (`from_ref_id` / `user_authorized_perceptual`) returning canonical `orig_profile_json` so BOTH sender and
//!    receiver sharpen copied blurry images to 4K with ZERO new physical files created!

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    storage::{
        StorageAdapter, StorageRegistry,
        permissions::{authorize_read, authorize_write},
        router::{auth_caller, decode_hex_32, sanitize_filename},
    },
};
use chatw_media_core::dedup::{
    AudioFingerprint, PdqHash256, PerceptualMatchVerdict, VideoFingerprint,
    audio_fingerprints_equivalent, compare_perceptual_images_v2, videos_are_safe_dedup_equivalent,
};

/// Insert all 16 MIH 16-bit bands of a PDQ 256-bit hash for O(1) Hamming candidate lookup.
pub async fn insert_pdq_mih_buckets(
    tx: &mut Transaction<'_, Postgres>,
    object_id: Uuid,
    scope_class: &str,
    pdq: &PdqHash256,
) -> Result<(), sqlx::Error> {
    if pdq.quality < 15 {
        return Ok(());
    }
    let bands = pdq.mih_bands();
    let idx_arr: Vec<i16> = (0..16i16).collect();
    let val_arr: Vec<i32> = bands.iter().map(|&b| b as i32).collect();
    sqlx::query(
        r#"
        INSERT INTO media_pdq_mih_buckets (band_idx, band_val, scope_class, object_id)
        SELECT b.band_idx, b.band_val, $2, $3
        FROM UNNEST($1::smallint[], $4::int[]) AS b(band_idx, band_val)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&idx_arr)
    .bind(scope_class)
    .bind(object_id)
    .bind(&val_arr)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub struct PerceptualDbMatch {
    pub object_id: Uuid,
    pub storage_backend: String,
    pub storage_key: String,
    pub is_cdc_assembled: bool,
    pub byte_size: i64,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub orig_width: Option<i32>,
    pub orig_height: Option<i32>,
    pub orig_profile_json: Option<String>,
    pub ai_recommended: bool,
    pub hamming: u32,
}

pub async fn physical_object_available(
    storage: &StorageRegistry,
    backend: &str,
    key: &str,
    is_cdc_assembled: bool,
) -> bool {
    if is_cdc_assembled {
        return true;
    }
    match backend {
        "s3_ready" => storage.s3.object_exists(key).await.unwrap_or(false),
        "temp_relay" => storage.temp_relay.object_exists(key).await.unwrap_or(false),
        "encrypted_backup" => storage.encrypted_backup.object_exists(key).await.unwrap_or(false),
        _ => storage.local_cas.object_exists(key).await.unwrap_or(false),
    }
}

/// O(1) Multi-Index Hashing (MIH) lookup over 16×16-bit bands + Triple Anti-False-Merge Verification!
pub async fn find_pdq_mih_match(
    pool: &PgPool,
    storage: &StorageRegistry,
    scope_class: &str,
    incoming: &PdqHash256,
    incoming_px: u64,
) -> AppResult<Option<PerceptualDbMatch>> {
    if incoming.quality < 18 {
        return Ok(None);
    }
    let bands = incoming.mih_bands();
    let idx_arr: Vec<i16> = (0..16i16).collect();
    let val_arr: Vec<i32> = bands.iter().map(|&b| b as i32).collect();

    // O(1) B-tree index lookup across all 16 bands, ranked by band collision count (closest candidates first!)
    let candidates = sqlx::query(
        r#"
        WITH hits AS (
            SELECT m.object_id, COUNT(*) AS band_hits
            FROM UNNEST($1::smallint[], $2::int[]) AS q(band_idx, band_val)
            JOIN media_pdq_mih_buckets m
              ON m.band_idx = q.band_idx
             AND m.band_val = q.band_val
             AND m.scope_class = $3
            GROUP BY m.object_id
            ORDER BY band_hits DESC
            LIMIT 64
        )
        SELECT d.object_id, d.perceptual_hash, d.perceptual_quality, d.caption_band_hash,
               d.color_moment_hash, d.aspect_ratio_milli,
               d.width, d.height, d.orig_width, d.orig_height, d.orig_profile_json, d.ai_recommended,
               o.storage_backend, o.storage_key, o.byte_size, o.mime_type, o.is_cdc_assembled
        FROM hits h
        JOIN media_dedup_indexes d ON d.object_id = h.object_id AND d.scope_class = $3
        JOIN media_objects o ON o.id = h.object_id
        WHERE d.perceptual_hash IS NOT NULL
        ORDER BY h.band_hits DESC
        "#,
    )
    .bind(&idx_arr)
    .bind(&val_arr)
    .bind(scope_class)
    .fetch_all(pool)
    .await?;

    let mut best: Option<PerceptualDbMatch> = None;

    for cand in candidates {
        let ph: Vec<u8> = cand.try_get("perceptual_hash")?;
        if ph.len() != 32 {
            continue;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&ph);
        let existing_pdq = PdqHash256 {
            bytes,
            quality: cand.try_get::<Option<i16>, _>("perceptual_quality")?.unwrap_or(0).clamp(0, 100) as u8,
            caption_band_hash: cand.try_get::<Option<i64>, _>("caption_band_hash")?.unwrap_or(0) as u64,
            color_moment_hash: cand.try_get::<Option<i64>, _>("color_moment_hash")?.unwrap_or(0) as u64,
            aspect_ratio_milli: cand.try_get::<Option<i16>, _>("aspect_ratio_milli")?.unwrap_or(0).max(0) as u16,
        };
        let w = cand.try_get::<Option<i32>, _>("width")?.unwrap_or(0).max(0) as u64;
        let h = cand.try_get::<Option<i32>, _>("height")?.unwrap_or(0).max(0) as u64;
        let ow = cand.try_get::<Option<i32>, _>("orig_width")?.unwrap_or(0).max(0) as u64;
        let oh = cand.try_get::<Option<i32>, _>("orig_height")?.unwrap_or(0).max(0) as u64;
        let effective_px = (w * h).max(ow * oh);
        let orig_profile_json: Option<String> = cand.try_get("orig_profile_json")?;
        let ai_rec: bool = cand.try_get("ai_recommended").unwrap_or(false) || orig_profile_json.is_some();

        if let PerceptualMatchVerdict::EquivalentCandidate { hamming } =
            compare_perceptual_images_v2(incoming, &existing_pdq, incoming_px, effective_px, ai_rec)
        {
            if best.as_ref().is_some_and(|b| b.hamming <= hamming) {
                continue;
            }
            let backend: String = cand.try_get("storage_backend")?;
            let key: String = cand.try_get("storage_key")?;
            let is_cdc: bool = cand.try_get("is_cdc_assembled").unwrap_or(false);
            if physical_object_available(storage, &backend, &key, is_cdc).await {
                best = Some(PerceptualDbMatch {
                    object_id: cand.try_get("object_id")?,
                    storage_backend: backend,
                    storage_key: key,
                    is_cdc_assembled: is_cdc,
                    byte_size: cand.try_get("byte_size")?,
                    mime_type: cand.try_get("mime_type")?,
                    width: cand.try_get("width")?,
                    height: cand.try_get("height")?,
                    orig_width: cand.try_get("orig_width")?,
                    orig_height: cand.try_get("orig_height")?,
                    orig_profile_json,
                    ai_recommended: ai_rec,
                    hamming,
                });
                if hamming <= 4 {
                    break; // Exact/near-exact perceptual match found!
                }
            }
        }
    }

    Ok(best)
}

/// Helper: atomically link a brand-new authorized `(media_references + attachments)` record onto an EXISTING
/// physical `media_object` (`ref_count += 1`, ZERO new physical file created on disk/S3!).
#[allow(clippy::too_many_arguments)]
async fn link_existing_object_for_user(
    pool: &PgPool,
    owner_id: Uuid,
    device_id: Option<Uuid>,
    scope_type: &str,
    scope_id: Option<Uuid>,
    message_id: Option<Uuid>,
    object_id: Uuid,
    from_ref_id: Option<Uuid>,
    file_name: &str,
    mime_override: Option<&str>,
    sha256_plain: Option<[u8; 32]>,
    dedup_match_kind: &str,
    hamming_distance: Option<u32>,
) -> AppResult<serde_json::Value> {
    let mut tx = pool.begin().await?;

    let obj = sqlx::query(
        r#"
        SELECT id, storage_backend, storage_key, mime_type, byte_size, sha256, blake3,
               width, height, orig_width, orig_height, orig_profile_json, ai_recommended
        FROM media_objects
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(object_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("canonical_object_not_found"))?;

    let backend: String = obj.try_get("storage_backend")?;
    let storage_key: String = obj.try_get("storage_key")?;
    let obj_mime: String = obj.try_get("mime_type")?;
    let byte_size: i64 = obj.try_get("byte_size")?;
    let obj_sha: Vec<u8> = obj.try_get("sha256")?;
    let obj_b3: Option<Vec<u8>> = obj.try_get("blake3")?;
    let width: Option<i32> = obj.try_get("width")?;
    let height: Option<i32> = obj.try_get("height")?;
    let orig_width: Option<i32> = obj.try_get("orig_width")?;
    let orig_height: Option<i32> = obj.try_get("orig_height")?;
    let orig_profile_json: Option<String> = obj.try_get("orig_profile_json")?;
    let ai_recommended: bool = obj.try_get::<Option<bool>, _>("ai_recommended")?.unwrap_or(false)
        || orig_profile_json.is_some();

    sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
        .bind(object_id)
        .execute(&mut *tx)
        .await?;

    // Also reuse any existing optimized/thumb variants attached to the canonical object!
    let mut opt_obj_id: Option<Uuid> = None;
    let mut thumb_obj_id: Option<Uuid> = None;
    let variants = sqlx::query("SELECT id, variant_kind FROM media_objects WHERE variant_of = $1")
        .bind(object_id)
        .fetch_all(&mut *tx)
        .await?;
    for v in variants {
        let vid: Uuid = v.try_get("id")?;
        let vk: String = v.try_get("variant_kind")?;
        if vk == "optimized" && opt_obj_id.is_none() {
            opt_obj_id = Some(vid);
            sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
                .bind(vid)
                .execute(&mut *tx)
                .await?;
        } else if (vk == "thumb" || vk == "thumbnail") && thumb_obj_id.is_none() {
            thumb_obj_id = Some(vid);
            sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1, gc_eligible_at = NULL WHERE id = $1")
                .bind(vid)
                .execute(&mut *tx)
                .await?;
        }
    }

    let declared_name = {
        let cleaned = sanitize_filename(file_name);
        if cleaned.is_empty() { "attachment.bin".to_string() } else { cleaned }
    };
    let final_mime = mime_override
        .filter(|m| !m.trim().is_empty() && *m != "application/octet-stream")
        .unwrap_or(obj_mime.as_str())
        .to_string();

    let ref_id = Uuid::new_v4();
    let plain_sha_slice = sha256_plain.as_ref().map(|a| &a[..]);
    sqlx::query(
        r#"
        INSERT INTO media_references
            (id, owner_id, scope_type, scope_id, message_id, object_id, optimized_object_id, thumb_object_id,
             original_filename, mime_type, byte_size, sha256_plain, sha256_cipher, forward_from_ref_id,
             storage_tier, availability_state)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'server_durable', 'server_durable')
        "#,
    )
    .bind(ref_id)
    .bind(owner_id)
    .bind(scope_type)
    .bind(scope_id)
    .bind(message_id)
    .bind(object_id)
    .bind(opt_obj_id)
    .bind(thumb_obj_id)
    .bind(&declared_name)
    .bind(&final_mime)
    .bind(byte_size)
    .bind(plain_sha_slice)
    .bind(&obj_sha)
    .bind(from_ref_id)
    .execute(&mut *tx)
    .await?;

    let compat_path = format!("{backend}::{storage_key}");
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
    .bind(byte_size)
    .bind(&final_mime)
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
        .bind(&obj_sha)
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await?;

    Ok(serde_json::json!({
        "claimed": true,
        "deduplicated": true,
        "reused_object": true,
        "dedup_match_kind": dedup_match_kind,
        "hamming_distance": hamming_distance,
        "id": ref_id,
        "ref_id": ref_id,
        "object_id": object_id,
        "file_name": declared_name,
        "file_size": byte_size,
        "mime_type": final_mime,
        "sha256": hex::encode(&obj_sha),
        "blake3": obj_b3.map(hex::encode),
        "width": width,
        "height": height,
        "orig_width": orig_width,
        "orig_height": orig_height,
        "orig_profile_json": orig_profile_json,
        "ai_recommended": ai_recommended,
        "has_optimized": opt_obj_id.is_some(),
        "has_thumb": thumb_obj_id.is_some(),
        "storage_tier": "server_durable",
        "storage_backend": backend,
        "availability_state": "server_durable",
        "download_url": format!("/api/attachments/{ref_id}"),
    }))
}

#[derive(Deserialize)]
pub struct DedupClaimReq {
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    /// Mode: `"from_ref_id"` | `"convergent_cipher_pow"` | `"shared_cas_or_perceptual"` | `"user_authorized_perceptual"`
    pub mode: String,
    pub from_ref_id: Option<Uuid>,
    /// Exact hashes
    pub blake3_hex: Option<String>,
    pub orig_blake3_hex: Option<String>,
    pub sha256_hex: Option<String>,
    pub sha256_plain: Option<String>,
    /// Proof-of-Ownership digest (`compute_pow_digest_hex`) proving caller physically holds the file bytes!
    pub pow_digest_hex: Option<String>,
    pub byte_size: Option<i64>,
    /// Perceptual image/meme PDQ 256-bit + Triple Guard metadata
    pub perceptual_hash_hex: Option<String>,
    pub perceptual_quality: Option<u8>,
    pub caption_band_hash: Option<u64>,
    pub color_moment_hash: Option<u64>,
    pub aspect_ratio_milli: Option<u16>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Video vPDQ & Audio Spectral Fingerprint
    pub video_fingerprint: Option<VideoFingerprint>,
    pub audio_fingerprint: Option<AudioFingerprint>,
}

/// Zero-Upload Atomic Deduplication Claim (`POST /api/storage/dedup/claim`).
/// Reuses the single canonical `media_object` on server (0 bytes uploaded, 0 new physical files created!)
/// and grants strict ACL-scoped access to the target conversation/account!
pub async fn handle_dedup_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DedupClaimReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.scope_type, req.scope_id).await?;

    let fname = req.file_name.as_deref().unwrap_or("attachment.bin");
    let plain_sha = decode_hex_32(req.sha256_plain.as_deref());

    match req.mode.as_str() {
        // MODE 1: Authorized Cross-Chat Copy / Paste / Forward (DM↔DM, DM↔Group, Group↔Group — even blurry un-enhanced copies!)
        "from_ref_id" => {
            let src_ref_id = req.from_ref_id.ok_or(AppError::BadRequest("missing_from_ref_id"))?;
            let src = sqlx::query(
                r#"
                SELECT owner_id, scope_type, scope_id, object_id, original_filename, mime_type, deleted_for_all
                FROM media_references WHERE id = $1
                "#,
            )
            .bind(src_ref_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound("source_reference_not_found"))?;

            let src_owner: Uuid = src.try_get("owner_id")?;
            let src_scope: String = src.try_get("scope_type")?;
            let src_scope_id: Option<Uuid> = src.try_get("scope_id")?;
            authorize_read(&state.db, user.id, &src_scope, src_scope_id, src_owner).await?;
            if src.try_get::<bool, _>("deleted_for_all")? {
                return Ok(Json(serde_json::json!({ "claimed": false, "reason": "source_deleted" })));
            }
            let Some(oid) = src.try_get::<Option<Uuid>, _>("object_id")? else {
                return Ok(Json(serde_json::json!({ "claimed": false, "reason": "source_no_server_object" })));
            };
            let src_name: String = src.try_get("original_filename")?;
            let src_mime: String = src.try_get("mime_type")?;
            let use_name = req.file_name.as_deref().filter(|s| !s.is_empty()).unwrap_or(&src_name);
            let use_mime = req.mime_type.as_deref().filter(|s| !s.is_empty()).unwrap_or(&src_mime);
            let out = link_existing_object_for_user(
                &state.db,
                user.id,
                user.device_id,
                &req.scope_type,
                req.scope_id,
                req.message_id,
                oid,
                Some(src_ref_id),
                use_name,
                Some(use_mime),
                plain_sha,
                "authorized_cross_chat_ref",
                Some(0),
            )
            .await?;
            return Ok(Json(out));
        }

        // MODE 2: Zero-Knowledge Convergent MLE Ciphertext Claim with Proof-of-Ownership (Cross-User E2EE DM Dedup!)
        // Server NEVER learns plaintext or decryption key; caller proves physical possession of ciphertext via `pow_digest`!
        "convergent_cipher_pow" => {
            let Some(b3) = decode_hex_32(req.blake3_hex.as_deref()) else {
                return Ok(Json(serde_json::json!({ "claimed": false, "reason": "missing_blake3" })));
            };
            let Some(pow) = decode_hex_32(req.pow_digest_hex.as_deref()) else {
                return Ok(Json(serde_json::json!({ "claimed": false, "reason": "missing_pow_digest" })));
            };
            if let Some(row) = sqlx::query(
                r#"
                SELECT id, storage_backend, storage_key, is_cdc_assembled, byte_size
                FROM media_objects
                WHERE blake3 = $1 AND pow_digest = $2 AND variant_kind = 'original'
                ORDER BY ref_count DESC
                LIMIT 1
                "#,
            )
            .bind(&b3[..])
            .bind(&pow[..])
            .fetch_optional(&state.db)
            .await?
            {
                let oid: Uuid = row.try_get("id")?;
                let backend: String = row.try_get("storage_backend")?;
                let key: String = row.try_get("storage_key")?;
                let is_cdc: bool = row.try_get("is_cdc_assembled").unwrap_or(false);
                let ex_size: i64 = row.try_get("byte_size")?;
                if req.byte_size.is_some_and(|s| s > 0 && s != ex_size) {
                    return Ok(Json(serde_json::json!({ "claimed": false, "reason": "size_mismatch" })));
                }
                if physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                    let out = link_existing_object_for_user(
                        &state.db,
                        user.id,
                        user.device_id,
                        &req.scope_type,
                        req.scope_id,
                        req.message_id,
                        oid,
                        None,
                        fname,
                        req.mime_type.as_deref(),
                        plain_sha,
                        "convergent_mle_pow",
                        Some(0),
                    )
                    .await?;
                    return Ok(Json(out));
                }
            }
            return Ok(Json(serde_json::json!({ "claimed": false, "reason": "no_convergent_match" })));
        }

        // MODE 3: User-Authorized Scope Lookup (searches ONLY objects this user ALREADY has read access to — works across devices!)
        "user_authorized_perceptual" => {
            if let Some(b3) = decode_hex_32(req.orig_blake3_hex.as_deref().or(req.blake3_hex.as_deref())) {
                if let Some(row) = sqlx::query(
                    r#"
                    SELECT r.id AS src_ref_id, o.id AS object_id, o.storage_backend, o.storage_key, o.is_cdc_assembled
                    FROM media_references r
                    JOIN media_objects o ON o.id = r.object_id
                    WHERE r.deleted_for_all = FALSE
                      AND (r.owner_id = $1 OR (r.scope_type = 'dm' AND r.scope_id = $1))
                      AND (o.blake3 = $2 OR o.plain_sha256 = $3)
                    ORDER BY r.created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(user.id)
                .bind(&b3[..])
                .bind(plain_sha.as_ref().map(|x| &x[..]))
                .fetch_optional(&state.db)
                .await?
                {
                    let oid: Uuid = row.try_get("object_id")?;
                    let src_ref: Uuid = row.try_get("src_ref_id")?;
                    let backend: String = row.try_get("storage_backend")?;
                    let key: String = row.try_get("storage_key")?;
                    let is_cdc: bool = row.try_get("is_cdc_assembled").unwrap_or(false);
                    if physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                        let out = link_existing_object_for_user(
                            &state.db,
                            user.id,
                            user.device_id,
                            &req.scope_type,
                            req.scope_id,
                            req.message_id,
                            oid,
                            Some(src_ref),
                            fname,
                            req.mime_type.as_deref(),
                            plain_sha,
                            "user_authorized_exact",
                            Some(0),
                        )
                        .await?;
                        return Ok(Json(out));
                    }
                }
            }
            return Ok(Json(serde_json::json!({ "claimed": false, "reason": "no_user_authorized_match" })));
        }

        // MODE 4: Shared Non-E2EE Multi-Tier Claim (Groups / Channels / Library / Stickers):
        // Exact BLAKE3/Orig-BLAKE3/SHA-256 -> O(1) MIH PDQ 256-bit (Memes/Blurry Copies) -> vPDQ Video -> Audio FP!
        "shared_cas_or_perceptual" => {
            if !matches!(req.scope_type.as_str(), "group" | "channel" | "library" | "sticker") {
                return Ok(Json(serde_json::json!({
                    "claimed": false,
                    "reason": "shared_claim_requires_shared_scope",
                })));
            }

            // Tier 1: Exact wire BLAKE3 / original pre-AVIF BLAKE3 / SHA-256 (O(1) B-tree index!)
            let wire_b3 = decode_hex_32(req.blake3_hex.as_deref());
            let orig_b3 = decode_hex_32(req.orig_blake3_hex.as_deref());
            let sha_arr = decode_hex_32(req.sha256_hex.as_deref().or(req.sha256_plain.as_deref()));

            if wire_b3.is_some() || orig_b3.is_some() || sha_arr.is_some() {
                if let Some(row) = sqlx::query(
                    r#"
                    SELECT d.object_id, o.storage_backend, o.storage_key, o.is_cdc_assembled
                    FROM media_dedup_indexes d
                    JOIN media_objects o ON o.id = d.object_id
                    WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE
                      AND (
                          ($1::bytea IS NOT NULL AND (d.blake3 = $1 OR d.orig_blake3 = $1))
                       OR ($2::bytea IS NOT NULL AND (d.orig_blake3 = $2 OR d.blake3 = $2))
                       OR ($3::bytea IS NOT NULL AND d.sha256 = $3)
                      )
                    LIMIT 1
                    "#,
                )
                .bind(wire_b3.as_ref().map(|x| &x[..]))
                .bind(orig_b3.as_ref().map(|x| &x[..]))
                .bind(sha_arr.as_ref().map(|x| &x[..]))
                .fetch_optional(&state.db)
                .await?
                {
                    let oid: Uuid = row.try_get("object_id")?;
                    let backend: String = row.try_get("storage_backend")?;
                    let key: String = row.try_get("storage_key")?;
                    let is_cdc: bool = row.try_get("is_cdc_assembled").unwrap_or(false);
                    if physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                        let out = link_existing_object_for_user(
                            &state.db,
                            user.id,
                            user.device_id,
                            &req.scope_type,
                            req.scope_id,
                            req.message_id,
                            oid,
                            None,
                            fname,
                            req.mime_type.as_deref(),
                            plain_sha,
                            "exact_shared_cas",
                            Some(0),
                        )
                        .await?;
                        return Ok(Json(out));
                    }
                }
            }

            // Tier 3: O(1) 16-Band MIH Meta PDQ 256-bit DCT + Meme Caption & Color-Moment & Aspect Guard!
            if let Some(pdq_hex) = req.perceptual_hash_hex.as_deref() {
                if let Some(incoming_pdq) = PdqHash256::from_hex_full(
                    pdq_hex,
                    req.perceptual_quality.unwrap_or(0),
                    req.caption_band_hash.unwrap_or(0),
                    req.color_moment_hash.unwrap_or(0),
                    req.aspect_ratio_milli.unwrap_or(0),
                ) {
                    let incoming_px = (req.width.unwrap_or(0) as u64) * (req.height.unwrap_or(0) as u64);
                    if let Some(hit) = find_pdq_mih_match(
                        &state.db,
                        &state.storage,
                        "shared_non_e2ee",
                        &incoming_pdq,
                        incoming_px,
                    )
                    .await?
                    {
                        let out = link_existing_object_for_user(
                            &state.db,
                            user.id,
                            user.device_id,
                            &req.scope_type,
                            req.scope_id,
                            req.message_id,
                            hit.object_id,
                            None,
                            fname,
                            req.mime_type.as_deref(),
                            plain_sha,
                            "perceptual_pdq_mih_match",
                            Some(hit.hamming),
                        )
                        .await?;
                        return Ok(Json(out));
                    }
                }
            }

            // Tier 4A: vPDQ Multi-Keyframe Video Match (MIH anchor pre-filter + full keyframe & duration ±3% verify)
            if let Some(ref vf) = req.video_fingerprint {
                let cands = if let Some(anchor) = vf.anchor_pdq() {
                    let bands = anchor.mih_bands();
                    let idx_arr: Vec<i16> = (0..16i16).collect();
                    let val_arr: Vec<i32> = bands.iter().map(|&b| b as i32).collect();
                    sqlx::query(
                        r#"
                        WITH hits AS (
                            SELECT m.object_id, COUNT(*) AS band_hits
                            FROM UNNEST($1::smallint[], $2::int[]) AS q(band_idx, band_val)
                            JOIN media_pdq_mih_buckets m
                              ON m.band_idx = q.band_idx AND m.band_val = q.band_val AND m.scope_class = 'video_shared'
                            GROUP BY m.object_id
                            ORDER BY band_hits DESC
                            LIMIT 64
                        )
                        SELECT d.object_id, d.video_fingerprint, o.storage_backend, o.storage_key, o.is_cdc_assembled
                        FROM hits h
                        JOIN media_dedup_indexes d ON d.object_id = h.object_id AND d.scope_class = 'shared_non_e2ee'
                        JOIN media_objects o ON o.id = h.object_id
                        WHERE d.video_fingerprint IS NOT NULL
                        "#,
                    )
                    .bind(&idx_arr)
                    .bind(&val_arr)
                    .fetch_all(&state.db)
                    .await?
                } else {
                    sqlx::query(
                        r#"
                        SELECT d.object_id, d.video_fingerprint, o.storage_backend, o.storage_key, o.is_cdc_assembled
                        FROM media_dedup_indexes d
                        JOIN media_objects o ON o.id = d.object_id
                        WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.video_fingerprint IS NOT NULL
                        ORDER BY d.created_at DESC
                        LIMIT 100
                        "#,
                    )
                    .fetch_all(&state.db)
                    .await?
                };
                for cand in cands {
                    if let Ok(val) = cand.try_get::<serde_json::Value, _>("video_fingerprint") {
                        if let Ok(cand_vf) = serde_json::from_value::<VideoFingerprint>(val) {
                            if videos_are_safe_dedup_equivalent(vf, &cand_vf) {
                                let oid: Uuid = cand.try_get("object_id")?;
                                let backend: String = cand.try_get("storage_backend")?;
                                let key: String = cand.try_get("storage_key")?;
                                let is_cdc: bool = cand.try_get("is_cdc_assembled").unwrap_or(false);
                                if physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                                    let out = link_existing_object_for_user(
                                        &state.db,
                                        user.id,
                                        user.device_id,
                                        &req.scope_type,
                                        req.scope_id,
                                        req.message_id,
                                        oid,
                                        None,
                                        fname,
                                        req.mime_type.as_deref(),
                                        plain_sha,
                                        "video_vpdq_mih_match",
                                        None,
                                    )
                                    .await?;
                                    return Ok(Json(out));
                                }
                            }
                        }
                    }
                }
            }

            // Tier 4B: 128-bit Audio Spectral Fingerprint Match
            if let Some(ref afp) = req.audio_fingerprint {
                let cands = sqlx::query(
                    r#"
                    SELECT d.object_id, d.audio_fingerprint, o.storage_backend, o.storage_key, o.is_cdc_assembled
                    FROM media_dedup_indexes d
                    JOIN media_objects o ON o.id = d.object_id
                    WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.audio_fingerprint IS NOT NULL
                    ORDER BY d.created_at DESC
                    LIMIT 200
                    "#,
                )
                .fetch_all(&state.db)
                .await?;
                for cand in cands {
                    if let Ok(val) = cand.try_get::<serde_json::Value, _>("audio_fingerprint") {
                        if let Ok(cand_afp) = serde_json::from_value::<AudioFingerprint>(val) {
                            if audio_fingerprints_equivalent(afp, &cand_afp) {
                                let oid: Uuid = cand.try_get("object_id")?;
                                let backend: String = cand.try_get("storage_backend")?;
                                let key: String = cand.try_get("storage_key")?;
                                let is_cdc: bool = cand.try_get("is_cdc_assembled").unwrap_or(false);
                                if physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                                    let out = link_existing_object_for_user(
                                        &state.db,
                                        user.id,
                                        user.device_id,
                                        &req.scope_type,
                                        req.scope_id,
                                        req.message_id,
                                        oid,
                                        None,
                                        fname,
                                        req.mime_type.as_deref(),
                                        plain_sha,
                                        "audio_spectral_match",
                                        None,
                                    )
                                    .await?;
                                    return Ok(Json(out));
                                }
                            }
                        }
                    }
                }
            }

            Ok(Json(serde_json::json!({ "claimed": false, "reason": "no_match" })))
        }

        _ => Err(AppError::BadRequest("invalid_dedup_claim_mode")),
    }
}

// -----------------------------------------------------------------------------
// Tier 2: FastCDC v2020 Sub-File Chunk Deduplication Endpoints + Streaming Read
// -----------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Clone)]
pub struct CdcChunkSpec {
    pub index: i32,
    pub offset: i64,
    pub length: i32,
    pub blake3_hex: String,
}

#[derive(Deserialize)]
pub struct CdcPlanReq {
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub chunks: Vec<CdcChunkSpec>,
}

/// FastCDC Delta Upload Planner (`POST /api/storage/dedup/cdc/plan`):
/// Client sends content-defined chunk BLAKE3 hashes; server replies with ONLY missing chunk indices!
pub async fn handle_cdc_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CdcPlanReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.scope_type, req.scope_id).await?;
    if req.chunks.is_empty() || req.chunks.len() > 2048 {
        return Err(AppError::BadRequest("invalid_cdc_chunk_count"));
    }

    let mut hash_vecs: Vec<Vec<u8>> = Vec::with_capacity(req.chunks.len());
    for c in &req.chunks {
        let arr = decode_hex_32(Some(&c.blake3_hex)).ok_or(AppError::BadRequest("invalid_chunk_blake3"))?;
        hash_vecs.push(arr.to_vec());
    }

    let rows = sqlx::query("SELECT chunk_blake3 FROM media_cdc_chunks WHERE chunk_blake3 = ANY($1)")
        .bind(&hash_vecs)
        .fetch_all(&state.db)
        .await?;
    let present: std::collections::HashSet<Vec<u8>> = rows
        .into_iter()
        .filter_map(|r| r.try_get::<Vec<u8>, _>("chunk_blake3").ok())
        .collect();

    let mut existing_indices = Vec::new();
    let mut missing_indices = Vec::new();
    let mut total_bytes: i64 = 0;
    let mut saved_bytes: i64 = 0;
    for (c, hv) in req.chunks.iter().zip(hash_vecs.iter()) {
        let len = c.length.max(0) as i64;
        total_bytes += len;
        if present.contains(hv) {
            existing_indices.push(c.index);
            saved_bytes += len;
        } else {
            missing_indices.push(c.index);
        }
    }
    let dedup_ratio = if total_bytes > 0 { (saved_bytes as f64) / (total_bytes as f64) } else { 0.0 };

    Ok(Json(serde_json::json!({
        "total_chunks": req.chunks.len(),
        "existing_indices": existing_indices,
        "missing_indices": missing_indices,
        "total_bytes": total_bytes,
        "saved_bytes": saved_bytes,
        "dedup_ratio": (dedup_ratio * 1000.0).round() / 1000.0,
    })))
}

/// Upload a single missing FastCDC chunk (`PUT /api/storage/dedup/cdc/chunks/{blake3_hex}`), verified by BLAKE3!
pub async fn handle_cdc_chunk_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(blake3_hex): Path<String>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let _user = auth_caller(&state, &headers).await?;
    if body.is_empty() || body.len() > 2 * 1024 * 1024 {
        return Err(AppError::BadRequest("invalid_cdc_chunk_size"));
    }
    let expected = decode_hex_32(Some(&blake3_hex)).ok_or(AppError::BadRequest("invalid_chunk_blake3"))?;
    let actual: [u8; 32] = blake3::hash(&body).into();
    if actual != expected {
        return Err(AppError::BadRequest("chunk_blake3_mismatch"));
    }

    let hex_clean = hex::encode(actual);
    let s3_ready = state.storage.s3.config().is_ready();
    let backend = if s3_ready { "s3_ready" } else { "local_cas" };
    let key = format!("cdc/{}/{}/{}.chk", &hex_clean[0..2], &hex_clean[2..4], hex_clean);

    let already_exists = match backend {
        "s3_ready" => state.storage.s3.object_exists(&key).await.unwrap_or(false),
        _ => state.storage.local_cas.object_exists(&key).await.unwrap_or(false),
    };
    if !already_exists {
        match backend {
            "s3_ready" => {
                state.storage.s3.put_object(&key, body.to_vec(), "application/octet-stream", None).await
                    .map_err(|_| AppError::Internal)?;
            }
            _ => {
                state.storage.local_cas.put_object(&key, body.to_vec(), "application/octet-stream", None).await
                    .map_err(|_| AppError::Internal)?;
            }
        }
    }

    sqlx::query(
        r#"
        INSERT INTO media_cdc_chunks (chunk_blake3, storage_backend, storage_key, byte_size, ref_count)
        VALUES ($1, $2, $3, $4, 0)
        ON CONFLICT (chunk_blake3) DO NOTHING
        "#,
    )
    .bind(&actual[..])
    .bind(backend)
    .bind(&key)
    .bind(body.len() as i32)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "stored": true,
        "blake3_hex": hex_clean,
        "byte_size": body.len(),
        "already_existed": already_exists,
    })))
}

#[derive(Deserialize)]
pub struct CdcCommitReq {
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub is_e2ee: Option<bool>,
    pub total_blake3_hex: String,
    pub total_sha256_hex: String,
    pub sha256_plain: Option<String>,
    pub pow_digest_hex: Option<String>,
    pub chunks: Vec<CdcChunkSpec>,
    pub video_fingerprint: Option<VideoFingerprint>,
    pub audio_fingerprint: Option<AudioFingerprint>,
}

/// Commit a FastCDC sub-file deduplicated recipe as a first-class `media_object` + `media_references` + `attachments`!
pub async fn handle_cdc_commit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CdcCommitReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.scope_type, req.scope_id).await?;
    if req.chunks.is_empty() || req.chunks.len() > 2048 {
        return Err(AppError::BadRequest("invalid_cdc_chunk_count"));
    }

    let total_b3 = decode_hex_32(Some(&req.total_blake3_hex)).ok_or(AppError::BadRequest("invalid_total_blake3"))?;
    let total_sha = decode_hex_32(Some(&req.total_sha256_hex)).ok_or(AppError::BadRequest("invalid_total_sha256"))?;
    let plain_sha = decode_hex_32(req.sha256_plain.as_deref());
    let pow_arr = decode_hex_32(req.pow_digest_hex.as_deref());
    let is_e2ee = req.is_e2ee.unwrap_or(req.scope_type == "dm");

    let mut parsed_chunks: Vec<(i32, i64, i32, [u8; 32])> = Vec::with_capacity(req.chunks.len());
    let mut total_size: i64 = 0;
    for (idx, c) in req.chunks.iter().enumerate() {
        let h = decode_hex_32(Some(&c.blake3_hex)).ok_or(AppError::BadRequest("invalid_chunk_blake3"))?;
        if c.length <= 0 {
            return Err(AppError::BadRequest("invalid_chunk_len"));
        }
        parsed_chunks.push((idx as i32, total_size, c.length, h));
        total_size += c.length as i64;
    }

    let mut tx = state.db.begin().await?;

    let s3_ready = state.storage.s3.config().is_ready();
    let backend = if s3_ready { "s3_ready" } else { "local_cas" };
    let cas_scope = if !is_e2ee && matches!(req.scope_type.as_str(), "group" | "channel" | "library" | "sticker") {
        format!("shared:cdc:{}", req.total_blake3_hex)
    } else {
        format!("priv_cdc:{}:{}", user.id, Uuid::new_v4())
    };
    let mime = req.mime_type.as_deref().filter(|s| !s.is_empty()).unwrap_or("application/octet-stream");
    let fname = req.file_name.as_deref().map(sanitize_filename).filter(|s| !s.is_empty()).unwrap_or_else(|| "file.bin".into());

    // If an identical FastCDC recipe object already exists under this `cas_scope`, reuse it atomically!
    if let Some(existing_oid) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM media_objects WHERE cas_scope = $1 AND variant_kind = 'original'",
    )
    .bind(&cas_scope)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.rollback().await?;
        let out = link_existing_object_for_user(
            &state.db,
            user.id,
            user.device_id,
            &req.scope_type,
            req.scope_id,
            req.message_id,
            existing_oid,
            None,
            &fname,
            Some(mime),
            plain_sha,
            "fastcdc_exact_recipe_reuse",
            Some(0),
        )
        .await?;
        let mut obj = out;
        if let Some(map) = obj.as_object_mut() {
            map.insert("cdc_total_chunks".into(), serde_json::json!(parsed_chunks.len()));
            map.insert("cdc_reused_chunks".into(), serde_json::json!(parsed_chunks.len()));
            map.insert("cdc_new_chunks".into(), serde_json::json!(0));
            map.insert("cdc_saved_bytes".into(), serde_json::json!(total_size));
        }
        return Ok(Json(obj));
    }

    // Verify all chunks exist in `media_cdc_chunks` and track how many were already referenced (saved!)
    let mut reused_chunks = 0usize;
    let mut saved_bytes: i64 = 0;
    for (_, _, len, h) in &parsed_chunks {
        let row = sqlx::query("SELECT byte_size, ref_count FROM media_cdc_chunks WHERE chunk_blake3 = $1 FOR UPDATE")
            .bind(&h[..])
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::BadRequest("missing_cdc_chunk"))?;
        let db_len: i32 = row.try_get("byte_size")?;
        if db_len != *len {
            return Err(AppError::BadRequest("cdc_chunk_size_mismatch"));
        }
        let rc: i32 = row.try_get("ref_count")?;
        if rc > 0 {
            reused_chunks += 1;
            saved_bytes += *len as i64;
        }
        sqlx::query("UPDATE media_cdc_chunks SET ref_count = ref_count + 1 WHERE chunk_blake3 = $1")
            .bind(&h[..])
            .execute(&mut *tx)
            .await?;
    }

    let oid = Uuid::new_v4();
    let storage_key = format!("cdc_recipe/{oid}");
    sqlx::query(
        r#"
        INSERT INTO media_objects
            (id, cas_scope, sha256, plain_sha256, blake3, pow_digest, is_cdc_assembled,
             storage_backend, storage_key, mime_type, file_ext, compression_codec, compression_ratio,
             byte_size, original_byte_size, ref_count, variant_kind)
        VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7, $8, $9, 'cdc', 'fastcdc_v2020', 1.0, $10, $10, 1, 'original')
        "#,
    )
    .bind(oid)
    .bind(&cas_scope)
    .bind(&total_sha[..])
    .bind(plain_sha.as_ref().map(|x| &x[..]))
    .bind(&total_b3[..])
    .bind(pow_arr.as_ref().map(|x| &x[..]))
    .bind(backend)
    .bind(&storage_key)
    .bind(mime)
    .bind(total_size)
    .execute(&mut *tx)
    .await?;

    for (idx, off, len, h) in &parsed_chunks {
        sqlx::query(
            "INSERT INTO media_cdc_recipes (object_id, chunk_index, chunk_blake3, byte_offset, byte_size) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(oid)
        .bind(*idx)
        .bind(&h[..])
        .bind(*off)
        .bind(*len)
        .execute(&mut *tx)
        .await?;
    }

    if !is_e2ee && matches!(req.scope_type.as_str(), "group" | "channel" | "library" | "sticker") {
        let vf_json = req.video_fingerprint.as_ref().and_then(|v| serde_json::to_value(v).ok());
        let af_json = req.audio_fingerprint.as_ref().and_then(|a| serde_json::to_value(a).ok());
        let _ = sqlx::query(
            r#"
            INSERT INTO media_dedup_indexes
                (object_id, scope_class, is_e2ee, blake3, orig_blake3, sha256, video_fingerprint, audio_fingerprint, byte_size)
            VALUES ($1, 'shared_non_e2ee', FALSE, $2, $2, $3, $4, $5, $6)
            ON CONFLICT (scope_class, blake3) DO NOTHING
            "#,
        )
        .bind(oid)
        .bind(&total_b3[..])
        .bind(&total_sha[..])
        .bind(&vf_json)
        .bind(&af_json)
        .bind(total_size)
        .execute(&mut *tx)
        .await;
        if let Some(anchor) = req.video_fingerprint.as_ref().and_then(|v| v.anchor_pdq()) {
            let _ = insert_pdq_mih_buckets(&mut tx, oid, "video_shared", &anchor).await;
        }
    }

    let ref_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO media_references
            (id, owner_id, scope_type, scope_id, message_id, object_id,
             original_filename, mime_type, byte_size, sha256_plain, sha256_cipher,
             storage_tier, availability_state)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'server_durable', 'server_durable')
        "#,
    )
    .bind(ref_id)
    .bind(user.id)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .bind(req.message_id)
    .bind(oid)
    .bind(&fname)
    .bind(mime)
    .bind(total_size)
    .bind(plain_sha.as_ref().map(|x| &x[..]))
    .bind(&total_sha[..])
    .execute(&mut *tx)
    .await?;

    let compat_path = format!("{backend}::{storage_key}");
    sqlx::query(
        r#"
        INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type, ref_id, scope_type, scope_id)
        VALUES ($1, $2, $3, $4, $5, $6, $1, $7, $8)
        "#,
    )
    .bind(ref_id)
    .bind(user.id)
    .bind(&compat_path)
    .bind(&fname)
    .bind(total_size)
    .bind(mime)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "id": ref_id,
        "ref_id": ref_id,
        "object_id": oid,
        "file_name": fname,
        "file_size": total_size,
        "mime_type": mime,
        "blake3": req.total_blake3_hex,
        "sha256": req.total_sha256_hex,
        "deduplicated": reused_chunks > 0,
        "dedup_match_kind": if reused_chunks > 0 { "fastcdc_subfile_chunks" } else { "fastcdc_new" },
        "cdc_total_chunks": parsed_chunks.len(),
        "cdc_reused_chunks": reused_chunks,
        "cdc_new_chunks": parsed_chunks.len() - reused_chunks,
        "cdc_saved_bytes": saved_bytes,
        "storage_tier": "server_durable",
        "availability_state": "server_durable",
        "download_url": format!("/api/attachments/{ref_id}"),
    })))
}

/// Transparently read an exact byte range `[start..=end]` (or whole file) from a FastCDC recipe object!
pub async fn read_cdc_object_range(
    pool: &PgPool,
    storage: &StorageRegistry,
    object_id: Uuid,
    range: Option<(u64, u64)>,
) -> AppResult<(Bytes, u64)> {
    let rows = sqlx::query(
        r#"
        SELECT r.byte_offset, r.byte_size, c.storage_backend, c.storage_key
        FROM media_cdc_recipes r
        JOIN media_cdc_chunks c ON c.chunk_blake3 = r.chunk_blake3
        WHERE r.object_id = $1
        ORDER BY r.chunk_index ASC
        "#,
    )
    .bind(object_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound("cdc_recipe_missing"));
    }

    let full_len: u64 = rows
        .iter()
        .map(|r| r.try_get::<i32, _>("byte_size").unwrap_or(0).max(0) as u64)
        .sum();

    let (req_start, req_end) = match range {
        Some((s, e)) => (s.min(full_len.saturating_sub(1)), e.min(full_len.saturating_sub(1))),
        None => (0, full_len.saturating_sub(1)),
    };
    let wanted_len = (req_end - req_start + 1) as usize;
    let mut out = Vec::with_capacity(wanted_len);

    for r in rows {
        let c_off = r.try_get::<i64, _>("byte_offset")? as u64;
        let c_len = r.try_get::<i32, _>("byte_size")? as u64;
        let c_end = c_off + c_len.saturating_sub(1);
        if c_end < req_start || c_off > req_end {
            continue;
        }
        let local_s = req_start.saturating_sub(c_off);
        let local_e = (req_end - c_off).min(c_len.saturating_sub(1));
        let backend: String = r.try_get("storage_backend")?;
        let key: String = r.try_get("storage_key")?;
        let (chunk_bytes, _) = match backend.as_str() {
            "s3_ready" => storage.s3.get_object_range(&key, local_s, local_e).await,
            _ => storage.local_cas.get_object_range(&key, local_s, local_e).await,
        }
        .map_err(|_| AppError::NotFound("cdc_chunk_missing"))?;
        out.extend_from_slice(&chunk_bytes);
    }

    Ok((Bytes::from(out), full_len))
}
