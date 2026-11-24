# Spec 117 Tasks

- [x] T001: Tag core bottom items (`rail-mobile-in-bottom`) and add `.mobile-bottom-bar` (5 core tabs: Messages, Friends, Search, Notifications, Account) in `app/src/components/navigation/rail.rs`.
- [x] T002: Update mobile CSS (`<= 767px`) in `app/assets/main.css` so `.app-rail` stays as a sleek `56px` vertical rail on the LEFT, `.mobile-bottom-bar` sits cleanly at the BOTTOM, `.nav-sidebar` scrolls cleanly beside the left rail and above the bottom bar, and center `.profile-close` `X`.
- [x] T003: Build release WASM (`./app/build-web.sh release`), update & run `app/e2e/spec116-mobile-ui-optimization.mjs` across `390×844` and `360×800`, visually inspect screenshots, and run desktop regression (`spec115`).
- [x] T004: Mark all tasks `[x]` and auto-commit + push to `origin main` per `AGENTS.md`.
