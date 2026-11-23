# Spec 088 — Technical Plan

## Files to Modify
1. `app/web/canvas_scrollbars.js`
   - Revert Spec 086 adaptive `thumbW` shrinking and unrounded `thumbX` back to the exact pre-Spec-086 (`6305366`) implementation:
     - Constant `THUMB_WIDTH = 3`.
     - `if (!isDoc && padRight >= THUMB_WIDTH + 2 && padRight <= GUTTER_MAX_PAD_RIGHT) { edgeMargin = (padRight - THUMB_WIDTH) / 2; }`
     - `const thumbX = Math.round(rawThumbX * dpr) / dpr;`
     - Use `THUMB_WIDTH` consistently in `getScrollMetrics`, `isThumbHovered`, and `render()`.
     - Retain read-only `window.__chatwCanvasScrollbarMetrics` (including `thumbW: THUMB_WIDTH`) for deterministic E2E assertions.
2. `app/assets/main.css`
   - Update the Spec 086 / 088 sidebar inset & gap block:
     - `--panel-inset: 5px;` (exact `1px` left + `3px` original scrollbar + `1px` right = `5px`).
     - `.nav-sidebar .sidebar-top-header` uses `padding: 0 var(--panel-inset, 5px) 0 calc(var(--panel-inset, 5px) + 5px) !important;`.
     - `.nav-sidebar .sidebar-scrollable`, `.nav-sidebar .friends-scroll`, `.nav-sidebar .category-block`, `.settings-nav`, `.server-settings-stage .settings-nav`, `.settings-nav-footer` all use `gap: 1px !important;`.
     - All sidebar list rows (`.conv-row`, `.chat-item`, `.request-card`, `.create-community-btn`, `.channel-row`, `.settings-nav-item`, `.settings-nav-item.settings-nav-dev`) retain `margin-top: 0 !important; margin-bottom: 0 !important;`.
3. `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs`
   - Update assertions to verify:
     - `padLeft === 5px` and `padRight === 5px` across Settings, Messages, Friends, Communities.
     - Exact `1px` vertical gaps between adjacent items in Settings (including Developer), Messages, Friends, Communities, and Community Channels.
     - Restored original `thumbW === 3` scrollbar mathematically centered with `leftGap === 1` and `rightGap === 1` (rounded pixel-crisp on 1x DPR).
     - Capture zoomed close-up PNG crops verifying pixel-level sharpness of the `3px` scrollbar and `1px` gaps.
