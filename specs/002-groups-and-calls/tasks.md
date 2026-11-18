# Tasks: Nhóm chat, Gọi thoại và Gọi video thời gian thực

## Phase 1: Database Migrations & Backend Core
- [x] Create `web/backend/migrations/0007_groups_and_calls.sql`
- [x] Update `web/backend/src/models.rs` with Group & Call data models
- [x] Implement `web/backend/src/groups/mod.rs` (routes, permissions, service logic)
- [x] Implement `web/backend/src/calls/mod.rs` (call session manager, WebRTC signaling relay)
- [x] Update `web/backend/src/websocket/mod.rs` with Call & Group WsEvents
- [x] Update `web/backend/src/main.rs` permissions-policy to allow camera/microphone
- [x] Register new routes in `web/backend/src/main.rs` & `web/backend/src/lib.rs`

## Phase 2: Frontend Services & Media Engine
- [x] Update `app/src/models.rs` with Group & Call models and WsEvents
- [x] Implement `app/src/services/groups.rs` (CRUD groups, members, messages)
- [x] Implement `app/src/services/calls.rs` (Call state machine, signaling client)
- [x] Implement `app/src/media/mod.rs` & `app/src/media/webrtc.rs` (cross-platform WebRTC peer connections, local stream capture, remote track attachment)

## Phase 3: Group Chat UI Components
- [x] Implement `app/src/components/groups/create_group.rs`
- [x] Implement `app/src/components/groups/group_info.rs` & `member_list.rs`
- [x] Implement `app/src/components/groups/invite_modal.rs`
- [x] Integrate Groups tab & group chat view into `app/src/pages/chat.rs`

## Phase 4: Audio & Video Calling UI Components
- [x] Implement `app/src/components/calls/incoming_call.rs`
- [x] Implement `app/src/components/calls/outgoing_call.rs`
- [x] Implement `app/src/components/calls/active_call.rs` (1:1 & Group Grid, video tiles, mic/camera controls, active speaker indicator)
- [x] Integrate call buttons in chat header (1:1 & Group) and global call overlay in `app/src/pages/chat.rs`

## Phase 5: Verification & Automated E2E Testing
- [x] Run backend integration tests & database migration verification
- [x] Run multi-account E2E test (Alice, Bob, Charlie) for Group Chat
- [x] Run multi-account E2E test with real media streaming for 1:1 Audio & Video Call
- [x] Run multi-account E2E test for Group Audio & Video Call
- [x] Verify `cargo check`, `cargo clippy`, and `cargo test --workspace --lib`
- [x] Build release web bundle & Linux desktop binary
- [x] Git commit & push to `origin main`

