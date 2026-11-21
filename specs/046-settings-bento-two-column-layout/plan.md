# Technical Plan — Spec 046: Settings Bento 2-Column Layout & Equal 1/3 Edge Spacing

## 1. CSS Grid & Symmetry Foundation (`app/assets/main.css`)
- Update `.settings-body`:
  - `padding: var(--space-4);` (exact `1rem` = `15–16px`, strictly 1/3 of prior `3rem` (`--space-12`) and 100% equal on top, right, bottom, left).
- Update `.settings-content`:
  - `width: 100%; max-width: none; display: flex; flex-direction: column; gap: var(--space-4);`
- Update `.settings-page-title`:
  - `margin: 0; line-height: 1.2;` (ensures measured `title.top - body.top === body.paddingTop === left === right`).
- Add clean Bento 2-Column Grid primitives:
  - `.settings-bento` / `.settings-cols`:
    `display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-4); align-items: start; width: 100%;`
  - `.settings-col`:
    `display: flex; flex-direction: column; gap: var(--space-4); min-width: 0;`
  - `.settings-span-full`: `grid-column: 1 / -1;`
  - Compact clean bento card styling on `.settings-group`: keep flat, borderless, shadowless aesthetic with uniform `var(--space-2)` internal spacing, clean single-line labels, and smooth responsive collapse (`@media (max-width: 960px) { .settings-bento, .settings-cols { grid-template-columns: 1fr; } }`).
- Also check `.settings-nav`: tighten padding slightly to `var(--space-4) var(--space-3)` so the left sidebar's top padding matches `.settings-body`'s top padding 100%!
- Also check `.settings-row input[type="range"]`: allow flexible width inside 2-column cards (`width: clamp(7.5rem, 11vw, 13rem); min-width: 7rem;`) so sliders in a 2-column Appearance card fit effortlessly without crowding or clipping labels! (Check `spec040` assertion: `sliderAudit.every((s) => s.height <= 8 && s.width >= 140)` — so set `min-width: 9.25rem; width: clamp(9.25rem, 12vw, 13.5rem);` which is `>= 140px` at 1440px viewport AND never crowds labels!)

## 2. Two-Column Balanced Layouts in Settings Components
- **`app/src/components/settings_more.rs`**:
  - `ChatPrefsSettings`: Wrap in `.settings-cols`: Left `.settings-col` = *Khi trò chuyện*; Right `.settings-col` = *Trạng thái* + *Dán bảng tính*.
  - `NotificationSettings`: Wrap in `.settings-cols`: Left `.settings-col` = *Thông báo*; Right `.settings-col` = *Âm thanh*.
  - `PrivacySettingsPanel`: Wrap in `.settings-cols`: Left `.settings-col` = *Ai có thể liên hệ với bạn*; Right `.settings-col` = *Danh sách chặn* + *Bảo vệ tin nhắn*.
  - `LockSettings`: Wrap in `.settings-cols`: Left `.settings-col` = *Khóa ứng dụng*; Right `.settings-col` = *Đặt / Đổi mật khẩu khóa*.
  - `AutoReplySettings`: Split cleanly into `.settings-cols`: Left `.settings-col` = *Bật trả lời tự động* + *Khi nào trả lời*; Right `.settings-col` = *Nội dung tin nhắn tự động*.
  - `AdvancedLookSettings`: Split into two composable halves OR provide `AppearanceColorsAndShapes` (Left) + `AppearanceWallpapersAndGlass` (Right) so `SettingsTab::Appearance` renders a super intuitive 2-column Bento layout with **Xem trước** right beside **Chủ đề / Màu nhấn / Chữ và mật độ**!
- **`app/src/components/settings_view.rs`**:
  - `SettingsTab::Account`: Full-width compact Profile Card on top, then `.settings-cols` with Left = *Thông tin tài khoản* and Right = *Mật khẩu và bảo mật* + *Thông tin định danh*.
  - `SettingsTab::Appearance`: `.settings-cols` with Left = *Chủ đề*, *Màu nhấn*, *Chữ và mật độ*, *Màu sắc nâng cao*, *Hình dạng và bố cục*; Right = *Xem trước*, *Hình nền trò chuyện*, *Hình nền toàn ứng dụng* + *Khôi phục giao diện mặc định*.
  - `SettingsTab::Customize`: `.settings-cols` with Left = `CustomizationSettings` (*Thanh lối tắt* + *Thông báo và hiệu ứng*) and Right = `SyncMotionSettings` (*Đồng bộ* + *Ảnh đại diện*).
  - `SettingsTab::Security`: `.settings-cols` with Left = *Xác minh 2 bước* + *Bảo vệ tin nhắn* (trạng thái + vân tay) and Right = *Sao lưu chìa khóa* + *Xuất dữ liệu* + *Đa tài khoản*.
  - `SettingsTab::Session`: `.settings-cols` with Left = `DeviceTrustPanel` and Right = *Đăng xuất* (with clean plain Vietnamese, removing "Khóa E2EE cục bộ").
- **`app/src/components/settings_extra.rs`**:
  - Allow `AccountSecurity` to cooperate cleanly inside the 2-column Account tab (or wrap its two groups cleanly inside `.settings-cols`).
- **`app/src/components/storage_ui.rs` & `app/src/components/backup_cloud_ui.rs`**:
  - Wrap multi-group sections cleanly into `.settings-cols` so `Lưu trữ`, `Đồng bộ`, `Sao lưu`, `Đám mây` also fill Left & Right evenly with zero wasted right-side empty space!
  - Strip ALL remaining jargon (`OPFS`, `IndexedDB`, `SigV4`, `Multipart`, `Local CAS`, `delta`, `AES-256-GCM`, `E2EE`, `TOTP`, `PKCE`) and ALL parentheses `(...)` while preserving `backup-cloud-check` / `storage-sync-check` selectors (`Google Drive`, `WebDAV`, `Thiết lập`, `Khôi phục hoàn tất`, `Chấp nhận đồng bộ`).

## 3. Automated Verification (`app/e2e/spec046-settings-bento-layout.mjs`)
- Verify on `Trò chuyện`, `Giao diện`, `Thông báo`, `Quyền riêng tư`, `Tài khoản`, `Khóa ứng dụng`, `Lưu trữ`, `Bảo mật`:
  - `paddingTop === paddingLeft === paddingRight` (exact `15px`–`16px`, strictly `<= 17px`, 1/3 of old `48px`!).
  - Left column and Right column sit side by side (`rightCol.left > leftCol.right`, `Math.abs(leftCol.top - rightCol.top) <= 2`).
  - Content fills the full width (`bodyClientRight - rightCol.right` equals `leftCol.left - bodyRect.left` within `±2px`!).
  - Zero `...`, zero unwanted `(...)`, zero technical jargon across checked tabs.
  - Capture screenshots for visual inspection with `read_file`.
- Run full regression suite (`spec028`, `spec040`, `spec041`, `spec042`, `spec043`, `spec044`, `spec045`).
