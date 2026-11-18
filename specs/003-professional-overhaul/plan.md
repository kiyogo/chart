# Plan: Professional Overhaul of ChatW

## 1. Design System Overhaul (Ưu tiên)
- Tạo CSS Token định nghĩa màu sắc, khoảng cách, font, bo góc.
- Áp dụng vào `app/assets/main.css`.
- Loại bỏ các class chứa hiệu ứng glow, bóng quá đậm, gradient neon.

## 2. Structural/Database Overhaul
- Migration DB hỗ trợ Category, Channel, và Profile nâng cao.
- Cập nhật Models (`web/backend/src/models.rs`, `app/src/models.rs`) để hỗ trợ Group/Channel.

## 3. Implementation Phases
- **P1: Design System & Semantic CSS**.
- **P2: Group/Channel Infrastructure & UI**.
- **P3: Profile & Contact System**.
- **P4: Messaging Features (Stickers, Reactions, Polls, Events)**.
- **P5: Settings, Notifications & Final QA**.
