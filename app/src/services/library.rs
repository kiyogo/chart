//! Personal media library (custom emoji, saved GIFs, profile artwork) and account APIs.

use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{ApiError, LibraryItem, SessionItem},
    platform,
};

async fn send<T: DeserializeOwned>(method: &str, path: &str, body: Option<serde_json::Value>) -> Result<T, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

/// Upload an image (PNG/JPEG/WebP/AVIF/GIF, at most 2 MB) to the personal media store.
pub async fn upload_media(bytes: Vec<u8>, mime: &str) -> Result<String, ApiError> {
    let (_, body) = platform::http_request("POST", "/api/media/upload", None, Some((bytes, mime))).await?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    v.get("url").and_then(|u| u.as_str()).map(str::to_string).ok_or(ApiError { code: "missing_url".into(), status: 200 })
}

pub async fn emojis_list() -> Result<Vec<LibraryItem>, ApiError> {
    send("GET", "/api/media/emojis", None).await
}
pub async fn emojis_create(name: &str, file_url: &str) -> Result<LibraryItem, ApiError> {
    send("POST", "/api/media/emojis", Some(json!({ "name": name, "file_url": file_url }))).await
}
pub async fn emojis_delete(id: &str) -> Result<serde_json::Value, ApiError> {
    send("DELETE", &format!("/api/media/emojis/{}", urlencoding::encode(id)), None).await
}
pub async fn gifs_list() -> Result<Vec<LibraryItem>, ApiError> {
    send("GET", "/api/media/gifs", None).await
}
pub async fn gifs_create(name: &str, file_url: &str) -> Result<LibraryItem, ApiError> {
    send("POST", "/api/media/gifs", Some(json!({ "name": name, "file_url": file_url }))).await
}
pub async fn gifs_delete(id: &str) -> Result<serde_json::Value, ApiError> {
    send("DELETE", &format!("/api/media/gifs/{}", urlencoding::encode(id)), None).await
}

pub async fn change_username(name: &str) -> Result<serde_json::Value, ApiError> {
    send("PUT", "/api/users/username", Some(json!({ "name": name }))).await
}
pub async fn sessions_list() -> Result<Vec<SessionItem>, ApiError> {
    send("GET", "/api/auth/sessions", None).await
}
pub async fn sessions_revoke(id: &str) -> Result<serde_json::Value, ApiError> {
    send("DELETE", &format!("/api/auth/sessions/{}", urlencoding::encode(id)), None).await
}
pub async fn sessions_revoke_others() -> Result<serde_json::Value, ApiError> {
    send("DELETE", "/api/auth/sessions", None).await
}
