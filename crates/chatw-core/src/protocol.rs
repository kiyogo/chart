use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub has_keys: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvidersResponse {
    pub google: bool,
    pub dev: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub relationship: String, // "none" | "friend" | "pending_sent" | "pending_received"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicKeysResponse {
    pub user_id: Uuid,
    pub ecdh_public: String,
    pub ecdsa_public: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendItem {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingRequestItem {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageWireItem {
    pub id: Uuid,
    pub seq: i64,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub envelope: String, // base64 URL-safe unpadded
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsServerEvent {
    #[serde(rename = "message.new")]
    MessageNew(MessageWireItem),

    #[serde(rename = "message.ack")]
    MessageAck { id: Uuid, seq: i64 },

    #[serde(rename = "friend.request")]
    FriendRequest(PendingRequestItem),

    #[serde(rename = "friend.accepted")]
    FriendAccepted { user_id: Uuid, username: String },

    #[serde(rename = "typing.start")]
    TypingStart { user_id: Uuid },

    #[serde(rename = "typing.stop")]
    TypingStop { user_id: Uuid },

    #[serde(rename = "error")]
    Error { code: String, message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsClientCommand {
    #[serde(rename = "typing.start")]
    TypingStart { recipient_id: Uuid },

    #[serde(rename = "typing.stop")]
    TypingStop { recipient_id: Uuid },

    #[serde(rename = "ack")]
    Ack { message_id: Uuid },
}
