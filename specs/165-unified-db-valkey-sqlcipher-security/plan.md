# Technical Implementation Plan: Spec 165

## 1. Crate & Dependency Additions (Researched Open-Source Best Practices)
- `web/backend/Cargo.toml`: Add `redis = { version = "0.32", features = ["tokio-comp", "connection-manager", "script"] }` (official Rust RESP/Valkey-compatible async client with auto-reconnecting `ConnectionManager` and Pub/Sub).
- `crates/chatw-msg-store/Cargo.toml`: Add native-only optional `rusqlite = { version = "0.37", features = ["bundled-sqlcipher", "modern_sqlite"] }` behind feature `sqlite-sqlcipher` (bundled SQLCipher statically compiles SQLite 3 + FTS5 + AES-256-CBC/HMAC-SHA512 page encryption linked against system `libcrypto`).
- `app/Cargo.toml`: Enable `sqlite-sqlcipher` on `chatw-msg-store` for `cfg(not(target_arch = "wasm32"))`.

## 2. Database Migration (`web/backend/migrations/0041_inline_small_chunks_and_ws_events.sql`)
1. `ALTER TABLE message_chunks ADD COLUMN IF NOT EXISTS inline_payload BYTEA;`
   - Add `CHECK ((storage_backend = 'pg_inline' AND inline_payload IS NOT NULL) OR (storage_backend <> 'pg_inline' AND inline_payload IS NULL))` enforcing strict zero-duplication between PostgreSQL and MinIO/S3.
   - Add covering partial indexes for active non-superseded chunk lookups & message-seq range scans.
2. `CREATE TABLE IF NOT EXISTS ws_event_log (...)`:
   - `event_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY`
   - `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`
   - `event_type TEXT NOT NULL`
   - `scope_type TEXT`
   - `scope_id TEXT`
   - `message_id UUID`
   - `payload JSONB NOT NULL`
   - `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
   - Index `(user_id, event_id DESC)` and `(created_at)` for bounded retention pruning.
3. `CREATE TABLE IF NOT EXISTS idempotency_keys (...)`:
   - `(user_id UUID NOT NULL, idempotency_key TEXT NOT NULL, method TEXT NOT NULL, route_path TEXT NOT NULL, status_code SMALLINT NOT NULL, response_json JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + interval '24 hours', PRIMARY KEY (user_id, idempotency_key))`

## 3. Server Modules
- `web/backend/src/config.rs`: Add `valkey_url: Option<String>` (`VALKEY_URL` / `REDIS_URL`) and `small_chunk_inline_threshold_bytes: usize` (`SMALL_CHUNK_INLINE_THRESHOLD_BYTES`, default `16384`).
- `web/backend/src/cache/mod.rs` & `web/backend/src/cache/valkey.rs`:
  - `ValkeyCoordinator`: non-blocking connect, `disabled()` mode, automatic circuit-breaker & reconnect.
  - `set_user_online` / `refresh_user_online` / `clear_user_online` / `is_user_online` (TTL 90s).
  - `set_typing` / `clear_typing` / `is_typing` (TTL 8s).
  - `check_rate_limit(bucket, limit, window_secs)` (atomic `INCR` + `EXPIRE` Lua script, local fallback when Valkey unavailable).
  - `try_acquire_lock` / `renew_lock` / `release_lock` (atomic Lua fencing-token compare-and-set/del).
  - `dedup_task(key, ttl_secs)` (`SET NX EX`).
  - `cache_get_json` / `cache_set_json` / `cache_del` (non-sensitive metadata only, strict TTL).
  - `publish_ws_event(instance_id, user_ids, event)` & `spawn_ws_pubsub_subscriber(ws_hub)` for multi-node WebSocket fan-out.
- `web/backend/src/websocket/mod.rs`:
  - Add `WsEvent::ManifestUpdated`, `WsEvent::ChunkChanged`, `WsEvent::PermissionsChanged`.
  - Add optional Valkey Pub/Sub hook (`instance_id: Uuid`, `valkey: Option<Arc<ValkeyCoordinator>>`) to `WsHub` so `send_to_user` / `send_to_users` deliver locally AND publish to peer nodes without duplicate local delivery.
  - Maintain Valkey online presence TTL on `register` / heartbeat / `unregister`.
- `web/backend/src/storage/message_chunks.rs`:
  - Support `pg_inline` storage for chunks `<= state.config.small_chunk_inline_threshold_bytes` and `s3_ready`/`local_cas` above threshold (zero duplication).
  - Integrate Valkey distributed lock fast-path + non-sensitive manifest metadata cache invalidation + `WsEvent::ManifestUpdated` / `WsEvent::ChunkChanged` + durable `ws_event_log` insertion!
  - Support `Idempotency-Key` header on `/msg-chunks/commit`.
- `web/backend/src/messages/mod.rs` & `web/backend/src/devices_sync/mod.rs`:
  - Support `Idempotency-Key` header on `POST /api/messages/` (in addition to `message_id` PK idempotency) + append durable `ws_event_log` entries on message send/edit/delete/pin/react.
  - Add `GET /api/sync/events?after_event_id=&limit=` for ordered, deduplicated WebSocket reconnect gap recovery.
- `web/backend/src/typing/mod.rs` & `web/backend/src/auth/mod.rs`:
  - Wire Valkey typing TTL + distributed login rate-limiter fallback + `AppState.valkey`.

## 4. Native Unified SQLite + SQLCipher + FTS5 (`crates/chatw-msg-store/src/sqlite_store.rs`)
- Single AES-256 SQLCipher encrypted DB per account:
  - `PRAGMA key = "x'<64-hex-chars>'"` (HKDF-SHA256 derived with domain `chatw-sqlcipher-db-v1` + account_id — separate from message/chunk/search keys!).
  - `PRAGMA cipher_memory_security = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON; PRAGMA page_size = 4096;`
- Schema: `messages_local`, `dialogs_meta`, `chunk_manifests`, `chunk_refs`, `message_chunk_map`, `sync_state`, `read_state`, `cache_index`, `task_queue`, `kv_meta`, and `messages_fts USING fts5(message_id UNINDEXED, scope_key UNINDEXED, sender_id UNINDEXED, created_at_ms UNINDEXED, msg_type UNINDEXED, text_exact, text_folded, tokenize='unicode61 remove_diacritics 0')`.
- Corrupt-DB detection (`PRAGMA quick_check`) + quarantine & safe rebuild recovery.
- External encrypted chunk files stored under `<account_dir>/chunks/{chunk_id}.cwec` (referenced in `chunk_refs`, zero BLOB duplication in SQLite).
- Wired into `app/src/storage/msg_store.rs` (`native_impl`).

## 5. Web / PWA Hardening & Protocol Polish
- `app/web/storage_engine.js`: Remove raw `localStorage` fallback root key (`chatw_msgstore_rk_*`), scrub any legacy key on boot, and wrap fallback vault keys exclusively using the non-extractable WebCrypto `device_vault_aes` key (`extractable: false`). Keep OPFS primary + IDB fallback with zero duplicate payloads.
- `app/web/core_bridge.js` & `app/src/services/api.rs`: Track `lastEventId`, send `Idempotency-Key` on HTTP message sends, trigger ordered HTTP gap recovery (`GET /api/sync/events?after_event_id=`) on WebSocket reconnect, and eliminate noisy production console logs.
- Verify `app/dist/` has zero `.map` source maps, zero exposed `.wasm` URLs, zero plaintext secrets.

## 6. Verification & Benchmarks
- `crates/chatw-msg-store/examples/benchmark_spec165_storage.rs`: Real measured benchmarks for SQLite vs SQLCipher overhead, inline BLOB vs external encrypted file, 1M-message FTS5 Vietnamese search, and PostgreSQL `pg_inline` vs Object Storage chunk threshold.
- `web/backend/tests/spec165_valkey_sqlcipher_security_test.rs`: Real `redis-server` Valkey lifecycle + mid-flight restart resilience test, multi-node Pub/Sub test, `pg_inline` vs CAS zero-duplication test, HTTP `Idempotency-Key` dedup test, WS reconnect `after_event_id` gap recovery test, and API/F12/IDOR security audit tests.
