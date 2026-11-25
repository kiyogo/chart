-- Spec 164: Incremental Zstd-22 Message Chunk Storage, Distributed Leases, OCC Manifests,
-- Append-Only Mutation/Tombstone Logs, Versioned Dictionaries, Encrypted Search Index Sync,
-- Hot/Warm/Cold Tiering, and Privacy-Safe Device Storage Telemetry.

-- 1. Extend device_storage_policies with adaptive storage mode & safe rollout flag
ALTER TABLE device_storage_policies
    ADD COLUMN IF NOT EXISTS storage_mode TEXT NOT NULL DEFAULT 'auto'
        CHECK (storage_mode IN ('auto', 'space_saver', 'high_performance', 'custom')),
    ADD COLUMN IF NOT EXISTS compression_rollout_enabled BOOLEAN NOT NULL DEFAULT TRUE;

-- 2. Per-conversation OCC Manifest Head (atomic compare-and-swap via manifest_version)
CREATE TABLE IF NOT EXISTS conversation_chunk_manifests (
    scope_type            TEXT NOT NULL CHECK (scope_type IN ('dm', 'group', 'channel')),
    scope_id              TEXT NOT NULL,
    owner_scope_key       TEXT NOT NULL, -- For DM: sorted pair or owner_id; for group/channel: 'shared'
    manifest_version      BIGINT NOT NULL DEFAULT 1,
    chunk_count           INTEGER NOT NULL DEFAULT 0,
    oldest_chunk_seq      BIGINT,
    newest_chunk_seq      BIGINT,
    last_mutation_seq     BIGINT NOT NULL DEFAULT 0,
    total_original_bytes  BIGINT NOT NULL DEFAULT 0,
    total_compressed_bytes BIGINT NOT NULL DEFAULT 0,
    total_encrypted_bytes BIGINT NOT NULL DEFAULT 0,
    sealed_manifest_b64   TEXT,          -- Optional E2EE-sealed manifest metadata for private DM scopes
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_type, scope_id, owner_scope_key)
);

-- 3. Immutable Message Chunks (Zstd Level 22 + E2EE for DMs; Hot -> Warm -> Cold tiering)
CREATE TABLE IF NOT EXISTS message_chunks (
    id                    UUID PRIMARY KEY,
    scope_type            TEXT NOT NULL CHECK (scope_type IN ('dm', 'group', 'channel')),
    scope_id              TEXT NOT NULL,
    owner_scope_key       TEXT NOT NULL,
    owner_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chunk_version         INTEGER NOT NULL DEFAULT 1,
    format_version        SMALLINT NOT NULL DEFAULT 1,
    access_epoch          INTEGER NOT NULL DEFAULT 1,
    compression_algo      TEXT NOT NULL DEFAULT 'zstd',
    compression_level     SMALLINT NOT NULL DEFAULT 22 CHECK (compression_level = 22),
    dictionary_id         INTEGER NOT NULL DEFAULT 0,
    original_size         BIGINT NOT NULL CHECK (original_size > 0),
    serialized_size       BIGINT NOT NULL CHECK (serialized_size > 0),
    compressed_size       BIGINT NOT NULL CHECK (compressed_size > 0),
    padded_size           BIGINT NOT NULL CHECK (padded_size > 0),
    encrypted_size        BIGINT NOT NULL CHECK (encrypted_size > 0),
    message_count         INTEGER NOT NULL CHECK (message_count > 0),
    seq_start             BIGINT NOT NULL,
    seq_end               BIGINT NOT NULL CHECK (seq_end >= seq_start),
    first_created_at      TIMESTAMPTZ NOT NULL,
    last_created_at       TIMESTAMPTZ NOT NULL,
    encryption_version    SMALLINT NOT NULL DEFAULT 1,
    key_version           INTEGER NOT NULL DEFAULT 1,
    is_e2ee               BOOLEAN NOT NULL DEFAULT TRUE,
    nonce_b64             TEXT NOT NULL DEFAULT '',
    plaintext_blake3_hex  TEXT NOT NULL,
    ciphertext_blake3_hex TEXT NOT NULL,
    storage_tier          TEXT NOT NULL DEFAULT 'warm' CHECK (storage_tier IN ('hot', 'warm', 'cold')),
    storage_backend       TEXT NOT NULL DEFAULT 'local_cas',
    storage_key           TEXT NOT NULL,
    sync_state            TEXT NOT NULL DEFAULT 'committed',
    sealed_meta_b64       TEXT,
    last_accessed_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    superseded_at         TIMESTAMPTZ,
    gc_eligible_at        TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS message_chunks_scope_seq_idx
    ON message_chunks (scope_type, scope_id, owner_scope_key, seq_start, seq_end)
    WHERE superseded_at IS NULL;

CREATE INDEX IF NOT EXISTS message_chunks_tier_access_idx
    ON message_chunks (storage_tier, last_accessed_at)
    WHERE superseded_at IS NULL;

CREATE INDEX IF NOT EXISTS message_chunks_gc_idx
    ON message_chunks (gc_eligible_at)
    WHERE gc_eligible_at IS NOT NULL;

-- 4. Append-only Message Mutations & Tombstones Log (edits/recalls/deletes/pins/reactions over chunks)
CREATE TABLE IF NOT EXISTS message_mutations (
    mutation_seq     BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    scope_type       TEXT NOT NULL CHECK (scope_type IN ('dm', 'group', 'channel')),
    scope_id         TEXT NOT NULL,
    message_id       UUID NOT NULL,
    chunk_id         UUID REFERENCES message_chunks(id) ON DELETE SET NULL,
    op_type          TEXT NOT NULL CHECK (op_type IN (
        'edit', 'delete', 'recall', 'pin', 'unpin', 'reaction_add', 'reaction_remove', 'bookmark', 'unbookmark'
    )),
    actor_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    payload_enc_b64  TEXT,
    payload_plain    JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS message_mutations_scope_seq_idx
    ON message_mutations (scope_type, scope_id, mutation_seq);

CREATE INDEX IF NOT EXISTS message_mutations_msg_idx
    ON message_mutations (message_id, mutation_seq);

-- 5. Distributed Leases with Monotonic Fencing Tokens & Crash-Recovery Checkpoints
CREATE TABLE IF NOT EXISTS compression_leases (
    lease_key        TEXT PRIMARY KEY,
    holder_device_id TEXT NOT NULL,
    holder_user_id   UUID REFERENCES users(id) ON DELETE CASCADE,
    fencing_token    BIGINT NOT NULL DEFAULT 1,
    status           TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'released')),
    checkpoint_json  JSONB NOT NULL DEFAULT '{}'::jsonb,
    leased_until     TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS compression_leases_expiry_idx
    ON compression_leases (leased_until);

-- 6. Versioned Zstd Dictionary Registry (public + E2EE private per-account/scope dictionaries)
CREATE TABLE IF NOT EXISTS compression_dictionaries (
    dict_id          INTEGER NOT NULL,
    owner_scope_key  TEXT NOT NULL, -- 'public' for built-in public dict, or '{user_id}:{scope_id}' for E2EE private dict
    version          INTEGER NOT NULL DEFAULT 1,
    is_e2ee          BOOLEAN NOT NULL DEFAULT FALSE,
    blake3_hex       TEXT NOT NULL,
    byte_size        INTEGER NOT NULL CHECK (byte_size > 0),
    storage_backend  TEXT NOT NULL DEFAULT 'local_cas',
    storage_key      TEXT NOT NULL,
    ref_count        BIGINT NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (dict_id, owner_scope_key, version)
);

-- 7. Encrypted Search Index Segments (strictly zero-knowledge ciphertext shards synced across devices)
CREATE TABLE IF NOT EXISTS encrypted_search_segments (
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope_key        TEXT NOT NULL,
    shard_index      INTEGER NOT NULL DEFAULT 0,
    segment_version  BIGINT NOT NULL DEFAULT 1,
    format_version   SMALLINT NOT NULL DEFAULT 1,
    key_version      INTEGER NOT NULL DEFAULT 1,
    doc_count        INTEGER NOT NULL DEFAULT 0,
    min_seq          BIGINT NOT NULL DEFAULT 0,
    max_seq          BIGINT NOT NULL DEFAULT 0,
    byte_size        INTEGER NOT NULL CHECK (byte_size > 0),
    blake3_hex       TEXT NOT NULL,
    storage_backend  TEXT NOT NULL DEFAULT 'local_cas',
    storage_key      TEXT NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, scope_key, shard_index)
);

CREATE INDEX IF NOT EXISTS encrypted_search_segments_user_idx
    ON encrypted_search_segments (user_id, updated_at DESC);

-- 8. Privacy-Safe Device Storage Telemetry (Section 32: ZERO message text, search queries, or keys)
CREATE TABLE IF NOT EXISTS device_storage_telemetry (
    user_id                  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id                TEXT NOT NULL,
    platform                 TEXT NOT NULL DEFAULT 'web',
    storage_mode             TEXT NOT NULL DEFAULT 'auto',
    compression_ratio        DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    compression_queue_size   BIGINT NOT NULL DEFAULT 0,
    compression_failures     BIGINT NOT NULL DEFAULT 0,
    decompression_failures   BIGINT NOT NULL DEFAULT 0,
    cache_hit_rate           DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    storage_usage_bytes      BIGINT NOT NULL DEFAULT 0,
    storage_budget_bytes     BIGINT NOT NULL DEFAULT 0,
    total_saved_bytes        BIGINT NOT NULL DEFAULT 0,
    indexed_message_count    BIGINT NOT NULL DEFAULT 0,
    last_search_latency_ms   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    last_sync_latency_ms     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    chunk_count              BIGINT NOT NULL DEFAULT 0,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, device_id)
);
