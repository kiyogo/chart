# Tasks 017: Discord Server Panel, Search & UI Polish

- [x] T001 Mở rộng backend `WsHub::online_user_ids()`, `ServerMemberItem` (presence, status, nameplate, name_effect) và thêm endpoint `GET /api/servers/{id}/search` kèm test tích hợp
- [x] T002 Sửa `VicoAiView` (`vico_ai_view.rs` + `main.css`): mọi avatar Vico AI (header, welcome sau xóa/tạo mới, message stream) dạng hình tròn `50%` full logo và cho phép xóa làm mới phiên chat
- [x] T003 Xóa thanh lọc `Tất cả / Bạn bè / Của tôi` & nút refresh trong `FeedView` (`feed_view.rs`)
- [x] T004 Tắt hoàn toàn hiệu ứng hover & viền xanh focus trên tất cả avatar và nút icon (`main.css`, `profile_card.rs`)
- [x] T005 Xây dựng `ServerMembersPanel` & `ServerHeaderTools` (ô `Tìm kiếm {ServerName}` + nút `PanelRightIcon`) phong cách Discord trong `server_members_panel.rs` và tích hợp vào cả Máy chủ thường lẫn Không gian Doanh nghiệp (`chat.rs`, `enterprise/mod.rs`)
- [ ] T006 Xóa các chữ `"Lời mời kết bạn"`, `"Không có lời mời nào"`, `"Tất cả bạn bè"` trong `sidebar.rs` và cân đều khoảng cách danh sách Bạn bè (`main.css`)
- [ ] T007 Xóa thanh tiêu đề `"Thông tin"` (`.details-header`) trong `details_panel.rs` để ảnh bìa hiển thị full từ mép trên
- [ ] T008 Chạy `cargo test`, build backend + web bundle, kiểm tra daemon live `127.0.0.1:8085` bằng trình duyệt và đẩy code lên GitHub `origin main`
