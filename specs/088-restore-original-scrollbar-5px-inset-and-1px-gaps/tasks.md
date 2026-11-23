# Spec 088 — Tasks

- [x] **T1**: Restore `app/web/canvas_scrollbars.js` 100% back to original constant `3px` pixel-snapped centered scrollbar (`commit 6305366` / `0c31331`).
- [x] **T2**: Update `app/assets/main.css` to `--panel-inset: 5px` (equal left/right inset giving `1px` white + `3px` thumb + `1px` white) and exact `gap: 1px !important` across all left sidebar lists (Messages, Friends, Communities, Channels, Settings, Server Settings, `.settings-nav-footer`).
- [x] **T3**: Update `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs` (plus zoomed pixel inspection of the scrollbar & 1px gap), rebuild release WASM (`./app/build-web.sh release`), run Rust tests + Playwright E2E, and inspect live zoomed screenshots via `read_file`.
- [x] **T4**: Mark all tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
