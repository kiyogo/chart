# Technical Plan (027)

## Architecture
1. **Shared Format Crate (`crates/kiyo-kdata`)**:
   - Dependencies: `argon2 = "0.5"`, `chacha20poly1305 = { version = "0.10", features = ["stream"] }`, `hmac = "0.12"`, `hkdf = "0.12"`, `sha2 = "0.10"`, `serde`, `serde_json`, `uuid`, `zeroize`, `zip = { version = "2", default-features = false, features = ["deflate", "aes-crypto"] }`, `flate2`.
   - Implements `.kdata` v1 streaming writer/reader (`KdataStreamWriter`, `KdataStreamReader`), AAD framing, Argon2id key wrapping, safe relative-path sanitization, ZIP bomb limits, and WinZip AES-256 / unencrypted ZIP helpers.
2. **Backend Migration (`web/backend/migrations/0027_kdata_cloud_backup.sql`)**:
   - Tables: `user_export_credentials`, `passkeys`, `passkey_challenges`, `export_authorizations`, `reauth_attempts`, `backup_jobs`, `cloud_connections`, `cloud_oauth_states`, `cloud_backup_policies`, `cloud_snapshots`, `cloud_chunks`, `cloud_snapshot_chunks`.
3. **Backend Modules (`web/backend/src/backup/` & `web/backend/src/cloud/`)**:
   - `backup/reauth.rs`: password/PIN setup & Argon2id verify, TOTP/backup-code/passkey challenge verify, rate limiting, short-lived `export_authorizations`, unencrypted ZIP multi-step gate, security audit logging.
   - `backup/engine.rs`: RBAC-filtered data gatherer (account, DM/group/channel messages, media refs, honesty flags for local-only/missing media), streaming `.kdata` exporter, AES-256 ZIP & locked unencrypted ZIP exporter, Universal Import & Selective Restore planner/executor with impersonation/dedup/tamper protection.
   - `cloud/mod.rs`: `CloudStorageProvider` trait + capability reporting.
   - `cloud/providers/{google_drive, onedrive, dropbox, webdav, s3_cloud}.rs`: real API adapters, OAuth2 PKCE + CSRF state, SSRF/TLS validation for WebDAV/Nextcloud/NAS, AES-256-GCM encrypted server-side credential storage.
   - `cloud/incremental.rs`: `kchunk_v1` per-user keyed chunk dedup, delta snapshot upload, remote `verifyBackup`, atomic commit, safe ref-counted GC.
   - `cloud/router.rs`: `/api/cloud/*` endpoints + auto-backup policy endpoints.
4. **Frontend (`app/`)**:
   - `app/src/services/backup_cloud.rs`: typed API client for `/api/backup/*` & `/api/cloud/*`.
   - `app/src/components/backup_cloud_ui.rs` + `app/src/components/settings_view.rs` + `app/src/components/storage_ui.rs` + `app/src/components/accounts.rs`: Flat Minimalist UI for `.kdata` export/import/restore with live progress, Cloud connections & incremental/auto backup, and Security Advanced Locked Unencrypted ZIP flow.
   - `app/web/storage_engine.js`: client streaming download/upload helpers + local media backfill during `.kdata` restore.
5. **Verification**:
   - Unit tests in `crates/kiyo-kdata`, `chatw`, `chatw-app`.
   - Integration tests `web/backend/tests/kdata_cloud_test.rs` covering `.kdata` tamper/truncation/block-swap/wrong-password, reauth enforcement, unencrypted ZIP lock/unlock/auto-revoke, ZIP bomb & path traversal rejection, duplicate-free re-import, RBAC/IDOR protection, and real local HTTP WebDAV/Nextcloud + S3 mock/local server roundtrip!
   - Playwright Chromium E2E test verifying Settings UI flows end-to-end against running server.
