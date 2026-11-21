# Tasks — Spec 035: Fix Voice Status Bar Icon Button & Crushed SVG

- [x] T001 Enforce `padding: 0; min-height: 0;` on `.icon-btn` and unify `.icon-btn.danger` / `.icon-btn.btn-danger` in `app/assets/main.css`.
- [x] T002 Switch voice status bar buttons in `app/src/components/navigation/sidebar.rs` from `icon-btn btn-danger` to `icon-btn danger`.
- [x] T003 Rebuild web bundle, verify voice status bar button & SVG dimensions via Playwright + visual screenshot inspection, rebuild/reinstall Linux Desktop app, and push to `origin/main`.
