//! Client API for spec 007: privacy & blocks, labels / pins / reads / wallpapers,
//! self chat, devices, group management, invites, bots, business, directory.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{
    models::{
        ApiError, BlockedUser, BotChatMessage, BotItem, BusinessData, ChatLabel, ConvPref, DirectoryResults,
        GroupSettings, InvitePreview, JoinRequestItem, PrivacySettings, ServerSettings,
    },
    platform,
};

async fn call<T: DeserializeOwned>(method: &str, path: &str, body: Option<Value>) -> Result<T, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}
fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

// Privacy & blocks
pub async fn privacy_get() -> Result<PrivacySettings, ApiError> { call("GET", "/api/privacy/settings", None).await }
pub async fn privacy_set(fields: Value) -> Result<PrivacySettings, ApiError> { call("PUT", "/api/privacy/settings", Some(fields)).await }
pub async fn blocks_list() -> Result<Vec<BlockedUser>, ApiError> { call("GET", "/api/privacy/blocks", None).await }
pub async fn block_user(user_id: &str) -> Result<Value, ApiError> { call("POST", "/api/privacy/blocks", Some(json!({ "user_id": user_id }))).await }
pub async fn unblock_user(user_id: &str) -> Result<Value, ApiError> { call("DELETE", &format!("/api/privacy/blocks/{}", enc(user_id)), None).await }

// Labels, prefs, reads, wallpapers, self chat
pub async fn labels_list() -> Result<Vec<ChatLabel>, ApiError> { call("GET", "/api/conversations/labels", None).await }
pub async fn label_create(name: &str, color: &str) -> Result<ChatLabel, ApiError> {
    call("POST", "/api/conversations/labels", Some(json!({ "name": name, "color": color }))).await
}
pub async fn label_update(id: &str, name: &str, color: &str) -> Result<ChatLabel, ApiError> {
    call("PUT", &format!("/api/conversations/labels/{}", enc(id)), Some(json!({ "name": name, "color": color }))).await
}
pub async fn label_delete(id: &str) -> Result<Value, ApiError> { call("DELETE", &format!("/api/conversations/labels/{}", enc(id)), None).await }
pub async fn prefs_list() -> Result<Vec<ConvPref>, ApiError> { call("GET", "/api/conversations/prefs", None).await }
pub async fn pref_set(conv_type: &str, conv_id: &str, fields: Value) -> Result<Value, ApiError> {
    call("PUT", &format!("/api/conversations/prefs/{}/{}", enc(conv_type), enc(conv_id)), Some(fields)).await
}
pub async fn mark_read(partner_id: &str, seq: i64) -> Result<Value, ApiError> {
    call("POST", "/api/conversations/reads", Some(json!({ "partner_id": partner_id, "seq": seq }))).await
}
pub async fn partner_read(partner_id: &str) -> Result<Option<i64>, ApiError> {
    let v: Value = call("GET", &format!("/api/conversations/reads/{}", enc(partner_id)), None).await?;
    Ok(v.get("seq").and_then(|s| s.as_i64()))
}
pub async fn wallpaper_get(conv_type: &str, conv_id: &str) -> Result<Option<String>, ApiError> {
    let v: Value = call("GET", &format!("/api/conversations/wallpaper/{}/{}", enc(conv_type), enc(conv_id)), None).await?;
    Ok(v.get("value").and_then(|s| s.as_str()).map(str::to_string))
}
pub async fn wallpaper_set(conv_type: &str, conv_id: &str, value: &str) -> Result<Value, ApiError> {
    call("PUT", &format!("/api/conversations/wallpaper/{}/{}", enc(conv_type), enc(conv_id)), Some(json!({ "value": value }))).await
}
pub async fn self_chat_enable() -> Result<Value, ApiError> { call("POST", "/api/conversations/self-chat", None).await }
pub async fn self_chat_delete() -> Result<Value, ApiError> { call("DELETE", "/api/conversations/self-chat", None).await }
pub async fn session_rename(session_id: &str, label: &str) -> Result<Value, ApiError> {
    call("PATCH", &format!("/api/auth/sessions/{}", enc(session_id)), Some(json!({ "label": label }))).await
}

// Groups
pub async fn group_settings(group_id: &str) -> Result<GroupSettings, ApiError> { call("GET", &format!("/api/groups/{}/settings", enc(group_id)), None).await }
pub async fn group_settings_set(group_id: &str, fields: Value) -> Result<GroupSettings, ApiError> {
    call("PUT", &format!("/api/groups/{}/settings", enc(group_id)), Some(fields)).await
}
pub async fn group_join_requests(group_id: &str) -> Result<Vec<JoinRequestItem>, ApiError> {
    call("GET", &format!("/api/groups/{}/join-requests", enc(group_id)), None).await
}
pub async fn group_decide_request(group_id: &str, user_id: &str, approve: bool) -> Result<Value, ApiError> {
    let d = if approve { "approve" } else { "reject" };
    call("POST", &format!("/api/groups/{}/join-requests/{}/{d}", enc(group_id), enc(user_id)), None).await
}
pub async fn group_block(group_id: &str) -> Result<Value, ApiError> { call("POST", &format!("/api/groups/{}/block", enc(group_id)), None).await }
pub async fn group_invite_link(group_id: &str) -> Result<String, ApiError> {
    let v: Value = call("GET", &format!("/api/groups/{}/invite-link", enc(group_id)), None).await?;
    v.get("code").and_then(|c| c.as_str()).map(str::to_string).ok_or(ApiError { code: "missing_code".into(), status: 200 })
}
pub async fn group_invite_regenerate(group_id: &str) -> Result<String, ApiError> {
    let v: Value = call("POST", &format!("/api/groups/{}/invite-link/regenerate", enc(group_id)), None).await?;
    v.get("code").and_then(|c| c.as_str()).map(str::to_string).ok_or(ApiError { code: "missing_code".into(), status: 200 })
}
pub async fn invite_preview(kind: &str, code: &str) -> Result<InvitePreview, ApiError> {
    let base = if kind == "server" { "servers" } else { "groups" };
    call("GET", &format!("/api/{base}/invite-preview/{}", enc(code)), None).await
}
pub async fn server_settings(server_id: &str) -> Result<ServerSettings, ApiError> { call("GET", &format!("/api/servers/{}/settings", enc(server_id)), None).await }
pub async fn server_settings_set(server_id: &str, fields: Value) -> Result<ServerSettings, ApiError> {
    call("PUT", &format!("/api/servers/{}/settings", enc(server_id)), Some(fields)).await
}

// Bots
pub async fn bots_mine() -> Result<Vec<BotItem>, ApiError> { call("GET", "/api/bots", None).await }
pub async fn bots_chats() -> Result<Vec<BotItem>, ApiError> { call("GET", "/api/bots/chats", None).await }
pub async fn bot_create(username: &str, display_name: &str, description: &str) -> Result<BotItem, ApiError> {
    call("POST", "/api/bots", Some(json!({ "username": username, "display_name": display_name, "description": description }))).await
}
pub async fn bot_update(id: &str, fields: Value) -> Result<BotItem, ApiError> { call("PUT", &format!("/api/bots/{}", enc(id)), Some(fields)).await }
pub async fn bot_delete(id: &str) -> Result<Value, ApiError> { call("DELETE", &format!("/api/bots/{}", enc(id)), None).await }
pub async fn bot_rotate(id: &str) -> Result<BotItem, ApiError> { call("POST", &format!("/api/bots/{}/token", enc(id)), None).await }
pub async fn bot_messages(id: &str) -> Result<Vec<BotChatMessage>, ApiError> { call("GET", &format!("/api/bots/{}/messages", enc(id)), None).await }
pub async fn bot_send(id: &str, text: &str) -> Result<BotChatMessage, ApiError> {
    call("POST", &format!("/api/bots/{}/messages", enc(id)), Some(json!({ "text": text }))).await
}
pub async fn bot_by_name(username: &str) -> Result<BotItem, ApiError> { call("GET", &format!("/api/bots/by-name/{}", enc(username)), None).await }
pub async fn bot_add_to_group(bot_id: &str, group_id: &str) -> Result<Value, ApiError> {
    call("POST", &format!("/api/bots/{}/groups/{}", enc(bot_id), enc(group_id)), None).await
}

// Business
pub async fn business_get() -> Result<BusinessData, ApiError> { call("GET", "/api/business/me", None).await }
pub async fn business_save(fields: Value) -> Result<BusinessData, ApiError> { call("PUT", "/api/business/me", Some(fields)).await }
pub async fn business_delete() -> Result<Value, ApiError> { call("DELETE", "/api/business/me", None).await }
pub async fn business_product_add(fields: Value) -> Result<BusinessData, ApiError> { call("POST", "/api/business/products", Some(fields)).await }
pub async fn business_product_delete(id: &str) -> Result<BusinessData, ApiError> { call("DELETE", &format!("/api/business/products/{}", enc(id)), None).await }
pub async fn business_reply_add(shortcut: &str, text: &str) -> Result<BusinessData, ApiError> {
    call("POST", "/api/business/quick-replies", Some(json!({ "shortcut": shortcut, "text": text }))).await
}
pub async fn business_reply_delete(id: &str) -> Result<BusinessData, ApiError> { call("DELETE", &format!("/api/business/quick-replies/{}", enc(id)), None).await }

// Directory
pub async fn directory_search(q: &str) -> Result<DirectoryResults, ApiError> { call("GET", &format!("/api/directory/search?q={}", enc(q)), None).await }
pub async fn directory_join_group(id: &str) -> Result<Value, ApiError> { call("POST", &format!("/api/directory/groups/{}/join", enc(id)), None).await }
pub async fn server_join_public(id: &str) -> Result<Value, ApiError> { call("POST", &format!("/api/servers/{}/join", enc(id)), None).await }

// Community server roles (Discord style)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServerRole {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub position: i32,
    pub permissions: i64,
    #[serde(default)]
    pub enterprise_permissions: i64,
}
pub async fn server_roles(server_id: &str) -> Result<Vec<ServerRole>, ApiError> { call("GET", &format!("/api/servers/{}/roles", enc(server_id)), None).await }
pub async fn server_role_create(server_id: &str, name: &str, color: &str, permissions: i64) -> Result<ServerRole, ApiError> {
    call("POST", &format!("/api/servers/{}/roles", enc(server_id)), Some(json!({ "name": name, "color": color, "permissions": permissions }))).await
}
pub async fn server_role_update(server_id: &str, role_id: &str, fields: Value) -> Result<ServerRole, ApiError> {
    call("PUT", &format!("/api/servers/{}/roles/{}", enc(server_id), enc(role_id)), Some(fields)).await
}
pub async fn server_role_delete(server_id: &str, role_id: &str) -> Result<Value, ApiError> {
    call("DELETE", &format!("/api/servers/{}/roles/{}", enc(server_id), enc(role_id)), None).await
}
pub async fn server_member_role(server_id: &str, user_id: &str, role_id: &str, on: bool) -> Result<Value, ApiError> {
    let m = if on { "PUT" } else { "DELETE" };
    call(m, &format!("/api/servers/{}/members/{}/roles/{}", enc(server_id), enc(user_id), enc(role_id)), None).await
}
pub async fn server_kick(server_id: &str, user_id: &str) -> Result<Value, ApiError> {
    call("DELETE", &format!("/api/servers/{}/members/{}", enc(server_id), enc(user_id)), None).await
}
pub async fn server_my_permissions(server_id: &str) -> Result<(i64, bool), ApiError> {
    let v: Value = call("GET", &format!("/api/servers/{}/my-permissions", enc(server_id)), None).await?;
    Ok((v.get("permissions").and_then(|p| p.as_i64()).unwrap_or(0), v.get("is_owner").and_then(|p| p.as_bool()).unwrap_or(false)))
}

pub async fn server_my_permissions_all(server_id: &str) -> Result<(i64, i64, bool), ApiError> {
    let v: Value = call("GET", &format!("/api/servers/{}/my-permissions", enc(server_id)), None).await?;
    Ok((
        v.get("permissions").and_then(|p| p.as_i64()).unwrap_or(0),
        v.get("enterprise_permissions").and_then(|p| p.as_i64()).unwrap_or(0),
        v.get("is_owner").and_then(|p| p.as_bool()).unwrap_or(false),
    ))
}
