# Tasks: Full-Page Community Settings & Resizer Spacing Fix

- [x] Task 1: Fix `.panel-resizer` horizontal margin math and indicator styling in `app/assets/main.css` so column gaps are not doubled
- [x] Task 2: Support `.settings-nav` width resizing (`--settings-nav-width`) in `app/assets/main.css`, `app/web/panel_resizer.js`, and `app/web/boot.js`
- [x] Task 3: Add `data-resizer="settings-nav"` divider inside `app/src/components/settings_view.rs`
- [x] Task 4: Refactor `app/src/components/server_settings.rs` into full-page `ServerSettingsView` matching `SettingsView` layout with vertical left navigation buttons, `.settings-profile-card` in Overview, and `.settings-list` cards
- [x] Task 5: Wire `ServerSettingsView` into `main-stage` in `app/src/pages/chat.rs`, removing the modal overlay and managing navigation/mobile visibility state cleanly
- [x] Task 6: Run unit & integration tests (`cargo test -p chatw-app --lib`, `cargo test -p chatw --lib`, `cargo test -p chatw --test integration_test`)
- [x] Task 7: Build release web bundle (`bash app/build-web.sh release`)
- [x] Task 8: Verify in live browser (normal column gap, full-page Community Settings layout, tab switching, dragging resizers, closing back to chat)
- [x] Task 9: Auto commit and push to GitHub `main` branch
