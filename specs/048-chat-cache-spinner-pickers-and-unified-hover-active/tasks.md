# Implementation Tasks — Spec 048

- [x] **T001: Chat Loading Spinner + Per-Conversation In-Memory Cache (Items 2 & 3)**
  - Add `loading_initial: bool` to `MessageList` (`app/src/components/messages.rs`) to render a centered spinner instead of flashing `.conversation-intro` / `.e2ee-notice`.
  - Add `dm_cache`, `group_cache`, `channel_cache` in `app/src/pages/chat.rs` so returning to any previously visited DM, Group, or Channel renders instantly without reloading, while silently refreshing in the background and staying in sync on send/WS/load-more.

- [x] **T002: Zero-Flash Anchor Positioning for Custom Pickers (Item 4)**
  - Fix `createPopover` & `positionPopover` in `app/web/custom_pickers.js` so select/color/date/time popovers never appear at `(0,0)` before settling at the anchor.

- [x] **T003: Selective Essential Settings Titles (Item 7)**
  - Remove redundant `.settings-group-title` nodes in `settings_more.rs`, `settings_view.rs`, `settings_extra.rs`, `accounts.rs`, keeping essential ones (`Xem trước`, `Chủ đề`, `Màu nhấn`, `Sao lưu chìa khóa bảo mật`, `Nhật ký bảo mật`, `Danh sách chặn`, `Đặt mật khẩu khóa`, `Nội dung tin nhắn tự động`, etc.).

- [x] **T004: Rail Padding, Server Header Spacing, Unified Translucent Hover/Active (No Color Change / No Bold) & Equal Sidebar Insets (Items 1, 5, 6, 8)**
  - Update `app/assets/theme.css`, `app/src/platform/mod.rs`, and `app/assets/main.css` for translucent `--bg-hover` & `--accent-soft`, rail top/bottom padding, balanced `.sidebar-server-header`, uniform left/right sidebar padding, generous `.settings-cols` column gap, zero text/icon color change, zero bolding on active/click, and clean subtle `.settings-group-title`.

- [x] **T005: Build Web Release, Run Comprehensive E2E (`spec048` + Regression), Visual Inspection, Build Desktop + `.deb`, Commit & Push**
  - Build Web release, run `app/e2e/spec048-ux-refinements.mjs` + regressions, inspect screenshots with `read_file`, fix any polish issues, build Linux Desktop + `.deb`, commit, and `git push origin main`.
