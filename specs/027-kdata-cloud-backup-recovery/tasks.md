# Tasks (027)

- [x] T001 Audit existing storage, sync, auth, security, and UI code (`Spec 022` & repo).
- [x] T002 Create `crates/kiyo-kdata` workspace crate and verify compilation on native & `wasm32`.
- [x] T003 Create DB migration `0027_kdata_cloud_backup.sql` and wire into backend.
- [x] T004 Implement backend `backup/` module: strong reauth + rate limit + audit log, streaming `.kdata` export/inspect/restore, locked multi-step unencrypted ZIP & optional AES-256 ZIP.
- [x] T005 Implement backend `cloud/` module: `CloudStorageProvider` interface, Google Drive/OneDrive/Dropbox OAuth PKCE, Nextcloud/WebDAV (with TLS & SSRF guard), S3/R2/MinIO/NAS, and Incremental Chunked Snapshot + GC engine.
- [x] T006 Implement frontend services, local streaming integration, and Flat Minimalist Settings UI (`Sao lưu & .kdata`, `Tài khoản Cloud`, `Bảo mật nâng cao`, `Lưu trữ & Đồng bộ` enhancements).
- [x] T007 Write & run comprehensive unit, integration, security, and Chromium E2E tests; honestly verify and document configured vs admin-credential-pending cloud providers.
- [x] T008 Build Web release, commit, and push cleanly to `origin main`.
