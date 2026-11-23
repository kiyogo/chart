# Spec 077 — Stealth WASM Runtime Delivery (`CWRT1` Packed Binary + `SharedWorker` Loader + IndexedDB Cold/Warm Cache)

## Problem Statement
Opening Chrome DevTools → Network (and specifically the **Wasm** filter) exposed two large raw WASM requests (`chatw_app_bg.wasm?v=...` from `chatw_app.js` and `⚙ chatw_app_bg.wasm?v=...` from `sw.js`, 18.2 MB uncompressed).

## Architecture (3-Layer Stealth & Performance Pipeline)
1. **Build-Time `CWRT1` Compressed Binary Packaging (`app/build-web.sh`)**:
   - Compresses `chatw_app_bg.wasm` (18.2 MB) using `gzip -9` (~3.55 MB — **5.1× smaller**) and wraps it in a `CWRT1` binary envelope (`8-byte magic + 4-byte build XOR mask + masked gzip payload`) written to `app/dist/pkg/core.bin`.
   - Deletes `app/dist/pkg/chatw_app_bg.wasm` and `chatw_app_bg.wasm.d.ts` from `app/dist/pkg/` so no raw `.wasm` asset is served.
   - Replaces the default `.wasm` URL fallback in `app/dist/pkg/chatw_app.js` so `chatw_app.js` never fetches a `.wasm` path.
2. **Cold-Boot `SharedWorker` Runtime Loader (`app/web/runtime_worker.js`)**:
   - `app/web/boot.js` checks IndexedDB `static_cache` (`runtime:core:${VER}`) first.
   - On cache miss, requests the packed `CWRT1` binary from a stateless `SharedWorker` (`/runtime_worker.js?v=${VER}`), which fetches `/pkg/core.bin?v=${VER}` inside the `SharedWorker` target (**100% invisible** in the page's Chrome DevTools Network tab — `0` entries in `Wasm`, `Fetch/XHR`, and `All`), transfers the `ArrayBuffer` zero-copy back to `boot.js`, and releases worker RAM after idle timeout.
   - Graceful fallback for browsers without `SharedWorker` (e.g. Safari): fetches `/pkg/core.bin?v=${VER}` (`application/octet-stream`, zero `.wasm` / `application/wasm` classification), unpacks in memory, and caches in IndexedDB.
3. **Warm-Boot 0-Network IndexedDB Cache + SW Exclusion**:
   - `boot.js` persists the 3.55 MB `CWRT1` payload into IndexedDB `static_cache` (`runtime:core:${VER}`) and prunes stale `runtime:core:*` versions via `window.__chatwStorage.deleteStaticCacheByPrefix`.
   - On subsequent page loads / reloads, `boot.js` reads directly from IndexedDB without spawning `runtime_worker.js` or making any network request, unpacks via native `DecompressionStream('gzip')`, and boots via `init({ module_or_path: wasmBuffer })` (`WebAssembly.instantiate`).
   - `app/web/sw.js` removes `.wasm` from `isCacheableStaticAsset`, explicitly bypasses `/pkg/core.bin` (preventing any `⚙` Service Worker fetch entry), and bumps `STATIC_CACHE_NAME` to `chatw-static-v5`.

## Acceptance Criteria
1. **ZERO `*.wasm` requests and ZERO `Type: wasm` entries** in Chrome DevTools Network tab on **both** Cold Boot (empty cache) and Warm Reload!
2. **ZERO `/pkg/core.bin` requests** visible in the page's Chrome DevTools Network tab on Chrome (`SharedWorker` enabled) on both Cold Boot and Warm Reload!
3. **ZERO `⚙` Service Worker fetch rows** for the runtime binary.
4. Direct `GET /pkg/chatw_app_bg.wasm` returns `404 Not Found`.
5. `app/dist/pkg/core.bin` is ≤ 5 MB (~3.55 MB) and starts with `CWRT1` magic (zero `\0asm` or `1f 8b` magic bytes on wire).
6. Full application boots identically, passes Spec 077 E2E verification + all existing regression suites (`spec062`, `spec072`, `spec073`, `spec076`).
