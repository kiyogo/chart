-- Spec 111: Hyper-Deduplication Engine (FastCDC v2020 + 16-Band PDQ MIH O(1) + Convergent MLE PoW + Audio/Video FP + Cross-Chat Blurry-Copy AI Recovery)
-- 100% additive migration preserving ALL existing users, E2EE keys, messages, attachments, and media_objects.

-- 1. Extend media_objects with canonical AI quality profile, original dimensions, Color-Moment/Aspect guards, PoW digest & CDC flag
ALTER TABLE media_objects
    ADD COLUMN IF NOT EXISTS orig_profile_json TEXT,
    ADD COLUMN IF NOT EXISTS orig_width INT,
    ADD COLUMN IF NOT EXISTS orig_height INT,
    ADD COLUMN IF NOT EXISTS color_moment_hash BIGINT,
    ADD COLUMN IF NOT EXISTS aspect_ratio_milli SMALLINT,
    ADD COLUMN IF NOT EXISTS pow_digest BYTEA CHECK (pow_digest IS NULL OR octet_length(pow_digest) = 32),
    ADD COLUMN IF NOT EXISTS is_cdc_assembled BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS media_objects_blake3_pow_idx
    ON media_objects (blake3, pow_digest)
    WHERE blake3 IS NOT NULL AND pow_digest IS NOT NULL;

-- 2. Extend media_dedup_indexes with original pre-AVIF BLAKE3, triple-guard metadata, audio FP, and canonical AI profile
ALTER TABLE media_dedup_indexes
    ADD COLUMN IF NOT EXISTS orig_blake3 BYTEA CHECK (orig_blake3 IS NULL OR octet_length(orig_blake3) = 32),
    ADD COLUMN IF NOT EXISTS color_moment_hash BIGINT,
    ADD COLUMN IF NOT EXISTS aspect_ratio_milli SMALLINT,
    ADD COLUMN IF NOT EXISTS audio_fingerprint JSONB,
    ADD COLUMN IF NOT EXISTS orig_profile_json TEXT,
    ADD COLUMN IF NOT EXISTS orig_width INT,
    ADD COLUMN IF NOT EXISTS orig_height INT,
    ADD COLUMN IF NOT EXISTS ai_recommended BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS media_dedup_indexes_orig_b3_idx
    ON media_dedup_indexes (scope_class, orig_blake3)
    WHERE orig_blake3 IS NOT NULL;

-- 3. O(1) 16-Band Multi-Index Hashing (MIH) for Meta PDQ 256-bit DCT (Images, Memes & Video Anchor Keyframes)
CREATE TABLE IF NOT EXISTS media_pdq_mih_buckets (
    band_idx    SMALLINT NOT NULL CHECK (band_idx >= 0 AND band_idx < 16),
    band_val    INT NOT NULL,
    scope_class TEXT NOT NULL CHECK (scope_class IN ('shared_non_e2ee', 'video_shared')),
    object_id   UUID NOT NULL REFERENCES media_objects(id) ON DELETE CASCADE,
    PRIMARY KEY (band_idx, band_val, scope_class, object_id)
);

CREATE INDEX IF NOT EXISTS media_pdq_mih_buckets_obj_idx
    ON media_pdq_mih_buckets (object_id);

-- 4. FastCDC v2020 Content-Defined Sub-File Chunk Store & Assembly Recipes (Large Files, Docs, Archives, Videos)
CREATE TABLE IF NOT EXISTS media_cdc_chunks (
    chunk_blake3    BYTEA PRIMARY KEY CHECK (octet_length(chunk_blake3) = 32),
    storage_backend TEXT NOT NULL,
    storage_key     TEXT NOT NULL,
    byte_size       INT NOT NULL CHECK (byte_size > 0),
    ref_count       INT NOT NULL DEFAULT 1 CHECK (ref_count >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS media_cdc_recipes (
    object_id    UUID NOT NULL REFERENCES media_objects(id) ON DELETE CASCADE,
    chunk_index  INT NOT NULL CHECK (chunk_index >= 0),
    chunk_blake3 BYTEA NOT NULL REFERENCES media_cdc_chunks(chunk_blake3),
    byte_offset  BIGINT NOT NULL CHECK (byte_offset >= 0),
    byte_size    INT NOT NULL CHECK (byte_size > 0),
    PRIMARY KEY (object_id, chunk_index)
);

CREATE INDEX IF NOT EXISTS media_cdc_recipes_chunk_idx
    ON media_cdc_recipes (chunk_blake3);
