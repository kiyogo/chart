# Tasks 149: Fix Toast/Sync/Modal UI Defects & Upload Stale-Session Error Handling

- [x] T1: Diagnose `.app-toast .icon-btn` CSS override, `"Device sync"` `.toast` flex bar, overlapping modals, and `"Không tải tệp lên được"` root cause in Chrome CDP.
- [x] T2: Fix `.app-toast .icon-btn` contrast & hover styling in `app/assets/main.css` (verified `btnColor: rgb(255, 255, 255)`, `svgColor: rgb(255, 255, 255)`, `btnRadius: 9999px`).
- [x] T3: Fix `e2ee-auto-sync-pill` layout class (`toast` -> `app-toast`) in `app/src/components/storage_ui.rs`.
- [x] T4: Add shared `upload_error_message` helper + `"unauthorized"` session recovery and defer `WelcomeTour` behind E2EE recovery/PIN lock in `app/src/pages/chat.rs`.
- [x] T5: Remove unused imports in `app/src/storage/realesrgan_gpu.rs`.
- [x] T6: Run `./app/build-web.sh` (0 warnings) & `cargo test -p chatw-media-core` (42/42 passed) and verify live Chrome CDP screenshots + computed toast colors + DM uploads.
- [x] T7: Commit and push to `origin/main` (`https://github.com/kiyogo/chart`).
