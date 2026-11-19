-- Spec 013: per-community profiles, community emoji / stickers, forum threads, events with RSVP,
-- moderation audit log, welcome screen and rules. Additive only.
-- Rollback: DROP TABLE server_audit_log, server_event_rsvps, server_events, forum_threads, server_expressions;
--   ALTER TABLE channel_messages DROP COLUMN thread_id; ALTER TABLE server_members DROP COLUMN avatar_url, DROP COLUMN bio, DROP COLUMN rules_accepted_at;
--   ALTER TABLE servers DROP COLUMN welcome_message, DROP COLUMN rules, DROP COLUMN require_rules;
ALTER TABLE server_members ADD COLUMN IF NOT EXISTS avatar_url TEXT CHECK (avatar_url IS NULL OR avatar_url LIKE '/uploads/%');
ALTER TABLE server_members ADD COLUMN IF NOT EXISTS bio TEXT CHECK (char_length(bio) <= 190);
ALTER TABLE server_members ADD COLUMN IF NOT EXISTS rules_accepted_at TIMESTAMPTZ;

ALTER TABLE servers ADD COLUMN IF NOT EXISTS welcome_message TEXT CHECK (char_length(welcome_message) <= 500);
ALTER TABLE servers ADD COLUMN IF NOT EXISTS rules TEXT CHECK (char_length(rules) <= 2000);
ALTER TABLE servers ADD COLUMN IF NOT EXISTS require_rules BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS server_expressions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('emoji', 'sticker')),
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 2 AND 32),
    file_url    TEXT NOT NULL CHECK (file_url LIKE '/uploads/%'),
    is_animated BOOLEAN NOT NULL DEFAULT FALSE,
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, kind, name)
);

CREATE TABLE IF NOT EXISTS forum_threads (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    pinned     BOOLEAN NOT NULL DEFAULT FALSE,
    locked     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS forum_threads_channel_idx ON forum_threads (channel_id, pinned DESC, last_activity DESC);
ALTER TABLE channel_messages ADD COLUMN IF NOT EXISTS thread_id UUID REFERENCES forum_threads(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS channel_messages_thread_idx ON channel_messages (thread_id, seq) WHERE thread_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS server_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    title       TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 100),
    description TEXT CHECK (char_length(description) <= 1000),
    location    TEXT CHECK (char_length(location) <= 120),
    starts_at   TIMESTAMPTZ NOT NULL,
    ends_at     TIMESTAMPTZ,
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ends_at IS NULL OR ends_at > starts_at)
);
CREATE INDEX IF NOT EXISTS server_events_server_idx ON server_events (server_id, starts_at);
CREATE TABLE IF NOT EXISTS server_event_rsvps (
    event_id   UUID NOT NULL REFERENCES server_events(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status     TEXT NOT NULL CHECK (status IN ('going', 'interested')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, user_id)
);

CREATE TABLE IF NOT EXISTS server_audit_log (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id  UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    actor_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    action     TEXT NOT NULL CHECK (char_length(action) <= 40),
    target     TEXT CHECK (char_length(target) <= 120),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS server_audit_log_server_idx ON server_audit_log (server_id, created_at DESC);

-- New permission bit 1024 "Manage Expressions" (community emoji / stickers) for roles that can manage the server.
UPDATE server_roles SET permissions = permissions | 1024 WHERE (permissions & 16) <> 0 OR (permissions & 64) <> 0;
