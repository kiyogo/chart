# Spec 119: Mobile Unified Frosted Glass & Seamless Bottom Bar Layout

## 1. Problem Statement
User testing revealed critical layout and visual regressions on mobile viewports (`@media (max-width: 767px)`):
1. **Vertical Dividing Seam ("thanh thẳng")**: A harsh vertical color boundary cuts straight down the left side and slices directly through the "Tin nhắn" tab icon in the bottom navigation bar (`media_1795528231213_708843a6.png`).
2. **Broken Frosted Glass ("không hiện đúng mờ trắng")**: The left navigation rail (`.app-rail`) appears as an opaque stark white column instead of blending seamlessly with the translucent frosted white glass of the sidebar and wallpaper (`media_1795528231213_9f3d0c60.png`).
3. **Uneven Icon Sizing ("các icon không to bằng đều nhau")**: Left rail items mix different squircle and glyph dimensions, and the bottom bar's 5th tab ("Tài khoản") renders a heavy solid purple letter circle ("1") that visually overpowers the 4 outline icons beside it.
4. **Lightbox Consistency**: Verify that Lightbox image corners remain strictly unrounded (`0px`) and scrollbar overlays remain completely suppressed across all screens.

## 2. Root Cause Analysis
1. **Vertical Dividing Seam in Bottom Bar**:
   - `.nav-sidebar` had `height: 100dvh;` while `.app-rail` had `height: calc(100dvh - 58px)`.
   - Consequently, behind `.mobile-bottom-bar` (`left: 0; right: 0; bottom: 0;`), the region `0..54px` had no panel (exposing the raw wallpaper), while the region `54px..100%` had `.nav-sidebar`'s white background.
   - Because `.mobile-bottom-bar` had translucent glass (`backdrop-filter: blur`), it blurred the split background behind it, producing a vertical line at `x = 54px` that cut directly through the middle of the "Tin nhắn" tab icon (`x = 0..78px`).
2. **Broken Frosted Glass on Left Rail**:
   - Desktop rule `.app-rail::before` (line 10398) added an extra layer with `background: color-mix(in srgb, var(--bg-primary) 100%, transparent);` (100% opaque white) and its own blur.
   - Combined with `html[data-rail-scrim="true"] .app-rail` (`rgba(255, 255, 255, 0.78)`), `.app-rail` was rendered as a solid opaque white bar, completely disconnected from `.nav-sidebar`'s translucent frosted wallpaper glass.
3. **Uneven Icon Sizes**:
   - In `.app-rail`, `.rail-brand svg` and `.rail-item.has-logo svg` were larger and heavier than outline icons.
   - In `.mobile-bottom-bar`, when the user had no uploaded avatar, `Avatar` rendered a solid purple circle with the initial "1", which clashed with the thin outline strokes of `MessageIcon`, `FriendsIcon`, `SearchIcon`, and `BellIcon`.

## 3. Requirements & Acceptance Criteria
- **AC1 (Zero Vertical Seam in Bottom Bar)**:
  - Both `.app-rail` and `.nav-sidebar` must terminate at `calc(100dvh - 58px - env(safe-area-inset-bottom))`.
  - The area behind `.mobile-bottom-bar` must be uniform, and `.mobile-bottom-bar` must render a continuous, self-contained frosted glass surface across `100vw`.
  - The "Tin nhắn" tab icon must have zero vertical dividing lines, zero split color, and 100% uniform background.
- **AC2 (Unified Frosted Glass "mờ trắng")**:
  - On `@media (max-width: 767px)`, `.app-rail::before` must be `display: none !important;`.
  - `.app-rail` and `.nav-sidebar` must use the exact same `background: color-mix(in srgb, var(--bg-surface) var(--panel-opacity, 72%), transparent) !important;` and `backdrop-filter: blur(var(--panel-blur, 16px)) saturate(1.3) !important;`.
  - There must be 0px border between `.app-rail` and `.nav-sidebar`, appearing as one harmonious frosted glass surface.
- **AC3 (Harmonious Icon Sizing)**:
  - In `.app-rail`: Every button touch target must be `40×40px`, and all SVG glyphs/squircles must be normalized to `24×24px`.
  - In `.mobile-bottom-bar`: All 5 tabs must have `22×22px` icons. For Tab 5 ("Tài khoản"), if no custom avatar photo is uploaded, display outline `UserIcon { size: 22 }` matching the visual style and stroke of Tabs 1–4.
- **AC4 (Regression Free)**:
  - Lightbox retains `0px` border-radius and hidden scrollbars.
  - Zero horizontal overflow (`overflow-x: clip`).
  - Automated E2E verification passes on `390×844` and `360×800`.
