# Tasks 022: Storage, Media Optimization, Multi-Device Sync & Peer Recovery

## Phase A: Database Schema & Storage Core
- [x] **A1**: Create `web/backend/migrations/0026_storage_sync_recovery.sql` (devices, sessions.device_id, device_login_requests, device_sync_cursors, device_storage_policies, media_objects CAS, media_references, media_device_locations, transfer_sessions/chunks, recovery_requests, conversation_prefs/channels policy columns, user_key_backups, migration_jobs).
- [x] **A2**: Add `image` & `tokio-util` (if needed, already in Cargo.lock) to `web/backend/Cargo.toml` and verify `cargo check -p chatw`.
- [x] **A3**: Implement provider-independent Storage Abstraction Layer in `web/backend/src/storage/` (`mod.rs`, `capabilities`, `existing.rs`, `local_cas.rs`, `temp_relay.rs`, `encrypted_backup.rs`, `s3.rs` with full AWS SigV4 signer + presigned URLs + multipart/lifecycle + health check + `migration.rs`).
- [x] **A4**: Implement `web/backend/src/media_opt/mod.rs` (honest magic-byte sniffer, dimension reader, genuine `image`+`ravif` AVIF/WebP/PNG adaptive transcoding, `512x512` + `128x128` thumbnails) and fix `.avif` format dishonesty bug in `web/backend/src/users/mod.rs`.

## Phase B: Conversation Policy Enforcement, Chunked Uploads, Deduplication & Relay
- [x] **B1**: Upgrade `web/backend/src/attachments/mod.rs` & build `web/backend/src/storage/router.rs`:
  - Strict IDOR prevention on `GET /api/attachments/{id}` and `/api/storage/references/{id}` for DM, Group, and Discord-style Server Channels (`VIEW_CHANNELS` / membership check).
  - Scoped CAS deduplication + reference reuse on forward (`POST /api/storage/forward`).
  - Variant downloads (`?variant=original|optimized|thumb`) + Byte-Range streaming + honest headers (`X-Content-SHA256`, `Content-Disposition`).
  - Resumable chunked uploads (`/api/storage/uploads/*`), DM encrypted Temp Relay TTL + replication ack (`/api/storage/relay/{ref_id}/ack`) + safe expiry cleanup, and `GET /api/storage/metrics` + policy/migration APIs.
- [x] **B2**: Wire DM/Group/Channel message deletion (`message.deleted`, group/channel delete) to safe `media_references` refcount decrement + WS broadcast `media.delete_for_all`.

## Phase C: Multi-Device Trust, Approval, Delta Sync & Two-Way Peer Recovery
- [x] **C1**: Implement `web/backend/src/devices_sync/mod.rs`:
  - Device registration & persistent trust state, login approval challenges (`6-digit code` + ECDH wrapped sync key), approve/reject endpoints, device revocation with immediate WS disconnect/kick, lost-all-devices 2FA/backup recovery, and `user_key_backups`.
  - Incremental cursor delta sync endpoint (`GET /api/sync/delta?since_seq=...`) covering DM/Group/Channel messages, edits/deletes, reactions, and media availability states.
  - Two-way DM Peer Recovery orchestrator (`/api/recovery/requests*`, `conversation_prefs.allow_peer_recovery`, device copy location verification & reporting).
- [x] **C2**: Extend `web/backend/src/websocket/mod.rs` & `web/backend/src/main.rs` (`WsEvent` variants: `DeviceApprovalRequest`, `DeviceApprovalResolved`, `DeviceRevoked`, `RecoveryRequestIncoming`, `RecoveryRequestUpdated`, `MediaDeleteForAll`, `RecoveryP2pSignal`), mount new routers, and spawn background expiry cleanup task cleanly.

## Phase D: Client Storage Engine, Adaptive Media Optimizer, WebRTC DataChannel & Cross-Platform Adapters
- [x] **D1**: Build `app/web/storage_engine.js` (IndexedDB `chatw_storage_v1` + OPFS + Cache API + `navigator.storage.persist()`/`estimate()` + `QuotaExceededError` safe eviction protecting `local_only`) and `app/web/media_optimizer.js` (adaptive sharpness-aware AVIF/WebP/PNG/JPEG compression + `512x512` avatar + poster frame + Opus voice helper + WebRTC `RTCDataChannel` chunked P2P transfer with backpressure & SHA-256).
- [x] **D2**: Update `app/web/index.html` & `app/build-web.sh` to bundle & cache-bust new JS engines seamlessly.
- [x] **D3**: Implement `app/src/storage/` (`mod.rs`, `browser.rs`, `native.rs`, `media_opt.rs`, `sync_manager.rs`, `recovery.rs`) and `app/src/services/storage_sync.rs` + extend `app/src/models.rs` and `app/src/services/ws.rs` for new `WsEvent`s & device registration/key backup helpers.

## Phase E: UI Integration & UX Verification
- [x] **E1**: Add Settings tabs `"Lưu trữ"` (quota, clear cache safety, real storage/dedup/bandwidth breakdown, S3/migration readiness status) & `"Đồng bộ"` (two-phase sync progress, delta sync trigger, encrypted DM backup opt-in, recovery queue/history, lost-all-device recovery wizard) + upgrade `DevicesDialog` (`Thiết bị`) with trust status, code approval/reject, revocation, and incoming `DeviceApprovalModal`!
- [x] **E2**: Upgrade `DmDetailsPanel` with per-DM `"Khôi phục file hai chiều"` toggle + missing media restore trigger; upgrade `Composer` with general File/Video/Audio/Voice recording + `"Bản gốc"` vs `"Nén tối ưu"` toggle; upgrade `RichContent` (`RichImage`, `RichFile`, `RichAudio`, `RichVideo`) with OPFS/IndexedDB local-first loading, original vs optimized downloads, copy-availability status badges, and honest Peer Recovery buttons!
- [x] **E3**: Style new components strictly within existing Flat Minimalist Pure CSS tokens in `app/assets/main.css` (zero broken spacing, dark & light theme parity, responsive mobile/desktop).

## Phase F: Comprehensive Testing, Live E2E & Auto-Push
- [x] **F1**: Add comprehensive backend unit & integration tests (`storage` unit tests for SigV4/CAS/relay/media_opt + `web/backend/tests/storage_sync_test.rs` covering all 25+ scenarios: DM/Group/Channel tiers, IDOR rejection, format honesty, dedup refcount, forward reuse, multipart resume, temp relay TTL & safe deletion guard, device trust/approval/rejection/revocation, lost-all-device fallback, cursor delta sync, 2-way recovery accept/deny/offline queue/no-copies, S3 presign/migration dry-run).
- [x] **F2**: Build debug/release server & WASM bundle (`./app/build-web.sh release`), restart live daemon, run existing E2E + new Playwright E2E verification (`app/e2e/storage-sync-check.mjs` checking browser OPFS/IndexedDB storage, adaptive avatar format, settings UI, two-user device approval & peer recovery / IDOR checks, zero console/page errors, visual screenshots).
- [x] **F3**: Run full `cargo test` workspace suite, verify cleanly, and commit + `git push origin main`.
