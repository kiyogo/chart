-- Phase 8: Group Categories, Channels, Reactions, Stickers & Extended Profiles

-- 1. Group Categories
CREATE TABLE IF NOT EXISTS group_categories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id   UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    position   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS group_categories_group_idx ON group_categories (group_id, position);

-- 2. Group Channels
CREATE TABLE IF NOT EXISTS group_channels (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    category_id UUID REFERENCES group_categories(id) ON DELETE SET NULL,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description TEXT CHECK (char_length(description) <= 256),
    channel_type TEXT NOT NULL DEFAULT 'text' CHECK (channel_type IN ('text', 'voice', 'video', 'announcement', 'forum', 'media')),
    position    INT NOT NULL DEFAULT 0,
    is_private  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS group_channels_group_idx ON group_channels (group_id, position);

-- 3. Group Messages: add channel_id
ALTER TABLE group_messages ADD COLUMN IF NOT EXISTS channel_id UUID REFERENCES group_channels(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS group_messages_channel_idx ON group_messages (channel_id, seq);

-- 4. Group Message Reactions
CREATE TABLE IF NOT EXISTS group_message_reactions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES group_messages(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji      TEXT NOT NULL CHECK (char_length(emoji) BETWEEN 1 AND 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, user_id, emoji)
);
CREATE INDEX IF NOT EXISTS group_message_reactions_msg_idx ON group_message_reactions (message_id);

-- 5. Sticker Packs and Stickers
CREATE TABLE IF NOT EXISTS sticker_packs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description TEXT CHECK (char_length(description) <= 256),
    author_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    cover_url   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS stickers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pack_id     UUID NOT NULL REFERENCES sticker_packs(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    file_url    TEXT NOT NULL,
    is_animated BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS stickers_pack_idx ON stickers (pack_id);

-- 6. User Profile extensions
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name TEXT CHECK (char_length(display_name) <= 64);
ALTER TABLE users ADD COLUMN IF NOT EXISTS badges TEXT[] NOT NULL DEFAULT '{}'::text[];
ALTER TABLE users ADD COLUMN IF NOT EXISTS social_links JSONB NOT NULL DEFAULT '{}'::jsonb;

-- 7. Ensure existing groups have default category and channel
DO $$
DECLARE
    g RECORD;
    cat_id UUID;
    chan_id UUID;
BEGIN
    FOR g IN SELECT id FROM groups LOOP
        IF NOT EXISTS (SELECT 1 FROM group_channels WHERE group_id = g.id) THEN
            cat_id := gen_random_uuid();
            chan_id := gen_random_uuid();
            INSERT INTO group_categories (id, group_id, name, position)
            VALUES (cat_id, g.id, 'Text Channels', 0);
            INSERT INTO group_channels (id, group_id, category_id, name, channel_type, position)
            VALUES (chan_id, g.id, cat_id, 'general', 'text', 0);
            UPDATE group_messages SET channel_id = chan_id WHERE group_id = g.id AND channel_id IS NULL;
        END IF;
    END LOOP;
END $$;

-- 8. Seed default sticker pack
INSERT INTO sticker_packs (id, name, description, cover_url)
VALUES ('00000000-0000-0000-0000-000000000001', 'ChatW Essentials', 'Bộ sticker chính thức của ChatW', 'https://api.dicebear.com/7.x/bottts/svg?seed=chatw')
ON CONFLICT (id) DO NOTHING;

INSERT INTO stickers (pack_id, name, file_url)
VALUES
('00000000-0000-0000-0000-000000000001', 'Wave', '👋'),
('00000000-0000-0000-0000-000000000001', 'Love', '💖'),
('00000000-0000-0000-0000-000000000001', 'Party', '🎉'),
('00000000-0000-0000-0000-000000000001', 'Rocket', '🚀'),
('00000000-0000-0000-0000-000000000001', 'Fire', '🔥'),
('00000000-0000-0000-0000-000000000001', 'Cool', '😎'),
('00000000-0000-0000-0000-000000000001', 'Laugh', '😂'),
('00000000-0000-0000-0000-000000000001', 'ThumbsUp', '👍')
ON CONFLICT DO NOTHING;
