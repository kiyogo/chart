//! Spec 012 client API: profile layout, presets, showcase, guestbook,
//! achievements, mutual friends and shared communities.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{models::ApiError, platform};

pub const SECTIONS: [(&str, &str); 9] = [
    ("about", "Giới thiệu"),
    ("details", "Thông tin"),
    ("badges", "Huy hiệu"),
    ("showcase", "Khu trưng bày"),
    ("achievements", "Thành tích"),
    ("guestbook", "Sổ lưu bút"),
    ("communities", "Cộng đồng chung"),
    ("mutual", "Bạn chung"),
    ("links", "Liên kết"),
];

/// (id, English template name)
pub const TEMPLATES: [(&str, &str); 21] = [
    ("minimal", "Minimal Profile"),
    ("modern", "Modern Profile"),
    ("anime", "Anime Profile"),
    ("gaming", "Gaming Profile"),
    ("developer", "Developer Profile"),
    ("creator", "Creator Profile"),
    ("artist", "Artist Profile"),
    ("music", "Music Profile"),
    ("retro", "Retro Profile"),
    ("pixel", "Pixel Profile"),
    ("neon", "Neon Profile"),
    ("glass", "Glass Profile"),
    ("elegant", "Elegant Profile"),
    ("dark", "Dark Profile"),
    ("light", "Light Profile"),
    ("pastel", "Pastel Profile"),
    ("nature", "Nature Profile"),
    ("cyber", "Cyber Profile"),
    ("vintage", "Vintage Profile"),
    ("professional", "Professional Profile"),
    ("community", "Community Profile"),
];

/// (id, title, description)
pub const ACHIEVEMENTS: [(&str, &str, &str); 15] = [
    ("first_message", "Lời chào đầu tiên", "Gửi tin nhắn đầu tiên"),
    ("chatterbox", "Người trò chuyện", "Gửi 100 tin nhắn"),
    ("first_friend", "Người bạn đầu tiên", "Kết bạn lần đầu"),
    ("social", "Hòa đồng", "Có 10 người bạn"),
    ("group_founder", "Người lập nhóm", "Tạo một nhóm"),
    ("community_member", "Thành viên cộng đồng", "Tham gia một cộng đồng"),
    ("sticker_maker", "Nhà làm sticker", "Tạo bộ sticker có ít nhất một sticker"),
    ("artist", "Họa sĩ", "Lưu một dự án trong ChatW Studio"),
    ("animator", "Nhà làm hoạt hình", "Lưu hoạt ảnh từ 8 khung hình"),
    ("secure", "Khóa chặt", "Bật xác minh hai bước"),
    ("stylist", "Phong cách", "Dùng hiệu ứng avatar, hiệu ứng tên hoặc bảng tên"),
    ("storyteller", "Người kể chuyện", "Điền ít nhất 4 mục hồ sơ"),
    ("bot_builder", "Thợ làm plugin", "Tạo một plugin"),
    ("merchant", "Chủ cửa hàng", "Bật trang Kinh doanh"),
    ("night_owl", "Cú đêm", "Thành tích ẩn"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub visible: bool,
    #[serde(default = "normal")]
    pub size: String,
}

fn normal() -> String {
    "normal".into()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ShowItem {
    pub id: String,
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub progress: i64,
    pub goal: i64,
    #[serde(default)]
    pub hidden: bool,
    pub unlocked: bool,
    #[serde(default)]
    pub unlocked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Entry {
    pub id: String,
    pub author_id: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub body: String,
    pub at: String,
    #[serde(default)]
    pub can_delete: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Community {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProfilePlus {
    pub template: String,
    pub sections: Vec<Section>,
    pub guestbook_mode: String,
    #[serde(default)]
    pub showcase: Vec<ShowItem>,
    #[serde(default)]
    pub achievements: Vec<Achievement>,
    #[serde(default)]
    pub guestbook: Vec<Entry>,
    #[serde(default)]
    pub can_sign_guestbook: bool,
    #[serde(default)]
    pub mutual_friends: Vec<String>,
    #[serde(default)]
    pub shared_communities: Vec<Community>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub template: String,
    pub sections: Vec<Section>,
}

pub fn default_sections() -> Vec<Section> {
    SECTIONS.iter().map(|(id, _)| Section { id: id.to_string(), visible: true, size: "normal".into() }).collect()
}

pub fn section_label(id: &str) -> &'static str {
    SECTIONS.iter().find(|s| s.0 == id).map(|s| s.1).unwrap_or("Mục")
}

pub fn template_name(id: &str) -> &'static str {
    TEMPLATES.iter().find(|t| t.0 == id).map(|t| t.1).unwrap_or("Minimal Profile")
}

fn bad() -> ApiError {
    ApiError { code: "invalid_json".into(), status: 200 }
}

async fn call(method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiError> {
    let (_, text) = platform::http_request(method, path, body.map(|b| b.to_string()), None).await?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| bad())
}

pub async fn get(user_id: &str) -> Result<ProfilePlus, ApiError> {
    serde_json::from_value(call("GET", &format!("/api/profile-plus/{}", urlencoding::encode(user_id)), None).await?).map_err(|_| bad())
}

pub async fn save_layout(template: &str, sections: &[Section], guestbook_mode: &str) -> Result<(), ApiError> {
    call("PUT", "/api/profile-plus/me/layout", Some(json!({ "template": template, "sections": sections, "guestbook_mode": guestbook_mode }))).await.map(|_| ())
}

pub async fn presets() -> Result<Vec<Preset>, ApiError> {
    serde_json::from_value(call("GET", "/api/profile-plus/me/presets", None).await?).map_err(|_| bad())
}

pub async fn save_preset(name: &str, template: &str, sections: &[Section]) -> Result<(), ApiError> {
    call("POST", "/api/profile-plus/me/presets", Some(json!({ "name": name, "template": template, "sections": sections }))).await.map(|_| ())
}

pub async fn delete_preset(id: &str) -> Result<(), ApiError> {
    call("DELETE", &format!("/api/profile-plus/me/presets/{}", urlencoding::encode(id)), None).await.map(|_| ())
}

pub async fn add_showcase(kind: &str, url: &str, title: &str) -> Result<(), ApiError> {
    call("POST", "/api/profile-plus/me/showcase", Some(json!({ "kind": kind, "url": url, "title": title }))).await.map(|_| ())
}

pub async fn order_showcase(ids: &[String], pinned: &[String]) -> Result<(), ApiError> {
    call("PUT", "/api/profile-plus/me/showcase/order", Some(json!({ "ids": ids, "pinned": pinned }))).await.map(|_| ())
}

pub async fn delete_showcase(id: &str) -> Result<(), ApiError> {
    call("DELETE", &format!("/api/profile-plus/me/showcase/{}", urlencoding::encode(id)), None).await.map(|_| ())
}

pub async fn sign(user_id: &str, body: &str) -> Result<(), ApiError> {
    call("POST", &format!("/api/profile-plus/{}/guestbook", urlencoding::encode(user_id)), Some(json!({ "body": body }))).await.map(|_| ())
}

pub async fn delete_entry(id: &str) -> Result<(), ApiError> {
    call("DELETE", &format!("/api/profile-plus/guestbook/{}", urlencoding::encode(id)), None).await.map(|_| ())
}

/// Layout as a portable JSON file (export / import / share a template).
pub fn export_layout(template: &str, sections: &[Section]) -> String {
    json!({ "chatw_profile_layout": 1, "template": template, "sections": sections }).to_string()
}

pub fn import_layout(text: &str) -> Option<(String, Vec<Section>)> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("chatw_profile_layout").and_then(|x| x.as_i64()) != Some(1) {
        return None;
    }
    let t = v.get("template")?.as_str()?.to_string();
    if !TEMPLATES.iter().any(|x| x.0 == t) {
        return None;
    }
    let mut s: Vec<Section> = serde_json::from_value(v.get("sections")?.clone()).ok()?;
    s.retain(|x| SECTIONS.iter().any(|k| k.0 == x.id) && (x.size == "normal" || x.size == "wide"));
    for (id, _) in SECTIONS {
        if !s.iter().any(|x| x.id == id) {
            s.push(Section { id: id.into(), visible: true, size: "normal".into() });
        }
    }
    Some((t, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trip_and_rejects_junk() {
        let mut s = default_sections();
        s.swap(0, 5);
        s[1].visible = false;
        let text = export_layout("neon", &s);
        let (t, back) = import_layout(&text).unwrap();
        assert_eq!(t, "neon");
        assert_eq!(back, s);
        assert!(import_layout(r#"{"chatw_profile_layout":1,"template":"evil","sections":[]}"#).is_none());
        let (_, filled) = import_layout(r#"{"chatw_profile_layout":1,"template":"pixel","sections":[{"id":"x","visible":true}]}"#).unwrap();
        assert_eq!(filled.len(), SECTIONS.len());
    }
}
