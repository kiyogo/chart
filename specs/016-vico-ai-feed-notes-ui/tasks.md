# Tasks 016: Vico AI, Social Feed ("Mới"), Notes Strip & Command Icons

- [x] T001 Tạo migration `web/backend/migrations/0023_social_feed.sql` cho bảng `feed_posts` và `feed_likes`
- [x] T002 Tạo backend module `web/backend/src/feed/mod.rs`, đăng ký vào `lib.rs`, `main.rs`, `websocket/mod.rs` và viết test tích hợp cho `/api/feed`
- [x] T003 Cập nhật `app/src/components/icons.rs`: sửa `VicoAiIcon` dùng ID duy nhất và bổ sung bộ vector SVG icon cho từng lệnh (`/roll`, `/flip`, `/calc`, `/checkin`, `/checkout`, `/leave`, `/help`, `/craft`, `/coords`, `/bolt`...)
- [x] T004 Đổi toàn bộ `"Trợ lý AI"` sang `"Vico AI"` kèm `VicoAiIcon` trong `enterprise/mod.rs`, `enterprise/ops.rs`, `enterprise/settings.rs`, `services/enterprise.rs`
- [x] T005 Sửa triệt để gợi ý lệnh `/` trong `plugins_view.rs`, `composer.rs`, `pages/chat.rs`, `assets/main.css`: phân giải đúng icon riêng cho từng lệnh Plugin & Doanh nghiệp, xóa khung viền khi có logo/icon, tăng kích thước logo/icon và đẩy `.suggest-list` sát xuống thanh nhập chat
- [x] T006 Xóa chữ `"Plugin"` header trong `PluginCustomPageView` (`plugins_view.rs`) và xóa `.avatar-plugin-badge` trong `conversations.rs` + `plugins_view.rs`
- [x] T007 Xây dựng dải Ghi chú / Tạo tin cuộn ngang (`StoriesNotesBar`) trong `conversations.rs` + CSS `main.css` kèm cập nhật trạng thái real-time qua `UserPresenceUpdate` và hộp thoại tạo/trả lời tin
- [x] T008 Thêm `NavSection::Feed` ("Mới") và `NavSection::VicoAi` ("Vico AI") vào `navigation/mod.rs`, `rail.rs`, `sidebar.rs`, tạo `services/feed.rs`, `services/vico.rs`, `components/feed_view.rs`, `components/vico_ai_view.rs`, `web/backend/src/vico/mod.rs` + migration `0024_vico_chat.sql`, sửa full-bleed logo `VicoAiIcon`, sửa `.note-bubble` không vỡ dòng, bỏ tiêu đề lặp và giãn chuẩn khoảng cách khung gợi ý lệnh
- [ ] T009 Chạy kiểm thử backend & frontend (`cargo test`), build binary + web bundle (`app/build-web.sh`), kiểm tra daemon live `127.0.0.1:8085`, kiểm tra UI thực tế và push GitHub `origin main`
