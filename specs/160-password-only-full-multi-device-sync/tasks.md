# Tasks — Spec 160

- [x] T1. Create migration `0037_account_password_auth.sql` adding `password_hash TEXT` on `users`.
- [x] T2. Expose Argon2id PHC helpers in `web/backend/src/backup/reauth.rs` and implement `/api/auth/register`, `/api/auth/login`, `/api/auth/password`, plus `MeResponse.has_password` and `ProvidersResponse.password` in `web/backend/src/auth/mod.rs`.
- [x] T3. Emit `WsEvent::PrefsUpdated` (`kind: "policy"`) in `web/backend/src/storage/router.rs:handle_update_policy`.
- [x] T4. Add zeroize-protected ephemeral account password store in `app/src/crypto/e2ee.rs` and clear it on logout (`app/src/lib.rs`).
- [x] T5. Implement automatic password-wrapped key backup creation & zero-prompt backup restore + trusted-device promotion in `app/src/storage/sync_manager.rs` and `app/src/lib.rs` (`check_auth`).
- [x] T6. Add `theme`, `accent`, `density`, `font_size` to `Prefs` (`app/src/prefs.rs`), connect `settings_view.rs` & `chat.rs` & storage policy hydration so all settings, appearance, and AI/media policy sync across devices.
- [x] T7. Add username+password sign-in/registration UI in `app/src/pages/login.rs` (+ CSS in `app/assets/main.css`), password management modal in `app/src/components/settings_extra.rs` (auto re-wrapping key backup), and clear account-password prompt copy in `app/src/components/storage_ui.rs`.
- [x] T8. Verify backend tests, client unit tests, WASM + server builds, and multi-device Playwright Chrome E2E (`spec160-password-full-sync.mjs`), then commit & push to `origin/main`.
