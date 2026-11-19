# Plan 016: Kiến trúc Triển khai Vico AI, Social Feed ("Mới"), Notes Strip & Command Suggestions

## 1. Kiến trúc Backend (`web/backend`)
- **Migration `0023_social_feed.sql` & `0024_vico_chat.sql`**:
  - Tạo bảng `feed_posts (id, author_id, parent_id, repost_of_id, content, media_url, visibility, created_at)` và `feed_likes (post_id, user_id, created_at)`.
  - Tạo bảng `vico_conversations`, `vico_messages`, `vico_user_configs` cho **Chat với Vico AI** độc lập hoàn toàn với AI trong Doanh nghiệp.
- **Module `web/backend/src/feed/mod.rs` & `web/backend/src/vico/mod.rs`**:
  - `/api/feed`: `GET`, `POST`, `DELETE`, `GET /{id}/replies`, `POST /{id}/like` kèm realtime `WsEvent::FeedUpdated`.
  - `/api/vico`: `GET/POST/DELETE /conversations`, `GET/POST /conversations/{id}/messages`, `GET/PUT /config` (tính toán nhanh, tóm tắt văn bản, giờ Việt Nam, giải đáp E2EE và kết nối tùy chọn OpenAI-compatible API).
- **WebSocket (`web/backend/src/websocket/mod.rs`)**:
  - Bổ sung `WsEvent::FeedUpdated(Value)` và đảm bảo `UserPresenceUpdate` cập nhật trực tiếp trạng thái & ghi chú cho bạn bè.

## 2. Kiến trúc Frontend (`app/`)
- **Icons (`app/src/components/icons.rs`)**:
  - Cập nhật `VicoAiIcon` dùng `AtomicUsize` qua `use_hook` để không bao giờ trùng gradient ID.
  - Bổ sung bộ icon vector chuẩn (`DiceIcon`, `CoinIcon`, `CalculatorIcon`, `TypeIcon` đã có, `LogInIcon`, `LogOutIcon`, `PlaneIcon`/`PalmtreeIcon`, `HelpCircleIcon`, `CubeIcon`, `BoltIcon`, `ActivityIcon`).
- **Command Suggestions & Plugin Icons (`app/src/components/plugins_view.rs`, `app/src/components/composer.rs`, `app/src/pages/chat.rs`, `app/assets/main.css`)**:
  - Tạo hàm phân giải icon theo từng lệnh `command_icon_and_logo(trigger, explicit_icon, plugin_icon, plugin_logo)` cho cả lệnh plugin và lệnh doanh nghiệp (`/roll` -> `svg:dice`, `/flip` -> `svg:coin`, `/time`/`/timer` -> `svg:clock`, `/calc` -> `svg:calc`, `/upper` -> `svg:upper`, `/help` -> `svg:help`, `/workspace` -> `svg:workspace`, `/task` -> `svg:task`, `/checkin` -> `svg:checkin`, `/checkout` -> `svg:checkout`, `/leave` -> `svg:leave`, `/meet` -> `svg:meet`, `/approve` -> `svg:approve`, `/announce` -> `svg:announce`, `/sound` -> giữ logo hoặc `svg:sound`, ...).
  - Trong `PluginGlyph`: hỗ trợ toàn bộ `svg:<command>` với màu nhấn ngữ nghĩa gọn đẹp.
  - Trong `composer.rs` & `main.css`: thêm class `.suggest-cmd-badge.has-logo` xóa background/border, tăng cỡ logo (`24px` cho `logo:`/`<svg>/url`, `20px` cho `svg:` icon) và kéo `.suggest-list` sát xuống `.composer-box`.
- **Xóa nhãn `"Plugin"` thừa**:
  - Bỏ badge `<span class="avatar-plugin-badge">Plugin</span>` trong `conversations.rs` và `plugins_view.rs`.
  - Bỏ tiêu đề `<h2>Plugin</h2>` trong `PluginCustomPageView` (chế độ `studio`) để header không còn khoảng trắng dư chữ "Plugin".
- **Notes Strip (`app/src/components/navigation/conversations.rs`, `app/assets/main.css`)**:
  - Component `StoriesNotesBar` đặt trên danh sách cuộc trò chuyện: bong bóng suy nghĩ `"Chia sẻ suy nghĩ"`, nút `+ Tạo tin`, danh sách bạn bè kèm bong bóng ghi chú, kéo ngang mượt mà bằng pointer drag + scroll wheel, modal chỉnh sửa/lưu ghi chú qua `api::users_update_profile` + `profile_ext::update_me`, click bạn bè để xem & trả lời nhanh qua DM.
- **Left Rail + Mới + Vico AI (`app/src/components/navigation/mod.rs`, `rail.rs`, `sidebar.rs`, `app/src/components/feed_view.rs`, `app/src/components/vico_ai_view.rs`, `app/src/services/feed.rs`, `app/src/pages/chat.rs`)**:
  - Thêm `NavSection::Feed` ("Mới") và `NavSection::VicoAi` ("Vico AI").
  - Cập nhật `NavigationSidebar`, `AppRail`, `Chat` và toàn bộ nhãn `"Trợ lý AI"` -> `"Vico AI"`.

## 3. Kiểm thử & Bàn giao
- `cargo test -p chatw` (integration + unit tests backend)
- `cargo test -p chatw-app --lib`
- `cargo build -p chatw --bin chatw` & `bash app/build-web.sh`
- Khởi động lại daemon `chatw` kiểm tra live trên `http://127.0.0.1:8085`
- `git add . && git commit && git push origin main`
