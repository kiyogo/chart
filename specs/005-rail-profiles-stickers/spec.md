# Spec 005 — Cột điều hướng trái, hồ sơ có hiệu ứng, sticker, tiện ích

## Yêu cầu người dùng
- Cột hẹp bên trái kiểu rail nhưng **không hiện server**; chứa mục điều hướng Tin nhắn, Bạn bè, Cộng đồng, Tìm nhanh.
- Trong Cài đặt có thể chọn ghim **cộng đồng** và/hoặc **nhóm** vào cột trái; các mục điều hướng vẫn luôn hiện.
- Hồ sơ phong phú kiểu Discord nhưng mang phong cách ChatW: màu chủ đề hồ sơ, khung avatar, hiệu ứng động, huy hiệu, ngày tham gia, popover khi bấm avatar.
- Sticker thật: bộ sticker ChatW gốc (SVG tự vẽ), tạo bộ riêng, tải ảnh sticker, gửi trong DM / nhóm / kênh.
- Tự đề xuất thêm tính năng hữu ích.

## Kiểm toán liên quan
- Seed `ChatW Essentials` lưu emoji vào `file_url` → ảnh sticker hỏng; cover trỏ `api.dicebear.com` bị CSP chặn.
- `PUT /api/users/me/profile` cho phép client tự đặt `badges` (leo thang đặc quyền hiển thị) và `custom_banner_url` tùy ý.
- Reaction DM: backend + WebSocket `reaction.update` đã có, UI chưa dùng.

## Tiêu chí nghiệm thu
1. Rail trái: mục có nhãn, badge chưa đọc/lời mời, menu tài khoản; mobile chuyển thành thanh tab dưới ở màn danh sách.
2. Cài đặt › Cột điều hướng: 2 công tắc ghim cộng đồng / ghim nhóm, lưu trên thiết bị, áp dụng tức thì.
3. Migration 0009: `profile_color`, `profile_accent`, `avatar_decoration`, `profile_effect` với CHECK; API trả thêm `member_since`.
4. Trình chỉnh hồ sơ có xem trước trực tiếp; popover hồ sơ khi bấm avatar trong tin nhắn; hiệu ứng tắt được và tôn trọng reduced-motion.
5. `POST /api/stickers/upload` (≤ 512 KB, PNG/WebP/GIF/AVIF, kiểm magic bytes); `DELETE` sticker/pack cho tác giả; quản lý sticker trong Cài đặt.
6. Sticker gửi được trong DM (bên trong bản mã E2EE), nhóm, kênh; hiển thị dạng ảnh.
7. Reaction DM realtime; bảng chuyển nhanh Ctrl+K; hình nền chat; tắt/bật âm thông báo.
8. Backend không nhận `badges` từ client; `custom_banner_url` phải là `/uploads/banners/…`.
9. cargo test + ui-audit đạt.
