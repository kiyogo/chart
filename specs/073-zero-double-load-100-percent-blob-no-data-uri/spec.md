# Spec 073: Zero Double-Loading, 100% Deduplicated `blob:` Media/Wallpapers, Zero `data:image/*` Network Entries & Pure Cache-First SW

## Problem Statement
Real-user Chrome DevTools (F12 → Network) inspection revealed four concrete leaks/duplicates:
1. Custom wallpapers (`/uploads/media/{id}`) loaded **twice** over HTTP (`CSS --app-wp-image` + `adaptive_surface.js` `new Image()`) instead of using a single deduplicated `blob:` URL via `SharedWorker`.
2. Chat images loaded **twice** with two different `blob:` URLs (`RichImage` created a fresh `URL.createObjectURL` via `storage_engine.js` on every mount/re-render instead of sharing `core_bridge.js`'s deduplicated `blobPool`).
3. Micro-blur previews (`<img class="rich-image-blur" src="data:image/webp;base64,...">`) and translucent edge strips (`canvas.toDataURL('image/png')`) emitted `data:image/*` entries in DevTools Network tab.
4. Service Worker `Stale-While-Revalidate` fired duplicate background `⚙` fetches for already-cached `/logo.svg` and versioned `chatw_app_bg.wasm?v=...`.

## Acceptance Criteria
- AC1: **0** raw `/uploads/...`, `/api/attachments/...`, or `/wallpapers/...` requests appear in Chrome DevTools Network tab across chat messages, avatars, stories, stickers, banners, and app/chat wallpapers.
- AC2: **0** `data:image/*` entries appear in Chrome DevTools Network tab (`RichImage` micro-blur paints via `<canvas>` + `createImageBitmap`, and `adaptive_surface.js` edge strips use revocable `blob:` URLs).
- AC3: Every unique image, avatar, and wallpaper creates **exactly 1** `blob:` URL and reuses it across re-mounts, panels, and `adaptive_surface.js` (**0** duplicate `blob:` creations for the same asset).
- AC4: Versioned/immutable static assets (`chatw_app_bg.wasm?v=...`, `/logo.svg`, `/locales/*`, `/stickers/*`) use pure Cache-First in `/sw.js` with **0** duplicate `⚙` background re-fetches.
- AC5: Verified via live Playwright Network audit (`spec073`) + visual screenshots + `spec071` & `spec072` & `spec061` regression suites.
