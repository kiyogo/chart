-- Spec 168: Registry for personal library / wallpaper / cover / decoration media
-- uploaded via POST /api/media/upload so GET /uploads/media/{file_name} can
-- authorize authenticated in-app reads without weakening chat attachment ACLs.
CREATE TABLE IF NOT EXISTS library_media (
    file_name   TEXT PRIMARY KEY,
    uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mime_type   TEXT NOT NULL,
    size_bytes  BIGINT NOT NULL CHECK (size_bytes > 0),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_library_media_uploader_created
    ON library_media (uploader_id, created_at DESC);
