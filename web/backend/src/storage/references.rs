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
        permissions::authorize_read,
        router::{auth_caller, decode_hex_32},
    },
    websocket::WsEvent,
};

pub async fn handle_get_ref_meta(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ref_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user = auth_caller(&state, &headers).await?;
    let r = sqlx::query(
        r#"
        SELECT owner_id, scope_type, scope_id, original_filename, mime_type, byte_size,
               storage_tier, availability_state, min_safe_copies, deleted_for_all,
               relay_expires_at, object_id, optimized_object_id, thumb_object_id,
               encode(sha256_plain, 'hex') AS sha_plain, encode(sha256_cipher, 'hex') AS sha_cipher
        FROM media_references WHERE id = $1
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
        "availability_state": r.try_get::<String, _>("availability_state")?,
        "deleted_for_all": r.try_get::<bool, _>("deleted_for_all")?,
        "relay_expires_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("relay_expires_at")?,
        "has_server_copy": r.try_get::<Option<Uuid>, _>("object_id")?.is_some(),
        "has_optimized": r.try_get::<Option<Uuid>, _>("optimized_object_id")?.is_some(),
        "has_thumb": r.try_get::<Option<Uuid>, _>("thumb_object_id")?.is_some(),
        "sha256_plain": r.try_get::<Option<String>, _>("sha_plain")?,
        "sha256_cipher": r.try_get::<Option<String>, _>("sha_cipher")?,
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
            "dedup_saved_bytes": dedup_saved_bytes,
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
