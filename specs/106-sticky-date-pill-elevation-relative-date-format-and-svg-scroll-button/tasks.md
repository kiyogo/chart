# Spec 106 Tasks: Sticky Date Pill Elevation, Relative Date Formatting & Modern SVG Scroll Button

- [x] Task 1: Update `app/src/components/messages.rs` to implement `"Today"`, `"Yesterday"` and year-aware `day_label`, and replace `"↓"` with an SVG down arrow.
- [x] Task 2: Update `app/web/i18n_runtime.js` and `app/dist/i18n_runtime.js` to translate `"Today"` to `"Hôm nay"`, `"Yesterday"` to `"Hôm qua"`, and format past years as `"{dd} tháng {mm} năm {yyyy}"`.
- [x] Task 3: Update `app/assets/main.css` to elevate `.date-separator` closer to the top and lower `.scroll-to-bottom-btn` closer to the bottom composer.
- [x] Task 4: Recompile WebAssembly bundle via `./app/build-web.sh release` and sync CSS/JS to `dist/`.
- [x] Task 5: Create and execute Playwright E2E verification test `app/e2e/spec105-date-formatting-and-svg-scroll-button.mjs`.
- [x] Task 6: Commit and push all changes to GitHub `origin main` according to the mandatory auto-push rule.
