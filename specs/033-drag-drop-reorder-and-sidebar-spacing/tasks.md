# Tasks — Spec 033: Drag-and-Drop Reordering, Distinct Plugin UIs & Sidebar Spacing

- [x] T001 Add universal `[data-drag-item]` HTML5 drag-and-drop bootstrap in `app/assets/canvas_scrollbars.js` (sync `dataTransfer.setData` + `dragover` `preventDefault`, ignores interactive child inputs/buttons).
- [x] T002 Add `PluginMod::reorder_section` and unit tests in `app/src/components/plugins_view.rs`.
- [x] T003 Refactor `app/src/components/plugin_settings.rs`: remove all index numbers & arrow buttons; add mouse drag-and-drop; give every plugin a distinct tailored UI (Notes draggable cards, Soundboard draggable Launchpad pad grid, Voxel live interactive mini-canvas, Cyber neon & live terminal actions, Tools toggle chips + draggable commands, Custom draggable blocks).
- [x] T004 Replace arrow reorder buttons with mouse drag-and-drop in `app/src/components/enterprise/settings.rs` (`Thanh bên`).
- [x] T005 Replace arrow reorder buttons with mouse drag-and-drop in `app/src/components/profile_plus_ui.rs` (`.section-list` & `.showcase-manage`).
- [x] T006 Replace arrow reorder button with mouse drag-and-drop in `app/src/components/accounts.rs` (`.account-list`).
- [x] T007 Replace layer arrow buttons with mouse drag-and-drop in `app/src/components/studio.rs` and `app/src/components/studio_engine.js`.
- [x] T008 Update `app/assets/main.css` and `app/src/components/navigation/sidebar.rs`: tighten `.sidebar-scrollable` vertical gap to `0.3rem` (exact 2/3 of `0.45rem`), unify `.community-item` & `.create-community-btn` 100% with `.chat-item`, and add drag-and-drop & plugin studio CSS styles.
- [x] T009 Update Playwright E2E suites (`app/e2e/plugin-system-check.mjs`, `app/e2e/spec030-ui-check.mjs`), build web bundle, run unit + all 4 E2E suites, and visually inspect screenshots.
- [x] T010 Rebuild and reinstall Linux desktop app (`./app/install-linux.sh`, `./app/package-deb.sh`), restart desktop app cleanly, commit and push to `origin/main`.
