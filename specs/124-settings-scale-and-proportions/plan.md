# Technical Plan: Spec 124 - Normalize Settings Scale, Navigation Proportions and Typography

## Architecture & Design
1. **CSS Variables (`app/assets/theme.css`)**:
   - Add `--settings-nav-width: clamp(18rem, 21vw, 22rem);` in `:root`.
2. **Main Stylesheet (`app/assets/main.css`)**:
   - Update `.settings-nav`:
     - `width: var(--settings-nav-width, 18.5rem);`
     - `min-width: 17.5rem;`
     - `padding: var(--panel-inset, 0.5rem) 8px;`
   - Update `.server-settings-stage .settings-nav`:
     - `width: var(--settings-nav-width, 18.5rem);`
     - `min-width: 17.5rem;`
   - Update `@media (max-width: 1023px)`:
     - Change `.settings-nav { width: 13rem; }` to `.settings-nav { width: clamp(16.5rem, 20vw, 19rem); }`.
   - Update `.settings-nav-item`:
     - `min-height: 2.5rem; height: 2.5rem;`
     - `gap: 12px;`
     - `padding: 0 14px;`
     - `border-radius: var(--radius-lg);`
     - `font-size: var(--text-md);`
   - Update line 5207:
     - `.settings-nav-item { min-height: 2.5rem; height: 2.5rem; white-space: nowrap; }`
   - Update `.settings-search`:
     - Input field `min-height: 2.375rem; font-size: var(--text-md); padding-left: 2.35rem; border-radius: var(--radius-lg);`
   - Update `.settings-nav-item svg`:
     - `width: 20px; height: 20px; flex-shrink: 0;`
3. **Resizer Engine (`app/web/panel_resizer.js`)**:
   - Update fallback and minimum dimensions:
     - `fallbackWidth: 296`
     - `minWidth: 240`
     - `maxWidth: () => Math.min(460, Math.round(window.innerWidth * 0.4))`
4. **Verification**:
   - Write Playwright E2E test `app/e2e/spec124-settings-scale.mjs`:
     - Measure `.settings-nav` rendered width >= 280px.
     - Measure `.settings-nav-item` rendered height >= 40px and font size >= 14px.
     - Ensure no text overflow/clipping for Vietnamese settings items.
     - Capture screenshot in artifact directory.
