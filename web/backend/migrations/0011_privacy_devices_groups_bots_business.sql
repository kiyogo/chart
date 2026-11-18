-- 0011: devices & self chat, privacy & blocks, labels & conversation prefs,
-- read receipts, shared chat wallpapers, group management & join requests,
-- public directory verification, bots, business pages, admin flag.

-- Devices: human label per session ("Windows 1", "Linux 2"), editable.
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS device_label TEXT
    CHECK (device_label IS NULL OR char_length(device_label) BETWEEN 1 AND 40);

-- Users: self chat, privacy, admin, suspension.
ALTER TABLE users ADD COLUMN IF NOT EXISTS self_chat BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_friend_requests TEXT NOT NULL DEFAULT 'everyone'
    CHECK (privacy_friend_requests IN ('everyone', 'nobody'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_calls TEXT NOT NULL DEFAULT 'friends'
    CHECK (privacy_calls IN ('friends', 'nobody'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_group_invites TEXT NOT NULL DEFAULT 'friends'
    CHECK (privacy_group_invites IN ('everyone', 'friends', 'nobody'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_discoverable BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_read_receipts BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS privacy_typing BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS user_blocks (
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CHECK (blocker_id <> blocked_id)
);
CREATE INDEX IF NOT EXISTS user_blocks_blocked_idx ON user_blocks (blocked_id);

-- Labels ("thẻ phân loại") and per-user conversation preferences.
CREATE TABLE IF NOT EXISTS user_labels (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 32),
    color      TEXT NOT NULL CHECK (color ~ '^#[0-9A-Fa-f]{6}$'),
    position   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS user_labels_user_idx ON user_labels (user_id);

CREATE TABLE IF NOT EXISTS conversation_prefs (
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conv_type      TEXT NOT NULL CHECK (conv_type IN ('dm', 'group', 'channel', 'self', 'bot')),
    conv_id        UUID NOT NULL,
    pinned_at      TIMESTAMPTZ,
    label_ids      UUID[] NOT NULL DEFAULT '{}',
    in_messages    BOOLEAN NOT NULL DEFAULT FALSE,
    in_communities BOOLEAN NOT NULL DEFAULT FALSE,
    muted          BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, conv_type, conv_id)
);

-- Read receipts for direct messages (only shared when the reader allows it).
CREATE TABLE IF NOT EXISTS dm_reads (
    reader_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    partner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_read_seq BIGINT NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (reader_id, partner_id)
);

-- Wallpaper chosen for a conversation, visible to every participant.
CREATE TABLE IF NOT EXISTS shared_wallpapers (
    conv_key   TEXT PRIMARY KEY CHECK (char_length(conv_key) <= 100),
    value      TEXT NOT NULL CHECK (char_length(value) <= 300),
    set_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Group management (Zalo-style member permissions).
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_send BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_poll BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_note BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_reminder BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_pin BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_rename BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS perm_avatar BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS require_approval BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS highlight_admins BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS allow_link_join BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS banner_url TEXT;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS is_public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS verification_status TEXT NOT NULL DEFAULT 'none'
    CHECK (verification_status IN ('none', 'pending', 'verified', 'rejected'));

CREATE TABLE IF NOT EXISTS group_join_requests (
    group_id   UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note       TEXT CHECK (char_length(note) <= 200),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id)
);

-- A user who blocks a group cannot be added back to it.
CREATE TABLE IF NOT EXISTS group_blocks (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id   UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, group_id)
);

-- Communities: public listing needs verification; optional search-engine indexing.
ALTER TABLE servers ADD COLUMN IF NOT EXISTS is_public BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS verification_status TEXT NOT NULL DEFAULT 'none'
    CHECK (verification_status IN ('none', 'pending', 'verified', 'rejected'));
ALTER TABLE servers ADD COLUMN IF NOT EXISTS public_index BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS banner_url TEXT;

-- Bots (Telegram-style): token is only stored as a SHA-256 hash.
CREATE TABLE IF NOT EXISTS bots (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username            TEXT NOT NULL CHECK (char_length(username) BETWEEN 2 AND 32),
    username_norm       TEXT NOT NULL UNIQUE,
    display_name        TEXT NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 64),
    description         TEXT CHECK (char_length(description) <= 512),
    avatar_url          TEXT,
    token_hash          BYTEA NOT NULL UNIQUE,
    token_hint          TEXT NOT NULL,
    is_public           BOOLEAN NOT NULL DEFAULT FALSE,
    verification_status TEXT NOT NULL DEFAULT 'none'
        CHECK (verification_status IN ('none', 'pending', 'verified', 'rejected')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS bots_owner_idx ON bots (owner_id);

-- Direct conversation between a user and a bot (NOT end-to-end encrypted).
CREATE TABLE IF NOT EXISTS bot_messages (
    seq        BIGSERIAL PRIMARY KEY,
    id         UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    bot_id     UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_bot   BOOLEAN NOT NULL,
    text       TEXT NOT NULL CHECK (char_length(text) BETWEEN 1 AND 4096),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS bot_messages_chat_idx ON bot_messages (bot_id, user_id, seq);

-- Groups a bot was added to.
CREATE TABLE IF NOT EXISTS bot_group_members (
    bot_id   UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (bot_id, group_id)
);
ALTER TABLE group_messages ADD COLUMN IF NOT EXISTS bot_id UUID REFERENCES bots(id) ON DELETE SET NULL;

-- Update queue consumed by getUpdates (long polling).
CREATE TABLE IF NOT EXISTS bot_updates (
    update_id  BIGSERIAL PRIMARY KEY,
    bot_id     UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    payload    JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS bot_updates_bot_idx ON bot_updates (bot_id, update_id);

-- Business ("Kinh doanh") public pages.
CREATE TABLE IF NOT EXISTS business_profiles (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    handle              TEXT NOT NULL UNIQUE CHECK (handle ~ '^[a-z0-9][a-z0-9_.-]{2,31}$'),
    name                TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    category            TEXT CHECK (char_length(category) <= 60),
    description         TEXT CHECK (char_length(description) <= 2000),
    address             TEXT CHECK (char_length(address) <= 200),
    hours               TEXT CHECK (char_length(hours) <= 200),
    website             TEXT CHECK (char_length(website) <= 200),
    contact             TEXT CHECK (char_length(contact) <= 80),
    cover_url           TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    greeting            TEXT CHECK (char_length(greeting) <= 500),
    away_message        TEXT CHECK (char_length(away_message) <= 500),
    verification_status TEXT NOT NULL DEFAULT 'none'
        CHECK (verification_status IN ('none', 'pending', 'verified', 'rejected')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS business_products (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    price       TEXT CHECK (char_length(price) <= 40),
    description TEXT CHECK (char_length(description) <= 1000),
    image_url   TEXT CHECK (image_url IS NULL OR image_url LIKE '/uploads/media/%'),
    position    INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS business_products_user_idx ON business_products (user_id, position);
CREATE TABLE IF NOT EXISTS business_quick_replies (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shortcut TEXT NOT NULL CHECK (shortcut ~ '^[a-z0-9_]{1,24}$'),
    text     TEXT NOT NULL CHECK (char_length(text) BETWEEN 1 AND 1000),
    UNIQUE (user_id, shortcut)
);
