-- Spec 158: 15-Day Inactivity Cold Image Compression Lifecycle
-- Tracks last user access time on media_objects & media_references so images remain untouched
-- while actively accessed (< 15 days) and compress automatically to AVIF after 15 days idle.

ALTER TABLE media_objects
    ADD COLUMN IF NOT EXISTS last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS cold_compressed_at TIMESTAMPTZ;

UPDATE media_objects
SET last_accessed_at = COALESCE(created_at, now())
WHERE last_accessed_at > created_at + interval '1 minute';

CREATE INDEX IF NOT EXISTS media_objects_cold_idle_idx
    ON media_objects (last_accessed_at)
    WHERE variant_kind = 'original' AND is_pinned = FALSE AND cold_compressed_at IS NULL;

ALTER TABLE media_references
    ADD COLUMN IF NOT EXISTS last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now();

UPDATE media_references
SET last_accessed_at = COALESCE(updated_at, created_at, now())
WHERE last_accessed_at > created_at + interval '1 minute';
