# Tasks 065

- [x] T001 Tighten `.chat-header::before` & `.composer::before` and add crisp `.chat-messages` viewport edge mask (with `.pinned-bar`/`.chat-banner` and `:not(:has(.lightbox, .modal-overlay))` guards) in `app/assets/main.css`.
- [x] T002 Align `hdrH` and `compH` edge-strip heights in `app/web/adaptive_surface.js`.
- [x] T003 Add scrolled-bubble-under-header visual & DOM assertion in `app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`.
- [x] T004 Build release WASM (`./app/build-web.sh release`), run all 6 E2E suites, visually inspect screenshots of scrolled blue bubbles near the chat header via `read_file`, and commit + push to `origin main`.
