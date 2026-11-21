//! Plugin Outbound Network Proxy, Account-Partitioned Encrypted Storage, Multi-Device Sync & Plugin Bots.
//! Extracted from `installations.rs` as a focused domain submodule (Spec 051).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    plugins::{
        installations::{
            PluginNetFetchReq, RegisterPluginBotReq, StoragePutReq, SyncQuery,
            broadcast_plugin_sync, fetch_installation_full, has_active_grant,
            is_server_side_e2ee_tainted,
        },
        security::{
            PLUGIN_RATE_LIMITER, build_pinned_client, decrypt_secret, encrypt_secret,
            require_scope_manage_access, require_scope_read_access, validate_outbound_url,
        },
    },
    privacy::auth_user,
};

// ============================================================================
// Account-Partitioned Plugin Storage & Encrypted Secrets (AES-256-GCM at rest)
// ============================================================================

pub async fn list_plugin_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let inst = sqlx::query("SELECT scope_type, scope_id FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("installation_not_found"))?;
    require_scope_read_access(&state, uid, &inst.get::<String, _>("scope_type"), inst.get("scope_id")).await?;

    let rows = sqlx::query(
        "SELECT storage_key, value, is_secret, size_bytes, syncable, updated_at
         FROM plugin_storage
         WHERE installation_id = $1 AND owner_user_id = $2
         ORDER BY storage_key",
    )
    .bind(inst_id)
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let mut total_bytes = 0i64;
    let mut items = serde_json::Map::new();
    let mut meta = Vec::new();
    for r in rows {
        let k: String = r.get("storage_key");
        let is_secret: bool = r.get("is_secret");
        let sz: i32 = r.get("size_bytes");
        total_bytes += sz as i64;
        if !is_secret {
            items.insert(k.clone(), r.get("value"));
        }
        meta.push(json!({
            "key": k,
            "is_secret": is_secret,
            "size_bytes": sz,
            "syncable": r.get::<bool, _>("syncable"),
            "updated_at": r.get::<DateTime<Utc>, _>("updated_at"),
        }));
    }
    Ok(Json(json!({
        "installation_id": inst_id,
        "total_bytes": total_bytes,
        "data": items,
        "keys": meta,
    })))
}

pub async fn put_plugin_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((inst_id, key)): Path<(Uuid, String)>,
    Json(req): Json<StoragePutReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let k = key.trim();
    if k.is_empty() || k.len() > 120 || k.contains("..") {
        return Err(AppError::BadRequest("invalid_storage_key"));
    }
    let inst = sqlx::query(
        "SELECT i.scope_type, i.scope_id, i.enabled, v.manifest
         FROM plugin_installations i JOIN plugin_versions v ON v.id = i.version_id
         WHERE i.id = $1",
    )
    .bind(inst_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("installation_not_found"))?;
    let st: String = inst.get("scope_type");
    let sid: Uuid = inst.get("scope_id");
    require_scope_read_access(&state, uid, &st, sid).await?;

    let raw_bytes = serde_json::to_vec(&req.value).map_err(|_| AppError::BadRequest("invalid_value"))?;
    if raw_bytes.len() > 128 * 1024 {
        return Err(AppError::PayloadTooLarge);
    }

    let max_kb = inst
        .get::<Value, _>("manifest")
        .pointer("/resource_limits/max_storage_kb")
        .and_then(|v| v.as_i64())
        .unwrap_or(512)
        .clamp(64, 10_240);
    let current_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(size_bytes), 0)::bigint FROM plugin_storage WHERE installation_id = $1 AND owner_user_id = $2 AND storage_key <> $3",
    )
    .bind(inst_id)
    .bind(uid)
    .bind(k)
    .fetch_one(&state.db)
    .await?;

    if current_bytes + raw_bytes.len() as i64 > max_kb * 1024 {
        return Err(AppError::PayloadTooLarge);
    }

    let (stored_val, ciphertext, syncable) = if req.is_secret {
        let ct = encrypt_secret(&state.config.session_secret, &raw_bytes)?;
        // Secrets are encrypted at rest and never exposed in plaintext sync payloads
        (json!(null), Some(ct), false)
    } else {
        (req.value, None, req.syncable)
    };

    sqlx::query(
        "INSERT INTO plugin_storage (installation_id, owner_user_id, storage_key, value, is_secret, ciphertext, size_bytes, syncable, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
         ON CONFLICT (installation_id, owner_user_id, storage_key)
         DO UPDATE SET value = EXCLUDED.value, is_secret = EXCLUDED.is_secret, ciphertext = EXCLUDED.ciphertext,
                       size_bytes = EXCLUDED.size_bytes, syncable = EXCLUDED.syncable, updated_at = now()",
    )
    .bind(inst_id)
    .bind(uid)
    .bind(k)
    .bind(&stored_val)
    .bind(req.is_secret)
    .bind(ciphertext.as_deref())
    .bind(raw_bytes.len() as i32)
    .bind(syncable)
    .execute(&state.db)
    .await?;

    if syncable {
        broadcast_plugin_sync(
            &state,
            "user",
            uid,
            uid,
            "storage_updated",
            inst_id,
            json!({ "key": k }),
        )
        .await;
    }

    Ok(Json(json!({ "key": k, "size_bytes": raw_bytes.len(), "is_secret": req.is_secret })))
}

pub async fn delete_plugin_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((inst_id, key)): Path<(Uuid, String)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    sqlx::query("DELETE FROM plugin_storage WHERE installation_id = $1 AND owner_user_id = $2 AND storage_key = $3")
        .bind(inst_id)
        .bind(uid)
        .bind(key.trim())
        .execute(&state.db)
        .await?;
    broadcast_plugin_sync(&state, "user", uid, uid, "storage_deleted", inst_id, json!({ "key": key.trim() })).await;
    Ok(Json(json!({ "deleted": true })))
}

// ============================================================================
// Multi-Device Sync Snapshot & Incremental State
// ============================================================================

pub async fn sync_plugins_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SyncQuery>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let sync_seq: i64 = sqlx::query_scalar("SELECT sync_seq FROM plugin_sync_cursors WHERE user_id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(0);

    let since = q.since_seq.unwrap_or(0);
    if since > 0 && since >= sync_seq {
        return Ok(Json(json!({
            "sync_seq": sync_seq,
            "unchanged": true,
            "installations": [],
            "storage": {},
        })));
    }

    let inst_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT id FROM plugin_installations
           WHERE (scope_type IN ('user', 'conversation') AND scope_id = $1)
              OR (scope_type = 'device' AND scope_id = $1 AND ($2::text IS NULL OR device_id = $2))
              OR (scope_type = 'server' AND scope_id IN (SELECT server_id FROM server_members WHERE user_id = $1))
              OR (scope_type = 'group' AND scope_id IN (SELECT group_id FROM group_members WHERE user_id = $1))
           ORDER BY display_order, created_at"#,
    )
    .bind(uid)
    .bind(q.device_id.as_deref())
    .fetch_all(&state.db)
    .await?;

    let mut installations = Vec::with_capacity(inst_ids.len());
    let mut storage_map = serde_json::Map::new();
    for id in inst_ids {
        if let Ok(full) = fetch_installation_full(&state.db, id, uid, q.device_id.as_deref()).await {
            let srows = sqlx::query(
                "SELECT storage_key, value FROM plugin_storage
                 WHERE installation_id = $1 AND owner_user_id = $2 AND syncable = true AND is_secret = false",
            )
            .bind(id)
            .bind(uid)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
            let mut kv = serde_json::Map::new();
            for sr in srows {
                kv.insert(sr.get("storage_key"), sr.get("value"));
            }
            storage_map.insert(id.to_string(), Value::Object(kv));
            installations.push(full);
        }
    }

    Ok(Json(json!({
        "sync_seq": sync_seq,
        "unchanged": false,
        "installations": installations,
        "storage": storage_map,
    })))
}

// ============================================================================
// SSRF-Protected, Rate-Limited, Permission-Checked Plugin Outbound Proxy
// ============================================================================

pub async fn plugin_network_fetch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
    Json(req): Json<PluginNetFetchReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let inst = sqlx::query(
        "SELECT i.scope_type, i.scope_id, i.package_id, i.enabled, v.manifest
         FROM plugin_installations i JOIN plugin_versions v ON v.id = i.version_id
         WHERE i.id = $1",
    )
    .bind(inst_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("installation_not_found"))?;

    let st: String = inst.get("scope_type");
    let sid: Uuid = inst.get("scope_id");
    let pkg_id: Uuid = inst.get("package_id");
    let enabled: bool = inst.get("enabled");
    require_scope_read_access(&state, uid, &st, sid).await?;
    if !enabled {
        return Err(AppError::Forbidden("plugin_disabled"));
    }

    // Must have network.outbound grant
    if !has_active_grant(&state.db, inst_id, uid, "network.outbound", None).await? {
        return Err(AppError::Forbidden("network_permission_denied"));
    }
    // Enforce E2EE egress policy using BOTH client taint flag AND authoritative server-side E2EE grant/audit state!
    let server_e2ee_tainted = is_server_side_e2ee_tainted(&state.db, inst_id, uid).await?;
    let effective_e2ee_tainted = req.e2ee_tainted || server_e2ee_tainted;
    if effective_e2ee_tainted
        && !has_active_grant(&state.db, inst_id, uid, "e2ee.egress_external", None).await?
        && !has_active_grant(&state.db, inst_id, uid, "e2ee.store_external", None).await?
    {
        return Err(AppError::Forbidden("e2ee_egress_denied"));
    }

    let manifest: Value = inst.get("manifest");
    let max_rpm = manifest
        .pointer("/resource_limits/max_requests_per_min")
        .and_then(|v| v.as_u64())
        .unwrap_or(30)
        .clamp(1, 120) as usize;
    if !PLUGIN_RATE_LIMITER.check_and_record(&format!("pnet:{inst_id}:{uid}"), max_rpm, Duration::from_secs(60)) {
        return Err(AppError::TooManyRequests);
    }

    if let Some(ref b) = req.body {
        if b.len() > 128 * 1024 {
            return Err(AppError::PayloadTooLarge);
        }
    }

    let target = validate_outbound_url(&req.url).await?;
    // Enforce allowed_domains if declared in manifest
    if let Some(arr) = manifest.get("allowed_domains").and_then(|v| v.as_array()) {
        if !arr.is_empty() {
            let allowed = arr.iter().filter_map(|v| v.as_str()).any(|d| {
                target.host.eq_ignore_ascii_case(d)
                    || target.host.to_ascii_lowercase().ends_with(&format!(".{}", d.to_ascii_lowercase()))
            });
            if !allowed {
                return Err(AppError::Forbidden("domain_not_allowlisted"));
            }
        }
    }

    let method = match req.method.to_ascii_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        _ => return Err(AppError::BadRequest("unsupported_http_method")),
    };

    let client = build_pinned_client(&target, 8)?;
    let mut builder = client.request(method, &target.url);
    for (k, v) in &req.headers {
        let kl = k.to_ascii_lowercase();
        if matches!(kl.as_str(), "host" | "cookie" | "connection" | "content-length") {
            continue;
        }
        builder = builder.header(k.as_str(), v.as_str());
    }

    // Server-side encrypted secret injection (API Key / OAuth token never exposed to sandbox JS)
    if let Some(ref skey) = req.inject_secret_key {
        let ct: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT ciphertext FROM plugin_storage WHERE installation_id = $1 AND owner_user_id = $2 AND storage_key = $3 AND is_secret = true",
        )
        .bind(inst_id)
        .bind(uid)
        .bind(skey)
        .fetch_optional(&state.db)
        .await?
        .flatten();
        if let Some(blob) = ct {
            let pt = decrypt_secret(&state.config.session_secret, &blob)?;
            let sec_str: String = serde_json::from_slice(&pt).unwrap_or_else(|_| String::from_utf8_lossy(&pt).into_owned());
            let hdr_name = req.inject_secret_header.as_deref().unwrap_or("Authorization");
            let prefix = req.inject_secret_prefix.as_deref().unwrap_or("Bearer ");
            builder = builder.header(hdr_name, format!("{prefix}{sec_str}"));
        }
    }

    if let Some(body) = req.body {
        builder = builder.body(body);
    }

    let mut resp = builder.send().await.map_err(|_| AppError::Unavailable)?;
    let status = resp.status().as_u16();
    const MAX_PROXY_RESP_BYTES: usize = 256 * 1024;
    if let Some(cl) = resp.content_length() {
        if cl > MAX_PROXY_RESP_BYTES as u64 {
            return Err(AppError::PayloadTooLarge);
        }
    }
    let mut bytes = Vec::with_capacity(16 * 1024);
    while let Some(chunk) = resp.chunk().await.map_err(|_| AppError::Unavailable)? {
        if bytes.len() + chunk.len() > MAX_PROXY_RESP_BYTES {
            return Err(AppError::PayloadTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    let body_text = String::from_utf8_lossy(&bytes).into_owned();

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, permission, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'network_fetch', 'network.outbound', $7)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(pkg_id)
    .bind(uid)
    .bind(&st)
    .bind(sid)
    .bind(json!({ "host": target.host, "status": status, "e2ee_tainted": effective_e2ee_tainted }))
    .execute(&state.db)
    .await;

    Ok(Json(json!({ "status": status, "body": body_text })))
}

// ============================================================================
// Plugin-Registered Bots (Distinct Bot Identity linked to Installation)
// ============================================================================

pub async fn register_plugin_bot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
    Json(req): Json<RegisterPluginBotReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let inst = sqlx::query("SELECT scope_type, scope_id, package_id, enabled FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("installation_not_found"))?;
    let st: String = inst.get("scope_type");
    let sid: Uuid = inst.get("scope_id");
    let pkg_id: Uuid = inst.get("package_id");
    let enabled: bool = inst.get("enabled");
    require_scope_manage_access(&state, uid, &st, sid).await?;
    if !enabled {
        return Err(AppError::Forbidden("plugin_disabled"));
    }
    if !has_active_grant(&state.db, inst_id, uid, "bots.manage", None).await? {
        return Err(AppError::Forbidden("bots_permission_denied"));
    }

    let uname = req.username.trim().to_ascii_lowercase();
    if !(2..=32).contains(&uname.len()) || !uname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest("invalid_bot_username"));
    }
    let display = req.display_name.trim();
    if display.len() < 2 || display.len() > 64 {
        return Err(AppError::BadRequest("invalid_bot_display_name"));
    }

    let dup: Option<Uuid> = sqlx::query_scalar("SELECT id FROM bots WHERE username_norm = $1")
        .bind(&uname)
        .fetch_optional(&state.db)
        .await?;
    if dup.is_some() {
        return Err(AppError::Conflict("bot_username_taken"));
    }

    let bot_id = Uuid::new_v4();
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|_| AppError::Internal)?;
    let raw_token = format!("{}:{}", &bot_id.simple().to_string()[..10], URL_SAFE_NO_PAD.encode(buf));
    let token_hash = Sha256::digest(raw_token.as_bytes()).to_vec();
    let token_hint = format!("{}***", &raw_token[..12]);

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO bots (id, owner_id, username, username_norm, display_name, description, avatar_url, token_hash, token_hint, is_public, verification_status)
         VALUES ($1, $2, $3, $3, $4, $5, $6, $7, $8, false, 'none')",
    )
    .bind(bot_id)
    .bind(uid)
    .bind(&uname)
    .bind(display)
    .bind(req.description.as_deref())
    .bind(req.avatar_url.as_deref())
    .bind(&token_hash)
    .bind(&token_hint)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO plugin_bots (id, installation_id, bot_id) VALUES ($1, $2, $3)")
        .bind(Uuid::new_v4())
        .bind(inst_id)
        .bind(bot_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, permission, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'bot_register', 'bots.manage', $7)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(pkg_id)
    .bind(uid)
    .bind(&st)
    .bind(sid)
    .bind(json!({ "bot_id": bot_id, "username": uname }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "bot_id": bot_id,
        "installation_id": inst_id,
        "username": uname,
        "display_name": display,
        "identity_type": "bot",
        "token_hint": token_hint,
        "token": raw_token,
    })))
}

pub async fn list_plugin_bots(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
) -> AppResult<Json<Vec<Value>>> {
    let uid = auth_user(&state, &headers).await?;
    let inst = sqlx::query("SELECT scope_type, scope_id FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("installation_not_found"))?;
    require_scope_read_access(&state, uid, &inst.get::<String, _>("scope_type"), inst.get("scope_id")).await?;

    let rows = sqlx::query(
        "SELECT b.id, b.username, b.display_name, b.description, b.avatar_url, b.token_hint, b.created_at
         FROM plugin_bots pb JOIN bots b ON b.id = pb.bot_id
         WHERE pb.installation_id = $1 ORDER BY b.created_at",
    )
    .bind(inst_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| {
                json!({
                    "bot_id": r.get::<Uuid, _>("id"),
                    "username": r.get::<String, _>("username"),
                    "display_name": r.get::<String, _>("display_name"),
                    "description": r.get::<Option<String>, _>("description"),
                    "avatar_url": r.get::<Option<String>, _>("avatar_url"),
                    "token_hint": r.get::<String, _>("token_hint"),
                    "identity_type": "bot",
                    "created_at": r.get::<DateTime<Utc>, _>("created_at"),
                })
            })
            .collect(),
    ))
}
