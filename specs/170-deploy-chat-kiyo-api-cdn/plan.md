# Plan 170: Separated API & Media CDN Origins + aaPanel Deployment

## Architecture
1. **Frontend (`app/`)**:
   - Add `app/web/origins.js` (defaulting `CHATW_API_URL = ''`, `CHATW_CDN_URL = ''`).
   - Include `<script src="/origins.js"></script>` before `/core_bridge.js` in `app/web/index.html`, and copy/cache-bust it in `app/build-web.sh`.
   - Update `app/src/platform/web.rs` and `app/src/platform/mod.rs` to add `cdn_base()` and resolve API vs Media URLs using `CHATW_API_URL` and `CHATW_CDN_URL` (falling back to same-origin when empty).
   - Update `app/web/core_bridge.js`, `app/web/storage_engine.js`, `app/web/sw.js`, `app/web/plugin_runtime.js`, `app/web/media_optimizer.js` to resolve WebSocket (`wss://api.chat01vn.kiyo.vn/api/ws?proto=rpc-bin`) and HTTP media/API paths via `CHATW_API_URL` and `CHATW_CDN_URL`.
2. **Backend (`web/backend/`)**:
   - Update `complete_oidc_callback` in `web/backend/src/auth/mod.rs` so web OAuth redirects to `state.config.public_origin` (and 2FA redirects to `{public_origin}/?twofa=1`) when not running as a popup.
3. **Server Deployment (`192.168.1.224`)**:
   - Build `app/dist/` (`./app/build-web.sh release`) locally and upload to `/www/wwwroot/chart/app/dist`.
   - Sync `web/backend` + `crates/` to `/www/wwwroot/chart/` and compile release binary on server with `rustup` (matching GLIBC 2.39), update `.env` (`PUBLIC_URL`, `CORS_ALLOWED_ORIGINS`, `GOOGLE_REDIRECT_URL`, `STATIC_DIR`, `VALKEY_URL`), restart `chart.service`.
   - Configure aaPanel (`default.db` + Nginx vhosts + Let's Encrypt SSL via `acme.sh` / aaPanel) for `chat.kiyo.vn`, `api.chat01vn.kiyo.vn`, `v01.chat-vn.kiyo.vn`, and disable `api.chat.kiyo.vn`.
