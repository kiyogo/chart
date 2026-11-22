# Spec 059 — Unified Glass Stacking Contexts, Rail Tooltips/Popovers, Full-Viewport Dismiss Backdrops & Crisp Popover Surfaces

## Problem Statement
When **Glass Blur (`data-panel-glass="true"`)** and translucent panels (`data-panel-translucent="true"`) are enabled over a wallpaper:
1. `.app-rail` applies `backdrop-filter: blur(...)` directly on `<nav class="app-rail">` with `z-index: auto`. In CSS Filter Effects Level 2, `backdrop-filter` simultaneously creates:
   - a new **stacking context** at `z-index: 0` (first in DOM order inside `.workspace-shell`), causing every later glass sibling (`.settings-nav`, `.nav-sidebar`, `.panel-resizer`, `.chat-column`, etc.) to paint **ON TOP OF `.app-rail`** and its children; and
   - a **containing block for `position: fixed`**, shrinking `.app-rail .menu-dismiss` to the `51px`-wide rail rect instead of the full viewport.
2. Consequently:
   - Every rail tooltip (`[data-tip]:hover::after` for Messages, Friends, Communities, Meet, Feed, Vico AI, Search, Plugins, Notifications, Settings) is painted **behind and blurred by** `.settings-nav` / `.nav-sidebar`.
   - The Download hover QR card (`.account-menu.rail-menu.download-rail-menu`) is completely painted **behind and blurred by** `.settings-nav` / `.nav-sidebar`, and pointer events hit `.settings-nav` instead of the QR card buttons.
   - The Account switcher menu (`.account-menu.rail-menu`) is trapped behind `.settings-nav` / `.nav-sidebar`, its `.menu-dismiss` backdrop only spans `51px`, and clicks outside the rail cannot dismiss it.
3. `html[data-panel-glass="true"] .rail-item.active` applies a redundant nested `backdrop-filter: blur(...)` inside `.app-rail`, causing double-blur / nested stacking conflicts.
4. Sibling glass panels (`.nav-sidebar`, `.settings-nav`, `.settings-body`, `.chat-column`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.right-panel`, `.notifications-page`, `.download-page`, `.search-page`, `.feed-column`, `.meet-column`, `.vico-chat-column`, `.plugin-panel`) do not drop direct `backdrop-filter` or elevate `z-index` when hosting open dropdown/popover dismiss backdrops (`.menu-dismiss`, `.cw-dropdown-menu`, `.account-menu`, `.picker-popover`, `.header-search-backdrop`, `.header-search-popover`, `.tools-menu`), nor do all panels elevate to `--z-modal` when hosting modals.
5. Floating popovers (`.picker-popover`, `.profile-popover`, `.header-search-popover`, `.suggest-list`, `.paste-choice`) used `background: var(--bg-surface-hover)` (`rgba(..., 0.045)` — 95.5% transparent) instead of opaque `var(--bg-elevated)` with crisp contrast, and rail tooltips / menus could inherit busy-wallpaper `text-shadow` halo from `.app-rail`.

## Acceptance Criteria
1. **Zero Trapped / Blurred Rail Tooltips & Popovers**:
   - With `data-panel-glass="true"`, `data-panel-translucent="true"`, and any wallpaper active:
     - Hovering **Friends**, **Settings**, or any rail icon renders its `[data-tip]:hover::after` tooltip **ON TOP OF** `.settings-nav` and `.nav-sidebar` with crisp contrast and zero blur bleed-through.
     - Hovering **Download** renders `.download-rail-menu` **ON TOP OF** `.settings-nav` and `.nav-sidebar`; `document.elementFromPoint()` at the popover center and its `"Tải cho máy tính"` button hits `.download-rail-menu` (never `.settings-nav`), and clicking it navigates cleanly.
     - Clicking the bottom **Account avatar** renders `.account-menu.rail-menu` **ON TOP OF** `.settings-nav` and `.nav-sidebar` (`elementFromPoint` hits `.account-menu.rail-menu`), AND `.app-rail .menu-dismiss` covers the **full viewport (`1440×900`)** so clicking outside dismisses the menu cleanly.
2. **Zero Glass Flicker on `.app-rail` & Zero Nested `backdrop-filter`**:
   - `.app-rail` renders its frosted glass via `.app-rail::before` (`z-index: -1`), keeping frosted glass intact while hovering or opening menus without trapping `position: fixed`.
   - `html[data-panel-glass="true"] .rail-item.active` has `backdrop-filter: none`.
   - In non-glass translucent mode (`data-panel-translucent="true"`, `data-panel-glass="false"`), `.app-rail` background remains 100% synced with `.settings-nav` (`spec052` parity).
3. **Unified Workspace Z-Index Hierarchy & Full-Viewport Backdrops Everywhere**:
   - Base panels sit at `z-index: 1` (via `:where(...)` so responsive drawers at `--z-drawer: 60` win automatically), `.panel-resizer` at `15`, `.app-rail` at `30` at rest / `80` on `:hover` / `var(--z-popover)` (`100`) when hosting open rail menus, and any panel hosting an active popover/dropdown elevates to `var(--z-popover)` (`100`) with full-viewport `.menu-dismiss`.
   - Any modal (`.modal-overlay`, `.settings-modal-backdrop`, `.lightbox`) at `var(--z-modal)` (`1000`) always renders strictly above `.app-rail`, `.nav-sidebar`, `.settings-nav`, and all sibling panels.
4. **Crisp Floating Popover Surfaces & Zero Halo Bleed**:
   - `.picker-popover`, `.profile-popover`, `.header-search-popover`, `.suggest-list`, `.paste-choice` use opaque elevated surfaces (`var(--bg-elevated)` with light/dark adaptive rules) and subtle separation shadow.
   - Tooltips and floating popovers/menus never inherit `text-shadow` from `data-text-halo="true"`.
5. **Zero Regressions**:
   - All existing UI E2E suites (`spec040`, `spec052`, `spec053`, `modal-overlay-check`) and the new `spec059-glass-stacking-popovers.mjs` suite pass 100%, confirmed by visual screenshot inspection.
