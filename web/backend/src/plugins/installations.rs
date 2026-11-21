//! Plugin Installations, Consent-Based Permissions, E2EE Governance, Privacy Dashboard,
//! Account-Partitioned Encrypted Storage, Multi-Device Realtime Sync, Plugin Bots & SSRF Proxy.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    plugins::{
        schema::{find_permission, is_personal_permission, is_sensitive_permission, is_valid_permission},
        security::{require_scope_manage_access, require_scope_read_access},
    },
    privacy::auth_user,
    websocket::WsEvent,
};

#[derive(Deserialize)]
pub struct InstallPreviewReq {
    pub package_id: Option<Uuid>,
    pub plugin_key: Option<String>,
    pub version_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct GrantSpec {
    pub permission: String,
    #[serde(default = "default_global_scope")]
    pub scope_type: String, // "global" | "conversation" | "server" | "group" | "channel" | "device"
    #[serde(default)]
    pub scope_id: Option<Uuid>,
    #[serde(default)]
    pub device_only: bool,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_global_scope() -> String {
    "global".into()
}

#[derive(Deserialize)]
pub struct CreateInstallationReq {
    pub package_id: Option<Uuid>,
    pub plugin_key: Option<String>,
    pub version_id: Option<Uuid>,
    #[serde(default = "default_user_scope")]
    pub scope_type: String,
    #[serde(default)]
    pub scope_id: Option<Uuid>,
    #[serde(default)]
    pub device_id: Option<String>,
    /// Must be explicitly confirmed by user after viewing permission consent dialog.
    #[serde(default)]
    pub consent_confirmed: bool,
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub grants: Vec<GrantSpec>,
    #[serde(default)]
    pub allowed_channels: Vec<Uuid>,
    #[serde(default)]
    pub blocked_channels: Vec<Uuid>,
    #[serde(default)]
    pub settings: Option<Value>,
}

fn default_user_scope() -> String {
    "user".into()
}

#[derive(Deserialize)]
pub struct UpdateInstallationReq {
    pub enabled: Option<bool>,
    pub display_order: Option<i32>,
    pub ui_slot: Option<String>,
    pub allowed_channels: Option<Vec<Uuid>>,
    pub blocked_channels: Option<Vec<Uuid>>,
    pub allowed_roles: Option<Vec<Uuid>>,
    pub settings: Option<Value>,
    pub version_id: Option<Uuid>,
    #[serde(default)]
    pub additional_grants: Vec<GrantSpec>,
}

#[derive(Deserialize)]
pub struct ListInstallationsQuery {
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct StoragePutReq {
    pub value: Value,
    #[serde(default)]
    pub is_secret: bool,
    #[serde(default = "default_true")]
    pub syncable: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct SyncQuery {
    pub since_seq: Option<i64>,
    pub device_id: Option<String>,
}

#[derive(Deserialize)]
pub struct AuditRecordReq {
    pub action: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<Uuid>,
    #[serde(default)]
    pub detail: Option<Value>,
}

#[derive(Deserialize)]
pub struct PluginNetFetchReq {
    pub url: String,
    #[serde(default = "default_get")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    /// Optional secret key stored in plugin_storage (is_secret=true) to inject server-side as Authorization/header without exposing secret to plugin sandbox.
    #[serde(default)]
    pub inject_secret_key: Option<String>,
    #[serde(default)]
    pub inject_secret_header: Option<String>,
    #[serde(default)]
    pub inject_secret_prefix: Option<String>,
    /// Set true by sandbox host whenever the plugin session read decrypted E2EE content.
    #[serde(default)]
    pub e2ee_tainted: bool,
}

fn default_get() -> String {
    "GET".into()
}

#[derive(Deserialize)]
pub struct RegisterPluginBotReq {
    pub username: String,
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct InstallationFull {
    pub id: Uuid,
    pub package_id: Uuid,
    pub plugin_key: String,
    pub name: String,
    pub icon_url: String,
    pub publisher_slug: String,
    pub publisher_name: String,
    pub publisher_verified: bool,
    pub version_id: Uuid,
    pub version: String,
    pub runtime: String,
    pub manifest: Value,
    pub bundle_code: String,
    pub ui_html: String,
    pub wasm_base64: String,
    pub sha256: String,
    pub requested_permissions: Value,
    pub approved_permissions: Value,
    pub sensitive_approval_status: String,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub installed_by: Uuid,
    pub enabled: bool,
    pub display_order: i32,
    pub ui_slot: String,
    pub allowed_channels: Value,
    pub blocked_channels: Value,
    pub allowed_roles: Value,
    pub settings: Value,
    pub sync_version: i64,
    pub grants: Vec<Value>,
    pub updated_at: DateTime<Utc>,
}

pub async fn bump_sync_cursor(db: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO plugin_sync_cursors (user_id, sync_seq, updated_at) VALUES ($1, 1, now())
         ON CONFLICT (user_id) DO UPDATE SET sync_seq = plugin_sync_cursors.sync_seq + 1, updated_at = now()
         RETURNING sync_seq",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(1)
}

pub async fn broadcast_plugin_sync(
    state: &AppState,
    scope_type: &str,
    scope_id: Uuid,
    actor: Uuid,
    action: &str,
    installation_id: Uuid,
    extra: Value,
) {
    let recipients: Vec<Uuid> = match scope_type {
        "server" => sqlx::query_scalar("SELECT user_id FROM server_members WHERE server_id = $1")
            .bind(scope_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_else(|_| vec![actor]),
        "group" => sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(scope_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_else(|_| vec![actor]),
        "channel" => {
            if let Ok(Some(sid)) = sqlx::query_scalar::<_, Uuid>("SELECT server_id FROM channels WHERE id = $1")
                .bind(scope_id)
                .fetch_optional(&state.db)
                .await
            {
                sqlx::query_scalar("SELECT user_id FROM server_members WHERE server_id = $1")
                    .bind(sid)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_else(|_| vec![actor])
            } else {
                vec![actor]
            }
        }
        _ => vec![actor],
    };

    let seq = bump_sync_cursor(&state.db, actor).await;
    for uid in &recipients {
        if *uid != actor {
            let _ = bump_sync_cursor(&state.db, *uid).await;
        }
    }
    let payload = json!({
        "action": action,
        "installation_id": installation_id,
        "scope_type": scope_type,
        "scope_id": scope_id,
        "sync_seq": seq,
        "detail": extra,
    });
    state
        .ws_hub
        .send_to_users(&recipients, WsEvent::PluginSync(payload))
        .await;
}

async fn resolve_package_and_version(
    db: &PgPool,
    uid: Uuid,
    package_id: Option<Uuid>,
    plugin_key: Option<&str>,
    version_id: Option<Uuid>,
) -> AppResult<(Uuid, Uuid, sqlx::postgres::PgRow)> {
    let row = if let Some(vid) = version_id {
        sqlx::query(
            "SELECT p.id AS package_id, p.plugin_key, p.name, p.icon_url,
                    pub.slug AS pub_slug, pub.display_name AS pub_name, pub.verification_status AS pub_status, pub.user_id AS pub_uid,
                    v.id AS version_id, v.version, v.runtime, v.manifest, v.sha256, v.size_bytes,
                    v.requested_permissions, v.approved_permissions, v.sensitive_approval_status, v.revoked
             FROM plugin_versions v
             JOIN plugin_packages p ON p.id = v.package_id
             JOIN plugin_publishers pub ON pub.id = p.publisher_id
             WHERE v.id = $1 AND (p.is_public = true OR pub.user_id = $2)",
        )
        .bind(vid)
        .bind(uid)
        .fetch_optional(db)
        .await?
    } else if let Some(pid) = package_id {
        sqlx::query(
            "SELECT p.id AS package_id, p.plugin_key, p.name, p.icon_url,
                    pub.slug AS pub_slug, pub.display_name AS pub_name, pub.verification_status AS pub_status, pub.user_id AS pub_uid,
                    v.id AS version_id, v.version, v.runtime, v.manifest, v.sha256, v.size_bytes,
                    v.requested_permissions, v.approved_permissions, v.sensitive_approval_status, v.revoked
             FROM plugin_packages p
             JOIN plugin_publishers pub ON pub.id = p.publisher_id
             JOIN plugin_versions v ON v.package_id = p.id AND v.version = p.latest_version
             WHERE p.id = $1 AND (p.is_public = true OR pub.user_id = $2)",
        )
        .bind(pid)
        .bind(uid)
        .fetch_optional(db)
        .await?
    } else if let Some(key) = plugin_key {
        sqlx::query(
            "SELECT p.id AS package_id, p.plugin_key, p.name, p.icon_url,
                    pub.slug AS pub_slug, pub.display_name AS pub_name, pub.verification_status AS pub_status, pub.user_id AS pub_uid,
                    v.id AS version_id, v.version, v.runtime, v.manifest, v.sha256, v.size_bytes,
                    v.requested_permissions, v.approved_permissions, v.sensitive_approval_status, v.revoked
             FROM plugin_packages p
             JOIN plugin_publishers pub ON pub.id = p.publisher_id
             JOIN plugin_versions v ON v.package_id = p.id AND v.version = p.latest_version
             WHERE p.plugin_key = $1 AND (p.is_public = true OR pub.user_id = $2)",
        )
        .bind(key)
        .bind(uid)
        .fetch_optional(db)
        .await?
    } else {
        return Err(AppError::BadRequest("package_reference_required"));
    }
    .ok_or(AppError::NotFound("package_or_version_not_found"))?;

    let revoked: bool = row.get("revoked");
    if revoked {
        return Err(AppError::Forbidden("version_revoked"));
    }
    Ok((row.get("package_id"), row.get("version_id"), row))
}

/// Step 1–6 of safe installation: inspects manifest, SHA-256, publisher checkmark & sensitive permissions WITHOUT installing.
pub async fn preview_install(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InstallPreviewReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let (pkg_id, ver_id, row) = resolve_package_and_version(
        &state.db,
        uid,
        req.package_id,
        req.plugin_key.as_deref(),
        req.version_id,
    )
    .await?;

    let req_perms: Vec<String> = serde_json::from_value(row.get("requested_permissions")).unwrap_or_default();
    let app_perms: Vec<String> = serde_json::from_value(row.get("approved_permissions")).unwrap_or_default();
    let pub_status: String = row.get("pub_status");
    let sens_status: String = row.get("sensitive_approval_status");

    let perm_details: Vec<Value> = req_perms
        .iter()
        .map(|pid| {
            let def = find_permission(pid);
            let sensitive = def.map(|d| d.sensitive).unwrap_or(true);
            let approved = !sensitive || (sens_status == "approved" && app_perms.contains(pid));
            json!({
                "id": pid,
                "label": def.map(|d| d.label).unwrap_or(pid.as_str()),
                "description": def.map(|d| d.description).unwrap_or(""),
                "category": def.map(|d| d.category).unwrap_or("other"),
                "sensitive": sensitive,
                "e2ee": def.map(|d| d.e2ee).unwrap_or(false),
                "device_only_default": def.map(|d| d.device_only_default).unwrap_or(false),
                "platform_approved": approved,
            })
        })
        .collect();

    Ok(Json(json!({
        "package_id": pkg_id,
        "version_id": ver_id,
        "plugin_key": row.get::<String, _>("plugin_key"),
        "name": row.get::<String, _>("name"),
        "icon_url": row.get::<String, _>("icon_url"),
        "version": row.get::<String, _>("version"),
        "runtime": row.get::<String, _>("runtime"),
        "sha256": row.get::<String, _>("sha256"),
        "size_bytes": row.get::<i32, _>("size_bytes"),
        "publisher_slug": row.get::<String, _>("pub_slug"),
        "publisher_name": row.get::<String, _>("pub_name"),
        "publisher_verified": pub_status == "verified",
        "sensitive_approval_status": sens_status,
        "permissions": perm_details,
        "requires_explicit_consent": true,
    })))
}

fn validate_grant_request(
    grant: &GrantSpec,
    requested_perms: &[String],
    approved_perms: &[String],
    sens_status: &str,
    pub_status: &str,
    installation_scope_type: &str,
) -> AppResult<()> {
    if !is_valid_permission(&grant.permission) {
        return Err(AppError::BadRequest("unknown_permission"));
    }
    if !requested_perms.contains(&grant.permission) {
        return Err(AppError::BadRequest("permission_not_declared_in_manifest"));
    }
    // Enforce two-tier approval: publisher verification NEVER bypasses sensitive permission approval!
    if is_sensitive_permission(&grant.permission)
        && !(sens_status == "approved" && approved_perms.contains(&grant.permission))
    {
        return Err(AppError::Forbidden("sensitive_permission_not_approved"));
    }
    // E2EE permissions additionally require a verified publisher (unverified plugins can NEVER request E2EE access)
    if grant.permission.starts_with("e2ee.") && pub_status != "verified" {
        return Err(AppError::Forbidden("e2ee_requires_verified_publisher"));
    }
    // Server/group admins CANNOT auto-grant personal E2EE or private contact/profile access of members!
    if matches!(installation_scope_type, "server" | "group" | "channel")
        && (grant.permission.starts_with("e2ee.")
            || matches!(grant.permission.as_str(), "contacts.read" | "contacts.write"))
    {
        return Err(AppError::Forbidden("server_cannot_grant_member_e2ee"));
    }
    if grant.permission.starts_with("e2ee.") {
        if grant.scope_type != "conversation" || grant.scope_id.is_none() {
            return Err(AppError::BadRequest("e2ee_requires_conversation_scope"));
        }
    }
    Ok(())
}

pub async fn create_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateInstallationReq>,
) -> AppResult<Json<InstallationFull>> {
    let uid = auth_user(&state, &headers).await?;
    if !req.consent_confirmed {
        return Err(AppError::Forbidden("explicit_consent_required"));
    }

    let scope_id = match req.scope_type.as_str() {
        "user" | "device" | "conversation" => req.scope_id.unwrap_or(uid),
        "server" | "group" | "channel" => req.scope_id.ok_or(AppError::BadRequest("scope_id_required"))?,
        _ => return Err(AppError::BadRequest("invalid_scope_type")),
    };
    require_scope_manage_access(&state, uid, &req.scope_type, scope_id).await?;

    let (pkg_id, ver_id, row) = resolve_package_and_version(
        &state.db,
        uid,
        req.package_id,
        req.plugin_key.as_deref(),
        req.version_id,
    )
    .await?;

    let req_perms: Vec<String> = serde_json::from_value(row.get("requested_permissions")).unwrap_or_default();
    let app_perms: Vec<String> = serde_json::from_value(row.get("approved_permissions")).unwrap_or_default();
    let sens_status: String = row.get("sensitive_approval_status");
    let pub_status: String = row.get("pub_status");

    for g in &req.grants {
        validate_grant_request(g, &req_perms, &app_perms, &sens_status, &pub_status, &req.scope_type)?;
    }

    let inst_id = Uuid::new_v4();
    let settings = req.settings.unwrap_or_else(|| json!({}));
    let mut tx = state.db.begin().await?;

    // Reject duplicate install of same package in same scope (use PATCH to update)
    let dup: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM plugin_installations WHERE package_id = $1 AND scope_type = $2 AND scope_id = $3",
    )
    .bind(pkg_id)
    .bind(&req.scope_type)
    .bind(scope_id)
    .fetch_optional(&mut *tx)
    .await?;
    if dup.is_some() {
        return Err(AppError::Conflict("plugin_already_installed_in_scope"));
    }

    sqlx::query(
        "INSERT INTO plugin_installations (id, package_id, version_id, scope_type, scope_id, installed_by, device_id, enabled, allowed_channels, blocked_channels, settings)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(inst_id)
    .bind(pkg_id)
    .bind(ver_id)
    .bind(&req.scope_type)
    .bind(scope_id)
    .bind(uid)
    .bind(req.device_id.as_deref())
    .bind(req.enable)
    .bind(serde_json::to_value(&req.allowed_channels).unwrap_or_else(|_| json!([])))
    .bind(serde_json::to_value(&req.blocked_channels).unwrap_or_else(|_| json!([])))
    .bind(&settings)
    .execute(&mut *tx)
    .await?;

    for g in &req.grants {
        let def = find_permission(&g.permission);
        let device_only = g.device_only || def.map(|d| d.device_only_default).unwrap_or(false);
        sqlx::query(
            "INSERT INTO plugin_grants (id, installation_id, user_id, permission, scope_type, scope_id, device_only, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(Uuid::new_v4())
        .bind(inst_id)
        .bind(uid)
        .bind(&g.permission)
        .bind(&g.scope_type)
        .bind(g.scope_id)
        .bind(device_only)
        .bind(g.expires_at)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE plugin_packages SET install_count = install_count + 1 WHERE id = $1")
        .bind(pkg_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'install', $7)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(pkg_id)
    .bind(uid)
    .bind(&req.scope_type)
    .bind(scope_id)
    .bind(json!({ "version_id": ver_id, "enabled": req.enable, "grants_count": req.grants.len() }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    broadcast_plugin_sync(
        &state,
        &req.scope_type,
        scope_id,
        uid,
        "installed",
        inst_id,
        json!({ "enabled": req.enable }),
    )
    .await;

    let full = fetch_installation_full(&state.db, inst_id, uid, req.device_id.as_deref()).await?;
    Ok(Json(full))
}

pub async fn fetch_installation_full(
    db: &PgPool,
    inst_id: Uuid,
    viewer_uid: Uuid,
    viewer_device: Option<&str>,
) -> AppResult<InstallationFull> {
    let r = sqlx::query(
        "SELECT i.*, p.plugin_key, p.name, p.icon_url,
                pub.slug AS pub_slug, pub.display_name AS pub_name, pub.verification_status AS pub_status,
                v.version, v.runtime, v.manifest, v.bundle_code, v.ui_html, v.wasm_base64, v.sha256,
                v.requested_permissions, v.approved_permissions, v.sensitive_approval_status
         FROM plugin_installations i
         JOIN plugin_packages p ON p.id = i.package_id
         JOIN plugin_publishers pub ON pub.id = p.publisher_id
         JOIN plugin_versions v ON v.id = i.version_id
         WHERE i.id = $1",
    )
    .bind(inst_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound("installation_not_found"))?;

    let inst_device: Option<String> = r.get("device_id");
    let same_device = viewer_device.is_none() || inst_device.as_deref() == viewer_device;

    let grant_rows = sqlx::query(
        "SELECT id, user_id, permission, scope_type, scope_id, device_only, expires_at, granted_at
         FROM plugin_grants
         WHERE installation_id = $1 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())
         ORDER BY granted_at",
    )
    .bind(inst_id)
    .fetch_all(db)
    .await?;

    let grants: Vec<Value> = grant_rows
        .into_iter()
        .filter(|g| {
            let dev_only: bool = g.get("device_only");
            let g_uid: Uuid = g.get("user_id");
            let perm: String = g.get("permission");
            // Device-only grants NEVER leak across different devices or users!
            if dev_only && (!same_device || g_uid != viewer_uid) {
                return false;
            }
            // Personal permissions (profile.read, contacts.*, e2ee.*) NEVER carry across users!
            if is_personal_permission(&perm) && g_uid != viewer_uid {
                return false;
            }
            true
        })
        .map(|g| {
            json!({
                "id": g.get::<Uuid, _>("id"),
                "user_id": g.get::<Uuid, _>("user_id"),
                "permission": g.get::<String, _>("permission"),
                "scope_type": g.get::<String, _>("scope_type"),
                "scope_id": g.get::<Option<Uuid>, _>("scope_id"),
                "device_only": g.get::<bool, _>("device_only"),
                "expires_at": g.get::<Option<DateTime<Utc>>, _>("expires_at"),
                "granted_at": g.get::<DateTime<Utc>, _>("granted_at"),
            })
        })
        .collect();

    let pub_status: String = r.get("pub_status");
    Ok(InstallationFull {
        id: r.get("id"),
        package_id: r.get("package_id"),
        plugin_key: r.get("plugin_key"),
        name: r.get("name"),
        icon_url: r.get("icon_url"),
        publisher_slug: r.get("pub_slug"),
        publisher_name: r.get("pub_name"),
        publisher_verified: pub_status == "verified",
        version_id: r.get("version_id"),
        version: r.get("version"),
        runtime: r.get("runtime"),
        manifest: r.get("manifest"),
        bundle_code: r.get("bundle_code"),
        ui_html: r.get("ui_html"),
        wasm_base64: r.get("wasm_base64"),
        sha256: r.get("sha256"),
        requested_permissions: r.get("requested_permissions"),
        approved_permissions: r.get("approved_permissions"),
        sensitive_approval_status: r.get("sensitive_approval_status"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        installed_by: r.get("installed_by"),
        enabled: r.get("enabled"),
        display_order: r.get("display_order"),
        ui_slot: r.get("ui_slot"),
        allowed_channels: r.get("allowed_channels"),
        blocked_channels: r.get("blocked_channels"),
        allowed_roles: r.get("allowed_roles"),
        settings: r.get("settings"),
        sync_version: r.get("sync_version"),
        grants,
        updated_at: r.get("updated_at"),
    })
}

pub async fn list_installations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListInstallationsQuery>,
) -> AppResult<Json<Vec<InstallationFull>>> {
    let uid = auth_user(&state, &headers).await?;
    let ids: Vec<Uuid> = if let (Some(st), Some(sid)) = (q.scope_type.as_deref(), q.scope_id) {
        require_scope_read_access(&state, uid, st, sid).await?;
        sqlx::query_scalar(
            "SELECT id FROM plugin_installations WHERE scope_type = $1 AND scope_id = $2 ORDER BY display_order, created_at",
        )
        .bind(st)
        .bind(sid)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_scalar(
            r#"SELECT id FROM plugin_installations
               WHERE (scope_type IN ('user', 'device', 'conversation') AND scope_id = $1)
                  OR (scope_type = 'server' AND scope_id IN (SELECT server_id FROM server_members WHERE user_id = $1))
                  OR (scope_type = 'group' AND scope_id IN (SELECT group_id FROM group_members WHERE user_id = $1))
                  OR (scope_type = 'channel' AND scope_id IN (SELECT c.id FROM channels c JOIN server_members m ON m.server_id = c.server_id WHERE m.user_id = $1))
               ORDER BY display_order, created_at"#,
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await?
    };

    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Ok(full) = fetch_installation_full(&state.db, id, uid, None).await {
            out.push(full);
        }
    }
    Ok(Json(out))
}

pub async fn update_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
    Json(req): Json<UpdateInstallationReq>,
) -> AppResult<Json<InstallationFull>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query(
        "SELECT scope_type, scope_id, package_id, version_id, enabled, display_order, ui_slot, allowed_channels, blocked_channels, allowed_roles, settings
         FROM plugin_installations WHERE id = $1",
    )
    .bind(inst_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("installation_not_found"))?;

    let scope_type: String = cur.get("scope_type");
    let scope_id: Uuid = cur.get("scope_id");
    let package_id: Uuid = cur.get("package_id");
    require_scope_manage_access(&state, uid, &scope_type, scope_id).await?;

    let mut target_ver_id: Uuid = cur.get("version_id");
    if let Some(new_vid) = req.version_id {
        let vrow = sqlx::query(
            "SELECT v.package_id, v.requested_permissions, v.approved_permissions, v.sensitive_approval_status, v.revoked,
                    pub.verification_status AS pub_status
             FROM plugin_versions v
             JOIN plugin_packages p ON p.id = v.package_id
             JOIN plugin_publishers pub ON pub.id = p.publisher_id
             WHERE v.id = $1",
        )
        .bind(new_vid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("version_not_found"))?;
        let v_pkg: Uuid = vrow.get("package_id");
        let revoked: bool = vrow.get("revoked");
        if v_pkg != package_id {
            return Err(AppError::BadRequest("version_package_mismatch"));
        }
        if revoked {
            return Err(AppError::Forbidden("version_revoked"));
        }
        let req_perms: Vec<String> = serde_json::from_value(vrow.get("requested_permissions")).unwrap_or_default();
        let app_perms: Vec<String> = serde_json::from_value(vrow.get("approved_permissions")).unwrap_or_default();
        let sens_status: String = vrow.get("sensitive_approval_status");
        let pub_status: String = vrow.get("pub_status");

        // Enforce no silent privilege escalation on plugin update:
        // Any NEW permission required by the new version must be explicitly provided in additional_grants.
        let existing_grants: Vec<String> = sqlx::query_scalar(
            "SELECT permission FROM plugin_grants WHERE installation_id = $1 AND revoked_at IS NULL",
        )
        .bind(inst_id)
        .fetch_all(&state.db)
        .await?;

        for g in &req.additional_grants {
            validate_grant_request(g, &req_perms, &app_perms, &sens_status, &pub_status, &scope_type)?;
        }
        let added_names: Vec<&str> = req.additional_grants.iter().map(|g| g.permission.as_str()).collect();
        for rp in &req_perms {
            if is_sensitive_permission(rp) && !existing_grants.contains(rp) && !added_names.contains(&rp.as_str()) {
                return Err(AppError::Forbidden("update_requires_new_permission_consent"));
            }
        }
        for g in &req.additional_grants {
            sqlx::query(
                "INSERT INTO plugin_grants (id, installation_id, user_id, permission, scope_type, scope_id, device_only, expires_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(Uuid::new_v4())
            .bind(inst_id)
            .bind(uid)
            .bind(&g.permission)
            .bind(&g.scope_type)
            .bind(g.scope_id)
            .bind(g.device_only)
            .bind(g.expires_at)
            .execute(&state.db)
            .await?;
        }
        target_ver_id = new_vid;
    }

    let enabled = req.enabled.unwrap_or_else(|| cur.get("enabled"));
    let display_order = req.display_order.unwrap_or_else(|| cur.get("display_order"));
    let ui_slot = req.ui_slot.unwrap_or_else(|| cur.get("ui_slot"));
    let allowed_channels = req
        .allowed_channels
        .map(|v| serde_json::to_value(v).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| cur.get("allowed_channels"));
    let blocked_channels = req
        .blocked_channels
        .map(|v| serde_json::to_value(v).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| cur.get("blocked_channels"));
    let allowed_roles = req
        .allowed_roles
        .map(|v| serde_json::to_value(v).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| cur.get("allowed_roles"));
    let settings = req.settings.unwrap_or_else(|| cur.get("settings"));

    sqlx::query(
        "UPDATE plugin_installations
         SET version_id = $1, enabled = $2, display_order = $3, ui_slot = $4,
             allowed_channels = $5, blocked_channels = $6, allowed_roles = $7,
             settings = $8, sync_version = sync_version + 1, updated_at = now()
         WHERE id = $9",
    )
    .bind(target_ver_id)
    .bind(enabled)
    .bind(display_order)
    .bind(&ui_slot)
    .bind(&allowed_channels)
    .bind(&blocked_channels)
    .bind(&allowed_roles)
    .bind(&settings)
    .bind(inst_id)
    .execute(&state.db)
    .await?;

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'update', $7)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(package_id)
    .bind(uid)
    .bind(&scope_type)
    .bind(scope_id)
    .bind(json!({ "enabled": enabled, "version_id": target_ver_id }))
    .execute(&state.db)
    .await;

    broadcast_plugin_sync(
        &state,
        &scope_type,
        scope_id,
        uid,
        "updated",
        inst_id,
        json!({ "enabled": enabled, "settings": settings }),
    )
    .await;

    let full = fetch_installation_full(&state.db, inst_id, uid, None).await?;
    Ok(Json(full))
}

pub async fn delete_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query("SELECT scope_type, scope_id, package_id FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("installation_not_found"))?;
    let scope_type: String = cur.get("scope_type");
    let scope_id: Uuid = cur.get("scope_id");
    let package_id: Uuid = cur.get("package_id");
    require_scope_manage_access(&state, uid, &scope_type, scope_id).await?;

    // Disable linked webhooks before removing installation
    sqlx::query("UPDATE plugin_webhooks SET enabled = false WHERE installation_id = $1")
        .bind(inst_id)
        .execute(&state.db)
        .await?;

    sqlx::query("DELETE FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .execute(&state.db)
        .await?;

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'uninstall', '{}'::jsonb)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(package_id)
    .bind(uid)
    .bind(&scope_type)
    .bind(scope_id)
    .execute(&state.db)
    .await;

    broadcast_plugin_sync(&state, &scope_type, scope_id, uid, "uninstalled", inst_id, json!({})).await;
    Ok(Json(json!({ "uninstalled": true, "installation_id": inst_id })))
}

// ============================================================================
// Permission Grants, Instant Revocation & Privacy Dashboard
// ============================================================================

pub async fn grant_permission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
    Json(req): Json<GrantSpec>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let cur = sqlx::query(
        "SELECT i.scope_type, i.scope_id, i.package_id,
                v.requested_permissions, v.approved_permissions, v.sensitive_approval_status,
                pub.verification_status AS pub_status
         FROM plugin_installations i
         JOIN plugin_versions v ON v.id = i.version_id
         JOIN plugin_packages p ON p.id = i.package_id
         JOIN plugin_publishers pub ON pub.id = p.publisher_id
         WHERE i.id = $1",
    )
    .bind(inst_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("installation_not_found"))?;

    let inst_scope: String = cur.get("scope_type");
    let inst_scope_id: Uuid = cur.get("scope_id");
    let package_id: Uuid = cur.get("package_id");
    require_scope_manage_access(&state, uid, &inst_scope, inst_scope_id).await?;

    let req_perms: Vec<String> = serde_json::from_value(cur.get("requested_permissions")).unwrap_or_default();
    let app_perms: Vec<String> = serde_json::from_value(cur.get("approved_permissions")).unwrap_or_default();
    let sens_status: String = cur.get("sensitive_approval_status");
    let pub_status: String = cur.get("pub_status");
    validate_grant_request(&req, &req_perms, &app_perms, &sens_status, &pub_status, &inst_scope)?;

    let gid = Uuid::new_v4();
    let def = find_permission(&req.permission);
    let device_only = req.device_only || def.map(|d| d.device_only_default).unwrap_or(false);
    sqlx::query(
        "INSERT INTO plugin_grants (id, installation_id, user_id, permission, scope_type, scope_id, device_only, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(gid)
    .bind(inst_id)
    .bind(uid)
    .bind(&req.permission)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .bind(device_only)
    .bind(req.expires_at)
    .execute(&state.db)
    .await?;

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, permission, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'grant', $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(package_id)
    .bind(uid)
    .bind(&req.scope_type)
    .bind(req.scope_id)
    .bind(&req.permission)
    .bind(json!({ "grant_id": gid, "device_only": device_only }))
    .execute(&state.db)
    .await;

    broadcast_plugin_sync(
        &state,
        &inst_scope,
        inst_scope_id,
        uid,
        "permission_granted",
        inst_id,
        json!({ "grant_id": gid, "permission": req.permission, "scope_type": req.scope_type, "scope_id": req.scope_id }),
    )
    .await;

    Ok(Json(json!({ "grant_id": gid, "permission": req.permission, "device_only": device_only })))
}

pub async fn revoke_permission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((inst_id, grant_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT g.user_id, g.permission, i.scope_type, i.scope_id, i.package_id
         FROM plugin_grants g JOIN plugin_installations i ON i.id = g.installation_id
         WHERE g.id = $1 AND g.installation_id = $2",
    )
    .bind(grant_id)
    .bind(inst_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("grant_not_found"))?;

    let g_uid: Uuid = row.get("user_id");
    let perm: String = row.get("permission");
    let scope_type: String = row.get("scope_type");
    let scope_id: Uuid = row.get("scope_id");
    let package_id: Uuid = row.get("package_id");
    if g_uid != uid {
        require_scope_manage_access(&state, uid, &scope_type, scope_id).await?;
    }

    sqlx::query("UPDATE plugin_grants SET revoked_at = now() WHERE id = $1")
        .bind(grant_id)
        .execute(&state.db)
        .await?;

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, permission, detail)
         VALUES ($1, $2, $3, $4, $5, $6, 'revoke', $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(package_id)
    .bind(uid)
    .bind(&scope_type)
    .bind(scope_id)
    .bind(&perm)
    .bind(json!({ "grant_id": grant_id }))
    .execute(&state.db)
    .await;

    broadcast_plugin_sync(
        &state,
        &scope_type,
        scope_id,
        uid,
        "permission_revoked",
        inst_id,
        json!({ "grant_id": grant_id, "permission": perm }),
    )
    .await;

    Ok(Json(json!({ "revoked": true, "grant_id": grant_id, "permission": perm })))
}

pub async fn privacy_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let grants = sqlx::query(
        r#"SELECT g.id, g.installation_id, g.permission, g.scope_type, g.scope_id, g.device_only,
                  g.expires_at, g.granted_at, g.revoked_at,
                  i.scope_type AS inst_scope_type, i.scope_id AS inst_scope_id, i.enabled,
                  p.plugin_key, p.name AS plugin_name, pub.display_name AS publisher_name,
                  pub.verification_status AS publisher_status
           FROM plugin_grants g
           JOIN plugin_installations i ON i.id = g.installation_id
           JOIN plugin_packages p ON p.id = i.package_id
           JOIN plugin_publishers pub ON pub.id = p.publisher_id
           WHERE g.user_id = $1
           ORDER BY g.revoked_at NULLS FIRST, g.granted_at DESC
           LIMIT 200"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let logs = sqlx::query(
        "SELECT id, installation_id, package_id, scope_type, scope_id, action, permission, detail, created_at
         FROM plugin_audit_logs
         WHERE actor_user_id = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "grants": grants.into_iter().map(|r| {
            let perm: String = r.get("permission");
            let def = find_permission(&perm);
            let pstat: String = r.get("publisher_status");
            json!({
                "id": r.get::<Uuid, _>("id"),
                "installation_id": r.get::<Uuid, _>("installation_id"),
                "plugin_key": r.get::<String, _>("plugin_key"),
                "plugin_name": r.get::<String, _>("plugin_name"),
                "publisher_name": r.get::<String, _>("publisher_name"),
                "publisher_verified": pstat == "verified",
                "enabled": r.get::<bool, _>("enabled"),
                "permission": perm,
                "permission_label": def.map(|d| d.label).unwrap_or(""),
                "sensitive": def.map(|d| d.sensitive).unwrap_or(true),
                "e2ee": def.map(|d| d.e2ee).unwrap_or(false),
                "scope_type": r.get::<String, _>("scope_type"),
                "scope_id": r.get::<Option<Uuid>, _>("scope_id"),
                "installation_scope_type": r.get::<String, _>("inst_scope_type"),
                "installation_scope_id": r.get::<Uuid, _>("inst_scope_id"),
                "device_only": r.get::<bool, _>("device_only"),
                "expires_at": r.get::<Option<DateTime<Utc>>, _>("expires_at"),
                "granted_at": r.get::<DateTime<Utc>, _>("granted_at"),
                "revoked_at": r.get::<Option<DateTime<Utc>>, _>("revoked_at"),
            })
        }).collect::<Vec<_>>(),
        "recent_activity": logs.into_iter().map(|l| json!({
            "id": l.get::<Uuid, _>("id"),
            "installation_id": l.get::<Option<Uuid>, _>("installation_id"),
            "scope_type": l.get::<String, _>("scope_type"),
            "scope_id": l.get::<Option<Uuid>, _>("scope_id"),
            "action": l.get::<String, _>("action"),
            "permission": l.get::<Option<String>, _>("permission"),
            "detail": l.get::<Value, _>("detail"),
            "created_at": l.get::<DateTime<Utc>, _>("created_at"),
        })).collect::<Vec<_>>(),
    })))
}

pub async fn has_active_grant(
    db: &PgPool,
    installation_id: Uuid,
    user_id: Uuid,
    permission: &str,
    conversation_scope_id: Option<Uuid>,
) -> AppResult<bool> {
    let personal = is_personal_permission(permission);
    let rows = if personal {
        sqlx::query(
            "SELECT scope_type, scope_id FROM plugin_grants
             WHERE installation_id = $1 AND user_id = $2 AND permission = $3
               AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())",
        )
        .bind(installation_id)
        .bind(user_id)
        .bind(permission)
        .fetch_all(db)
        .await?
    } else {
        // Non-personal permissions (e.g., network.outbound, storage.local, bots.manage) can be satisfied
        // either by the current user's grant OR by the server/group installation owner's grant on a shared installation.
        sqlx::query(
            "SELECT g.scope_type, g.scope_id
             FROM plugin_grants g
             JOIN plugin_installations i ON i.id = g.installation_id
             WHERE g.installation_id = $1 AND g.permission = $3
               AND (g.user_id = $2 OR (i.scope_type IN ('server', 'group', 'channel') AND g.user_id = i.installed_by AND NOT g.device_only))
               AND g.revoked_at IS NULL AND (g.expires_at IS NULL OR g.expires_at > now())",
        )
        .bind(installation_id)
        .bind(user_id)
        .bind(permission)
        .fetch_all(db)
        .await?
    };

    for r in rows {
        let st: String = r.get("scope_type");
        let sid: Option<Uuid> = r.get("scope_id");
        if st == "global" && !permission.starts_with("e2ee.") {
            return Ok(true);
        }
        if let Some(target) = conversation_scope_id {
            if sid == Some(target) {
                return Ok(true);
            }
        } else if st != "conversation" || matches!(permission, "e2ee.egress_external" | "e2ee.store_external") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Checks server-side whether `(installation_id, user_id)` is E2EE-tainted or locked to local-only E2EE processing:
/// - Any active `e2ee.read_decrypted` or `e2ee.process_local` grant for `(installation_id, user_id)`, OR
/// - Any recorded `e2ee_read` audit entry in `plugin_audit_logs` for `(installation_id, user_id)`.
pub async fn is_server_side_e2ee_tainted(
    db: &PgPool,
    installation_id: Uuid,
    user_id: Uuid,
) -> AppResult<bool> {
    let has_e2ee_grant_or_audit: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM plugin_grants
           WHERE installation_id = $1 AND user_id = $2
             AND permission IN ('e2ee.read_decrypted', 'e2ee.process_local')
             AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())
           UNION ALL
           SELECT 1 FROM plugin_audit_logs
           WHERE installation_id = $1 AND actor_user_id = $2
             AND action = 'e2ee_read'
           LIMIT 1"#,
    )
    .bind(installation_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    Ok(has_e2ee_grant_or_audit.is_some())
}

/// Verifies E2EE or data-access permission server-side and records an immutable audit trail entry.
pub async fn record_plugin_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(inst_id): Path<Uuid>,
    Json(req): Json<AuditRecordReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let inst = sqlx::query("SELECT package_id, scope_type, scope_id, enabled FROM plugin_installations WHERE id = $1")
        .bind(inst_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("installation_not_found"))?;
    let pkg_id: Uuid = inst.get("package_id");
    let st: String = inst.get("scope_type");
    let sid: Uuid = inst.get("scope_id");
    let enabled: bool = inst.get("enabled");
    require_scope_read_access(&state, uid, &st, sid).await?;

    if let Some(ref perm) = req.permission {
        if !enabled {
            return Err(AppError::Forbidden("plugin_disabled"));
        }
        if !has_active_grant(&state.db, inst_id, uid, perm, req.conversation_id).await? {
            return Err(AppError::Forbidden("permission_denied"));
        }
    }

    sqlx::query(
        "INSERT INTO plugin_audit_logs (id, installation_id, package_id, actor_user_id, scope_type, scope_id, action, permission, detail)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(inst_id)
    .bind(pkg_id)
    .bind(uid)
    .bind(&st)
    .bind(req.conversation_id.unwrap_or(sid))
    .bind(req.action.trim())
    .bind(req.permission.as_deref())
    .bind(req.detail.unwrap_or_else(|| json!({})))
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "ok": true })))
}

pub use crate::plugins::runtime_bridge::{
    delete_plugin_storage, list_plugin_bots, list_plugin_storage, plugin_network_fetch,
    put_plugin_storage, register_plugin_bot, sync_plugins_state,
};
