//! Client API for Spec 016: Threads-style Social Feed ("Mới").

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepostPreview {
    pub id: String,
    pub author_id: String,
    pub author_username: String,
    #[serde(default)]
    pub author_display_name: Option<String>,
    #[serde(default)]
    pub author_avatar_url: Option<String>,
    pub content: String,
    #[serde(default)]
    pub media_url: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedPostItem {
    pub id: String,
    pub author_id: String,
    pub author_username: String,
    #[serde(default)]
    pub author_display_name: Option<String>,
    #[serde(default)]
    pub author_avatar_url: Option<String>,
    #[serde(default)]
    pub author_nameplate: String,
    #[serde(default)]
    pub author_custom_nameplate_url: Option<String>,
    #[serde(default)]
    pub author_name_effect: String,
    #[serde(default)]
    pub author_profile_color: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub repost_of_id: Option<String>,
    #[serde(default)]
    pub repost_of: Option<RepostPreview>,
    pub content: String,
    #[serde(default)]
    pub media_url: Option<String>,
    pub visibility: String,
    pub like_count: i64,
    pub reply_count: i64,
    pub repost_count: i64,
    pub liked_by_me: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
struct LikeResp {
    liked: bool,
    like_count: i64,
}

pub async fn list_feed(filter: &str) -> Result<Vec<FeedPostItem>, ApiError> {
    let f = urlencoding::encode(filter);
    let (_, text) = platform::http_request("GET", &format!("/api/feed?filter={f}"), None, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn create_post(
    content: &str,
    media_url: Option<&str>,
    visibility: &str,
    parent_id: Option<&str>,
    repost_of_id: Option<&str>,
) -> Result<String, ApiError> {
    let body = json!({
        "content": content,
        "media_url": media_url,
        "visibility": visibility,
        "parent_id": parent_id,
        "repost_of_id": repost_of_id,
    });
    let (_, text) = platform::http_request("POST", "/api/feed", Some(body.to_string()), None).await?;
    let v: Value = serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string())
}

pub async fn list_replies(post_id: &str) -> Result<Vec<FeedPostItem>, ApiError> {
    let id = urlencoding::encode(post_id);
    let (_, text) = platform::http_request("GET", &format!("/api/feed/{id}/replies"), None, None).await?;
    serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })
}

pub async fn toggle_like(post_id: &str) -> Result<(bool, i64), ApiError> {
    let id = urlencoding::encode(post_id);
    let (_, text) = platform::http_request("POST", &format!("/api/feed/{id}/like"), None, None).await?;
    let r: LikeResp = serde_json::from_str(&text).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    Ok((r.liked, r.like_count))
}

pub async fn delete_post(post_id: &str) -> Result<(), ApiError> {
    let id = urlencoding::encode(post_id);
    let _ = platform::http_request("DELETE", &format!("/api/feed/{id}"), None, None).await?;
    Ok(())
}
