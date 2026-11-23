# Tasks: Spec 071 — Telegram Web K/A Full 12-Pillar Architecture

## Phase 1: Backend Binary RPC Multiplexer (`CWBF2`), O(1) Chunks, PTS Delta Sync & Idempotency
- [x] T001 Add `flate2 = "1"` to `web/backend/Cargo.toml` and `rpc_router: Arc<OnceLock<Router>>` to `AppState` in `web/backend/src/auth/mod.rs`
- [x] T002 Create `web/backend/src/websocket/rpc_bin.rs` implementing `CWBF2` (ECDH P-256 + HKDF-SHA256 + AES-256-GCM + AAD + monotonic seq + Zlib deflate >512B + 16B random padding), multiplexed `OP_RPC_CALL`, O(1) `OP_MEDIA_CHUNK_REQ`, `OP_UPLOAD_PART`, `OP_DELTA_SYNC_REQ`, and Rust unit tests
- [x] T003 Wire `?proto=rpc-bin` in `web/backend/src/websocket/mod.rs`, mount `rpc_router` + `/stream/{*rest}` 403 fallback in `web/backend/src/main.rs`, and add `message_id` idempotency in `web/backend/src/messages/mod.rs`

## Phase 2: SharedWorker Core, Service Worker Direct-Port Proxy, Media Worker & IndexedDB v4
- [x] T004 Create `app/web/core_worker.js` (`SharedWorker` + Dedicated fallback: single `CWBF2` WS, RPC multiplexer, 4-lane Priority `DownloadManager` w/ Abort & prefetch, SW `MessagePort` bridge, resumable `OP_UPLOAD_PART`, `PTS` delta sync, multi-tab `tab_sync`)
- [x] T005 Create `app/web/core_bridge.js` (`window.__chatwCore`: worker lifecycle, SW port bridging, LRU ref-counted `blob:` URL pool, logout wipe, offline toggle, `uploadChunked`, `deltaSync`)
- [x] T006 Upgrade `app/web/sw.js` & `app/web/media_stream_bridge.js` (`/stream/{grantId}` pulling chunks from `SharedWorker` via `MessagePort` with 0 server fetches, `ReadableStream` HTTP `206` + O(1) `CWCH2` seek + 5-layer `403 Forbidden`, App Shell & Static CacheStorage)
- [x] T007 Upgrade `app/web/storage_engine.js` (`DB_VERSION = 4`: `dialogs`, `users_peers`, `sync_state`, `drafts`, `static_cache`, non-extractable `crypto_keys`) and `app/web/i18n_runtime.js` (IndexedDB/RPC zero-fetch i18n)
- [x] T008 Create `app/web/media_worker.js` (`OffscreenCanvas` compression, EXIF strip, micro-blur) and upgrade `app/web/media_optimizer.js` (48-bar 5-bit voice waveform + Media Worker offload)

## Phase 3: Frontend Rust/Dioxus Integration (0 Fetch/XHR, 100% `blob:` DOM, Bi-Directional 100k Virtual Scroller, Waveform & Drafts)
- [x] T009 Wire `app/src/platform/web.rs` & `app/src/platform/mod.rs` (`http_request`, `fetch_bytes`, `run_ws_loop` through `window.__chatwCore` + `blob:` URL helpers, drafts, dialogs cache, PTS sync, logout wipe)
- [x] T010 Update `app/src/components/avatar.rs`, `app/src/components/rich.rs`, and `app/src/components/messages.rs` (100% `blob:` URLs for images/avatars/stickers/GIFs, interactive Voice Waveform seek bar, `/stream/{grantId}` streaming, bi-directional 100,000+ message Virtual Scroller capped at ≤ 120 DOM nodes, floating Scroll-to-Bottom button with unread counter)
- [x] T011 Update `app/src/pages/chat.rs` & `app/src/storage/` (0ms `dialogs` hydration, per-chat IndexedDB `drafts`, idempotent `message_id` outbox send, `PTS` delta sync, multi-tab `tab_sync`, full session wipe on logout/lock)
- [x] T012 Update `app/assets/main.css`, `app/web/index.html`, and `app/build-web.sh`

## Phase 4: Build, Full 12-Pillar Verification & Mandatory Git Auto-Push
- [x] T013 Syntax-check all JS/Worker/SW files, run `cargo test -p chatw --lib` & `cargo test -p chatw-app --lib`, rebuild backend + restart `:8085`, build release WASM (`./app/build-web.sh release`)
- [x] T014 Write & run `app/e2e/spec071-telegram-web-k-12-pillars.mjs` verifying ALL 5 Pillar-12 audits (21/21 PASS) + visually inspect screenshots via `read_file`
- [x] T015 Run ALL existing E2E regression suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`, `spec069`) — 100% PASS
- [x] T016 Mark all tasks `[x]`, commit, and `git push origin main`
