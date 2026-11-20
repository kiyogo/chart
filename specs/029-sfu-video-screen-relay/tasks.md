# Tasks: Server-Side WebRTC SFU (`str0m`) & Screen Share

- [x] T001 Add `str0m` (`0.25.0`, `rust-crypto`) to `web/backend/Cargo.toml`
- [x] T002 Implement multi-room Rust `str0m` SFU engine in `web/backend/src/calls/sfu.rs` and wire `SfuHandle` into `AppState` & `/api/calls/{id}/sfu/join` + `/leave` + `leave_call` / `end_call`
- [x] T003 Rewrite `app/src/media/webrtc.rs` + `app/src/services/calls.rs` to stream mic/cam/screen to server SFU, negotiate via DataChannel `"offer/answer"`, handle `screenTrack.onended`, and attach separate remote cam/screen/audio streams
- [x] T004 Update `app/src/components/calls/active_call.rs` & `app/src/pages/chat.rs` for Google Meet / Zoom Presentation Stage + seamless screen share & camera viewing even when camera is off
- [x] T005 Compile backend (`cargo check` / `cargo build`), rebuild web bundle (`./app/build-web.sh`), restart running `chatw` server, run live 2-user Playwright SFU screen-share E2E test verifying server-forwarded video frames on both users, and push to `origin main`
