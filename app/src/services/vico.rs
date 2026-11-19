//! Client API for Spec 016: Standalone "Chat với Vico AI" (separate from Enterprise AI).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VicoConversation {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub last_message: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VicoMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct VicoConfig {
    #[serde(default)]
    pub api_endpoint: String,
    #[serde(default)]
    pub has_api_key: bool,
    #[serde(default)]
    pub model_name: String,
}

#[derive(Deserialize)]
pub struct SendVicoResp {
    pub user_message: VicoMessage,
    pub assistant_message: VicoMessage,
}

pub async fn list_conversations() -> Result<Vec<VicoConversation>, ApiError> {
    let (_, text) = platform::http_request("GET", "/api/vico/conversations", None, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn create_conversation(title: Option<&str>) -> Result<VicoConversation, ApiError> {
    let body = json!({ "title": title }).to_string();
    let (_, text) = platform::http_request("POST", "/api/vico/conversations", Some(body), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn delete_conversation(conv_id: &str) -> Result<(), ApiError> {
    let id = urlencoding::encode(conv_id);
    let _ = platform::http_request("DELETE", &format!("/api/vico/conversations/{id}"), None, None).await?;
    Ok(())
}

pub async fn list_messages(conv_id: &str) -> Result<Vec<VicoMessage>, ApiError> {
    let id = urlencoding::encode(conv_id);
    let (_, text) = platform::http_request("GET", &format!("/api/vico/conversations/{id}/messages"), None, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn send_message(conv_id: &str, content: &str) -> Result<SendVicoResp, ApiError> {
    let id = urlencoding::encode(conv_id);
    let body = json!({ "content": content }).to_string();
    let (_, text) = platform::http_request("POST", &format!("/api/vico/conversations/{id}/messages"), Some(body), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn get_config() -> Result<VicoConfig, ApiError> {
    let (_, text) = platform::http_request("GET", "/api/vico/config", None, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn save_config(endpoint: &str, api_key: Option<&str>, model_name: &str) -> Result<(), ApiError> {
    let body = json!({
        "api_endpoint": endpoint,
        "api_key": api_key,
        "model_name": model_name,
    })
    .to_string();
    let _: Value = serde_json::from_str(
        &platform::http_request("PUT", "/api/vico/config", Some(body), None).await?.1,
    )
    .map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    Ok(())
}
