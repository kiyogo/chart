# Spec 116 Technical Plan

## Files to Modify
1. **`app/src/components/composer.rs`**:
   - Add `composer-media-shortcut` CSS class to the separate `GIF` and `Sticker` shortcut buttons so mobile (`<= 767px`) hides the redundant shortcuts while keeping all Emoji/GIF/Sticker tabs 1 tap away inside the MediaPicker.
2. **`app/src/components/rich.rs`**:
   - Verify Lightbox zoom button classes (`lightbox-zoom-out`, `lightbox-zoom-in`, `lightbox-zoom-readout`) so mobile `<= 640px` can cleanly hide desktop zoom controls while keeping pinch/double-tap zoom + AI SVG toggle + Rotate + Download + Close.
3. **`app/assets/main.css`**:
   - Fix `.chat-header-meta` truncation (`display: block` or flex inner text span with proper `text-overflow: ellipsis`).
   - Add comprehensive mobile (`@media (max-width: 767px)` and `@media (max-width: 640px)`) optimization block covering:
     - Viewport & touch ergonomics (`overscroll-behavior`, `-webkit-tap-highlight-color: transparent`, `touch-action: manipulation`, `100dvh`, `font-size: 16px` inputs).
     - Native bottom navigation bar (`.app-rail`, `.rail-nav`, `.rail-item`, `.rail-bottom`, `.account-menu.rail-menu`).
     - Conversations list, search bar, stories strip, and touch-friendly rows.
     - Active chat header (safe-area-top, compact 34px avatar, crisp ellipsis, balanced action spacing), message bubbles, rich images, and floating scroll-to-bottom button.
     - Single-line mobile composer bar + bottom-anchored `.picker-popover` / `.tools-menu`.
     - Mobile Lightbox (edge-to-edge image, spacious topbar without desktop zoom clutter).
     - Mobile bottom-sheet modals (`.modal-overlay`, `.modal-card`), full-screen Details drawer, Settings list/detail, Profile editor, and Auth layout.
4. **`app/e2e/spec116-mobile-ui-optimization.mjs`**:
   - Playwright E2E test verifying both `390×844` (iPhone 14 Pro) and `360×800` (Android) across Auth, Conversations, Active DM Chat, Composer geometry, Header truncation, Details Drawer, Account Menu, Settings List/Detail, and Lightbox — asserting zero horizontal overflow, single-line composer, touch target sizes, and zero page errors, while capturing visual verification screenshots.
