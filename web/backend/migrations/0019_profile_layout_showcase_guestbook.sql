-- Spec 012: profile layout (order / visibility / size of sections, template), saved layout presets,
-- showcase items, guestbook and achievement unlock history. Additive only.
-- Rollback: DROP TABLE user_achievements, profile_guestbook, profile_showcase, profile_layout_presets, profile_layouts;
CREATE TABLE IF NOT EXISTS profile_layouts (
    user_id        UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    template       TEXT NOT NULL DEFAULT 'minimal' CHECK (char_length(template) <= 24),
    sections       JSONB NOT NULL DEFAULT '[]'::jsonb,
    guestbook_mode TEXT NOT NULL DEFAULT 'friends' CHECK (guestbook_mode IN ('off', 'friends', 'everyone')),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS profile_layout_presets (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 40),
    template   TEXT NOT NULL CHECK (char_length(template) <= 24),
    sections   JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS profile_layout_presets_user_idx ON profile_layout_presets (user_id);

CREATE TABLE IF NOT EXISTS profile_showcase (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('sticker', 'emoji', 'image', 'gif', 'link')),
    ref_url    TEXT NOT NULL CHECK (char_length(ref_url) <= 300),
    title      TEXT CHECK (char_length(title) <= 40),
    position   INT NOT NULL DEFAULT 0,
    pinned     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS profile_showcase_user_idx ON profile_showcase (user_id, position);

CREATE TABLE IF NOT EXISTS profile_guestbook (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body       TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 300),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS profile_guestbook_owner_idx ON profile_guestbook (owner_id, created_at DESC);

CREATE TABLE IF NOT EXISTS user_achievements (
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement TEXT NOT NULL CHECK (char_length(achievement) <= 32),
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, achievement)
);
