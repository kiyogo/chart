# Spec 118: Borderless Frosted-Glass Mobile UI & 100% Uniform Icon Sizing

## Problem Statement
Real-device screenshot with Light Wallpaper + Frosted Glass (`media_1795527163608_88c53a83.jpg`) exposed 3 concrete visual defects in Spec 117:
1. **Harsh straight divider lines ("thanh thẳng")**: `1px solid var(--border-subtle)` on `.app-rail` (`border-right`), `.rail-bottom` (`border-top`), `.rail-pins` (`border-top`), and `.mobile-bottom-bar` (`border-top`) drew sharp straight white lines cutting across the frosted wallpaper.
2. **Broken frosted white glass ("không hiện đúng mờ trắng")**: `.mobile-bottom-bar` rendered as solid opaque white (`background: var(--bg-sidebar)`) instead of matching the translucent frosted glass (`backdrop-filter: blur(...)`) of `.nav-sidebar`, and `.app-rail`'s mobile rule clobbered its wallpaper glass styling.
3. **Uneven icon sizes ("các icon không to bằng đều nhau")**: On the left rail, icons rendered at 4 different sizes (`40px` logo, `22px` nav icons, `34px` Vico AI, `26px` Settings gear), and on the bottom bar the Account avatar (`24px`) was larger than the `22px` tab icons.

## Acceptance Criteria
- **AC-1 (Zero Straight Divider Lines)**: `borderTopWidth`, `borderRightWidth`, `borderBottomWidth`, `borderLeftWidth` are ALL `'0px'` on `.app-rail`, `.rail-bottom`, `.rail-pins`, and `.mobile-bottom-bar`.
- **AC-2 (Unified Frosted Glass with Wallpaper)**: When wallpaper / `data-panel-glass="true"` / `data-panel-translucent="true"` is active, `.app-rail`, `.nav-sidebar`, and `.mobile-bottom-bar` all share translucent frosted glass (`backdrop-filter: blur(...)`, `rgba(...)` background) with zero opaque white slab at the bottom.
- **AC-3 (100% Uniform Icon Sizes)**:
  - All outline icons in `.app-rail` (including Settings gear in `.rail-bottom`) render at exact `24×24px`.
  - All logo icons in `.app-rail` (`.rail-brand svg` and `.rail-item.has-logo svg`) render at exact `28×28px`.
  - All 5 icons in `.mobile-bottom-bar` (including Account avatar `.user-avatar`) render at exact `22×22px`.
- **AC-4 (Zero Regressions)**: Verified visually in BOTH Light Glass Wallpaper mode AND Dark mode on `390×844` and `360×800`, plus desktop `spec115` 100% PASS.
