# Tasks: Professional Overhaul of ChatW

## Phase 1: CSS System & Cleanup
- [x] Định nghĩa `app/assets/theme.css` với đầy đủ semantic tokens Light/Dark mode.
- [x] Audit `app/assets/main.css` & các component, sửa lỗi thiếu token, sửa `.auth-container`, thêm `.modal-overlay` và 4-column Discord layout.
- [x] Áp dụng `theme.css` và lưu theme Light/Dark toàn hệ thống.

## Phase 2: Backend Foundations
- [x] Migration: Categories & Channels (`0008_group_channels_and_features.sql`).
- [x] Migration: Profiles & Stickers.
- [x] Cập nhật API Handler: Categories, Channels, Reactions, Stickers, Profiles, Banner upload (`/api/users/banner`), Server discovery & direct join (`/api/servers/discover`, `/api/servers/{id}/join`), default Voice category/channel on server create.

## Phase 3: Modular Architecture & Discord 4-Column UI
- [ ] Tách `app/src/components/navigation/` (`rail.rs`, `sidebar.rs`, `modals.rs`).
- [ ] Thanh logo bên trái (Rail A 68px) hiển thị DMs, Friends, icon từng cộng đồng có tooltip, active pill, badge, nút thêm/khám phá.
- [ ] Sidebar thứ hai (Sidebar B 260px) phân biệt rõ DM 1-1 / Nhóm DM và Cộng đồng (danh mục dọc thu gọn được + kênh dọc `#text`, `voice`, `announcement`, bỏ thanh kênh ngang cũ).
- [ ] Sidebar phải (Column D) hiển thị thành viên server / thông tin đối phương / thông tin nhóm không che khung chat trên desktop.
- [ ] Trang Cài đặt đầy đủ 2 cột (`settings_view.rs`): Tài khoản & Hồ sơ (Avatar + Banner), Giao diện Light/Dark, Bảo mật & Sao lưu/Khôi phục khóa E2EE.

## Phase 4: E2EE Hardening & Rich Messaging
- [x] Sửa lỗi rò rỉ khóa giữa các tài khoản (`get_user_keys`) và đồng bộ khóa server khi đăng nhập (`lib.rs`).
- [x] Phân loại lỗi giải mã (`DecryptError`: `WrongDeviceKey`, `SignatureMismatch`, `Corrupted`) + tự động thử lại khi đối phương đổi khóa + sao lưu/khôi phục khóa phía client.
- [ ] Hiển thị Sticker dạng hình ảnh thực, ghi âm & phát Voice Message thật qua `/api/attachments`, cập nhật `ProfileModal` hỗ trợ tải ảnh bìa & ảnh đại diện.

## Phase 5: Multi-Account Verification, Responsive Screenshots & Push
- [ ] Chạy `cargo test --workspace`, `cargo build --release`, WASM release build.
- [ ] Kiểm thử đa tài khoản (Alice, Bob, Charlie): DM E2EE, Server/Category/Channel, Voice channel, Group DM, Sticker, Reaction, Profile/Banner, Permissions.
- [ ] Chụp bộ ảnh Playwright trên desktop (1280x800, 1920x1080), tablet (768x1024), mobile (375x667), Light & Dark mode, cập nhật `screenshots_gallery.md`.
- [ ] Commit và push lên `origin main`.
