# Plan & Tasks: Spec 074

- [x] T001 Fix `core_bridge.js` (`isEffectivelyOffline` loopback/connected check, attach `port.onmessage` before `port.start()`, `sw.onerror` fallback, `console.warn` on RPC failure)
- [x] T002 Fix `app/src/platform/web.rs` (`isEffectivelyOffline` check so live WS / loopback is never falsely blocked by `navigator.onLine === false`)
- [x] T003 Fix `app/src/pages/chat.rs` (`flush_outbox` purge of non-transient errors + remove stuck pending bubbles; separate `sender_fingerprint_mismatch` message + `console.warn` diagnostic)
- [x] T004 Add `tracing::warn!` for non-2xx `/api/messages` RPCs in `web/backend/src/websocket/rpc_bin.rs`
- [x] T005 Rebuild backend + release WASM, restart `:8085` (confirmed live `sender_fingerprint_mismatch` on `123123`), verify with `spec072` (5/5) + `spec073` (6/6) + `spec071` (21/21) + `spec069` (18/18), commit & push to `origin main`
