# Feature Specification: Telegram Web K/A-Grade Worker, Service Worker Streaming, Offline-First & Virtual Chat Architecture

**Feature Branch**: `069-telegram-grade-worker-sw-offline-architecture`  
**Created**: 2026-11-23  
**Status**: In Progress  
**Input**: Upgrade ChatW with full Telegram Web K/A production architecture: (1) Dedicated Web Worker handling WebSocket + ECDH-negotiated encrypted binary protocol (`CWBF1`), (2) Service Worker media proxy (`/stream/media/{grantId}`) with HTTP `206 Partial Content` chunked streaming (including O(1) random-access E2EE `CWCH2` decryption) and strict `403 Forbidden` anti-direct-access enforcement in both SW and Axum backend, (3) Offline-first IndexedDB message cache (0ms hydration) + persistent offline outbox + Optimistic UI (`ClockIcon` pending state), and (4) 100,000+ message virtualized scroll performance with scroll-anchor preservation + progressive blur-up micro-preview media loading.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Multi-Threaded Dedicated Web Worker & Encrypted Binary WebSocket (`CWBF1`) (Priority: P1)
As a ChatW user, I want all WebSocket networking, heartbeats, reconnects, and realtime frame decryption to execute in a Dedicated Web Worker using an encrypted binary protocol so the UI stays at 60fps and F12 Network inspection only shows opaque encrypted binary frames.

**Acceptance Scenarios**:
1. **Given** an authenticated web session, **When** the app boots, **Then** a Dedicated Web Worker (`/network_worker.js`) opens `ws(s)://.../api/ws?proto=bin` with `binaryType = 'arraybuffer'`.
2. **Given** a binary WS upgrade, **When** the server completes the ephemeral P-256 ECDH + HKDF-SHA256 handshake (`OP_HELLO` / `OP_HELLO_ACK`), **Then** every subsequent `WsEvent` travels as an AES-256-GCM encrypted `Message::Binary` (`CWBF1`) frame with header AAD authentication and zero plaintext JSON on the wire.
3. **Given** a real-time DM/group/channel event, **When** the Worker decrypts the `CWBF1` binary frame off the Main Thread and posts it to the UI bridge, **Then** the message appears in real time with identical behavior to the legacy JSON channel.
4. **Given** legacy or native clients connecting to `/api/ws` without `?proto=bin`, **Then** the server maintains 100% backward-compatible JSON text transport.

### User Story 2 — Service Worker Chunked HTTP 206 Media Streaming & Strict `403 Forbidden` Protection (Priority: P1)
As a ChatW user, I want videos and audio (both standard and E2EE `CWCH2`) to start playing and seek immediately via chunked HTTP `206 Partial Content` through a Service Worker proxy, while preventing unauthorized direct access if anyone copies a stream or media URL into another tab.

**Acceptance Scenarios**:
1. **Given** the web app loads on same-origin HTTP(S), **When** `media_stream_bridge.js` initializes, **Then** `/sw.js` registers at scope `/` and controls the tab.
2. **Given** a video or audio attachment (non-E2EE or E2EE `CWCH2`), **When** rendered in `RichMediaAttachment`, **Then** the client registers a tab-scoped grant with `/sw.js` and streams via `/stream/media/{grantId}` returning `206 Partial Content` with accurate `Content-Range` and `Accept-Ranges: bytes`.
3. **Given** an E2EE `CWCH2` video, **When** the player requests a byte range in the middle of the video (seek), **Then** `/sw.js` reads the 21-byte `CWCH2` header, calculates the exact O(1) ciphertext chunk offset(s), fetches ONLY those encrypted chunks via Range, decrypts via Web Crypto AES-256-GCM with per-chunk derived nonce & AAD, caches decrypted chunks, and returns exact `206 Partial Content` bytes.
4. **Given** a user copies `/stream/media/{grantId}`, `/api/attachments/{id}`, or `/uploads/media/{file}` and opens it directly in a new browser tab (`mode: navigate` / `destination: document`) or from another tab without the bound `clientId`, **Then** `/sw.js` and/or the Axum backend immediately returns **`403 Forbidden`**.

### User Story 3 — Offline-First IndexedDB Instant Hydration, Persistent Outbox & Optimistic UI (Priority: P1)
As a ChatW user, I want opening a chat or reloading the page to render cached message history from IndexedDB in 0ms, and sending a message to appear immediately with a pending clock icon (`ClockIcon`)—automatically queuing in IndexedDB if offline and delivering once reconnected.

**Acceptance Scenarios**:
1. **Given** a conversation with prior history, **When** the user switches to it or reloads the page, **Then** `chatw_storage_v1` IndexedDB (`chat_messages`) hydrates the cached history immediately before network fetch completes, and reconciles cleanly when the server responds.
2. **Given** the user sends a DM, group, or channel message, **When** they press Enter, **Then** the message bubble renders at 0ms with a pending `ClockIcon` indicator and transitions to confirmed `Sent` upon server acknowledgement.
3. **Given** the browser goes offline, **When** the user sends a message, **Then** the message stays visible in pending state and is persisted in IndexedDB `chat_outbox`; when connectivity returns (`online` event / WS reconnect), the outbox flushes automatically in FIFO order and marks the message `Sent`.

### User Story 4 — 100,000+ Message Virtual Scroll Performance & Progressive Blur-Up Media (Priority: P2)
As a ChatW user in a huge conversation (100,000+ messages) with rich images and videos, I want smooth 60fps scrolling with zero jump when loading older slices, and instant blurred micro-previews before full high-res media finishes loading.

**Acceptance Scenarios**:
1. **Given** a message list with 100,000 messages in memory, **When** rendered by `MessageList`, **Then** `compute_message_window_bounds` mounts only the active window (`120` rows initially), CSS applies `content-visibility: auto`, and expanding older slices preserves exact scroll position with zero jump.
2. **Given** an image or video attachment uploaded via the web composer, **When** optimized by `media_optimizer.js`, **Then** a tiny 16×16 blurred micro-preview (`blur`, ~250 bytes) and aspect-ratio dimensions (`w`, `h`) are embedded in the rich payload.
3. **Given** a message containing `blur` + `w`/`h`, **When** rendered in `RichImage` or `RichMediaAttachment`, **Then** the exact aspect-ratio box displays the blurred preview at 0ms and cross-fades smoothly (`220ms`) once the full media is ready.

## Non-Functional & Compliance Requirements
- **Zero regressions**: All existing Rust unit tests (`cargo test --workspace`) and Playwright E2E suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`) must remain 100% green.
- **Canonical English UI strings & zero `...`**: All new code strings in English, no trailing `...`.
- **Real crypto & real transport**: No mock crypto or mock APIs; genuine P-256 ECDH, HKDF-SHA256, AES-256-GCM, HMAC-SHA256, Web Worker, Service Worker, and IndexedDB.
