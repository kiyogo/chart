# Implementation Plan — Spec 034

1. **`app/assets/main.css`**:
   - Add `overflow-x: hidden;` to `.sidebar-scrollable`.
   - Extend `.sidebar-scrollable > .notes-strip-wrap` full-bleed across the sidebar padding via `margin: 0 -0.45rem; flex-shrink: 0;`.
   - Set `.sidebar-scrollable > .notes-strip-wrap .notes-strip` horizontal padding to `0.9rem` (`scroll-padding-inline: 0.9rem`) so resting alignment matches `.chat-item` avatars while scrolled avatars travel flush to `0px` at both left and right frame edges.
2. **`app/e2e/spec030-ui-check.mjs`**:
   - Assert `Math.abs(stripRect.left - sidebarRect.left) <= 0.5` and `Math.abs(stripRect.right - sidebarRect.right) <= 0.5` (0px white clipping strip on both sides), and verify scrolled avatar touches the left frame edge cleanly.
3. **Build, Verify, Reinstall Desktop & Auto-Push**:
   - `./app/build-web.sh`
   - Run `spec030-ui-check.mjs`, `scrollbar-switch-check.mjs`, `borderless-shadowless-check.mjs` + visually inspect scrolled avatar screenshot.
   - Rebuild & reinstall native Linux desktop app + push to `origin/main`.
