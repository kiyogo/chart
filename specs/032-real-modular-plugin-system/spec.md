# Feature Specification: Real Modular Plugin Architecture & Per-Plugin Custom Settings (Spec 032)

**Status**: Approved  
**Date**: 2026-11-21

## Problem Statement
Previously, every plugin in ChatW displayed:
1. Generic capability pills (`🤖 Bot`, `◧ Bên trái`, `📄 Trang`, `🎨 Giao diện`, `⏱ HUD`) on cards and a floating HUD overlay (`.plugin-floating-hud`).
2. The exact same cloned 4-tab editor (`Cơ bản | Giao diện | Trang & HUD | Bot & Lệnh`) with a generic `"Mô tả"` box, giant icon picker dump, and 5 identical toggles (`Chế độ Bot trò chuyện`, `Ứng dụng ở cột bên trái`, `Thêm trang lên thanh điều hướng`, `Tùy chỉnh giao diện App`, `Hiển thị HUD nổi`) regardless of what the plugin actually does.

## Goals & Acceptance Criteria
1. **100% Removal of Bot / Trang / Giao diện / HUD Boilerplate**:
   - Zero occurrences of `"🤖 Bot"`, `"📄 Trang"`, `"🎨 Giao diện"`, `"⏱ HUD"`, `"◧ Bên trái"` capability pills.
   - Zero occurrences of `"Trang & HUD"`, `"Bot & Lệnh"`, `"Hiển thị HUD nổi"`, `"Chế độ Bot trò chuyện"`, or `.plugin-floating-hud` anywhere in the UI or DOM.
   - Zero fake local Bot entries injected into the Messages conversation list (`enabled_bot_plugins_as_items()` returns empty).
   - Zero forced `"Mô tả"` textarea on plugin customization pages or plugin cards.
2. **Plugin-Owned Logo Policy (`allow_custom_logo`)**:
   - Each plugin supplies its own signature logo by default.
   - Only when a plugin declares `allow_custom_logo: true` is a compact inline `"Đổi logo"` control offered (upload image/SVG or reset to default) inside its settings header — never a giant cloned preset picker dumped onto every plugin.
3. **Distinct, Real, Working Per-Plugin UI & Reorderable Layouts**:
   - Every plugin renders its OWN dedicated settings & layout customization panel (inspired by Obsidian `PluginSettingTab` / Vencord `definePluginSettings`):
     - **Sổ tay & Việc làm** (`notes`): Reorder/toggle workspace modules (`Pomodoro`, `Việc cần làm`, `Ghi chú nhanh`), layout mode (2-column / stacked), open position (sidebar / main), real Pomodoro focus/break minute presets & sound alert, live stats & quick maintenance actions. Live `NotesAndTodoPage` respects all settings immediately.
     - **Soundboard 8-Bit** (`soundboard`): Real Web Audio mixer (master volume slider, pitch shift slider, typing sound pack selector), grid columns (`2`/`3`/`4`), and reorderable/toggleable sound pads with instant test playback. Live `SoundboardPage` respects pad order, visibility, columns, volume, and pitch immediately.
     - **Bảng vẽ Voxel** (`voxel`): Default brush color palette, toggle grid lines, toggle 3D bevel, brush click audio, reorder/toggle workspace modules (`Bảng màu`, `Lưới vẽ`, `Thao tác`), and canvas reset/export. Logo owned by plugin (`allow_custom_logo: false`). Live `VoxelSandboxPage` applies all changes immediately.
     - **Giao diện Sakura** (`theme_sakura`): Curated Sakura palettes, live accent color picker, glass blur slider, corner radius, font family, petal animation toggle, navigation rail placement, and reset button.
     - **Cyber Neon** (`cyber`): Curated Neon presets, accent/bubble colors, neon/glass frame style, Matrix rain toggle, reorder/toggle console sections (`Thông số`, `Bảng lệnh`), command sound toggle, and optional custom CSS.
     - **Công cụ Lệnh nhanh** (`tools`): Interactive slash-command & quick-tool utility plugin (NOT a Bot) with reorderable/toggleable commands (`/calc`, `/roll`, `/flip`, `/time`, `/upper`), live test runner, custom slash snippet creator, and quick composer buttons, plus a real interactive `QuickToolsPage` workspace.
     - **Custom / Imported Plugins** (`custom`): Declarative `custom_settings` (toggle / slider / select / text / color) + reorderable interactive `custom_blocks` (`card`, `button`, `counter`, `checklist`, `input_action`, `timer`, `link`) + optional live CSS, with zero `"Mô tả"` or Bot/Trang/Giao diện/HUD boilerplate.
4. **Full Functional Verification**:
   - All plugin pages, sidebar panels, live Web Audio, live theme syncing, slash commands, custom JSON import/export, and reorderable sections work without errors in both Web and Linux Desktop builds.
