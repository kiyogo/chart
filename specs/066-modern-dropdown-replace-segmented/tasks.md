# Tasks 066 — Modern Web Dropdown Replacement

- [x] T001 Upgrade `app/web/custom_pickers.js` with W3C APG Listbox keyboard navigation (`ArrowDown`/`ArrowUp`/`Home`/`End`/`Enter`/`Space`/`Escape`/typeahead), `aria-expanded`, `data-side` positioning, `translate="no"` / `.notranslate`, and RTL edge alignment.
- [x] T002 Replace `Segmented` with modern `SelectMenu` (`<select class="cw-dropdown">`) across all 12 call sites in `app/src/components/settings_more.rs` and convert Settings rows to sleek horizontal `.settings-row` layout.
- [x] T003 Replace `.segmented` in `app/src/components/settings_view.rs` (`Display density`), `profile_modal.rs`, `settings_extra.rs`, `avatar_motion.rs`, `studio.rs`, and `draw_studio.rs` with `<select class="cw-dropdown">`.
- [x] T004 Replace all `.ent-segmented` bars in `app/src/components/enterprise/{hr_dept,tasks_projects,time_leave}.rs` with `<select class="cw-dropdown">`.
- [x] T005 Remove obsolete `.segmented` / `.ent-segmented` rules in `app/assets/main.css` (and stale comment references in `app/web/adaptive_surface.js`, `app/src/platform/mod.rs`) and add cohesive Spec 066 modern web dropdown trigger + popover styling (light/dark/translucent/RTL/open-chevron).
- [x] T006 Update E2E suites (`spec052`, `spec062`) to assert zero `.segmented` / `.ent-segmented`, verify modern dropdown interaction + low-opacity translucent selected highlight + Blue/Violet sync + Vietnamese/Arabic RTL, and capture visual verification screenshots.
- [x] T007 Run JS syntax checks, `cargo test -p chatw-app --lib`, `./app/build-web.sh release`, and all Playwright E2E suites; visually inspect screenshots via `read_file`.
- [x] T008 Commit and push to `origin main` per `AGENTS.md` Rule #1.
