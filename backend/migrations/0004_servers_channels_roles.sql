-- Giai doan 4: Discord-style Server, Categories, Channels, Roles, and Permissions

-- 1. Servers table
CREATE TABLE IF NOT EXISTS servers (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 2 AND 64),
    description TEXT,
    icon_url    TEXT,
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS servers_owner_idx ON servers (owner_id);

-- 2. Server Categories (for grouping channels)
CREATE TABLE IF NOT EXISTS server_categories (
    id         UUID PRIMARY KEY,
    server_id  UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    position   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS server_categories_server_idx ON server_categories (server_id, position);

-- 3. Server Channels (Text, Voice, Announcement, Forum)
CREATE TABLE IF NOT EXISTS channels (
    id           UUID PRIMARY KEY,
    server_id    UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    category_id  UUID REFERENCES server_categories(id) ON DELETE SET NULL,
    name         TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    channel_type TEXT NOT NULL CHECK (channel_type IN ('text', 'voice', 'announcement', 'forum')),
    position     INT NOT NULL DEFAULT 0,
    is_private   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS channels_server_idx ON channels (server_id, position);

-- 4. Server Roles & Permissions (RBAC)
CREATE TABLE IF NOT EXISTS server_roles (
    id          UUID PRIMARY KEY,
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 32),
    color       TEXT NOT NULL DEFAULT '#99AAB5',
    position    INT NOT NULL DEFAULT 0,
    permissions BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS server_roles_server_idx ON server_roles (server_id, position);

-- 5. Server Members
CREATE TABLE IF NOT EXISTS server_members (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    nickname  TEXT,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);
CREATE INDEX IF NOT EXISTS server_members_user_idx ON server_members (user_id);

-- 6. Server Member Roles
CREATE TABLE IF NOT EXISTS server_member_roles (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id   UUID NOT NULL REFERENCES server_roles(id) ON DELETE CASCADE,
    PRIMARY KEY (server_id, user_id, role_id)
);

-- 7. Channel Messages
CREATE TABLE IF NOT EXISTS channel_messages (
    id          UUID PRIMARY KEY,
    seq         BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content     TEXT NOT NULL,
    reply_to_id UUID REFERENCES channel_messages(id) ON DELETE SET NULL,
    is_pinned   BOOLEAN NOT NULL DEFAULT FALSE,
    is_edited   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS channel_messages_channel_idx ON channel_messages (channel_id, seq);

-- 8. Server Invites
CREATE TABLE IF NOT EXISTS server_invites (
    code       TEXT PRIMARY KEY CHECK (char_length(code) BETWEEN 6 AND 16),
    server_id  UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    inviter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_uses   INT,
    uses       INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS server_invites_server_idx ON server_invites (server_id);
