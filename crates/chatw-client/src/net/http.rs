use chatw_core::{
    AttachmentItem, AuditLogItem, ChannelItem, ChannelMessageItem, CreateCategoryRequest,
    CreateChannelRequest, CreateServerRequest, FriendItem, MeResponse, MessageWireItem,
    PendingRequestItem, PostChannelMessageRequest, PublicKeysResponse, ServerCategoryItem,
    ServerItem, ServerMemberItem, UpdateProfileRequest, UpdateVoiceStateRequest,
    UserProfileResponse, UserSearchResult, VoiceStateItem, WebRtcSignalPayload,
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

    pub async fn toggle_reaction(
        &self,
        message_id: Uuid,
        emoji: &str,
    ) -> Result<Vec<chatw_core::ReactionSummary>, String> {
        let url = format!("{}/api/messages/{}/reactions", self.base_url, message_id);
        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "emoji": emoji }))
            .send()
            .await
            .map_err(|e| format!("Lỗi cập nhật biểu cảm: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể cập nhật biểu cảm".into());
        }

        res.json::<Vec<chatw_core::ReactionSummary>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn pin_message(&self, message_id: Uuid, pin: bool) -> Result<(), String> {
        let url = format!("{}/api/messages/{}/pin", self.base_url, message_id);
        let req = if pin {
            self.client.post(&url)
        } else {
            self.client.delete(&url)
        };
        let res = req
            .send()
            .await
            .map_err(|e| format!("Lỗi ghim tin nhắn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể ghim tin nhắn".into());
        }
        Ok(())
    }

    pub async fn delete_message(&self, message_id: Uuid) -> Result<(), String> {
        let url = format!("{}/api/messages/{}", self.base_url, message_id);
        let res = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi xóa tin nhắn: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể xóa tin nhắn".into());
        }
        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Vec<chatw_core::SessionItem>, String> {
        let url = format!("{}/api/auth/sessions", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi lấy danh sách phiên: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể lấy danh sách phiên".into());
        }

        res.json::<Vec<chatw_core::SessionItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<(), String> {
        let url = format!("{}/api/auth/sessions/{}", self.base_url, session_id);
        let res = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi đăng xuất thiết bị: {e}"))?;

        if !res.status().is_success() {
            return Err("Không thể đăng xuất thiết bị".into());
        }
        Ok(())
    }

    pub async fn list_servers(&self) -> Result<Vec<ServerItem>, String> {
        let url = format!("{}/api/servers", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải danh sách server: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải danh sách server".into());
        }
        res.json::<Vec<ServerItem>>().await.map_err(|e| e.to_string())
    }

    pub async fn create_server(&self, req: &CreateServerRequest) -> Result<ServerItem, String> {
        let url = format!("{}/api/servers", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi tạo server: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tạo server".into());
        }
        res.json::<ServerItem>().await.map_err(|e| e.to_string())
    }

    pub async fn list_channels(&self, server_id: Uuid) -> Result<Vec<ChannelItem>, String> {
        let url = format!("{}/api/servers/{server_id}/channels", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải kênh: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải danh sách kênh".into());
        }
        res.json::<Vec<ChannelItem>>().await.map_err(|e| e.to_string())
    }

    pub async fn create_channel(
        &self,
        server_id: Uuid,
        req: &CreateChannelRequest,
    ) -> Result<ChannelItem, String> {
        let url = format!("{}/api/servers/{server_id}/channels", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi tạo kênh: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tạo kênh".into());
        }
        res.json::<ChannelItem>().await.map_err(|e| e.to_string())
    }

    pub async fn create_category(
        &self,
        server_id: Uuid,
        req: &CreateCategoryRequest,
    ) -> Result<ServerCategoryItem, String> {
        let url = format!("{}/api/servers/{server_id}/categories", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi tạo danh mục: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tạo danh mục".into());
        }
        res.json::<ServerCategoryItem>().await.map_err(|e| e.to_string())
    }

    pub async fn create_invite(&self, server_id: Uuid) -> Result<String, String> {
        let url = format!("{}/api/servers/{server_id}/invites", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tạo lời mời: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tạo lời mời".into());
        }
        let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        v["code"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Mã mời không hợp lệ".to_string())
    }

    pub async fn join_server(&self, invite_code: &str) -> Result<ServerItem, String> {
        let url = format!("{}/api/servers/join/{invite_code}", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tham gia server: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tham gia server".into());
        }
        res.json::<ServerItem>().await.map_err(|e| e.to_string())
    }

    pub async fn list_server_members(
        &self,
        server_id: Uuid,
    ) -> Result<Vec<ServerMemberItem>, String> {
        let url = format!("{}/api/servers/{server_id}/members", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi lấy danh sách thành viên: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải thành viên server".into());
        }
        res.json::<Vec<ServerMemberItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_channel_messages(
        &self,
        channel_id: Uuid,
    ) -> Result<Vec<ChannelMessageItem>, String> {
        let url = format!("{}/api/channels/{channel_id}/messages", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải tin nhắn kênh: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải tin nhắn kênh".into());
        }
        res.json::<Vec<ChannelMessageItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn post_channel_message(
        &self,
        channel_id: Uuid,
        req: &PostChannelMessageRequest,
    ) -> Result<ChannelMessageItem, String> {
        let url = format!("{}/api/channels/{channel_id}/messages", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi gửi tin nhắn kênh: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể gửi tin nhắn kênh".into());
        }
        res.json::<ChannelMessageItem>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_profile(
        &self,
        req: &UpdateProfileRequest,
    ) -> Result<UserProfileResponse, String> {
        let url = format!("{}/api/users/profile", self.base_url);
        let res = self
            .client
            .put(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi cập nhật hồ sơ: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể cập nhật hồ sơ".into());
        }
        res.json::<UserProfileResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<UserProfileResponse, String> {
        let url = format!("{}/api/users/{user_id}/profile", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi lấy thông tin hồ sơ: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể lấy hồ sơ người dùng".into());
        }
        res.json::<UserProfileResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn webrtc_signal(&self, req: &WebRtcSignalPayload) -> Result<(), String> {
        let url = format!("{}/api/webrtc/signal", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi gửi tín hiệu WebRTC: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể gửi tín hiệu WebRTC".into());
        }
        Ok(())
    }

    pub async fn join_voice(&self, channel_id: Uuid) -> Result<Vec<VoiceStateItem>, String> {
        let url = format!("{}/api/webrtc/voice/{channel_id}/join", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tham gia thoại: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tham gia phòng thoại".into());
        }
        res.json::<Vec<VoiceStateItem>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_voice_state(
        &self,
        channel_id: Uuid,
        req: &UpdateVoiceStateRequest,
    ) -> Result<VoiceStateItem, String> {
        let url = format!("{}/api/webrtc/voice/{channel_id}/state", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Lỗi cập nhật thoại: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể cập nhật thoại".into());
        }
        res.json::<VoiceStateItem>().await.map_err(|e| e.to_string())
    }

    pub async fn leave_voice(&self, channel_id: Uuid) -> Result<(), String> {
        let url = format!("{}/api/webrtc/voice/{channel_id}/leave", self.base_url);
        let res = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi rời phòng thoại: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể rời phòng thoại".into());
        }
        Ok(())
    }

    pub async fn upload_attachment(
        &self,
        file_name: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<AttachmentItem, String> {
        let url = format!("{}/api/attachments", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("x-file-name", file_name)
            .header("content-type", content_type)
            .body(data)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải tệp lên: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải tệp lên".into());
        }
        res.json::<AttachmentItem>().await.map_err(|e| e.to_string())
    }

    pub async fn get_audit_logs(&self) -> Result<Vec<AuditLogItem>, String> {
        let url = format!("{}/api/audit/logs", self.base_url);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Lỗi tải nhật ký kiểm toán: {e}"))?;
        if !res.status().is_success() {
            return Err("Không thể tải nhật ký".into());
        }
        res.json::<Vec<AuditLogItem>>().await.map_err(|e| e.to_string())
    }
}
