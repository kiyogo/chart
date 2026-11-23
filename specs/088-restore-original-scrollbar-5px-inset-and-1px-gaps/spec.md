# Spec 088 — Restore Original 3px Canvas Scrollbar, Balanced 5px Equal Left/Right Sidebar Inset & Exact 1px Sidebar Item Gap

## Problem Statement
1. **Broken/Squished Scrollbar (`media_1795431146135_2a8ba1e1.png`)**: Spec 086 reduced `--panel-inset` to `3px` and shrank the canvas scrollbar thumb in `app/web/canvas_scrollbars.js` from `3px` down to an unrounded `2px` (`0.5px` on each side). On 1x displays, subpixel antialiasing turned the scrollbar into a blurry faint hairline colliding with both the active blue item and the right panel border.
2. **0px Touching Sidebar Items (`media_1795431217446_0fb3d06e.png`)**: Setting `gap: 0` on sidebar lists (Settings, Communities, Channels, Friends, Messages) caused adjacent hovered and active pills to smash directly into each other with zero visual separation.

## Goals & Acceptance Criteria
1. **100% Original Canvas Scrollbar (`app/web/canvas_scrollbars.js`)**:
   - Restore the scrollbar engine 100% to its pre-Spec-086 design (`commit 6305366` / `0c31331`): constant crisp **`THUMB_WIDTH = 3`**, exact `edgeMargin = (padRight - THUMB_WIDTH) / 2` when `padRight >= THUMB_WIDTH + 2`, pixel-snapped `thumbX = Math.round(rawThumbX * dpr) / dpr`, identical original light/dark colors, opacity, hover/drag hitboxes, and pill cap radius.
2. **Balanced Slightly Wider Equal Left/Right Inset (`--panel-inset: 5px`)**:
   - Increase `--panel-inset` equally on left & right from `3px` to **`5px`** (`+2px` per side — still noticeably tighter than the old `8px`, and mathematically the exact odd-integer minimum `THUMB_WIDTH + 2 = 5px` where the 100% original `3px` scrollbar renders with **exact `1px` crisp white gap on the left** and **exact `1px` crisp white gap on the right** on both 1x and 2x screens, with zero subpixel blur).
3. **Exact `1px` Vertical Spacing Between Sidebar Items**:
   - Set `gap: 1px !important` across all left sidebar lists: Messages (`.nav-sidebar .sidebar-scrollable`), Friends (`.nav-sidebar .friends-scroll`), Communities & Channels (`.nav-sidebar .sidebar-scrollable`, `.nav-sidebar .category-block`), Settings (`.settings-nav`), and Server Settings (`.server-settings-stage .settings-nav`, `.settings-nav-footer`).
   - Keep `margin-top: 0 !important; margin-bottom: 0 !important;` on all row items (including `.settings-nav-item.settings-nav-dev` so "Nhà phát triển" remains 100% uniform at `1px` gap with every other Settings item).
4. **Build, Automated Tests, Live Visual Pixel Verification & Git Push**:
   - Update E2E checks (`app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs`), rebuild release bundle (`./app/build-web.sh release`), verify live screenshots zoomed-in on both the `3px` scrollbar (`1px | 3px | 1px`) and Settings/Communities/Messages `1px` item gaps, and commit + push to `origin main`.
