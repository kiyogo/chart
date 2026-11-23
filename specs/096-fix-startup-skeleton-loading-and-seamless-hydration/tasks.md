# Tasks 096: Fix Startup Skeleton Loading and Seamless Hydration

- [x] Task 1: Update `chat.rs` startup sequence to retain `is_dialogs_loading` throughout bootstrap and include `privacy` & `bots`
- [x] Task 2: Cache `privacy` and `bots` in `save_dialogs_cache` & restore in `load_dialogs_cache`
- [x] Task 3: Wire `loading_initial` and `loading_more` to `MessageList` for DMs, groups, and channels
- [x] Task 4: Wire `SkeletonDetails` into right details sidebar when `dm_loading_initial()` is active
- [x] Task 5: Compile Web WASM release bundle via `./app/build-web.sh release`
- [x] Task 6: Build and run automated Playwright test `app/e2e/spec096-startup-skeleton-loading.mjs` with screenshots
- [x] Task 7: Commit and push changes to `origin main` following the mandatory auto-push rule
