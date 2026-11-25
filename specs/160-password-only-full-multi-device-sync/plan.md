# Implementation Plan — Spec 160

## Reference Pattern (AGENTS.md Rule 3)
Follows the open-source Bitwarden / Matrix 4S zero-knowledge client-side vault architecture:
- **Independent Auth vs Encryption Domain**: Backend `users.password_hash` stores a salted Argon2id PHC strictly for session authentication. Client derives a separate key-encryption key (KEK) from the entered account password using an independent CSPRNG 16-byte salt + Argon2id (`19456 KiB`, `t=2`, `p=1`) and wraps `UserKeys` with AES-256-GCM (`a2v1:`) stored in `user_key_backups`. The server never observes plaintext private keys.
- **Ephemeral Session Handoff**: Account password is kept briefly in an in-memory `zeroize`-protected container between auth submission and `check_auth` / `bootstrap_device_and_sync_for_user`, then immediately zeroized once vault creation/unlock completes.

## Files to Create / Modify
1. **Migration**: `web/backend/migrations/0037_account_password_auth.sql` (`ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT`).
2. **Backend**:
   - `web/backend/src/backup/reauth.rs`: expose `pub(crate) fn hash_argon2_phc` and `pub(crate) fn verify_argon2_phc`.
   - `web/backend/src/auth/mod.rs`: add `/register`, `/login`, `/password` endpoints; extend `MeResponse` (`has_password`) & `ProvidersResponse` (`password`); reuse `users::validate_username`.
   - `web/backend/src/storage/router.rs`: emit `WsEvent::PrefsUpdated(json!({ "kind": "policy" }))` in `handle_update_policy`.
   - `web/backend/src/devices_sync/mod.rs`: ensure device registration/recovery transitions cleanly when client already holds verified `UserKeys` (`recover_lost` proof or direct registration).
3. **Frontend / Client Core**:
   - `app/src/crypto/e2ee.rs`: add zeroize-backed `set_ephemeral_account_password`, `get_ephemeral_account_password`, `clear_ephemeral_account_password`.
   - `app/src/storage/sync_manager.rs`: add `ensure_password_key_backup`, `try_auto_restore_with_ephemeral_password`, shared `apply_storage_policy_local`, ensure `recover_lost_devices_with_proof` promotes trusted state if needed, and hydrate policy on bootstrap.
   - `app/src/models.rs` & `app/src/services/api.rs`: update `MeResponse`, `ProvidersResponse`, and add `auth_register`, `auth_login`, `auth_set_password`.
   - `app/src/lib.rs`: integrate automatic vault unwrap / auto-backup in `check_auth` before setting `E2EE_RECOVERY_REQUIRED`; wipe ephemeral password on logout.
   - `app/src/prefs.rs` & `app/src/pages/chat.rs`: add `theme`, `accent`, `density`, `font_size` to `Prefs` and apply them via `platform::apply_appearance`; handle `WsEvent::PrefsUpdated` (`kind == "policy"`).
   - `app/src/pages/login.rs` & `app/assets/main.css`: add Sign in / Create account password form (preserving `.auth-dev-form` & Google OAuth).
   - `app/src/components/settings_extra.rs`, `app/src/components/settings_view.rs`, `app/src/components/storage_ui.rs`: wire `PasswordDialog` (re-wrapping E2EE backup on change), sync `update_appearance` through `PrefsCtx`, and clarify `E2eeUnlockRecoveryModal` fallback copy.
4. **Tests**:
   - Backend Rust test in `web/backend/tests/storage_sync_test.rs`.
   - Client unit tests in `app/src/prefs.rs`.
   - Multi-device Playwright E2E in `app/e2e/spec160-password-full-sync.mjs`.
