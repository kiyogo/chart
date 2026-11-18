use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{
        ApiError, AvatarUploadResponse, ChannelItem, ChannelMessageItem, FriendItem, MeResponse,
        MessageWireItem, PendingRequestItem, ProvidersResponse, PublicKeysResponse,
        RequestActionResponse, SendMessageResponse, ServerItem, SetNameResponse, UserSearchResult,
        VoiceStateItem,
    },
    platform,
};

async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let (_, body) = platform::http_request("GET", path, None, None).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

async fn post_json<T: DeserializeOwned>(
    path: &str,
    payload: serde_json::Value,
) -> Result<T, ApiError> {
    let (_, body) = platform::http_request("POST", path, Some(payload.to_string()), None).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

async fn post_empty(path: &str, payload: Option<serde_json::Value>) -> Result<(), ApiError> {
    let body = payload.map(|v| v.to_string());
    let _ = platform::http_request("POST", path, body, None).await?;
    Ok(())
}

// --- Auth ---

pub async fn auth_me() -> Result<MeResponse, ApiError> {
    get_json("/api/auth/me").await
}

pub async fn auth_providers() -> Result<ProvidersResponse, ApiError> {
    get_json("/api/auth/providers").await
}

pub async fn auth_logout() -> Result<(), ApiError> {
    post_empty("/api/auth/logout", None).await
}

pub async fn auth_dev_login(username: Option<String>) -> Result<(), ApiError> {
    post_empty("/api/auth/dev-login", Some(json!({ "username": username }))).await
}

// --- Users ---

pub async fn users_set_name(name: &str) -> Result<SetNameResponse, ApiError> {
    post_json("/api/users/name", json!({ "name": name })).await
}

pub async fn users_search(q: &str) -> Result<Vec<UserSearchResult>, ApiError> {
    let encoded = urlencoding::encode(q);
    get_json(&format!("/api/users/search?q={encoded}")).await
}

pub async fn users_upload_avatar(avif_bytes: Vec<u8>) -> Result<AvatarUploadResponse, ApiError> {
    let (_, body) =
        platform::http_request("POST", "/api/users/avatar", None, Some((avif_bytes, "image/avif")))
            .await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

// --- Keys ---

pub async fn keys_register(
    ecdh_public: &str,
    ecdsa_public: &str,
) -> Result<PublicKeysResponse, ApiError> {
    post_json(
        "/api/keys",
        json!({
            "ecdh_public": ecdh_public,
            "ecdsa_public": ecdsa_public
        }),
    )
    .await
}

pub async fn keys_get(user_id: &str) -> Result<PublicKeysResponse, ApiError> {
    let encoded = urlencoding::encode(user_id);
    get_json(&format!("/api/keys/{encoded}")).await
}

// --- Friends ---

pub async fn friends_send_request_by_username(
    username: &str,
) -> Result<RequestActionResponse, ApiError> {
    post_json("/api/friends/requests", json!({ "username": username })).await
}

pub async fn friends_send_request_by_id(
    receiver_id: &str,
) -> Result<RequestActionResponse, ApiError> {
    post_json(
        "/api/friends/requests",
        json!({ "receiver_id": receiver_id }),
    )
    .await
}

pub async fn friends_get_requests() -> Result<Vec<PendingRequestItem>, ApiError> {
    get_json("/api/friends/requests").await
}

pub async fn friends_accept_request(id: &str) -> Result<RequestActionResponse, ApiError> {
    let encoded = urlencoding::encode(id);
    post_json(&format!("/api/friends/requests/{encoded}/accept"), json!({})).await
}

pub async fn friends_list() -> Result<Vec<FriendItem>, ApiError> {
    get_json("/api/friends").await
}

// --- Messages ---

pub async fn messages_send(
    message_id: &str,
    recipient_id: &str,
    recipient_envelope: &str,
    sender_envelope: &str,
) -> Result<SendMessageResponse, ApiError> {
    post_json(
        "/api/messages",
        json!({
            "message_id": message_id,
            "recipient_id": recipient_id,
            "recipient_envelope": recipient_envelope,
            "sender_envelope": sender_envelope
        }),
    )
    .await
}

pub async fn messages_get(
    friend_id: &str,
    before_seq: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<MessageWireItem>, ApiError> {
    let encoded = urlencoding::encode(friend_id);
    let mut path = format!("/api/messages/{encoded}?limit={}", limit.unwrap_or(50));
    if let Some(seq) = before_seq {
        path.push_str(&format!("&before_seq={seq}"));
    }
    get_json(&path).await
}

// --- Typing ---

pub async fn typing_send(friend_id: &str, is_typing: bool) -> Result<(), ApiError> {
    post_empty(
        "/api/typing",
        Some(json!({
            "friend_id": friend_id,
            "is_typing": is_typing
        })),
    )
    .await
}

// --- Servers & Channels ---

pub async fn servers_list() -> Result<Vec<ServerItem>, ApiError> {
    get_json("/api/servers").await
}

pub async fn servers_create(name: &str) -> Result<ServerItem, ApiError> {
    post_json("/api/servers", json!({ "name": name })).await
}

pub async fn servers_join(code: &str) -> Result<ServerItem, ApiError> {
    let encoded = urlencoding::encode(code);
    post_json(&format!("/api/servers/join/{encoded}"), json!({})).await
}

pub async fn servers_get_channels(server_id: &str) -> Result<Vec<ChannelItem>, ApiError> {
    let encoded = urlencoding::encode(server_id);
    get_json(&format!("/api/servers/{encoded}/channels")).await
}

pub async fn channels_get_messages(
    channel_id: &str,
    limit: i64,
) -> Result<Vec<ChannelMessageItem>, ApiError> {
    let encoded = urlencoding::encode(channel_id);
    get_json(&format!("/api/channels/{encoded}/messages?limit={limit}")).await
}

pub async fn channels_post_message(
    channel_id: &str,
    content: &str,
) -> Result<ChannelMessageItem, ApiError> {
    let encoded = urlencoding::encode(channel_id);
    post_json(
        &format!("/api/channels/{encoded}/messages"),
        json!({ "content": content }),
    )
    .await
}

// --- WebRTC / Voice ---

pub async fn webrtc_join_voice(channel_id: &str) -> Result<Vec<VoiceStateItem>, ApiError> {
    let encoded = urlencoding::encode(channel_id);
    post_json(&format!("/api/webrtc/voice/{encoded}/join"), json!({})).await
}

pub async fn webrtc_update_voice_state(
    channel_id: &str,
    self_mute: bool,
) -> Result<VoiceStateItem, ApiError> {
    let encoded = urlencoding::encode(channel_id);
    post_json(
        &format!("/api/webrtc/voice/{encoded}/state"),
        json!({ "self_mute": self_mute }),
    )
    .await
}

pub async fn webrtc_leave_voice(channel_id: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(channel_id);
    post_empty(&format!("/api/webrtc/voice/{encoded}/leave"), None).await
}

