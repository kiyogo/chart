# Plan 061 — Sticky-Bottom Controller, Exact Edge Fade Projection & Real-Time Per-Position Surface Engine

## Target Files
1. `app/web/adaptive_surface.js` (NEW — unified 60fps real-time engine bundled for both Web & Desktop):
   - **`window.__chatwStickToBottom`**:
     - `pin(instant)` & `unpin()` & `isPinning()` API.
     - Automatically discovers `#chat-messages-scroll` & `.message-stream` via `MutationObserver`, attaches `ResizeObserver` on `.message-stream` + `#chat-messages-scroll`, capture-phase `load` listener for `<img>`/`<video>`, and explicit user upward-intent listeners (`wheel`, `touchstart`/`touchmove`, `pointerdown`, `keydown`).
   - **Offscreen Backdrop Canvas Model (`160×90`)**:
     - Renders active app wallpaper (uploaded image with exact `cover` geometry + `-28px` shell inset, all 8 presets `aurora`/`sunset`/`ocean`/`mint`/`night`/`paper`/`dots`/`bubbles`, or solid color) + `--app-wp-blur` + `--app-wp-dim` + panel base tint at `panelOpacity` + optional glass blur.
     - Exposes `window.__chatwAdaptiveSurface.refresh()` called automatically by `window.__chatwRecomputeContrast()`, resize, scroll (capture rAF), and resizer drag.
   - **Exact 1px-Accurate Top & Bottom Chat Edge Fades (`syncChatEdgeFades`)**:
     - Updates `--chat-col-w`, `--chat-col-h` for preset/custom chat wallpapers.
     - Computes exact `cover` image projection `--chat-bg-w`, `--chat-bg-h`, `--chat-bg-x`, `--chat-bg-top-y`, `--chat-bg-bot-y` for `.chat-column.wp-image`.
     - Crops exact 2D canvas strips (`--chat-top-fade-strip`, `--chat-bot-fade-strip`, `data-chat-edge-strip="true"`) for translucent `wp-none` chat panels over app-wide wallpapers.
   - **Real-Time Per-Position Element Adaptation (`updateLocalSurfaces`)**:
     - Samples each visible item's exact viewport rect `(left, top, width, height)` in the pre-composited `160×90` buffer (10 points per item, ~0.002ms/item) and updates `data-local-tone`, `data-local-scrim`, `--local-hover-bg`, `--local-scrim-bg` on scroll/resize/appearance change.
2. `app/web/index.html`, `app/build-web.sh`, `app/src/main.rs`:
   - Load and cache-bust `/adaptive_surface.js` on Web and embed via `include_str!("../web/adaptive_surface.js")` on Desktop.
3. `app/src/platform/mod.rs`:
   - Upgrade `scroll_chat_to_bottom(instant)` to invoke `window.__chatwStickToBottom.pin(instant)` (with fallback).
   - Upgrade `snapshot_chat_scroll_height()` to call `window.__chatwStickToBottom?.unpin()` before capturing scroll height so loading older history never snaps down.
   - Invoke `window.__chatwAdaptiveSurface?.refresh()` at the end of `window.__chatwRecomputeContrast()`.
4. `app/src/components/messages.rs` & `app/src/components/rich.rs`:
   - Guard `messages.rs` `onscroll` so it never fires `on_load_more` while `loading_initial` is true or `window.__chatwStickToBottom?.isPinning()` is active.
   - Use eager decoding on already-decrypted in-memory `data:`/`blob:` `EncryptedImage` tags so images lay out immediately without lazy-load stalls.
5. `app/assets/main.css`:
   - Remove white `var(--bg-surface)` + `blur(14px)` bottom band from `.composer::before` and replace with exact mirrored 1px wallpaper fade matching `.chat-header::before` across ALL chat wallpapers (`wp-none`, `wp-ocean`, `wp-sunset`, `wp-mint`, `wp-aurora`, `wp-night`, `wp-paper`, `wp-dots`, `wp-bubbles`, `wp-custom`, `wp-image`, and `data-chat-edge-strip="true"`).
   - Remove legacy `.chat-column.wp-image .chat-header::before { display: none !important; }` so uploaded chat images get exact native-resolution top & bottom fades.
   - Add zero-shift CSS rules for `[data-local-tone="dark"]`, `[data-local-tone="light"]`, `[data-local-scrim="true"]`, and `--local-hover-bg` so per-element real-time ink & frosted local hover/scrim work cleanly without ever touching `--accent` or adding layout shift.
6. `app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`:
   - Comprehensive live Chromium E2E + visual screenshots covering all 3 pillars.
