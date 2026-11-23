-- Spec 110: Ultra Media Compression, On-Device AI Restoration, S3 Multipart, Safe Dedup & AV1 E2EE Streaming
-- 100% additive migration preserving ALL existing users, messages, E2EE keys, attachments, and media_objects.

-- 1. Extend media_objects with BLAKE3, perceptual PDQ hash, codec telemetry, quality SSIM, and GC grace/pin guards
ALTER TABLE media_objects
    ADD COLUMN IF NOT EXISTS blake3 BYTEA CHECK (blake3 IS NULL OR octet_length(blake3) = 32),
    ADD COLUMN IF NOT EXISTS perceptual_hash BYTEA CHECK (perceptual_hash IS NULL OR octet_length(perceptual_hash) = 32),
    ADD COLUMN IF NOT EXISTS perceptual_quality SMALLINT,
    ADD COLUMN IF NOT EXISTS caption_band_hash BIGINT,
    ADD COLUMN IF NOT EXISTS compression_codec TEXT,
    ADD COLUMN IF NOT EXISTS compression_ratio REAL,
    ADD COLUMN IF NOT EXISTS quality_ssim REAL,
    ADD COLUMN IF NOT EXISTS media_class TEXT,
    ADD COLUMN IF NOT EXISTS ai_recommended BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS gc_eligible_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS media_objects_blake3_idx
    ON media_objects (blake3) WHERE blake3 IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_objects_gc_idx
    ON media_objects (gc_eligible_at) WHERE ref_count = 0 AND is_pinned = FALSE;

-- 2. Privacy-safe Deduplication Index (STRICTLY non-E2EE shared scopes ONLY — DB CHECK enforces is_e2ee = FALSE!)
CREATE TABLE IF NOT EXISTS media_dedup_indexes (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_id          UUID NOT NULL REFERENCES media_objects(id) ON DELETE CASCADE,
    scope_class        TEXT NOT NULL CHECK (scope_class IN ('shared_non_e2ee')),
    is_e2ee            BOOLEAN NOT NULL DEFAULT FALSE CHECK (is_e2ee = FALSE),
    blake3             BYTEA NOT NULL CHECK (octet_length(blake3) = 32),
    sha256             BYTEA NOT NULL CHECK (octet_length(sha256) = 32),
    perceptual_hash    BYTEA CHECK (perceptual_hash IS NULL OR octet_length(perceptual_hash) = 32),
    perceptual_quality SMALLINT,
    caption_band_hash  BIGINT,
    video_fingerprint  JSONB,
    width              INT,
    height             INT,
    byte_size          BIGINT NOT NULL CHECK (byte_size >= 0),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (scope_class, blake3)
);

CREATE INDEX IF NOT EXISTS media_dedup_indexes_sha_idx ON media_dedup_indexes (scope_class, sha256);
CREATE INDEX IF NOT EXISTS media_dedup_indexes_pdq_idx ON media_dedup_indexes (scope_class, perceptual_hash) WHERE perceptual_hash IS NOT NULL;

-- 3. Authenticated E2EE Video/Audio Streaming Manifests & ABR Variants
CREATE TABLE IF NOT EXISTS media_chunk_manifests (
    ref_id               UUID PRIMARY KEY REFERENCES media_references(id) ON DELETE CASCADE,
    version              SMALLINT NOT NULL DEFAULT 3,
    mime_type            TEXT NOT NULL,
    codec                TEXT NOT NULL,
    duration_ms          BIGINT NOT NULL DEFAULT 0,
    is_e2ee              BOOLEAN NOT NULL DEFAULT TRUE,
    sealed_manifest_b64  TEXT,
    variants_json        JSONB NOT NULL DEFAULT '[]'::jsonb,
    total_segments       INT NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS media_chunks (
    ref_id        UUID NOT NULL REFERENCES media_references(id) ON DELETE CASCADE,
    variant_id    TEXT NOT NULL DEFAULT 'default',
    chunk_index   INT NOT NULL,
    byte_offset   BIGINT NOT NULL CHECK (byte_offset >= 0),
    cipher_len    INT NOT NULL CHECK (cipher_len > 0),
    plain_len     INT NOT NULL CHECK (plain_len >= 0),
    start_ms      BIGINT NOT NULL DEFAULT 0,
    duration_ms   INT NOT NULL DEFAULT 0,
    is_keyframe   BOOLEAN NOT NULL DEFAULT FALSE,
    blake3        BYTEA CHECK (blake3 IS NULL OR octet_length(blake3) = 32),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (ref_id, variant_id, chunk_index)
);

-- 4. Extend device_storage_policies with Server-First durable DM, AI enhancement, media quality & data saver controls
ALTER TABLE device_storage_policies
    ADD COLUMN IF NOT EXISTS media_quality_mode TEXT NOT NULL DEFAULT 'ultra_optimized'
        CHECK (media_quality_mode IN ('ultra_optimized', 'data_saver', 'original')),
    ADD COLUMN IF NOT EXISTS ai_enhancement_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS data_saver_mode BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS auto_cache_cleanup BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS server_durable_dm BOOLEAN NOT NULL DEFAULT TRUE;
