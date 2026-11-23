# Spec 074: Outbox Hard-Error Purge, Loopback Online Resilience, `SharedWorker` Fallback & Accurate Key-Mismatch Messaging

## Problem Statement
1. Stale 4xx DM failures (such as `sender_fingerprint_mismatch` on a pending secondary device like `123123`'s `Linux · Chrome`) stayed stuck in IndexedDB `chat_outbox` forever because `flush_outbox` only purged 4 error strings and broke the loop on everything else.
2. `navigator.onLine === false` on Linux desktops short-circuited `http_request` / `core.rpc` even when the browser had an active WebSocket or was connected to local `127.0.0.1` / `localhost`, falsely emitting `"Offline: message queued and will send automatically when reconnected"`.
3. `sender_fingerprint_mismatch` was grouped under `"Recipient encryption key changed"` instead of clearly explaining that *this device's* E2EE key does not match the account's registered key.
4. `SharedWorker` lacked an `onerror` fallback to Dedicated `Worker`, and `port.start()` ran before `port.onmessage` was attached.

## Acceptance Criteria
- AC1: `flush_outbox` purges any non-transient 4xx/crypto error (`!is_transient_offline`) from `chat_outbox` and clears stale pending bubbles instead of retrying infinitely.
- AC2: `navigator.onLine === false` never blocks an active SharedWorker WS or loopback (`127.0.0.1` / `localhost`) request; real `simulatedOffline` and real network disconnection still queue optimistically and flush cleanly.
- AC3: `sender_fingerprint_mismatch` displays a clear, accurate device-key message and removes the failed outbox item (never claims "Offline").
- AC4: Backend logs non-2xx `/api/messages` RPC rejections via `tracing::warn!`, and frontend logs send errors via `console.warn`.
