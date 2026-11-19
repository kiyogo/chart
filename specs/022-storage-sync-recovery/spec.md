# Spec 022: Storage Abstraction, Adaptive Media Optimization, Multi-Device Sync & Two-Way Peer Recovery

## 1. Context & Architectural Reality (Audited Baseline)
Audit of `/run/media/ngson/DATA1/DATA/source/chatw`:
- **Backend**: Rust (edition 2024), Axum 0.8.9, SQLx 0.9.0, PostgreSQL (`messages`, `group_messages`, `channel_messages`, `attachments`, `device_keys`, `sessions`, `conversation_prefs`, `custom_emojis`, `saved_gifs`, `sticker_packs`).
- **Frontend**: Rust + Dioxus 0.7.10 (`app/`) compiling to Web (`wasm32-unknown-unknown` via `app/build-web.sh`), Desktop (`dioxus/desktop`), and Mobile (`dioxus/mobile`), with JS bridge helpers in `app/web/*.js` and E2EE (`ECDH P-256`, `ECDSA P-256`, `HKDF-SHA-256`, `AES-256-GCM`) exclusively on client.
- **Current Gaps**:
  1. `GET /api/attachments/{id}` authenticates session only without verifying conversation/channel membership (IDOR).
  2. Attachments are stored un-deduplicated as flat `.bin` files in `{uploads_dir}/attachments/{id}.bin`; no CAS, no refcounting, no temporary relay TTL, no S3/MinIO readiness, no chunked/resumable transfer.
  3. `POST /api/users/avatar` unconditionally writes `.avif` extension even when client fallback produced WebP/JPEG/PNG.
  4. `device_keys` is keyed `ON CONFLICT (user_id)` (single key per account), lacking multi-device identity, trusted device challenge/approval, key handoff, revocation isolation, or cursor-based delta sync.
  5. Web client relies on `localStorage` only; no OPFS + IndexedDB + Cache API chunked cache, no persistent storage/quota management, no cache-vs-source deletion guard, no two-way WebRTC DataChannel DM peer recovery, and no general file/voice/video attachment messaging.

## 2. Core Objectives & Scope Classification
### 2.1. Three-Tier Conversation Storage Policy
| Chat Type | Text & Metadata | Media, Voice, Files, GIFs, Stickers | Access Control & Retention |
|---|---|---|---|
| **1-on-1 DM (`dm`)** | Durable PostgreSQL (`messages`, edits, deletes, pins, replies, reactions, sync seq). Never purged for inactivity. | Client-first storage (`OPFS`/`IndexedDB` or Native CAS), synced across account's own trusted devices + held by both DM peers. Server uses **Encrypted Temp Relay** (`TemporaryRelayStorageAdapter`, default TTL 7d) only until delivery policy satisfied, OR opt-in **Encrypted Server Backup** (`EncryptedBackupStorageAdapter`). Forwarded media reuses `media_references` (zero physical duplication). | Only the two DM participants (or single owner in `"Chính mình"`). Safe relay deletion requires explicit ack/copy count (`min_safe_copies`) or surfaces expiration risk. |
| **Group Chat (`group`)** | Durable PostgreSQL (`group_messages` + metadata + roles/permissions). | Durable Server Storage (`LocalFileStorageAdapter` / S3-ready CAS) for images, video, GIF, custom stickers, voice notes, and files. Client caches locally within quota. | Active group members only (`group_members` check on upload, download, and forward). |
| **Discord-style Server (`server_channel`)** | Durable PostgreSQL (`channel_messages` + RBAC policies). | Durable Server Storage governed by channel retention policy (`durable`, `ephemeral`, `max_age_days`, `allow_media`) and RBAC (`VIEW_CHANNEL`, `READ_HISTORY`, `SEND_MESSAGES`, `ATTACH_FILES`). | Strict RBAC check on every download/metadata/recovery request. Zero cross-conversation reference leakage. |

### 2.2. Provider-Independent Storage Abstraction Layer
Unified `StorageAdapter` trait + explicit `StorageCapabilities` (`put_object`, `get_object`, `get_object_stream`, `delete_object`, `object_exists`, `get_metadata`, `get_content_hash`, `list_references`, `create_temporary_access`, `revoke_temporary_access`, `begin_multipart_upload`, `upload_chunk`, `complete_multipart_upload`, `download_chunk`, `resume_transfer`, `verify_integrity`, `cleanup_expired_objects`):
1. `ExistingStorageAdapter`: Backward-compatible adapter resolving existing `/uploads/...` and legacy `attachments` paths safely without breaking historical messages.
2. `LocalFileStorageAdapter`: Content-addressed CAS (`uploads/cas/aa/bb/{sha256}.{ext}`) with atomic temp-write + rename, SHA-256 verification, byte-range reads, and refcount-safe delete.
3. `BrowserStorageAdapter`: IndexedDB + OPFS (with safe fallback) + Cache API + `navigator.storage.persist()` + `estimate()` + `QuotaExceededError` defense + safe cache eviction (protects `local_only` / single-copy unbacked files).
4. `NativeDeviceStorageAdapter`: Desktop/Mobile filesystem CAS & chunk cache under app data directory with `0600` permissions.
5. `S3CompatibleStorageAdapter`: Production-structured S3/MinIO adapter supporting configuration (`S3_ENDPOINT`, `S3_REGION`, `S3_BUCKET`, `S3_PREFIX`, `S3_FORCE_PATH_STYLE`), AWS **SigV4** canonical request signer & presigned PUT/GET URL generator, multipart coordination, lifecycle rules, health probe, and dry-run/resumable **Migration Worker** (`migration_jobs`) — without requiring a live MinIO instance today.
6. `TemporaryRelayStorageAdapter`: Encrypted DM ephemeral relay (`uploads/relay/{id}.enc`) with TTL, minimum replicated copy verification (`delivered_copies`), expiry warning states, and background/on-demand cleanup.
7. `DevicePeerStorageAdapter`: Chunked E2EE/authenticated P2P transfer over WebRTC `RTCDataChannel` (`ordered: true`, backpressure via `bufferedAmountLowThreshold`, per-chunk + full-file SHA-256 verification, WebSocket signaling relay, automatic fallback to Temp Relay).
8. `EncryptedBackupStorageAdapter`: User opt-in E2EE cloud/server archive for DM media & encrypted key vault backup, verifiable via ciphertext SHA-256 without server plaintext visibility.

### 2.3. Client Cache, Quotas & Deletion Safety
- Configurable client media cache quota (default `1 GB`, presets `500 MB`, `1 GB`, `2 GB`, `5 GB`, `10 GB`, `Unlimited`), auto-download rules (`always`, `wifi_only`, `manual`), and clear separation of **Clear Local Cache** vs **Delete for Everyone**.
- Automatic eviction prioritizes already-backed/server-durable `Group`/`Server` caches and multi-device replicated copies (LRU), **never silently purging `local_only` unique files**.
- `QuotaExceededError` triggers graceful non-destructive cleanup of safe disposable caches first, then explicit user warning if still full.

### 2.4. Multi-Device Identity, Approval, Sync & Revocation
- **Per-device identity**: Every device creates & stores persistent `device_id`, OS/client metadata, and P-256 keys (`devices` table linked to `sessions`). Existing single-key `device_keys` stays intact for E2EE DM compatibility.
- **New-device login approval**: When signing in on a new untrusted device while >=1 trusted device exists, server opens a `device_login_requests` challenge (6-digit code, OS/IP/time) and notifies online trusted devices via `WsEvent::DeviceApprovalRequest`. Trusted devices can **Approve** (optionally transferring ECDH-wrapped E2EE key material) or **Reject** (immediately revoking session).
- **Offline / lost-all-device fallback**: Fallback recovery via 2FA TOTP / Backup Code / Recovery Key Vault restore (`EncryptedBackup`), clearly communicating to user whether server messages/group media remain available and offering peer recovery for unbacked DM files.
- **Revocation**: Immediately revokes session + device trust + `WsEvent::DeviceRevoked`, blocking subsequent sync/relay/P2P authorization.
- **Two-phase incremental sync**:
  - `GET /api/sync/delta?since_seq=...` for DMs, groups, channels, reactions, edits/deletes, pins, and media manifests.
  - Phase A (immediate): conversations, text messages, avatars, stickers, metadata, tiny thumbnails.
  - Phase B (background queue): media/files/voice/video paused/resumed gracefully, persisted across reload.

### 2.5. Two-Way DM Peer Recovery Protocol
- Lookup priority for missing local DM media:
  `1. Local Device Cache -> 2. Same-Account Trusted Devices -> 3. Encrypted Backup / Active Temp Relay -> 4. DM Peer Device (if peer allows) -> 5. Honest Permanently Unrecoverable`.
- Explicit state machine persisted on backend & client:
  `local_only`, `pending_delivery`, `available_on_peer`, `replicated_multi_device`, `backed_up_encrypted`, `restored_from_peer`, `recovery_pending`, `queued_offline_peer`, `denied_by_peer`, `no_remaining_copies`.
- Per-DM toggle `"Cho phép khôi phục file hai chiều"` stored in `conversation_prefs` + synced.
- Transfers chunked (32 KB over WebRTC DataChannel OR chunked server relay), encrypted with original E2EE keys/peer transfer envelope, verified byte-for-byte with SHA-256 (`sha256_plain` + `sha256_cipher`).
- Never mark recovery successful if zero surviving copies exist or peer declines.

### 2.6. Adaptive Media Optimization & Honest Format Integrity
- Client + Backend adaptive image optimizer (`image` 0.25.10 + `ravif` + Canvas/OffscreenCanvas):
  - Detects crisp UI screenshots/text (high edge-contrast / flat histogram / alpha) vs photographic scenes: protects sharp text & transparency (higher quality or lossless WebP/PNG) while compressing natural photos adaptively via binary-search AVIF/WebP quality tuning (`<= 1920px` optimized rendition, keeping original SHA-256 intact when requested).
  - Avatars normalized cleanly to `512x512` + `128x128` thumbnail, **always encoding genuinely to AVIF (`image/avif`) or matching the real container extension to the actual sniffed magic bytes** (fixing the existing `.avif` mislabeling bug in `users/mod.rs`).
  - Both **Original** (`?variant=original`, exact SHA-256 preserved) and **Optimized** (`?variant=optimized` / thumbnail `?variant=thumb`) download modes supported cleanly with accurate `Content-Type` and filename extension.
  - Supports **Images** (AVIF/WebP/PNG/JPEG), **Animated GIF & Stickers** (alpha + animation preserved), **Voice Messages** (`audio/webm;codecs=opus` / `audio/ogg;codecs=opus` via `MediaRecorder`), **Video** (codec inspection + thumbnail/poster + chunked transfer + original/optimized handling), and **General Attachments**.

### 2.7. Privacy-Safe Deduplication & Observability UI
- Scoped CAS deduplication (`media_objects` keyed by SHA-256 + visibility/scope guard + `ref_count`) prevents unauthorized cross-user existence probing (E2EE DM files only share references via authorized forward chains or identical caller-scoped ciphertext).
- Real measured storage/bandwidth metrics API (`GET /api/storage/metrics`) + client `estimate()` displayed inside native Flat Minimalist Settings (`Lưu trữ & Bộ nhớ`, `Đồng bộ & Khôi phục`, upgraded `Thiết bị`, incoming approval modal, per-DM recovery toggle in `DmDetailsPanel`, and Composer attachment/voice/compression controls).

## 3. Acceptance Criteria
1. All legacy messages, avatars, stickers, banners, library items, and `/api/attachments/{id}` continue loading with zero regression.
2. Attempting to fetch another conversation/group/channel's attachment fails with `403 Forbidden` (IDOR closed).
3. Avatar upload stores honest magic-byte matching format (genuinely transcoded `.avif` + `128x128` thumb when AVIF encoder produces bytes, or true format extension otherwise).
4. Forwarded media and duplicate group/channel uploads reuse `media_objects` with transactional `ref_count`, reclaiming physical blob only when `ref_count == 0` and zero active relay/backup pins remain.
5. Untrusted new device receives pending trust challenge (`device_login_requests`); trusted device approval unlocks sync, rejection invalidates session; revoking device closes WS/sync access immediately; lost-all-device flow supports 2FA/backup code + recovery center.
6. Incremental cursor delta sync returns only newly mutated/created records after `since_seq` with zero duplicate message insertion.
7. Two-way DM peer recovery succeeds across two simulated clients (WebRTC DataChannel + server relay fallback), queues cleanly when peer is offline, respects per-DM recovery permission denial, verifies SHA-256, and reports `no_remaining_copies` honestly when zero copies remain.
8. Client OPFS + IndexedDB cache honors configurable limit, prevents deleting `local_only` unique items during Clear Cache, separates local removal from Delete for Everyone, and handles `QuotaExceededError`.
9. S3/MinIO adapter signs valid SigV4 headers & presigned URLs and executes dry-run/staged migration worker state transitions cleanly in unit/integration tests without requiring a running MinIO binary.
10. Automated unit, integration (all 25+ scenarios), release WASM bundle build, and live Chromium Playwright E2E checks execute cleanly with zero skipped/broken flows and are pushed to `origin main`.
