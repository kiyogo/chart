# Technical Plan: Spec 032 — Real Modular Plugin System

## Data Model Extensions (`PluginMod` in `app/src/components/plugins_view.rs`)
Keep backwards compatibility with existing `localStorage` JSON via `#[serde(default)]`:
- `allow_custom_logo: bool` — controls whether the user can override the plugin's logo.
- `default_logo_url: String` — preserves the plugin's built-in logo for 1-click reset.
- `section_order: Vec<String>` — ordered list of module/item IDs for the plugin's live UI.
- `hidden_sections: Vec<String>` — module/item IDs hidden by the user.
- `config: serde_json::Map<String, serde_json::Value>` — typed per-plugin settings (e.g., `focus_minutes`, `break_minutes`, `sound_alert`, `layout_mode`, `volume`, `pitch`, `columns`, `show_grid`, `bevel_blocks`, `default_color`, `disabled_commands`).
- `custom_settings: Vec<PluginSettingDef>` — declarative settings schema (`key`, `label`, `kind`: `"toggle" | "slider" | "select" | "text" | "color"`, `options`, `min`, `max`, `default_value`) for custom/imported third-party plugins.

## Files Modified
1. **`app/src/components/plugins_view.rs`**:
   - Remove `.plugin-floating-hud` rendering from `PluginGlobalOverlay`.
   - Set `enabled_bot_plugins_as_items()` to return `Vec::new()` (no fake bot in DM sidebar).
   - Add `play_mod_sound_opts(kind, volume, pitch)` using real Web Audio gain & pitch scaling; wire `sync_plugins_to_dom` to also respect Soundboard volume/pitch/typing sound settings.
   - Update `NotesAndTodoPage`, `PomodoroTimer`, `SoundboardPage`, `VoxelSandboxPage`, `CyberDashboardPage`, add `QuickToolsPage` (for `tools`), and update `CustomBlocksView` so every live plugin reads `section_order`, `hidden_sections`, and `config` dynamically.
   - Completely replace `PluginsSettings` cards & editor:
     - Clean cards without `"🤖 Bot"`, `"📄 Trang"`, `"🎨 Giao diện"`, `"⏱ HUD"`, `"◧ Bên trái"`, or description paragraph bloat.
     - Per-plugin dedicated configuration panels (`render_notes_settings`, `render_soundboard_settings`, `render_voxel_settings`, `render_sakura_settings`, `render_cyber_settings`, `render_tools_settings`, `render_custom_plugin_settings`) with move-up/move-down (`↑` / `↓`) layout ordering, real live controls, and conditional compact logo customization only when `allow_custom_logo` is true.
     - Clean `Tạo mới` flow without description box or bot token boilerplate.
2. **`app/assets/main.css`**:
   - Remove `.plugin-floating-hud` styles completely.
   - Add clean Flat Minimalist CSS for `.plugin-cfg-grid`, `.plugin-cfg-card`, `.plugin-order-row`, `.plugin-order-btns`, `.plugin-slider-row`, `.plugin-swatch-strip`, `.plugin-logo-inline`, `.plugin-tools-grid`.
3. **`app/e2e/plugin-system-check.mjs`**:
   - Dedicated Playwright E2E test verifying:
     - Zero occurrences of `"Bot"`, `"📄 Trang"`, `"🎨 Giao diện"`, `"⏱ HUD"`, `"Chế độ Bot"`, `"Hiển thị HUD nổi"`, `.plugin-floating-hud`, or `"Mô tả"` in Plugin management & settings.
     - Distinct UI when configuring `Sổ tay & Việc làm` vs `Soundboard 8-Bit` vs `Bảng vẽ Voxel` vs `Giao diện Sakura` vs `Công cụ Lệnh nhanh`.
     - Reordering a section in `Sổ tay & Việc làm` persists and changes the live order in the opened `Sổ tay` workspace.
     - Reordering pads / changing columns in `Soundboard 8-Bit` updates the live `Soundboard` grid.
     - Enabling `Giao diện Sakura` applies live CSS variables cleanly without errors.
