# Plan: Spec 072 — SharedWorker Resilience & Reconnect Fix

1. **`app/web/core_worker.js`**:
   - Fix `sendEncrypted` so `sendChain = task.catch(() => {})` never poisons, and reset `sendChain = Promise.resolve()` on every `handleHello`.
   - Track `everAuthenticated = true` upon first successful ECDH handshake so reconnects never permanently die during backend restarts; cap retry backoff at 5000ms.
   - Make `reconnect_now` accept optional `wsUrl` and recover even if `running` / `targetUrl` were unset.
   - Use `seq = 0` for `OP_PING` heartbeats so they never collide with `clientSeq`.
   - Add 64-bit sliding replay check on incoming encrypted server frames.
2. **`app/web/core_bridge.js` & `app/build-web.sh`**:
   - Bump `SharedWorker` name to `chatw-core-v2` (and version query `/core_worker.js?v=${TS}` in `build-web.sh`) so existing open browser tabs cleanly spin up the fixed `core_worker.js` instead of staying stuck on an old poisoned `chatw-core-v1` worker!
   - Pass `wsUrl: activeWsUrl || defaultWsUrl()` in `reconnectNow()`.
   - Expose `forceDropSocketForTest()` on `window.__chatwCore` for E2E verification of post-drop recovery.
3. **`web/backend/src/websocket/rpc_bin.rs`**:
   - Refactor `out_tx` to send `OutboundFrame::{Plain(Vec<u8>), Encrypted { op, req_id, payload, allow_compress }}` to the single `writer_task`, which strictly assigns monotonic `server_seq` and encrypts in wire order.
   - Replace strict `h.seq <= last_client_seq` with a 64-bit RFC 6479 sliding replay window (`ReplayWindow`) + unit tests.
4. **`app/src/platform/web.rs`**:
   - If an online `core.rpcFromRust` returns `ok: false` / `status: 0` (while `!isSimulatedOffline() && navigator.onLine !== false`), automatically trigger `core.reconnectNow()` + `await core.waitUntilReady(2000)` and retry the RPC once over Binary WS (and if still unavailable while online, fall back to authenticated `fetch()` so a live user is NEVER stuck unable to send messages!).
5. **`app/src/services/dm.rs` & `app/src/pages/chat.rs`**:
   - Return structured offline code `"network_error"` only when `ApiError.status == 0`; return actual server/crypto error code otherwise.
   - Only show `"Offline: message queued..."` on genuine network/offline errors; surface actual error text on hard rejections.
6. **Verification**:
   - Run Rust unit tests (`chatw` + `chatw-app`), rebuild backend & release WASM, restart `:8085`.
   - Add a dedicated Playwright test reproducing (a) forced WS drops + rapid post-drop DM send, (b) 15 concurrent RPCs + messages, (c) existing session upgrade without `chatw_auth_hint`, and (d) self-chat + image message send.
   - Re-run `spec071` and `spec069` E2E suites + commit & push to `origin main`.
