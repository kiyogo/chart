use chatw_core::{
    FriendItem, MeResponse, MessageWireItem, PendingRequestItem, PublicKeysResponse,
    UserSearchResult,
};
use reqwest::cookie::Jar;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct HttpClient {
    pub base_url: String,
    client: reqwest::Client,
    pub cookie_jar: Arc<Jar>,
}

impl HttpClient {
    pub fn new(base_url: String) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client build");

        Self {
            base_url,
            client,
            cookie_jar,
        }
    }

    pub async fn dev_login(&self, username: Option<&str>) -> Result<(), String> {
        let url = format!("{}/api/auth/dev-login", self.base_url);
        let body = serde_json::json!({
            "username": username
        });
        let res = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Lỗi kết nối máy chủ: {e}"))?;

        if !res.status().is_success() {
            return Err(format!("Đăng nhập thất bại (HTTP {})", res.status()));
        }
        Ok(())
    }

    pub async fn get_me(&self) -> Result<MeResponse, String> {
        let url = format!("{}/api/auth/me", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi kết nối: {e}"))?;

        if !res.status().is_success() {
            return Err("Chưa đăng nhập".into());
        }

        res.json::<MeResponse>()
            .await
            .map_err(|e| format!("Lỗi giải mã thông tin tài khoản: {e}"))
    }

    pub async fn register_keys(&self, ecdh_b64: &str, ecdsa_b64: &str) -> Result<(), String> {
        let url = format!("{}/api/keys", self.base_url);
        let body = serde_json::json!({
            "ecdh_public": ecdh_b64,
            "ecdsa_public": ecdsa_b64,
        });
        let res = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Lỗi đăng ký khóa: {e}"))?;

        if !res.status().is_success() {
            return Err(format!("Đăng ký khóa thất bại (HTTP {})", res.status()));
        }
        Ok(())
    }

    pub async fn set_name(&self, name: &str) -> Result<(), String> {
        let url = format!("{}/api/users/name", self.base_url);
        let body = serde_json::json!({ "name": name });
        let res = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Lỗi đặt tên: {e}"))?;

        if !res.status().is_success() {
            let err_txt = res.text().await.unwrap_or_default();
            if err_txt.contains("username_taken") {
                return Err("Tên tài khoản này đã có người sử dụng".into());
            }
            return Err("Không thể đặt tên tài khoản".into());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn search_users(&self, query: &str) -> Result<Vec<UserSearchResult>, String> {
        let url = format!(
            "{}/api/users/search?q={}",
            self.base_url,
            urlencoding::encode(query)
        );
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tìm kiếm: {e}"))?;

        if !res.status().is_success() {
            return Err("Lỗi tìm kiếm người dùng".into());
        }
        res.json::<Vec<UserSearchResult>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_user_keys(&self, user_id: Uuid) -> Result<PublicKeysResponse, String> {
        let url = format!("{}/api/keys/{}", self.base_url, user_id);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi lấy khóa công khai: {e}"))?;

        if !res.status().is_success() {
            return Err("Người dùng này chưa đăng ký khóa mã hóa E2EE".into());
        }
        res.json::<PublicKeysResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_friends(&self) -> Result<Vec<FriendItem>, String> {
        let url = format!("{}/api/friends", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải danh bạ bạn bè: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể tải danh sách bạn bè".into());
        }
        res.json::<Vec<FriendItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_pending_requests(&self) -> Result<Vec<PendingRequestItem>, String> {
        let url = format!("{}/api/friends/requests", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải lời mời kết bạn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể tải lời mời kết bạn".into());
        }
        res.json::<Vec<PendingRequestItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn send_friend_request(&self, username: &str) -> Result<(), String> {
        let url = format!("{}/api/friends/requests", self.base_url);
        let body = serde_json::json!({ "username": username });
        let res = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Lỗi gửi yêu cầu kết bạn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể gửi lời mời kết bạn".into());
        }
        Ok(())
    }

    pub async fn accept_friend_request(&self, request_id: Uuid) -> Result<(), String> {
        let url = format!(
            "{}/api/friends/requests/{}/accept",
            self.base_url, request_id
        );
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi đồng ý kết bạn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể chấp nhận kết bạn".into());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn reject_friend_request(&self, _request_id: Uuid) -> Result<(), String> {
        // Backend handles request clearance upon mutual actions
        Ok(())
    }

    pub async fn get_message_history(
        &self,
        friend_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageWireItem>, String> {
        let url = format!(
            "{}/api/messages/{}?limit={}",
            self.base_url, friend_id, limit
        );
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải lịch sử tin nhắn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể tải lịch sử tin nhắn".into());
        }
        res.json::<Vec<MessageWireItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn send_message(
        &self,
        message_id: Uuid,
        recipient_id: Uuid,
        recipient_envelope: &str,
        sender_envelope: &str,
    ) -> Result<(), String> {
        let url = format!("{}/api/messages", self.base_url);
        let payload = serde_json::json!({
            "message_id": message_id,
            "recipient_id": recipient_id,
            "recipient_envelope": recipient_envelope,
            "sender_envelope": sender_envelope,
        });
        let res = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Lỗi gửi tin nhắn: {e}"))?;

        if !res.status().is_success() {
            return Err(format!("Gửi tin nhắn thất bại (HTTP {})", res.status()));
        }
        Ok(())
    }

    pub fn set_session_token(&self, token: &str) {
        let cookie_str = format!("chatw_session={token}; Path=/; HttpOnly");
        if let Ok(url) = self.base_url.parse::<reqwest::Url>() {
            self.cookie_jar.add_cookie_str(&cookie_str, &url);
        }
    }

    pub async fn create_oauth_ticket(&self) -> Result<Uuid, String> {
        let url = format!("{}/api/auth/oauth-ticket", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tạo phiên xác thực: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể tạo phiên xác thực".into());
        }

        let val = res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;
        let ticket_str = val["ticket"].as_str().ok_or("Thiếu mã xác thực")?;
        Uuid::parse_str(ticket_str).map_err(|e| e.to_string())
    }

    pub async fn check_oauth_ticket(&self, ticket: Uuid) -> Result<Option<String>, String> {
        let url = format!("{}/api/auth/oauth-ticket/{}", self.base_url, ticket);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi kiểm tra xác thực: {e}"))?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let val = res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;
        if val["ready"].as_bool().unwrap_or(false) {
            if let Some(tok) = val["token"].as_str() {
                return Ok(Some(tok.to_string()));
            }
        }
        Ok(None)
    }

    pub async fn logout(&self) -> Result<(), String> {
        let url = format!("{}/api/auth/logout", self.base_url);
        let _ = self.client.post(&url).send().await;
        Ok(())
    }
}
