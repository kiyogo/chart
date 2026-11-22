# Task List: Spec 055

- [x] T001: Update `CONTRAST_ENGINE_JS` in `app/src/platform/mod.rs` (vivid translucent `--accent-soft`, frameless clean controls on smooth wallpapers, borderless flat secondary buttons, accurate preset luminance).
- [x] T002: Update `app/assets/main.css` (fix `.inline-form > input:not([type="color"])` width bug, remove flat grey wireframe button/switch borders on smooth surfaces).
- [x] T003: Update default `wp_color` in `app/src/components/settings_more.rs` to fresh `#60A5FA`.
- [x] T004: Update `app/e2e/spec052-visual-and-contrast-audit.mjs` to assert vivid visible active pills on `Cực quang` 100% + `Hoàng hôn` 72%, frameless smooth segmented controls, compact color input, and 0.00px size shift on busy photo transition.
- [x] T005: Build web release (`./app/build-web.sh release`), run all 4 Playwright E2E suites (`spec052`, `spec053`, `spec048`, `spec050`), visually inspect screenshots of `Cực quang` 100%, `Hoàng hôn` 72% & 100%, `Đại dương` 100%, `Đêm sao` 100%, and `Sáng`.
- [x] T006: Mark complete, commit, and push to `origin main`.
