# Spec 091 — Technical Plan

1. `app/assets/main.css`: Update `gap: 1px !important` -> `gap: 2px !important` on `.nav-sidebar .sidebar-scrollable`, `.nav-sidebar .friends-scroll`, `.nav-sidebar .category-block`, `.settings-nav`, `.server-settings-stage .settings-nav`, and `.settings-nav-footer` (while `.conv-list-scroll` / `:has(> .conv-row)` remains `gap: 0 !important`).
2. `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs`: Assert `gaps` are all `2` (`[2,2,2,2,2,2,2,2]`) in Settings, `gap === '2px'` in Friends & Communities, and `gap === '0px'` in Messages.
3. Rebuild `./app/build-web.sh release`, run Playwright E2E, mark tasks `[x]`, commit & push to `origin main`.
