# Technical Plan: Spec 168 — Fix Wallpaper, Banner & Background Image Uploads

## 1. Database Migration (`web/backend/migrations/0043_library_media_registry.sql`)
- Create `library_media` table (`file_name TEXT PRIMARY KEY`, `uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`, `mime_type TEXT NOT NULL`, `size_bytes BIGINT NOT NULL`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`) with index on `uploader_id`.

## 2. Backend Changes
- `web/backend/src/library/mod.rs`:
  - Increase `MAX_MEDIA_BYTES` from `2 MiB` to `10 MiB` (`10 * 1024 * 1024`).
  - Persist every uploaded `/uploads/media/{filename}` into `library_media` with its sniffed MIME type and byte size.
  - Extend `sniff_image_ext` to recognize `mif1`/`msf1` AVIF brands too (matching `users::sniff_image`).
- `web/backend/src/attachments/mod.rs`:
  - In `handle_protected_media_file`: after checking `attachments` (preserving 100% of chat attachment ACLs), check `library_media WHERE file_name = $1` and serve with `serve_existing_key_with_range` (still requiring valid authenticated session + `enforce_anti_direct_media_access`!).

## 3. Frontend Changes
- `app/src/platform/mod.rs`:
  - Upgrade `process_selected_wallpaper_webp`: preserve original WebP/AVIF/JPEG/PNG up to 4 MB when <= 2560px or compress with adaptive dimension downscaling (`2560 → 1920 → 1600 → 1280`), verify actual `blob.type`, fall back `image/webp` → `image/jpeg`, raise output cap to `8 MB`, return accurate `blob.type`.
  - Upgrade `process_banner_from_input` & `process_avatar_from_input`: add `new Image()` fallback when `createImageBitmap` fails, verify `blob.type` on `canvas.toBlob` (so unsupported `image/avif` properly falls back to `image/webp` / `image/jpeg` instead of giant PNG), use chunked `0x8000` base64 encoding.
  - Raise `read_selected_file_raw` cap from `2 MB` to `8 MB`.
  - Upgrade `apply_customization`: add `fetch(rawWp, { credentials: 'include' })` fallback if `core.downloadMediaBlobUrl` fails or `core` is not yet connected, so custom app wallpapers ALWAYS render immediately and after reload.
- `app/src/platform/web.rs` & `app/web/core_bridge.js`:
  - Add `/api/media/`, `/api/users/banner`, `/api/users/avatar`, `/banner` to extended upload timeout (`45s`).
  - Fall back to same-origin `fetch()` if binary RPC transport returns `status == 0` on an upload (`raw_body.is_some()`).
- `app/src/services/api.rs`:
  - Detect actual sniffed MIME type in `users_upload_banner` instead of always hardcoding `"image/avif"`.
- `app/src/components/settings_more.rs` & `app/src/components/chat_extras.rs`:
  - Clear error notice on new file selection, handle `svg_not_allowed` / `too_large` / `unsupported` / API error codes explicitly, and show a clean preview thumbnail + "Clear" option when a custom uploaded App Wallpaper is active.

## 4. Verification
- Run backend unit tests (`cargo test -p chatw --lib`) + frontend `cargo check` + `./app/build-web.sh release`.
- Run real Google Chrome CDP end-to-end test uploading a real PNG/high-res image as:
  1. **App wallpaper** (verify HTTP 200 on `/api/media/upload`, HTTP 200 on `/uploads/media/...`, `--app-wp-image` set and rendered, persists after `Page.reload()`!)
  2. **Chat wallpaper** (verify HTTP 200 + `.chat-column.wp-image` rendered!)
  3. **Profile banner** (verify HTTP 200 on `/api/users/banner` + HTTP 200 on `/uploads/banners/...`!)
- Commit & push to `https://github.com/kiyogo/chart` (`main`).
