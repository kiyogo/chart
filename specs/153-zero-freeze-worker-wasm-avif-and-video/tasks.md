# Tasks: Spec 153

- [x] T001 Create Spec 153 `spec.md`, `plan.md`, `tasks.md`
- [x] T002 Create `crates/chatw-media-worker` (`Cargo.toml`, `src/lib.rs`) and add to workspace `Cargo.toml`
- [x] T003 Optimize `FastAdaptive` single-thread speed/bit-depth in `crates/chatw-media-core/src/avif.rs`
- [x] T004 Update `app/build-web.sh` to compile `chatw-media-worker` `--release`, pack `CWRT1` `media_worker_wasm.bin`, and copy module worker assets
- [x] T005 Update `app/web/media_worker.js` and `app/web/media_optimizer.js` for 100% off-main-thread WASM AVIF/PDQ/BLAKE3 + fast single-pass video fingerprinting
- [x] T006 Update `app/src/components/rich.rs`, `app/web/ai_restorer.js`, and `web/backend/src/websocket/rpc_bin.rs`
- [x] T007 Verify zero main-thread freeze, video + image DM uploads, unit tests, and Chrome E2E
- [x] T008 Commit and push to `origin/main` per `AGENTS.md` Rule 1
