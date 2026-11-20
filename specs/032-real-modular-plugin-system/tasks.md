# Tasks: Spec 032 — Real Modular Plugin System

- [x] T001: Extend `PluginMod` data model (`allow_custom_logo`, `default_logo_url`, `section_order`, `hidden_sections`, `config`, `custom_settings`) and clean built-in defaults (remove HUD & fake Bot mode, convert Quick Assistant to `Công cụ Lệnh nhanh` utility plugin).
- [x] T002: Remove `.plugin-floating-hud` from `PluginGlobalOverlay` and `app/assets/main.css`, and disable fake local bots in `enabled_bot_plugins_as_items()`.
- [x] T003: Upgrade `play_mod_sound_opts` (real volume & pitch), `NotesAndTodoPage` + `PomodoroTimer`, `SoundboardPage`, `VoxelSandboxPage`, `CyberDashboardPage`, add `QuickToolsPage`, and make all live plugin UIs dynamically follow `section_order`, `hidden_sections`, and `config`.
- [x] T004: Replace cloned 4-tab plugin editor and capability pills in `PluginsSettings` with distinct, plugin-specific customizable/reorderable settings panels (no description box, no Bot/Trang/Giao diện/HUD, optional compact logo override per `allow_custom_logo`).
- [x] T005: Add clean pure-CSS styles in `app/assets/main.css` for the new per-plugin settings panels and Quick Tools UI.
- [x] T006: Build Web (`./app/build-web.sh`) and Linux Desktop (`cargo build -p chatw-app --features desktop --release` + `./app/install-linux.sh`), run Playwright E2E (`plugin-system-check.mjs`, `spec030-ui-check.mjs`, `borderless-shadowless-check.mjs`, `scrollbar-switch-check.mjs`), visually inspect screenshots.
- [x] T007: Commit and push all changes to `origin main`.
