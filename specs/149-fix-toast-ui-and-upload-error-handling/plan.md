# Plan 149: Fix Toast/Sync/Modal UI Defects & Upload Stale-Session Error Handling

1. `app/assets/main.css`:
   - Exclude `.app-toast .icon-btn` / add high-specificity override after line 9533 so `.app-toast .icon-btn` + `.app-toast .icon-btn svg` always use `color: var(--bg-elevated) !important`, circular radius, `background: transparent !important` at rest, and subtle translucent hover (`color-mix(in srgb, var(--bg-elevated) 18%, transparent)`).
2. `app/src/components/storage_ui.rs`:
   - Change `class: "toast"` on `e2ee-auto-sync-pill` (line 1195) to `class: "app-toast"`.
3. `app/src/pages/chat.rs`:
   - Add shared helper `upload_error_message(code: &str, channel_mode: bool) -> String` used by `send_dm_media`, `send_group_media`, and `send_channel_media`.
   - When `err == "unauthorized"`, wipe stale session & invoke `on_logout.call(())` after showing `"Session expired. Please sign in again"`.
   - Defer `WelcomeTour` while `locked() || crate::storage::E2EE_RECOVERY_REQUIRED.read().is_some()`.
4. `app/src/storage/realesrgan_gpu.rs`:
   - Remove the 5 unused imports reported by `cargo build`.
5. Rebuild `./app/build-web.sh`, run `cargo test -p chatw-media-core`, and verify in headless Chrome CDP with screenshots & assertions.
