# Spec: Professional Overhaul of ChatW

## 1. Mục tiêu
Nâng cấp ChatW từ bản MVP hiện tại thành ứng dụng chat chuyên nghiệp, đa nền tảng, với giao diện Flat Modern, hệ thống nhóm kênh hoàn chỉnh, và nâng cao các tính năng nhắn tin/gọi thoại/hồ sơ cá nhân.

## 2. Các thay đổi chính
- **Design System**: Loại bỏ hoàn toàn hiệu ứng AI (glow, bóng giả), chuyển sang Flat Modern UI với màu sắc trung tính và khoảng cách hợp lý. Hỗ trợ Light/Dark mode chuẩn semantic tokens.
- **Group Channels**: Hỗ trợ danh mục (Categories) và đa loại kênh (Text, Voice, Video, Announcement, Forum, Media).
- **Profile System**: Nâng cấp hồ sơ cá nhân với ảnh bìa, bio, trạng thái, và nhiều tùy chọn cá nhân hóa.
- **Advanced Messaging**: Thêm Reaction, Poll, Event, Reminder, Stickers, và cải thiện quản lý tin nhắn (edit, pin, bookmark).
- **Settings & Notifications**: Hệ thống cài đặt chi tiết và thông báo linh hoạt.

## 3. Tiêu chí nghiệm thu (Acceptance Criteria)
- [ ] Xóa toàn bộ box-shadow, neon, glow trong CSS.
- [ ] Implement bộ CSS Variables semantic (colors, spacing, typography).
- [ ] Light mode và Dark mode nhất quán, không trộn lẫn.
- [ ] Giao diện cuộc gọi (Audio/Video) gọn gàng, không glow, tile responsive.
- [ ] Hệ thống nhóm có Categories & Channels hoạt động.
- [ ] Hồ sơ cá nhân đầy đủ tính năng.
- [ ] Kiểm thử E2E đạt 100% với Alice, Bob, Charlie trên môi trường thực tế.
- [ ] Không làm mất dữ liệu người dùng cũ.
- [ ] Không thay đổi giao thức E2EE cốt lõi.
