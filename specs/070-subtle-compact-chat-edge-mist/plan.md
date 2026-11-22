# Implementation Plan: Spec 070 — Subtle Compact Top & Bottom Chat Edge Mist

## Targeted File Changes
1. `app/assets/main.css`:
   - `.composer::before`: change `top: -30px` → `top: -6px`; replace heavy `0.86` bottom mask with sheer `0.48 -> 0.28 -> 0.10 -> 0.02 -> transparent`.
   - `.chat-column:has(> .chat-messages) > .chat-header::before`: change `height: calc(var(--header-height) + 30px)` → `height: calc(var(--header-height) + 12px)`; replace heavy `0.86` top mask with subtle translucent `0.56 -> 0.38 -> 0.18 -> 0.05 -> transparent`.
   - `.chat-column ... > .chat-messages` mask-image (normal & pinned-bar variants): tighten top dissolve from `84px` → `56px` and bottom dissolve from `92px` → `56px` with gentler edge transparency so bubbles directly above the composer remain 100% `#000` vivid and content behind the composer never washes solid white.
2. `app/web/adaptive_surface.js`:
   - Synchronize `topMistH = hdrH + 12` and `botMistH = compRectH + 6` so `wp-image` cover offset and 2D per-pixel canvas strips match CSS 1:1.
3. `app/e2e/spec052-visual-and-contrast-audit.mjs` & `app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`:
   - Update mist height & gentle opacity curve assertions to match Spec 070 bounds (`beforeHeight` in `56..76px`, `0.56` top, `0.48` bottom, `56px` scroll dissolve).
