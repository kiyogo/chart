# Technical Plan 022: Storage, Media Optimization, Multi-Device Sync & Peer Recovery

## 1. Module Architecture & Data Model

### 1.1. Database Migration (`web/backend/migrations/0026_storage_sync_recovery.sql`)
Additive, backward-compatible PostgreSQL migration:
1. **`devices` table**:
   - `id UUID PRIMARY KEY`, `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`
   - `device_fingerprint_client TEXT`, `device_name TEXT`, `device_label TEXT`, `platform TEXT NOT NULL DEFAULT 'web'` (`web|desktop|mobile`)
   - `ecdh_public BYTEA` (65B optional per-device P-256), `ecdsa_public BYTEA` (65B optional), `key_fingerprint BYTEA`
   - `trust_state TEXT NOT NULL CHECK (trust_state IN ('trusted','pending','revoked')) DEFAULT 'trusted'`
   - `approved_by_device_id UUID REFERENCES devices(id) ON DELETE SET NULL`, `approved_at TIMESTAMPTZ`, `revoked_at TIMESTAMPTZ`, `last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
2. **`sessions` alterations**:
   - `ADD COLUMN IF NOT EXISTS device_id UUID REFERENCES devices(id) ON DELETE SET NULL`
   - Backfill existing sessions cleanly if `device_id IS NULL` on migration or touch.
3. **`device_login_requests` table**:
   - `id UUID PRIMARY KEY`, `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`
   - `requesting_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE`
   - `verification_code TEXT NOT NULL`, `user_agent TEXT`, `ip_masked TEXT`, `status TEXT NOT NULL CHECK (status IN ('pending','approved','rejected','expired')) DEFAULT 'pending'`
   - `wrapped_sync_key TEXT` (optional ECDH-sealed E2EE backup blob passed by approver to requesting device), `approver_device_id UUID REFERENCES devices(id) ON DELETE SET NULL`, `expires_at TIMESTAMPTZ NOT NULL`, `decided_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
4. **`device_sync_cursors` & `device_storage_policies` tables**:
   - Track per-device sync cursors (`dm_seq`, `group_seq`, `channel_seq`, `event_seq`, `last_sync_at`) and user storage/sync settings (`cache_limit_mb` default `1024`, `auto_download_mode` default `'wifi'`, `dm_relay_ttl_hours` default `168`, `dm_encrypted_backup_enabled` default `false`, `require_device_approval` default `true`, `prefer_original_upload` default `false`).
5. **`media_objects` (Scoped CAS & Variants)**:
   - `id UUID PRIMARY KEY`, `cas_scope TEXT NOT NULL` (`shared:<sha256>` for server/group/channel plaintext; `e2ee:<user_or_pair>:<cipher_sha256>` for E2EE blobs to prevent cross-account probing)
   - `sha256 BYTEA NOT NULL`, `plain_sha256 BYTEA`, `storage_backend TEXT NOT NULL` (`local_cas|legacy|temp_relay|s3_ready|encrypted_backup|peer_only`)
   - `storage_key TEXT NOT NULL`, `mime_type TEXT NOT NULL`, `file_ext TEXT NOT NULL`, `byte_size BIGINT NOT NULL`, `original_byte_size BIGINT NOT NULL`, `width INT`, `height INT`, `duration_ms INT`, `codec TEXT`, `is_animated BOOLEAN NOT NULL DEFAULT FALSE`, `ref_count INT NOT NULL DEFAULT 1`, `variant_of UUID REFERENCES media_objects(id) ON DELETE SET NULL`, `variant_kind TEXT NOT NULL DEFAULT 'original'` (`original|optimized|thumb|poster`), `expires_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
   - `UNIQUE (cas_scope, variant_kind)`
6. **`media_references` (Per-Conversation Authorized Pointer + Retention State)**:
   - `id UUID PRIMARY KEY` (mapped cleanly from `attachments.id` or newly created attachment reference!)
   - `object_id UUID REFERENCES media_objects(id) ON DELETE SET NULL`, `optimized_object_id UUID REFERENCES media_objects(id) ON DELETE SET NULL`, `thumb_object_id UUID REFERENCES media_objects(id) ON DELETE SET NULL`
   - `owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`
   - `scope_type TEXT NOT NULL CHECK (scope_type IN ('dm','group','channel','avatar','banner','library','sticker'))`
   - `scope_id UUID`, `message_id UUID`, `original_filename TEXT NOT NULL`, `mime_type TEXT NOT NULL`, `byte_size BIGINT NOT NULL`, `sha256_plain BYTEA`, `sha256_cipher BYTEA`, `forward_from_ref_id UUID REFERENCES media_references(id) ON DELETE SET NULL`
   - `storage_tier TEXT NOT NULL CHECK (storage_tier IN ('dm_client_preferred','server_durable','temp_relay','encrypted_backup'))`
   - `availability_state TEXT NOT NULL CHECK (availability_state IN ('local_only','pending_delivery','available_on_peer','replicated_multi_device','backed_up_encrypted','server_durable','restored_from_peer','recovery_pending','queued_offline_peer','denied_by_peer','no_remaining_copies','deleted_for_everyone'))`
   - `min_safe_copies INT NOT NULL DEFAULT 2`, `deleted_for_all BOOLEAN NOT NULL DEFAULT FALSE`, `relay_expires_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`, `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()`
7. **`media_device_locations`**:
   - `(ref_id UUID, device_id UUID, user_id UUID, has_original BOOLEAN, has_optimized BOOLEAN, local_tier TEXT, verified_sha256 BYTEA, last_verified_at TIMESTAMPTZ)` — tracks exactly which devices hold a verified copy!
8. **`transfer_sessions` & `transfer_chunks`**:
   - Resumable multipart/chunked uploads & downloads (`session_id`, `user_id`, `device_id`, `scope_type`, `scope_id`, `total_size`, `chunk_size`, `total_chunks`, `expected_sha256`, `status`, `s3_upload_id`, `created_at`) + chunk completion records.
9. **`recovery_requests` & `conversation_prefs` extension & `channels` policy extension & `migration_jobs`**:
   - `recovery_requests` (`id`, `ref_id`, `requester_user_id`, `requester_device_id`, `target_user_id`, `target_device_id`, `transport_mode` [`webrtc_p2p|encrypted_relay|device_sync`], `status`, `relay_ref_id`, `reason`, `expires_at`, `created_at`, `updated_at`)
   - `conversation_prefs`: `ADD COLUMN IF NOT EXISTS allow_peer_recovery BOOLEAN NOT NULL DEFAULT TRUE`, `ADD COLUMN IF NOT EXISTS dm_encrypted_backup BOOLEAN NOT NULL DEFAULT FALSE`
   - `channels`: `ADD COLUMN IF NOT EXISTS media_retention_policy TEXT NOT NULL DEFAULT 'durable'`, `ADD COLUMN IF NOT EXISTS media_max_mb INT NOT NULL DEFAULT 100`
   - `migration_jobs`: Ready queue/audit log for CAS -> S3/MinIO migration (`status`, `scanned_objects`, `migrated_objects`, `verified_objects`, `failed_objects`, `dry_run`).
10. **`user_key_backups` table**:
   - Stores client-encrypted E2EE recovery bundle (`wrapped_keys_b64`, `kdf_salt_b64`, `kdf_params`, `key_fingerprint`) so users who lose ALL devices can restore E2EE identity with their passphrase/recovery key or trusted approver without exposing plaintext keys to server.

### 1.2. Backend Modules (`web/backend/src/`)
1. **`storage/mod.rs`**:
   - Core traits: `StorageAdapter` (async ops: `capabilities()`, `put_object()`, `get_object()`, `get_object_range()`, `delete_object()`, `object_exists()`, `get_metadata()`, `verify_integrity()`, `begin_multipart()`, `upload_chunk()`, `complete_multipart()`, `create_temporary_access()`, `cleanup_expired_objects()`).
   - `StorageCapabilities` boolean flags + max chunk sizes + backend id.
   - `storage/existing.rs` (`ExistingStorageAdapter`), `storage/local_cas.rs` (`LocalFileStorageAdapter`), `storage/temp_relay.rs` (`TemporaryRelayStorageAdapter`), `storage/encrypted_backup.rs` (`EncryptedBackupStorageAdapter`), `storage/s3.rs` (`S3CompatibleStorageAdapter` with full **AWS SigV4 signer**, **Presigned PUT/GET URL generator**, Multipart XML/ETag coordinator, Lifecycle policy builder, HTTP/MinIO client integration via `reqwest` & `hmac` + `sha2` + `hex`), `storage/migration.rs` (Dry-run & live migration job executor).
   - `storage/router.rs`: HTTP API router `/api/storage/*` & upgraded `/api/attachments/*`:
     - `POST /api/attachments` (backward-compatible! Accepts optional query/headers `scope_type`, `scope_id`, `message_id`, `file_name`, `mime_type`, `sha256_plain`, `sha256_cipher`, `variant_kind`, `forward_from_id`, `preserve_original`, `upload_mode` -> validates permission, deduplicates safely, stores CAS or Temp Relay based on scope tier!)
     - `GET /api/attachments/{id}` (queries legacy `attachments` OR `media_references` + enforces **strict DM participant / Group member / Server channel RBAC permission check**, supports `variant=original|optimized|thumb` & `Range` requests, honest `Content-Type` + `Content-Disposition`, `X-Content-SHA256`!)
     - `POST /api/storage/uploads/init`, `PUT /api/storage/uploads/{id}/chunks/{index}`, `POST /api/storage/uploads/{id}/complete`, `GET /api/storage/uploads/{id}` (resumable chunked transfers with integrity verification)
     - `POST /api/storage/forward` (creates reference-only forward without duplicating bytes on disk)
     - `GET /api/storage/metrics` (exact DB + filesystem measured stats + dedup/compression savings + relay/recovery counts)
     - `POST /api/storage/relay/{ref_id}/ack` (tracks delivered device copies & safely purges temp relay only when replication threshold met)
     - `DELETE /api/storage/references/{ref_id}` (Delete Local Cache / Delete For Everyone semantics + refcount GC)
     - `GET /api/storage/policy` & `PUT /api/storage/policy` (user storage quota/sync preferences + S3 readiness status + migration dry-run endpoint `POST /api/storage/migration/run`)
2. **`media_opt/mod.rs`** (Backend adaptive image verifier & optimizer using `image 0.25.10`):
   - Real magic byte sniffer (`avif`, `webp`, `png`, `jpeg`, `gif`, `mp4`, `webm`, `ogg`).
   - Avatar processor: center-crop or resize to `512x512` + `128x128` thumbnail, **actually transcodes to genuine AVIF when requested/compatible OR saves honest detected extension**, updating `users/mod.rs` to NEVER save mismatched `.avif` for non-AVIF data!
   - Server/Group media variant generator (original byte preservation + optional optimized WebP/AVIF + lightweight thumbnail <= 16KB for fast Phase A sync).
3. **`devices_sync/mod.rs`** (Multi-device trust, approval, recovery, & cursor delta sync `/api/devices/*`, `/api/sync/*`, `/api/recovery/*`):
   - `POST /api/devices/register` (registers current session's persistent device ID & public keys; evaluates trust policy -> `trusted` for first device, `pending` if user has existing trusted devices + approval required -> emits `WsEvent::DeviceApprovalRequest`)
   - `GET /api/devices`, `POST /api/devices/requests/{id}/approve`, `POST /api/devices/requests/{id}/reject`, `DELETE /api/devices/{id}` (revoke + immediate `WsEvent::DeviceRevoked` + session invalidation), `POST /api/devices/recover-challenge` (2FA / Backup code / key backup unwrap when all old devices lost)
   - `PUT/GET /api/keys/backup` (user encrypted recovery vault bundle)
   - `GET /api/sync/delta` (incremental sequence cursor across DMs, groups, channels, reactions, edits, deletes, locations, pending recovery)
   - `POST /api/recovery/requests`, `GET /api/recovery/requests`, `POST /api/recovery/requests/{id}/respond` (accept + offer WebRTC or Encrypted Relay upload, or deny), `POST /api/recovery/requests/{id}/complete`, `PUT /api/storage/locations/report` (device reports verified SHA-256 cached copies so `media_device_locations` and availability badges stay 100% truthful!)

### 1.3. Client Architecture (`app/src/` & `app/web/`)
1. **`app/web/storage_engine.js`** + **`app/web/media_optimizer.js`**:
   - **Browser Storage Engine (`window.__chatwStorage`)**:
     - Requests `navigator.storage.persist()` cleanly on boot/login; monitors `navigator.storage.estimate()`.
     - Manages IndexedDB (`chatw_storage_v1`: `objects`, `chunks`, `metadata`, `sync_queue`, `sync_state`, `locations`) + OPFS (`navigator.storage.getDirectory()` under `chatw_cas/`) with automatic graceful fallback to IndexedDB when OPFS unavailable, plus Cache API mirror for fast media URLs.
     - Handles `QuotaExceededError`: auto-evicts lowest-priority `Group`/`Channel`/multi-device-backed cached blobs first, NEVER evicts `local_only` single-copy DM media; surfaces explicit quota status.
     - Chunked read/write helpers with Web Crypto `crypto.subtle.digest('SHA-256', ...)` verification.
   - **Client Adaptive Image & Media Optimizer (`window.__chatwMediaOpt`)**:
     - Analyzes canvas luminance gradient & edge sharpness (text/screenshot classifier vs photography) to adaptively compress via AVIF -> WebP -> JPEG binary search while protecting readable text screenshots (`high sharpness` / `PNG` lossless when crisp UI screenshot) and alpha channels!
     - Generates `512x512` AVIF-first + honest MIME avatar AND thumbnail, returning `{ bytesB64, mime, ext, width, height, originalSize, optimizedSize }` so extension/MIME is NEVER lied about.
     - Video metadata & poster frame extractor + Opus voice recording helper (`MediaRecorder` with `audio/webm;codecs=opus` / `audio/ogg;codecs=opus`).
   - **WebRTC Peer File Transfer (`window.__chatwPeerTransfer`)**:
     - Extends/coexists with `webrtc.rs` cleanly using reliable ordered `RTCDataChannel` (`chatw-file`), 32 KB chunks, `bufferedAmountLowThreshold = 64 KB`, `HIGH_WATER_MARK = 256 KB`, SHA-256 chunk/file check, WebSocket signaling messages (`recovery.p2p.signal`), and progress callback into Rust/WASM!
2. **Client Rust Modules (`app/src/`)**:
   - `app/src/storage/` (`mod.rs`, `adapter.rs`, `browser.rs`, `native.rs`, `sync_manager.rs`, `recovery.rs`, `media_opt.rs`): Cross-platform `ClientStorageAdapter` with `BrowserStorageAdapter` (WASM) + `NativeDeviceStorageAdapter` (Desktop/Mobile filesystem CAS under `~/.local/share/chatw/cas` / mobile app data dir) + two-phase sync scheduler + resumable chunk uploader/downloader + WebRTC peer recovery + relay fallback!
   - `app/src/services/storage_sync.rs`: Clean async API wrappers for storage, devices, approvals, key backups, delta sync, location reporting, recovery requests, and metrics.
   - **UI Integration (Preserving existing Flat Minimalist Design)**:
     - `SettingsView` (`settings_view.rs`): Add concise nav items `"Lưu trữ"` (`SettingsTab::Storage`) and `"Đồng bộ"` (`SettingsTab::SyncRecovery`), plus upgrade `"Thiết bị"` / `DevicesDialog` (`settings_extra.rs`) with Trusted status badge, Pending New-Device Approval cards (code verification, Accept/Reject), Lost-Device Recovery / Encrypted Key Backup setup!
     - Incoming live `DeviceApprovalModal` when `WsEvent::DeviceApprovalRequest` arrives on an active trusted session!
     - `DmDetailsPanel` (`details_panel.rs`): Per-DM toggle `"Khôi phục file hai chiều"` + copy availability summary + missing file recovery action.
     - `Composer` & `ComposerTools` (`composer.rs`, `rich.rs`): Support Image/Video/File picker + Voice note recording + `"Bản gốc"` vs `"Tối ưu"` mode toggle.
     - `RichContent` (`rich.rs`): Upgrade `RichImage` & add `RichFile`, `RichAudio`, `RichVideo` reading local OPFS/IndexedDB cache first, showing truthful copy status pills, `"Tải bản gốc"` / `"Tải bản nén"`, `"Khôi phục từ thiết bị kia"`, and Clear Cache vs Delete for Everyone action safety!
     - `app/build-web.sh` & `app/web/index.html`: Bundle/load `storage_engine.js` & `media_optimizer.js` cleanly with cache-busting query versions and inspect CSP headers if needed (all scripts `'self'`!).

## 2. Backward Compatibility & Security Guarantees
- Existing routes (`/api/attachments`, `/api/attachments/{id}`, `/api/users/avatar`, `/api/media/upload`, `/api/messages`, `/api/groups`, `/api/channels`) stay 100% compatible with existing payloads while closing IDOR holes and recording CAS/reference metadata transparently.
- Private E2EE keys never leave client unencrypted (`user_key_backups` stores client-side AES-GCM PBKDF2/HKDF wrapped key blob; P2P approvals wrap via P-256 ECDH ephemeral key exchange).
- No mock stubs: `S3CompatibleStorageAdapter` lists exact supported capabilities (`presign: true`, `multipart: true`, `sigv4: true`, live network probe checks actual `S3_ENDPOINT` configuration rather than faking cloud storage when unconfigured).
