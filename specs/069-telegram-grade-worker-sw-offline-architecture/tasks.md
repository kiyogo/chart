# Tasks: Spec 069 — Telegram Web K/A-Grade Worker, SW Streaming, Offline-First & Virtual Chat

## Phase 1: Backend Binary WebSocket (`CWBF1`) & Protected Media Guards
- [x] T001 Implement `?proto=bin` P-256 ECDH + HKDF-SHA256 + AES-256-GCM `CWBF1` binary protocol & unit tests in `web/backend/src/websocket/mod.rs`
- [x] T002 Implement `403 Forbidden` direct-navigation/cross-site guard, `POST /api/attachments/stream-token` HMAC-SHA256 tokens, & unit tests in `web/backend/src/attachments/mod.rs`
- [x] T003 Add `/stream/media/{*rest}` 403 fallback, `/sw.js` `Service-Worker-Allowed: /`, `media-src` CSP, and CORS headers in `web/backend/src/main.rs`

## Phase 2: Dedicated Web Worker & Service Worker Media Proxy
- [x] T004 Create `app/web/network_worker.js` (ECDH binary WS client, AES-256-GCM frame decryption, ping/pong watchdog, reconnect)
- [x] T005 Create `app/web/network_bridge.js` (`window.__chatwNetBridge`) and wire `app/src/platform/web.rs::run_ws_loop`
- [x] T006 Create `app/web/sw.js` (4-layer `403 Forbidden` tab-bound guard + HTTP `206 Partial Content` Range proxy + O(1) `CWCH2` chunked AES-256-GCM streaming + LRU chunk cache)
- [x] T007 Create `app/web/media_stream_bridge.js` (`window.__chatwMediaStream`) and wire SW streaming + grant revocation in `app/src/components/rich.rs`

## Phase 3: Offline-First IndexedDB Cache, Persistent Outbox & Optimistic UI
- [x] T008 Upgrade `app/web/storage_engine.js` (`DB_VERSION = 3`) with `chat_messages` & `chat_outbox` stores and helpers
- [x] T009 Add cross-platform storage wrappers in `app/src/storage/{mod.rs,browser.rs,native.rs}`
- [x] T010 Wire 0ms IndexedDB hydration, write-through, Optimistic UI pending state, and auto-flushing offline outbox in `app/src/pages/chat.rs`
- [x] T011 Render `ClockIcon` pending badge & `.pending` state + scroll-anchor preservation + 100k window unit test in `app/src/components/messages.rs`

## Phase 4: Progressive Blur-Up Media, 100k CSS Virtualization & Bundling
- [x] T012 Add 16×16 micro-blur `blurDataUrl` + `width`/`height` extraction in `app/web/media_optimizer.js` and `app/src/storage/media_opt.rs`
- [x] T013 Render instant blurred preview + `aspect-ratio` + 220ms cross-fade in `app/src/components/rich.rs` and `app/assets/main.css` (+ `content-visibility: auto` on `.message-row`)
- [x] T014 Update `app/web/index.html` and `app/build-web.sh` to copy & cache-bust all new scripts and `/sw.js`

## Phase 5: Build, Verification & Git Push
- [x] T015 Verify JS syntax (`node --check`), Rust tests (`cargo test -p chatw` & `cargo test -p chatw-app --lib`), rebuild backend & restart `:8085`, build release WASM (`./app/build-web.sh release`)
- [x] T016 Write & run Playwright E2E `app/e2e/spec069-telegram-grade-worker-sw-streaming-offline.mjs` + visually inspect screenshots via `read_file`
- [x] T017 Run existing E2E regression suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`)
- [x] T018 Update `tasks.md` checkboxes, commit, and `git push origin main`
