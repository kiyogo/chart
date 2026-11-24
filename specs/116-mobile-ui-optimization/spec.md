# Spec 116: Complete Mobile UI Optimization (`<= 767px`)

## Problem Statement
Baseline mobile visual audit at `390×844` (iPhone 14/15 Pro) and `360×800` (Android) revealed several mobile UX/UI defects:
1. **Bottom Navigation Rail (`.app-rail`)**: 8 navigation icons are crammed into a narrow row with no breathing room and inconsistent sizing.
2. **Chat Header (`.chat-header`)**:
   - `.chat-header-meta` uses `display: flex` + `text-overflow: ellipsis`, causing `"Mã hóa đầu cuối"` to hard-clip mid-character (`"Mã hóa đầu cu"`).
   - Missing `env(safe-area-inset-top)` padding for notched/Dynamic Island PWAs.
   - Header action buttons and large avatar crowd out the peer username.
3. **Mobile Composer (`.composer-box`)**:
   - 5 icon buttons (`+`, Emoji, GIF, Sticker, Send) squeeze the `<textarea>` so even a short placeholder wraps onto 2 lines (`77px` tall bar).
   - GIF and Sticker are already reachable inside the unified MediaPicker via the Emoji button.
4. **Mobile Lightbox (`.lightbox-topbar`)**:
   - Desktop zoom buttons (`-`, `100%`, `+`) + AI toggle + Rotate + Download + Close crowd out the filename on `360–390px` screens.
5. **Mobile Modals, Sheets, Popovers, Details Drawer, Settings & Safe Areas**:
   - Modals and menus need native bottom-sheet geometry (`100dvh`, `env(safe-area-inset-bottom)`), `16px` input font-size (zero iOS Safari zoom), `>= 44px` tap targets, and `0px` horizontal document overflow across every screen.

## Goals & Acceptance Criteria
- **AC-1 (Zero Horizontal Overflow)**: `document.documentElement.scrollWidth <= window.innerWidth` across Auth, Conversations List, Active DM Chat, Details Drawer, Account Menu, Settings List/Detail, and Lightbox on both `390×844` and `360×800`.
- **AC-2 (Single-Line Clean Mobile Composer)**: On `390×844` and `360×800`, `.composer-box` fits cleanly on 1 single line (`height <= 64px`) with `font-size >= 16px` and `>= 180px` width for `.composer-input`.
- **AC-3 (No Mid-Character Header Clipping)**: `.chat-header-title` and `.chat-header-meta` truncate cleanly without mid-character clipping; header respects `env(safe-area-inset-top)`.
- **AC-4 (Native Bottom Tab Bar)**: `.app-rail` on mobile has `>= 44px` comfortable touch targets, clean top hairline divider, smooth horizontal snap scrolling, and bottom-sheet `.account-menu`.
- **AC-5 (Mobile-First Lightbox & Bottom Sheets)**: Lightbox topbar hides desktop zoom buttons on `<= 640px` leaving clean room for filename + AI toggle + Rotate + Download + Close; Lightbox image uses full mobile width (`100vw`); modals render as clean bottom sheets with safe-area padding.
- **AC-6 (Zero Desktop Regressions)**: All desktop (`>= 768px`) E2E suites (`spec115`, `spec114`, `spec113`, `spec112`, `spec101`, `verify_clean_ai_ui`) remain 100% PASS.
