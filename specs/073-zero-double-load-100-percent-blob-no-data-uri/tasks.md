# Tasks: Spec 073 — Zero Double-Load, 100% Deduplicated `blob:` & Zero `data:image/*`

- [x] T001 Upgrade `app/web/core_bridge.js` with `getCachedBlobUrlSync`, `registerCachedBlobUrl`, and `installBlobDomEnforcer` (`<img>` prototype blob interceptor, `<canvas data-blur-b64>` painter, and inline CSS `url('/uploads/...')` blob rewriter)
- [x] T002 Upgrade `app/web/adaptive_surface.js` (reuse `blob:` URL in `getLoadedImage`, replace `toDataURL` edge strips with revocable `blob:` URLs) and `app/web/sw.js` (pure Cache-First `chatw-static-v3` with zero duplicate background `fetch()`)
- [x] T003 Upgrade `app/src/platform/mod.rs` (`apply_customization` blob-only app wallpaper + `get_cached_blob_url_sync` / `resolve_e2ee_blob_media_url` / `register_cached_blob_url`) and `app/src/components/chat_extras.rs` (zero raw `/uploads/` CSS via DOM enforcer)
- [x] T004 Upgrade `app/src/components/rich.rs` (`RichImage` 100% deduplicated single `blob:` URL via `SharedWorker` + `<canvas data-blur-b64>` zero `data:` URI blur-up), `app/src/components/avatar.rs`, and `app/src/components/messages.rs` (sync cache init, zero double-load)
- [x] T005 Rebuild release WASM, run unit tests (31/31) + new `spec073` (6/6) + `spec072` (5/5) + `spec071` (21/21) + `spec061` (7/7) + `spec069` (18/18), visually inspect screenshot, commit & push to `origin main`
