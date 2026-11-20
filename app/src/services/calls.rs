use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{ApiError, CallSessionItem},
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

// 1. Start call
pub async fn calls_start(
    call_type: &str,
    receiver_id: Option<String>,
    group_id: Option<String>,
) -> Result<CallSessionItem, ApiError> {
    post_json(
        "/api/calls/start",
        json!({
            "call_type": call_type,
            "receiver_id": receiver_id,
            "group_id": group_id,
        }),
    )
    .await
}

// 2. Accept call
pub async fn calls_accept(call_id: &str) -> Result<CallSessionItem, ApiError> {
    let encoded = urlencoding::encode(call_id);
    post_json(&format!("/api/calls/{encoded}/accept"), json!({})).await
}

// 3. Reject call
pub async fn calls_reject(call_id: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(call_id);
    let _: serde_json::Value =
        post_json(&format!("/api/calls/{encoded}/reject"), json!({})).await?;
    Ok(())
}

// 4. End call
pub async fn calls_end(call_id: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(call_id);
    let _: serde_json::Value = post_json(&format!("/api/calls/{encoded}/end"), json!({})).await?;
    Ok(())
}

// 5. Leave call
pub async fn calls_leave(call_id: &str) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(call_id);
    let _: serde_json::Value = post_json(&format!("/api/calls/{encoded}/leave"), json!({})).await?;
    Ok(())
}

// 6. Relay WebRTC signal
pub async fn calls_send_signal(
    call_id: &str,
    target_user_id: Option<String>,
    signal_type: &str,
    data: serde_json::Value,
) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(call_id);
    let _: serde_json::Value = post_json(
        &format!("/api/calls/{encoded}/signal"),
        json!({
            "target_user_id": target_user_id,
            "signal_type": signal_type,
            "data": data,
        }),
    )
    .await?;
    Ok(())
}

// 7. Update call mute / camera state
pub async fn calls_update_state(
    call_id: &str,
    muted: Option<bool>,
    camera_off: Option<bool>,
) -> Result<(), ApiError> {
    let encoded = urlencoding::encode(call_id);
    let _: serde_json::Value = post_json(
        &format!("/api/calls/{encoded}/state"),
        json!({
            "muted": muted,
            "camera_off": camera_off,
        }),
    )
    .await?;
    Ok(())
}

// 8. Get current active call
pub async fn calls_get_active() -> Result<Option<CallSessionItem>, ApiError> {
    get_json("/api/calls/active").await
}

// 9. Get call history
pub async fn calls_get_history() -> Result<Vec<CallSessionItem>, ApiError> {
    get_json("/api/calls/history").await
}

// 10. Extract meet code from either raw code or full URL
pub fn extract_meet_code(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    if let Some(idx) = s.find("meet=") {
        s = s[idx + 5..].to_string();
    } else if let Some(idx) = s.find("/meet/") {
        s = s[idx + 6..].to_string();
    } else if let Some(idx) = s.rfind('/') {
        s = s[idx + 1..].to_string();
    }
    if let Some(idx) = s.find(['?', '#', '&', ' ']) {
        s = s[..idx].to_string();
    }
    s.trim().to_string()
}

// 11. Create a shareable Meet room (Google Meet / Zoom style)
pub async fn calls_create_meet(
    title: Option<String>,
    call_type: &str,
) -> Result<CallSessionItem, ApiError> {
    post_json(
        "/api/calls/meet",
        json!({
            "title": title,
            "call_type": call_type,
        }),
    )
    .await
}

// 12. Join a Meet room by code or full link
pub async fn calls_join_meet(code_or_link: &str) -> Result<CallSessionItem, ApiError> {
    let code = extract_meet_code(code_or_link);
    let encoded = urlencoding::encode(&code);
    post_json(&format!("/api/calls/meet/{encoded}/join"), json!({})).await
}

// 13. List user's Meet rooms
pub async fn calls_list_meets() -> Result<Vec<CallSessionItem>, ApiError> {
    get_json("/api/calls/meet/list").await
}

