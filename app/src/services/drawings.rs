//! Server storage for ChatW Studio projects (private to their owner).

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub width: i64,
    pub height: i64,
    pub frames: i64,
    pub layers: i64,
    pub size: i64,
    pub has_thumb: bool,
    pub updated_at: String,
}

fn bad() -> ApiError {
    ApiError { code: "invalid_json".into(), status: 200 }
}

pub async fn list() -> Result<Vec<ProjectItem>, ApiError> {
    let (_, body) = platform::http_request("GET", "/api/drawings", None, None).await?;
    serde_json::from_str(&body).map_err(|_| bad())
}

/// Create (`id` = None) or overwrite a project. Returns its id.
pub async fn save(id: Option<&str>, name: &str, kind: &str, data: &str, thumb_data_url: &str) -> Result<String, ApiError> {
    let thumb = thumb_data_url.strip_prefix("data:image/png;base64,").unwrap_or_default();
    let body = json!({ "name": name, "kind": kind, "data": data, "thumb": thumb }).to_string();
    let (method, path) = match id {
        Some(i) => ("PUT", format!("/api/drawings/{}", urlencoding::encode(i))),
        None => ("POST", "/api/drawings".to_string()),
    };
    let (_, resp) = platform::http_request(method, &path, Some(body), None).await?;
    let v: Value = serde_json::from_str(&resp).map_err(|_| bad())?;
    v.get("id").and_then(|x| x.as_str()).map(str::to_string).ok_or_else(bad)
}

/// (name, kind, project document)
pub async fn open(id: &str) -> Result<(String, String, Value), ApiError> {
    let (_, body) = platform::http_request("GET", &format!("/api/drawings/{}", urlencoding::encode(id)), None, None).await?;
    let v: Value = serde_json::from_str(&body).map_err(|_| bad())?;
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("Untitled").to_string();
    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("free").to_string();
    Ok((name, kind, v.get("data").cloned().unwrap_or(Value::Null)))
}

pub async fn delete(id: &str) -> Result<(), ApiError> {
    platform::http_request("DELETE", &format!("/api/drawings/{}", urlencoding::encode(id)), None, None).await.map(|_| ())
}

pub fn thumb_url(id: &str) -> String {
    platform::resolve_asset_url(&format!("/api/drawings/{}/thumb", urlencoding::encode(id)))
}
