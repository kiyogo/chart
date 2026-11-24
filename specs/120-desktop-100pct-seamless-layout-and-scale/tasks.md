# Tasks - Spec 120: Desktop 100% Seamless Layout, Zero Wallpaper Gaps & Standard Scale Harmonization

- [x] Task 1: Update design tokens in `app/assets/theme.css` (desktop framed cards `--panel-radius: 1.125rem`, `--shell-gap: 0.75rem`, compact element scale defaults).
- [x] Task 2: Refactor desktop layout containers in `app/assets/main.css` (`.workspace-shell`, `.settings-page`, `.chat-main-container`, `.panel-resizer`) for balanced framed card layout with 100% density.
- [x] Task 3: Normalize oversized items to 100% standard density (`.settings-nav-item`, `.channel-row`, `.category-row`, `.rail-item`).
- [x] Task 4: Harmonize frosted glass and borderless surfaces across all views.
- [x] Task 5: Build release web assets (`./app/build-web.sh release`) and run comprehensive E2E verification test suite (`spec120`, `spec116`, `spec115`).
- [x] Task 6: Commit and auto-push to `origin main` according to Rule 1.
