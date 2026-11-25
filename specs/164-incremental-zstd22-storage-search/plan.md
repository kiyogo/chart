# Technical Plan — Spec 164

## 1. Workspace & Crate Design (Single Shared Rust Core)
Add `crates/chatw-msg-store` to root `Cargo.toml` workspace members, depended upon by:
- `web/backend` (`features = ["native-zstd"]` -> `zstd 0.13` C `libzstd` Level 22 + dict trainer)
- `app` (`chatw-app`: `native-zstd` on `not(target_arch = "wasm32")`, `pure-zstd` on `wasm32`)
- `crates/chatw-media-worker` (`features = ["pure-zstd"]` -> `structured-zstd 0.4` pure-Rust RFC 8878 Level 22 + dictionaries + SIMD128 in `--release` WASM worker!)

### Modules inside `crates/chatw-msg-store/src/`:
- `lib.rs`: Public API, version constants, error types (`MsgStoreError`), cross-platform entrypoints.
- `model.rs`: `CompactMessage`, `MessageFlags`, `ScopeKind`, `ChunkMetadata`, `ChunkManifest`, `ChunkMutation`, `MutationOp`, `StorageTier`, `StorageMode`, `DeviceCapabilityProfile`, `ResourceSignals`, `TelemetrySnapshot`.
- `codec.rs`: `CWMC1` lossless columnar binary serializer/deserializer (sender table, ZigZag LEB128 varint delta seq & timestamp, bit-packed flags, chunk-local repeated string/emoji/ref table).
- `dict.rs`: Built-in versioned public Vietnamese/multilingual chat Zstd dictionary (`DICT_V1_ID = 1`, deterministic built-in trained bytes + BLAKE3 hash), `DictionaryRegistry`, encrypted private dictionary envelope (`CWDC1`), effectiveness gate.
- `zstd22.rs`: Unified Zstandard **Level 22** engine:
  - Native: `zstd::bulk::Compressor` / `Decompressor` reusable context pool, explicit `compression_level = 22`, adaptive bounded `window_log`, dictionary `CDict`/`DDict`, streaming bounded decompression bomb guard.
  - WASM (`pure-zstd`): `structured-zstd` Level 22 with dictionary & streaming + bounded decompression guard.
  - Cross-engine compatibility test suite verifying `native libzstd L22 <-> pure-Rust L22` round-trip!
- `crypto.rs`: Domain-separated HKDF-SHA-256 key derivation, CRIME/BREACH side-channel quantized padding (`CWPD1`), AES-256-GCM chunk sealing (`CWMC1_AEAD`) with complete AAD binding, verify-before-commit helper, zeroized key types.
- `incremental.rs`: 7-day eligibility filter (`MIN_COMPRESS_AGE_SECS = 604_800`), adaptive chunk target sizer (64KB..16MB), low-volume max wait, bounded per-tick batch planner, crash-safe `CompressionCheckpoint`, pause/resume/backoff, mutation overlay & bounded compaction planner.
- `budget.rs`: `DynamicStorageBudget` with tiered free-space table + hysteresis (`±12%` deadband), 4 `StorageMode` presets, bounded `L1ChunkCache` (zeroize on lock/logout), pluggable cache eviction benchmark implementations (LRU, LFU, TinyLFU, W-TinyLFU, Segmented LRU, Weighted Cost-Aware), `PrefetchPlanner`.
- `search.rs`: Unicode NFD/NFC + full Vietnamese diacritic folding + CJK bigram + emoji tokenizer, sharded positional inverted index, BM25 + recency + exact-diacritic ranking, full filter matrix, search result cache, cooperative cancellation, coverage state reporting, incremental upsert/locator-update/edit/delete/tombstone/low-storage-compact, and AES-256-GCM + Zstd-22 `CWSE1` encrypted shard serialization.
- `scheduler.rs`: 10-priority unified `ResourceScheduler` (Realtime > Calls > UI > ActiveRead > Search > Sync > Prefetch > Index > Compress > Maintenance), CPU/RAM/Battery/Thermal/Call throttling, privacy-safe `TelemetryCollector`.

## 2. Database & Backend Architecture (`web/backend/`)
- **Migration**: `web/backend/migrations/0040_message_chunks_and_encrypted_search.sql` creating:
  - `message_chunks`, `conversation_chunk_manifests`, `message_mutations`, `compression_leases`, `compression_dictionaries`, `encrypted_search_segments`, `device_storage_telemetry`
  - Alter `device_storage_policies` adding `storage_mode` & `compression_rollout_enabled`.
- **Backend Module**: `web/backend/src/storage/message_chunks.rs` (mounted at `/api/storage/msg-chunks`, `/api/storage/search-index`, `/api/storage/telemetry`):
  - Strict conversation RBAC + epoch access boundary checks.
  - Distributed lease acquire/renew/release with fencing tokens & crash-recovery checkpoint JSON.
  - Atomic OCC chunk commit (`WHERE manifest_version = $expected`, `409 Conflict` on race, upload-then-verify-then-commit, deferred safe GC).
  - Transparent Warm/Cold chunk serving (`local_cas` / `s3_ready` with S3 cold storage class & conditional PUT in `storage/s3.rs`), access tracking & Cold -> Warm promotion.
  - Mutation log append & delta read, wired into `messages/mod.rs` and `groups/mod.rs` edit/delete/pin/reaction routes.
  - Background incremental worker in `main.rs` (lease-protected across replicas) for >=7d plaintext group/channel compaction + Warm->Cold tiering + safe GC.
  - Extended `/api/storage/metrics` and `/api/storage/policy` (`storage_mode`).

## 3. Client & Multi-Platform Integration (`crates/chatw-media-worker/`, `app/`)
- **WASM Off-Main-Thread Worker (`crates/chatw-media-worker/src/lib.rs` + `app/web/media_worker.js`)**:
  - Export Zstd-22 chunk seal/open, encrypted Vietnamese search index upsert/mutate/query, and adaptive storage budget computation.
- **Web Storage Engine & PWA (`app/web/storage_engine.js`, `app/web/sw.js`, `app/public/manifest.json`, `app/web/index.html`)**:
  - IDB v6 non-destructive upgrade adding `msg_chunks`, `chunk_manifests`, `search_shards`, `zstd_dicts`, `compression_checkpoints`.
  - Encrypt local message chunks & search shards at rest; integrate L1 RAM LRU cache, incremental background compressor (with backpressure & pause during calls), incremental search indexer, scroll-aware prefetch, and adaptive hysteresis budget.
  - Add PWA `manifest.json` + App Shell caching in `sw.js` for true offline reloads.
- **Native Desktop & Mobile Parity (`app/src/storage/native.rs`, `app/src/storage/mod.rs`, `app/src/storage/sync_manager.rs`, `app/src/platform/mod.rs`)**:
  - Native encrypted Zstd-22 message chunk store + local encrypted search index + real drafts/dialogs/logout-wipe on Desktop/Mobile + platform-specific cache vs app-data directories (macOS/iOS `Caches`, Linux `XDG_CACHE_HOME`/`XDG_DATA_HOME`, Windows `LOCALAPPDATA`, Android app files/cache).
  - Fix `sync_manager.rs` to persist & index Group & Channel delta-synced messages alongside DMs.
  - Fix `messages.rs:565` scroll-up pagination when virtualized (`>120` messages) and add cross-chunk lazy `jump_to_message` in `chat.rs`.
- **UI (`app/src/components/storage_ui.rs`, `app/src/components/search_page.rs`, `app/src/components/details_panel.rs`, `app/src/pages/chat.rs`)**:
  - Add minimalist storage breakdown (total, messages+saved, media, files, cache, offline, search index) + 4 storage modes (`Tự động`, `Tiết kiệm dung lượng`, `Hiệu năng cao`, `Tùy chỉnh`) + safe clear cache in `StorageSettingsPanel`.
  - Wire instant encrypted local full-text message search (Vietnamese + filters + coverage badge + jump to message) into `SearchPage`, `DmDetailsPanel`, and `SelfDetailsPanel`.

## 4. Verification, Benchmarks & Security Testing
- Real benchmark binary/test in `crates/chatw-msg-store` measuring 1K, 10K, 100K, 1M messages across Vietnamese/English/Japanese/Chinese/emoji/short/long/metadata/group corpora, chunk sizes 64KB..16MB, eviction algorithms, and search P50/P95/P99 latency.
- Unit, property, fuzz-style malformed input, compression bomb, side-channel padding, replay/rollback/tamper, cross-account isolation, crash recovery, concurrent multi-device lease, and backend integration tests (`storage_sync_test.rs` + dedicated `msg_chunk_search_test.rs`).
- Full workspace checks: `cargo fmt --check`, `cargo clippy`, `cargo test` across all crates, `cargo check` for native targets (`x86_64-unknown-linux-gnu`, `wasm32-unknown-unknown`, and installed cross targets), `./app/build-web.sh release`, release backend build, daemon restart, and push to `origin main`.
