# Tasks 062

- [x] T001 Build complete, genuine 32-language JSON locale packs (`app/web/locales/{code}.json`) covering all core UI screens & dynamic templates with zero empty values and zero `...`.
- [x] T002 Implement `app/web/i18n_runtime.js` (Dioxus-safe `WeakMap` DOM/attribute/template translator with strict user-content exclusions, caching, RTL support, and instant boot language restoration).
- [x] T003 Update `app/src/i18n.rs`, `app/src/lib.rs`, `app/src/components/settings_view.rs`, `app/src/services/settings_sync.rs`, `app/web/index.html`, and `app/build-web.sh` to wire all 33 languages seamlessly across Rust + Web runtime + reload persistence + build bundle.
- [x] T004 Update `app/web/adaptive_surface.js` to delete `data-local-scrim` / `--local-scrim-bg` completely and exclude filled buttons & active pills from `data-local-tone` text inversion.
- [x] T005 Update `app/src/platform/mod.rs` to remove `data-framed-controls` per-button white box clutter, remove milky busy-photo `--accent-soft` desync, and keep `--on-accent` contrast guaranteed for all presets and custom colors.
- [x] T006 Update `app/assets/main.css` to: (a) remove `[data-local-scrim]` & `[data-framed-controls]` clutter, (b) synchronize all active pills/tabs/buttons/rail/theme rings with `--accent` + `--on-accent`, (c) unify `.segmented` into one tight sleek pill track, (d) refine `.btn-secondary`/`.btn-ghost`/`.btn-avatar-upload` & all inputs (`khung nhập`), (e) eliminate `Bahasa Indone...` truncation in `.lang-grid`, and (f) polish RTL rules.
- [x] T007 Run `./app/build-web.sh release` and verify clean compilation + locale bundling.
- [x] T008 Write & execute `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs` (9/9 PASS), inspect all rendered screenshots visually via `read_file`, and run regression suites (`spec052` 12/12, `spec053` 4/4, `spec059` 12/12, `spec061` 6/6, `modal-overlay-check` 7/7 — 50/50 total PASS).
- [x] T009 Commit and push all changes to `origin main` (`https://github.com/kiyogo/chart`).
