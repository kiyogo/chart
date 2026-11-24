# Spec 116 Tasks

- [x] T001: Tag redundant mobile `GIF`/`Sticker` shortcut buttons with `composer-media-shortcut` in `app/src/components/composer.rs` and verify Lightbox zoom control classes in `app/src/components/rich.rs`.
- [x] T002: Implement comprehensive mobile (`<= 767px` & `<= 640px`) responsive UI/UX polish in `app/assets/main.css` (touch ergonomics, safe-area insets, bottom tab bar, chat header ellipsis fix, single-line composer, edge-to-edge mobile Lightbox, bottom-sheet modals/menus, Details drawer, Settings, Auth).
- [x] T003: Build release WASM (`./app/build-web.sh release`), write & run `app/e2e/spec116-mobile-ui-optimization.mjs` across `390×844` and `360×800`, visually inspect all before/after mobile screenshots, and verify desktop regression suites (`spec115`, `spec114`, `spec113`).
- [x] T004: Mark all tasks `[x]` and auto-commit + push to `origin main` per `AGENTS.md`.
