# Implementation Plan: Spec 069 — Telegram Web K/A-Grade Worker, SW Streaming, Offline-First & Virtual Chat

**Branch**: `main` | **Date**: 2026-11-23 | **Spec**: [spec.md](./spec.md)

## Architecture & Data Flow

```text
[User presses Send]
   │
   ├─► 0ms Optimistic UI: append DisplayMessage(Pending + ClockIcon) -> scroll bottom
   ├─► Write-through IndexedDB: `chat_outbox` + `chat_messages` (DB v3)
   ├─► Online? -> Encrypt E2EE envelope -> POST /api/messages (or Group/Channel API)
   │      ├─► 200 OK -> Replace local-* ID with server ID/seq, status=Sent, delete outbox entry
   │      └─► Offline/Network error -> Keep Pending in `chat_outbox`, auto-flush on `online`/WS reconnect
   │
[Incoming Realtime Events]
   Server (Axum /api/ws?proto=bin)
      │  P-256 ECDH + HKDF-SHA256 -> ephemeral 256-bit AES-GCM session key
      │  Encrypt WsEvent -> CWBF1 binary frame (28B header + ciphertext + 16B GCM tag, AAD=header)
      ▼
   Dedicated Web Worker (`app/web/network_worker.js`)
      │  Decrypts CWBF1 binary frame off Main Thread via Web Crypto AES-GCM
      │  Ping/Pong watchdog + exponential backoff reconnect
      ▼
   Main Thread Bridge (`app/web/network_bridge.js` -> `app/src/platform/web.rs::run_ws_loop`)
      │  Emits WsEvent to Dioxus -> updates Signals + writes through to IndexedDB `chat_messages`

[Protected Chunked Media Streaming]
   `<video src="/stream/media/{grantId}">`
      │  HTTP `Range: bytes=start-end`
      ▼
   Service Worker (`app/web/sw.js`)
      │  4-Layer 403 Guard:
      │    1) Block `mode === 'navigate'` / `destination === 'document'` -> 403
      │    2) Allow only `video|audio|image|empty` destinations -> else 403
      │    3) Require valid unexpired `grantId` -> else 403
      │    4) Require `event.clientId === grant.ownerClientId` & live same-origin client -> else 403
      │
      │  If E2EE `CWCH2`:
      │    - Read 21B header (chunkSize, chunkCount, totalPlainLen)
      │    - Compute O(1) encrypted chunk offset: `21 + idx * (4 + chunkSize + 16)`
      │    - Fetch ONLY needed chunk(s) with `Range` + `X-ChatW-Stream-Token`
      │    - Decrypt per-chunk AES-256-GCM (nonce XOR idx, AAD=`CWCH2||idx||isFinal`) + LRU cache
      │    - Return HTTP `206 Partial Content` (`Content-Range`, `Accept-Ranges: bytes`)
      ▼
   Axum Backend (`web/backend/src/attachments/mod.rs` & `main.rs`)
      │  - `/stream/media/{*rest}` fallback -> ALWAYS `403 Forbidden`
      │  - `/api/attachments/{id}` & `/uploads/media/{file}`:
      │      Block `Sec-Fetch-Mode: navigate` / `Sec-Fetch-Dest: document` / `Sec-Fetch-Site: cross-site` -> `403 Forbidden`
      │      Validate `X-ChatW-Stream-Token` (HMAC-SHA256) if supplied
      │      Serve exact HTTP `206 Partial Content` Range slice
```

## File Change Manifest

### Backend (`web/backend/`)
1. `web/backend/src/websocket/mod.rs`:
   - Add `WsQuery { proto: Option<String> }` on `/api/ws`.
   - Implement `CWBF1` binary framing, P-256 ECDH + HKDF-SHA256 ephemeral key derivation, AES-256-GCM frame encryption (`OP_HELLO`, `OP_HELLO_ACK`, `OP_EVENT`, `OP_PING`, `OP_PONG`), and Rust unit tests.
2. `web/backend/src/attachments/mod.rs`:
   - Add `enforce_anti_direct_media_access(&headers, &state, user_id, session_id, path)` returning `403 Forbidden` on direct document navigation (`Sec-Fetch-Mode: navigate` / `Sec-Fetch-Dest: document` / `Sec-Fetch-Site: cross-site`) or invalid `X-ChatW-Stream-Token`.
   - Add `POST /api/attachments/stream-token` issuing short-lived (5-min) HMAC-SHA256 tokens bound to `(user_id, session_id, path)`.
   - Add unit tests for anti-navigation guard & HMAC stream token verification.
3. `web/backend/src/main.rs`:
   - Add explicit `/stream/media/{*rest}` route returning `403 Forbidden` for any non-SW request.
   - Add `Service-Worker-Allowed: /` header for `/sw.js` and `media-src 'self' blob: data:;` in CSP.
   - Add `x-chatw-stream-token` and `range` to CORS `allow_headers`.

### Frontend Web Workers & JS Engines (`app/web/`)
4. `app/web/network_worker.js` *(new)*: Dedicated Web Worker for `/api/ws?proto=bin`, P-256 ECDH + HKDF + AES-256-GCM decryption, `OP_PING`/`OP_PONG` heartbeat, exponential backoff + jitter reconnect.
5. `app/web/network_bridge.js` *(new)*: Main-thread bridge (`window.__chatwNetBridge`) managing the Worker, online/offline reconnection, and stats telemetry.
6. `app/web/sw.js` *(new)*: Root-scope Service Worker intercepting `/stream/media/{grantId}`, 4-layer `403 Forbidden` anti-direct-access enforcement, non-E2EE Range passthrough, and O(1) random-access `CWCH2` chunked AES-256-GCM HTTP `206 Partial Content` streaming with LRU chunk cache.
7. `app/web/media_stream_bridge.js` *(new)*: Registers `/sw.js`, acquires HMAC stream tokens, manages tab-bound grants (`window.__chatwMediaStream`).
8. `app/web/storage_engine.js`: Bump `DB_VERSION` 2 → 3; add `chat_messages` and `chat_outbox` stores + clean async methods.
9. `app/web/media_optimizer.js`: Generate 16×16 micro-blur `blurDataUrl` (~250B) + `width`/`height` for images and video poster frames.
10. `app/web/index.html` & `app/build-web.sh`: Load and bundle `network_bridge.js`, `media_stream_bridge.js`, `network_worker.js`, and `/sw.js`.

### Frontend Rust / Dioxus (`app/src/` & `app/assets/`)
11. `app/src/platform/web.rs`: Wire `run_ws_loop` through `window.__chatwNetBridge` (Dedicated Worker binary WS) with graceful fallback.
12. `app/src/storage/{mod.rs,browser.rs,native.rs}`: Add `CachedDmMessage`, `OutboxEntry`, and async wrappers for IndexedDB `chat_messages` & `chat_outbox`.
13. `app/src/storage/media_opt.rs`: Add `blur_data_url`, `width`, `height` to `OptimizedMediaPayload`.
14. `app/src/components/rich.rs`:
    - Add `blur`, `w`, `h` (`#[serde(default)]`) to `ImageData` and `MediaFileData`.
    - Progressive blur-up cross-fade in `RichImage` & `RichMediaAttachment`.
    - Register tab-bound SW `/stream/media/{grantId}` for `video`/`audio` in `RichMediaAttachment` (with `use_drop` grant cleanup).
15. `app/src/components/messages.rs`:
    - Render crisp `ClockIcon { size: 12 }` when `msg.is_self && msg.is_pending` + `.pending` row class.
    - Preserve exact `scrollHeight` anchor when expanding older virtualized slices (`dom_render_limit += 120`).
    - Add unit test verifying 100,000-message window bounds (`120` mounted out of `100,000`).
16. `app/src/pages/chat.rs`:
    - Wire 0ms IndexedDB read-first hydration + write-through for DM, Group, and Channel histories.
    - Wire Optimistic UI (`MessageStatus::Pending` / `local-*` IDs) + `chat_outbox` persistence + automatic FIFO flush on reconnect/`online`!
    - Fix `is_pending` mapping (`m.status == MessageStatus::Pending` / `m.id.starts_with("local-")`).
17. `app/assets/main.css`:
    - Add `content-visibility: auto; contain-intrinsic-size: auto 68px;` for `.message-row`, `.pending` bubble style + `.pending-clock`, and progressive blur-up cross-fade rules (`.rich-image-stage`, `.rich-image-blur`, `.rich-image-full`).

## Verification Strategy
1. `node --check` on all 6 JS files (`network_worker.js`, `network_bridge.js`, `sw.js`, `media_stream_bridge.js`, `storage_engine.js`, `media_optimizer.js`).
2. `cargo test -p chatw` + `cargo test -p chatw-app --lib` (verify ECDH binary WS framing, anti-hotlink 403 guard, HMAC stream token, 100k message virtual window bounds).
3. Rebuild backend (`cargo build -p chatw`), cleanly restart `:8085` server, rebuild release WASM (`./app/build-web.sh release`).
4. Write & run Playwright E2E `app/e2e/spec069-telegram-grade-worker-sw-streaming-offline.mjs` covering all 4 pillars + visual screenshot inspection via `read_file`.
5. Run all existing E2E regression suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`).
