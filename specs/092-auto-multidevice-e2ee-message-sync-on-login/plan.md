# Spec 092 — Technical Plan

## Backend Changes (`web/backend/`)
1. `web/backend/migrations/0033_auto_device_sync.sql`
   - `ALTER TABLE device_storage_policies ALTER COLUMN require_device_approval SET DEFAULT FALSE;`
   - `UPDATE device_storage_policies SET require_device_approval = FALSE;`
2. `web/backend/src/devices_sync/mod.rs`
   - `handle_register_device`:
     - Default `require_approval` fallback `.unwrap_or(false)`.
     - Detect `needs_key_sync = ecdsa.is_none() || fp.is_none()` and `has_account_keys` (`SELECT EXISTS(SELECT 1 FROM device_keys WHERE user_id = $1)`).
     - Keep `initial_trust = "pending"` when `other_trusted_count > 0 && (require_approval || needs_key_sync)` so untrusted devices without keys cannot overwrite identity keys before receiving `wrapped_sync_key`.
     - Whenever `trust_state == "pending" || (needs_key_sync && has_account_keys && other_trusted_count > 0 && req.ecdh_public_b64.is_some())`:
       - Reuse an unexpired `pending` request (or create a fresh one; if `ecdh` changed, expire stale requests first so the new transport key is always used).
       - Always broadcast `WsEvent::DeviceApprovalRequest` with `"auto_approve": !require_approval` so online trusted devices immediately auto-sync.
   - `handle_list_devices`:
     - Return `"require_device_approval"` from `device_storage_policies` (default `false`).
     - Filter out `r.requesting_device_id <> caller_did` from `pending_requests`.
   - `handle_delta_sync`:
     - Add `AND deleted_for_all = FALSE` on `messages` query.
     - Return `"has_more"` boolean alongside `next_cursors`.
3. `web/backend/src/backup/engine.rs` & `web/backend/src/storage/router.rs`:
   - Align `require_device_approval` default fallback to `false`.

## Client Changes (`app/`)
1. `app/src/services/storage_sync.rs`
   - Add `#[serde(default)] pub require_device_approval: bool` on `DevicesListResponse`.
   - Support configurable `limit` (`delta_sync_with_limit`, default `250`).
2. `app/src/storage/sync_manager.rs`
   - Add `DeltaSyncOutcome` struct (`total_decrypted`, `updated_previews: HashMap<String, String>`, `touched_partners: HashSet<String>`).
   - Add pure `merge_healed_dm_messages(existing: Vec<DisplayMessage>, incoming: Vec<DisplayMessage>) -> (Vec<DisplayMessage>, bool)` with unit tests verifying stale `MessageStatus::Error` (`"Cannot read on this device"`) entries are replaced in-place by newly decrypted `MessageStatus::Sent` messages.
   - Implement `auto_fulfill_pending_device_syncs(user_id: &str, my_keys: &UserKeys) -> usize`: lists pending requests and automatically wraps + approves when `!require_device_approval`.
   - Upgrade `run_incremental_delta_sync_for_user(user_id: &str, keys: Option<&UserKeys>) -> Result<DeltaSyncOutcome, String>`:
     - Skips advancing `dm_since` when `keys.is_none()`.
     - Loops paginated `/api/sync/delta` (`limit=250`, up to 20 pages) when `keys.is_some()`.
     - Fetches/verifies peer keys once per unique partner, decrypts every DM/Self envelope locally, merges & heals IndexedDB `dm:{uid}:{partner_id}`, and collects latest decrypted preview text per partner.
   - Upgrade `bootstrap_device_and_sync_for_user` to:
     - Unwrap any waiting `wrapped_sync_key` (clearing `E2EE_RECOVERY_REQUIRED`).
     - If trusted + has keys: call `auto_fulfill_pending_device_syncs` AND `run_incremental_delta_sync_for_user`.
3. `app/src/pages/chat.rs`
   - Add unified `apply_synced_keys_and_refresh` callback called on boot key sync, `WsEvent::DeviceApprovalResolved`, `E2eeUnlockRecoveryModal::on_unlocked`, and Settings key import:
     - Updates `user_keys`, clears `dm_cache` & `peer_keys_cache`, resets sync cursor `(0, 0, 0)`, runs `run_incremental_delta_sync_for_user`, merges `updated_previews` into `last_previews` + persists, and re-opens `active_friend` immediately so open chats re-decrypt live.
   - Update `WsEvent::DeviceApprovalRequest`: if `auto_approve` is true and trusted device has `UserKeys` + `ecdh_pub`, auto-wrap & approve immediately in background; otherwise show `IncomingDeviceApprovalModal`.
   - Update `select_friend` merge loop to use `merge_healed_dm_messages` so stale `"Cannot read on this device"` cache entries are always replaced by newly decrypted messages, and avoid overwriting good previews with `"Cannot read on this device"`.
   - When `bootstrap_device_and_sync_for_user` finishes on an already-keyed device (same browser login), also apply `updated_previews` to `last_previews` and heal `active_friend` if open.
4. `app/src/components/storage_ui.rs`
   - Default `StoragePolicy::require_device_approval` to `false`.
   - Add 2.5s silent auto-sync grace phase (fast 400ms polling + live WS) in `E2eeUnlockRecoveryModal` for `"missing_local_keys"` so zero modal interrupts the user when another device auto-syncs on login.

## Testing & Deployment
1. `cargo test -p chatw-app --lib` + `cargo test -p chatw --lib`
2. Rebuild backend (`cargo build -p chatw`), restart live daemon on `127.0.0.1:8085` (auto-running migration `0033`).
3. Rebuild release WASM (`./app/build-web.sh release`).
4. Run new two-device Playwright E2E `app/e2e/spec092-auto-multidevice-message-sync.mjs` + regression E2E (`spec086`, `spec089`).
5. Visually inspect screenshots via `read_file`, mark tasks `[x]`, and commit + push to `origin main`.
