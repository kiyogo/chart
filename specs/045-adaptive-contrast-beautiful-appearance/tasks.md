# Implementation Tasks — Spec 045: Adaptive Contrast & Foolproof Aesthetic Compensation Engine

- [x] **T001: Implement Real-Time Adaptive Contrast & WCAG Color Compensator in `app/src/platform/mod.rs`**
  - Add unified `window.__chatwRecomputeContrast()` engine triggered from both `apply_appearance()` and `apply_customization()`, plus `prefers-color-scheme` changes and offscreen 32×32 canvas wallpaper pixel sampling.
  - Compute `data-surface-tone` (`light` | `dark`), `data-rail-tone` (`light` | `dark`), `data-wp-busy`, `data-panel-scrim`, `data-rail-scrim` accurately from composite wallpaper + dim + panel opacity luminance.
  - Implement WCAG 2.1 HSL lightness compensation for `custom_accent` (`--accent`, `--accent-hover`, `--on-accent`, `--accent-text`, `--accent-soft`, `--accent-soft-strong`) and custom bubbles (`--msg-sent`, `--msg-sent-text`, `--msg-sent-meta`, `--msg-recv`, `--msg-recv-text`, `--msg-recv-meta`) with bubble-vs-background separation guard.

- [x] **T002: Add Adaptive Surface Tokens, Protective Scrims & Visual Polish in `app/assets/main.css`**
  - Add `html[data-theme="light"][data-surface-tone="dark"]` and `html[data-theme="dark"][data-surface-tone="light"]` adaptive surface & ink overrides.
  - Add `html[data-rail-tone="dark"]` / `light` and `html[data-rail-scrim="true"]` adaptive rail icon contrast rules.
  - Upgrade translucent card/row/tile/nav/heading/chat-header/chat-item protective surfaces under `data-panel-scrim="true"` & `data-wp-busy="true"` without forcing glass blur when `data-panel-glass="false"` and keeping borderless/shadowless flat design.
  - Fix `.settings-group-title` inline icon alignment (`display: flex; align-items: center; gap: 8px;`).
  - Add full dark-surface readability tokens to `.chat-column.wp-night` and `.settings-preview-chat.wp-night`.

- [x] **T003: Live Chat Wallpaper Sync in `app/src/components/settings_view.rs`**
  - Bind `wp-{prefs.wallpaper}` to `.settings-preview-chat` so default chat wallpaper and compensated bubble colors preview live.

- [x] **T004: Automated E2E Contrast & Visual Verification (`app/e2e/spec045-adaptive-contrast.mjs`)**
  - Create Playwright test covering all 8 stress scenarios (including busy anime-style high-contrast photo, 80% dim in light mode, pure white wallpaper in dark mode, neon/white/black accents & bubbles, `wp-night` in light mode) and capture screenshots to visually inspect!
  - Visually inspect generated screenshots with `read_file` to guarantee perfection!

- [x] **T005: Build Release Bundles, Run Full Regression Suite & Auto-Push**
  - Build Web WASM release (`./app/build-web.sh release`).
  - Run `spec045`, `spec044`, `spec043`, `spec042`, `spec041`, `spec040`, `borderless-shadowless-check`.
  - Rebuild Linux desktop release & `.deb` package.
  - Commit and `git push origin main` per AGENTS.md Rule #1.
