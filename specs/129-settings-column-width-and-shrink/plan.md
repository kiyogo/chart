# Plan 129: Settings Column Default Width & Minimum Width Synchronization

## 1. CSS Adjustments
- `app/assets/theme.css`:
  - Change `--settings-nav-width: clamp(19rem, 23vw, 24rem);` to `--settings-nav-width: var(--sidebar-width);`.
- `app/assets/main.css`:
  - In `.settings-nav` and `.server-settings-stage .settings-nav`:
    - Change `width: var(--settings-nav-width, 20rem);` to `width: var(--settings-nav-width, var(--sidebar-width));`.
    - Change `min-width: 19rem;` to `min-width: 0;`.
  - In `@media (max-width: 1023px)`:
    - Change `.settings-nav { width: clamp(16.5rem, 20vw, 19rem); }` to `.settings-nav { width: var(--settings-nav-width, var(--sidebar-width)); }`.

## 2. Panel Resizer Engine Adjustments
- `app/web/panel_resizer.js`:
  - In `CONFIGS['settings-nav']`:
    - Set `fallbackWidth: 272` (matching `sidebar`).
    - Set `minWidth: 180` (matching `sidebar`).
    - Set `maxWidth: () => Math.min(500, Math.round(window.innerWidth * 0.45))` (matching `sidebar`).

## 3. Boot Script Adjustments
- `app/web/boot.js`:
  - Ensure old hardcoded `294px` / `296px` defaults are ignored/cleaned up so the new synchronized default applies immediately.

## 4. Verification
- Playwright E2E script `app/e2e/spec129-settings-column-width.mjs`:
  - Measures `.nav-sidebar` width in messages view.
  - Navigates to Settings view.
  - Measures `.settings-nav` default width and verifies it matches `.nav-sidebar` within 1px.
  - Drags `.panel-resizer[data-resizer="settings-nav"]` to the left to shrink the column.
  - Verifies that `.settings-nav` shrinks smoothly down to <= 200px (e.g. ~185px).
  - Captures screenshot artifacts for desktop default and shrunken view.
