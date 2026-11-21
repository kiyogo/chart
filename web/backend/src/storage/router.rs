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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/capabilities", get(handle_capabilities))
        .route("/policy", get(handle_get_policy).put(handle_update_policy))
        .route("/conversations/{peer_id}", get(handle_get_conv_policy).put(handle_update_conv_policy))
        .route("/uploads/init", post(handle_init_upload))
        .route("/uploads/{id}", get(handle_get_upload_status))
        .route(
            "/uploads/{id}/chunks/{index}",
            put(handle_upload_chunk).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        .route("/uploads/{id}/complete", post(handle_complete_upload))
        .route("/forward", post(handle_forward_reference))
        .route("/references/{id}/meta", get(handle_get_ref_meta))
        .route("/references/{id}/presign", post(handle_presign_ref))
        .route("/references/{id}", delete(handle_delete_reference))
        .route("/relay/{id}/ack", post(handle_relay_ack))
        .route("/locations/report", put(handle_report_locations))
        .route("/metrics", get(handle_storage_metrics))
        .route("/cleanup", post(handle_cleanup_relays))
        .route("/migration/run", post(handle_run_migration))
}

#[derive(Deserialize)]
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
    let stored_sha_arr = sha256_bytes(&bytes);
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
            .unwrap_or("application/octet-stream")
            .to_string()
    } else if sniffed.mime != "application/octet-stream" {
        sniffed.mime.to_string()
    } else if !content_type_header.is_empty() {
        content_type_header.to_string()
    } else {
        "application/octet-stream".into()
    };
    let actual_ext = if is_e2ee { "enc" } else { sniffed.ext };

    let policy_row = sqlx::query(
        "SELECT dm_relay_ttl_hours, dm_encrypted_backup FROM device_storage_policies WHERE user_id = $1",
    )
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;
    let relay_ttl_hours: i32 = policy_row.as_ref().and_then(|r| r.try_get("dm_relay_ttl_hours").ok()).unwrap_or(168);
    let default_dm_backup: bool = policy_row.as_ref().and_then(|r| r.try_get("dm_encrypted_backup").ok()).unwrap_or(false);

    // Determine storage tier strictly from conversation classification:
    // Group & Server channels ALWAYS durable server storage; DMs use Temp Relay (or Encrypted Backup if opted in).
    let wants_backup = meta.backup_dm.unwrap_or(default_dm_backup);
    let (storage_tier, availability_state, relay_expires_at, backend_kind) = match scope_type {
        "group" | "channel" | "avatar" | "banner" | "library" | "sticker" => (
            "server_durable",
            "server_durable",
            None,
            "local_cas",
        ),
        "dm" if wants_backup => (
            "encrypted_backup",
            "backed_up_encrypted",
            None,
            "encrypted_backup",
        ),
        _ => (
            "temp_relay",
            "pending_delivery",
            Some(chrono::Utc::now() + chrono::Duration::hours(relay_ttl_hours as i64)),
            "temp_relay",
        ),
    };

    // Privacy-safe CAS scope:
    // - Server/Group/Channel public-within-scope content can deduplicate globally within CAS scope (`shared:{sha}`)
    // - DM/E2EE content isolates CAS keys per pair/owner (`e2ee:{owner}:{scope}:{sha}`) so zero cross-user existence leakage!
    let sha_hex = hex::encode(stored_sha_arr);
    let cas_scope = if matches!(scope_type, "group" | "channel" | "library" | "sticker") && !is_e2ee {
        format!("shared:{sha_hex}")
    } else if storage_tier == "temp_relay" {
        format!("relay:{}:{}", owner_id, Uuid::new_v4())
    } else if storage_tier == "encrypted_backup" {
        format!("backup:{}:{}", owner_id, sha_hex)
    } else {
        format!("priv:{}:{}:{}", owner_id, scope_id.unwrap_or(owner_id), sha_hex)
    };

    let (dims_w, dims_h) = if !is_e2ee && matches!(sniffed.kind, MediaKind::Image) {
        probe_image_dimensions(&bytes).map(|(w, h)| (Some(w as i32), Some(h as i32))).unwrap_or((None, None))
    } else {
        (None, None)
    };

    let storage_key = match backend_kind {
        "temp_relay" => format!("{}.enc", Uuid::new_v4()),
        "encrypted_backup" => format!("{}/{}.enc", owner_id, sha_hex),
        _ => LocalFileStorageAdapter::cas_key_for_sha(&stored_sha_arr, actual_ext),
    };

    let mut tx = pool.begin().await?;
    let existing_obj = sqlx::query(
        "SELECT id, storage_key FROM media_objects WHERE cas_scope = $1 AND variant_kind = 'original'",
    )
    .bind(&cas_scope)
    .fetch_optional(&mut *tx)
    .await?;

    let (original_obj_id, deduplicated) = if let Some(existing_row) = existing_obj {
        let oid: Uuid = existing_row.try_get("id")?;
        let ex_key: String = existing_row.try_get("storage_key")?;
        if backend_kind == "local_cas" && !storage.local_cas.object_exists(&ex_key).await.unwrap_or(false) {
            storage.local_cas.put_object(&ex_key, bytes.clone(), &actual_mime, None).await
                .map_err(|_| AppError::Internal)?;
        }
        sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
            .bind(oid)
            .execute(&mut *tx)
            .await?;
        (oid, true)
    } else {
        match backend_kind {
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
        sqlx::query(
            r#"
            INSERT INTO media_objects
                (id, cas_scope, sha256, plain_sha256, storage_backend, storage_key, mime_type, file_ext,
                 byte_size, original_byte_size, width, height, is_animated, ref_count, variant_kind, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, $10, $11, $12, 1, 'original', $13)
            "#,
        )
        .bind(oid)
        .bind(&cas_scope)
        .bind(&stored_sha_arr[..])
        .bind(plain_sha.as_ref().map(|p| &p[..]))
        .bind(backend_kind)
        .bind(&storage_key)
        .bind(&actual_mime)
        .bind(actual_ext)
        .bind(bytes.len() as i64)
        .bind(dims_w)
        .bind(dims_h)
        .bind(matches!(sniffed.kind, MediaKind::AnimatedImage))
        .bind(relay_expires_at)
        .execute(&mut *tx)
        .await?;
        (oid, false)
    };

    // For non-E2EE images in durable scopes (group/channel/library), also produce adaptive optimized + thumbnail variants.
    let mut opt_obj_id: Option<Uuid> = None;
    let mut thumb_obj_id: Option<Uuid> = None;
    if !is_e2ee && !preserve_original && backend_kind == "local_cas" && matches!(sniffed.kind, MediaKind::Image) {
        if let Ok(opt) = optimize_chat_image(&bytes) {
            if opt.bytes.len() < bytes.len() {
                let opt_sha = sha256_bytes(&opt.bytes);
                let opt_key = LocalFileStorageAdapter::cas_key_for_sha(&opt_sha, opt.ext);
                let opt_scope = format!("{cas_scope}:opt");
                if storage.local_cas.put_object(&opt_key, opt.bytes.clone(), opt.mime, None).await.is_ok() {
                    let oid = sqlx::query_scalar::<_, Uuid>(
                        r#"
                        INSERT INTO media_objects
                            (id, cas_scope, sha256, storage_backend, storage_key, mime_type, file_ext,
                             byte_size, original_byte_size, width, height, is_animated, ref_count, variant_of, variant_kind)
                        VALUES ($1, $2, $3, 'local_cas', $4, $5, $6, $7, $8, $9, $10, FALSE, 1, $11, 'optimized')
                        ON CONFLICT (cas_scope, variant_kind) DO UPDATE SET ref_count = media_objects.ref_count + 1
                        RETURNING id
                        "#,
                    )
                    .bind(Uuid::new_v4())
                    .bind(&opt_scope)
                    .bind(&opt_sha[..])
                    .bind(&opt_key)
                    .bind(opt.mime)
                    .bind(opt.ext)
                    .bind(opt.bytes.len() as i64)
                    .bind(bytes.len() as i64)
                    .bind(opt.width as i32)
                    .bind(opt.height as i32)
                    .bind(original_obj_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    opt_obj_id = Some(oid);
                }
            }
            if let Some(tb) = opt.thumb_bytes {
                let tb_sha = sha256_bytes(&tb);
                let tb_key = LocalFileStorageAdapter::cas_key_for_sha(&tb_sha, opt.thumb_ext);
                let tb_scope = format!("{cas_scope}:thumb");
                if storage.local_cas.put_object(&tb_key, tb.clone(), opt.thumb_mime, None).await.is_ok() {
                    let tid = sqlx::query_scalar::<_, Uuid>(
                        r#"
                        INSERT INTO media_objects
                            (id, cas_scope, sha256, storage_backend, storage_key, mime_type, file_ext,
                             byte_size, original_byte_size, width, height, is_animated, ref_count, variant_of, variant_kind)
                        VALUES ($1, $2, $3, 'local_cas', $4, $5, $6, $7, $8, 320, 320, FALSE, 1, $9, 'thumb')
                        ON CONFLICT (cas_scope, variant_kind) DO UPDATE SET ref_count = media_objects.ref_count + 1
                        RETURNING id
                        "#,
                    )
                    .bind(Uuid::new_v4())
                    .bind(&tb_scope)
                    .bind(&tb_sha[..])
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
        "uploader_id": owner_id,
        "file_name": declared_name,
        "file_size": bytes.len() as i64,
        "mime_type": actual_mime,
        "sha256": sha_hex,
        "storage_tier": storage_tier,
        "availability_state": availability_state,
        "deduplicated": deduplicated,
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
               o.storage_backend AS orig_backend, o.storage_key AS orig_key, o.mime_type AS orig_mime, o.sha256 AS orig_sha, o.byte_size AS orig_size,
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
    let (backend, key, mime, sha, total_size) = if wanted == "thumb" && row.try_get::<Option<String>, _>("th_key")?.is_some() {
        (
            row.try_get::<Option<String>, _>("th_backend")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("th_key")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("th_mime")?.unwrap_or_else(|| "image/webp".into()),
            row.try_get::<Option<Vec<u8>>, _>("th_sha")?.unwrap_or_default(),
            row.try_get::<Option<i64>, _>("th_size")?.unwrap_or(0) as u64,
        )
    } else if wanted == "optimized" && row.try_get::<Option<String>, _>("opt_key")?.is_some() {
        (
            row.try_get::<Option<String>, _>("opt_backend")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("opt_key")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("opt_mime")?.unwrap_or_else(|| "image/avif".into()),
            row.try_get::<Option<Vec<u8>>, _>("opt_sha")?.unwrap_or_default(),
            row.try_get::<Option<i64>, _>("opt_size")?.unwrap_or(0) as u64,
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
        )
    };

    let filename: String = row.try_get("original_filename")?;
    let safe_disp = sanitize_filename(&filename);
    let sha_hex = hex::encode(sha);

    if let Some((start, end_opt)) = range_header.and_then(parse_byte_range) {
        let end = end_opt.unwrap_or(total_size.saturating_sub(1)).min(total_size.saturating_sub(1));
        let (chunk, full_len) = match backend.as_str() {
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
        "s3_lifecycle_preview": state.storage.s3.lifecycle_policy_descriptor(7),
        "default_cache_limit_mb": 1024,
    })))
}

#[derive(Serialize, Deserialize)]
struct PolicyPayload {
    pub cache_limit_mb: i32,
    pub auto_download_mode: String,
    pub dm_relay_ttl_hours: i32,
    pub dm_encrypted_backup: bool,
    pub require_device_approval: bool,
    pub prefer_original_upload: bool,
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
        "SELECT cache_limit_mb, auto_download_mode, dm_relay_ttl_hours, dm_encrypted_backup, require_device_approval, prefer_original_upload FROM device_storage_policies WHERE user_id = $1",
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
    if !(1..=720).contains(&p.dm_relay_ttl_hours) || p.cache_limit_mb < 0 {
        return Err(AppError::BadRequest("invalid_policy_bounds"));
    }
    sqlx::query(
        r#"
        INSERT INTO device_storage_policies
            (user_id, cache_limit_mb, auto_download_mode, dm_relay_ttl_hours, dm_encrypted_backup, require_device_approval, prefer_original_upload, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, now())
        ON CONFLICT (user_id) DO UPDATE SET
            cache_limit_mb = EXCLUDED.cache_limit_mb,
            auto_download_mode = EXCLUDED.auto_download_mode,
            dm_relay_ttl_hours = EXCLUDED.dm_relay_ttl_hours,
            dm_encrypted_backup = EXCLUDED.dm_encrypted_backup,
            require_device_approval = EXCLUDED.require_device_approval,
            prefer_original_upload = EXCLUDED.prefer_original_upload,
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
    if req.total_size <= 0 || req.total_size > 200 * 1024 * 1024 {
        return Err(AppError::BadRequest("invalid_total_size"));
    }

    let chunk_size = req.chunk_size.unwrap_or(1024 * 1024).clamp(4096, 8 * 1024 * 1024);
    let total_chunks = ((req.total_size + chunk_size as i64 - 1) / chunk_size as i64) as i32;
    let transfer_id = Uuid::new_v4();
    let expected_sha = decode_hex_32(req.sha256_cipher.as_deref());
    let plain_sha = decode_hex_32(req.sha256_plain.as_deref());

    // Instant deduplication fast-path check for non-E2EE group/channel uploads.
    let is_e2ee = req.is_e2ee.unwrap_or(false);
    if !is_e2ee && matches!(req.scope_type.as_str(), "group" | "channel") {
        if let Some(ref hex_sha) = req.sha256_cipher {
            let cas_scope = format!("shared:{}", hex_sha.to_lowercase());
            if let Some(ex_row) = sqlx::query(
                "SELECT id, storage_key FROM media_objects WHERE cas_scope = $1 AND variant_kind = 'original'",
            )
            .bind(&cas_scope)
            .fetch_optional(&state.db)
            .await?
            {
                let obj_id: Uuid = ex_row.try_get("id")?;
                let ex_key: String = ex_row.try_get("storage_key")?;
                if state.storage.local_cas.object_exists(&ex_key).await.unwrap_or(false) {
                    let ref_id = Uuid::new_v4();
                let mut tx = state.db.begin().await?;
                sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                    .bind(obj_id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    r#"
                    INSERT INTO media_references
                        (id, owner_id, scope_type, scope_id, object_id, original_filename, mime_type,
                         byte_size, sha256_plain, sha256_cipher, storage_tier, availability_state)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'server_durable', 'server_durable')
                    "#,
                )
                .bind(ref_id)
                .bind(user.id)
                .bind(&req.scope_type)
                .bind(req.scope_id)
                .bind(obj_id)
                .bind(sanitize_filename(&req.file_name))
                .bind(&req.mime_type)
                .bind(req.total_size)
                .bind(plain_sha.as_ref().map(|x| &x[..]))
                .bind(expected_sha.as_ref().map(|x| &x[..]))
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(Json(serde_json::json!({
                    "dedup_instant": true,
                    "transfer_id": transfer_id,
                    "ref_id": ref_id,
                    "download_url": format!("/api/attachments/{ref_id}"),
                })));
                }
            }
        }
    }

    let init = state
        .storage
        .local_cas
        .begin_multipart_transfer(&format!("transfer/{transfer_id}"), &req.mime_type)
        .await
        .map_err(|_| AppError::Internal)?;

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
    .bind(&init.upload_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "dedup_instant": false,
        "transfer_id": transfer_id,
        "multipart_id": init.upload_id,
        "chunk_size": chunk_size,
        "total_chunks": total_chunks,
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

    state
        .storage
        .local_cas
        .upload_chunk(&multipart_id, chunk_index as u32, body.to_vec())
        .await
        .map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        INSERT INTO transfer_chunks (transfer_id, chunk_index, byte_size, chunk_sha256)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (transfer_id, chunk_index) DO UPDATE SET byte_size = EXCLUDED.byte_size, chunk_sha256 = EXCLUDED.chunk_sha256, uploaded_at = now()
        "#,
    )
    .bind(transfer_id)
    .bind(chunk_index)
    .bind(body.len() as i32)
    .bind(&chunk_sha[..])
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "chunk_index": chunk_index,
        "chunk_sha256": hex::encode(chunk_sha),
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
    let assembled = state
        .storage
        .local_cas
        .assemble_chunks(&multipart_id, total_chunks as u32)
        .await
        .map_err(|_| AppError::Internal)?;

    let expected_size: i64 = row.try_get("total_size")?;
    if assembled.len() as i64 != expected_size {
        return Err(AppError::BadRequest("size_mismatch"));
    }
    let expected_sha: Option<Vec<u8>> = row.try_get("expected_sha256")?;
    if let Some(exp) = expected_sha.filter(|v| v.len() == 32) {
        if sha256_bytes(&assembled).as_slice() != exp.as_slice() {
            return Err(AppError::BadRequest("integrity_mismatch"));
        }
    }

    let scope_type: String = row.try_get("scope_type")?;
    let scope_id: Option<Uuid> = row.try_get("scope_id")?;
    let original_filename: String = row.try_get("original_filename")?;
    let mime_type: String = row.try_get("mime_type")?;
    let plain_sha: Option<Vec<u8>> = row.try_get("plain_sha256")?;
    let is_e2ee: bool = row.try_get("is_e2ee")?;
    let preserve_original: bool = row.try_get("preserve_original")?;
    let b = body.map(|j| j.0).unwrap_or_default();

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
            sqlx::query("UPDATE media_objects SET ref_count = ref_count + 1 WHERE id = $1")
                .bind(oid)
                .execute(&mut *tx)
                .await?;
        }
    }

    let target_tier = if matches!(req.target_scope_type.as_str(), "group" | "channel") {
        "server_durable"
    } else if obj_id.is_some() {
        "temp_relay"
    } else {
        "dm_client_preferred"
    };
    let target_avail = if obj_id.is_some() {
        if target_tier == "server_durable" { "server_durable" } else { "pending_delivery" }
    } else {
        "replicated_multi_device"
    };

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
    .bind(src.try_get::<String, _>("original_filename")?)
    .bind(src.try_get::<String, _>("mime_type")?)
    .bind(src.try_get::<i64, _>("byte_size")?)
    .bind(src.try_get::<Option<Vec<u8>>, _>("sha256_plain")?)
    .bind(src.try_get::<Option<Vec<u8>>, _>("sha256_cipher")?)
    .bind(req.from_ref_id)
    .bind(target_tier)
    .bind(target_avail)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(serde_json::json!({
        "ref_id": new_ref_id,
        "reused_object": true,
        "download_url": format!("/api/attachments/{new_ref_id}"),
    })))
}

pub use crate::storage::references::{
    BatchLocationsReq, DeleteRefQuery, LocationItem, RelayAckBody, RunMigrationReq,
    handle_cleanup_relays, handle_delete_reference, handle_get_ref_meta, handle_presign_ref,
    handle_relay_ack, handle_report_locations, handle_run_migration, handle_storage_metrics,
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
