# Feature Specification: Global Canvas Scrollbar & Toggle Switch Polish (024)

## Problem Statement
1. **Canvas Scrollbars (`app/web/canvas_scrollbars.js`)**:
   - Currently render too thick (`6px` normal + `4px` shadowBlur) when scrolling or approaching the edge.
   - Hovering directly over the scrollbar does not noticeably expand it (`6px -> 8px` is barely perceptible, and proximity triggers `8px` early from `24px` away, plus `.panel-resizer` / overlay elements can intercept `e.target` at the panel edge so hover fails).
2. **Toggle Switches (`app/assets/main.css`)**:
   - Knob inside `input[type="checkbox"].switch` has asymmetrical top/bottom/side gaps (`2px` vs `3px`) due to fractional `rem` rounding at `15px` root font-size and browser DPR/zoom.
   - `.ent-btn-group > input` and `.ent-toolbar > input` lack `:not([type="checkbox"])`, stretching switches inside `.ent-btn-group` (e.g., Enterprise Settings -> Thanh bên) to `min-width: 115px` while the knob still translates only `~20px`.

## Acceptance Criteria
1. All vertical scrollbars across the entire web use thin `3px` pills when scrolling or when cursor approaches within `28px` of the right edge, with crisp Flat Minimalist rendering (no thick blur halo).
2. Moving the cursor onto the scrollbar zone (`<= 14px` from right edge or directly over thumb) smoothly animates thumb width from `3px` to `8px` (`> 2.5x` wider) and increases contrast. Hover detection works reliably even when `.panel-resizer` or sibling elements overlap the edge (`document.elementsFromPoint`).
3. Every `input[type="checkbox"].switch` across the entire app maintains identical fixed dimensions (`44px × 24px`, locked `min-width`/`max-width`/`flex-basis`) regardless of parent flex/input rules (`.ent-btn-group`, `.ent-toolbar`, `.settings-row`, modals, plugin settings).
4. The circular knob (`18px × 18px`) is vertically centered with `top: 50%; transform: translate3d(..., -50%, 0)` and horizontally spaced `3px` from left when unchecked and `3px` from right when checked (`translate3d(20px, -50%, 0)`), eliminating subpixel asymmetry.
5. Verified on the live running server in headless Chromium + full cargo test suite passes with zero regressions.
