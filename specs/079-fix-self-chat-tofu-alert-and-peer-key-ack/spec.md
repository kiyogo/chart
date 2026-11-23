# Spec 079 — Eliminate False Self-Chat TOFU Key-Change Alert & Make Peer Key Verification Reliable

## Problem Statement
After resetting or rotating their own device encryption key, opening **"Note to self" ("Chính mình")** repeatedly popped up:
`"Security alert: this person's encryption key has changed. Check the security code in Information."` (`"Cảnh báo bảo mật: khóa mã hóa của người này đã thay đổi. Kiểm tra mã an toàn trong Thông tin."`)

Root causes:
1. `get_or_verify_peer_keys_scoped(Some(&uid), &friend.id, ...)` treated self-chat (`peer_id == my_user_id`) identically to a third-party peer, recording a permanent `chatw_e2ee_peer_alert_{uid}_{uid}` in `localStorage` whenever the user rotated their own key.
2. Self-chat ("Note to self") has no right-hand `DmDetailsPanel` ("Information") drawer, leaving the user with no way to dismiss the permanent self-alert.
3. For real friends, `DmDetailsPanel` looked up `my_uid` via `api::auth_me_cached_id()` (instead of receiving `current_user_id` directly), kept the Security section collapsed even when an active TOFU alert existed, and did not clear legacy unscoped alert keys or refresh `peer_keys_cache` upon verification.

## Acceptance Criteria
1. **Zero False Alerts on Self-Chat (`peer_id == my_user_id`)**:
   - `get_or_verify_peer_keys_scoped` never emits `key_changed_warning` when `my_user_id == Some(peer_id)`, proactively purges any pre-existing `chatw_e2ee_peer_alert_{uid}_{uid}` and legacy `chatw_e2ee_peer_alert_{uid}`, and stores the self entry as `verified: true`.
   - `reset_device_encryption_key` immediately updates & clears the self peer slot.
   - `select_friend` additionally guards `friend.id != uid` before firing any peer-key-change toast.
2. **Reliable Peer Key Verification for Real Friends**:
   - `acknowledge_peer_key_change` clears both scoped (`chatw_e2ee_peer_alert_{uid}_{peer_id}`) and legacy (`chatw_e2ee_peer_alert_{peer_id}`) slots.
   - `DmDetailsPanel` receives `my_user_id` explicitly, auto-expands the Security section when a peer TOFU alert is active, and notifies `ChatPage` (`on_key_verified`) to clear `peer_keys_cache` for that peer so the alert never re-appears.
3. **Verified End-to-End**:
   - New Rust unit test in `app/src/crypto/e2ee.rs` verifies self-chat rotation produces zero alert and cleans pre-existing self alerts while third-party peer substitution still triggers `key_changed_warning`.
   - Live Playwright test (`app/e2e/spec079-no-self-chat-tofu-alert.mjs`) resets device key, opens "Chính mình" repeatedly across reload, and confirms ZERO `"Cảnh báo bảo mật: khóa mã hóa của người này đã thay đổi"` toasts ever appear.
