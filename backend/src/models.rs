use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod permissions {
    pub const VIEW_CHANNELS: i64 = 1 << 0;
    pub const SEND_MESSAGES: i64 = 1 << 1;
    pub const MANAGE_CHANNELS: i64 = 1 << 2;
    pub const MANAGE_ROLES: i64 = 1 << 3;
    pub const MANAGE_SERVER: i64 = 1 << 4;
    pub const CREATE_INVITES: i64 = 1 << 5;
    pub const ADMINISTRATOR: i64 = 1 << 6;
    pub const DEFAULT_EVERYONE: i64 = VIEW_CHANNELS | SEND_MESSAGES | CREATE_INVITES;
    pub const ALL: i64 = (1 << 7) - 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerCategoryItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub position: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub channel_type: String,
    pub position: i32,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub category_id: Option<Uuid>,
    pub channel_type: Option<String>,
    pub position: Option<i32>,
    pub is_private: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerRoleItem {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub color: String,
    pub position: i32,
    pub permissions: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub color: Option<String>,
    pub position: Option<i32>,
    pub permissions: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMemberItem {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub roles: Vec<Uuid>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelMessageItem {
    pub id: Uuid,
    pub seq: i64,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub sender_avatar_url: Option<String>,
    pub content: String,
    pub reply_to_id: Option<Uuid>,
    pub is_pinned: bool,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostChannelMessageRequest {
    pub content: String,
    pub reply_to_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub bio: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub presence: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub presence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebRtcSignalPayload {
    pub target_user_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub signal_type: String,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceStateItem {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub self_mute: bool,
    pub self_deaf: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateVoiceStateRequest {
    pub self_mute: Option<bool>,
    pub self_deaf: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentItem {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub download_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}
