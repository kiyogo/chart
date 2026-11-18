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

