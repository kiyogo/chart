//! ChatW Plugin Platform Frontend API Client & Sandboxed Host Runtime Bridge.
//! Works identically across Web (WASM) and Desktop (WebKitGTK / WebView2 / WKWebView).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

async fn req_json<T: serde::de::DeserializeOwned>(
    method: &str,
    path: &str,
    payload: Option<Value>,
) -> Result<T, ApiError> {
    let body = payload.map(|v| v.to_string());
    let (_, text) = platform::http_request(method, path, body, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

async fn req_ok(method: &str, path: &str, payload: Option<Value>) -> Result<Value, ApiError> {
    let body = payload.map(|v| v.to_string());
    let (_, text) = platform::http_request(method, path, body, None).await?;
    if text.trim().is_empty() {
        return Ok(json!({ "ok": true }));
    }
    Ok(serde_json::from_str(&text).unwrap_or_else(|_| json!({ "ok": true, "raw": text })))
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PermissionCatalogItem {
    pub id: String,
    pub label: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub e2ee: bool,
    #[serde(default)]
    pub device_only_default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PublisherInfo {
    pub id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    pub slug: String,
    pub display_name: String,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MarketplaceItem {
    pub id: String,
    pub plugin_key: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub latest_version: String,
    #[serde(default)]
    pub install_count: i64,
    #[serde(default)]
    pub rating_avg: f64,
    #[serde(default)]
    pub rating_count: i64,
    #[serde(default)]
    pub is_builtin: bool,
    #[serde(default)]
    pub publisher_slug: String,
    #[serde(default)]
    pub publisher_name: String,
    #[serde(default)]
    pub publisher_verified: bool,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub requested_permissions: Vec<String>,
    #[serde(default)]
    pub sensitive_approval_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InstallGrantItem {
    pub id: String,
    pub permission: String,
    #[serde(default)]
    pub scope_type: String,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub device_only: bool,
    #[serde(default)]
    pub granted_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InstallationItem {
    pub id: String,
    pub package_id: String,
    pub version_id: String,
    pub plugin_key: String,
    pub name: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub publisher_slug: String,
    #[serde(default)]
    pub publisher_verified: bool,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub scope_type: String,
    #[serde(default)]
    pub scope_id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub ui_slot: String,
    #[serde(default)]
    pub settings: Value,
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    #[serde(default)]
    pub blocked_channels: Vec<String>,
    #[serde(default)]
    pub manifest: Value,
    #[serde(default)]
    pub bundle_code: String,
    #[serde(default)]
    pub ui_html: String,
    #[serde(default)]
    pub wasm_base64: String,
    #[serde(default)]
    pub grants: Vec<InstallGrantItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PrivacyGrantRow {
    pub id: String,
    pub installation_id: String,
    pub plugin_key: String,
    pub plugin_name: String,
    #[serde(default)]
    pub publisher_name: String,
    #[serde(default)]
    pub publisher_verified: bool,
    pub permission: String,
    #[serde(default)]
    pub permission_label: String,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub e2ee: bool,
    #[serde(default)]
    pub scope_type: String,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub device_only: bool,
    #[serde(default)]
    pub granted_at: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PrivacyAuditRow {
    pub id: String,
    #[serde(default)]
    pub installation_id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PrivacyDashboardData {
    #[serde(default)]
    pub grants: Vec<PrivacyGrantRow>,
    #[serde(default)]
    pub recent_activity: Vec<PrivacyAuditRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct WebhookItem {
    pub id: String,
    pub direction: String,
    pub scope_type: String,
    pub scope_id: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub token_hint: String,
    #[serde(default)]
    pub target_url: Option<String>,
    #[serde(default)]
    pub subscribed_events: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rate_limit_per_min: i32,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct WebhookDeliveryItem {
    pub id: String,
    pub event_type: String,
    pub event_id: String,
    pub status: String,
    #[serde(default)]
    pub http_status: Option<i32>,
    #[serde(default)]
    pub attempt_count: i32,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<i32>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DynamicMessageAction {
    #[serde(rename = "installationId")]
    pub installation_id: String,
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DynamicComposerButton {
    #[serde(rename = "installationId")]
    pub installation_id: String,
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub content: String,
}

// ============================================================================
// HTTP API Wrappers
// ============================================================================

pub async fn get_permissions_catalog() -> Result<Vec<PermissionCatalogItem>, ApiError> {
    let val: Value = req_json("GET", "/api/plugins/permissions", None).await?;
    Ok(serde_json::from_value(val.get("permissions").cloned().unwrap_or(json!([]))).unwrap_or_default())
}

pub async fn list_marketplace(query: &str, category: &str, verified_only: bool) -> Result<Vec<MarketplaceItem>, ApiError> {
    let mut parts = Vec::new();
    if !query.trim().is_empty() {
        parts.push(format!("q={}", urlencoding::encode(query.trim())));
    }
    if !category.trim().is_empty() && category != "all" {
        parts.push(format!("category={}", urlencoding::encode(category.trim())));
    }
    if verified_only {
        parts.push("verified=true".into());
    }
    let path = if parts.is_empty() {
        "/api/plugins/marketplace".to_string()
    } else {
        format!("/api/plugins/marketplace?{}", parts.join("&"))
    };
    req_json("GET", &path, None).await
}

pub async fn preview_install(package_id: &str) -> Result<Value, ApiError> {
    req_json("POST", "/api/plugins/install/preview", Some(json!({ "package_id": package_id }))).await
}

pub async fn list_installations(scope_type: Option<&str>, scope_id: Option<&str>) -> Result<Vec<InstallationItem>, ApiError> {
    let path = match (scope_type, scope_id) {
        (Some(st), Some(sid)) => format!(
            "/api/plugins/installations?scope_type={}&scope_id={}",
            urlencoding::encode(st),
            urlencoding::encode(sid)
        ),
        _ => "/api/plugins/installations".to_string(),
    };
    req_json("GET", &path, None).await
}

pub async fn create_installation(payload: Value) -> Result<Value, ApiError> {
    req_json("POST", "/api/plugins/installations", Some(payload)).await
}

pub async fn update_installation(id: &str, payload: Value) -> Result<Value, ApiError> {
    req_json("PATCH", &format!("/api/plugins/installations/{id}"), Some(payload)).await
}

pub async fn delete_installation(id: &str) -> Result<Value, ApiError> {
    req_ok("DELETE", &format!("/api/plugins/installations/{id}"), None).await
}

pub async fn revoke_grant(installation_id: &str, grant_id: &str) -> Result<Value, ApiError> {
    req_ok("DELETE", &format!("/api/plugins/installations/{installation_id}/grants/{grant_id}"), None).await
}

pub async fn get_privacy_dashboard() -> Result<PrivacyDashboardData, ApiError> {
    req_json("GET", "/api/plugins/privacy", None).await
}

pub async fn get_my_publisher() -> Result<Option<PublisherInfo>, ApiError> {
    req_json("GET", "/api/plugins/publishers/me", None).await
}

pub async fn register_publisher(slug: &str, display_name: &str, website_url: &str, bio: &str) -> Result<PublisherInfo, ApiError> {
    req_json(
        "POST",
        "/api/plugins/publishers",
        Some(json!({
            "slug": slug,
            "display_name": display_name,
            "website_url": if website_url.trim().is_empty() { Value::Null } else { json!(website_url.trim()) },
            "bio": if bio.trim().is_empty() { Value::Null } else { json!(bio.trim()) },
        })),
    )
    .await
}

pub async fn request_publisher_verification() -> Result<Value, ApiError> {
    req_ok("POST", "/api/plugins/publishers/verify", None).await
}

pub async fn publish_package(payload: Value) -> Result<Value, ApiError> {
    req_json("POST", "/api/plugins/packages", Some(payload)).await
}

pub async fn submit_review(package_id: &str, rating: i32, comment: &str) -> Result<Value, ApiError> {
    req_ok("POST", &format!("/api/plugins/marketplace/{package_id}/reviews"), Some(json!({ "rating": rating, "comment": comment }))).await
}

pub async fn report_package(package_id: &str, reason: &str, details: &str) -> Result<Value, ApiError> {
    req_ok("POST", &format!("/api/plugins/marketplace/{package_id}/report"), Some(json!({ "reason": reason, "details": details }))).await
}

pub async fn list_webhooks(scope_type: Option<&str>, scope_id: Option<&str>) -> Result<Vec<WebhookItem>, ApiError> {
    let path = match (scope_type, scope_id) {
        (Some(st), Some(sid)) => format!(
            "/api/webhooks?scope_type={}&scope_id={}",
            urlencoding::encode(st),
            urlencoding::encode(sid)
        ),
        _ => "/api/webhooks".to_string(),
    };
    req_json("GET", &path, None).await
}

pub async fn create_webhook(payload: Value) -> Result<Value, ApiError> {
    req_json("POST", "/api/webhooks", Some(payload)).await
}

pub async fn update_webhook(id: &str, payload: Value) -> Result<Value, ApiError> {
    req_ok("PATCH", &format!("/api/webhooks/{id}"), Some(payload)).await
}

pub async fn delete_webhook(id: &str) -> Result<Value, ApiError> {
    req_ok("DELETE", &format!("/api/webhooks/{id}"), None).await
}

pub async fn rotate_webhook_token(id: &str) -> Result<Value, ApiError> {
    req_json("POST", &format!("/api/webhooks/{id}/rotate-token"), None).await
}

pub async fn test_webhook(id: &str) -> Result<Value, ApiError> {
    req_json("POST", &format!("/api/webhooks/{id}/test"), None).await
}

pub async fn list_webhook_deliveries(id: &str) -> Result<Vec<WebhookDeliveryItem>, ApiError> {
    req_json("GET", &format!("/api/webhooks/{id}/deliveries"), None).await
}

// ============================================================================
// Sandboxed Host Runtime (`window.ChatwPluginHost`) JS Bridge
// ============================================================================

pub fn host_set_account(id: &str, username: &str, display_name: &str) {
    let payload = json!({ "id": id, "username": username, "displayName": display_name }).to_string();
    let script = format!(
        "if (window.ChatwPluginHost) window.ChatwPluginHost.setAccount({payload});"
    );
    let _ = dioxus::document::eval(&script);
}

pub fn host_sync_installations(installations: &[InstallationItem]) {
    let payload = serde_json::to_string(installations).unwrap_or_else(|_| "[]".into());
    let script = format!(
        "if (window.ChatwPluginHost) window.ChatwPluginHost.syncInstallations({payload});"
    );
    let _ = dioxus::document::eval(&script);
}

pub fn host_mount_ui(selector: &str, installation_id: &str, _slot: &str) {
    let sel = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into());
    let id = serde_json::to_string(installation_id).unwrap_or_else(|_| "\"\"".into());
    let script = format!(
        "setTimeout(function() {{ if (window.ChatwPluginHost) window.ChatwPluginHost.mountUiBySelector({id}, {sel}); }}, 60);"
    );
    let _ = dioxus::document::eval(&script);
}

pub fn host_revoke_grant_local(installation_id: &str, grant_id: &str) {
    let iid = serde_json::to_string(installation_id).unwrap_or_else(|_| "\"\"".into());
    let gid = serde_json::to_string(grant_id).unwrap_or_else(|_| "\"\"".into());
    let script = format!(
        "if (window.ChatwPluginHost) window.ChatwPluginHost.revokeGrantImmediately({iid}, {gid});"
    );
    let _ = dioxus::document::eval(&script);
}

pub fn host_emit_event(event_name: &str, payload: Value) {
    let ev = serde_json::to_string(event_name).unwrap_or_else(|_| "\"\"".into());
    let data = payload.to_string();
    let script = format!(
        "if (window.ChatwPluginHost) window.ChatwPluginHost.emitEvent({ev}, {data});"
    );
    let _ = dioxus::document::eval(&script);
}

pub async fn host_run_command(trigger: &str, args: &str, context: Value) -> Option<String> {
    let tr = serde_json::to_string(trigger).unwrap_or_else(|_| "\"\"".into());
    let ar = serde_json::to_string(args).unwrap_or_else(|_| "\"\"".into());
    let ctx = context.to_string();
    let script = format!(
        "if (!window.ChatwPluginHost) return null;\\\nconst r = await window.ChatwPluginHost.runCommand({tr}, {ar}, {ctx});\\\nif (!r) return null;\\\nif (typeof r === 'string') return r;\\\nif (r.text) return String(r.text);\\\nif (r.error) return 'Plugin error: ' + String(r.error);\\\nreturn JSON.stringify(r);"
    );
    let eval = dioxus::document::eval(&script);
    match eval.await {
        Ok(Value::String(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

pub async fn host_get_message_actions(channel_id: Option<&str>) -> Vec<DynamicMessageAction> {
    let cid = channel_id
        .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "null".into()))
        .unwrap_or_else(|| "null".into());
    let script = format!(
        "return window.ChatwPluginHost ? window.ChatwPluginHost.getMessageActions({cid}) : [];"
    );
    match dioxus::document::eval(&script).await {
        Ok(val) => serde_json::from_value(val).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn host_trigger_message_action(installation_id: &str, action_id: &str, message: Value) -> Option<Value> {
    let iid = serde_json::to_string(installation_id).unwrap_or_else(|_| "\"\"".into());
    let aid = serde_json::to_string(action_id).unwrap_or_else(|_| "\"\"".into());
    let msg = message.to_string();
    let script = format!(
        "if (!window.ChatwPluginHost) return null;\
         return await window.ChatwPluginHost.triggerMessageAction({iid}, {aid}, {msg});"
    );
    dioxus::document::eval(&script).await.ok()
}

pub async fn host_get_composer_buttons(channel_id: Option<&str>) -> Vec<DynamicComposerButton> {
    let cid = channel_id
        .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "null".into()))
        .unwrap_or_else(|| "null".into());
    let script = format!(
        "return window.ChatwPluginHost ? window.ChatwPluginHost.getComposerButtons({cid}) : [];"
    );
    match dioxus::document::eval(&script).await {
        Ok(val) => serde_json::from_value(val).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
