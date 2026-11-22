# Implementation Plan: Spec 065 — Clean Chat Scroll Mask & Zero Header-Message Collision

1. **`app/assets/main.css`**:
   - Tighten `.chat-header::before` (`height: var(--header-height)`, `mask-image: linear-gradient(to bottom, #000 0%, #000 84%, transparent 100%)`).
   - Tighten `.composer::before` (`top: -6px`, `mask-image: linear-gradient(to top, #000 0%, #000 86%, transparent 100%)`).
   - Add crisp viewport `mask-image` / `-webkit-mask-image` on `.chat-column:has(> .chat-messages):not(:has(> .pinned-bar, > .chat-banner)) > .chat-messages:not(:has(.lightbox, .modal-overlay))` and `.chat-column:has(> .chat-messages):has(> .pinned-bar, > .chat-banner) > .chat-messages:not(:has(.lightbox, .modal-overlay))` so messages vanish cleanly BEFORE reaching the header band (`0..var(--header-height)`) and BEFORE colliding with the composer box, with zero milky white smear over bubbles.
2. **`app/web/adaptive_surface.js`**:
   - Match `hdrH = hdr.getBoundingClientRect().height` and `compH = comp.getBoundingClientRect().height + 6` for exact 1px wallpaper strip projection.
3. **Verification (`app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`)**:
   - Add a dedicated assertion + screenshot verifying `.chat-messages` mask cleanly blocks scrolled blue message bubbles from ever overlapping header icons (no washed-out milky smudge, crisp header background behind `📞`, `📹`, `◫`, `...`).
   - Rebuild `./app/build-web.sh release`, run all 6 Playwright E2E suites, visually inspect screenshots via `read_file`, and push to `origin main`.
