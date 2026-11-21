# Implementation Tasks — Spec 044: Ultra-Intuitive Settings UI/UX Simplification

- [x] **T001: Update `settings_view.rs` Navigation & Security Tab**
  - Simplify `NAV` labels: "Khóa ứng dụng", "Dung lượng", "Tin nhắn", "Tự trả lời", "Lối tắt", "Hình dán", "Vẽ hình", "Trợ giúp".
  - Replace raw crypto algorithm strings in `SettingsTab::Security` with clean friendly text.
  - Remove all `...` and `(...)` in `settings_view.rs`.

- [x] **T002: Overhaul `settings_more.rs` Components**
  - Rewrite `LockSettings`: remove PBKDF2/salt technical jargon, use clean everyday words.
  - Rewrite `AutoReplySettings`: simplify explanations.
  - Rewrite `ChatPrefsSettings`: short punchy toggle labels and simple paste options.
  - Rewrite `NotificationSettings`: clear notification controls.
  - Rewrite `PrivacySettingsPanel`: simple contact rules.
  - Rewrite `AdvancedLookSettings`: short slider descriptions, remove `(0px đến 32px)`, clean reset button.
  - Rewrite `SyncMotionSettings`: short sync text.

- [x] **T003: Overhaul `settings_extra.rs` Descriptions**
  - In `CustomizationSettings`, simplify rail shortcuts text.
  - In `AccountSecurity`, remove PKCE/OAuth technical jargon, simplify device and login text.
  - Remove any `...` or `(...)` in `settings_extra.rs`.

- [x] **T004: Automated E2E Verification**
  - Create `app/e2e/spec044-settings-ux-simplification.mjs`.
  - Assert zero `...` and zero `(...)` across Settings tabs.
  - Assert absence of technical jargon words ("PBKDF2", "ECDH", "AES-256", "salt", "PKCE", "OAuth").
  - Verify tab navigation works smoothly.

- [x] **T005: Rebuild, Regression Verification & Auto-Push**
  - Build release web WASM bundle (`./app/build-web.sh release`).
  - Run regression suites (Spec 044, 043, 042, 041, 040).
  - Build Linux desktop release binary and package `.deb`.
  - Commit with Conventional Commits and push to `origin/main` per AGENTS.md Rule #1.
