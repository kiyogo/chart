-- Spec 009: avatar motion, richer profile details with visibility, status expiry,
-- per-account settings sync and private contact notes.
-- Additive only. Rollback:
--   ALTER TABLE users DROP COLUMN avatar_motion, DROP COLUMN status_expires_at, DROP COLUMN pronouns,
--     DROP COLUMN location, DROP COLUMN website, DROP COLUMN occupation, DROP COLUMN interests,
--     DROP COLUMN detail_visibility;
--   DROP TABLE user_settings; DROP TABLE contact_notes;

ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_motion JSONB;
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_expires_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS pronouns TEXT CHECK (char_length(pronouns) <= 40);
ALTER TABLE users ADD COLUMN IF NOT EXISTS location TEXT CHECK (char_length(location) <= 64);
ALTER TABLE users ADD COLUMN IF NOT EXISTS website TEXT CHECK (char_length(website) <= 200);
ALTER TABLE users ADD COLUMN IF NOT EXISTS occupation TEXT CHECK (char_length(occupation) <= 64);
ALTER TABLE users ADD COLUMN IF NOT EXISTS interests TEXT[] NOT NULL DEFAULT '{}'::text[];
ALTER TABLE users ADD COLUMN IF NOT EXISTS detail_visibility JSONB NOT NULL DEFAULT '{}'::jsonb;
CREATE INDEX IF NOT EXISTS users_status_expiry_idx ON users (status_expires_at) WHERE status_expires_at IS NOT NULL;

-- Existing one-word avatar effects become motion objects with default parameters.
UPDATE users SET avatar_motion = jsonb_build_object('effect', avatar_effect)
WHERE avatar_motion IS NULL AND avatar_effect IS NOT NULL AND avatar_effect <> 'none';

CREATE TABLE IF NOT EXISTS user_settings (
    user_id    UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    data       JSONB NOT NULL DEFAULT '{}'::jsonb,
    version    BIGINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS contact_notes (
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    nickname   TEXT CHECK (char_length(nickname) <= 40),
    note       TEXT CHECK (char_length(note) <= 500),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, target_id),
    CHECK (owner_id <> target_id)
);
