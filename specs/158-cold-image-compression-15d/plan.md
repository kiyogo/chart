# Technical Plan: Spec 158

1. **`crates/chatw-media-core/src/cache.rs`**:
   - Add `COLD_IMAGE_COMPRESSION_IDLE_DAYS`, `COLD_IMAGE_COMPRESSION_IDLE_SECS`, `ColdImageDecision`, and `evaluate_cold_image_policy(...)` with unit tests verifying active (`0..14.99d`) vs idle (`>= 15d`) vs access reset.
2. **Backend Migration & Storage (`web/backend/`)**:
   - `migrations/0036_cold_image_compression_15d.sql`: add `last_accessed_at`, `cold_compressed_at`, and partial idle index on `media_objects` & `media_references`.
   - `src/storage/router.rs`: touch `last_accessed_at = now()` on non-thumb image reads (throttled `1h` window to prevent write amplification); defer non-E2EE day-0 `optimized` AVIF generation so active images stay original until cold.
   - `src/storage/migration.rs` & `src/main.rs`: add `compress_inactive_images_15d(pool, storage, idle_days)` compressing inactive `>= 15 days` non-E2EE images to `avif_cold_15d` and schedule hourly.
3. **Client Upload & Idle Cache Compactor (`app/web/`)**:
   - `media_optimizer.js` & `media_worker.js`: default to `original_passthrough` unless `qualityMode === 'data_saver'`, `opts.forceCompress === true`, or `> 24 MB`.
   - `storage_engine.js`: store `created_at`, `last_accessed_at`, `cold_compressed_15d` and expose `compressColdImages15d(idleDays = 15, dryRun = false)` scheduled automatically on idle after `init()`.
4. Verify with Rust unit tests, backend integration test (`storage_sync_test` + 15-day cold compression test against real Postgres), and Chrome E2E (`compressColdImages15d` keeping `< 15d` intact & compressing `>= 15d`), commit & push.
