# Spec 077 — Tasks

- [x] **T1**: Add `deleteStaticCacheByPrefix(prefix, keepKey)` to `app/web/storage_engine.js` so stale `runtime:core:*` IndexedDB entries are pruned automatically on new builds.
- [x] **T2**: Create stateless `SharedWorker` runtime fetcher `app/web/runtime_worker.js` that fetches `/pkg/core.bin?v=...` off-main-thread, deduplicates concurrent tab requests, transfers the `ArrayBuffer` zero-copy, and terminates cleanly via `self.close()` when idle.
- [x] **T3**: Update `app/web/boot.js` to load the `CWRT1` binary from IndexedDB `static_cache` (warm boot = 0 network, 0 worker), `SharedWorker` `runtime_worker.js` on authenticated cold boot (0 page Network entries even for `core.bin`), or `networkidle`-safe `application/octet-stream` `/pkg/core.bin` on initial cold boot (0 `.wasm` / `Type: wasm` entries), unmask + decompress via `DecompressionStream('gzip')`, cache in IndexedDB, and instantiate via `init({ module_or_path: wasmBuf })`.
- [x] **T4**: Update `app/web/sw.js` to remove `.wasm` from `isCacheableStaticAsset`, bypass `/pkg/core.bin` completely, and bump `STATIC_CACHE_NAME` to `chatw-static-v5`.
- [x] **T5**: Update `app/build-web.sh` to pack `chatw_app_bg.wasm` into `CWRT1` masked gzip `app/dist/pkg/core.bin` (5.16× smaller), remove `.wasm` artifacts from `app/dist/pkg/`, patch `chatw_app.js` to remove `.wasm` default URL, and inject build version token into `boot.js`.
- [x] **T6**: Build release bundle (`./app/build-web.sh release`), write & run `app/e2e/spec077-stealth-wasm-hidden-network.mjs` (9/9 PASS) + regression suites (`spec072` 5/5, `spec073` 7/7, `spec076` 10/10), and visually verify live app rendering.
- [x] **T7**: Mark all tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
