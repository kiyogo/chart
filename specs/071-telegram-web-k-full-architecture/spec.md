# Feature Specification: Telegram Web K/A Full 12-Pillar Architecture (Spec 071)

**Feature Branch**: `071-telegram-web-k-full-architecture`  
**Created**: 2026-11-23  
**Status**: In Progress  
**Input**: Complete refactor of ChatW's Network, Storage, Media Streaming, Security, and Virtual Rendering layers to match Telegram Web K (`tweb`) 1:1 across all 12 production engineering pillars:
1. `SharedWorker` (`core_worker.js`, fallback Dedicated Worker) + single ECDH P-256 + HKDF-SHA256 + AES-256-GCM Binary RPC Multiplexer (`CWBF2`, Zlib deflate > 512B, 16-byte random padding, monotonic `seq`, in-process Axum router dispatch) eliminating 100% of post-login `fetch('/api/...')` & Main-Thread WebSocket connections.
2. Priority `DownloadManager` inside `SharedWorker` (`CRITICAL > HIGH > NORMAL > LOW`, concurrency cap 6, Abort cancellation) pulling media chunks over Binary WS (`OP_MEDIA_CHUNK_REQ`/`RES`) and exposing ONLY `blob:` URLs in the DOM (0 `/uploads/...` or `/api/attachments/...` in `<img>`).
3. Service Worker (`/sw.js`) `/stream/{grantId}` proxy pulling chunks directly from `SharedWorker` over a direct `MessagePort` (0 server `fetch()` in SW!), streaming HTTP `206 Partial Content` `ReadableStream` with O(1) E2EE `CWCH2` seek + smart 1–2 chunk prefetch + 5-layer `403 Forbidden` protection (and backend `/stream/*` 403 fallback).
4. App Shell & Static Assets offline cache (`Cache-First` + `Stale-While-Revalidate` in `/sw.js` + IndexedDB `static_cache` for `/locales/*.json` & `/stickers/openmoji/catalog.json` eliminating redundant fetches).
5. Offline-first 0ms IndexedDB v4 (`dialogs`, `chat_messages`, `users_peers`, `chat_outbox`, `sync_state`, `drafts`, `static_cache`, `crypto_keys`) + `PTS/QTS` Delta Sync (`OP_DELTA_SYNC_REQ`/`RES`) + `<5ms` Multi-Tab sync via `SharedWorker` ports.
6. Optimistic UI (0ms `ClockIcon` pending bubble) + persistent FIFO `chat_outbox` + backend `random_id` (`message_id`) idempotency guaranteeing 0 duplicates and 0 lost messages across offline/reconnect/reload.
7. Dedicated Media Web Worker (`media_worker.js`, `OffscreenCanvas`, EXIF stripping, micro-blur preview, 48-bar voice waveform) + Resumable 512 KiB chunked Binary WS upload (`OP_UPLOAD_PART`/`OP_UPLOAD_ACK`) with progress & cancel/resume.
8. True bi-directional 100,000+ message Virtual Scroller (≤ 120 `.message-row` DOM nodes at any scroll position, top/bottom virtual spacers, scroll anchoring, Jump-to-Message highlight, floating scroll-to-bottom button with unread badge).
9. Progressive 0ms Blur-Up media (`aspect-ratio`, 0 CLS), interactive Canvas/SVG Voice Waveform seek, smart sticky-bottom + unread scroll button, and IndexedDB per-chat drafts.
10. Non-extractable Web Crypto `CryptoKey` (`extractable: false`) storage in IndexedDB, zero plaintext tokens/messages in `localStorage`/`sessionStorage`, and full logout/lock wipe across all tabs, SW grants, and `blob:` URLs.
11. Backend `/api/ws?proto=rpc-bin` binary RPC + in-process `tower::ServiceExt::oneshot` dispatch + O(1) seek range reads (`OP_MEDIA_CHUNK_REQ`) + idempotent message sends + strict `403 Forbidden` on `/stream/*` & direct media navigation.
12. Comprehensive live Playwright E2E suite verifying Zero Fetch/XHR, Zero Main-Thread WS, 100% `blob:` images, `/stream/{grantId}` 206 seek + 403 anti-leak, 0ms offline hydration + 10-message offline burst with 0 duplicates, 100,000-message virtual scroller (≤ 120 DOM nodes), and 0 console/page errors.
