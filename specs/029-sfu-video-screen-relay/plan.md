# Implementation Plan: Server-Side WebRTC SFU (`str0m`) & Screen Share

## Open-Source Reference
- Based on **`algesten/str0m`** (`v0.25.0`, `features = ["rust-crypto"]`, `examples/chat.rs` Sans-I/O SFU reference), adapted for multi-room Axum routing, dual loopback+LAN host UDP sockets, room isolation by `call_id`, explicit `cam`/`screen`/`audio` stream MSIDs (`sfu_{user_id}_{tag}`), and proactive PLI keyframe triggers.

## Files to Modify / Create
1. **`web/backend/Cargo.toml`**: Add `str0m = { version = "0.25.0", default-features = false, features = ["rust-crypto"] }`.
2. **`web/backend/src/calls/sfu.rs`** *(new)*:
   - `SfuHandle` stored in `AppState` (initialized once at server startup in `web/backend/src/main.rs` / `auth::AppState`).
   - Dedicated `kiyo-sfu` OS thread running the Sans-I/O UDP multiplex loop across rooms (`call_id`), with both `127.0.0.1` loopback and LAN IPv4 UDP sockets + `str0m::crypto::from_feature_flags().install_process_default()` (safe `try` install).
   - Handles `SfuCommand::JoinPeer { call_id, user_id, offer_sdp, meta, reply_tx }`, `SfuCommand::LeavePeer { call_id, user_id }`, `SfuCommand::EndCall { call_id }`.
   - Forwards `Event::MediaAdded`, `Event::MediaData`, `Event::MediaChanged`, `Event::KeyframeRequest`, `Event::ChannelOpen`, `Event::ChannelData`.
3. **`web/backend/src/auth/mod.rs` & `web/backend/src/main.rs`**: Register `sfu: Arc<calls::sfu::SfuHandle>` on `AppState`.
4. **`web/backend/src/calls/mod.rs`**:
   - Add `pub mod sfu;`.
   - Routes: `POST /{id}/sfu/join`, `POST /{id}/sfu/leave` (and hook `end_call` / `leave_call` into `state.sfu`).
5. **`app/src/services/calls.rs`**: Add `calls_sfu_join(call_id, sdp, meta)` and `calls_sfu_leave(call_id)`.
6. **`app/src/media/webrtc.rs`**:
   - Rewrite JS controller (`window.__chatwWebRTC`) to connect directly to server SFU via `POST /api/calls/{id}/sfu/join` + DataChannel `"offer/answer"` negotiation queue.
   - Separate local `localAudioStream`, `localCamStream`, `screenStream`, and remote `remoteAudioStreams`, `remoteCamStreams`, `remoteScreenStreams`.
   - Emit structured state `{ local_screen_active, remote_screens: [{ user_id, track_id }], remote_cams: [user_id] }` queried/listened by `ActiveCallOverlay`.
   - Safe `getUserMedia` handling when hardware mic/camera is missing (SFU DataChannel still connects so screen sharing & receiving remote video/audio always work!).
7. **`app/src/components/calls/active_call.rs` & `app/src/pages/chat.rs`**:
   - Always invoke `init_sfu_call(&sess.id, is_video, &me_uid)` on call start/accept/join/meet-link.
   - Render Google Meet / Zoom-style Presentation Stage (`#webrtc-local-screen-video` / `#webrtc-remote-screen-video`) + participant tiles (`#webrtc-local-video`, `#webrtc-remote-video-{uid}`, `#webrtc-remote-audio-{uid}`).
   - Add `.call-stage-*` clean flat CSS in `app/assets/main.css` if needed (or clean inline styling consistent with `active_call.rs`).
