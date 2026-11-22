# Tasks 067 — Simplified Settings, About & Developer Mode

- [x] T001 Add device-only `developer_mode: bool` to `app/src/prefs.rs` + unit test coverage.
- [x] T002 Create `app/src/components/settings_about_dev.rs` (iOS/Android-style `AboutOverviewPanel` with 7-tap build unlock, explicit `Developer mode` toggle, copy diagnostics & `DeveloperDiagnosticsPanel`) and export in `app/src/components/mod.rs`.
- [x] T003 Redesign `app/src/components/settings_view.rs` into 8 compact everyday sidebar tabs + conditional 9th `Developer` tab with sleek flat underline sub-tabs, smart deep-linking search, and 100% zero feature loss (moving `AdvancedSecurityExportSection`, raw E2EE keys, `SyncRecoverySettingsPanel`, `BotsSettings`, and `Diagnostics` into `Developer`).
- [x] T004 Add refined Spec 067 CSS in `app/assets/main.css` for `.settings-subtabs`, `.about-hero`, `.about-card`, `.about-row`, `.about-dev-card`, `.dev-header-strip` (light/dark/translucent wallpaper & RTL parity).
- [x] T005 Add all new Spec 067 canonical English UI strings to `app/web/locales/_keys.json` and translate across all 197 world language packs (with authentic native Vietnamese `vi.json`), verifying 100% parity.
- [x] T006 Update E2E verification suites (`spec052`, `spec061`, `spec062`) to test the 8-tab compact sidebar, sub-tabs, `About` screen, `Developer mode` ON/OFF lifecycle, `Advanced export` inside `Developer`, Vietnamese zero-truncation & Arabic RTL parity, and capture visual screenshots.
- [x] T007 Run `cargo test -p chatw-app --lib`, `./app/build-web.sh release`, and all active Playwright E2E suites; visually inspect all screenshots via `read_file`.
- [x] T008 Commit and push to `origin main` per `AGENTS.md` Rule #1.
