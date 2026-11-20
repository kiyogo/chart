# Feature Specification: KIYO Chat Complete Storage, .kdata Encrypted Backup, Cloud Sync & Recovery (027)

## Context & Continuity
Continues Spec 022 (`specs/022-storage-sync-recovery/`) on the existing Axum 0.8 + PostgreSQL 16 + Dioxus 0.7 codebase without replacing existing storage, E2EE, messaging, groups, or server features.

## Core Requirements
1. **Official Encrypted `.kdata` Format (`KIYO_Backup_YYYY-MM-DD.kdata`)**:
   - Purpose-built binary container (`KIYODATA` magic, versioned header, Argon2id key derivation, wrapped random DEK, `XChaCha20-Poly1305` `BE32` streaming AEAD blocks with per-block AAD bound to `backup_id + version + section + block_index`, encrypted manifest + content index, running SHA-256 transcript + HMAC integrity trailer `KIYOEND1`).
   - Mandatory encryption at all times (zero unencrypted `.kdata` mode), streaming-capable for large 1GB-100GB datasets without whole-archive RAM buffering or plaintext disk staging.
   - Contains exportable account profile/prefs (never passwords/tokens/raw device private keys), accessible messages & reactions/replies/edit/delete state, accessible media & thumbnails, technical manifest & checksums.
2. **Universal Import & Selective Restore Engine**:
   - Validates magic bytes, header MAC, AEAD tags, ZIP-bomb ratios/size caps, and relative-path sanitization (`..` / absolute paths blocked).
   - Supports `.kdata`, valid encrypted AES-256 ZIP, and authorized unencrypted ZIP.
   - Supports full, selective (messages, images, video, audio, files, avatar, settings, single conversation), missing-only backfill, and new-device restore with deduplication via stable IDs/SHA-256 and strict RBAC/impersonation guards.
3. **Strong Backend Re-Authentication for Export**:
   - Normal login session alone cannot export backups. Requires `POST /api/backup/reauth` (Argon2id account/export password, TOTP 2FA, one-time backup code, or passkey/OAuth reauth) issuing a short-lived, single-scope, rate-limited `export_authorization` token with audit logging.
4. **Locked-by-Default Unencrypted ZIP Export**:
   - Hidden from normal Backup/Export views; locked under `Cài đặt -> Bảo mật -> Tùy chọn xuất dữ liệu nâng cao`.
   - Requires multi-step risk warning, strong backend reauth, exact data summary review, final single-task confirmation, and immediate token revocation after completion/failure/abort. Supports optional WinZip AES-256 ZIP with honest metadata exposure notice.
5. **Cloud Storage Providers, Auto-Backup & Incremental Chunking**:
   - `CloudStorageProvider` abstraction covering Google Drive (OAuth 2.0 PKCE + `drive.file`/`drive.appdata`), OneDrive (Graph API PKCE + upload session), Dropbox (App Folder PKCE + `upload_session_v2`), Nextcloud & WebDAV (real PROPFIND/MKCOL/PUT/GET with HTTPS/TLS & SSRF checks), and S3/R2/MinIO/NAS (encrypted credentials stored server-side).
   - Incremental chunked snapshots (`kchunk_v1`) uploading only delta chunks, verifying remote integrity before atomic commit, and preserving referenced chunks during GC.
   - Automatic backup scheduler policies (`daily`, `weekly`, `on_change`, `wifi_only`, `charging_only`, rate/size caps, pause/resume/cancel).
6. **Settings UI Integration & Media Integrity States**:
   - Integrates seamlessly into Flat Minimalist Dioxus settings (`Sao lưu & .kdata`, `Tài khoản Cloud`, `Lưu trữ`, `Đồng bộ`, `Thiết bị`, `Bảo mật nâng cao`) with real progress tracking, honest capability/quota reporting, and 10-state file availability tracking.
