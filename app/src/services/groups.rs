use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{
        ApiError, GroupCategoryItem, GroupChannelItem, GroupInviteItem, GroupItem, GroupMemberItem,
        GroupMessageItem,
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

async fn put_json<T: DeserializeOwned>(
    path: &str,
    payload: serde_json::Value,
) -> Result<T, ApiError> {
    let (_, body) = platform::http_request("PUT", path, Some(payload.to_string()), None).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

async fn delete_json<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let (_, body) = platform::http_request("DELETE", path, None, None).await?;
    serde_json::from_str(&body).map_err(|_| ApiError {
        code: "invalid_json".into(),
        status: 200,
    })
}

// 1. List user's groups
pub async fn groups_list() -> Result<Vec<GroupItem>, ApiError> {
    get_json("/api/groups").await
}

// 2. Create group
pub async fn groups_create(
    name: &str,
    description: Option<String>,
    avatar_url: Option<String>,
    member_ids: Vec<String>,
) -> Result<GroupItem, ApiError> {
    post_json(
        "/api/groups",
        json!({
            "name": name,
            "description": description,
            "avatar_url": avatar_url,
            "member_ids": member_ids,
        }),
    )
    .await
}

// 3. Get group details
pub async fn groups_get(group_id: &str) -> Result<GroupItem, ApiError> {
    let encoded = urlencoding::encode(group_id);
    get_json(&format!("/api/groups/{encoded}")).await
}

// 4. Update group
pub async fn groups_update(
    group_id: &str,
    name: Option<String>,
    description: Option<String>,
    avatar_url: Option<String>,
) -> Result<GroupItem, ApiError> {
    let encoded = urlencoding::encode(group_id);
    put_json(
        &format!("/api/groups/{encoded}"),
        json!({
            "name": name,
            "description": description,
            "avatar_url": avatar_url,
        }),
    )
    .await
}

// 5. Delete group (owner only)
pub async fn groups_delete(group_id: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(group_id);
    let _: serde_json::Value = delete_json(&format!("/api/groups/{encoded}")).await?;
    Ok(())
}

// 6. List members
pub async fn groups_get_members(group_id: &str) -> Result<Vec<GroupMemberItem>, ApiError> {
    let encoded = urlencoding::encode(group_id);
    get_json(&format!("/api/groups/{encoded}/members")).await
}

// 7. Add member
pub async fn groups_add_member(
    group_id: &str,
    user_id: Option<String>,
    username: Option<String>,
) -> Result<GroupMemberItem, ApiError> {
    let encoded = urlencoding::encode(group_id);
    post_json(
        &format!("/api/groups/{encoded}/members"),
        json!({
            "user_id": user_id,
            "username": username,
        }),
    )
    .await
}

// 8. Remove member / leave group
pub async fn groups_remove_member(group_id: &str, user_id: &str) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_uid = urlencoding::encode(user_id);
    let _: serde_json::Value =
        delete_json(&format!("/api/groups/{enc_gid}/members/{enc_uid}")).await?;
    Ok(())
}

// 9. Update role
pub async fn groups_update_member_role(
    group_id: &str,
    user_id: &str,
    role: &str,
) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_uid = urlencoding::encode(user_id);
    let _: serde_json::Value = put_json(
        &format!("/api/groups/{enc_gid}/members/{enc_uid}/role"),
        json!({ "role": role }),
    )
    .await?;
    Ok(())
}

// 10. Create invite link
pub async fn groups_create_invite(
    group_id: &str,
    max_uses: Option<i32>,
    expires_in_seconds: Option<i64>,
) -> Result<GroupInviteItem, ApiError> {
    let encoded = urlencoding::encode(group_id);
    post_json(
        &format!("/api/groups/{encoded}/invites"),
        json!({
            "max_uses": max_uses,
            "expires_in_seconds": expires_in_seconds,
        }),
    )
    .await
}

// 11. Join group by invite
pub async fn groups_join_by_invite(code: &str) -> Result<GroupItem, ApiError> {
    let encoded = urlencoding::encode(code);
    post_json(&format!("/api/groups/join/{encoded}"), json!({})).await
}

// 12. List messages
pub async fn groups_get_messages(
    group_id: &str,
    channel_id: Option<&str>,
    limit: i64,
    before_seq: Option<i64>,
) -> Result<Vec<GroupMessageItem>, ApiError> {
    let encoded = urlencoding::encode(group_id);
    let mut path = format!("/api/groups/{encoded}/messages?limit={limit}");
    if let Some(cid) = channel_id {
        let enc_cid = urlencoding::encode(cid);
        path.push_str(&format!("&channel_id={enc_cid}"));
    }
    if let Some(seq) = before_seq {
        path.push_str(&format!("&before_seq={seq}"));
    }
    get_json(&path).await
}

// 13. Send group message
pub async fn groups_send_message(
    group_id: &str,
    channel_id: Option<&str>,
    content: &str,
    reply_to_id: Option<String>,
) -> Result<GroupMessageItem, ApiError> {
    let encoded = urlencoding::encode(group_id);
    post_json(
        &format!("/api/groups/{encoded}/messages"),
        json!({
            "content": content,
            "channel_id": channel_id,
            "reply_to_id": reply_to_id,
        }),
    )
    .await
}

// 14. Edit message
pub async fn groups_edit_message(
    group_id: &str,
    message_id: &str,
    content: &str,
) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_mid = urlencoding::encode(message_id);
    let _: serde_json::Value = put_json(
        &format!("/api/groups/{enc_gid}/messages/{enc_mid}"),
        json!({ "content": content }),
    )
    .await?;
    Ok(())
}

// 15. Delete message
pub async fn groups_delete_message(group_id: &str, message_id: &str) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_mid = urlencoding::encode(message_id);
    let _: serde_json::Value =
        delete_json(&format!("/api/groups/{enc_gid}/messages/{enc_mid}")).await?;
    Ok(())
}

// 16. Pin / Unpin message
pub async fn groups_pin_message(group_id: &str, message_id: &str) -> Result<bool, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_mid = urlencoding::encode(message_id);
    let res: serde_json::Value =
        post_json(&format!("/api/groups/{enc_gid}/messages/{enc_mid}/pin"), json!({})).await?;
    Ok(res.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false))
}

// 17. List categories
pub async fn groups_list_categories(group_id: &str) -> Result<Vec<GroupCategoryItem>, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    get_json(&format!("/api/groups/{enc_gid}/categories")).await
}

// 18. Create category
pub async fn groups_create_category(
    group_id: &str,
    name: &str,
    position: Option<i32>,
) -> Result<GroupCategoryItem, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    post_json(
        &format!("/api/groups/{enc_gid}/categories"),
        json!({ "name": name, "position": position }),
    )
    .await
}

// 19. Delete category
pub async fn groups_delete_category(group_id: &str, category_id: &str) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_cid = urlencoding::encode(category_id);
    let _: serde_json::Value =
        delete_json(&format!("/api/groups/{enc_gid}/categories/{enc_cid}")).await?;
    Ok(())
}

// 20. List channels
pub async fn groups_list_channels(group_id: &str) -> Result<Vec<GroupChannelItem>, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    get_json(&format!("/api/groups/{enc_gid}/channels")).await
}

// 21. Create channel
pub async fn groups_create_channel(
    group_id: &str,
    category_id: Option<&str>,
    name: &str,
    channel_type: Option<&str>,
) -> Result<GroupChannelItem, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    post_json(
        &format!("/api/groups/{enc_gid}/channels"),
        json!({
            "category_id": category_id,
            "name": name,
            "channel_type": channel_type.unwrap_or("text"),
        }),
    )
    .await
}

// 22. Delete channel
pub async fn groups_delete_channel(group_id: &str, channel_id: &str) -> Result<(), ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_cid = urlencoding::encode(channel_id);
    let _: serde_json::Value =
        delete_json(&format!("/api/groups/{enc_gid}/channels/{enc_cid}")).await?;
    Ok(())
}

// 23. Toggle Reaction
pub async fn groups_toggle_reaction(
    group_id: &str,
    message_id: &str,
    emoji: &str,
) -> Result<String, ApiError> {
    let enc_gid = urlencoding::encode(group_id);
    let enc_mid = urlencoding::encode(message_id);
    let res: serde_json::Value = post_json(
        &format!("/api/groups/{enc_gid}/messages/{enc_mid}/reactions"),
        json!({ "emoji": emoji }),
    )
    .await?;
    Ok(res.get("action").and_then(|v| v.as_str()).unwrap_or("added").to_string())
}

