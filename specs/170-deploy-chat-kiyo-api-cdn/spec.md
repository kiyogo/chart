# Spec 170: Deploy ChatW Dioxus App & Separated API / Media CDN Origins (`chat.kiyo.vn`)

## Overview
Deploy the ChatW Dioxus WebAssembly application (`app/dist`) and Rust Axum backend (`web/backend`) onto the production aaPanel server (`192.168.1.224`) using three dedicated domains:
- **Web App**: `https://chat.kiyo.vn`
- **API & WebSocket & OAuth**: `https://api.chat01vn.kiyo.vn` (replacing retired `api.chat.kiyo.vn`)
- **Media CDN (images, video, audio, attachments, stickers)**: `https://v01.chat-vn.kiyo.vn`

## Requirements
1. **Runtime Origin Configuration (`app/`)**:
   - Support optional `window.CHATW_API_URL` and `window.CHATW_CDN_URL` loaded from external `/origins.js` (CSP-safe, zero inline scripts).
   - Default to `""` (same-origin) in the repository so local development and automated tests remain 100% unchanged.
   - Route API & WebSocket & OAuth flows (`/api/...`, `/api/ws`, `/api/auth/google`, `/api/auth/kiyo`) to `CHATW_API_URL` when configured.
   - Route media asset & attachment fetches/uploads (`/uploads/...`, `/api/attachments/...`, `/api/media/...`, `/api/storage/uploads/...`, `/api/storage/msg-chunks/...`, `/stickers/...`, `/wallpapers/...`) to `CHATW_CDN_URL` when configured.
2. **Backend OAuth & Cross-Subdomain Support (`web/backend/`)**:
   - Redirect web OAuth sign-in completions back to `PUBLIC_URL` (`https://chat.kiyo.vn/`) when not opened as a desktop ticket or popup.
   - Support `CORS_ALLOWED_ORIGINS` covering `https://chat.kiyo.vn`, `https://api.chat01vn.kiyo.vn`, and `https://v01.chat-vn.kiyo.vn`.
3. **aaPanel + Nginx + SSL Deployment**:
   - Update `chat.kiyo.vn` in aaPanel to serve `app/dist/` with WASM-compatible CSP (`'wasm-unsafe-eval'`, `worker-src 'self' blob:`, `connect-src` and `img-src`/`media-src` allowing `api.chat01vn.kiyo.vn` and `v01.chat-vn.kiyo.vn`).
   - Create & enable `api.chat01vn.kiyo.vn` and `v01.chat-vn.kiyo.vn` in aaPanel with Let's Encrypt SSL certificates, HTTP/2, WebSocket upgrades, `Domain=.kiyo.vn` cookie sharing, and streaming-optimized media proxy headers (`Accept-Ranges`, `proxy_buffering off`, CORS).
   - Retire `api.chat.kiyo.vn` from active vhosts.

## Acceptance Criteria
- `https://chat.kiyo.vn/` loads the Dioxus WebAssembly app (`200 OK`) with `origins.js` pointing API to `https://api.chat01vn.kiyo.vn` and Media CDN to `https://v01.chat-vn.kiyo.vn`.
- `https://api.chat01vn.kiyo.vn/api/auth/providers` returns valid JSON (`200 OK`) with CORS headers for `https://chat.kiyo.vn`.
- `https://v01.chat-vn.kiyo.vn/uploads/avatars/...` and media/attachment endpoints respond with valid TLS, CORS, and byte-range streaming support.
- `api.chat.kiyo.vn` is disabled/removed as requested.
- `chatw` systemd service runs the updated release binary with all SQLx migrations applied cleanly.
