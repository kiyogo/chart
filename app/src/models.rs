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
    #[serde(default)]
    pub device_id: Option<String>,
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
    pub display_name: Option<String>,
    #[serde(default)]
    pub nameplate: Option<String>,
    #[serde(default)]
    pub custom_nameplate_url: Option<String>,
    #[serde(default)]
    pub name_effect: Option<String>,
    #[serde(default)]
    pub profile_color: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub status_emoji: Option<String>,
    #[serde(default)]
    pub presence: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub avatar_decoration: Option<String>,
    #[serde(default)]
    pub avatar_motion: Option<crate::components::avatar_motion::AvatarMotion>,
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
    pub is_enterprise: bool,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub member_count: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerCategoryItem {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub position: i32,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerMemberItem {
    pub server_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub joined_at: String,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerMessageSearchHit {
    pub message_id: String,
    pub channel_id: String,
    pub channel_name: String,
    pub sender_id: String,
    pub sender_name: String,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub content: String,
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
    /// emoji -> user ids who reacted (DM reactions are plain metadata on the server).
    pub reactions: Vec<(String, Vec<String>)>,
    pub is_pinned: bool,
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
    #[serde(rename = "message.read")]
    MessageRead(serde_json::Value),
    #[serde(rename = "wallpaper.updated")]
    WallpaperUpdated(serde_json::Value),
    #[serde(rename = "bot.message")]
    BotMessage(serde_json::Value),
    #[serde(rename = "group.join_request")]
    GroupJoinRequest(serde_json::Value),
    #[serde(rename = "group.settings")]
    GroupSettings(serde_json::Value),
    #[serde(rename = "prefs.updated")]
    PrefsUpdated(serde_json::Value),
    #[serde(rename = "profile.updated")]
    ProfileUpdated(serde_json::Value),
    #[serde(rename = "security.alert")]
    SecurityAlert(serde_json::Value),
    #[serde(rename = "enterprise.event")]
    EnterpriseEvent(serde_json::Value),
    #[serde(rename = "feed.updated")]
    FeedUpdated(serde_json::Value),
    #[serde(rename = "device.approval.request")]
    DeviceApprovalRequest(serde_json::Value),
    #[serde(rename = "device.approval.resolved")]
    DeviceApprovalResolved(serde_json::Value),
    #[serde(rename = "device.revoked")]
    DeviceRevoked(serde_json::Value),
    #[serde(rename = "recovery.request.incoming")]
    RecoveryRequestIncoming(serde_json::Value),
    #[serde(rename = "recovery.request.updated")]
    RecoveryRequestUpdated(serde_json::Value),
    #[serde(rename = "recovery.p2p.signal")]
    RecoveryP2pSignal(serde_json::Value),
    #[serde(rename = "media.delete_for_all")]
    MediaDeleteForAll(serde_json::Value),
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
    #[serde(default)]
    pub profile_color: Option<String>,
    #[serde(default)]
    pub profile_accent: Option<String>,
    #[serde(default)]
    pub avatar_decoration: Option<String>,
    #[serde(default)]
    pub profile_effect: Option<String>,
    #[serde(default)]
    pub member_since: Option<String>,
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
    // Spec 009 (merged from /api/profile-ext)
    #[serde(default)]
    pub avatar_motion: Option<crate::components::avatar_motion::AvatarMotion>,
    #[serde(default)]
    pub details: ProfileDetails,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub detail_visibility: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    pub status_expires_at: Option<String>,
}

/// Optional profile details; each one is only present when its owner lets the viewer see it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileDetails {
    #[serde(default)]
    pub pronouns: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub occupation: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
}

impl ProfileDetails {
    pub fn has_any(&self) -> bool {
        [&self.location, &self.website, &self.occupation].iter().any(|v| v.as_ref().is_some_and(|x| !x.is_empty())) || !self.interests.is_empty()
    }
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
    #[serde(default)]
    pub is_bot: bool,
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


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryItem {
    pub id: String,
    pub name: String,
    pub file_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionItem {
    pub id: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub device_label: Option<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub last_active: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub is_current: bool,
}

// ---------------------------------------------------------------------------
// Spec 007: privacy, conversation organisation, groups, bots, business, directory
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PrivacySettings {
    pub friend_requests: String,
    pub calls: String,
    pub group_invites: String,
    pub discoverable: bool,
    pub read_receipts: bool,
    pub typing: bool,
    pub self_chat: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockedUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatLabel {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub position: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConvPref {
    pub conv_type: String,
    pub conv_id: String,
    #[serde(default)]
    pub pinned_at: Option<String>,
    #[serde(default)]
    pub label_ids: Vec<String>,
    #[serde(default)]
    pub in_messages: bool,
    #[serde(default)]
    pub in_communities: bool,
    #[serde(default)]
    pub muted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GroupSettings {
    pub perm_send: bool,
    pub perm_poll: bool,
    pub perm_note: bool,
    pub perm_reminder: bool,
    pub perm_pin: bool,
    pub perm_rename: bool,
    pub perm_avatar: bool,
    pub require_approval: bool,
    pub highlight_admins: bool,
    pub allow_link_join: bool,
    pub is_public: bool,
    pub verification_status: String,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub my_role: Option<String>,
    #[serde(default)]
    pub pending_requests: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JoinRequestItem {
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InvitePreview {
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub member_count: i64,
    #[serde(default)]
    pub require_approval: bool,
    #[serde(default)]
    pub is_member: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ServerSettings {
    pub is_public: bool,
    pub public_index: bool,
    pub verification_status: String,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub is_enterprise: bool,
    #[serde(default)]
    pub retention_locked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BotItem {
    pub id: String,
    #[serde(default)]
    pub owner_id: String,
    pub username: String,
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub verification_status: String,
    #[serde(default)]
    pub token_hint: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub last_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BotChatMessage {
    pub id: String,
    pub seq: i64,
    pub from_bot: bool,
    pub text: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct BusinessProfile {
    pub handle: String,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub hours: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub greeting: Option<String>,
    #[serde(default)]
    pub away_message: Option<String>,
    #[serde(default)]
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BusinessProduct {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuickReply {
    pub id: String,
    pub shortcut: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct BusinessData {
    #[serde(default)]
    pub profile: Option<BusinessProfile>,
    #[serde(default)]
    pub products: Vec<BusinessProduct>,
    #[serde(default)]
    pub quick_replies: Vec<QuickReply>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DirectoryEntry {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub member_count: i64,
    #[serde(default)]
    pub joined: bool,
    #[serde(default)]
    pub require_approval: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DirectoryResults {
    #[serde(default)]
    pub servers: Vec<DirectoryEntry>,
    #[serde(default)]
    pub groups: Vec<DirectoryEntry>,
    #[serde(default)]
    pub bots: Vec<DirectoryEntry>,
    #[serde(default)]
    pub businesses: Vec<DirectoryEntry>,
}
