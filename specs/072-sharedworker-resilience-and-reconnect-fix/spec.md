# Spec 072: SharedWorker `sendChain` Resilience, Strictly-Ordered Server `seq`, Auto-Reconnect & Honest Error Classification

## Problem Statement
When a user sends a DM on an existing/reconnected session (for example after a backend restart or transient WebSocket drop), the UI persistently displays `"Offline: message queued and will send automatically when reconnected"` even though the browser and server are online.

## Root Causes Identified
1. **Permanent `sendChain` Promise Poisoning (`app/web/core_worker.js`)**: `sendChain = sendChain.then(...).catch((err) => { throw err; })` re-throws inside `.catch()`, permanently leaving `sendChain` in the rejected state after a single transient error so every subsequent RPC fails forever with `ws_not_ready`.
2. **Reconnect Starvation (`app/web/core_worker.js`)**: `authFailedCloseCount >= 4` permanently halts reconnection even for previously authenticated sessions (`everAuthenticated`), and `reconnect_now` returns early if `targetUrl` wasn't initialized yet.
3. **Server-Side Out-of-Order `out_seq` (`web/backend/src/websocket/rpc_bin.rs`)**: Concurrent `tokio::spawn` tasks incremented `out_seq` before encryption/queuing; moving sequence assignment and encryption into the single `writer_task` guarantees strict wire order.
4. **No Single Auto-Retry on Transient Online RPC Failure (`app/src/platform/web.rs`)**: `try_core_worker_rpc` immediately returned `status: 0` when an online RPC failed during a momentary WS reconnect instead of reconnecting and retrying once.
5. **Misclassified DM Errors (`app/src/services/dm.rs` & `app/src/pages/chat.rs`)**: Non-network 4xx/5xx or TOFU/crypto errors fell into the offline branch and displayed the "Offline: message queued..." toast.
6. **Cache-Busting `core_worker.js` (`app/web/core_bridge.js` & `app/build-web.sh`)**: Ensure upgraded clients always load the fixed `core_worker.js` script cleanly instead of attaching to a stale SharedWorker name from an earlier build.

## Acceptance Criteria
- AC1: After a transient WS drop / forced socket close / backend restart, `sendChain` never poisons and subsequent messages send immediately with `MessageStatus::Sent` (zero "Offline" false positives).
- AC2: Previously authenticated SharedWorker sessions always recover automatically even after >4 failed reconnect attempts while backend restarts.
- AC3: Concurrent RPCs + server events maintain strictly monotonic `seq` in both directions with a 64-bit sliding anti-replay window.
- AC4: Existing logged-in users upgrading without `chatw_auth_hint` in `localStorage` seamlessly connect SharedWorker and send messages with 0 errors.
- AC5: Real offline sends (`simulatedOffline` or `navigator.onLine === false`) still queue optimistically and flush cleanly with 0 duplicates; real server/crypto rejections display an accurate error instead of pretending to be offline.
