# Plan - Spec 120: Desktop 100% Seamless Layout, Zero Wallpaper Gaps & Standard Scale Harmonization

## 1. Architecture & Design Decisions
1. **Zero Shell Gap & Zero Panel Radius**:
   - In `app/assets/theme.css`:
     Change `--shell-gap: 0.75rem;` to `--shell-gap: 0px;` and `--panel-radius: 1.125rem;` to `--panel-radius: 0px;`.
   - In `app/assets/main.css`:
     - Set `.workspace-shell { padding: 0; gap: 0; }`.
     - Set `.settings-page { gap: 0; }`.
     - Set `.chat-main-container { gap: 0; }`.
     - Set `.main-stage > .main-stage { gap: 0; }`.
     - Set `.chat-column { margin-bottom: 0; border-radius: 0; }`.
     - Set `.nav-sidebar { border-radius: 0; }`.
     - Set `.settings-nav { border-radius: 0; }`.
     - Set `.settings-body { border-radius: 0; }`.
   - This ensures all adjacent panels touch flush (0px gap), completely eradicating any background wallpaper stripes between panels.
2. **Compact Row Proportions & 100% Scale**:
   - Typography: Lock `--font-size-root: 15px;` for 100% clean rendering.
   - Settings nav items:
     - Reset `.settings-nav-item`:
       `min-height: 2.125rem; height: 2.125rem; font-size: 0.875rem; padding: 0 10px; border-radius: var(--radius-md); gap: 8px;`
     - Clean active/hover pills: `background: var(--accent-soft)` for active, `background: var(--bg-hover)` for hover.
   - Channel items:
     - Reset `.channel-row`:
       `min-height: 2rem; height: 2rem; font-size: 0.875rem; padding: 0 8px; border-radius: var(--radius-sm); gap: 6px;`
     - Active/hover pills: clean subtle background instead of 12px pill lozenges.
   - Category rows:
     - Reset `.category-row`:
       `height: 28px; padding: 0 8px; font-size: 0.75rem; text-transform: uppercase; font-weight: 600;`
   - Rail items:
     - Reset `.rail-item`:
       `width: 38px; height: 38px; border-radius: var(--radius-md);`
     - Center 24px icon glyphs.
3. **Resizer Centering**:
   - Adjust `.panel-resizer`:
     `margin: 0 -4px; width: 8px; background: transparent; z-index: 20;`
     Seamless, invisible boundary line with smooth cursor and drag handle.
4. **Frosted Glass Consistency**:
   - Ensure wallpaper glass (`html[data-app-wp]` and `html[data-panel-glass="true"]`) applies clean frosted background across rail, sidebar, and body seamlessly without visual seams.

## 2. Files to Modify
- `app/assets/theme.css`: `--shell-gap: 0px;`, `--panel-radius: 0px;`, compact default heights.
- `app/assets/main.css`: `.workspace-shell`, `.settings-page`, `.chat-main-container`, `.settings-nav-item`, `.channel-row`, `.category-row`, `.panel-resizer`.
- `app/e2e/spec120-seamless-desktop-scale.mjs`: Automated Playwright test verifying desktop zero-gap, flush panels, compact dimensions, and visual harmony.
