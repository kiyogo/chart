# Plan 017: Architecture & Implementation Plan

## 1. Backend (`web/backend`)
- `src/websocket/mod.rs`: Thêm `WsHub::online_user_ids()` trả về `HashSet<Uuid>` các user đang kết nối WebSocket.
- `src/models.rs`: Mở rộng `ServerMemberItem` với các trường tùy chọn (`display_name`, `presence`, `status_text`, `status_emoji`, `nameplate`, `custom_nameplate_url`, `name_effect`, `avatar_decoration`) để tương thích ngược.
- `src/servers/mod.rs`:
  - Cập nhật `handle_list_members` join đủ thông tin hồ sơ + tính toán `presence` thực tế từ `users.presence` và `ws_hub.online_user_ids()`.
  - Thêm route `GET /api/servers/{id}/search?q=...` tìm kiếm tin nhắn trong các kênh thuộc máy chủ (có kiểm tra quyền `VIEW_CHANNELS` & private channel).
- `tests/integration_test.rs`: Bổ sung test cho `GET /api/servers/{id}/search` và metadata thành viên trả về.

## 2. Frontend Services & Models (`app/src`)
- `src/models.rs`: Cập nhật `ServerMemberItem` + `ServerMessageHit` cho kết quả tìm kiếm tin nhắn máy chủ.
- `src/services/api.rs`: Thêm `servers_search_messages(server_id, q)`.
- `src/components/vico_ai_view.rs`:
  - Bọc `VicoAiIcon { full: true }` trong `.vico-round-avatar` / `.vico-msg-avatar` hình tròn (`border-radius: 50%`).
  - Luôn hiển thị nút xóa cuộc trò chuyện (`TrashIcon`); nếu xóa cuộc trò chuyện cuối cùng, tải lại `list_conversations()` để tự tạo phiên mới sạch với avatar welcome tròn.
- `src/components/feed_view.rs`: Xóa `.feed-top-bar` (`Tất cả`, `Bạn bè`, `Của tôi`, `RefreshIcon`) và dùng `"all"` mặc định.
- `src/components/profile_card.rs` & `app/assets/main.css`:
  - Loại bỏ toàn bộ `:hover` transition/transform/scale/animation trên avatar (`.note-item`, `.avm-t-hover:hover`, `.user-avatar`, `.feed-avatar-btn`, `.message-avatar-slot`, `.rail-account`).
  - Tắt `:focus-visible` outline xanh cứng cho các nút avatar, icon-btn, note-item, member row.
- Tạo `src/components/server_members_panel.rs`:
  - `ServerMembersPanel`: bảng thành viên bên phải kiểu Discord (nhóm theo vai trò + ngoại tuyến, nameplate, avatar + chấm trạng thái không hiệu ứng hover, màu tên theo vai trò, status subtitle, click mở hồ sơ).
  - `ServerHeaderTools`: cụm công cụ góc phải header dùng chung cho cả kênh máy chủ lẫn `EnterpriseWorkspace` gồm ô `Tìm kiếm {server_name}` (với dropdown kết quả thực tế: Thành viên, Tin nhắn kênh, và Dữ liệu doanh nghiệp) + nút toggle `PanelRightIcon` đóng/mở `ServerMembersPanel`.
- `src/components/enterprise/mod.rs` & `src/pages/chat.rs`:
  - Tải `server_members` & `server_roles` khi chọn máy chủ và cập nhật realtime khi nhận `UserPresenceUpdate` / `ProfileUpdated`.
  - Gắn `ServerHeaderTools` & `ServerMembersPanel` đồng nhất vào cả header kênh máy chủ lẫn header `EnterpriseWorkspace`.

## 3. Verification
- `cargo test -p chatw-app --lib`
- `cargo test -p chatw -- --test-threads=1`
- `cargo build -p chatw --bin chatw && bash app/build-web.sh`
- Khởi động lại daemon live tại `127.0.0.1:8085`, kiểm tra toàn bộ giao diện qua `browser` subagent, sau đó commit & push GitHub (`origin main`).
