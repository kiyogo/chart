# Tasks

- [x] T001 Fix canvas scrollbar hover size (`3px -> 5px`) and rounded-corner inset bounds in `app/web/canvas_scrollbars.js` & `app/e2e/scrollbar-switch-check.mjs`
- [x] T002 Add rail & download icons (`BellIcon`, `DownloadIcon`, `PlayStoreLogoIcon`, `AppStoreLogoIcon`, OS icons) in `app/src/components/icons.rs`
- [x] T003 Create `DownloadView` (`app/src/components/download_view.rs`), add `NavSection::Download`, and add Thông báo + Tải ứng dụng (with QR + CH Play + App Store hover popover) to `AppRail` (`rail.rs`) & `chat.rs`
- [x] T004 Remove `<- Cài đặt` header from `app/src/components/settings_view.rs`
- [x] T005 Update `"Chia sẻ suy nghĩ"` bubble (pure white, subtle dark border, top-left above avatar, distinct `+` vs thought click targets, closer to search bar), `.rail-account` zero white ring, `.account-menu` white + faint dark border, and `FeedView` strip styling in `conversations.rs`, `feed_view.rs`, and `main.css`
- [x] T006 Rebuild (`./app/build-web.sh`), run Playwright E2E checks + inspect screenshots visually across all updated views, and commit + push to `origin main`
- [x] T007 Polish rail popovers (`.account-menu.rail-menu`, `.download-rail-menu`): more offset from edge, soft blurred outer border only (no gray frame boxes, no border on QR or store buttons), uniform 4-sided inner spacing
- [x] T008 Create `NotificationsView` (`app/src/components/notifications_view.rs`), add `NavSection::Notifications`, and wire rail `"Thông báo"` click to open this dedicated full page
- [x] T009 Fix `DownloadView` white `#FFFFFF` rounded `.main-stage` surface, remove QR border frame, and remove top-left `<-` arrow + logo + title in `download_view.rs` & `main.css`
- [x] T010 Rebuild (`./app/build-web.sh`), update & run `spec030-ui-check.mjs` + `borderless-shadowless-check.mjs` + `scrollbar-switch-check.mjs`, visually inspect screenshots, and commit + push to `origin main`
- [x] T011 Enlarge QR SVG (`quiet_zone(false)`), match `8px` uniform menu margins & compact store spacing on `.download-rail-menu`, strip all gray container boxes from `DownloadView`, and remove `scale(...)` zoom inflation across `pop-in`, `dialog-in`, rail icons, and UI controls in `download_view.rs` & `main.css`
- [x] T012 Rebuild (`./app/build-web.sh`), update & run all E2E checks, verify screenshots visually, and commit + push to `origin main`
