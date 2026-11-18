-- 0010: Profile effects v2 (avatar / name / nameplate, custom uploads), personal
-- emoji + GIF library, English names for built-in stickers, animated sticker pack.

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_avatar_decoration_check;
ALTER TABLE users ADD CONSTRAINT users_avatar_decoration_check
    CHECK (avatar_decoration IN ('none', 'halo', 'crown', 'sprout', 'stars', 'cat', 'custom'));
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_profile_effect_check;
ALTER TABLE users ADD CONSTRAINT users_profile_effect_check
    CHECK (profile_effect IN ('none', 'sparkle', 'snow', 'aurora', 'bubbles', 'confetti', 'hearts', 'custom'));

ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_effect TEXT NOT NULL DEFAULT 'none'
    CHECK (avatar_effect IN ('none', 'spin', 'pulse', 'float', 'wobble', 'rainbow'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS name_effect TEXT NOT NULL DEFAULT 'none'
    CHECK (name_effect IN ('none', 'gradient', 'shimmer', 'rainbow', 'neon', 'wave'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS nameplate TEXT NOT NULL DEFAULT 'none'
    CHECK (nameplate IN ('none', 'aurora', 'sunset', 'ocean', 'galaxy', 'sakura', 'forest', 'custom'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_decoration_url TEXT
    CHECK (custom_decoration_url IS NULL OR custom_decoration_url LIKE '/uploads/media/%');
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_nameplate_url TEXT
    CHECK (custom_nameplate_url IS NULL OR custom_nameplate_url LIKE '/uploads/media/%');
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_effect_url TEXT
    CHECK (custom_effect_url IS NULL OR custom_effect_url LIKE '/uploads/media/%');
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_effect_anim TEXT NOT NULL DEFAULT 'none'
    CHECK (custom_effect_anim IN ('none', 'float', 'spin', 'pulse', 'drift', 'sway'));

CREATE TABLE IF NOT EXISTS custom_emojis (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (name ~ '^[A-Za-z0-9_]{2,32}$'),
    file_url   TEXT NOT NULL CHECK (file_url LIKE '/uploads/media/%'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, name)
);
CREATE INDEX IF NOT EXISTS custom_emojis_owner_idx ON custom_emojis (owner_id);

CREATE TABLE IF NOT EXISTS saved_gifs (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    file_url   TEXT NOT NULL CHECK (file_url LIKE '/uploads/media/%' OR file_url LIKE '/stickers/%'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS saved_gifs_owner_idx ON saved_gifs (owner_id);

-- Sticker and emoji names are always English.
UPDATE sticker_packs SET name = 'Bubble Chat', description = 'Original ChatW stickers'
WHERE id = '00000000-0000-0000-0000-000000000001';
UPDATE stickers SET name = CASE file_url
    WHEN '/stickers/bong-hello.svg' THEN 'Hello'
    WHEN '/stickers/bong-love.svg' THEN 'Love it'
    WHEN '/stickers/bong-laugh.svg' THEN 'LOL'
    WHEN '/stickers/bong-ok.svg' THEN 'Thumbs up'
    WHEN '/stickers/bong-wow.svg' THEN 'Wow'
    WHEN '/stickers/bong-sad.svg' THEN 'Sad'
    WHEN '/stickers/bong-sleepy.svg' THEN 'Sleepy'
    WHEN '/stickers/bong-party.svg' THEN 'Party'
    WHEN '/stickers/bong-thanks.svg' THEN 'Thank you'
    WHEN '/stickers/bong-think.svg' THEN 'Hmm'
    ELSE name END
WHERE pack_id = '00000000-0000-0000-0000-000000000001';

INSERT INTO sticker_packs (id, name, description, cover_url)
VALUES ('00000000-0000-0000-0000-000000000002', 'Bubble Motion', 'Animated ChatW stickers', '/stickers/anim/wave.svg')
ON CONFLICT (id) DO NOTHING;
DELETE FROM stickers WHERE pack_id = '00000000-0000-0000-0000-000000000002';
INSERT INTO stickers (pack_id, name, file_url, is_animated) VALUES
('00000000-0000-0000-0000-000000000002', 'Wave', '/stickers/anim/wave.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Heartbeat', '/stickers/anim/heartbeat.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Bounce', '/stickers/anim/bounce.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Dizzy', '/stickers/anim/dizzy.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Celebrate', '/stickers/anim/celebrate.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Typing', '/stickers/anim/typing.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Cry', '/stickers/anim/cry.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Angry', '/stickers/anim/angry.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Sleep', '/stickers/anim/sleep.svg', TRUE),
('00000000-0000-0000-0000-000000000002', 'Rocket', '/stickers/anim/rocket.svg', TRUE);
