# Feature Specification: Spec 168 — Fix Wallpaper, Banner & Background Image Uploads

## 1. Root Cause Analysis
Users reported that uploading background/wallpaper/banner images fails and cannot load. Codebase & backend inspection revealed 4 concrete defects:
1. **Missing `library_media` ACL record (`403 media_reference_not_found`)**: `POST /api/media/upload` (`web/backend/src/library/mod.rs`) writes uploaded wallpapers, covers, banners, decorations, and emojis to `uploads/media/{uuid}.{ext}` without registering them in PostgreSQL, while `GET /uploads/media/{file_name}` (`web/backend/src/attachments/mod.rs`) only queries `attachments` and rejects everything else with `403 Forbidden (media_reference_not_found)`.
2. **Oversized PNG / 2 MB ceiling failures on client & server**:
   - `web/backend/src/library/mod.rs` caps `MAX_MEDIA_BYTES` at `2 MiB` (too small for modern wallpapers/covers/GIFs).
   - `process_selected_wallpaper_webp` (`app/src/platform/mod.rs`) does not adaptively downscale dimensions when compressed size exceeds the cap, hardcodes `mime: 'image/webp'` even when Safari/WebKit returns PNG, and lacks a `image/jpeg` fallback.
   - `process_banner_from_input` (`app/src/platform/mod.rs`) calls `canvas.toBlob('image/avif')` which silently returns uncompressed PNG on Chrome/Firefox/Edge without checking `blob.type`, lacks `new Image()` fallback, and uses O(N²) char-by-char string concatenation.
   - `read_selected_file_raw` (`app/src/platform/mod.rs`) rejects files > `2 MiB`.
3. **Upload transport & error surfacing**:
   - `core_bridge.js` excluded `/api/media/`, `/api/users/banner`, `/api/users/avatar`, and `/banner` from the extended upload RPC timeout (`45s`).
   - `platform::web::http_request` did not fall back to same-origin `fetch()` when binary RPC failed transiently (`status == 0`).
   - `apply_customization` (`app/src/platform/mod.rs`) lacked a fallback to load `/uploads/media/...` app wallpapers when `window.__chatwCore` was not yet connected on initial page load.
   - `settings_more.rs` did not clear prior error messages on retry and swallowed unknown errors silently.

## 2. Acceptance Criteria
1. Uploading an **App wallpaper** (`Settings → Appearance → App wallpaper → Upload your image`) succeeds for PNG, JPEG, WebP, AVIF, and high-res (> 4K / > 5 MB) photos, persists across page reload, and visibly renders `--app-wp-image` with `200 OK`.
2. Uploading a **Chat wallpaper** (`Chat menu → Wallpaper → Upload image`) succeeds, persists, and renders on `.chat-column` with `200 OK`.
3. Uploading a **Profile banner** (`Settings → Account → Change banner` and `Profile → Edit → Change banner`) and **Group/Community cover** succeeds cleanly for PNG, JPEG, WebP, AVIF, and animated GIFs without PNG bloat or errors.
4. Security invariants remain 100% intact: unauthenticated requests to `/uploads/media/*` still return `401`, direct cross-site/tab navigation still returns `403`, SVG uploads remain strictly blocked, and private DM/Group/Channel `attachments` ACL remains untouched.
