# Spec 120: Desktop 100% Seamless Layout, Zero Wallpaper Gaps & Standard Scale Harmonization

## 1. Context & Problem Statement
- **User Feedback**:
  - *"nó bị hiện quá là to rồi á, tất cả hiện bằng 100% và đồng bộ giao diện với nhau đi"*
  - User uploaded desktop screenshots (`media_1795529637004_1d9d4e3c.png` and `media_1795529661828_6c913182.png`) showing:
    1. Harsh, bright orange wallpaper stripes dividing columns:
       - Stripe 1 between Left Rail (`.app-rail`) and Sidebar (`.nav-sidebar` or `.settings-nav`).
       - Stripe 2 between Sidebar and Main Stage / Settings Body (`.chat-column` or `.settings-body`).
       - Outer padding around `.workspace-shell` exposing wallpaper around all 4 edges.
    2. Bloated, oversized UI elements ("hiện quá là to"):
       - `.settings-nav-item` is 40px–48px tall with 12px pill border-radius and large margins.
       - `.channel-row` is 36px tall with 12px bulbous pill border-radius (`--radius-lg`).
       - Elements look artificially zoomed in and disconnected.

## 2. Root Cause Analysis
1. **Desktop Shell Gap & Panel Radius**:
   - In `app/assets/theme.css`: `--shell-gap: 0.75rem;` (12px) and `--panel-radius: 1.125rem;` (18px).
   - In `app/assets/main.css`:
     - `.workspace-shell { padding: var(--shell-gap); gap: var(--shell-gap); }`
     - `.settings-page { gap: var(--shell-gap); }`
     - `.chat-main-container { gap: var(--shell-gap); }`
     - `.main-stage > .main-stage { gap: var(--shell-gap); }`
   - When wallpaper or glass is active (`html[data-app-wp]` or `html[data-panel-glass="true"]`), the 12px gaps and 18px rounded corners expose the wallpaper behind panels as harsh vertical stripes.
2. **Oversized Item Rules**:
   - `.settings-nav-item` had competing declarations up to `min-height: 3rem;` and `border-radius: var(--radius-lg);`.
   - `.channel-row` had `border-radius: var(--radius-lg); min-height: 2.25rem;` making active/hover pills look like bulky lozenges.
   - Left rail items and header heights lacked compact 100% harmonization.

## 3. Goals & Acceptance Criteria
1. **100% Edge-to-Edge Seamless Layout**:
   - Set `--shell-gap: 0px;` and `--panel-radius: 0px;` so `.workspace-shell`, `.settings-page`, and `.chat-main-container` have zero gaps and zero outer padding.
   - Panels (`.app-rail`, `.nav-sidebar`, `.main-stage`, `.chat-column`, `.settings-nav`, `.settings-body`) touch flush (0px gap), completely eliminating orange vertical stripes.
2. **100% Standard Scale & Compact Row Proportions**:
   - Lock `--font-size-root: 15px;` for standard 100% typography.
   - Normalize `.settings-nav-item` to compact `height: 2.125rem; min-height: 2.125rem;` (34px), `padding: 0 10px; font-size: 0.875rem; border-radius: var(--radius-md);` (8px subtle rect).
   - Normalize `.channel-row` to compact `height: 2rem; min-height: 2rem;` (32px), `padding: 0 8px; font-size: 0.875rem; border-radius: var(--radius-sm);` (6px subtle rect).
   - Normalize `.category-row` to compact header height `28px`, subtle text transform, 8px padding.
   - Center `.panel-resizer` with `margin: 0 -4px; width: 8px; background: transparent;` so dragging works smoothly with zero visual gap.
3. **Synchronize Visual Harmony Across Desktop & Mobile**:
   - Zero divider lines, pure borderless surfaces.
   - Unified frosted glass palette on panels (`color-mix` + `backdrop-filter: blur(16px)`).
   - Pass 100% automated E2E tests for desktop zero-gap layout, compact scale, and prevent mobile regressions.
