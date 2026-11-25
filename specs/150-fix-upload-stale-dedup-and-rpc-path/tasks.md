# Tasks: Spec 150

- [x] T001 Write Spec 150 `spec.md`, `plan.md`, `tasks.md`
- [x] T002 Add `removeMedia`, `stashPreviewFile` for all `localHit` branches, `allowSenderOriginal` guard, and `reencodeFromPreviewUrl` in `app/web/media_optimizer.js`
- [x] T003 Add `reencode_from_preview_url` & `evict_local_dedup_entry` in `app/src/storage/media_opt.rs` and auto-recovery fallback in `app/src/services/storage_sync.rs`
- [x] T004 Update `upload_error_message` and error logging in `app/src/pages/chat.rs`
- [x] T005 Update `is_safe_internal_path` and 4xx/5xx RPC logging in `web/backend/src/websocket/rpc_bin.rs` + unit test + increase upload RPC timeout in `core_bridge.js` / `core_worker.js`
- [x] T006 Run `./app/build-web.sh`, `cargo test -p chatw-media-core`, `cargo test -p chatw` (targeted unit tests), rebuild `./target/debug/chatw`, restart daemon
- [x] T007 Verify stale-dedup auto-recovery, regular image upload, pasted image upload, and file upload end-to-end in live Chrome CDP & Postgres
- [x] T008 Commit and push to `origin/main` per `AGENTS.md` Rule 1
