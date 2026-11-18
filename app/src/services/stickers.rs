use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    models::{ApiError, StickerItem, StickerPackItem},
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

pub async fn stickers_list_packs() -> Result<Vec<StickerPackItem>, ApiError> {
    get_json("/api/stickers/packs").await
}

pub async fn stickers_create_pack(
    name: &str,
    description: Option<&str>,
    cover_url: Option<&str>,
) -> Result<StickerPackItem, ApiError> {
    post_json(
        "/api/stickers/packs",
        json!({
            "name": name,
            "description": description,
            "cover_url": cover_url,
        }),
    )
    .await
}

pub async fn stickers_add_sticker(
    pack_id: &str,
    name: &str,
    file_url: &str,
    is_animated: bool,
) -> Result<StickerItem, ApiError> {
    let enc_pid = urlencoding::encode(pack_id);
    post_json(
        &format!("/api/stickers/packs/{enc_pid}/stickers"),
        json!({
            "name": name,
            "file_url": file_url,
            "is_animated": is_animated,
        }),
    )
    .await
}
