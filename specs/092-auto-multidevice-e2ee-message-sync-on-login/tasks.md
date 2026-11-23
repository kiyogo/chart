# Spec 092 — Tasks

- [x] **T1**: Add migration `web/backend/migrations/0033_auto_device_sync.sql` and upgrade `web/backend/src/devices_sync/mod.rs` (+ fallback defaults in `web/backend/src/backup/engine.rs`) for automatic key-sync request creation/re-broadcast, `auto_approve` WS signaling, `deleted_for_all` filtering, and paginated `has_more` delta sync.
- [x] **T2**: Upgrade `app/src/services/storage_sync.rs` and `app/src/storage/sync_manager.rs` with `auto_fulfill_pending_device_syncs`, `merge_healed_dm_messages` (+ unit tests), and real paginated DM decryption & IndexedDB/preview hydration in `run_incremental_delta_sync_for_user`.
- [x] **T3**: Wire automatic WS key fulfillment, unified `apply_synced_keys_and_refresh`, and IndexedDB error-healing in `app/src/pages/chat.rs`, plus the 2.5s silent auto-sync grace window in `app/src/components/storage_ui.rs`.
- [x] **T4**: Run Rust unit tests (`chatw-app` + `chatw`), rebuild & restart live backend on `127.0.0.1:8085`, rebuild release WASM (`./app/build-web.sh release`), write & run two-context Playwright E2E `app/e2e/spec092-auto-multidevice-message-sync.mjs` + regression E2E, and inspect live screenshots via `read_file`.
- [x] **T5**: Mark all tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
