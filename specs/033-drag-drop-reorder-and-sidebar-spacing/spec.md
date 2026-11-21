# Feature Specification: Drag-and-Drop Reordering Across Web, Distinct Plugin UIs & Community/Chat Sidebar Spacing

**Feature Branch**: `033-drag-drop-reorder-and-sidebar-spacing`
**Created**: 2026-11-21
**Status**: In Progress

## Problem Statement

1. **Numbered & Arrow-Based Reordering Across the App**: Multiple reorderable lists across the web application still display numeric indices (`1`, `2`, `3`) and/or up/down arrow buttons (`↑`, `↓`, `>`, `v`) — including Plugin customizers, Enterprise Settings (`Thanh bên`), Profile Layout & Showcase editors, Account ordering, and Studio Layer controls. Users should reorder items directly by dragging with the mouse cursor everywhere, with zero index numbers and zero reorder arrows.
2. **Repetitive Plugin Configuration Cards**: Nearly every built-in plugin currently renders the same stacked `"Sắp xếp khu vực..."` list card, making distinct plugins (such as `Bảng vẽ Voxel`, `Soundboard 8-Bit`, `Cyber Neon`, and `Công cụ Lệnh nhanh`) feel cloned rather than purpose-built.
3. **Sidebar Card Spacing Mismatch & Tighter Chat Rhythm**:
   - Community list cards (`.community-item`, `.create-community-btn`) currently have inconsistent spacing (`margin-top: var(--space-2)`, oversized `--list-item-py` padding, and wider icon-to-text gap) compared to `.chat-item` cards in the Messages sidebar.
   - Chat list frames in `.sidebar-scrollable` currently sit `0.45rem` (`6.75px`) apart and should be brought closer together to exactly **2/3** of that vertical gap (`0.3rem` / `4.5px`), with Community frames matching Chat frames 100%.

## User Stories & Acceptance Criteria

### US1 — Universal Mouse Drag-and-Drop Reordering (Zero Numbers, Zero Arrows)
- **AC1.1**: Zero index-number badges (`.plugin-order-index`) and zero reorder arrow buttons (`↑`, `↓`, `>`/`v` move buttons) exist in Plugin settings, Enterprise Settings (`Thanh bên`), Profile Layout sections & Showcase, Accounts ordering, and Studio layer controls.
- **AC1.2**: Every reorderable list/grid supports smooth mouse drag-and-drop (`draggable="true"`, `cursor: grab` / `grabbing`, directional `.drag-over-before` / `.drag-over-after` or `.drag-over-tile` indicator, `.is-dragging` state) in both Web WASM (Chromium/Firefox) and Dioxus Desktop (WebKitGTK).
- **AC1.3**: Dropping an item commits the exact new order immediately (local storage or server PATCH) and preserves toggle switches, inputs, and buttons inside draggable rows without accidental drags.

### US2 — Purpose-Built, Non-Repetitive Plugin Settings UIs
- **AC2.1**: `Bảng vẽ Voxel` removes the cloned `"Sắp xếp khu vực bảng vẽ"` list and instead offers brush/grid toggles alongside a live interactive mini 16×12 Voxel canvas preview with instant templates (`Địa hình mẫu`, `Lâu đài`, `Xóa trắng`).
- **AC2.2**: `Soundboard 8-Bit` replaces its vertical list with an interactive visual Launchpad pad grid (matching the chosen column count) where users drag pads directly with the mouse to reorder, click to audition audio, and toggle visibility per pad.
- **AC2.3**: `Cyber Neon` removes the cloned `"Bố cục Trạm lệnh"` reorder list and focuses on Neon themes, glow colors, Matrix FX, console toggles, quick live terminal test actions, and custom CSS.
- **AC2.4**: `Công cụ Lệnh nhanh` replaces its cloned section-order list with compact clickable utility toggle chips, live `/command` runner, composer quick buttons, and mouse-draggable `/command` items.
- **AC2.5**: `Sổ tay & Việc làm` keeps 3 clean draggable workspace module cards (zero numbers, zero arrows) alongside layout & Pomodoro settings, and `CustomPluginSettings` provides draggable custom UI blocks.

### US3 — 100% Identical Community & Chat Sidebar Spacing + 2/3 Tighter Row Gap
- **AC3.1**: `.sidebar-scrollable` vertical row gap is tightened from `0.45rem` (`6.75px`) to exact 2/3: `0.3rem` (`4.5px`).
- **AC3.2**: `.community-item` and `.create-community-btn` share the exact same geometry as `.chat-item`: `padding: 0.45rem`, `gap: 0.625rem`, `margin: 0`, `border-radius: var(--radius-lg)`, identical left/right inset (`0.45rem`), and identical vertical gap (`0.3rem` / `4.5px`) between frames.
- **AC3.3**: The `3px` centered canvas scrollbar and Flat Minimalist borderless/shadowless visual rules remain 100% intact.

## Out of Scope
- Keyboard shortcut legends (`↑ ↓ di chuyển` inside Quick Switcher help hints) are documentation hints, not reorder controls, and remain untouched.
