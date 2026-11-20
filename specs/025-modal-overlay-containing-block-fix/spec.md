# Feature Specification: Global Modal & Popover Containing-Block Fix (025)

## Problem Statement
`backdrop-filter: blur(...)` on `.composer-box`, `.composer-context`, and `.chat-column.wp-image .chat-header` (plus static low `z-index: 12` on `.composer` and sibling panels) creates CSS `position: fixed` containing blocks and stacking context traps. As a result, modals and dismiss backdrops rendered inside `.composer-box` (such as *Tạo bình chọn*, *Tạo ghi chú*, *Tạo nhắc hẹn*, `.menu-dismiss`, and mobile `.picker-popover`) are clipped inside the 52px `.composer-box` at the bottom of the screen instead of centering across the full viewport.

## Acceptance Criteria
1. `.composer-box`, `.composer-context`, and `.chat-column.wp-image .chat-header` apply translucent background and `backdrop-filter` on `::before` pseudo-elements so they never trap `position: fixed` descendants.
2. Parent layout regions (`.composer`, `.chat-header`, `.chat-column`, `.nav-sidebar`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.settings-nav`, `.settings-body`) dynamically elevate their `z-index` via `:has(...)` whenever they host a popover (`--z-popover`) or modal/lightbox (`--z-modal`).
3. Theme plugin `glass` rules in `plugins_view.rs` exclude panels currently hosting an open modal/lightbox so `position: fixed` overlays remain full-viewport.
4. Verified in headless Chromium on Desktop (`1440×900`) and Mobile (`390×844`) for *Tạo bình chọn*, *Tạo ghi chú*, *Tạo nhắc hẹn*, `.menu-dismiss`, and `.picker-popover`.
