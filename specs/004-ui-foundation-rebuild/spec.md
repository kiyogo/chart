# Spec 004 — UI Foundation Rebuild (Giai đoạn 1–4 của Master Prompt "ChatW Ultimate Rebuild")

## Bối cảnh (kết quả kiểm toán)
- Commit `b897523` viết lại `main.css` sang shell 4 cột (`workspace-shell`, `nav-rail`, `nav-sidebar`, `main-stage`) và tạo
  `NavigationRail`, `NavigationSidebar`, `SettingsView`, `CommunityModal`… nhưng `pages/chat.rs` **không render** các
  component này (chỉ import). Layout thực tế dùng class cũ (`sidebar`, `chat-view`, `sidebar-header`, `server-item`,
  `channel-item`, `voice-btn`…) **không có CSS** → giao diện thô, cột trái co lại, mobile bị đẩy lệch 191px.
- Đo bằng Playwright (artifacts/screenshots/before/results.json): bubble rộng tới 1540px ở 2560px; panel thông tin che
  chat ở 768px/900px; composer bị che ở 768px; settings 375px cắt 12 hàng; tin nhắn của chính mình hiển thị 2 lần
  (WebSocket echo + phản hồi API); Shift+Enter gửi tin thay vì xuống dòng (input nằm trong `<form>`).
- Sticker trong nhóm hiển thị như URL văn bản; emoji picker chỉ 24 ký tự.

## Mục tiêu giai đoạn này
Sửa lỗi nền tảng + kiến trúc UI + design system + chat UI, không thay stack, không đổi wire format backend, không
động vào giao thức E2EE.

## Tiêu chí nghiệm thu
1. Shell 4 cột thật: Rail (logo cộng đồng, DM, bạn bè, thêm cộng đồng) → Context sidebar (DM/Group DM, Bạn bè, kênh theo
   danh mục của cộng đồng) → Main → Details panel.
2. Details panel không bao giờ che chat ở ≥1280px (inline). Dưới 1280px là drawer có backdrop + nút đóng (overlay có chủ
   đích). Mobile (<768px): mỗi lần một màn hình, có nút quay lại.
3. Không có thanh cuộn ngang ở 14 viewport (320→3840). Header + composer luôn nằm trong viewport và không bị che.
4. Bubble: chiều rộng tối đa theo độ đọc (≈ 68ch, ≤ 720px), URL dài xuống dòng, gom nhóm tin liên tiếp, phân cách ngày,
   giờ gọn trong bubble, font tin nhắn ≥ 14px.
5. Composer: textarea tự giãn, Enter gửi, Shift+Enter xuống dòng, emoji picker có tìm kiếm + danh mục + gần đây.
6. Không nhân đôi tin nhắn tự gửi.
7. Design tokens: light / dark / system, accent color, cỡ chữ, mật độ (compact/comfortable/spacious) — lưu localStorage,
   áp dụng bằng biến CSS (không dùng `zoom`).
8. Trang đăng nhập thiết kế lại; đăng nhập thử nghiệm chỉ hiện khi backend báo `dev=true` (chỉ cho phép với PUBLIC_URL
   loopback — đã có ở `config.rs`).
9. Settings: nhiều mục, mobile dạng danh sách → trang con, vân tay khóa không bị cắt.
10. Sticker hiển thị dạng ảnh trong nhóm/kênh.
11. Bằng chứng: script `app/e2e/ui-audit.mjs` chạy trên backend thật với tài khoản thật, ảnh chụp trước/sau trong
    `artifacts/screenshots/`.

## Ngoài phạm vi (ghi rõ trong artifacts/feature_matrix.md)
Thread, poll, scheduled/disappearing messages, SFU, screen share, multi-account, command palette, GIF, server settings đầy
đủ, audit log UI, E2EE cho nhóm/kênh, virtual scrolling, v.v.
