# Tasks — Spec 038: Chat Full-Bleed Bottom Canvas & Redesigned Larger Icons

- [x] T001 Remove bottom shell strip clipping on `.chat-column` so messages and images extend flush (`0px`) to the bottom edge of the viewport (`app/assets/main.css`).
- [x] T002 Redesign SVG icons in `app/src/components/icons.rs` ("ChatW Duo v2") with bolder `2.05` strokes, fuller `2..22` optical geometry, clean single-stroked paths, `0.22` duotone depth, and transparent `evenodd` center cutout on `SettingsIcon`.
- [x] T003 Enlarge navigation rail icons (`30px` brand, `28px` top nav, `26px` bottom nav) in `app/src/components/navigation/rail.rs`, composer icons (`24px`) in `app/src/components/composer.rs` / `rich.rs`, chat header icons (`22px`) in `app/src/pages/chat.rs`, uniform message hover icons (`19px`) in `app/src/components/messages.rs`, and rail button sizing in `app/assets/main.css`.
- [x] T004 Rebuild Web WASM release, run new Playwright visual & geometry verification (`app/e2e/spec038-chat-bottom-icons.mjs`) + inspect Light/Dark screenshots + run all existing E2E regression suites (`spec030`, `borderless-shadowless`, `scrollbar-switch`, `plugin-system`).
- [x] T005 Rebuild & install Linux Desktop release + `.deb`, mark tasks complete, commit, and push to `origin/main`.
