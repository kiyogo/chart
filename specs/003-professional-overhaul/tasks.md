# Tasks: Professional Overhaul of ChatW

## Phase 1: CSS System & Cleanup
- [x] Định nghĩa `app/assets/theme.css` với semantic tokens.
- [x] Audit `app/assets/main.css` & các component, xóa glow, shadow, gradient, blur.
- [x] Áp dụng `theme.css` toàn hệ thống.

## Phase 2: Backend Foundations
- [x] Migration: Categories & Channels (`0008_group_channels_and_features.sql`).
- [x] Migration: Profiles & Stickers.
- [x] Cập nhật API Handler tương ứng (Categories, Channels, Reactions, Stickers, Profiles).

## Phase 3: Component Library & UI Layout
- [x] Xây dựng lại Base Layout (nhất quán).
- [x] Xây dựng Component Nhóm/Kênh (Group Channels Bar & Channel switching).
- [x] Xây dựng lại Active Call UI (không glow).
- [x] Xây dựng lại Incoming Call Modal (chuyên nghiệp).

## Phase 4: Feature Implementation
- [x] Profile System & Avatar/Cover (`ProfileModal` with edit bio, status, presence, display name).
- [x] Sticker System (`StickerPicker`, packs & stickers integration).
- [x] Message Reactions System (emoji reaction pills & quick reactions).
- [ ] Polls/Events/Reminders.

## Phase 5: Verification & Deployment
- [x] E2E Test (Toàn bộ bộ test cũ + mới qua `scratch/test_e2e_overhaul.mjs`).
- [x] Git commit & push.
