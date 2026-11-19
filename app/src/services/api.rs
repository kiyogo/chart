use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{
        ApiError, AvatarUploadResponse, ChannelItem, ChannelMessageItem, FriendItem, MeResponse,
        MessageWireItem, PendingRequestItem, ProvidersResponse, PublicKeysResponse,
        RequestActionResponse, SendMessageResponse, ServerItem, SetNameResponse, UserProfileResponse,
        UserSearchResult, VoiceStateItem,
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

pub async fn users_upload_banner(avif_bytes: Vec<u8>) -> Result<String, ApiError> {
    let (_, body) =
        platform::http_request("POST", "/api/users/banner", None, Some((avif_bytes, "image/avif")))
            .await?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })?;
    v.get("banner_url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or(ApiError {
            code: "missing_banner_url".into(),
            status: 200,
        })
}

pub async fn attachments_upload(bytes: Vec<u8>, mime_type: &str) -> Result<String, ApiError> {
    let (_, body) =
        platform::http_request("POST", "/api/attachments", None, Some((bytes, mime_type))).await?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })?;
    v.get("download_url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or(ApiError {
            code: "missing_download_url".into(),
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
    messages_send_reply(message_id, recipient_id, recipient_envelope, sender_envelope, None).await
}

/// Send an E2EE direct message, optionally as a reply (the reply id is metadata only).
pub async fn messages_send_reply(
    message_id: &str,
    recipient_id: &str,
    recipient_envelope: &str,
    sender_envelope: &str,
    reply_to_id: Option<&str>,
) -> Result<SendMessageResponse, ApiError> {
    post_json(
        "/api/messages",
        json!({
            "message_id": message_id,
            "recipient_id": recipient_id,
            "recipient_envelope": recipient_envelope,
            "sender_envelope": sender_envelope,
            "reply_to_id": reply_to_id
        }),
    )
    .await
}

/// Pin or unpin a direct message (both participants see it).
pub async fn messages_pin(message_id: &str, pinned: bool) -> Result<(), ApiError> {
    let method = if pinned { "POST" } else { "DELETE" };
    platform::http_request(method, &format!("/api/messages/{}/pin", urlencoding::encode(message_id)), None, None)
        .await
        .map(|_| ())
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

pub async fn servers_get_categories(server_id: &str) -> Result<Vec<crate::models::ServerCategoryItem>, ApiError> {
    let encoded = urlencoding::encode(server_id);
    get_json(&format!("/api/servers/{encoded}/categories")).await
}

pub async fn servers_create_category(
    server_id: &str,
    name: &str,
) -> Result<crate::models::ServerCategoryItem, ApiError> {
    let encoded = urlencoding::encode(server_id);
    post_json(&format!("/api/servers/{encoded}/categories"), json!({ "name": name })).await
}

pub async fn servers_get_channels(server_id: &str) -> Result<Vec<ChannelItem>, ApiError> {
    let encoded = urlencoding::encode(server_id);
    get_json(&format!("/api/servers/{encoded}/channels")).await
}

pub async fn servers_create_channel(
    server_id: &str,
    name: &str,
    category_id: Option<&str>,
    channel_type: &str,
) -> Result<ChannelItem, ApiError> {
    let encoded = urlencoding::encode(server_id);
    post_json(
        &format!("/api/servers/{encoded}/channels"),
        json!({
            "name": name,
            "category_id": category_id,
            "channel_type": channel_type
        }),
    )
    .await
}

pub async fn servers_get_members(server_id: &str) -> Result<Vec<crate::models::ServerMemberItem>, ApiError> {
    let encoded = urlencoding::encode(server_id);
    get_json(&format!("/api/servers/{encoded}/members")).await
}

pub async fn servers_create_invite(server_id: &str) -> Result<String, ApiError> {
    let encoded = urlencoding::encode(server_id);
    let v: serde_json::Value =
        post_json(&format!("/api/servers/{encoded}/invites"), json!({})).await?;
    v.get("code")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or(ApiError {
            code: "missing_invite_code".into(),
            status: 200,
        })
}

pub async fn servers_discover() -> Result<Vec<ServerItem>, ApiError> {
    get_json("/api/servers/discover").await
}

pub async fn servers_join_direct(server_id: &str) -> Result<ServerItem, ApiError> {
    let encoded = urlencoding::encode(server_id);
    post_json(&format!("/api/servers/{encoded}/join"), json!({})).await
}

pub async fn webrtc_get_voice_participants(channel_id: &str) -> Result<Vec<VoiceStateItem>, ApiError> {
    let encoded = urlencoding::encode(channel_id);
    get_json(&format!("/api/webrtc/voice/{encoded}/participants")).await
}

pub async fn messages_toggle_reaction(message_id: &str, emoji: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(message_id);
    post_empty(
        &format!("/api/messages/{encoded}/reactions"),
        Some(json!({ "emoji": emoji })),
    )
    .await
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

// --- Profiles ---

pub async fn get_user_profile(user_id: &str) -> Result<UserProfileResponse, ApiError> {
    let encoded = urlencoding::encode(user_id);
    let mut p: UserProfileResponse = get_json(&format!("/api/users/{encoded}/profile")).await?;
    // Avatar motion, visible details and my private nickname / note (spec 009).
    if let Ok(ext) = crate::services::profile_ext::fetch(user_id).await {
        crate::services::profile_ext::merge_into(&mut p, &ext);
    }
    Ok(p)
}

pub async fn update_my_profile(
    display_name: Option<&str>,
    bio: Option<&str>,
    status_text: Option<&str>,
    status_emoji: Option<&str>,
    presence: Option<&str>,
    custom_banner_url: Option<&str>,
) -> Result<UserProfileResponse, ApiError> {
    post_json(
        "/api/users/me/profile",
        json!({
            "display_name": display_name,
            "bio": bio,
            "status_text": status_text,
            "status_emoji": status_emoji,
            "presence": presence,
            "custom_banner_url": custom_banner_url,
        }),
    )
    .await
}


/// Partial profile update: only keys present in `fields` are changed.
/// Empty string clears `profile_color` / `profile_accent`.
pub async fn update_my_profile_fields(fields: serde_json::Value) -> Result<UserProfileResponse, ApiError> {
    post_json("/api/users/me/profile", fields).await
}
