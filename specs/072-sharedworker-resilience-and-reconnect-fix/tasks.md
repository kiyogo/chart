# Tasks: Spec 072 — SharedWorker Resilience & Reconnect Fix

- [x] T001 Fix `sendChain` poisoning, `everAuthenticated` reconnect resilience, `reconnect_now`, and `OP_PING` seq in `app/web/core_worker.js`
- [x] T002 Bump SharedWorker instance name (`chatw-core-v2`), pass `wsUrl` in `reconnectNow()`, add `forceDropSocketForTest()` in `app/web/core_bridge.js`, and version `/core_worker.js?v=${TS}` in `app/build-web.sh`
- [x] T003 Move server `out_seq` encryption into single `writer_task` (`OutboundFrame`) and add 64-bit `ReplayWindow` + unit tests in `web/backend/src/websocket/rpc_bin.rs`
- [x] T004 Add online single reconnect+retry (and safe online fallback) in `app/src/platform/web.rs` and accurate offline vs server error classification in `app/src/services/dm.rs` & `app/src/pages/chat.rs`
- [x] T005 Rebuild backend + release WASM, restart `:8085`, verify with Playwright (`spec072` 5/5 + `spec071` 21/21 + `spec069` 18/18), visually inspect screenshot, commit & push to `origin main`
