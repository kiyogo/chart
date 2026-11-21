# Implementation Tasks — Spec 047

- [x] **T001: Remove All Settings Titles & Ensure Self-Explanatory Inline Row Labels**
  - Hide `.settings-page-title`, `.settings-nav-title`, `.settings-group-title` cleanly in `app/assets/main.css`.
  - Update `app/src/components/accounts.rs` (`Xác minh hai bước` inline row label) and `app/src/components/settings_more.rs` (`Dán bảng tính`, `Hình nền trò chuyện`, `Hình nền toàn ứng dụng` inline row labels).

- [x] **T002: Remove All Nested White/Card Boxes Across the Entire App (Opaque + Translucent + Wallpaper + Glass)**
  - Strip Spec 045's white card/pill scrims on `.settings-row`, `.settings-list`, `.settings-nav-item`, `.chat-item`, `.chat-header-identity`, `.chat-header-actions`, `.meet-card`, `.feed-card`, `.mod-card`, `.theme-card`, `.wallpaper-tile`, `.settings-profile-card`, `.settings-preview-chat`, etc. in `app/assets/main.css`.
  - Flatten all static inner card backgrounds across the entire app (`.account-row`, `.security-event`, `.backup-codes`, `.partner-info-card`, `.sticker-pack-card`, `.event-card`, `.forum-row`, `.forum-post`, `.welcome-channel`, `.welcome-rules`, `.plugin-card`, `.result-card`, `.product-card`, `.bot-card`, `.inv-card`, `.fingerprint`, `.segmented`, etc.) while keeping clean spacing, hover/active states, and adaptive text readability.

- [x] **T003: Build Web Release, Verify Visually & Run E2E (`spec047` + Regression), Build Desktop + `.deb`, Commit & Push**
  - Build Web release (`./app/build-web.sh release`).
  - Run `app/e2e/spec047-headerless-no-white-boxes.mjs` and inspect every screenshot with `read_file`.
  - Run regression suite (`spec046`, `spec044`, `spec043`, `spec042`, `spec041`, `spec040`, `spec047`).
  - Build Linux Desktop release & `.deb`, commit, and `git push origin main`.
