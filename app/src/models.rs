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
    #[serde(rename = "group.message.new")]
    GroupMessageNew(GroupMessagePayload),
    #[serde(rename = "group.member.joined")]
    GroupMemberJoined(serde_json::Value),
    #[serde(rename = "group.member.left")]
    GroupMemberLeft(serde_json::Value),
    #[serde(rename = "group.updated")]
    GroupUpdated(serde_json::Value),
    #[serde(rename = "group.reaction.update")]
    GroupReactionUpdate(serde_json::Value),
    #[serde(rename = "group.channel.new")]
    GroupChannelNew(serde_json::Value),
    #[serde(rename = "group.channel.deleted")]
    GroupChannelDeleted(serde_json::Value),
    #[serde(rename = "call.incoming")]
    CallIncoming(CallSessionPayload),
    #[serde(rename = "call.accepted")]
    CallAccepted(serde_json::Value),
    #[serde(rename = "call.rejected")]
    CallRejected(serde_json::Value),
    #[serde(rename = "call.ended")]
    CallEnded(serde_json::Value),
    #[serde(rename = "call.signal")]
    CallSignal(serde_json::Value),
    #[serde(rename = "call.participant.joined")]
    CallParticipantJoined(serde_json::Value),
    #[serde(rename = "call.participant.left")]
    CallParticipantLeft(serde_json::Value),
    #[serde(rename = "call.state.update")]
    CallStateUpdate(serde_json::Value),
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum GroupMessagePayload {
    Wrapped { message: GroupMessageItem },
    Flat(GroupMessageItem),
}

impl GroupMessagePayload {
    pub fn into_item(self) -> GroupMessageItem {
        match self {
            Self::Wrapped { message } => message,
            Self::Flat(item) => item,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum CallSessionPayload {
    Wrapped { session: CallSessionItem },
    Flat(CallSessionItem),
}

impl CallSessionPayload {
    pub fn into_item(self) -> CallSessionItem {
        match self {
            Self::Wrapped { session } => session,
            Self::Flat(item) => item,
        }
    }
}

// --- Groups Models for App ---

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub owner_id: String,
    #[serde(default)]
    pub member_count: i64,
    #[serde(default)]
    pub role: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupMemberItem {
    pub group_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupInviteItem {
    pub code: String,
    pub group_id: String,
    pub inviter_id: String,
    #[serde(default)]
    pub max_uses: Option<i32>,
    pub uses: i32,
    #[serde(default)]
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupCategoryItem {
    pub id: String,
    pub group_id: String,
    pub name: String,
    #[serde(default)]
    pub position: i32,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupChannelItem {
    pub id: String,
    pub group_id: String,
    #[serde(default)]
    pub category_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_channel_type")]
    pub channel_type: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub is_private: bool,
    pub created_at: String,
}

fn default_channel_type() -> String {
    "text".to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupReactionItem {
    pub id: String,
    pub message_id: String,
    pub user_id: String,
    pub username: String,
    pub emoji: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StickerPackItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author_id: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub stickers: Vec<StickerItem>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StickerItem {
    pub id: String,
    pub pack_id: String,
    pub name: String,
    pub file_url: String,
    #[serde(default)]
    pub is_animated: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub custom_banner_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub status_emoji: Option<String>,
    pub presence: String,
    #[serde(default)]
    pub badges: Vec<String>,
    #[serde(default)]
    pub social_links: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupMessageItem {
    pub id: String,
    pub seq: i64,
    pub group_id: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    pub sender_id: String,
    #[serde(alias = "sender_name")]
    pub sender_username: String,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub content: String,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_edited: bool,
    #[serde(default)]
    pub reactions: Vec<GroupReactionItem>,
    pub created_at: String,
}

// --- Calls Models for App ---

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallSessionItem {
    pub id: String,
    pub call_type: String, // "audio" | "video"
    pub caller_id: String,
    pub caller_name: String,
    #[serde(default)]
    pub caller_avatar_url: Option<String>,
    #[serde(default)]
    pub receiver_id: Option<String>,
    #[serde(default)]
    pub receiver_name: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
    pub status: String, // "ringing" | "active" | "ended" | "rejected" | "missed" | "busy"
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub participants: Vec<CallParticipantItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallParticipantItem {
    pub call_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub joined_at: String,
    #[serde(default)]
    pub left_at: Option<String>,
    pub muted: bool,
    pub camera_off: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallSignalPayload {
    #[serde(default)]
    pub target_user_id: Option<String>,
    pub signal_type: String, // "offer" | "answer" | "ice_candidate"
    pub data: serde_json::Value,
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

