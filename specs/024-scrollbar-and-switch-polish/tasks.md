# Tasks (024)

- [x] T001 Inspect `canvas_scrollbars.js`, `panel_resizer.js`, and `app/assets/main.css` and measure rendered pixel geometry from screenshots.
- [x] T002 Upgrade `app/web/canvas_scrollbars.js` to thin `3px` idle/scroll pill, smooth `3px -> 8px` expansion on scrollbar hover/drag, and `elementsFromPoint` hit-testing past overlapping resizers.
- [x] T003 Fix `input[type="checkbox"].switch` geometric centering (`top: 50%` + `translate3d` + exact `px` sizing) and fix `.ent-btn-group > input` / `.ent-toolbar > input` exclusions in `app/assets/main.css`.
- [x] T004 Rebuild web release bundle (`./app/build-web.sh release`) and verify with Chromium Playwright script (measuring every switch box + knob offset and scrollbar hover width) and full cargo tests.
- [x] T005 Auto-commit and push all changes to `origin main`.
