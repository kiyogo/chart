//! Spec 009 client API: avatar motion, profile details with visibility,
//! status expiry and private contact nicknames / notes.

use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::{Value, json};

use crate::{
    components::avatar_motion::AvatarMotion,
    models::{ApiError, UserProfileResponse},
    platform,
};

/// My nickname / private note for each person: user id → (nickname, note).
pub static CONTACTS: GlobalSignal<HashMap<String, (Option<String>, Option<String>)>> = Signal::global(HashMap::new);

fn bad_json() -> ApiError {
    ApiError { code: "invalid_json".into(), status: 200 }
}

pub async fn fetch(user_id: &str) -> Result<Value, ApiError> {
    let (_, body) = platform::http_request("GET", &format!("/api/profile-ext/{}", urlencoding::encode(user_id)), None, None).await?;
    serde_json::from_str(&body).map_err(|_| bad_json())
}

/// Copy the extended fields onto a profile (ignores anything malformed).
pub fn merge_into(p: &mut UserProfileResponse, v: &Value) {
    p.avatar_motion = v.get("avatar_motion").and_then(|m| serde_json::from_value::<AvatarMotion>(m.clone()).ok()).filter(|m| m.is_valid());
    if let Some(d) = v.get("details") {
        p.details = serde_json::from_value(d.clone()).unwrap_or_default();
    }
    p.nickname = v.get("nickname").and_then(|x| x.as_str()).map(str::to_string);
    p.note = v.get("note").and_then(|x| x.as_str()).map(str::to_string);
    p.detail_visibility = v.get("detail_visibility").and_then(|x| serde_json::from_value(x.clone()).ok());
    p.status_expires_at = v.get("status_expires_at").and_then(|x| x.as_str()).map(str::to_string);
}

/// `fields` may hold avatar_motion, pronouns, location, website, occupation,
/// interests, detail_visibility and status_expires_in_minutes.
pub async fn update_me(fields: Value) -> Result<Value, ApiError> {
    let (_, body) = platform::http_request("PUT", "/api/profile-ext/me", Some(fields.to_string()), None).await?;
    serde_json::from_str(&body).map_err(|_| bad_json())
}

pub async fn set_motion(m: Option<&AvatarMotion>) -> Result<Value, ApiError> {
    update_me(json!({ "avatar_motion": m })).await
}

pub async fn load_contacts() -> Result<(), ApiError> {
    let (_, body) = platform::http_request("GET", "/api/contacts/notes", None, None).await?;
    let list: Vec<Value> = serde_json::from_str(&body).map_err(|_| bad_json())?;
    let map = list
        .into_iter()
        .filter_map(|v| {
            let id = v.get("user_id")?.as_str()?.to_string();
            let nick = v.get("nickname").and_then(|x| x.as_str()).map(str::to_string);
            let note = v.get("note").and_then(|x| x.as_str()).map(str::to_string);
            Some((id, (nick, note)))
        })
        .collect();
    *CONTACTS.write() = map;
    Ok(())
}

pub async fn save_note(user_id: &str, nickname: &str, note: &str) -> Result<(), ApiError> {
    let body = json!({ "nickname": nickname, "note": note }).to_string();
    platform::http_request("PUT", &format!("/api/contacts/{}/note", urlencoding::encode(user_id)), Some(body), None).await?;
    let nick = Some(nickname.trim().to_string()).filter(|s| !s.is_empty());
    let n = Some(note.trim().to_string()).filter(|s| !s.is_empty());
    CONTACTS.write().insert(user_id.to_string(), (nick, n));
    Ok(())
}

/// The name I chose for someone, if any.
pub fn nickname_of(user_id: &str) -> Option<String> {
    CONTACTS.read().get(user_id).and_then(|(n, _)| n.clone()).filter(|n| !n.is_empty())
}
