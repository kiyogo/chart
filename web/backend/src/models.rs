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
    pub const KICK_MEMBERS: i64 = 1 << 7;
    pub const MENTION_EVERYONE: i64 = 1 << 8;
    pub const CONNECT_VOICE: i64 = 1 << 9;
    /// Spec 013: add / remove community emoji and stickers.
    pub const MANAGE_EXPRESSIONS: i64 = 1 << 10;
    pub const DEFAULT_EVERYONE: i64 = VIEW_CHANNELS | SEND_MESSAGES | CREATE_INVITES | CONNECT_VOICE;
    pub const ALL: i64 = (1 << 11) - 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub is_enterprise: bool,
    /// Filled by the list endpoint; omitted elsewhere (additive, backwards compatible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_owner: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_enterprise: Option<bool>,
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
    #[serde(default)]
    pub enterprise_permissions: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub color: Option<String>,
    pub position: Option<i32>,
    pub permissions: Option<i64>,
    pub enterprise_permissions: Option<i64>,
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
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub presence: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub status_emoji: Option<String>,
    #[serde(default)]
    pub nameplate: Option<String>,
    #[serde(default)]
    pub custom_nameplate_url: Option<String>,
    #[serde(default)]
    pub name_effect: Option<String>,
    #[serde(default)]
    pub avatar_decoration: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessageSearchHit {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub channel_name: String,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub sender_avatar_url: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
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
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub presence: Option<String>,
    pub custom_banner_url: Option<String>,
    /// Ignored: badges are awarded by the server, never set by the client.
    #[serde(default)]
    pub badges: Option<Vec<String>>,
    pub social_links: Option<serde_json::Value>,
    #[serde(default)]
    pub profile_color: Option<String>,
    #[serde(default)]
    pub profile_accent: Option<String>,
    #[serde(default)]
    pub avatar_decoration: Option<String>,
    #[serde(default)]
    pub profile_effect: Option<String>,
    #[serde(default)]
    pub avatar_effect: Option<String>,
    #[serde(default)]
    pub name_effect: Option<String>,
    #[serde(default)]
    pub nameplate: Option<String>,
    #[serde(default)]
    pub custom_decoration_url: Option<String>,
    #[serde(default)]
    pub custom_nameplate_url: Option<String>,
    #[serde(default)]
    pub custom_effect_url: Option<String>,
    #[serde(default)]
    pub custom_effect_anim: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub custom_banner_url: Option<String>,
    pub bio: Option<String>,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub presence: String,
    #[serde(default)]
    pub badges: Vec<String>,
    #[serde(default)]
    pub social_links: serde_json::Value,
    pub profile_color: Option<String>,
    pub profile_accent: Option<String>,
    pub avatar_decoration: String,
    pub profile_effect: String,
    pub member_since: DateTime<Utc>,
    pub avatar_effect: String,
    pub name_effect: String,
    pub nameplate: String,
    pub custom_decoration_url: Option<String>,
    pub custom_nameplate_url: Option<String>,
    pub custom_effect_url: Option<String>,
    pub custom_effect_anim: String,
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

// --- Groups Models ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub owner_id: Uuid,
    pub member_count: i64,
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub member_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMemberItem {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateGroupMemberRoleRequest {
    pub role: String, // "admin", "member", "owner" (transfer)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupInviteItem {
    pub code: String,
    pub group_id: Uuid,
    pub inviter_id: Uuid,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupInviteRequest {
    pub max_uses: Option<i32>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupCategoryItem {
    pub id: Uuid,
    pub group_id: Uuid,
    pub name: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupCategoryRequest {
    pub name: String,
    pub position: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupChannelItem {
    pub id: Uuid,
    pub group_id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub channel_type: String, // "text" | "voice" | "video" | "announcement" | "forum" | "media"
    pub position: i32,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupChannelRequest {
    pub category_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub channel_type: Option<String>,
    pub position: Option<i32>,
    pub is_private: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupReactionItem {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToggleReactionRequest {
    pub emoji: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StickerPackItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub author_id: Option<Uuid>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub stickers: Vec<StickerItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StickerItem {
    pub id: Uuid,
    pub pack_id: Uuid,
    pub name: String,
    pub file_url: String,
    pub is_animated: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStickerPackRequest {
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddStickerRequest {
    pub name: String,
    pub file_url: String,
    pub is_animated: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMessageItem {
    pub id: Uuid,
    pub seq: i64,
    pub group_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub sender_avatar_url: Option<String>,
    pub content: String,
    pub reply_to_id: Option<Uuid>,
    pub is_pinned: bool,
    pub is_edited: bool,
    #[serde(default)]
    pub reactions: Vec<GroupReactionItem>,
    pub created_at: DateTime<Utc>,
    /// Message posted by a bot through the Bot API.
    #[serde(default)]
    pub is_bot: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostGroupMessageRequest {
    pub content: String,
    pub reply_to_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
}

// --- Calls Models ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallSessionItem {
    pub id: Uuid,
    pub call_type: String, // "audio" | "video"
    pub caller_id: Uuid,
    pub caller_name: String,
    pub caller_avatar_url: Option<String>,
    pub receiver_id: Option<Uuid>,
    pub receiver_name: Option<String>,
    pub group_id: Option<Uuid>,
    pub group_name: Option<String>,
    #[serde(default)]
    pub meet_code: Option<String>,
    #[serde(default)]
    pub meet_title: Option<String>,
    pub status: String, // "ringing" | "active" | "ended" | "rejected" | "missed" | "busy"
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub participants: Vec<CallParticipantItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMeetRequest {
    pub title: Option<String>,
    pub call_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartCallRequest {
    pub call_type: String, // "audio" | "video"
    pub receiver_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallParticipantItem {
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub muted: bool,
    pub camera_off: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallSignalPayload {
    pub target_user_id: Option<Uuid>,
    pub signal_type: String, // "offer" | "answer" | "ice_candidate" | "reconnect"
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateCallStateRequest {
    pub muted: Option<bool>,
    pub camera_off: Option<bool>,
}


