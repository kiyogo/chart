# Tasks: Spec 070 — Subtle Compact Top & Bottom Chat Edge Mist

- [x] T001 Tighten & lighten `.composer::before`, `.chat-header::before`, and `.chat-messages` top/bottom edge mist in `app/assets/main.css`
- [x] T002 Synchronize `topMistH = hdrH + 10` and `botMistH = compRectH + 4` in `app/web/adaptive_surface.js`
- [x] T003 Update mist assertions in `app/e2e/spec052-visual-and-contrast-audit.mjs` and `app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`, and add in-flight outbox dedup guard in `app/src/pages/chat.rs`
- [x] T004 Rebuild release WASM bundle (`./app/build-web.sh release`), run all 6 E2E suites, visually inspect top/bottom screenshots via `read_file`
- [x] T005 Mark tasks complete, commit, and `git push origin main`
