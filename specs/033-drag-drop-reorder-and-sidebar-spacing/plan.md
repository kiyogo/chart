# Technical Implementation Plan — Spec 033

## Architecture Overview

1. **Cross-Platform Drag-and-Drop Bootstrap (`app/assets/canvas_scrollbars.js`)**:
   - Add a lightweight delegated HTML5 drag-and-drop initializer alongside the existing canvas scrollbar script (automatically loaded in both Web and Desktop bundles via `index.html` and `app/src/main.rs`):
     - On `dragstart` (capture phase) for any `[data-drag-item]`: block accidental drags starting inside text inputs/selects/textareas/buttons/range sliders; otherwise set `e.dataTransfer.effectAllowed = 'move'` and `e.dataTransfer.setData('text/plain', item.dataset.dragKey || '1')` synchronously (critical for Firefox & WebKitGTK Desktop).
     - On `dragover` (capture phase) over any `[data-drag-item]` belonging to the active drag group: call `e.preventDefault()` and set `e.dataTransfer.dropEffect = 'move'`.
     - On `drop` / `dragend`: clear active drag group cleanly.
   - All reorder state (`drag_from`, `drag_over`) and reorder callbacks live cleanly in native Dioxus signals (`ondragstart`, `ondragover`, `ondrop`, `ondragend`).

2. **Plugin Data Model (`app/src/components/plugins_view.rs`)**:
   - Add `PluginMod::reorder_section(&mut self, from_key: &str, to_key: &str, defaults: &[&str])` using `remove` + `insert`.
   - Add unit tests verifying forward, backward, and same-key drag reorders.

3. **Modular, Non-Repetitive Plugin Settings (`app/src/components/plugin_settings.rs`)**:
   - Rewrite `SectionOrderControl` (used by `NotesPluginSettings`) to remove `.plugin-order-index` and `.plugin-order-btns` completely and render clean draggable rows with visual drop indicators.
   - Redesign `SoundboardPluginSettings` right card into `.soundboard-cfg-grid` (interactive visual pad grid respecting `columns`, click to play synth sound, drag pad-to-pad to reorder, compact corner switch).
   - Redesign `VoxelPluginSettings` right card into a live interactive 16×12 mini-canvas preview synced with `VOXEL_KEY` + quick templates (`Địa hình mẫu`, `Lâu đài`, `Xóa trắng`), removing `SectionOrderControl`.
   - Redesign `CyberPluginSettings` right card into Console toggles (`Hiện thẻ thông số kết nối`, `Âm thanh phản hồi Sci-Fi`), quick terminal action test (`Ping`, `Kiểm tra E2EE`), and `CSS bổ sung`, removing `SectionOrderControl`.
   - Redesign `ToolsPluginSettings`: replace `SectionOrderControl` with compact inline utility toggle chips (`Máy tính`, `Xúc xắc`, `Văn bản`, `Lệnh tắt`), and upgrade `/command` list rows to drag-and-drop (zero arrows).
   - Upgrade `CustomPluginSettings` custom block rows to drag-and-drop (zero arrows).

4. **App-Wide Drag-and-Drop Migration**:
   - `app/src/components/enterprise/settings.rs`: remove `ChevronRightIcon` / `ChevronDownIcon` move buttons from `Thanh bên`; wire mouse drag-and-drop with `save_patch` (`module_order`).
   - `app/src/components/profile_plus_ui.rs`: remove `↑` / `↓` buttons from `.section-list` and `↑` button from `.showcase-manage`; wire native Dioxus drag-and-drop with drop indicators on both.
   - `app/src/components/accounts.rs`: remove `↑` button from `.account-list`; wire native Dioxus drag-and-drop persisting `ORDER_KEY`.
   - `app/src/components/studio.rs` & `app/src/components/studio_engine.js`: add `reorderLayer(from, to)` in JS, remove `↑`/`↓` buttons in `studio.rs`, and wire drag-and-drop directly on `.studio-layer` rows.

5. **Sidebar Spacing Unification (`app/assets/main.css` & `app/src/components/navigation/sidebar.rs`)**:
   - `.sidebar-scrollable`: change `gap: 0.45rem;` -> `gap: 0.3rem;` (`4.5px`, exact 2/3 of `6.75px`). Keep `padding: 0 0.45rem var(--space-2);`.
   - `.create-community-btn`: add `chat-item` semantics (`padding: 0.45rem; gap: 0.625rem; margin: 0; min-height: 0; border-radius: var(--radius-lg);`) so `.community-item` and `.create-community-btn` match `.chat-item` 100%.
   - `.sidebar-section-header`: tighten bottom padding (`padding: var(--space-2) 0.45rem 0.125rem;`) so Community header flows naturally into cards.
   - Clean up unused `.plugin-order-index`, `.plugin-order-btns`, `.plugin-step-btn` CSS and add crisp drag-and-drop indicator styles (`.draggable-item`, `.is-dragging`, `.drag-over-before`, `.drag-over-after`, `.drag-over-tile`, `.soundboard-cfg-grid`, `.voxel-mini-canvas`).

## Verification Plan
- `cargo test -p chatw-app --lib`
- `./app/build-web.sh`
- Playwright E2E suites:
  - `app/e2e/plugin-system-check.mjs` (updated for zero numbers/arrows, real drag-and-drop reorder verification in Sổ tay + Soundboard pad grid + Enterprise `Thanh bên`, distinct Voxel/Cyber/Tools UIs)
  - `app/e2e/spec030-ui-check.mjs` (updated to assert `4.5px` = 2/3 chat gap AND 100% identical padding/inset/gap for Community cards vs Chat cards)
  - `app/e2e/borderless-shadowless-check.mjs`
  - `app/e2e/scrollbar-switch-check.mjs`
- Visual inspection of captured screenshots + rebuild/reinstall native Linux desktop app + auto-push to `origin/main`.
