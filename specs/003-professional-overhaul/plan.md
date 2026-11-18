# Plan: Professional Overhaul of ChatW

## 1. Giai đoạn thiết kế (CSS System)
- Định nghĩa hệ thống biến CSS `semantic tokens` (Light/Dark mode).
- Tạo `app/assets/theme.css` mới.
- Audit và loại bỏ toàn bộ CSS vi phạm trong `app/assets/main.css` và các component.

## 2. Giai đoạn Backend (Data Models & Migrations)
- Tạo migrations cho Categories, Channels, Profile details, Stickers, Polls.
- Cập nhật backend models và API handlers.

## 3. Giai đoạn Frontend (Components)
- Xây dựng component hệ thống (Global Layout, Modal, Popover).
- Xây dựng lại cấu trúc Nhóm/Kênh UI.
- Xây dựng Profile System.
- Xây dựng Sticker System.
- Xây dựng lại UI Cuộc gọi (theo yêu cầu 6, 7).

## 4. Giai đoạn kiểm thử (Automated & Manual)
- E2E testing cho từng tính năng mới.
- Kiểm tra tính nhất quán giao diện trên đa nền tảng.
