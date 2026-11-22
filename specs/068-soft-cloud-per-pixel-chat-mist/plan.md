# Plan 068 — Soft Per-Pixel Cloud Mist Edge Fade

## Files to Modify
1. **`app/assets/main.css`**:
   - Update `.chat-column:has(> .chat-messages) > .chat-header::before`:
     - `height: calc(var(--header-height) + 30px)`
     - Smooth 6-stop cubic cloud `mask-image`: `linear-gradient(to bottom, rgba(0, 0, 0, 0.86) 0%, rgba(0, 0, 0, 0.66) 22%, rgba(0, 0, 0, 0.40) 46%, rgba(0, 0, 0, 0.18) 68%, rgba(0, 0, 0, 0.05) 86%, transparent 100%)`
   - Update `.composer::before`:
     - `top: -30px; bottom: 0;`
     - Smooth 6-stop cubic cloud `mask-image`: `linear-gradient(to top, rgba(0, 0, 0, 0.86) 0%, rgba(0, 0, 0, 0.66) 22%, rgba(0, 0, 0, 0.40) 46%, rgba(0, 0, 0, 0.18) 68%, rgba(0, 0, 0, 0.05) 86%, transparent 100%)`
   - Update `.chat-column:has(> .chat-messages) > .chat-messages` `mask-image` (both with and without pinned bar):
     - Smooth 8-stop cubic cloud dissolve at top (`0px -> 84px`) and bottom (`calc(100% - 92px) -> 100%`) so bubbles dissolve smoothly like drifting through soft mist rather than hitting a hard white cutoff.
2. **`app/web/adaptive_surface.js`**:
   - In `syncChatEdgeFades()`: update `topMistH = hdrH + 30` and `botMistH = compRect.height + 30` (matching CSS `calc(var(--header-height) + 30px)` and `top: -30px`) for both `wp-image` (`--chat-bg-bot-y`) and `cropPanelStripDataUrl` (`--chat-top-fade-strip`, `--chat-bot-fade-strip`) so per-pixel 2D wallpaper alignment is 1:1 exact across the entire cloud mist zone!
3. **`app/e2e/spec052-visual-and-contrast-audit.mjs` & `app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`**:
   - Update height & cloud-mask assertions to assert zero hard solid bar (`!mask.includes('84%')`, multi-stop cubic cloud alpha curve, `hdrBeforeH` matches `hdrH + 30`) and capture scrolled-bubble screenshots on White chat (`wp-none`), Ocean chat (`wp-ocean`), AND Translucent multi-color wallpaper chat (`data-chat-edge-strip="true"`) in `verify_068/`.
