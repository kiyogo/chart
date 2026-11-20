# Feature Specification: Google Meet / Zoom SFU Media & Screen Share Relay

**Feature Branch**: `main`  
**Created**: 2026-11-20  
**Status**: Approved  

## User Request
"lỗi mình chia sẻ màn hình rồi mà nó vẫn không hiện cái gì ,... nó phải bắn video ,... lên server xong nó sẽ từ server trả về cho người dùng làm giống như 100% công nghệ như gg met hay zoom á"

## Root Causes Identified
1. Screen sharing previously relied on direct P2P `addTrack` without renegotiation and hid the local `<video>` behind `display: none` when `is_camera_off` was true or when `getDisplayMedia` ran before `<video>` mounted in audio calls.
2. Media did not pass through a central SFU server — there was no server-side RTP/frame forwarding (`str0m` SFU) receiving published camera, microphone, and screen tracks and returning them to subscribers in the room.
3. No dedicated screen-share presentation stage existed in `ActiveCallOverlay`, and clicking the browser's native "Dừng chia sẻ" button did not notify the UI or remote peers to tear down the shared screen.

## Functional Requirements
- **FR-001 (Server SFU Engine)**: Integrate pure-Rust `str0m` (`v0.25.0`, `rust-crypto`) SFU engine in `web/backend/src/calls/sfu.rs` bound to UDP socket(s) (`127.0.0.1` + LAN IPv4), forwarding incoming audio, camera video, and screen-share video frames from each publisher in a call room (`call_id`) to all other active peers in that room.
- **FR-002 (SFU Signaling & Track Metadata)**: Expose `POST /api/calls/{id}/sfu/join` and `POST /api/calls/{id}/sfu/leave`, plus DataChannel `"offer/answer"` dynamic SDP renegotiation supporting `meta` tags (`audio`, `cam`, `screen`), immediate `KeyframeRequestKind::Pli` on subscriber track open, and clean `stop_media` when a presenter stops sharing or leaves.
- **FR-003 (Independent Media Transceivers & Screen Share)**: Update `app/src/media/webrtc.rs` so each client establishes a single `RTCPeerConnection` to the server SFU, publishes mic, camera, and screen share independently (works even when camera/mic are absent or muted), renegotiates cleanly over DataChannel `"offer/answer"`, and handles native browser `screenTrack.onended`.
- **FR-004 (Google Meet / Zoom Presentation Stage UI)**: Update `ActiveCallOverlay` (`app/src/components/calls/active_call.rs`) to render a large live Presentation Stage (`object-fit: contain`) whenever local or remote screen sharing is active, keeping participant tiles visible without hiding active video streams.

## Success Criteria
- Sharing screen in a live meeting sends video frames up to the backend Rust SFU (`str0m`) and the server forwards the live video stream back to all other participants in the room.
- Both presenter preview ("Bạn đang chia sẻ màn hình") and remote subscriber view (`"{username} đang chia sẻ màn hình"`) render non-black, live updating video frames (`videoWidth > 0`, frames advancing).
- Stopping screen share (via app button or browser bar) cleanly removes the presentation stage on both presenter and remote viewers, and re-sharing works repeatedly.
- Verified with `cargo test`, `./app/build-web.sh`, and live two-user Chromium E2E test with fake/synthetic screen capture (`getDisplayMedia`) + pixel/frame verification.
