use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    pub id: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub has_keys: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProvidersResponse {
    pub google: bool,
    pub dev: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetNameResponse {
    pub id: String,
    pub username: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub relationship: String, // "none" | "friend" | "pending_sent" | "pending_received"
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicKeysResponse {
    pub user_id: String,
    pub ecdh_public: String,
    pub ecdsa_public: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingRequestItem {
    pub id: String,
    pub sender_id: String,
    pub sender_username: String,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FriendItem {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: i64,
    #[serde(default, alias = "users")]
    pub user_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageWireItem {
    pub id: String,
    pub seq: i64,
    pub sender_id: String,
    pub recipient_id: String,
    pub envelope: String,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_edited: bool,
    #[serde(default)]
    pub deleted_for_all: bool,
    #[serde(default)]
    pub reactions: Vec<ReactionSummary>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub id: String,
    pub seq: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestActionResponse {
    pub id: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub owner_id: String,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub member_count: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelItem {
    pub id: String,
    pub server_id: String,
    #[serde(default)]
    pub category_id: Option<String>,
    pub name: String,
    pub channel_type: String, // "text" | "voice" | "announcement"
    #[serde(default)]
    pub position: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelMessageItem {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    #[serde(alias = "sender_name")]
    pub sender_username: String,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub content: String,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceStateItem {
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub self_mute: bool,
    pub self_deaf: bool,
    #[serde(default)]
    pub self_video: bool,
    #[serde(default)]
    pub joined_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayMessage {
    pub id: String,
    pub seq: i64,
    pub sender_id: String,
    pub recipient_id: String,
    pub text: String,
    pub created_at: String,
    pub is_self: bool,
    pub status: MessageStatus,
    pub reply_to_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageStatus {
    Sent,
    Pending,
    Error,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    #[serde(rename = "message.new")]
    MessageNew {
        id: String,
        seq: i64,
        sender_id: String,
        recipient_id: String,
        envelope: String,
        #[serde(default)]
        reply_to_id: Option<String>,
        created_at: String,
    },
    #[serde(rename = "friend.request")]
    FriendRequest {
        id: String,
        sender_id: String,
        sender_username: String,
        created_at: String,
    },
    #[serde(rename = "friend.accepted")]
    FriendAccepted { user_id: String, username: String },
    #[serde(rename = "typing.start")]
    TypingStart { user_id: String },
    #[serde(rename = "typing.stop")]
    TypingStop { user_id: String },
    #[serde(rename = "message.deleted")]
    MessageDeleted { message_id: String },
    #[serde(rename = "message.edited")]
    MessageEdited {
        message_id: String,
        envelope: String,
    },
    #[serde(rename = "channel.message.new")]
    ChannelMessageNew(ChannelMessagePayload),
    #[serde(rename = "message.pinned")]
    MessagePinned {
        message_id: String,
        is_pinned: bool,
    },
    #[serde(rename = "reaction.update")]
    ReactionUpdate(serde_json::Value),
    #[serde(rename = "voice.state.update")]
    VoiceStateUpdate(VoiceStateItem),
    #[serde(rename = "user.presence.update")]
    UserPresenceUpdate(serde_json::Value),
    #[serde(rename = "webrtc.signal")]
    WebRtcSignal(serde_json::Value),
    #[serde(other)]
    Unknown,
}

/// Handles both backend `{ "message": ChannelMessageItem }` and flat `ChannelMessageItem`.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ChannelMessagePayload {
    Wrapped { message: ChannelMessageItem },
    Flat(ChannelMessageItem),
}

impl ChannelMessagePayload {
    pub fn into_item(self) -> ChannelMessageItem {
        match self {
            Self::Wrapped { message } => message,
            Self::Flat(item) => item,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub status: u16,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl std::error::Error for ApiError {}

