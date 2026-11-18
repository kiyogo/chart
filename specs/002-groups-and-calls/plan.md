# Technical Plan: Nhóm chat, Gọi thoại và Gọi video thời gian thực

## 1. Kiến trúc Tổng thể

```
[ Frontend: Dioxus (Rust) ]
  ├── UI Components:
  │     ├── Groups: GroupList, CreateGroupModal, GroupHeader, GroupInfo, MemberList, InviteModal
  │     └── Calls: CallOverlay, IncomingCallModal, AudioCallView, VideoCallView, GroupCallGrid
  ├── Media Engine (app/src/media/):
  │     ├── webrtc.rs: WebRTC PeerConnection, MediaStream, Audio/Video tracks, ICE handlers
  │     └── audio_meter.rs: Web AudioContext / AnalyserNode active speaker detection
  └── Services:
        ├── groups.rs: API client for groups CRUD, members, invites, messages
        └── calls.rs: Call session state machine, signaling client (offer/answer/ICE)

                ▲ HTTP (POST/GET/PUT/DELETE) & WebSocket (WsEvent)
                ▼

[ Backend: Axum 0.8 (Rust) ]
  ├── Database: PostgreSQL (SQLx)
  │     ├── groups, group_members, group_invites, group_messages
  │     └── call_sessions, call_participants
  ├── Routers:
  │     ├── /api/groups: Group creation, membership, invites, messages
  │     └── /api/calls: Call session creation, accept/reject, WebRTC signaling relay, history
  └── WebSocket Hub:
        ├── Broadcast group messages & member events
        └── Relay call signaling events (call.incoming, call.accepted, call.rejected, call.signal)
```

## 2. Mô hình Dữ liệu (PostgreSQL Migration `0007_groups_and_calls.sql`)
- `groups`: `id UUID PRIMARY KEY`, `name TEXT`, `description TEXT`, `avatar_url TEXT`, `owner_id UUID REFERENCES users(id)`, `created_at TIMESTAMPTZ`.
- `group_members`: `group_id UUID REFERENCES groups(id)`, `user_id UUID REFERENCES users(id)`, `role TEXT ('owner', 'admin', 'member')`, `joined_at TIMESTAMPTZ`, PRIMARY KEY (group_id, user_id).
- `group_invites`: `code TEXT PRIMARY KEY`, `group_id UUID`, `inviter_id UUID`, `max_uses INT`, `uses INT`, `expires_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ`.
- `group_messages`: `id UUID PRIMARY KEY`, `seq BIGINT GENERATED ALWAYS AS IDENTITY`, `group_id UUID REFERENCES groups(id)`, `sender_id UUID REFERENCES users(id)`, `content TEXT`, `reply_to_id UUID REFERENCES group_messages(id)`, `is_pinned BOOLEAN`, `is_edited BOOLEAN`, `created_at TIMESTAMPTZ`.
- `call_sessions`: `id UUID PRIMARY KEY`, `call_type TEXT ('audio', 'video')`, `caller_id UUID REFERENCES users(id)`, `receiver_id UUID REFERENCES users(id)`, `group_id UUID REFERENCES groups(id)`, `status TEXT ('ringing', 'active', 'ended', 'rejected', 'missed', 'busy')`, `started_at TIMESTAMPTZ`, `ended_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ`.
- `call_participants`: `call_id UUID REFERENCES call_sessions(id)`, `user_id UUID REFERENCES users(id)`, `joined_at TIMESTAMPTZ`, `left_at TIMESTAMPTZ`, `muted BOOLEAN`, `camera_off BOOLEAN`, PRIMARY KEY (call_id, user_id).

## 3. WebRTC & Media Engine
- 1:1 Call: Direct P2P WebRTC connection via SDP offer/answer relay. ICE STUN servers (Google STUN + configurable TURN credentials).
- Group Call: Multi-peer mesh WebRTC signaling relay via Axum WebSocket for up to 6 participants per room (efficient mesh with low overhead), or audio/video routing.
- Audio / Video Tracks: WebRTC MediaStream connected to `<video>` and `<audio>` elements.
- Active speaker detection: Real-time audio analyser polling volume RMS.

