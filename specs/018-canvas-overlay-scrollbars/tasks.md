# Tasks: Canvas Overlay Scrollbars

- [x] Task 1: Create `app/web/canvas_scrollbars.js` implementing the Canvas scrollbar engine with proximity and scroll detection
- [x] Task 2: Update `app/assets/main.css` to hide native scrollbar thumb by default and avoid broad `*:hover` reveal
- [x] Task 3: Update `app/web/index.html` and `app/build-web.sh` to include `canvas_scrollbars.js` in the distribution bundle
- [x] Task 4: Run unit and integration tests (`cargo test -p chatw-app --lib`, `cargo test -p chatw --lib`, `cargo test -p chatw --test integration_test`)
- [x] Task 5: Build release web bundle (`bash app/build-web.sh release`)
- [x] Task 6: Verify in browser that scrollbars are invisible when cursor is away, reveal when near edge or when scrolling, and dragging works smoothly
- [x] Task 7: Auto commit and push to GitHub `main` branch
