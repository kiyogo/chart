# Spec 092 — Automatic Multi-Device E2EE Key & Full Message History Sync on Login

## Problem Statement
When a user logs into an existing account on another browser/device (or reloads after key sync), messages frequently show **"Không đọc được trên máy này"** (`Cannot read on this device`, `media_1795435702604_c71721ba.jpg`) due to five compounding defects:
1. `run_incremental_delta_sync()` in `app/src/storage/sync_manager.rs` fetched `dm_messages` from `/api/sync/delta`, counted `.len()`, and **discarded every message** without decrypting, saving to IndexedDB, or updating conversation sidebar previews — and even advanced the `dm_since` cursor before E2EE keys arrived.
2. `select_friend` in `app/src/pages/chat.rs` merged fresh decrypted messages using `if !existing.contains(&m.id)`, permanently trapping stale `MessageStatus::Error` (`"Cannot read on this device"`) bubbles in IndexedDB even after the correct E2EE keys became available.
3. All key-arrival paths (`bootstrap_device_and_sync_for_user`, `WsEvent::DeviceApprovalResolved`, `E2eeUnlockRecoveryModal::on_unlocked`) set `user_keys` without clearing `dm_cache`, resetting sync cursors, syncing all historical DMs, refreshing sidebar `last_previews`, or re-decrypting the active open conversation.
4. Trusted online devices required manual 6-digit approval clicks (`require_device_approval` defaulted to `TRUE`), never polled `pending_requests` on boot/reconnect, and backend `handle_register_device` skipped creating/re-broadcasting key-transfer requests when `require_device_approval == false` or when a device reloaded.
5. `E2eeUnlockRecoveryModal` popped up immediately at millisecond 0 instead of allowing a short silent auto-sync grace window while an online trusted device transfers the E2EE-wrapped key (~200–500ms).

## Open-Source & Security Architecture (Zero-Knowledge Blind Relay)
Modeled after Matrix Cross-Signing/Device Key Sharing, Signal Linked Devices, and WhatsApp Multi-Device History Sync while strictly preserving ChatW's Blind-Relay E2EE guarantee:
- The backend **never** stores or inspects plaintext private keys or plaintext messages.
- A newly logged-in device generates a local ephemeral P-256 ECDH transport keypair and registers its public key with the server.
- An already-authenticated trusted device holding `UserKeys` **automatically** encrypts `UserKeys` directly to the new device's ephemeral P-256 public key using `wrap_sync_key_for_device` (ECDH P-256 + HKDF-SHA-256 + AES-256-GCM + AAD + ECDSA P-256 signature) and relays the opaque `sync:v1:...` ciphertext via the server (unless the user explicitly opts into manual 6-digit confirmation in Settings → Storage → Devices).
- The new device verifies the ECDSA signature against the account's pinned identity key, unwraps `UserKeys`, and immediately runs paginated full DM history delta sync (`/api/sync/delta`) to decrypt and populate all conversations, IndexedDB caches, and sidebar previews.

## Acceptance Criteria
1. **Backend Auto-Sync Signaling & Paginated Delta (`web/backend/`)**:
   - Migration `0033_auto_device_sync.sql` sets `require_device_approval` default to `FALSE` (enabling zero-click auto-sync by default while preserving manual opt-in).
   - `handle_register_device` always creates or refreshes & re-broadcasts a `device_login_requests` key-sync request whenever a device registers without full E2EE identity keys (`ecdsa.is_none() || fp.is_none()`) on an account that already has server identity keys, including `auto_approve: !require_approval` in the WS event.
   - `handle_list_devices` includes `require_device_approval` and returns pending key-sync requests to trusted devices (excluding self).
   - `handle_delta_sync` excludes `deleted_for_all = TRUE` messages and returns `has_more` so clients can paginate through complete history.
2. **Automatic Trusted-Device Key Fulfillment (`app/src/storage/sync_manager.rs`, `app/src/pages/chat.rs`)**:
   - Trusted devices with valid `UserKeys` automatically fulfill pending key-sync requests on boot/WS connect AND live on `WsEvent::DeviceApprovalRequest` when `auto_approve` is enabled (wrapping `UserKeys` client-side via `wrap_sync_key_for_device`), falling back to `IncomingDeviceApprovalModal` when manual approval is enabled.
3. **Real Full History Decryption & Error-Healing (`app/src/storage/sync_manager.rs`, `app/src/pages/chat.rs`)**:
   - `run_incremental_delta_sync_for_user` only advances `dm_since` when `UserKeys` are present, paginates `/api/sync/delta`, decrypts all DM & Self-Chat envelopes locally, replaces stale `MessageStatus::Error` entries in IndexedDB, and returns updated sidebar previews (`partner_id -> text`).
   - `select_friend` replaces cached `MessageStatus::Error` entries in-place when fresh decrypted `MessageStatus::Sent` messages arrive.
   - Unified `apply_synced_keys_and_refresh` reloads `dm_cache`, `last_previews`, and the open conversation immediately whenever keys arrive via boot, WS, or modal unlock.
4. **Silent Auto-Sync Grace Window (`app/src/components/storage_ui.rs`)**:
   - `E2eeUnlockRecoveryModal` polls fast during an initial 2.5s non-blocking grace phase (showing a sleek sync indicator instead of flashing a blocking dialog when an online device auto-syncs within <500ms), then shows the full recovery dialog only if all other devices are offline while continuing to listen in the background.
5. **End-to-End Verification & Auto-Push**:
   - Rust unit tests for both `chatw-app` and `chatw` backend pass (including error-healing merge & delta sync tests).
   - Real multi-context Playwright E2E (`app/e2e/spec092-auto-multidevice-message-sync.mjs`) proves zero-click auto-sync across two isolated browser contexts (live online AND offline-then-online), with 0 `"Không đọc được trên máy này"` errors, plus live screenshots.
   - Rebuild backend & release frontend WASM, restart live server, mark tasks `[x]`, and commit + push to `origin main`.
