# Tasks: Spec 158

- [x] Create migration `web/backend/migrations/0036_cold_image_compression_15d.sql`
- [x] Add `ColdImageDecision` & `evaluate_cold_image_policy` + tests in `crates/chatw-media-core/src/cache.rs`
- [x] Implement server access-tracking (`last_accessed_at`), defer day-0 compression, and add hourly `compress_inactive_images_15d` in `web/backend/src/`
- [x] Keep active uploads as-is (`< 15d`) in `media_optimizer.js` / `media_worker.js` and add `compressColdImages15d` in `storage_engine.js`
- [x] Verify via unit tests, backend Postgres compression test, and Chrome E2E, then commit & push
