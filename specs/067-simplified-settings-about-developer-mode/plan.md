# Plan 067 — Implementation Plan

## Files to Create / Modify
1. **`app/src/prefs.rs`**:
   - Add `pub developer_mode: bool` to `Prefs` (default `false`).
   - Add `"developer_mode"` to `DEVICE_ONLY` (keeps developer mode device-local, never synced across devices) and add unit test assertion verifying `developer_mode` is excluded from `sync_payload()`.
2. **`app/src/components/settings_about_dev.rs`** (new focused module — keeps `settings_view.rs` clean and modular!):
   - `AboutOverviewPanel`:
     - Clean iOS/Android-style hero identity card (ChatW `LogoIcon`, name, version pill, tagline).
     - Grouped key-value info table (`App name`, `Version`, `Build number` with 7-tap Android unlock, `Platform`, `Encryption`, `Languages`, `Privacy model`, `License`).
     - Action row (`Copy diagnostics`, `Replay tour`).
     - Dedicated **`Developer mode`** toggle card with crisp subtitle + live `"Open Developer"` button when enabled.
   - `DeveloperDiagnosticsPanel`:
     - Compact live diagnostics key-value card (Language & direction, Theme & accent, Wallpaper & transparency, Runtime platform) + quick actions (`Copy diagnostics`, `Reset appearance`, `Turn off developer mode`).
3. **`app/src/components/mod.rs`**:
   - Register `pub mod settings_about_dev;`.
4. **`app/src/components/settings_view.rs`**:
   - Refactor `SettingsTab` into **8 everyday tabs + 1 conditional `Developer` tab**:
     `Account`, `PrivacySecurity`, `ChatNotifications`, `Appearance`, `Language`, `DataBackup`, `Creative`, `About`, `Developer`.
   - Add clean sub-tab signals (`account_sub`, `privacy_sub`, `chat_sub`, `appearance_sub`, `data_sub`, `creative_sub`, `about_sub`, `dev_sub`) rendered via a reusable sleek flat underline tab bar (`SettingsSubTabs`).
   - Guard `Developer` tab visibility strictly behind `prefs_ctx.prefs.read().developer_mode`, and auto-switch `tab` back to `About` if `developer_mode` is turned off while viewing `Developer`.
   - Upgrade Settings Search (`nav_query`) with smart sub-tab deep-linking so searching `"cloud"`, `"backup"`, `"auto reply"`, `"app lock"`, `"draw"`, `"business"`, `"advanced export"`, `"đám mây"`, `"âm báo"`, etc. jumps straight to the right parent tab + sub-tab!
   - Keep `Account`, `Appearance`, and `Language` opening directly to `Profile`, `Theme & look`, and the 197-language grid respectively so common 1-click flows remain instant.
5. **`app/assets/main.css`**:
   - Add compact, modern styles for `.settings-subtabs`, `.settings-subtab` (flat underline indicator, zero pill clutter, RTL & dark/light/translucent wallpaper contrast), `.about-hero`, `.about-card`, `.about-row`, `.about-dev-card`, `.dev-header-strip`, and `.settings-nav-badge`.
6. **Global 197-Language Localization (`scripts/i18n_add_spec067_keys.py` -> `app/web/locales/_keys.json` + 197 `{code}.json`)**:
   - Append all newly introduced Spec 067 canonical English UI strings cleanly to `_keys.json` (sorted / deduplicated cleanly while preserving exact index alignment across all 197 packs!), apply authentic hand-curated Vietnamese (`vi.json`) translations + high-accuracy translations across all remaining 196 world language packs, and verify 100% parity (0 empty strings, 0 `...`).
7. **E2E Verification Suites**:
   - Update `app/e2e/spec052-visual-and-contrast-audit.mjs` (navigate to `Data & backup -> Cloud` for the 10 SVG cloud logos check) and `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs`:
     - Verify default Settings sidebar has **exactly 8** items (`Developer` hidden by default).
     - Verify `Privacy & security` (`Privacy`, `App lock`, `Security`), `Chat & notifications` (`Chat`, `Notifications`, `Auto reply`), `Appearance` (`Theme & look`, `Personalize`), `Data & backup` (`Storage`, `Backup`, `Cloud`), `Creative` (`Library`, `Draw`, `Inventory`, `Showcase`) all work seamlessly.
     - Verify `About` renders iOS/Android-style info rows + `Developer mode` switch; toggling ON reveals `Developer` as the 9th sidebar item with `Advanced export` (`Xuất dữ liệu nâng cao`), `Encryption keys`, `Sync & recovery`, `Extensions`, and `Diagnostics`; toggling OFF hides `Developer` cleanly!
     - Capture full-page visual verification screenshots in `verify_067/` (compact 8-item sidebar, `About` iOS/Android screen, `Developer mode` ON with 9th `Developer` tab & `Advanced export`, Vietnamese translated Settings, Arabic RTL Settings).
