# Feature Specification: Unified Database, Valkey Coordination, SQLCipher+FTS5 Local DB, HTTP/WS Protocol & Security Hardening (Spec 165)

**Feature Branch**: `main`  
**Created**: 2026-11-26  
**Status**: In Progress  
**Extends**: Spec 164 (`164-zstd22-msg-storage-search`) — unified into a single coherent architecture with ZERO duplicate subsystems.

---

## 1. Objective & Architectural Allocation (Sections 36–44)

Unify ChatW's server, native, and web data layers under a strict single-responsibility technology map without introducing duplicate databases, duplicate caches, or weakening E2EE / existing security layers:

| Layer | Technology | Responsibility |
|---|---|---|
| Primary Server DB | **PostgreSQL 16/18** | Accounts, profiles, contacts, groups, servers, channels, RBAC, devices, `<7d` E2EE messages, chunk manifests/versions, message mappings, sync state, durable job queues, mutations/edits/deletes/recalls, durable WS event log, idempotency keys, and **inline small chunks** (`<= SMALL_CHUNK_INLINE_THRESHOLD`, benchmarked) with strict zero-duplication vs Object Storage. |
| Ephemeral Server Coordinator | **Valkey** (RESP-compatible) | Online presence (TTL), typing indicators (TTL), non-sensitive metadata cache (TTL), distributed rate limiting, atomic Lua distributed lock/lease fast-path, task deduplication, multi-device coordination, multi-node WebSocket Pub/Sub fan-out. Strictly bounded RAM/TTL; **never** sole store of messages; full automatic recovery from PostgreSQL on Valkey restart or outage. |
| Object Storage | **MinIO / S3-compatible** (existing `StorageRegistry`) | Zstd-22 E2EE chunks (`> SMALL_CHUNK_INLINE_THRESHOLD`), media, thumbnails, encrypted private dictionaries, encrypted search index shards, encrypted backups. Private bucket only; zero credential/internal-URL leakage. |
| Native Local DB (Android, iOS, Windows, Linux, macOS) | **Unified SQLite + SQLCipher + FTS5** | Single encrypted database per account (`chatw_account_{id}.db`) storing local messages, metadata, dialogs, message/chunk mappings, manifests, sync/read states, cache index, task queue, offline state, and Vietnamese/multilingual FTS5 full-text search index. Large Zstd-22 chunks & media stored as encrypted external files referenced by DB (benchmarked vs inline BLOB). |
| Web / PWA Local Storage | **OPFS + IndexedDB + Rust WASM Worker (`chatw-msg-store`)** | Encrypted page/record storage in OPFS + IndexedDB metadata + off-main-thread Rust WASM `CWSE1` encrypted inverted index + Zstd-22 pure-Rust engine. Zero plaintext keys in `localStorage`; non-extractable WebCrypto `CryptoKey` wrapping for local vault keys; zero duplicate copies across stores. (Honest assessment: native C SQLCipher / C SQLite WASM requires clang/emscripten which is absent in `wasm32-unknown-unknown` toolchain, so page/shard-level AES-256-GCM encryption + Rust WASM `CWSE1` index provides equivalent security and sub-millisecond search without bloating the CWRT1 bundle.) |
| Messaging Protocol | **HTTP Request Send + WebSocket Receive** | All mutations (`send`, `edit`, `recall`, `delete`, `react`), history sync, chunk download/upload, search index sync, manifest sync, and dictionary fetch remain HTTP Requests with `Idempotency-Key` deduplication, retry with exponential backoff, keep-alive/HTTP2, and cancellation. WebSocket remains receive-only for realtime events including `manifest.updated`, `chunk.changed`, `permissions.changed`, plus ordered HTTP gap recovery (`GET /api/sync/events?after_event_id=`) on reconnect. |
| Security & F12 Hardening | **Existing ChatW Security + E2EE** | Zero plaintext messages/chunks/search-index/private-dict/keys in Network, WS frames, Console, IndexedDB, OPFS, query params; zero S3 credentials or internal URLs; zero production source maps; strict RBAC, anti-IDOR, cross-account isolation, anti-replay. |

---

## 2. Acceptance Criteria

1. **PostgreSQL + Small-Chunk Inline Threshold**:
   - Migration `0041_inline_small_chunks_and_ws_events.sql` adds `inline_payload BYTEA` to `message_chunks` with DB `CHECK` constraint enforcing **mutually exclusive** storage (`pg_inline` vs `s3_ready`/`local_cas`) so chunks are NEVER duplicated between PostgreSQL and Object Storage.
   - `message_chunks` commits chunks `<= small_chunk_inline_threshold_bytes` (default 16 KiB, benchmark-validated) into `pg_inline`, and larger chunks into Object Storage (`s3_ready` or `local_cas`).
2. **Valkey Coordinator & Restart Resilience**:
   - `web/backend/src/cache/valkey.rs` connects via `VALKEY_URL` / `REDIS_URL` (gracefully disabled / automatic PG fallback if unconfigured or temporarily down).
   - Implements presence TTL, typing TTL, non-sensitive cache TTL, distributed rate limiter, atomic Lua distributed lock/lease, task dedup, and `WsHub` multi-instance Pub/Sub event distribution (`instance_id` loop prevention).
   - Verified against a live `redis-server` process: killing and restarting the Valkey server mid-operation causes **zero message loss**, **zero failed durable writes**, and seamless fallback/reconnection.
3. **HTTP Send / WebSocket Receive + Reconnect Gap Recovery**:
   - `Idempotency-Key` header deduplication backed by PostgreSQL `idempotency_keys` (and Valkey fast-path) guarantees duplicate HTTP message sends / chunk commits never create duplicate rows.
   - `WsEvent` adds `manifest.updated`, `chunk.changed`, and `permissions.changed`.
   - Durable `ws_event_log` table + `GET /api/sync/events?after_event_id=&limit=` HTTP endpoint enables ordered, deduplicated gap recovery after WebSocket disconnects or Valkey restarts.
4. **Unified Native SQLite + SQLCipher + FTS5 Database**:
   - `crates/chatw-msg-store/src/sqlite_store.rs` (feature `sqlite-sqlcipher`, native targets) implements a single AES-256 SQLCipher encrypted DB per account containing messages, dialogs, chunk manifests, external chunk file references, message-chunk mapping, sync/read states, cache index, task queue, and `messages_fts` (`FTS5`, `unicode61`, dual accented `text_exact` + Vietnamese accent-folded `text_folded` columns).
   - Wired directly into `app/src/storage/msg_store.rs` (`native_impl`) replacing ad-hoc standalone shard files.
   - Verified: raw `.db` file on disk contains zero readable plaintext, wrong key fails to open, corruption triggers controlled recovery, and FTS5 searches Vietnamese accented/unaccented/prefix/phrase/sender/scope/time filters accurately.
5. **Web / PWA Security & F12 Audit**:
   - Eliminates raw `chatw_msgstore_rk_*` key persistence from `localStorage` in `app/web/storage_engine.js`; replaces with non-extractable WebCrypto `device_vault_aes` wrapped vault key and zero-duplication OPFS/IDB policy.
   - Verifies zero sensitive console logging, zero production `.map` files in `app/dist/`, and intact CWRT1 / CWBF2 / media streaming protections.
6. **Benchmarks & Comprehensive Test Suite**:
   - Real measured benchmarks for:
     - PostgreSQL `pg_inline` vs Object Storage across chunk sizes (2 KiB – 256 KiB).
     - SQLite vs SQLCipher encryption overhead, inline BLOB vs external encrypted file, and FTS5 Vietnamese search scaling (up to 1,000,000 messages!).
     - Valkey latency & RAM footprint.
   - All workspace tests + real Valkey integration & restart test + Web release build pass cleanly and are pushed to `origin main`.
