# Tasks — Spec 014

- [x] T001 Add `--avatar-header` token in `app/assets/theme.css` across all densities (`default`, `compact`, `spacious`).
- [x] T002 Fix `input:focus` isolation & `.switch` toggle styling in `app/assets/main.css` (eliminate double/white inner ring).
- [x] T003 Synchronize `.chat-header` spacing & uniform header avatar sizing in `app/assets/main.css` and `app/src/components/plugins_view.rs`.
- [x] T004 Fix message bubble touching avatar in `app/assets/main.css` (`.message-row` gap, `.message-avatar-slot`) and `app/src/components/messages.rs`.
- [x] T005 Offset `.plugin-floating-hud` when `.details-panel` / `.partner-info-sidebar` is open in `app/assets/main.css`.
- [x] T006 Remove Plugin Studio header icon (`PluginIcon`) and description in `app/src/components/plugins_view.rs`.
- [x] T007 Upgrade `sync_plugins_to_dom` in `app/src/components/plugins_view.rs` with full dark/light semantic token derivation (`--bg-input`, `--bg-active`, `--bg-primary`, `--bg-warning`, `--border`, `--on-accent`, `color-scheme`, etc.) and clean `plugin_logo_data_url` circular fill.
- [x] T008 Implement Plugin Install (`Tải`) & Uninstall (`Gỡ`) lifecycle (`installed: bool`, store migration, filtered runtime hooks, and UI sections) in `app/src/components/plugins_view.rs` and `app/assets/main.css`.
- [x] T009 Verify `cargo check`, unit tests (`cargo test -p chatw-app --lib`), `bash app/build-web.sh`, live bundle on `127.0.0.1:8085`, mark tasks complete, and commit + push to `main`.
