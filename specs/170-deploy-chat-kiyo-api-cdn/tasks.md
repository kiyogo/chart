# Tasks 170: Deploy ChatW App & API/CDN Domains

- [x] T001 Add `app/web/origins.js`, include in `app/web/index.html`, and copy/version in `app/build-web.sh`
- [x] T002 Wire `CHATW_API_URL` and `CHATW_CDN_URL` resolution in `app/src/platform/mod.rs` and `app/src/platform/web.rs`
- [x] T003 Wire `CHATW_API_URL` and `CHATW_CDN_URL` in `app/web/core_bridge.js`, `app/web/storage_engine.js`, `app/web/media_optimizer.js`, `app/web/plugin_runtime.js`, and `app/web/sw.js`
- [x] T004 Update `web/backend/src/auth/mod.rs` web OAuth callback redirect to `public_origin`
- [x] T005 Build `app/dist` release bundle (`./app/build-web.sh release`) and run `cargo test` checks
- [x] T006 Deploy backend & `app/dist` to `192.168.1.224` (`/www/wwwroot/chart`), configure `.env` & restart `chart.service`
- [x] T007 Register/update `chat.kiyo.vn`, `api.chat01vn.kiyo.vn`, `v01.chat-vn.kiyo.vn` in aaPanel, issue SSL certs, disable `api.chat.kiyo.vn`, and reload Nginx
- [x] T008 Verify live HTTPS, API, WebSocket, and Media CDN endpoints and push repository changes to GitHub
