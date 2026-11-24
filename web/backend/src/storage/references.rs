//! Storage References Metadata, Presigned Access, Relay Acknowledgments, Device Location Reporting & Metrics.
//! Extracted from `storage/router.rs` as a dedicated domain submodule (Spec 051).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    storage::{
        StorageAdapter, migration,
        permissions::{authorize_read, authorize_write},
        router::{auth_caller, decode_hex_32},
    },
    websocket::WsEvent,
};
use chatw_media_core::dedup::{
    AudioFingerprint, PdqHash256, VideoFingerprint, audio_fingerprints_equivalent,
    evaluate_dedup_policy, videos_are_safe_dedup_equivalent,
};

pub async fn handle_get_ref_meta(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query(
        r#"
        SELECT r.owner_id, r.scope_type, r.scope_id, r.original_filename, r.mime_type, r.byte_size,
               r.storage_tier, r.availability_state, r.min_safe_copies, r.deleted_for_all,
               r.relay_expires_at, r.object_id, r.optimized_object_id, r.thumb_object_id,
               encode(r.sha256_plain, 'hex') AS sha_plain, encode(r.sha256_cipher, 'hex') AS sha_cipher,
               o.storage_backend, encode(o.blake3, 'hex') AS blake3_hex, o.compression_codec,
               o.compression_ratio, o.quality_ssim, o.media_class, o.ai_recommended
        FROM media_references r
        LEFT JOIN media_objects o ON o.id = r.object_id
        WHERE r.id = $1
        "#,
    )
    .bind(ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("reference_not_found"))?;

    let owner_id: Uuid = r.try_get("owner_id")?;
    let scope_type: String = r.try_get("scope_type")?;
    let scope_id: Option<Uuid> = r.try_get("scope_id")?;
    authorize_read(&state.db, user.id, &scope_type, scope_id, owner_id).await?;

    let locs = sqlx::query(
        r#"
        SELECT mdl.device_id, mdl.user_id, mdl.has_original, mdl.has_optimized, mdl.local_tier, mdl.last_verified_at,
               d.device_label
        FROM media_device_locations mdl
        LEFT JOIN devices d ON d.id = mdl.device_id
        WHERE mdl.ref_id = $1
        "#,
    )
    .bind(ref_id)
    .fetch_all(&state.db)
    .await?;

    let devices_json: Vec<serde_json::Value> = locs
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "device_id": l.try_get::<Uuid, _>("device_id").ok(),
                "is_own": l.try_get::<Uuid, _>("user_id").ok() == Some(user.id),
                "device_label": l.try_get::<Option<String>, _>("device_label").ok().flatten(),
                "has_original": l.try_get::<bool, _>("has_original").unwrap_or(false),
                "has_optimized": l.try_get::<bool, _>("has_optimized").unwrap_or(false),
                "local_tier": l.try_get::<String, _>("local_tier").unwrap_or_default(),
                "last_verified_at": l.try_get::<chrono::DateTime<chrono::Utc>, _>("last_verified_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "ref_id": ref_id,
        "scope_type": scope_type,
        "scope_id": scope_id,
        "file_name": r.try_get::<String, _>("original_filename")?,
        "mime_type": r.try_get::<String, _>("mime_type")?,
        "byte_size": r.try_get::<i64, _>("byte_size")?,
        "storage_tier": r.try_get::<String, _>("storage_tier")?,
        "storage_backend": r.try_get::<Option<String>, _>("storage_backend")?,
        "availability_state": r.try_get::<String, _>("availability_state")?,
        "deleted_for_all": r.try_get::<bool, _>("deleted_for_all")?,
        "relay_expires_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("relay_expires_at")?,
        "has_server_copy": r.try_get::<Option<Uuid>, _>("object_id")?.is_some(),
        "has_optimized": r.try_get::<Option<Uuid>, _>("optimized_object_id")?.is_some(),
        "has_thumb": r.try_get::<Option<Uuid>, _>("thumb_object_id")?.is_some(),
        "sha256_plain": r.try_get::<Option<String>, _>("sha_plain")?,
        "sha256_cipher": r.try_get::<Option<String>, _>("sha_cipher")?,
        "blake3": r.try_get::<Option<String>, _>("blake3_hex")?,
        "compression_codec": r.try_get::<Option<String>, _>("compression_codec")?,
        "compression_ratio": r.try_get::<Option<f32>, _>("compression_ratio")?,
        "quality_ssim": r.try_get::<Option<f32>, _>("quality_ssim")?,
        "media_class": r.try_get::<Option<String>, _>("media_class")?,
        "ai_recommended": r.try_get::<Option<bool>, _>("ai_recommended")?.unwrap_or(false),
        "device_locations": devices_json,
    })))
}

pub async fn handle_presign_ref(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query(
        r#"
        SELECT r.owner_id, r.scope_type, r.scope_id, o.storage_key
        FROM media_references r
        JOIN media_objects o ON o.id = r.object_id
        WHERE r.id = $1
        "#,
    )
    .bind(ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("reference_not_found"))?;
    authorize_read(
        &state.db,
        user.id,
        &r.try_get::<String, _>("scope_type")?,
        r.try_get::<Option<Uuid>, _>("scope_id")?,
        r.try_get::<Uuid, _>("owner_id")?,
    )
    .await?;

    let key: String = r.try_get("storage_key")?;
    match state.storage.s3.create_temporary_access(&key, "GET", 900).await {
        Ok(access) => Ok(Json(serde_json::json!({ "mode": "s3_presigned", "access": access }))),
        Err(_) => Ok(Json(serde_json::json!({
            "mode": "local_proxy",
            "url": format!("/api/attachments/{ref_id}"),
        }))),
    }
}

#[derive(Deserialize, Default)]
pub struct RelayAckBody {
    pub local_tier: Option<String>,
    pub sha256_hex: Option<String>,
    pub has_original: Option<bool>,
    pub has_optimized: Option<bool>,
}

pub async fn handle_relay_ack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
    Json(body): Json<RelayAckBody>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let device_id = user.device_id.ok_or(AppError::BadRequest("device_not_registered"))?;

    let r = sqlx::query(
        "SELECT owner_id, scope_type, scope_id, storage_tier, min_safe_copies, object_id FROM media_references WHERE id = $1",
    )
    .bind(ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("reference_not_found"))?;

    let owner_id: Uuid = r.try_get("owner_id")?;
    let scope_type: String = r.try_get("scope_type")?;
    let scope_id: Option<Uuid> = r.try_get("scope_id")?;
    authorize_read(&state.db, user.id, &scope_type, scope_id, owner_id).await?;

    let tier = body.local_tier.as_deref().filter(|t| matches!(*t, "opfs" | "indexeddb" | "cache_api" | "native_fs" | "server_relay")).unwrap_or("indexeddb");
    let sha_bytes = decode_hex_32(body.sha256_hex.as_deref());
    sqlx::query(
        r#"
        INSERT INTO media_device_locations (ref_id, device_id, user_id, has_original, has_optimized, local_tier, verified_sha256, last_verified_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, now())
        ON CONFLICT (ref_id, device_id) DO UPDATE SET
            has_original = EXCLUDED.has_original,
            has_optimized = EXCLUDED.has_optimized,
            local_tier = EXCLUDED.local_tier,
            verified_sha256 = COALESCE(EXCLUDED.verified_sha256, media_device_locations.verified_sha256),
            last_verified_at = now()
        "#,
    )
    .bind(ref_id)
    .bind(device_id)
    .bind(user.id)
    .bind(body.has_original.unwrap_or(true))
    .bind(body.has_optimized.unwrap_or(false))
    .bind(tier)
    .bind(sha_bytes.as_ref().map(|x| &x[..]))
    .execute(&state.db)
    .await?;

    let copy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT device_id) FROM media_device_locations WHERE ref_id = $1 AND local_tier <> 'server_relay'",
    )
    .bind(ref_id)
    .fetch_one(&state.db)
    .await?;
    let distinct_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM media_device_locations WHERE ref_id = $1 AND local_tier <> 'server_relay'",
    )
    .bind(ref_id)
    .fetch_one(&state.db)
    .await?;

    let min_safe: i32 = r.try_get("min_safe_copies")?;
    let storage_tier: String = r.try_get("storage_tier")?;
    let object_id: Option<Uuid> = r.try_get("object_id")?;

    let mut relay_cleaned = false;
    let new_state = if distinct_users >= 2 {
        "available_on_peer"
    } else if copy_count >= 2 {
        "replicated_multi_device"
    } else {
        "local_only"
    };

    if storage_tier == "temp_relay" && copy_count >= min_safe as i64 && distinct_users >= 2 {
        if let Some(oid) = object_id {
            let _ = migration::release_object_reference(&state.db, &state.storage, oid).await;
            relay_cleaned = true;
        }
        sqlx::query(
            "UPDATE media_references SET object_id = NULL, storage_tier = 'dm_client_preferred', availability_state = $2, updated_at = now() WHERE id = $1",
        )
        .bind(ref_id)
        .bind(new_state)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE media_references SET availability_state = $2, updated_at = now() WHERE id = $1 AND availability_state NOT IN ('server_durable','backed_up_encrypted','deleted_for_everyone')",
        )
        .bind(ref_id)
        .bind(new_state)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(serde_json::json!({
        "ref_id": ref_id,
        "local_copy_count": copy_count,
        "distinct_users": distinct_users,
        "relay_cleaned": relay_cleaned,
        "availability_state": new_state,
    })))
}

#[derive(Deserialize)]
pub struct BatchLocationsReq {
    pub items: Vec<LocationItem>,
}

#[derive(Deserialize)]
pub struct LocationItem {
    pub ref_id: Uuid,
    pub local_tier: String,
    pub sha256_hex: Option<String>,
    pub has_original: Option<bool>,
}

pub async fn handle_report_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BatchLocationsReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let device_id = user.device_id.ok_or(AppError::BadRequest("device_not_registered"))?;
    let mut updated = 0usize;
    for item in req.items.into_iter().take(200) {
        let tier = if matches!(item.local_tier.as_str(), "opfs" | "indexeddb" | "cache_api" | "native_fs") {
            item.local_tier.as_str()
        } else {
            "indexeddb"
        };
        let sha_bytes = decode_hex_32(item.sha256_hex.as_deref());
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM media_references WHERE id = $1 AND (owner_id = $2 OR scope_id = $2 OR scope_type IN ('group','channel')))",
        )
        .bind(item.ref_id)
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
        if !exists {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO media_device_locations (ref_id, device_id, user_id, has_original, local_tier, verified_sha256, last_verified_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            ON CONFLICT (ref_id, device_id) DO UPDATE SET local_tier = EXCLUDED.local_tier, verified_sha256 = COALESCE(EXCLUDED.verified_sha256, media_device_locations.verified_sha256), last_verified_at = now()
            "#,
        )
        .bind(item.ref_id)
        .bind(device_id)
        .bind(user.id)
        .bind(item.has_original.unwrap_or(true))
        .bind(tier)
        .bind(sha_bytes.as_ref().map(|x| &x[..]))
        .execute(&state.db)
        .await?;
        updated += 1;
    }
    Ok(Json(serde_json::json!({ "reported": updated })))
}

#[derive(Deserialize)]
pub struct DeleteRefQuery {
    pub scope: Option<String>,
    pub force: Option<bool>,
}

pub async fn handle_delete_reference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
    Query(q): Query<DeleteRefQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query(
        "SELECT owner_id, scope_type, scope_id, object_id, optimized_object_id, thumb_object_id, availability_state FROM media_references WHERE id = $1",
    )
    .bind(ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("reference_not_found"))?;
    let owner_id: Uuid = r.try_get("owner_id")?;
    let scope_type: String = r.try_get("scope_type")?;
    let scope_id: Option<Uuid> = r.try_get("scope_id")?;
    let avail: String = r.try_get("availability_state")?;

    let scope = q.scope.as_deref().unwrap_or("local_cache");
    if scope == "local_cache" {
        if avail == "local_only" && !q.force.unwrap_or(false) {
            return Err(AppError::BadRequest("unique_local_file_requires_force"));
        }
        if let Some(did) = user.device_id {
            sqlx::query("DELETE FROM media_device_locations WHERE ref_id = $1 AND device_id = $2")
                .bind(ref_id)
                .bind(did)
                .execute(&state.db)
                .await?;
        }
        return Ok(Json(serde_json::json!({ "deleted_scope": "local_cache", "ref_id": ref_id })));
    }

    if owner_id != user.id {
        return Err(AppError::Forbidden("not_media_owner"));
    }

    for col in ["object_id", "optimized_object_id", "thumb_object_id"] {
        if let Ok(Some(oid)) = r.try_get::<Option<Uuid>, _>(col) {
            let _ = migration::release_object_reference(&state.db, &state.storage, oid).await;
        }
    }
    sqlx::query(
        "UPDATE media_references SET object_id = NULL, optimized_object_id = NULL, thumb_object_id = NULL, deleted_for_all = TRUE, availability_state = 'deleted_for_everyone', updated_at = now() WHERE id = $1",
    )
    .bind(ref_id)
    .execute(&state.db)
    .await?;
    sqlx::query("DELETE FROM media_device_locations WHERE ref_id = $1")
        .bind(ref_id)
        .execute(&state.db)
        .await?;

    let payload = serde_json::json!({
        "ref_id": ref_id,
        "scope_type": scope_type,
        "scope_id": scope_id,
    });
    if scope_type == "dm" {
        if let Some(peer) = scope_id {
            state.ws_hub.send_to_users(&[user.id, peer], WsEvent::MediaDeleteForAll(payload)).await;
        } else {
            state.ws_hub.send_to_user(&user.id, WsEvent::MediaDeleteForAll(payload)).await;
        }
    } else {
        state.ws_hub.send_to_user(&user.id, WsEvent::MediaDeleteForAll(payload)).await;
    }

    Ok(Json(serde_json::json!({ "deleted_scope": "everyone", "ref_id": ref_id })))
}

pub async fn handle_storage_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;

    let user_refs = sqlx::query(
        r#"
        SELECT
            COUNT(*)::bigint AS total_refs,
            COALESCE(SUM(byte_size), 0)::bigint AS total_ref_bytes,
            COUNT(*) FILTER (WHERE scope_type = 'dm')::bigint AS dm_refs,
            COALESCE(SUM(byte_size) FILTER (WHERE scope_type = 'dm'), 0)::bigint AS dm_bytes,
            COUNT(*) FILTER (WHERE scope_type = 'group')::bigint AS group_refs,
            COALESCE(SUM(byte_size) FILTER (WHERE scope_type = 'group'), 0)::bigint AS group_bytes,
            COUNT(*) FILTER (WHERE scope_type = 'channel')::bigint AS channel_refs,
            COALESCE(SUM(byte_size) FILTER (WHERE scope_type = 'channel'), 0)::bigint AS channel_bytes,
            COUNT(*) FILTER (WHERE storage_tier = 'temp_relay')::bigint AS active_relay_refs,
            COALESCE(SUM(byte_size) FILTER (WHERE storage_tier = 'temp_relay'), 0)::bigint AS active_relay_bytes,
            COUNT(*) FILTER (WHERE availability_state = 'local_only')::bigint AS local_only_count,
            COUNT(*) FILTER (WHERE availability_state = 'replicated_multi_device')::bigint AS replicated_count,
            COUNT(*) FILTER (WHERE availability_state = 'backed_up_encrypted')::bigint AS backed_up_count,
            COUNT(*) FILTER (WHERE availability_state = 'no_remaining_copies')::bigint AS lost_count
        FROM media_references
        WHERE deleted_for_all = FALSE AND (owner_id = $1 OR scope_id = $1)
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let cas_stats = sqlx::query(
        r#"
        SELECT
            COUNT(*)::bigint AS unique_objects,
            COALESCE(SUM(byte_size), 0)::bigint AS physical_bytes,
            COALESCE(SUM(original_byte_size * ref_count), 0)::bigint AS logical_bytes,
            COALESCE(SUM(byte_size) FILTER (WHERE storage_backend = 'temp_relay'), 0)::bigint AS relay_physical_bytes,
            COALESCE(SUM(byte_size) FILTER (WHERE storage_backend = 'encrypted_backup'), 0)::bigint AS backup_physical_bytes,
            COALESCE(SUM(GREATEST(0, original_byte_size - byte_size)), 0)::bigint AS compression_saved_bytes
        FROM media_objects
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let logical_bytes: i64 = cas_stats.try_get("logical_bytes")?;
    let physical_bytes: i64 = cas_stats.try_get("physical_bytes")?;
    let dedup_saved_bytes = (logical_bytes - physical_bytes).max(0);

    let rec_stats = sqlx::query(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'completed')::bigint AS completed,
            COUNT(*) FILTER (WHERE status IN ('pending', 'queued_offline_peer', 'transferring'))::bigint AS pending,
            COUNT(*) FILTER (WHERE status IN ('denied', 'no_remaining_copies', 'expired'))::bigint AS failed
        FROM recovery_requests
        WHERE requester_user_id = $1 OR target_user_id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let expiring_at_risk: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM media_references r
        WHERE (r.owner_id = $1 OR r.scope_id = $1)
          AND r.storage_tier = 'temp_relay'
          AND r.relay_expires_at IS NOT NULL
          AND r.relay_expires_at < now() + interval '48 hours'
          AND (SELECT COUNT(DISTINCT user_id) FROM media_device_locations mdl WHERE mdl.ref_id = r.id AND mdl.local_tier <> 'server_relay') < 2
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let cdc_stats = sqlx::query(
        r#"
        SELECT
            COUNT(*)::bigint AS unique_chunks,
            COALESCE(SUM(byte_size::bigint), 0)::bigint AS physical_chunk_bytes,
            COALESCE(SUM(byte_size::bigint * GREATEST(ref_count, 1)), 0)::bigint AS logical_chunk_bytes
        FROM media_cdc_chunks
        "#,
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let cdc_unique_chunks: i64 = cdc_stats.as_ref().and_then(|r| r.try_get("unique_chunks").ok()).unwrap_or(0);
    let cdc_phys: i64 = cdc_stats.as_ref().and_then(|r| r.try_get("physical_chunk_bytes").ok()).unwrap_or(0);
    let cdc_log: i64 = cdc_stats.as_ref().and_then(|r| r.try_get("logical_chunk_bytes").ok()).unwrap_or(0);
    let cdc_saved_bytes = (cdc_log - cdc_phys).max(0);
    let mih_indexed_images: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT object_id) FROM media_pdq_mih_buckets")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "user": {
            "total_refs": user_refs.try_get::<i64, _>("total_refs")?,
            "total_ref_bytes": user_refs.try_get::<i64, _>("total_ref_bytes")?,
            "dm_refs": user_refs.try_get::<i64, _>("dm_refs")?,
            "dm_bytes": user_refs.try_get::<i64, _>("dm_bytes")?,
            "group_refs": user_refs.try_get::<i64, _>("group_refs")?,
            "group_bytes": user_refs.try_get::<i64, _>("group_bytes")?,
            "channel_refs": user_refs.try_get::<i64, _>("channel_refs")?,
            "channel_bytes": user_refs.try_get::<i64, _>("channel_bytes")?,
            "active_relay_refs": user_refs.try_get::<i64, _>("active_relay_refs")?,
            "active_relay_bytes": user_refs.try_get::<i64, _>("active_relay_bytes")?,
            "local_only_count": user_refs.try_get::<i64, _>("local_only_count")?,
            "replicated_count": user_refs.try_get::<i64, _>("replicated_count")?,
            "backed_up_count": user_refs.try_get::<i64, _>("backed_up_count")?,
            "lost_count": user_refs.try_get::<i64, _>("lost_count")?,
            "expiring_without_safe_copies": expiring_at_risk,
        },
        "server": {
            "unique_objects": cas_stats.try_get::<i64, _>("unique_objects")?,
            "physical_bytes": physical_bytes,
            "logical_bytes": logical_bytes,
            "dedup_saved_bytes": dedup_saved_bytes + cdc_saved_bytes,
            "cdc_unique_chunks": cdc_unique_chunks,
            "cdc_saved_bytes": cdc_saved_bytes,
            "mih_indexed_images": mih_indexed_images,
            "compression_saved_bytes": cas_stats.try_get::<i64, _>("compression_saved_bytes")?,
            "relay_physical_bytes": cas_stats.try_get::<i64, _>("relay_physical_bytes")?,
            "backup_physical_bytes": cas_stats.try_get::<i64, _>("backup_physical_bytes")?,
        },
        "recovery": {
            "completed": rec_stats.try_get::<i64, _>("completed")?,
            "pending": rec_stats.try_get::<i64, _>("pending")?,
            "failed": rec_stats.try_get::<i64, _>("failed")?,
        }
    })))
}

pub async fn handle_cleanup_relays(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<migration::RelayCleanupOutcome>> {
    let _user = auth_caller(&state, &headers).await?;
    let outcome = migration::cleanup_relays_safely(&state.db, &state.storage).await?;
    Ok(Json(outcome))
}

#[derive(Deserialize, Default)]
pub struct RunMigrationReq {
    pub dry_run: Option<bool>,
}

pub async fn handle_run_migration(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<RunMigrationReq>>,
) -> AppResult<Json<migration::MigrationJobResult>> {
    let _user = auth_caller(&state, &headers).await?;
    let dry_run = body.and_then(|b| b.0.dry_run).unwrap_or(true);
    let res = migration::execute_migration_job(&state.db, &state.storage, dry_run).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct DedupCheckReq {
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub is_e2ee: bool,
    pub blake3_hex: Option<String>,
    pub orig_blake3_hex: Option<String>,
    pub sha256_hex: Option<String>,
    pub perceptual_hash_hex: Option<String>,
    pub perceptual_quality: Option<u8>,
    pub caption_band_hash: Option<u64>,
    pub color_moment_hash: Option<u64>,
    pub aspect_ratio_milli: Option<u16>,
    pub video_fingerprint: Option<VideoFingerprint>,
    pub audio_fingerprint: Option<AudioFingerprint>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Privacy-enforced Smart Deduplication Check (`POST /api/storage/dedup/check`).
/// STRICT INVARIANT: If `is_e2ee == true`, NEVER performs a global plaintext or perceptual DB lookup!
pub async fn handle_dedup_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DedupCheckReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    authorize_write(&state.db, user.id, &req.scope_type, req.scope_id).await?;
    if crate::storage::router::is_blocked_svg_upload(req.file_name.as_deref(), req.mime_type.as_deref(), None, None) {
        return Err(AppError::BadRequest("svg_upload_not_allowed"));
    }

    let probe_key = req
        .blake3_hex
        .as_deref()
        .or(req.sha256_hex.as_deref())
        .unwrap_or("probe");
    let decision = evaluate_dedup_policy(
        &req.scope_type,
        &user.id.to_string(),
        req.scope_id.map(|u| u.to_string()).as_deref(),
        req.is_e2ee,
        false,
        probe_key,
    );

    if !decision.allow_exact_global_lookup {
        return Ok(Json(serde_json::json!({
            "dedup_allowed": false,
            "matched": false,
            "match_kind": "none",
            "reason": decision.reason,
        })));
    }

    // 1. Exact cryptographic match (BLAKE3 primary, SHA-256 fallback) in shared_non_e2ee index
    if let Some(b3_arr) = decode_hex_32(req.blake3_hex.as_deref()) {
        if let Some(row) = sqlx::query(
            r#"
            SELECT d.object_id, o.storage_backend, o.storage_key, o.byte_size
            FROM media_dedup_indexes d
            JOIN media_objects o ON o.id = d.object_id
            WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.blake3 = $1
            LIMIT 1
            "#,
        )
        .bind(&b3_arr[..])
        .fetch_optional(&state.db)
        .await?
        {
            let oid: Uuid = row.try_get("object_id")?;
            let backend: String = row.try_get("storage_backend")?;
            let key: String = row.try_get("storage_key")?;
            let exists = match backend.as_str() {
                "s3_ready" => state.storage.s3.object_exists(&key).await.unwrap_or(false),
                _ => state.storage.local_cas.object_exists(&key).await.unwrap_or(false),
            };
            if exists {
                return Ok(Json(serde_json::json!({
                    "dedup_allowed": true,
                    "matched": true,
                    "match_kind": "exact_blake3",
                    "object_id": oid,
                    "byte_size": row.try_get::<i64, _>("byte_size")?,
                })));
            }
        }
    }

    if let Some(sha_arr) = decode_hex_32(req.sha256_hex.as_deref()) {
        if let Some(row) = sqlx::query(
            r#"
            SELECT d.object_id, o.storage_backend, o.storage_key, o.byte_size
            FROM media_dedup_indexes d
            JOIN media_objects o ON o.id = d.object_id
            WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.sha256 = $1
            LIMIT 1
            "#,
        )
        .bind(&sha_arr[..])
        .fetch_optional(&state.db)
        .await?
        {
            let oid: Uuid = row.try_get("object_id")?;
            let backend: String = row.try_get("storage_backend")?;
            let key: String = row.try_get("storage_key")?;
            let exists = match backend.as_str() {
                "s3_ready" => state.storage.s3.object_exists(&key).await.unwrap_or(false),
                _ => state.storage.local_cas.object_exists(&key).await.unwrap_or(false),
            };
            if exists {
                return Ok(Json(serde_json::json!({
                    "dedup_allowed": true,
                    "matched": true,
                    "match_kind": "exact_sha256",
                    "object_id": oid,
                    "byte_size": row.try_get::<i64, _>("byte_size")?,
                })));
            }
        }
    }

    // 2. O(1) 16-Band Multi-Index Hashing (MIH) PDQ 256-bit DCT match with Triple Meme/Color/Aspect Guard
    if decision.allow_perceptual_lookup {
        if let Some(pdq_hex) = req.perceptual_hash_hex.as_deref() {
            if let Some(incoming_pdq) = PdqHash256::from_hex_full(
                pdq_hex,
                req.perceptual_quality.unwrap_or(0),
                req.caption_band_hash.unwrap_or(0),
                req.color_moment_hash.unwrap_or(0),
                req.aspect_ratio_milli.unwrap_or(0),
            ) {
                let incoming_px = (req.width.unwrap_or(0) as u64) * (req.height.unwrap_or(0) as u64);
                if let Some(hit) = crate::storage::dedup_engine::find_pdq_mih_match(
                    &state.db,
                    &state.storage,
                    "shared_non_e2ee",
                    &incoming_pdq,
                    incoming_px,
                )
                .await?
                {
                    return Ok(Json(serde_json::json!({
                        "dedup_allowed": true,
                        "matched": true,
                        "match_kind": "perceptual_pdq_mih_candidate",
                        "hamming_distance": hit.hamming,
                        "object_id": hit.object_id,
                        "byte_size": hit.byte_size,
                        "orig_profile_json": hit.orig_profile_json,
                        "ai_recommended": hit.ai_recommended,
                    })));
                }
            }
        }

        // 4. Audio 128-bit Spectral Fingerprint match
        if let Some(ref afp) = req.audio_fingerprint {
            let audio_cands = sqlx::query(
                r#"
                SELECT d.object_id, d.audio_fingerprint, o.storage_backend, o.storage_key, o.byte_size, o.is_cdc_assembled
                FROM media_dedup_indexes d
                JOIN media_objects o ON o.id = d.object_id
                WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.audio_fingerprint IS NOT NULL
                ORDER BY d.created_at DESC
                LIMIT 150
                "#,
            )
            .fetch_all(&state.db)
            .await?;
            for cand in audio_cands {
                if let Ok(val) = cand.try_get::<serde_json::Value, _>("audio_fingerprint") {
                    if let Ok(cand_afp) = serde_json::from_value::<AudioFingerprint>(val) {
                        if audio_fingerprints_equivalent(afp, &cand_afp) {
                            let oid: Uuid = cand.try_get("object_id")?;
                            let backend: String = cand.try_get("storage_backend")?;
                            let key: String = cand.try_get("storage_key")?;
                            let is_cdc: bool = cand.try_get("is_cdc_assembled").unwrap_or(false);
                            if crate::storage::dedup_engine::physical_object_available(&state.storage, &backend, &key, is_cdc).await {
                                return Ok(Json(serde_json::json!({
                                    "dedup_allowed": true,
                                    "matched": true,
                                    "match_kind": "audio_spectral_match",
                                    "object_id": oid,
                                    "byte_size": cand.try_get::<i64, _>("byte_size")?,
                                })));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Multi-keyframe vPDQ video fingerprint match (duration <= 3% difference & >= 90% keyframe hashes match)
    if decision.allow_perceptual_lookup {
        if let Some(ref vf) = req.video_fingerprint {
            let video_cands = sqlx::query(
                r#"
                SELECT d.object_id, d.video_fingerprint, o.storage_backend, o.storage_key, o.byte_size
                FROM media_dedup_indexes d
                JOIN media_objects o ON o.id = d.object_id
                WHERE d.scope_class = 'shared_non_e2ee' AND d.is_e2ee = FALSE AND d.video_fingerprint IS NOT NULL
                ORDER BY d.created_at DESC
                LIMIT 100
                "#,
            )
            .fetch_all(&state.db)
            .await?;

            for cand in video_cands {
                if let Ok(cand_val) = cand.try_get::<serde_json::Value, _>("video_fingerprint") {
                    if let Ok(cand_vf) = serde_json::from_value::<VideoFingerprint>(cand_val) {
                        if videos_are_safe_dedup_equivalent(vf, &cand_vf) {
                            let oid: Uuid = cand.try_get("object_id")?;
                            let backend: String = cand.try_get("storage_backend")?;
                            let key: String = cand.try_get("storage_key")?;
                            let exists = match backend.as_str() {
                                "s3_ready" => state.storage.s3.object_exists(&key).await.unwrap_or(false),
                                _ => state.storage.local_cas.object_exists(&key).await.unwrap_or(false),
                            };
                            if exists {
                                return Ok(Json(serde_json::json!({
                                    "dedup_allowed": true,
                                    "matched": true,
                                    "match_kind": "video_fingerprint_match",
                                    "object_id": oid,
                                    "byte_size": cand.try_get::<i64, _>("byte_size")?,
                                })));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "dedup_allowed": true,
        "matched": false,
        "match_kind": "none",
        "reason": decision.reason,
    })))
}

#[derive(Deserialize)]
pub struct PutManifestChunk {
    pub variant_id: Option<String>,
    pub chunk_index: i32,
    pub byte_offset: i64,
    pub cipher_len: i32,
    pub plain_len: i32,
    pub start_ms: Option<i64>,
    pub duration_ms: Option<i32>,
    pub is_keyframe: Option<bool>,
    pub blake3_hex: Option<String>,
}

#[derive(Deserialize)]
pub struct PutManifestReq {
    pub version: Option<i16>,
    pub mime_type: String,
    pub codec: String,
    pub duration_ms: Option<i64>,
    pub is_e2ee: Option<bool>,
    pub sealed_manifest_b64: Option<String>,
    pub variants_json: Option<serde_json::Value>,
    pub chunks: Option<Vec<PutManifestChunk>>,
}

/// Store an authenticated E2EE streaming manifest + chunk byte-offset index (`POST /api/storage/references/{id}/manifest`).
pub async fn handle_put_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
    Json(req): Json<PutManifestReq>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query("SELECT owner_id FROM media_references WHERE id = $1")
        .bind(ref_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("reference_not_found"))?;
    let owner_id: Uuid = r.try_get("owner_id")?;
    if owner_id != user.id {
        return Err(AppError::Forbidden("not_manifest_owner"));
    }

    let chunks = req.chunks.unwrap_or_default();
    let total_segments = chunks.len() as i32;
    let variants = req.variants_json.unwrap_or_else(|| serde_json::json!([]));

    let mut tx = state.db.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO media_chunk_manifests
            (ref_id, version, mime_type, codec, duration_ms, is_e2ee, sealed_manifest_b64, variants_json, total_segments, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
        ON CONFLICT (ref_id) DO UPDATE SET
            version = EXCLUDED.version,
            mime_type = EXCLUDED.mime_type,
            codec = EXCLUDED.codec,
            duration_ms = EXCLUDED.duration_ms,
            is_e2ee = EXCLUDED.is_e2ee,
            sealed_manifest_b64 = EXCLUDED.sealed_manifest_b64,
            variants_json = EXCLUDED.variants_json,
            total_segments = EXCLUDED.total_segments,
            updated_at = now()
        "#,
    )
    .bind(ref_id)
    .bind(req.version.unwrap_or(3))
    .bind(&req.mime_type)
    .bind(&req.codec)
    .bind(req.duration_ms.unwrap_or(0))
    .bind(req.is_e2ee.unwrap_or(true))
    .bind(&req.sealed_manifest_b64)
    .bind(&variants)
    .bind(total_segments)
    .execute(&mut *tx)
    .await?;

    if !chunks.is_empty() {
        sqlx::query("DELETE FROM media_chunks WHERE ref_id = $1")
            .bind(ref_id)
            .execute(&mut *tx)
            .await?;
        for c in chunks.into_iter().take(2000) {
            let b3 = decode_hex_32(c.blake3_hex.as_deref());
            sqlx::query(
                r#"
                INSERT INTO media_chunks
                    (ref_id, variant_id, chunk_index, byte_offset, cipher_len, plain_len, start_ms, duration_ms, is_keyframe, blake3)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(ref_id)
            .bind(c.variant_id.as_deref().unwrap_or("default"))
            .bind(c.chunk_index)
            .bind(c.byte_offset.max(0))
            .bind(c.cipher_len.max(1))
            .bind(c.plain_len.max(0))
            .bind(c.start_ms.unwrap_or(0))
            .bind(c.duration_ms.unwrap_or(0))
            .bind(c.is_keyframe.unwrap_or(false))
            .bind(b3.as_ref().map(|x| &x[..]))
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "ref_id": ref_id,
        "total_segments": total_segments,
    })))
}

/// Fetch streaming manifest + chunk byte ranges (`GET /api/storage/references/{id}/manifest`), enforced by `authorize_read`.
pub async fn handle_get_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query("SELECT owner_id, scope_type, scope_id, deleted_for_all FROM media_references WHERE id = $1")
        .bind(ref_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("reference_not_found"))?;
    let owner_id: Uuid = r.try_get("owner_id")?;
    let scope_type: String = r.try_get("scope_type")?;
    let scope_id: Option<Uuid> = r.try_get("scope_id")?;
    authorize_read(&state.db, user.id, &scope_type, scope_id, owner_id).await?;
    if r.try_get::<bool, _>("deleted_for_all")? {
        return Err(AppError::NotFound("deleted_for_everyone"));
    }

    let m = sqlx::query(
        r#"
        SELECT version, mime_type, codec, duration_ms, is_e2ee, sealed_manifest_b64, variants_json, total_segments
        FROM media_chunk_manifests WHERE ref_id = $1
        "#,
    )
    .bind(ref_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("manifest_not_found"))?;

    let chunk_rows = sqlx::query(
        r#"
        SELECT variant_id, chunk_index, byte_offset, cipher_len, plain_len, start_ms, duration_ms,
               is_keyframe, encode(blake3, 'hex') AS blake3_hex
        FROM media_chunks
        WHERE ref_id = $1
        ORDER BY variant_id ASC, chunk_index ASC
        "#,
    )
    .bind(ref_id)
    .fetch_all(&state.db)
    .await?;

    let chunks_json: Vec<serde_json::Value> = chunk_rows
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "variant_id": c.try_get::<String, _>("variant_id").unwrap_or_default(),
                "chunk_index": c.try_get::<i32, _>("chunk_index").unwrap_or(0),
                "byte_offset": c.try_get::<i64, _>("byte_offset").unwrap_or(0),
                "cipher_len": c.try_get::<i32, _>("cipher_len").unwrap_or(0),
                "plain_len": c.try_get::<i32, _>("plain_len").unwrap_or(0),
                "start_ms": c.try_get::<i64, _>("start_ms").unwrap_or(0),
                "duration_ms": c.try_get::<i32, _>("duration_ms").unwrap_or(0),
                "is_keyframe": c.try_get::<bool, _>("is_keyframe").unwrap_or(false),
                "blake3_hex": c.try_get::<Option<String>, _>("blake3_hex").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "ref_id": ref_id,
        "version": m.try_get::<i16, _>("version")?,
        "mime_type": m.try_get::<String, _>("mime_type")?,
        "codec": m.try_get::<String, _>("codec")?,
        "duration_ms": m.try_get::<i64, _>("duration_ms")?,
        "is_e2ee": m.try_get::<bool, _>("is_e2ee")?,
        "sealed_manifest_b64": m.try_get::<Option<String>, _>("sealed_manifest_b64")?,
        "variants_json": m.try_get::<serde_json::Value, _>("variants_json")?,
        "total_segments": m.try_get::<i32, _>("total_segments")?,
        "chunks": chunks_json,
    })))
}

/// Abort expired multipart uploads and sweep unpinned zero-ref GC objects (`POST /api/storage/gc/incomplete`).
pub async fn handle_gc_incomplete(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<migration::IncompleteAndGcReport>> {
    let _user = auth_caller(&state, &headers).await?;
    let report = migration::cleanup_incomplete_uploads_and_gc(&state.db, &state.storage).await?;
    Ok(Json(report))
}
