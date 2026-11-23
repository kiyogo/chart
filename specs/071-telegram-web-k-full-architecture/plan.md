# Implementation Plan: Spec 071 — Telegram Web K/A Full 12-Pillar Architecture

**Date**: 2026-11-23 | **Spec**: [spec.md](./spec.md)

## End-to-End Architecture

```text
[Tab 1 .. Tab N (Main Thread — 0 Fetch/XHR, 0 WebSocket, 0 Raw Media URLs)]
   ├─► MessagePort ──► SharedWorker (`app/web/core_worker.js`, fallback Dedicated Worker)
   │                      ├─ Single Binary WebSocket: `ws(s)://.../api/ws?proto=rpc-bin` (CWBF2)
   │                      │    • ECDH P-256 + HKDF-SHA256 -> non-extractable AES-256-GCM key
   │                      │    • Header AAD + Monotonic Seq + Zlib Deflate (>512B) + 16B Random Padding
   │                      ├─ Multiplexed Binary RPC (`OP_RPC_CALL` / `OP_RPC_RES`, 32-bit `req_id`)
   │                      ├─ Priority DownloadManager (`CRITICAL > HIGH > NORMAL > LOW`, 6 slots, Abort)
   │                      │    • `OP_MEDIA_CHUNK_REQ` / `OP_MEDIA_CHUNK_RES` -> E2EE Decrypt -> `blob:` URL
   │                      ├─ Resumable Chunked Upload (`OP_UPLOAD_PART` / `OP_UPLOAD_ACK`, 512 KiB)
   │                      ├─ PTS Delta Sync (`OP_DELTA_SYNC_REQ` / `OP_DELTA_SYNC_RES`)
   │                      └─ Multi-Tab Instant Sync (`tab_sync` across all connected MessagePorts)
   │
   └─► Direct MessagePort (`portA <-> portB`) ──► Service Worker (`app/web/sw.js`, scope `/`)
                          ├─ Intercepts `/stream/{grantId}` (and `/stream/media/{grantId}`)
                          ├─ 5-Layer `403 Forbidden` (no navigate/doc/iframe, media dest only,
                          │   128-bit TTL grant, strict `ownerClientId === event.clientId`, same-origin)
                          ├─ Pulls byte range directly from SharedWorker over `workerPort` (0 server fetch!)
                          ├─ Streams HTTP `206 Partial Content` `ReadableStream` + O(1) CWCH2 seek + prefetch
                          └─ App Shell & Static Offline Cache (`locales/*.json`, `catalog.json`, SVGs, WASM)

[Backend — Rust / Axum (`web/backend/`)]
   ├─ `web/backend/src/websocket/rpc_bin.rs`: CWBF2 framing, ECDH P-256, Zlib deflate, 16B padding,
   │    replay-checked seq, concurrent in-process `Router::oneshot` RPC dispatch, O(1) Range chunk reader,
   │    resumable upload part handler, and PTS delta sync handler.
   ├─ `web/backend/src/messages/mod.rs`: Idempotent `message_id` (`ON CONFLICT (id) DO NOTHING`) preventing duplicates.
   └─ `web/backend/src/main.rs`: Exposes `rpc_router` on `AppState`, enforces `/stream/{*rest}` -> `403 Forbidden`.
```

## Detailed File Manifest

### Backend (`web/backend/`)
1. `web/backend/Cargo.toml`: Add `flate2 = "1"` (already in `Cargo.lock`).
2. `web/backend/src/auth/mod.rs`: Add `pub rpc_router: Arc<std::sync::OnceLock<axum::Router>>` to `AppState`.
3. `web/backend/src/websocket/rpc_bin.rs` *(new)*: Full `CWBF2` binary protocol encoder/decoder (Zlib deflate >512B, 16-byte random padding, AAD, monotonic seq), ECDH handshake, multiplexed `OP_RPC_CALL`, O(1) `OP_MEDIA_CHUNK_REQ`, resumable `OP_UPLOAD_PART`, `OP_DELTA_SYNC_REQ`, and comprehensive unit tests.
4. `web/backend/src/websocket/mod.rs`: Export `pub mod rpc_bin;` and dispatch `?proto=rpc-bin` to `rpc_bin::handle_socket_rpc_bin` while keeping `?proto=bin` and JSON backward-compatible.
5. `web/backend/src/messages/mod.rs`: Make `handle_send_message` idempotent on `payload.message_id` (`random_id`) so network retries never create duplicate messages.
6. `web/backend/src/main.rs`: Mount internal `rpc_router` into `state.rpc_router`, add `/stream/{*rest}` `403 Forbidden` fallback route alongside `/stream/media/{*rest}`.

### Frontend Workers & JS Core (`app/web/`)
7. `app/web/core_worker.js` *(new)*: Unified `SharedWorker` (auto-detects `SharedWorkerGlobalScope` vs `DedicatedWorkerGlobalScope` so ONE script serves both!) implementing:
   - Single `CWBF2` encrypted Binary WebSocket (`/api/ws?proto=rpc-bin`) with ECDH P-256, Zlib deflate (`CompressionStream`/`DecompressionStream`), 16B random padding, monotonic seq, heartbeat, and reconnect.
   - Multiplexed `OP_RPC_CALL` queue with automatic wait-for-handshake & offline error reporting.
   - 4-lane Priority `DownloadManager` (`CRITICAL > HIGH > NORMAL > LOW`, concurrency 6, Abort) + E2EE decryption + smart 1–2 chunk prefetch.
   - Direct `MessagePort` endpoint for Service Worker `/stream/{grantId}` chunk requests (0 server HTTP fetch!).
   - Resumable chunked binary uploads (`OP_UPLOAD_PART`) with progress notifications.
   - `OP_DELTA_SYNC_REQ` PTS cursor sync.
   - Multi-tab instant `tab_sync` broadcast across all connected ports.
8. `app/web/core_bridge.js` *(new)*: Main-thread controller (`window.__chatwCore`) replacing direct `network_bridge.js`:
   - Connects to `SharedWorker('/core_worker.js')` (with automatic Dedicated `Worker('/core_worker.js')` fallback).
   - Bridges a direct Transferable `MessageChannel` between `core_worker.js` and `/sw.js` so SW pulls media chunks straight from `SharedWorker`!
   - Manages LRU ref-counted `blob:` URL registry (`downloadMediaBlobUrl`, `revokeBlobUrl`, `wipeAllOnLogout`).
   - Exposes `rpc(method, path, body, contentType, extraHeaders)`, `downloadBytes(url, opts)`, `uploadChunked(...)`, `deltaSync(cursors)`, `broadcastTabSync(...)`, `setSimulatedOffline(bool)`, `getStats()`.
9. `app/web/sw.js`: Upgrade to Spec 071:
   - Intercept BOTH `/stream/{grantId}` and `/stream/media/{grantId}`.
   - Pull media byte ranges directly through `workerPort` (`SharedWorker` → `OP_MEDIA_CHUNK_REQ`) with zero server `fetch()`!
   - Stream `ReadableStream` HTTP `206 Partial Content` with O(1) `CWCH2` decryption + 1–2 chunk background prefetch + 5-layer `403 Forbidden`.
   - Offline App Shell & Static Asset CacheStorage (`Cache-First` + `Stale-While-Revalidate`) for `/locales/*`, `/stickers/*`, fonts, CSS, JS, WASM.
10. `app/web/media_stream_bridge.js`: Upgrade to register `/stream/{grantId}` grants (no `/api/attachments/stream-token` HTTP fetch needed when `workerPort` is attached!), and wire `ATTACH_WORKER_PORT` automatically whenever SW controller activates/changes.
11. `app/web/media_worker.js` *(new)* + `app/web/media_optimizer.js`: Offload image resizing/AVIF/WebP compression/EXIF stripping/micro-blur generation to `OffscreenCanvas` Dedicated Worker (`media_worker.js`) + extract 48-bar 5-bit voice waveform amplitudes!
12. `app/web/storage_engine.js`: Bump `DB_VERSION` 3 → 4; add `dialogs`, `users_peers`, `sync_state`, `drafts`, `static_cache`, and `crypto_keys` (non-extractable `CryptoKey` store!).
13. `app/web/i18n_runtime.js`: Load `/locales/_keys.json` & `/locales/{code}.json` from IndexedDB `static_cache` first, then `window.__chatwCore.rpc` (0 Fetch/XHR!).
14. `app/web/index.html` & `app/build-web.sh`: Bundle & load `core_bridge.js`, `core_worker.js`, `media_worker.js`, `sw.js`, `media_stream_bridge.js`.

### Frontend Rust / Dioxus (`app/src/` & `app/assets/`)
15. `app/src/platform/web.rs` & `app/src/platform/mod.rs`:
    - Route `http_request` and `fetch_bytes` through `window.__chatwCore.rpc` / `downloadBytes` once authenticated (keeping pre-auth/cookie-mutating auth routes on HTTP `fetch()`).
    - Route `run_ws_loop` through `window.__chatwCore.subscribe`.
    - Add `resolve_blob_media_url(url, priority)` & `revoke_blob_media_url(url)` + `wipe_client_session_on_logout()` + `save_draft` / `load_draft` + `save_dialogs_cache` / `load_dialogs_cache` + `delta_sync_pts`!
16. `app/src/components/avatar.rs`, `app/src/components/rich.rs`, `app/src/components/messages.rs`:
    - 100% `blob:` URLs for avatars, chat images, stickers, GIFs, inline emojis (ZERO `/uploads/...` or `/api/attachments/...` in DOM `<img>`!).
    - Voice Note Waveform interactive seek bar (`waveform: Vec<u8>` on `MediaFileData`) + `/stream/{grantId}` playback.
    - True bi-directional 100,000+ message virtual scroller in `MessageList` (sliding window `(win_start, win_end)` capped at ≤ 80–120 mounted `.message-row` nodes with top/bottom spacer heights, `ResizeObserver` height tracking, floating Scroll-to-Bottom button with unread counter, and `window.__chatwVirtualBench` verification hook!).
17. `app/src/pages/chat.rs`:
    - 0ms IndexedDB `dialogs` hydration on boot (friends/groups/servers render immediately even offline!), per-conversation IndexedDB `drafts` persistence, `PTS` delta sync on reconnect, multi-tab `tab_sync` listener, `message_id = client_id` idempotent DM send, and full `wipe_client_session_on_logout()` on logout/lock.
18. `app/assets/main.css`: Styling for floating Scroll-to-Bottom button + unread badge, Voice Waveform seek bars, upload progress ring, and virtual spacers.

## Verification Strategy
1. `node --check` on every JS/Worker/SW file.
2. `cargo test -p chatw --lib` + `cargo test -p chatw-app --lib`.
3. Rebuild backend (`cargo build -p chatw`), restart `:8085` cleanly, rebuild release WASM (`./app/build-web.sh release`).
4. Write & run `app/e2e/spec071-telegram-web-k-12-pillars.mjs` verifying ALL 5 audit suites from Pillar 12 + visual screenshot inspection via `read_file`.
5. Run ALL existing E2E suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`, `spec069`) confirming 100% pass with zero regressions!
