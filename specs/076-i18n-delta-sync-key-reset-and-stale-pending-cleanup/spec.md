# Spec 076 — Complete 197-Language i18n Delta Sync, Error-Bubble Translation & Contrast, Stale Pending Cleanup, and 1-Click Security Key Reset

## Problem Statement
1. **Untranslated UI & System Error Bubbles (Wrong Language)**:
   - Newly added Spec 069–075 strings (offline outbox toast, mismatched encryption key toast/bubble, session expired, recipient key errors, `Unable to send ({0})`, lightbox `Download original`, voice seek aria-labels, scroll-to-bottom labels) were missing from `app/web/locales/_keys.json` and the 197 world language packs (`app/web/locales/*.json`).
   - `app/web/i18n_runtime.js` included `.message-text` in `PROTECTED_SELECTOR` and unconditionally skipped all `.message-bubble` elements outside `.settings-preview-chat`, which blocked system-generated `.message-bubble.error-bubble` rows (such as `"Cannot read on this device"` and `"This device's encryption key does not match your account..."`) from ever being translated!
   - Stale IndexedDB (`i18n:*:v2`) and Service Worker (`chatw-static-v3`) caches served older locale packs without the new keys.
2. **Untrusted / Mismatched Device Encryption Key Dead-End (`sender_fingerprint_mismatch`)**:
   - When an existing account signs in on a secondary browser (`trust_state = pending`), its local E2EE key does not match the registered server `device_keys` fingerprint, causing `POST /api/messages` to reject with `400 sender_fingerprint_mismatch`.
   - **Settings → Privacy & security → Security** displayed a misleading static `"Active & secure"` badge (checking only `user_keys.is_some()` without verifying server fingerprint parity or device trust state) and offered NO button to sync or reset the device encryption key.
   - `E2eeUnlockRecoveryModal`'s `"Confirm creation of new key"` called `POST /api/keys` directly without first promoting the pending device via `POST /api/devices/recover-lost` (`confirm_emergency_reset: true`), causing `403 cannot_overwrite_identity_key_from_untrusted_device`.
3. **Orphaned Ghost Pending Bubbles (`local-*`) in IndexedDB `chat_messages`**:
   - Optimistic `local-*` bubbles were written into IndexedDB `chat_messages` before send, then removed from `chat_outbox` on hard 4xx errors without pruning/resaving `chat_messages`, causing old rejected messages to keep reappearing as blue pending clock bubbles on reload.
4. **Error Bubble Contrast & Stray Clock Icon**:
   - Error bubbles with `local-*` IDs inherited `is_pending = true`, rendering a spinning/static clock icon inside red error bubbles.
   - `.message-row.sent .bubble-meta` rendered faint white timestamps on light pink `--bg-error` error bubbles.

## Acceptance Criteria
1. **100% i18n Coverage Across All 197 Languages + System Error Bubble Translation**:
   - `app/web/i18n_runtime.js` translates system `.message-bubble.error-bubble` text while still strictly protecting all user-authored messages (`.message-bubble:not(.error-bubble) .message-text`).
   - All Spec 069–076 strings and templates are added to `app/web/locales/_keys.json` and all 197 `app/web/locales/*.json` files with curated native Vietnamese in `vi.json`.
   - `i18n_runtime.js` cache keys bump to `:v3` and `app/web/sw.js` bumps `STATIC_CACHE_NAME` to `chatw-static-v4`.
2. **Live Key Status & 1-Click Sync / Reset in `Settings → Privacy & security → Security`**:
   - `DeviceEncryptionKeyCard` checks live `api::keys_get` fingerprint parity + device trust state.
   - When mismatched or pending, displays red `"Key mismatch"` badge, explanation, `"Sync from trusted device"`, and two-step `"Reset device key"` (calling `recover_lost_devices_with_proof(..., confirm_emergency_reset=true)` when pending, generating fresh local keys, registering via `keys_register_rotated`, updating `DEVICE_TRUST_STATE` to `"trusted"`, and clearing caches so sending works immediately).
   - `E2eeUnlockRecoveryModal` shares the same `reset_device_encryption_key` flow.
3. **Zero Orphaned Pending Bubbles**:
   - Hydrating `chat_messages` from IndexedDB (`select_friend`, `select_group`, `select_channel`), merging server history, and purging hard-rejected items in `send_dm_text` / `flush_outbox` automatically remove any `local-*` message not backed by `chat_outbox` or `in_flight_outbox` and persist the cleaned cache.
4. **Clean Error Bubble Visuals**:
   - Error bubbles never carry `is_pending` or render `ClockIcon`.
   - `.message-bubble.error-bubble .bubble-meta` uses crisp, high-contrast muted danger ink readable on both light and dark themes.
