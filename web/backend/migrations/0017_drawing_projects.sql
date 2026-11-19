-- Spec 010: private drawing / animation projects (every layer and frame kept for re-editing).
-- Rollback: DROP TABLE drawing_projects;
CREATE TABLE IF NOT EXISTS drawing_projects (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    kind        TEXT NOT NULL DEFAULT 'free' CHECK (kind IN ('free', 'avatar', 'cover', 'sticker', 'emoji', 'decoration', 'nameplate', 'effect')),
    width       INT NOT NULL CHECK (width BETWEEN 16 AND 1024),
    height      INT NOT NULL CHECK (height BETWEEN 16 AND 1024),
    frame_count INT NOT NULL CHECK (frame_count BETWEEN 1 AND 60),
    layer_count INT NOT NULL CHECK (layer_count BETWEEN 1 AND 12),
    data        BYTEA NOT NULL CHECK (octet_length(data) <= 6291456),
    thumb       BYTEA CHECK (thumb IS NULL OR octet_length(thumb) <= 262144),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS drawing_projects_owner_idx ON drawing_projects (owner_id, updated_at DESC);
