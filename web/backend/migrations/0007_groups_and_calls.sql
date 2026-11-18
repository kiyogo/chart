-- Phase 7: Group Conversations & Audio/Video Calling

-- 1. Groups table
CREATE TABLE IF NOT EXISTS groups (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description TEXT CHECK (char_length(description) <= 256),
    avatar_url  TEXT,
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS groups_owner_idx ON groups (owner_id);

-- 2. Group Members
CREATE TABLE IF NOT EXISTS group_members (
    group_id   UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX IF NOT EXISTS group_members_user_idx ON group_members (user_id);

-- 3. Group Invites
CREATE TABLE IF NOT EXISTS group_invites (
    code       TEXT PRIMARY KEY CHECK (char_length(code) BETWEEN 6 AND 32),
    group_id   UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    inviter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_uses   INT,
    uses       INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS group_invites_group_idx ON group_invites (group_id);

-- 4. Group Messages
CREATE TABLE IF NOT EXISTS group_messages (
    id          UUID PRIMARY KEY,
    seq         BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    group_id    UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    sender_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content     TEXT NOT NULL,
    reply_to_id UUID REFERENCES group_messages(id) ON DELETE SET NULL,
    is_pinned   BOOLEAN NOT NULL DEFAULT FALSE,
    is_edited   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS group_messages_group_idx ON group_messages (group_id, seq);

-- 5. Call Sessions
CREATE TABLE IF NOT EXISTS call_sessions (
    id          UUID PRIMARY KEY,
    call_type   TEXT NOT NULL CHECK (call_type IN ('audio', 'video')),
    caller_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id UUID REFERENCES users(id) ON DELETE CASCADE,
    group_id    UUID REFERENCES groups(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('ringing', 'active', 'ended', 'rejected', 'missed', 'busy')),
    started_at  TIMESTAMPTZ,
    ended_at    TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS call_sessions_caller_idx ON call_sessions (caller_id, status);
CREATE INDEX IF NOT EXISTS call_sessions_receiver_idx ON call_sessions (receiver_id, status);
CREATE INDEX IF NOT EXISTS call_sessions_group_idx ON call_sessions (group_id, status);

-- 6. Call Participants
CREATE TABLE IF NOT EXISTS call_participants (
    call_id    UUID NOT NULL REFERENCES call_sessions(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at    TIMESTAMPTZ,
    muted      BOOLEAN NOT NULL DEFAULT FALSE,
    camera_off BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (call_id, user_id)
);
CREATE INDEX IF NOT EXISTS call_participants_user_idx ON call_participants (user_id);

