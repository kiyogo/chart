# Spec 019: Resizable UI Panels (Sidebar and Details Panel)

## 1. Context & User Problem
The user uploaded two screenshots highlighting:
1. The border between the left sidebar (`.nav-sidebar`) and the main chat container (`.main-stage` / `.chat-column`).
2. The border between the main chat container and the right details panel (`.details-panel` / `.server-members-panel`).

User Request:
> "tất cả mấy cái này có thể tùy chỉnh kích thước được hết nhé"
> (All of these must be resizable)

## 2. Acceptance Criteria
- [ ] Left sidebar (`.nav-sidebar`) can be resized horizontally by clicking and dragging the divider between it and the main chat container.
- [ ] Right panel (`.details-panel`, `.partner-info-sidebar`, `.server-members-panel`) can be resized horizontally by clicking and dragging the divider between it and the main chat container.
- [ ] Resizer handles show `col-resize` cursor on hover with visual feedback (accent line highlight on hover/drag).
- [ ] Smooth 60fps/120fps drag interaction with minimum and maximum width constraints:
  - Sidebar: min 180px, max 480px.
  - Details panel: min 200px, max 520px.
- [ ] Double-clicking the resizer handle resets the panel width to its comfortable default.
- [ ] User's custom panel widths are persisted in `localStorage` (`chatw_sidebar_width`, `chatw_details_width`) and restored immediately upon page reload without layout shift.
- [ ] Responsive safety: On small/mobile screens (< 768px), resizers are disabled or hidden so single-pane mobile layout is preserved.
- [ ] Passes all unit tests, compiles cleanly in release mode, and auto-pushes to GitHub `main`.
