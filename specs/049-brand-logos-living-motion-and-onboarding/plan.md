# Technical Plan — Spec 049: Brand Logos Suite, Living Motion System & Interactive Beginner Guide

## Architecture & Module Map
1. **`app/src/components/brand_logos.rs`** (NEW):
   - Pure inline SVG components (no network fetch, HiDPI crisp, dark/light safe):
     - `CloudProviderLogo { provider_id, size }` (Google Drive, OneDrive, Dropbox, Nextcloud, WebDAV, Amazon S3, Cloudflare R2, MinIO, NAS, Apple iCloud).
     - `OsBrandLogo { text, size }` (Linux Tux, Windows 11, macOS/Apple, Android, iOS, Generic Web).
     - `BrowserBrandLogo { text, size }` (Chrome, Firefox, Safari, Edge, Brave, Opera, ChatW Desktop).
     - `DeviceBrandPair { text, size }` (renders Browser logo + overlapping corner OS badge in a living hoverable container `.device-brand-pair`).
     - `GoogleAuthLogo`, `E2eeLiveShield`, `TwoFactorLogo`, `PasskeyLogo`, `ZipBackupLogo`, `SecurityEventIcon { kind }`, `LanguageFlagLogo { code }`.
   - Exported from `app/src/components/mod.rs`.
2. **`app/src/components/icons.rs`**:
   - Upgrade `LinuxLogoIcon` to delegate to the multicolor Linux Tux SVG so `DownloadView` and all callers render the real Linux penguin logo.
3. **Component Integration**:
   - `app/src/components/backup_cloud_ui.rs`:
     - Add friendly 3-step visual beginner flow banner at top of Cloud panel.
     - Render `CloudProviderLogo` on every provider row, connected cloud account row, and connection setup header.
     - Replace raw OAuth env-var jargon (`GOOGLE_DRIVE_CLIENT_ID (...)`) with plain friendly Vietnamese provider descriptions + clean status badge.
   - `app/src/components/accounts.rs`:
     - Add live animated security status pill header + `TwoFactorLogo` on 2FA row.
     - Render `DeviceBrandPair` (Browser + OS logo, e.g., Chrome + Linux Tux) and `SecurityEventIcon` on every `Nhật ký bảo mật` row with clean horizontal layout.
   - `app/src/components/storage_ui.rs`:
     - Render `DeviceBrandPair` on every device row in `DeviceTrustPanel` (both trusted devices and pending requests) + live green pulse dot on current device.
   - `app/src/components/settings_extra.rs`:
     - Render `DeviceBrandPair` in `DevicesDialog` (replacing generic `DevicesIcon`).
     - Render `GoogleAuthLogo` on the Google sign-in row and email privacy row in `AccountSecurity`.
   - `app/src/components/settings_more.rs`:
     - Render `LanguageFlagLogo` on every tile in `LanguageSettings`.
   - `app/src/components/download_view.rs`:
     - Auto-detect user's OS via `navigator.userAgent` and highlight the matching card with a living `"Gợi ý cho máy của bạn"` pulse badge + animated QR scan line.
   - `app/src/components/guide.rs`:
     - Animate `Illustration` SVG elements (floating shapes, twinkling stars, shield pulse).
     - Upgrade `WelcomeTour` with animated progress bar, subtle step transitions, and `window.cwSparkBurst` celebration on finish.
     - Add interactive persisted **Beginner Quick-Start Checklist** (5 steps with animated progress meter, SVG icons, and check-off spark burst) to `GuideSettings`.
   - `app/src/pages/chat.rs`:
     - Upgrade `.empty-state` (Messages & Communities) with the D3Motion-inspired **Living Luminous Orb** (`cw-living-orb`) and 4 interactive 1-click beginner quick-start cards (`Thêm bạn bè`, `Tạo nhóm`, `Tùy biến giao diện`, `Hướng dẫn người mới`) with spotlight shimmer.
4. **Living Motion Engine**:
   - `app/web/living_motion.js` (NEW, bundled in `app/build-web.sh` & desktop eval/index.html):
     - Smooth rAF cursor spotlight (`--mx`, `--my`) on `.cw-spotlight` elements.
     - Lightweight Canvas/DOM micro-confetti burst `window.cwSparkBurst(x, y)` using current `--accent` + harmonious tones (respects `data-reduce-motion="true"`).
   - `app/assets/main.css` & `app/assets/theme.css`:
     - Add spring timing tokens (`--spring-snappy`, `--spring-smooth`, `--spring-bounce`).
     - Style `.brand-badge`, `.device-brand-pair`, `.cw-living-orb`, `.cw-spotlight`, `.cw-quickstart-grid`, `.cw-checklist`, `.cw-cloud-guide`, `.cw-live-dot`, `.cw-qr-scan`, staggered entrance animations, elastic switch stretch, message spring-in, and full `data-reduce-motion="true"` + `prefers-reduced-motion: reduce` overrides.
     - Preserve 100% of Spec 047/048 rules (zero solid white boxes, translucent hover/active, zero text/icon color change, zero bolding).
5. **Verification**:
   - New live Chromium E2E test `app/e2e/spec049-brand-logos-living-motion.mjs` checking every brand logo SVG across Cloud, Security Log, Devices, Account, Language, Download, animated Empty State + Beginner Checklist + WelcomeTour + living motion keyframes, plus visual screenshot inspection and Spec 048 regression!
