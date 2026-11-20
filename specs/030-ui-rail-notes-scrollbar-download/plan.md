# Implementation Plan

1. **`app/web/canvas_scrollbars.js`** + **`app/e2e/scrollbar-switch-check.mjs`**:
   - Adjust `THUMB_EXPANDED_WIDTH = 5`, `THUMB_EDGE_MARGIN = 4`, `EDGE_OUTER_TOLERANCE_PX = 2`, `TRACK_INSET_Y = 12` (computing corner-safe vertical track bounds inside `getScrollMetrics`), soften hover opacity/color, and update E2E expectations (`4..7px` expanded width, within container bounds).
2. **`app/src/components/icons.rs`**:
   - Add `BellIcon`, `DownloadIcon`, `PlayStoreLogoIcon`, `AppStoreLogoIcon`, `WindowsLogoIcon`, `MacLogoIcon`, `LinuxLogoIcon`, `AndroidIcon`.
3. **`app/src/components/navigation/mod.rs`**, **`app/src/components/navigation/rail.rs`**, **`app/src/components/download_view.rs`**, **`app/src/pages/chat.rs`**:
   - Add `NavSection::Download`.
   - Implement `DownloadView` with hero title, intro, top/bottom buttons, QR code (`qrcode`), Desktop selector (Windows / macOS / Linux), and Mobile selector (Android `.apk` / iOS).
   - Add Notifications (`BellIcon`) + Download (`DownloadIcon`) triggers in `AppRail` `.rail-bottom`, with pure-white QR download hover card (CH Play + App Store single logos) and white notifications popover.
   - Support `/?page=download` deep link on startup.
4. **`app/src/components/settings_view.rs`**:
   - Remove the desktop `<header class="chat-header">` (`<- Cài đặt`) while keeping a mobile-only back trigger inside `.settings-body` if viewport `< 768px`.
5. **`app/src/components/navigation/conversations.rs`** + **`app/src/components/feed_view.rs`** + **`app/assets/main.css`**:
   - Tighten `.sidebar-search` & `.notes-strip` vertical spacing near `"Tìm cuộc trò chuyện"`.
   - Anchor `"Chia sẻ suy nghĩ"` bubble top-left above avatar (`#FFFFFF`, subtle faint dark border, authentic two-dot thought cloud tail, zero clipping), separate `+` button (`stop_propagation()` -> `"story"`) vs bubble button (`"thought"`), and remove `.rail-account[aria-expanded="true"]` white avatar ring.
   - Style `.account-menu` / rail popovers pure white `#FFFFFF` with subtle dark hairline border, and add `.download-page-*` flat minimalist styles.
