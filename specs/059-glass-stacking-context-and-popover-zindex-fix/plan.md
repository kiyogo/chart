# Plan 059 — Unified Glass Stacking Contexts & Popover Surface Architecture

## Target Files
- `app/assets/main.css`
- `app/e2e/spec059-glass-stacking-popovers.mjs` (new live Chromium E2E + visual screenshot suite)

## Technical Design

### 1. `.app-rail` Pseudo-Element Glass Layer & Z-Index Ladder (`app/assets/main.css`)
- On `.app-rail`:
  - Set `position: relative; z-index: 30; overflow: visible;` (strictly above base panels `1` and `.panel-resizer` `15`, below drawers `60` and modals `1000`).
  - Set `.app-rail:hover, .app-rail:focus-within { z-index: 80; }` so rail tooltips paint above every panel, resizer, and drawer while remaining below any open `z-index: 100` popover in another panel.
  - Set `.app-rail:has(.account-menu, .rail-menu, .menu-dismiss) { z-index: var(--z-popover); }` (`100`) so Download QR hover card and Account menu always paint cleanly on top.
  - Set `.app-rail:has(.modal-overlay, .settings-modal-backdrop, .lightbox) { z-index: var(--z-modal); }` (`1000`).
- Move glass blur off `.app-rail` onto `.app-rail::before`:
  ```css
  html[data-panel-glass="true"] .app-rail {
    background: transparent !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
  }
  html[data-panel-glass="true"] .app-rail::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: color-mix(in srgb, var(--bg-primary) var(--panel-opacity, 100%), transparent);
    backdrop-filter: blur(var(--panel-blur, 12px));
    -webkit-backdrop-filter: blur(var(--panel-blur, 12px));
    z-index: -1;
    pointer-events: none;
  }
  ```
- Remove duplicate nested `html[data-panel-glass="true"] .rail-item.active { backdrop-filter: ... }` (lines 3833–3836).

### 2. Sibling Workspace Panel Base Stacking & Popover/Modal Elevation (`app/assets/main.css`)
- Give all workspace panels zero-specificity base stacking:
  ```css
  :where(
    .nav-sidebar, .settings-nav, .settings-body, .chat-column, .details-panel,
    .partner-info-sidebar, .server-members-panel, .right-panel,
    .notifications-page, .download-page, .search-page,
    .feed-column, .meet-column, .vico-chat-column, .plugin-panel
  ) {
    position: relative;
    z-index: 1;
  }
  ```
- Elevate any panel hosting an active popover/dropdown/fixed dismiss backdrop to `z-index: var(--z-popover)` (`100`), and any panel hosting a modal/lightbox (`.modal-overlay, .settings-modal-backdrop, .lightbox`) to `z-index: var(--z-modal)` (`1000`).
- Extend the `:not(:has(...))` glass exclusion on sibling panels (lines 3779–3791 & 9592–9598) to also exclude `.menu-dismiss, .header-search-backdrop, .profile-popover` so `position: fixed` backdrops/popovers rendered inside glass panels ALWAYS span the full viewport (`1440×900`).
- Add `.conv-row:has(.account-menu, .menu-dismiss) { z-index: var(--z-popover); }` so conversation row menus always paint above sibling rows.

### 3. Crisp Tooltip, Rail Popover & Floating Surface Contrast (`app/assets/main.css`)
- Refine `.rail-nav [data-tip]:hover::after, .rail-bottom [data-tip]:hover::after` with `left: calc(100% + 10px);`, crisp drop shadow `0 4px 14px rgba(15, 23, 42, 0.22)`, and `text-shadow: none !important;`.
- Add `border-radius: var(--radius-md); overflow: hidden;` on `.download-popover-qr`.
- Upgrade `.picker-popover`, `.profile-popover`, `.header-search-popover`, `.suggest-list`, `.paste-choice` from translucent `var(--bg-surface-hover)` to opaque `var(--bg-elevated)` with subtle crisp border/shadow and dark/light adaptive tone rules matching `.account-menu`.
- Exempt `.account-menu`, `.rail-menu`, `.download-rail-menu`, `.cw-dropdown-menu`, `.picker-popover`, `.profile-popover`, `.header-search-popover`, `.ent-search-popover`, `.suggest-list`, and rail tooltips from `data-text-halo="true"` `text-shadow`.

### 4. Build & Verification Plan
1. Rebuild release WASM bundle via `./app/build-web.sh release`.
2. Write & run `app/e2e/spec059-glass-stacking-popovers.mjs` testing all rail tooltips, Download QR popover hit-testing & clicking, Account menu hit-testing & full-viewport dismiss, Sidebar `+` menu & dismiss in Glass+Wallpaper mode, Chat AttachMenu & Emoji picker opacity/stacking in Glass+Wallpaper mode, Modal stacking above `.app-rail`, and Dark/Light themes — capturing screenshots and inspecting every screenshot visually via `read_file`.
3. Re-run `spec040`, `spec052`, `spec053`, `modal-overlay-check` to prove zero regressions.
4. Commit and push to `origin main`.
