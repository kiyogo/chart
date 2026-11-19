//! Spec 011 client API: two-step verification, security log and several
//! signed-in accounts on one device.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

fn bad() -> ApiError {
    ApiError { code: "invalid_json".into(), status: 200 }
}

async fn call(method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| bad())
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Status {
    pub totp_enabled: bool,
    #[serde(default)]
    pub backup_codes_left: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Event {
    pub kind: String,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Account {
    pub user_id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub unread: i64,
    pub active: bool,
}

impl Account {
    pub fn name(&self) -> String {
        self.display_name.clone().filter(|n| !n.trim().is_empty()).or(self.username.clone()).unwrap_or_else(|| "Tài khoản".into())
    }
}

pub async fn status() -> Result<Status, ApiError> {
    serde_json::from_value(call("GET", "/api/security/status", None).await?).map_err(|_| bad())
}

pub async fn events() -> Result<Vec<Event>, ApiError> {
    serde_json::from_value(call("GET", "/api/security/events", None).await?).map_err(|_| bad())
}

/// (base32 secret, otpauth URI)
pub async fn totp_setup() -> Result<(String, String), ApiError> {
    let v = call("POST", "/api/security/totp/setup", Some(json!({}))).await?;
    Ok((
        v.get("secret").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        v.get("otpauth_uri").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
    ))
}

fn codes_of(v: &Value) -> Vec<String> {
    v.get("backup_codes").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

pub async fn totp_enable(code: &str) -> Result<Vec<String>, ApiError> {
    Ok(codes_of(&call("POST", "/api/security/totp/enable", Some(json!({ "code": code }))).await?))
}

pub async fn totp_disable(code: &str) -> Result<(), ApiError> {
    call("POST", "/api/security/totp/disable", Some(json!({ "code": code }))).await.map(|_| ())
}

pub async fn regenerate_codes(code: &str) -> Result<Vec<String>, ApiError> {
    Ok(codes_of(&call("POST", "/api/security/backup-codes/regenerate", Some(json!({ "code": code }))).await?))
}

pub async fn twofa_pending() -> bool {
    call("GET", "/api/auth/2fa/pending", None).await.ok().and_then(|v| v.get("pending").and_then(|p| p.as_bool())).unwrap_or(false)
}

pub async fn twofa_verify(code: &str) -> Result<(), ApiError> {
    call("POST", "/api/auth/2fa/verify", Some(json!({ "code": code }))).await.map(|_| ())
}

pub async fn accounts() -> Result<Vec<Account>, ApiError> {
    serde_json::from_value(call("GET", "/api/auth/accounts", None).await?).map_err(|_| bad())
}

pub async fn add_account() -> Result<(), ApiError> {
    call("POST", "/api/auth/accounts/add", Some(json!({}))).await.map(|_| ())
}

pub async fn switch_account(user_id: &str) -> Result<(), ApiError> {
    call("POST", "/api/auth/accounts/switch", Some(json!({ "user_id": user_id }))).await.map(|_| ())
}

pub async fn logout_account(user_id: &str) -> Result<(), ApiError> {
    call("POST", &format!("/api/auth/accounts/{}/logout", urlencoding::encode(user_id)), Some(json!({}))).await.map(|_| ())
}

pub async fn logout_all() -> Result<(), ApiError> {
    call("POST", "/api/auth/logout-all", Some(json!({}))).await.map(|_| ())
}

pub async fn revoke_other_sessions(code: Option<&str>) -> Result<(), ApiError> {
    let path = match code {
        Some(c) => format!("/api/auth/sessions?code={}", urlencoding::encode(c)),
        None => "/api/auth/sessions".to_string(),
    };
    call("DELETE", &path, None).await.map(|_| ())
}

/// Start over with the newly active account (keys, preferences and caches are per account).
pub fn reload_app() {
    let _ = dioxus::document::eval("window.location.href = '/';");
}

/// Vietnamese label for a security event.
pub fn event_label(kind: &str) -> &'static str {
    match kind {
        "login" => "Đăng nhập",
        "login_2fa" => "Đăng nhập có xác minh hai bước",
        "logout" => "Đăng xuất",
        "new_device" => "Đăng nhập từ thiết bị mới",
        "failed_2fa" => "Nhập sai mã xác minh",
        "backup_code_used" => "Dùng một mã dự phòng",
        "totp_enabled" => "Bật xác minh hai bước",
        "totp_disabled" => "Tắt xác minh hai bước",
        "backup_codes_regenerated" => "Tạo lại mã dự phòng",
        "sessions_revoked_others" => "Đăng xuất mọi thiết bị khác",
        "account_added" => "Thêm tài khoản khác trên thiết bị",
        "account_switched" => "Chuyển sang tài khoản này",
        _ => "Hoạt động bảo mật",
    }
}

pub fn error_text(code: &str) -> String {
    match code {
        "invalid_code" => "Mã không đúng hoặc đã dùng".into(),
        "challenge_expired" | "no_challenge" => "Phiên xác minh đã hết hạn, hãy đăng nhập lại".into(),
        "rate_limited" => "Nhập sai quá nhiều lần, hãy thử lại sau 15 phút".into(),
        "second_factor_required" => "Cần mã xác minh hai bước".into(),
        "account_limit_reached" => "Tối đa 5 tài khoản trên một thiết bị".into(),
        "account_not_signed_in" => "Tài khoản này đã đăng xuất".into(),
        c => format!("Không thực hiện được ({c})"),
    }
}
