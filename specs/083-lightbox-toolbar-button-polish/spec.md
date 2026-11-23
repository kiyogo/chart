# Spec 083 — Lightbox Top-Right Toolbar Button Visual Polish & Wallpaper Override Fix

## Problem Statement
In the full-viewport image lightbox (especially when a custom wallpaper with `data-panel-translucent="true"` is active and the image is zoomed underneath the topbar):
1. The **Download** button carried the generic `.btn-secondary` class, which was hijacked by a high-specificity translucent-wallpaper override (`app/assets/main.css:11169`), painting a large opaque pale-white circle + inset ring behind a white icon (making the icon nearly invisible and jarring).
2. `filter: drop-shadow(...)` was applied to the `.lightbox-tool-btn` container instead of only the `<svg>`, creating a muddy grey halo around button hover circles.
3. The 5 toolbar icons had mismatched optical sizes and weights (`RotateCwIcon` radius 9 looked ~30% larger than `ZoomInIcon`/`ZoomOutIcon`, `DownloadIcon` included a duotone translucent tray + heavier stroke, `CloseIcon` looked thin).
4. Over bright zoomed-in images, the topbar gradient scrim was too light (`0.58` over `60px`) to cleanly separate white icons.

## Design Goals
- **Zero borders / zero frames** (preserving Spec 080): transparent at rest, smooth dark-scrim-backed subtle circular hover (`rgba(255, 255, 255, 0.16)`), subtle scale-down on `:active`, **zero** white opaque blob even when custom translucent wallpaper is active.
- **Remove `.btn-secondary`** from the lightbox Download anchor (`class="lightbox-tool-btn lightbox-download"`).
- **Drop-shadow strictly on `<svg>`**, never on the button container.
- **Harmonized single-weight lightbox icon set**: all 5 icons (`Zoom Out`, `Zoom In`, `Rotate`, `Download`, `Close`) share one identical `18px` display size, balanced `16px` optical core on a `24×24` grid, uniform `1.85px` rounded stroke, and zero duotone fills.
- **Strengthened borderless top gradient scrim** (`76px`, `rgba(8, 10, 14, 0.72) → 0`) so controls remain crisp and legible even over pure-white zoomed photos.

## Acceptance Criteria
1. Under **custom wallpaper + translucent panel mode** (`data-panel-translucent="true"` + `data-app-wp`), ALL 6 top-right lightbox controls have `background: transparent` at rest, `border: none`, `box-shadow: none`, and uniform subtle hover `rgba(255, 255, 255, 0.16)` (never pale opaque white).
2. All 5 SVG icons share uniform `18×18` optical weight with `stroke-width: 1.85` and zero `<path stroke="none">` duotone blotches.
3. Live Playwright E2E + close-up hover screenshot over a bright zoomed-in photo visually confirm clean, crisp, cohesive controls.
4. Rust unit tests + E2E tests pass and changes are committed & pushed to `origin main`.
