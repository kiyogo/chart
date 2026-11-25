# Spec 160: Password-Only Zero-Friction Full Multi-Device Synchronization

## Problem Statement
Currently, ChatW only supports Google OAuth and development-mode passwordless login (`/api/auth/dev-login`). Users cannot sign up or log in with a username and account password, and signing in on a second device (especially when the first device is offline) requires either a manual 6-digit approval code from the old device or a separate E2EE recovery passphrase (`E2eeUnlockRecoveryModal`). Additionally, top-level appearance settings (`chatw_theme`, `chatw_accent`, `chatw_density`, `chatw_font_size`) and media/AI policies (`device_storage_policies`) are either local-only or only hydrated lazily when opening Settings -> Storage.

## Goal
Allow any user to sign in on **any device** using **only their account username and password** and automatically synchronize **all features** (E2EE identity & conversation keys, decrypted message history across DMs/Self-Chat/groups/servers, appearance & custom themes, notifications, chat behaviors, sticker/emoji favorites, media quality & AI enhancement policies, privacy & plugins) with **zero secondary PINs, zero manual 6-digit device approvals, and zero interrupting recovery modals**, while preserving strict zero-knowledge client-side E2EE.

## Acceptance Criteria
1. **Username & Account Password Auth**:
   - Database adds `password_hash TEXT` to `users` (`0037_account_password_auth.sql`).
   - Backend provides `POST /api/auth/register`, `POST /api/auth/login`, and `POST /api/auth/password` (Argon2id PHC verification/hashing reusing `backup::reauth`), plus `has_password` on `/api/auth/me` and `password: true` on `/api/auth/providers`.
   - Login UI exposes a clean Sign-in / Create-account password form (preserving `.auth-dev-form` when `DEV_LOGIN=true` and Google OAuth when configured).
   - Settings -> Account allows setting or changing the account password (without current password if none was previously set, e.g. for Google OAuth / dev accounts).
2. **Automatic Account-Password E2EE Vault Wrap & Unwrap**:
   - Password entered on login/register/password-change is held strictly in ephemeral RAM (`EPHEMERAL_ACCOUNT_PW`, zeroized on wipe/logout) and never persisted to disk/localStorage.
   - Device 1 automatically encrypts `UserKeys` (`a2v1:` Argon2id + AES-256-GCM with independent CSPRNG salt) using the account password and uploads ciphertext via `PUT /api/keys/backup`.
   - Device 2 logging in with username + password automatically fetches `GET /api/keys/backup`, decrypts `UserKeys` before leaving auth bootstrap, promotes Device 2 to `trust_state = "trusted"` (signing `device_recovery_proof` if needed when `!totp_enabled`), runs full delta decryption, and clears the ephemeral password — showing **zero** unlock/approval modal even when Device 1 is completely offline.
   - Changing the account password in Settings automatically re-wraps and uploads `user_key_backups` with the new password.
3. **Full Feature Sync Across Devices**:
   - `Prefs` (`app/src/prefs.rs`) includes `theme`, `accent`, `density`, and `font_size`, syncing via `/api/settings-sync` and applying via `platform::apply_appearance` on login and live `WsEvent::PrefsUpdated`.
   - `bootstrap_device_and_sync_for_user` immediately pulls and hydrates `device_storage_policies` (`chatw_cache_limit_mb`, `chatw_media_quality_mode`, `chatw_ai_enhance`) so AI enhancement and media preferences apply immediately on any new device, and `handle_update_policy` broadcasts `WsEvent::PrefsUpdated` (`kind: "policy"`).
4. **Verification**:
   - Backend integration test verifying `/api/auth/register`, `/api/auth/login`, `/api/auth/password`, and `/api/keys/backup` multi-device recovery with account password.
   - Real multi-context Chrome Playwright E2E test proving Device 1 registers, customizes theme/prefs/policy, sends E2EE DM -> Device 1 closes -> Device 2 signs in fresh with ONLY username + password -> zero modal, `trust_state === 'trusted'`, E2EE DM decrypted, and theme/prefs/policy synced.
