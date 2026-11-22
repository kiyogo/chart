//! Plugin Marketplace, Developer Portal, Publisher Verification & Sensitive Permission Approval.

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
        schema::{
            PERMISSION_CATALOG, PluginManifest, compute_package_sha256, is_safe_http_url,
            is_sensitive_permission, is_valid_permission, is_valid_slug,
            validate_manifest_and_artifacts, verify_package_signature,
        },
        security::is_platform_admin,
    },
    privacy::auth_user,
};

#[derive(Serialize)]
pub struct PublisherItem {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    pub website_url: Option<String>,
    pub bio: Option<String>,
    pub verification_status: String,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct MarketplacePackageItem {
    pub id: Uuid,
    pub plugin_key: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub category: String,
    pub icon_url: String,
    pub screenshots: Value,
    pub video_url: Option<String>,
    pub platforms: Value,
    pub is_public: bool,
    pub is_builtin: bool,
    pub latest_version: String,
    pub install_count: i64,
    pub rating_avg: f64,
    pub rating_count: i32,
    pub publisher_id: Uuid,
    pub publisher_slug: String,
    pub publisher_name: String,
    pub publisher_verified: bool,
    pub latest_version_id: Option<Uuid>,
    pub runtime: String,
    pub requested_permissions: Value,
    pub approved_permissions: Value,
    pub sensitive_approval_status: String,
    pub sha256: String,
}

#[derive(Deserialize)]
pub struct RegisterPublisherReq {
    pub slug: String,
    pub display_name: String,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
}

#[derive(Deserialize)]
pub struct PublishPackageReq {
    pub manifest: PluginManifest,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub screenshots: Option<Vec<String>>,
    #[serde(default)]
    pub video_url: Option<String>,
    #[serde(default = "default_true_b")]
    pub is_public: bool,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub bundle_code: String,
    #[serde(default)]
    pub ui_html: String,
    #[serde(default)]
    pub wasm_base64: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

fn default_true_b() -> bool {
    true
}

#[derive(Deserialize)]
pub struct PublishVersionReq {
    pub manifest: PluginManifest,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub bundle_code: String,
    #[serde(default)]
    pub ui_html: String,
    #[serde(default)]
    pub wasm_base64: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Deserialize)]
pub struct RollbackReq {
    pub version: String,
}

#[derive(Deserialize)]
pub struct MarketplaceQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub verified: Option<bool>,
    pub mine: Option<bool>,
}

#[derive(Deserialize)]
pub struct ReviewReq {
    pub rating: i32,
    #[serde(default)]
    pub comment: String,
}

#[derive(Deserialize)]
pub struct ReportReq {
    pub reason: String,
    #[serde(default)]
    pub details: String,
}

#[derive(Deserialize)]
pub struct ApproveSensitivePermsReq {
    pub status: String, // "approved" | "rejected"
    #[serde(default)]
    pub approved_permissions: Vec<String>,
}

pub async fn get_permission_catalog() -> Json<Value> {
    Json(json!({ "permissions": PERMISSION_CATALOG }))
}

pub async fn get_my_publisher(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Option<PublisherItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT id, user_id, slug, display_name, website_url, bio, verification_status, verified_at, created_at
         FROM plugin_publishers WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(row.map(|r| {
        let status: String = r.get("verification_status");
        PublisherItem {
            id: r.get("id"),
            user_id: r.get("user_id"),
            slug: r.get("slug"),
            display_name: r.get("display_name"),
            website_url: r.get("website_url"),
            bio: r.get("bio"),
            verified: status == "verified",
            verification_status: status,
            verified_at: r.get("verified_at"),
            created_at: r.get("created_at"),
        }
    })))
}

pub async fn register_publisher(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterPublisherReq>,
) -> AppResult<Json<PublisherItem>> {
    let uid = auth_user(&state, &headers).await?;
    let slug = req.slug.trim().to_ascii_lowercase();
    if !is_valid_slug(&slug) || slug == "chatw" || slug == "system" || slug == "admin" {
        return Err(AppError::BadRequest("invalid_publisher_slug"));
    }
    let display = req.display_name.trim();
    if display.len() < 2 || display.len() > 64 {
        return Err(AppError::BadRequest("invalid_display_name"));
    }
    if let Some(ref w) = req.website_url {
        if !is_safe_http_url(w) {
            return Err(AppError::BadRequest("invalid_website_url"));
        }
    }

    // Check existing publisher for user or colliding slug
    let existing_user: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM plugin_publishers WHERE user_id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?;
    if existing_user.is_some() {
        return Err(AppError::Conflict("publisher_already_registered"));
    }
    let existing_slug: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM plugin_publishers WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(&state.db)
            .await?;
    if existing_slug.is_some() {
        return Err(AppError::Conflict("publisher_slug_taken"));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO plugin_publishers (id, user_id, slug, display_name, website_url, bio, public_key, verification_status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'unverified', $8, $8)",
    )
    .bind(id)
    .bind(uid)
    .bind(&slug)
    .bind(display)
    .bind(req.website_url.as_deref())
    .bind(req.bio.as_deref())
    .bind(req.public_key.as_deref())
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(PublisherItem {
        id,
        user_id: Some(uid),
        slug,
        display_name: display.to_string(),
        website_url: req.website_url,
        bio: req.bio,
        verification_status: "unverified".into(),
        verified: false,
        verified_at: None,
        created_at: now,
    }))
}

pub async fn request_publisher_verification(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let res = sqlx::query(
        "UPDATE plugin_publishers SET verification_status = 'pending', updated_at = now()
         WHERE user_id = $1 AND verification_status IN ('unverified', 'rejected')
         RETURNING id",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;
    let Some(r) = res else {
        return Err(AppError::BadRequest("verification_not_eligible"));
    };
    let pid: Uuid = r.get("id");
    Ok(Json(json!({ "publisher_id": pid, "verification_status": "pending" })))
}

pub async fn admin_decide_publisher(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((publisher_id, decision)): Path<(Uuid, String)>,
) -> AppResult<Json<Value>> {
    let admin_id = auth_user(&state, &headers).await?;
    if !is_platform_admin(&state, admin_id).await? {
        return Err(AppError::Forbidden("admin_only"));
    }
    let status = match decision.as_str() {
        "verify" => "verified",
        "reject" => "rejected",
        "revoke" => "unverified",
        _ => return Err(AppError::BadRequest("invalid_decision")),
    };
    let row = sqlx::query(
        "UPDATE plugin_publishers
         SET verification_status = $1,
             verified_at = CASE WHEN $1 = 'verified' THEN now() ELSE NULL END,
             verified_by = CASE WHEN $1 = 'verified' THEN $2 ELSE NULL END,
             updated_at = now()
         WHERE id = $3
         RETURNING id, slug",
    )
    .bind(status)
    .bind(admin_id)
    .bind(publisher_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("publisher_not_found"))?;

    let slug: String = row.get("slug");
    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, actor_user_id, scope_type, action, detail)
         VALUES ($1, $2, 'platform', 'publisher_verification', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(admin_id)
    .bind(json!({ "publisher_id": publisher_id, "slug": slug, "status": status }))
    .execute(&state.db)
    .await;

    Ok(Json(json!({
        "publisher_id": publisher_id,
        "slug": slug,
        "verification_status": status,
        "verified": status == "verified",
    })))
}

pub async fn admin_decide_version_permissions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(version_id): Path<Uuid>,
    Json(req): Json<ApproveSensitivePermsReq>,
) -> AppResult<Json<Value>> {
    let admin_id = auth_user(&state, &headers).await?;
    if !is_platform_admin(&state, admin_id).await? {
        return Err(AppError::Forbidden("admin_only"));
    }
    if !matches!(req.status.as_str(), "approved" | "rejected") {
        return Err(AppError::BadRequest("invalid_status"));
    }
    for p in &req.approved_permissions {
        if !is_valid_permission(p) {
            return Err(AppError::BadRequest("unknown_permission"));
        }
    }

    let ver = sqlx::query("SELECT package_id, requested_permissions FROM plugin_versions WHERE id = $1")
        .bind(version_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("version_not_found"))?;

    let package_id: Uuid = ver.get("package_id");
    let req_perms: Vec<String> = serde_json::from_value(ver.get("requested_permissions")).unwrap_or_default();

    // Only approve permissions that the version actually requested
    let filtered: Vec<String> = if req.status == "approved" {
        req.approved_permissions
            .into_iter()
            .filter(|p| req_perms.contains(p))
            .collect()
    } else {
        Vec::new()
    };

    sqlx::query(
        "UPDATE plugin_versions
         SET sensitive_approval_status = $1, approved_permissions = $2, approved_by = $3, approved_at = now()
         WHERE id = $4",
    )
    .bind(&req.status)
    .bind(serde_json::to_value(&filtered).unwrap_or_else(|_| json!([])))
    .bind(admin_id)
    .bind(version_id)
    .execute(&state.db)
    .await?;

    let _ = sqlx::query(
        "INSERT INTO plugin_audit_logs (id, package_id, actor_user_id, scope_type, action, detail)
         VALUES ($1, $2, $3, 'platform', 'sensitive_permission_review', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(package_id)
    .bind(admin_id)
    .bind(json!({ "version_id": version_id, "status": req.status, "approved_permissions": filtered }))
    .execute(&state.db)
    .await;

    Ok(Json(json!({
        "version_id": version_id,
        "sensitive_approval_status": req.status,
        "approved_permissions": filtered,
    })))
}

pub async fn publish_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut req): Json<PublishPackageReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let pub_row = sqlx::query("SELECT id, slug, public_key FROM plugin_publishers WHERE user_id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Forbidden("publisher_registration_required"))?;
    let publisher_id: Uuid = pub_row.get("id");
    let publisher_slug: String = pub_row.get("slug");
    let publisher_pubkey: Option<String> = pub_row.try_get("public_key").ok().flatten();

    req.manifest.publisher = publisher_slug.clone();
    let (sha256, size_bytes, clean_perms) = validate_manifest_and_artifacts(
        &req.manifest,
        &publisher_slug,
        &req.bundle_code,
        &req.ui_html,
        &req.wasm_base64,
        req.sha256.as_deref(),
    )
    .map_err(AppError::BadRequest)?;

    let signature_verified = match (publisher_pubkey.as_deref().filter(|s| !s.trim().is_empty()), req.signature.as_deref().filter(|s| !s.trim().is_empty())) {
        (Some(pk), Some(sig)) => {
            verify_package_signature(pk, sig, &req.manifest.plugin_key, &req.manifest.version, &sha256)
                .map_err(AppError::BadRequest)?;
            true
        }
        (Some(_), None) => return Err(AppError::BadRequest("package_signature_required")),
        (None, Some(_)) => return Err(AppError::BadRequest("publisher_public_key_required")),
        (None, None) => false,
    };

    let icon_url = req.icon_url.unwrap_or_default();
    if !icon_url.is_empty() && !icon_url.starts_with('/') && !icon_url.starts_with("logo:") && !is_safe_http_url(&icon_url) {
        return Err(AppError::BadRequest("invalid_icon_url"));
    }
    if let Some(ref v) = req.video_url {
        if !is_safe_http_url(v) {
            return Err(AppError::BadRequest("invalid_video_url"));
        }
    }

    // Prevent overwriting an existing plugin_key owned by another publisher (or duplicate)
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM plugin_packages WHERE plugin_key = $1")
            .bind(&req.manifest.plugin_key)
            .fetch_optional(&state.db)
            .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("plugin_key_already_exists"));
    }

    let has_sensitive = clean_perms.iter().any(|p| is_sensitive_permission(p));
    let sens_status = if has_sensitive { "pending" } else { "not_required" };
    let category = req.category.unwrap_or_else(|| "utility".into());
    let platforms = if req.manifest.platforms.is_empty() {
        json!(["web", "linux", "windows", "macos"])
    } else {
        serde_json::to_value(&req.manifest.platforms).unwrap_or_else(|_| json!(["web"]))
    };
    let screenshots = serde_json::to_value(req.screenshots.unwrap_or_default()).unwrap_or_else(|_| json!([]));

    let pkg_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let mut tx = state.db.begin().await?;

    sqlx::query(
        "INSERT INTO plugin_packages (id, plugin_key, publisher_id, name, summary, description, category, icon_url, screenshots, video_url, platforms, is_public, is_builtin, latest_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, false, $13)",
    )
    .bind(pkg_id)
    .bind(&req.manifest.plugin_key)
    .bind(publisher_id)
    .bind(req.manifest.name.trim())
    .bind(req.summary.trim())
    .bind(req.description.trim())
    .bind(category.trim())
    .bind(&icon_url)
    .bind(screenshots)
    .bind(req.video_url.as_deref())
    .bind(platforms)
    .bind(req.is_public)
    .bind(&req.manifest.version)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO plugin_versions (id, package_id, version, changelog, runtime, sdk_version, manifest, bundle_code, wasm_base64, ui_html, sha256, signature, signature_verified, size_bytes, requested_permissions, sensitive_approval_status, approved_permissions)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, '[]'::jsonb)",
    )
    .bind(ver_id)
    .bind(pkg_id)
    .bind(&req.manifest.version)
    .bind(req.changelog.trim())
    .bind(&req.manifest.runtime)
    .bind(&req.manifest.sdk_version)
    .bind(serde_json::to_value(&req.manifest).unwrap_or_else(|_| json!({})))
    .bind(&req.bundle_code)
    .bind(&req.wasm_base64)
    .bind(&req.ui_html)
    .bind(&sha256)
    .bind(req.signature.as_deref())
    .bind(signature_verified)
    .bind(size_bytes as i32)
    .bind(serde_json::to_value(&clean_perms).unwrap_or_else(|_| json!([])))
    .bind(sens_status)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "package_id": pkg_id,
        "version_id": ver_id,
        "plugin_key": req.manifest.plugin_key,
        "version": req.manifest.version,
        "sha256": sha256,
        "signature_verified": signature_verified,
        "size_bytes": size_bytes,
        "sensitive_approval_status": sens_status,
        "requested_permissions": clean_perms,
    })))
}

pub async fn publish_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(package_id): Path<Uuid>,
    Json(mut req): Json<PublishVersionReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let pkg = sqlx::query(
        "SELECT p.plugin_key, p.publisher_id, pub.slug, pub.user_id, pub.public_key
         FROM plugin_packages p
         JOIN plugin_publishers pub ON pub.id = p.publisher_id
         WHERE p.id = $1",
    )
    .bind(package_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("package_not_found"))?;

    let owner_uid: Option<Uuid> = pkg.get("user_id");
    if owner_uid != Some(uid) {
        return Err(AppError::Forbidden("not_package_owner"));
    }
    let plugin_key: String = pkg.get("plugin_key");
    let pub_slug: String = pkg.get("slug");
    let pub_pubkey: Option<String> = pkg.try_get("public_key").ok().flatten();
    if req.manifest.plugin_key != plugin_key {
        return Err(AppError::BadRequest("plugin_key_immutable"));
    }
    req.manifest.publisher = pub_slug.clone();

    let (sha256, size_bytes, clean_perms) = validate_manifest_and_artifacts(
        &req.manifest,
        &pub_slug,
        &req.bundle_code,
        &req.ui_html,
        &req.wasm_base64,
        req.sha256.as_deref(),
    )
    .map_err(AppError::BadRequest)?;

    let signature_verified = match (pub_pubkey.as_deref().filter(|s| !s.trim().is_empty()), req.signature.as_deref().filter(|s| !s.trim().is_empty())) {
        (Some(pk), Some(sig)) => {
            verify_package_signature(pk, sig, &req.manifest.plugin_key, &req.manifest.version, &sha256)
                .map_err(AppError::BadRequest)?;
            true
        }
        (Some(_), None) => return Err(AppError::BadRequest("package_signature_required")),
        (None, Some(_)) => return Err(AppError::BadRequest("publisher_public_key_required")),
        (None, None) => false,
    };

    let dup: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM plugin_versions WHERE package_id = $1 AND version = $2",
    )
    .bind(package_id)
    .bind(&req.manifest.version)
    .fetch_optional(&state.db)
    .await?;
    if dup.is_some() {
        return Err(AppError::Conflict("version_already_exists"));
    }

    let has_sensitive = clean_perms.iter().any(|p| is_sensitive_permission(p));
    let sens_status = if has_sensitive { "pending" } else { "not_required" };
    let ver_id = Uuid::new_v4();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO plugin_versions (id, package_id, version, changelog, runtime, sdk_version, manifest, bundle_code, wasm_base64, ui_html, sha256, signature, signature_verified, size_bytes, requested_permissions, sensitive_approval_status, approved_permissions)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, '[]'::jsonb)",
    )
    .bind(ver_id)
    .bind(package_id)
    .bind(&req.manifest.version)
    .bind(req.changelog.trim())
    .bind(&req.manifest.runtime)
    .bind(&req.manifest.sdk_version)
    .bind(serde_json::to_value(&req.manifest).unwrap_or_else(|_| json!({})))
    .bind(&req.bundle_code)
    .bind(&req.wasm_base64)
    .bind(&req.ui_html)
    .bind(&sha256)
    .bind(req.signature.as_deref())
    .bind(signature_verified)
    .bind(size_bytes as i32)
    .bind(serde_json::to_value(&clean_perms).unwrap_or_else(|_| json!([])))
    .bind(sens_status)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE plugin_packages SET latest_version = $1, updated_at = now() WHERE id = $2")
        .bind(&req.manifest.version)
        .bind(package_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "package_id": package_id,
        "version_id": ver_id,
        "version": req.manifest.version,
        "sha256": sha256,
        "signature_verified": signature_verified,
        "sensitive_approval_status": sens_status,
    })))
}

pub async fn rollback_package_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(package_id): Path<Uuid>,
    Json(req): Json<RollbackReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let pkg = sqlx::query(
        "SELECT pub.user_id FROM plugin_packages p JOIN plugin_publishers pub ON pub.id = p.publisher_id WHERE p.id = $1",
    )
    .bind(package_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("package_not_found"))?;
    let owner: Option<Uuid> = pkg.get("user_id");
    if owner != Some(uid) && !is_platform_admin(&state, uid).await? {
        return Err(AppError::Forbidden("not_package_owner"));
    }
    let ver = sqlx::query(
        "SELECT id FROM plugin_versions WHERE package_id = $1 AND version = $2 AND NOT revoked",
    )
    .bind(package_id)
    .bind(req.version.trim())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("version_not_found"))?;
    let vid: Uuid = ver.get("id");
    sqlx::query("UPDATE plugin_packages SET latest_version = $1, updated_at = now() WHERE id = $2")
        .bind(req.version.trim())
        .bind(package_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "package_id": package_id, "latest_version": req.version.trim(), "version_id": vid })))
}

pub async fn revoke_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(version_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT v.package_id, pub.user_id
         FROM plugin_versions v
         JOIN plugin_packages p ON p.id = v.package_id
         JOIN plugin_publishers pub ON pub.id = p.publisher_id
         WHERE v.id = $1",
    )
    .bind(version_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("version_not_found"))?;
    let owner: Option<Uuid> = row.get("user_id");
    if owner != Some(uid) && !is_platform_admin(&state, uid).await? {
        return Err(AppError::Forbidden("not_package_owner"));
    }
    sqlx::query("UPDATE plugin_versions SET revoked = true WHERE id = $1")
        .bind(version_id)
        .execute(&state.db)
        .await?;
    // Disable active installations on revoked version immediately
    sqlx::query("UPDATE plugin_installations SET enabled = false, updated_at = now() WHERE version_id = $1")
        .bind(version_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "version_id": version_id, "revoked": true })))
}

pub async fn list_marketplace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MarketplaceQuery>,
) -> AppResult<Json<Vec<MarketplacePackageItem>>> {
    let uid = auth_user(&state, &headers).await?;
    let search = q.q.unwrap_or_default().trim().to_lowercase();
    let cat = q.category.unwrap_or_default().trim().to_lowercase();
    let verified_only = q.verified.unwrap_or(false);
    let mine_only = q.mine.unwrap_or(false);

    let rows = sqlx::query(
        r#"SELECT p.id, p.plugin_key, p.name, p.summary, p.description, p.category, p.icon_url,
                  p.screenshots, p.video_url, p.platforms, p.is_public, p.is_builtin,
                  p.latest_version, p.install_count, p.rating_avg, p.rating_count,
                  pub.id AS publisher_id, pub.slug AS publisher_slug, pub.display_name AS publisher_name,
                  pub.verification_status AS publisher_status, pub.user_id AS publisher_uid,
                  v.id AS version_id, v.runtime, v.requested_permissions, v.approved_permissions,
                  v.sensitive_approval_status, v.sha256
           FROM plugin_packages p
           JOIN plugin_publishers pub ON pub.id = p.publisher_id
           LEFT JOIN plugin_versions v ON v.package_id = p.id AND v.version = p.latest_version AND NOT v.revoked
           WHERE (p.is_public = true OR pub.user_id = $1)
           ORDER BY p.is_builtin DESC, p.install_count DESC, p.created_at DESC
           LIMIT 100"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let mut out = Vec::new();
    for r in rows {
        let p_uid: Option<Uuid> = r.get("publisher_uid");
        if mine_only && p_uid != Some(uid) {
            continue;
        }
        let pub_status: String = r.get("publisher_status");
        let verified = pub_status == "verified";
        if verified_only && !verified {
            continue;
        }
        let category: String = r.get("category");
        if !cat.is_empty() && cat != "all" && category.to_lowercase() != cat {
            continue;
        }
        let name: String = r.get("name");
        let plugin_key: String = r.get("plugin_key");
        let summary: String = r.get("summary");
        if !search.is_empty()
            && !name.to_lowercase().contains(&search)
            && !plugin_key.to_lowercase().contains(&search)
            && !summary.to_lowercase().contains(&search)
        {
            continue;
        }
        out.push(MarketplacePackageItem {
            id: r.get("id"),
            plugin_key,
            name,
            summary,
            description: r.get("description"),
            category,
            icon_url: r.get("icon_url"),
            screenshots: r.get("screenshots"),
            video_url: r.get("video_url"),
            platforms: r.get("platforms"),
            is_public: r.get("is_public"),
            is_builtin: r.get("is_builtin"),
            latest_version: r.get("latest_version"),
            install_count: r.get("install_count"),
            rating_avg: r.get("rating_avg"),
            rating_count: r.get("rating_count"),
            publisher_id: r.get("publisher_id"),
            publisher_slug: r.get("publisher_slug"),
            publisher_name: r.get("publisher_name"),
            publisher_verified: verified,
            latest_version_id: r.get("version_id"),
            runtime: r.try_get::<String, _>("runtime").unwrap_or_else(|_| "js".into()),
            requested_permissions: r.try_get::<Value, _>("requested_permissions").unwrap_or_else(|_| json!([])),
            approved_permissions: r.try_get::<Value, _>("approved_permissions").unwrap_or_else(|_| json!([])),
            sensitive_approval_status: r.try_get::<String, _>("sensitive_approval_status").unwrap_or_else(|_| "not_required".into()),
            sha256: r.try_get::<String, _>("sha256").unwrap_or_default(),
        });
    }
    Ok(Json(out))
}

pub async fn get_marketplace_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_or_key): Path<String>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let pkg = if let Ok(uuid) = Uuid::parse_str(&id_or_key) {
        sqlx::query(
            "SELECT p.*, pub.slug AS publisher_slug, pub.display_name AS publisher_name, pub.verification_status AS publisher_status, pub.user_id AS publisher_uid
             FROM plugin_packages p JOIN plugin_publishers pub ON pub.id = p.publisher_id
             WHERE p.id = $1 AND (p.is_public = true OR pub.user_id = $2)",
        )
        .bind(uuid)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query(
            "SELECT p.*, pub.slug AS publisher_slug, pub.display_name AS publisher_name, pub.verification_status AS publisher_status, pub.user_id AS publisher_uid
             FROM plugin_packages p JOIN plugin_publishers pub ON pub.id = p.publisher_id
             WHERE p.plugin_key = $1 AND (p.is_public = true OR pub.user_id = $2)",
        )
        .bind(&id_or_key)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
    }
    .ok_or(AppError::NotFound("package_not_found"))?;

    let pkg_id: Uuid = pkg.get("id");
    let versions = sqlx::query(
        "SELECT id, version, changelog, runtime, sdk_version, manifest, sha256, size_bytes, requested_permissions, approved_permissions, sensitive_approval_status, revoked, created_at
         FROM plugin_versions WHERE package_id = $1 ORDER BY created_at DESC LIMIT 25",
    )
    .bind(pkg_id)
    .fetch_all(&state.db)
    .await?;

    let reviews = sqlx::query(
        "SELECT r.id, r.rating, r.comment, r.created_at, u.username
         FROM plugin_reviews r JOIN users u ON u.id = r.user_id
         WHERE r.package_id = $1 ORDER BY r.created_at DESC LIMIT 30",
    )
    .bind(pkg_id)
    .fetch_all(&state.db)
    .await?;

    let pub_status: String = pkg.get("publisher_status");
    Ok(Json(json!({
        "id": pkg_id,
        "plugin_key": pkg.get::<String, _>("plugin_key"),
        "name": pkg.get::<String, _>("name"),
        "summary": pkg.get::<String, _>("summary"),
        "description": pkg.get::<String, _>("description"),
        "category": pkg.get::<String, _>("category"),
        "icon_url": pkg.get::<String, _>("icon_url"),
        "screenshots": pkg.get::<Value, _>("screenshots"),
        "video_url": pkg.get::<Option<String>, _>("video_url"),
        "platforms": pkg.get::<Value, _>("platforms"),
        "latest_version": pkg.get::<String, _>("latest_version"),
        "install_count": pkg.get::<i64, _>("install_count"),
        "rating_avg": pkg.get::<f64, _>("rating_avg"),
        "rating_count": pkg.get::<i32, _>("rating_count"),
        "publisher_slug": pkg.get::<String, _>("publisher_slug"),
        "publisher_name": pkg.get::<String, _>("publisher_name"),
        "publisher_verified": pub_status == "verified",
        "versions": versions.into_iter().map(|v| json!({
            "id": v.get::<Uuid, _>("id"),
            "version": v.get::<String, _>("version"),
            "changelog": v.get::<String, _>("changelog"),
            "runtime": v.get::<String, _>("runtime"),
            "sdk_version": v.get::<String, _>("sdk_version"),
            "manifest": v.get::<Value, _>("manifest"),
            "sha256": v.get::<String, _>("sha256"),
            "size_bytes": v.get::<i32, _>("size_bytes"),
            "requested_permissions": v.get::<Value, _>("requested_permissions"),
            "approved_permissions": v.get::<Value, _>("approved_permissions"),
            "sensitive_approval_status": v.get::<String, _>("sensitive_approval_status"),
            "revoked": v.get::<bool, _>("revoked"),
            "created_at": v.get::<DateTime<Utc>, _>("created_at"),
        })).collect::<Vec<_>>(),
        "reviews": reviews.into_iter().map(|r| json!({
            "id": r.get::<Uuid, _>("id"),
            "username": r.get::<Option<String>, _>("username").unwrap_or_default(),
            "rating": r.get::<i32, _>("rating"),
            "comment": r.get::<String, _>("comment"),
            "created_at": r.get::<DateTime<Utc>, _>("created_at"),
        })).collect::<Vec<_>>(),
    })))
}

pub async fn upsert_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(package_id): Path<Uuid>,
    Json(req): Json<ReviewReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    if !(1..=5).contains(&req.rating) || req.comment.len() > 1000 {
        return Err(AppError::BadRequest("invalid_review"));
    }
    sqlx::query(
        "INSERT INTO plugin_reviews (id, package_id, user_id, rating, comment)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (package_id, user_id) DO UPDATE SET rating = EXCLUDED.rating, comment = EXCLUDED.comment, created_at = now()",
    )
    .bind(Uuid::new_v4())
    .bind(package_id)
    .bind(uid)
    .bind(req.rating)
    .bind(req.comment.trim())
    .execute(&state.db)
    .await?;

    let stats = sqlx::query(
        "SELECT COALESCE(AVG(rating)::double precision, 0) AS avg, COUNT(*)::int AS cnt FROM plugin_reviews WHERE package_id = $1",
    )
    .bind(package_id)
    .fetch_one(&state.db)
    .await?;
    let avg: f64 = stats.get("avg");
    let cnt: i32 = stats.get("cnt");
    sqlx::query("UPDATE plugin_packages SET rating_avg = $1, rating_count = $2 WHERE id = $3")
        .bind(avg)
        .bind(cnt)
        .bind(package_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "rating_avg": avg, "rating_count": cnt })))
}

pub async fn report_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(package_id): Path<Uuid>,
    Json(req): Json<ReportReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    if req.reason.trim().is_empty() || req.reason.len() > 120 || req.details.len() > 2000 {
        return Err(AppError::BadRequest("invalid_report"));
    }
    let rid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO plugin_reports (id, package_id, reporter_id, reason, details) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(rid)
    .bind(package_id)
    .bind(uid)
    .bind(req.reason.trim())
    .bind(req.details.trim())
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "report_id": rid, "status": "open" })))
}

/// Idempotently seeds official verified ChatW built-in catalog packages on server startup.
pub async fn seed_official_catalog(db: &PgPool) {
    let pub_id: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM plugin_publishers WHERE slug = 'chatw'")
        .fetch_optional(db)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            let id = Uuid::new_v4();
            let _ = sqlx::query(
                "INSERT INTO plugin_publishers (id, user_id, slug, display_name, website_url, bio, verification_status, verified_at)
                 VALUES ($1, NULL, 'chatw', 'ChatW Official', 'https://github.com/kiyogo/chart', 'Nhóm phát triển chính thức nền tảng ChatW', 'verified', now())
                 ON CONFLICT (slug) DO NOTHING",
            )
            .bind(id)
            .execute(db)
            .await;
            match sqlx::query_scalar::<_, Uuid>("SELECT id FROM plugin_publishers WHERE slug = 'chatw'")
                .fetch_optional(db)
                .await
            {
                Ok(Some(existing)) => existing,
                _ => return,
            }
        }
        Err(_) => return,
    };

    // Official sample real JS runtime plugin showing full third-party capabilities directly in Marketplace
    let sample_key = "chatw.focus_board";
    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM plugin_packages WHERE plugin_key = $1")
        .bind(sample_key)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    if exists.is_some() {
        return;
    }

    let manifest = PluginManifest {
        plugin_key: sample_key.into(),
        name: "Task Board & Quick Notes".into(),
        version: "1.0.0".into(),
        publisher: "chatw".into(),
        entry_point: "index.js".into(),
        runtime: "js".into(),
        sdk_version: "1.0.0".into(),
        platforms: vec!["web".into(), "linux".into(), "windows".into(), "macos".into()],
        permissions: vec![
            "ui.page".into(),
            "ui.sidebar".into(),
            "ui.message_action".into(),
            "ui.composer".into(),
            "commands.register".into(),
            "storage.local".into(),
            "storage.sync".into(),
            "events.lifecycle".into(),
        ],
        ui_contributions: json!({
            "messageActions": [{ "id": "save_task", "label": "Save as work", "icon": "check" }],
            "composerButtons": [{ "id": "insert_checklist", "label": "New job", "content": "/task " }]
        }),
        commands: vec![json!({ "trigger": "/task", "description": "Quickly add tasks to the board" })],
        event_subscriptions: vec!["onEnable".into(), "onMessageAction".into(), "onCommand".into()],
        ..Default::default()
    };

    let bundle_code = r#"
chatw.onCommand('/task', async (ctx) => {
  const title = (ctx.args || '').trim();
  if (!title) return { text: 'Nhập nội dung: /task Hoàn thành báo cáo' };
  const tasks = (await chatw.storage.get('tasks')) || [];
  tasks.unshift({ id: Date.now().toString(36), title, done: false, createdAt: new Date().toISOString() });
  await chatw.storage.set('tasks', tasks.slice(0, 100));
  chatw.ui.notify('Đã thêm nhiệm vụ: ' + title);
  return { text: 'Đã lưu nhiệm vụ: ' + title };
});
chatw.onMessageAction('save_task', async (msg) => {
  const title = String(msg.text || '').trim().slice(0, 140);
  if (!title) return;
  const tasks = (await chatw.storage.get('tasks')) || [];
  tasks.unshift({ id: Date.now().toString(36), title, done: false, from: msg.senderName || '', createdAt: new Date().toISOString() });
  await chatw.storage.set('tasks', tasks.slice(0, 100));
  chatw.ui.notify('Đã lưu tin nhắn thành nhiệm vụ');
});
"#;

    let ui_html = r#"<!doctype html>
<html><head><meta charset="utf-8"><style>
body{margin:0;padding:16px;font-family:var(--cw-font,system-ui,sans-serif);color:var(--cw-text,#0f172a);background:transparent}
h2{margin:0 0 12px;font-size:16px}
.row{display:flex;gap:8px;margin-bottom:12px}
input{flex:1;padding:8px 10px;border-radius:8px;border:none;background:var(--cw-input,#f1f5f9);color:inherit;font:inherit}
button{padding:8px 12px;border-radius:8px;border:none;background:var(--cw-accent,#2563eb);color:#fff;font-weight:600;cursor:pointer}
.list{display:flex;flex-direction:column;gap:6px}
.item{display:flex;align-items:center;gap:8px;padding:8px 10px;border-radius:10px;background:var(--cw-surface,#f8fafc)}
.item.done span{text-decoration:line-through;opacity:.55}
.del{margin-left:auto;background:transparent;color:#ef4444;padding:4px 8px}
</style></head><body>
<h2>Bảng Nhiệm Vụ (JS Sandbox)</h2>
<div class="row"><input id="inp" placeholder="Thêm việc mới"/><button id="add">Thêm</button></div>
<div id="list" class="list"></div>
<script>
let tasks = [];
async function load(){ tasks = (await chatw.storage.get('tasks')) || []; render(); }
function render(){
  const el = document.getElementById('list'); el.innerHTML = '';
  tasks.forEach((t, i) => {
    const d = document.createElement('div'); d.className = 'item' + (t.done ? ' done' : '');
    const cb = document.createElement('input'); cb.type = 'checkbox'; cb.checked = !!t.done;
    cb.onchange = async () => { tasks[i].done = cb.checked; await chatw.storage.set('tasks', tasks); render(); };
    const sp = document.createElement('span'); sp.textContent = t.title;
    const dl = document.createElement('button'); dl.className = 'del'; dl.textContent = 'Xóa';
    dl.onclick = async () => { tasks.splice(i, 1); await chatw.storage.set('tasks', tasks); render(); };
    d.append(cb, sp, dl); el.append(d);
  });
}
document.getElementById('add').onclick = async () => {
  const inp = document.getElementById('inp'); const v = inp.value.trim(); if (!v) return;
  tasks.unshift({ id: Date.now().toString(36), title: v, done: false });
  inp.value = ''; await chatw.storage.set('tasks', tasks); render();
};
chatw.onReady(load);
</script></body></html>"#;

    let manifest_str = serde_json::to_string(&manifest).unwrap_or_default();
    let sha = compute_package_sha256(&manifest_str, bundle_code, ui_html, "");
    let pkg_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let _ = sqlx::query(
        "INSERT INTO plugin_packages (id, plugin_key, publisher_id, name, summary, description, category, icon_url, platforms, is_public, is_builtin, latest_version)
         VALUES ($1, $2, $3, $4, $5, $6, 'productivity', 'logo:notes', '[\"web\",\"linux\",\"windows\",\"macos\"]'::jsonb, true, false, '1.0.0')",
    )
    .bind(pkg_id)
    .bind(sample_key)
    .bind(pub_id)
    .bind(&manifest.name)
    .bind("JS plugin runs in sandbox: save messages as tasks, command /task and sync across devices")
    .bind("Fully illustrated standalone HTML/CSS/JS interface, message action buttons, keyboard shortcuts, and synchronous storage via ChatW Plugin SDK.")
    .execute(db)
    .await;

    let _ = sqlx::query(
        "INSERT INTO plugin_versions (id, package_id, version, changelog, runtime, sdk_version, manifest, bundle_code, wasm_base64, ui_html, sha256, size_bytes, requested_permissions, sensitive_approval_status, approved_permissions)
         VALUES ($1, $2, '1.0.0', 'Phiên bản đầu tiên trên Plugin Runtime v2', 'js', '1.0.0', $3, $4, '', $5, $6, $7, $8, 'not_required', '[]'::jsonb)",
    )
    .bind(ver_id)
    .bind(pkg_id)
    .bind(serde_json::to_value(&manifest).unwrap_or_else(|_| json!({})))
    .bind(bundle_code)
    .bind(ui_html)
    .bind(&sha)
    .bind((bundle_code.len() + ui_html.len()) as i32)
    .bind(serde_json::to_value(&manifest.permissions).unwrap_or_else(|_| json!([])))
    .execute(db)
    .await;
}
