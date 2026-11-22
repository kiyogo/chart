# Tasks 063

- [x] T001 Update `app/assets/main.css` to render all active pills, nav items, rail active icon, segmented active options, language tile, and selected chat rows with low-opacity translucent `var(--accent-soft)` tint + readable surface-matched ink, and soften translucent control/input surfaces.
- [x] T002 Update `app/web/adaptive_surface.js` and `app/src/platform/mod.rs` to keep low-opacity `--accent-soft`, soft translucent button/input tokens, and per-position ink adaptation on translucent active nav/rail rows while keeping `data-local-scrim` deleted and `.btn-secondary`/`.segmented` protected.
- [x] T003 Update E2E assertions in `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs` and `app/e2e/spec052-visual-and-contrast-audit.mjs` to enforce low-opacity translucent active pills (rejecting solid 100% fills).
- [x] T004 Build release WASM via `./app/build-web.sh release`, run all 6 Playwright E2E suites, and visually inspect screenshots via `read_file`.
- [x] T005 Commit and push all changes to `origin main` (`https://github.com/kiyogo/chart`).
