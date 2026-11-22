# Tasks 068 — Soft Per-Pixel Cloud Mist Edge Fade

- [x] T001 Replace hard solid bar masks on `.chat-header::before`, `.composer::before`, and `.chat-messages` in `app/assets/main.css` with smooth multi-stop cubic-eased cloud mist dissolves.
- [x] T002 Synchronize 2D per-pixel canvas strip & photo wallpaper crop heights in `app/web/adaptive_surface.js` (`hdrH + 30`, `compH + 30`) so every `(x, y)` mist pixel matches the wallpaper 1:1.
- [x] T003 Update `spec052` and `spec061` E2E assertions for cubic cloud mist & capture visual verification screenshots across White, Ocean, and Translucent multi-color wallpapers.
- [x] T004 Rebuild release WASM (`./app/build-web.sh release`), run all active Playwright E2E suites, and visually inspect screenshots via `read_file`.
- [x] T005 Commit and push to `origin main` per `AGENTS.md` Rule #1.
