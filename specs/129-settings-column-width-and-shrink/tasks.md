# Tasks 129: Settings Column Default Width & Minimum Width Synchronization

- [x] 1. Update `--settings-nav-width` in `app/assets/theme.css` to `var(--sidebar-width)`
- [x] 2. Update `.settings-nav` and `.server-settings-stage .settings-nav` in `app/assets/main.css` to `width: var(--settings-nav-width, var(--sidebar-width));` and `min-width: 0;`
- [x] 3. Update `panel_resizer.js` to set `minWidth: 180` and `fallbackWidth: 272` for `'settings-nav'`
- [x] 4. Update `boot.js` to purge old stale `294px`/`296px` defaults
- [x] 5. Rebuild Web frontend bundle with `./app/build-web.sh`
- [x] 6. Create and run Playwright E2E verification test `app/e2e/spec129-settings-column-width.mjs`
- [x] 7. Run regression tests (`spec124`, `spec128`)
- [x] 8. Mandatory auto-commit and push to `origin main` (Rule 1)
