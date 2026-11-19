# Tasks: Resizable UI Panels

- [x] Task 1: Create `app/web/panel_resizer.js` with smooth drag resizing, limits, double-click reset, and localStorage persistence
- [x] Task 2: Update `app/assets/main.css` to define `--sidebar-width`, `--details-width`, resizer styles, and make `.server-members-panel` responsive to `--details-width`
- [x] Task 3: Update `app/web/boot.js` to restore saved panel widths on initial boot before paint
- [x] Task 4: Insert left and right resizer handles into `app/src/pages/chat.rs`
- [x] Task 5: Update `app/web/index.html` and `app/build-web.sh` to include `panel_resizer.js` in the release web bundle
- [x] Task 6: Run tests (`cargo test -p chatw-app --lib` and `cargo test -p chatw --lib`)
- [x] Task 7: Build release web bundle (`bash app/build-web.sh release`)
- [x] Task 8: Verify in live browser that dragging resizes the sidebar and details panel smoothly
- [x] Task 9: Auto commit and push to GitHub `main` branch
