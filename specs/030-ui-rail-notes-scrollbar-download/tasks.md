# Tasks

- [x] T001 Fix canvas scrollbar hover size (`3px -> 5px`) and rounded-corner inset bounds in `app/web/canvas_scrollbars.js` & `app/e2e/scrollbar-switch-check.mjs`
- [x] T002 Add rail & download icons (`BellIcon`, `DownloadIcon`, `PlayStoreLogoIcon`, `AppStoreLogoIcon`, OS icons) in `app/src/components/icons.rs`
- [x] T003 Create `DownloadView` (`app/src/components/download_view.rs`), add `NavSection::Download`, and add Thông báo + Tải ứng dụng (with QR + CH Play + App Store hover popover) to `AppRail` (`rail.rs`) & `chat.rs`
- [x] T004 Remove `<- Cài đặt` header from `app/src/components/settings_view.rs`
- [x] T005 Update `"Chia sẻ suy nghĩ"` bubble (pure white, subtle dark border, top-left above avatar, distinct `+` vs thought click targets, closer to search bar), `.rail-account` zero white ring, `.account-menu` white + faint dark border, and `FeedView` strip styling in `conversations.rs`, `feed_view.rs`, and `main.css`
- [x] T006 Rebuild (`./app/build-web.sh`), run Playwright E2E checks + inspect screenshots visually across all updated views, and commit + push to `origin main`
