# Implementation Plan: Flat Borderless & Shadowless UI Pass

## Architecture Impact
Pure CSS token and surface update in `app/assets/theme.css` and `app/assets/main.css` (Dioxus asset-bundled stylesheet compiled via `dx build` / `./app/build-web.sh`). No Rust logic or API changes required.

## File Changes
1. **`app/assets/theme.css`**
   - Set `--shadow-sm: none;`, `--shadow-md: none;`, `--shadow-lg: none;` in `:root` (light theme) and `[data-theme="dark"]` (dark theme) so no UI component inherits a dark blurry drop-shadow edge anywhere.
2. **`app/assets/main.css`**
   - `.feed-card`: remove `box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);` -> `box-shadow: none;`.
   - `.meet-column`: change `background: var(--bg-primary);` -> `background: var(--bg-secondary);` so white `.feed-card` / `.meet-card` blocks stand out cleanly on light cool gray (`#F4F6F9`) without shadows or borders.
   - `.composer-box`: remove `box-shadow: var(--shadow-md);` -> `box-shadow: none;`; slightly refine `.composer-box::before` fill (`color-mix(in srgb, var(--bg-input) 96%, var(--bg-primary))`).
   - `.note-bubble` & `.note-plus-badge`: remove `box-shadow: 0 2px 8px rgba(0, 0, 0, 0.13);` and `0 1px 4px rgba(0, 0, 0, 0.18);`; use `background: var(--bg-surface-hover)` for `.note-bubble` (+ `::before`/`::after`) and clean cutout `box-shadow: 0 0 0 2px var(--bg-sidebar)` for `.note-plus-badge`.
   - `.studio-tools`, `.studio-tools:hover`, `.studio-swatch`: remove hairline dark border (`border: 1px solid ...`, `border-color: ...`) and dark shadow (`0 8px 30px rgba(0, 0, 0, 0.16)`, `inset 0 0 0 1px rgba(0,0,0,0.12)`).
   - `input[type="range"]::-webkit-slider-thumb` & `::-moz-range-thumb`: replace `0 1px 4px rgba(0, 0, 0, 0.35), 0 0 0 2px var(--bg-primary)` with crisp `0 0 0 2px var(--bg-primary)`.
   - `input[type="checkbox"].switch::after` & `.voxel-cell.filled`: remove `rgba(0, 0, 0, ...)` dark shadows (`box-shadow: none;`).
   - `.settings-select`: replace `border: 1px solid var(--border-subtle); background: var(--bg-surface);` with `border: none; background: var(--bg-input);`.
   - `.details-recovery-box`: remove `border-top: 1px solid var(--border-subtle);`.
   - `.header-server-search:focus-within`: remove `box-shadow: 0 0 0 1px var(--border-strong);` and keep clean `background: var(--bg-surface-hover);`.
   - Floating popovers/menus without full-screen backdrop (`.account-menu`, `.picker-popover`, `.profile-popover`, `.header-search-popover`, `.ent-search-popover`, `.suggest-list`, `.paste-choice`, `.message-actions`, `.react-tray`, `.plugin-floating-hud`): shift surface from `var(--bg-elevated)` (`#FFFFFF`) to tonal flat `var(--bg-surface-hover)` (`#F3F5F8` / `#21252C`) where overlapping white chat/sidebar canvases, and keep `box-shadow: none` everywhere! (Inputs inside `.picker-popover` use `var(--bg-primary)` for clean separation.)
   - `.voice-controls-dock`: use `background: var(--bg-surface-hover); box-shadow: none;` so it contrasts cleanly on `var(--bg-chat)`.
   - Responsive `.details-panel` / `.partner-info-sidebar`: `box-shadow: none;` (already backed by `.drawer-backdrop` `--bg-overlay`).

## Verification Plan
- Rebuild web bundle via `./app/build-web.sh`.
- Run existing E2E regression check: `node app/e2e/scrollbar-switch-check.mjs`.
- Run targeted Playwright inspection script verifying computed `boxShadow === 'none'` and no dark `border` across MeetView, FeedView, Chat/Composer, Sidebar/Notes, Settings/Storage, Modals, and Popovers, plus visually inspect screenshots.
