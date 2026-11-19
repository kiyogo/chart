//! Spec 013 client API: community profiles, emoji / stickers, events, forum
//! threads, audit log, welcome screen and rules.

use dioxus::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Expression {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub file_url: String,
    #[serde(default)]
    pub is_animated: bool,
}

/// Emoji and stickers of the community whose channel is open: (server id, items).
pub static ACTIVE_EXPRESSIONS: GlobalSignal<Option<(String, Vec<Expression>)>> = Signal::global(|| None);

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub starts_at: String,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub going: i64,
    #[serde(default)]
    pub interested: i64,
    #[serde(default)]
    pub mine: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub pinned: bool,
    pub locked: bool,
    #[serde(default)]
    pub author: Option<String>,
    pub replies: i64,
    pub last_activity: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Post {
    pub id: String,
    pub sender_id: String,
    #[serde(default)]
    pub sender_name: Option<String>,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WelcomeChannel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Welcome {
    pub name: String,
    #[serde(default)]
    pub welcome_message: Option<String>,
    #[serde(default)]
    pub rules: Option<String>,
    #[serde(default)]
    pub require_rules: bool,
    #[serde(default)]
    pub accepted_at: Option<String>,
    #[serde(default)]
    pub channels: Vec<WelcomeChannel>,
}

impl Welcome {
    pub fn has_content(&self) -> bool {
        self.welcome_message.as_ref().is_some_and(|w| !w.trim().is_empty()) || self.rules.as_ref().is_some_and(|r| !r.trim().is_empty())
    }
    pub fn must_accept(&self) -> bool {
        self.require_rules && self.accepted_at.is_none()
    }
}

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

fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

/// (nickname, avatar_url, bio)
pub async fn my_profile(server: &str) -> Result<(String, Option<String>, String), ApiError> {
    let v = call("GET", &format!("/api/servers/{}/me/profile", enc(server)), None).await?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
    Ok((s("nickname").unwrap_or_default(), s("avatar_url"), s("bio").unwrap_or_default()))
}

pub async fn save_my_profile(server: &str, nickname: &str, avatar_url: Option<&str>, bio: &str) -> Result<(), ApiError> {
    call("PUT", &format!("/api/servers/{}/me/profile", enc(server)), Some(json!({ "nickname": nickname, "avatar_url": avatar_url, "bio": bio }))).await.map(|_| ())
}

pub async fn expressions(server: &str) -> Result<Vec<Expression>, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/servers/{}/expressions", enc(server)), None).await?).map_err(|_| bad())
}

pub async fn add_expression(server: &str, kind: &str, name: &str, file_url: &str, animated: bool) -> Result<(), ApiError> {
    call("POST", &format!("/api/servers/{}/expressions", enc(server)), Some(json!({ "kind": kind, "name": name, "file_url": file_url, "is_animated": animated }))).await.map(|_| ())
}

pub async fn delete_expression(server: &str, id: &str) -> Result<(), ApiError> {
    call("DELETE", &format!("/api/servers/{}/expressions/{}", enc(server), enc(id)), None).await.map(|_| ())
}

/// Load the open community's emoji / stickers for the pickers.
pub async fn activate(server: &str) {
    let list = expressions(server).await.unwrap_or_default();
    *ACTIVE_EXPRESSIONS.write() = Some((server.to_string(), list));
}

pub async fn events(server: &str) -> Result<Vec<Event>, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/servers/{}/events", enc(server)), None).await?).map_err(|_| bad())
}

pub async fn create_event(server: &str, title: &str, description: &str, location: &str, starts_at: &str, ends_at: Option<&str>) -> Result<(), ApiError> {
    call(
        "POST",
        &format!("/api/servers/{}/events", enc(server)),
        Some(json!({ "title": title, "description": description, "location": location, "starts_at": starts_at, "ends_at": ends_at })),
    )
    .await
    .map(|_| ())
}

pub async fn delete_event(server: &str, id: &str) -> Result<(), ApiError> {
    call("DELETE", &format!("/api/servers/{}/events/{}", enc(server), enc(id)), None).await.map(|_| ())
}

pub async fn rsvp(server: &str, id: &str, status: &str) -> Result<(), ApiError> {
    call("PUT", &format!("/api/servers/{}/events/{}/rsvp", enc(server), enc(id)), Some(json!({ "status": status }))).await.map(|_| ())
}

pub async fn audit_log(server: &str) -> Result<Vec<AuditEntry>, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/servers/{}/audit-log", enc(server)), None).await?).map_err(|_| bad())
}

pub async fn welcome(server: &str) -> Result<Welcome, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/servers/{}/welcome", enc(server)), None).await?).map_err(|_| bad())
}

pub async fn save_welcome(server: &str, message: &str, rules: &str, require: bool) -> Result<(), ApiError> {
    call("PUT", &format!("/api/servers/{}/welcome", enc(server)), Some(json!({ "welcome_message": message, "rules": rules, "require_rules": require }))).await.map(|_| ())
}

pub async fn accept_rules(server: &str) -> Result<(), ApiError> {
    call("POST", &format!("/api/servers/{}/accept-rules", enc(server)), Some(json!({}))).await.map(|_| ())
}

pub async fn threads(channel: &str) -> Result<Vec<Thread>, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/channels/{}/threads", enc(channel)), None).await?).map_err(|_| bad())
}

pub async fn create_thread(channel: &str, title: &str, body: &str) -> Result<String, ApiError> {
    let v = call("POST", &format!("/api/channels/{}/threads", enc(channel)), Some(json!({ "title": title, "body": body }))).await?;
    v.get("id").and_then(|x| x.as_str()).map(str::to_string).ok_or_else(bad)
}

pub async fn update_thread(channel: &str, id: &str, pinned: Option<bool>, locked: Option<bool>) -> Result<(), ApiError> {
    call("PUT", &format!("/api/channels/{}/threads/{}", enc(channel), enc(id)), Some(json!({ "pinned": pinned, "locked": locked }))).await.map(|_| ())
}

pub async fn posts(channel: &str, thread: &str) -> Result<Vec<Post>, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/channels/{}/threads/{}/messages", enc(channel), enc(thread)), None).await?).map_err(|_| bad())
}

pub async fn reply(channel: &str, thread: &str, content: &str) -> Result<(), ApiError> {
    call("POST", &format!("/api/channels/{}/threads/{}/messages", enc(channel), enc(thread)), Some(json!({ "content": content }))).await.map(|_| ())
}

pub fn error_text(code: &str) -> String {
    match code {
        "rules_not_accepted" => "Hãy đọc và đồng ý nội quy của cộng đồng trước khi gửi".into(),
        "missing_permission" => "Bạn không có quyền làm việc này".into(),
        "thread_locked" => "Chủ đề đã bị khóa".into(),
        "name_taken" => "Tên này đã có trong cộng đồng".into(),
        "invalid_name" => "Tên chỉ gồm chữ thường không dấu, số và dấu _ (2–32 ký tự)".into(),
        "expression_limit_reached" => "Tối đa 100 emoji và 100 sticker".into(),
        "invalid_event_time" => "Giờ kết thúc phải sau giờ bắt đầu".into(),
        c => format!("Không thực hiện được ({c})"),
    }
}

pub fn audit_label(action: &str) -> &'static str {
    match action {
        "role_created" => "Tạo vai trò",
        "role_updated" => "Sửa vai trò",
        "role_deleted" => "Xóa vai trò",
        "role_assigned" => "Gán vai trò",
        "role_removed" => "Gỡ vai trò",
        "member_kicked" => "Mời thành viên ra",
        "channel_created" => "Tạo kênh",
        "settings_updated" => "Đổi cài đặt cộng đồng",
        "emoji_added" => "Thêm emoji",
        "emoji_deleted" => "Xóa emoji",
        "sticker_added" => "Thêm sticker",
        "sticker_deleted" => "Xóa sticker",
        "event_created" => "Tạo sự kiện",
        "event_deleted" => "Xóa sự kiện",
        "welcome_updated" => "Sửa màn hình chào mừng và nội quy",
        "thread_locked" => "Khóa chủ đề diễn đàn",
        _ => "Thay đổi",
    }
}
