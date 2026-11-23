# Plan: Spec 073 — Zero Double-Load, 100% Deduplicated `blob:` & Zero `data:image/*`

1. **`app/web/core_bridge.js`**:
   - Add `getCachedBlobUrlSync(url, keyB64, nonceB64)` and `registerCachedBlobUrl(cacheKey, bytes, mime)` to `window.__chatwCore` for 0ms synchronous `blobPool` reuse across Dioxus re-mounts.
   - Add `installBlobDomEnforcer()`:
     - Intercept `Element.prototype.setAttribute` (for `<img>` `src`) and `HTMLImageElement.prototype.src` setter so any `/uploads/...`, `/api/attachments/...`, `/stickers/...`, or `/wallpapers/...` URL is resolved synchronously from `blobPool` or pulled ONCE via `SharedWorker` `resolveMediaBlobUrl` (0 raw HTTP image fetches across all 35+ components!).
     - Automatically paint `<canvas data-blur-b64="..." data-blur-mime="...">` via `atob` → `Blob` → `createImageBitmap` → `ctx.drawImage` (0 `data:image/*` URLs in DOM/Network!).
     - Automatically resolve `[data-chat-wp-raw]` / `[data-blob-bg]` elements into `--chat-wp-image: url('<blobUrl>')` / `background-image: url('<blobUrl>')` ONCE via `blobPool`.
2. **`app/src/platform/mod.rs`**:
   - Add `get_cached_blob_url_sync`, `resolve_e2ee_blob_media_url`, and `register_cached_blob_url` helpers.
   - Update `apply_customization` so custom `/uploads/media/...` app wallpapers are NEVER placed directly into `--app-wp-image`; instead JS resolves them ONCE through `window.__chatwCore.resolveMediaBlobUrl` and updates `--app-wp-image` + `window.__chatwCustom.wpImageUrl` + `adaptive_surface.js` with that single `blob:` URL.
3. **`app/web/adaptive_surface.js`**:
   - Route `getLoadedImage(url, onReady)` through `window.__chatwCore.resolveMediaBlobUrl` whenever `url` is `/uploads/...`, `/api/attachments/...`, or `/wallpapers/...` so wallpaper pixels reuse the single `blob:` URL (0 duplicate HTTP fetches!).
   - Replace `cropPanelStripDataUrl` (`canvas.toDataURL`) with `cropPanelStripBlobUrl` (`canvas.toBlob` → `URL.createObjectURL` with revocation of prior strip `blob:` URLs) so 0 `data:image/png;base64,...` entries ever appear.
4. **`app/src/components/rich.rs`, `avatar.rs`, `messages.rs`, `chat_extras.rs`**:
   - `RichImage`: sync-init from `blobPool`, key effect strictly on `(u, k, n)`, resolve BOTH E2EE & plain images via `resolve_e2ee_blob_media_url` (1 single `blob:` URL!), render micro-blur via `<canvas data-blur-b64>` ONLY while loading (0 `<img src="data:image/...">`), remove `poster: "data:image/..."` from `<video>`.
   - `Avatar` & `BlobAssetImg`: sync-init from `get_cached_blob_url_sync` and guard against reloading unchanged URLs.
   - `chat_extras.rs` (`wallpaper_attrs`): emit `--chat-wp-raw` / `data-chat-wp-raw` instead of raw `--chat-wp-image: url('/uploads/...')` so browser never fetches `/uploads/media/...`.
5. **`app/web/sw.js`**:
   - Bump `STATIC_CACHE_NAME` to `chatw-static-v3` and switch `isCacheableStaticAsset` to pure Cache-First (no redundant background `fetch()` on cache hit!).
6. **Verification**:
   - Write & run `app/e2e/spec073-zero-double-load-blob-only-no-data-uri.mjs` testing custom wallpaper + E2EE image with micro-blur + avatar + reload, asserting:
     - 0 `/uploads/*` requests in Network tab
     - 0 `data:image/*` requests/DOM URLs
     - Exactly 1 `blob:` URL per unique image/avatar/wallpaper (no double load!)
     - 0 duplicate `logo.svg` / `chatw_app_bg.wasm` requests on warm reload
   - Visually inspect screenshots + run `spec071`, `spec072`, `spec061` regression suites + commit & push to `origin main`.
