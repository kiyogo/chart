# Implementation Tasks — Spec 042: Intuitive Panel Transparency Scale, Optional Glass Blur Toggle & Live Blur Responsiveness

- [x] **T001: Model & Schema Update in `app/src/prefs.rs`**
  - Add `#[serde(default)] pub panel_glass: bool` to `Prefs`.
  - Set default `panel_glass: false` in `Prefs::default()`.
  - Retain `panel_glass` in `sanitize()` without altering serialization.

- [x] **T002: Platform CSS & DOM Attributes in `app/src/platform/mod.rs`**
  - Update `apply_theme_and_prefs` to output:
    - `--panel-opacity: format!("{}%", p.panel_opacity)`
    - `--panel-blur: format!("{}px", if p.panel_glass { p.panel_blur } else { 0 })`
    - `data-panel-glass: (p.panel_glass && p.panel_opacity < 100).to_string()`
    - `data-panel-translucent: (p.panel_opacity < 100).to_string()`

- [x] **T003: CSS Styling in `app/assets/main.css`**
  - Ensure translucent panel backgrounds apply under `html[data-panel-translucent="true"]` and `html[data-panel-glass="true"]`.
  - Ensure backdrop blur (`backdrop-filter: blur(var(--panel-blur, 12px))`) only applies under `html[data-panel-glass="true"]`.

- [x] **T004: Settings UI Overhaul in `app/src/components/settings_more.rs`**
  - Invert slider to `Độ trong suốt khung trắng: {panel_transparency}%` (0% = solid, 100% = fully transparent).
  - Add `Toggle` switch for `Làm mờ kính khung (Frosted glass)`.
  - When switch turned ON and transparency is 0%, automatically set transparency to 40% (`panel_opacity = 60`).
  - Conditionally display blur slider `Mức độ làm mờ kính: {pr.panel_blur}px` only when `pr.panel_glass == true`.
  - Reset `p.panel_glass = false` on "Khôi phục giao diện mặc định".

- [x] **T005: Playwright Automated E2E Test Suite in `app/e2e/spec042-transparency-toggle-blur.mjs`**
  - Verify default state (0% transparency, glass blur toggle is OFF, zero blur).
  - Test dragging transparency slider from 0% to 50% without blur.
  - Test toggling ON glass blur switch, verifying `data-panel-glass="true"` and blur slider appearance.
  - Test adjusting blur slider and verifying `--panel-blur` live update.
  - Test "Khôi phục giao diện mặc định" button.

- [x] **T006: Rebuild, Regression Verification & Packaging**
  - Run `./app/build-web.sh release`.
  - Run Playwright test suite `app/e2e/spec042-transparency-toggle-blur.mjs` and previous regression suites.
  - Rebuild desktop binary and package `.deb`.
  - Commit and git push to `origin/main` per AGENTS.md Rule #1.
