# Implementation Tasks: Spec 165

## Phase 1: Spec & Dependencies
- [x] T001 Create `specs/165-unified-db-valkey-sqlcipher-security/{spec.md,plan.md,tasks.md}`
- [x] T002 Add `redis` (`0.32`, `tokio-comp`, `connection-manager`, `script`) to `web/backend/Cargo.toml` and `rusqlite` (`0.37`, `bundled-sqlcipher`, `modern_sqlite`) to `crates/chatw-msg-store/Cargo.toml` + `app/Cargo.toml` (native)

## Phase 2: PostgreSQL Migration & Valkey Coordinator
- [x] T003 Create `web/backend/migrations/0041_inline_small_chunks_and_ws_events.sql` (`inline_payload BYTEA` zero-duplication CHECK, `ws_event_log`, `idempotency_keys`, covering indexes)
- [x] T004 Implement `web/backend/src/cache/valkey.rs` & `web/backend/src/cache/mod.rs` (presence TTL, typing TTL, rate limiter, Lua distributed locks, task dedup, metadata cache, multi-node WS Pub/Sub, fast-fail timeout + circuit breaker + automatic graceful fallback on outage/restart)
- [x] T005 Update `Config`, `AppState`, `main.rs`, `typing/mod.rs`, `auth/mod.rs`, and all integration test `AppState` initializers

## Phase 3: HTTP Idempotency, Inline Small Chunks & WS Reconnect Gap Recovery
- [x] T006 Update `web/backend/src/storage/message_chunks.rs` for `pg_inline` vs Object Storage threshold, Valkey lock/cache integration, `Idempotency-Key`, and `WsEvent::ManifestUpdated` / `ChunkChanged` emission + durable `ws_event_log`
- [x] T007 Update `web/backend/src/websocket/mod.rs`, `messages/mod.rs`, and `devices_sync/mod.rs` with `ManifestUpdated`/`ChunkChanged`/`PermissionsChanged`, Valkey Pub/Sub fan-out, HTTP `Idempotency-Key`, and `GET /api/sync/events?after_event_id=` reconnect recovery

## Phase 4: Native Unified SQLite + SQLCipher + FTS5 Database
- [x] T008 Implement `crates/chatw-msg-store/src/sqlite_store.rs` (`EncryptedSqliteStore` with SQLCipher AES-256, FTS5 Vietnamese accented/unaccented search, O(log N) rowid updates/deletes, external encrypted chunk refs, corrupt-DB recovery)
- [x] T009 Wire `EncryptedSqliteStore` into `app/src/storage/msg_store.rs` (`native_impl`)

## Phase 5: Web / PWA Security & Protocol Hardening
- [x] T010 Harden `app/web/storage_engine.js` (remove raw `localStorage` fallback key, use non-extractable WebCrypto vault wrapping, zero duplicate storage) and `app/web/core_bridge.js` + `app/src/platform/native.rs` (`Idempotency-Key`, WS reconnect event catch-up, zero sensitive console logs)

## Phase 6: Benchmarks, Tests, Build & Mandatory Git Push
- [x] T011 Run `crates/chatw-msg-store/examples/benchmark_spec165_storage.rs` (SQLite vs SQLCipher, inline BLOB vs external file, 1M-message FTS5 Vietnamese search)
- [x] T012 Run `web/backend/tests/spec165_valkey_sqlcipher_security_test.rs` (live `redis-server` Valkey presence/typing/locks/PubSub + **Valkey restart resilience**, PG inline vs CAS, idempotency, WS reconnect recovery, F12/RBAC/IDOR security checks) + full workspace `cargo test`
- [x] T013 Build Web bundle (`./app/build-web.sh release`) & Backend binary, verify zero `.map` files in `app/dist`, restart live ChatW backend daemon
- [x] T014 Mark all tasks `[x]`, `git add .`, commit, and `git push origin main` to `https://github.com/kiyogo/chart`
