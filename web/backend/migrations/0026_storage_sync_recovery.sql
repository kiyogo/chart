-- Spec 022: Storage Abstraction, Adaptive Media Optimization, Multi-Device Sync & Peer Recovery
-- Additive migration preserving all legacy users, sessions, messages, groups, channels, and attachments.

-- 1. Persistent per-device identity and cryptographic trust hierarchy
CREATE TABLE IF NOT EXISTS devices (
    id                 UUID PRIMARY KEY,
    user_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_device_id   TEXT CHECK (char_length(client_device_id) BETWEEN 8 AND 128),
    device_name        TEXT,
    device_label       TEXT,
    platform           TEXT NOT NULL DEFAULT 'web' CHECK (platform IN ('web', 'desktop', 'mobile', 'cli')),
    ecdh_public        BYTEA CHECK (ecdh_public IS NULL OR octet_length(ecdh_public) = 65),
    ecdsa_public       BYTEA CHECK (ecdsa_public IS NULL OR octet_length(ecdsa_public) = 65),
    key_fingerprint    BYTEA CHECK (key_fingerprint IS NULL OR octet_length(key_fingerprint) = 32),
    trust_state        TEXT NOT NULL DEFAULT 'trusted' CHECK (trust_state IN ('trusted', 'pending', 'revoked')),
    approved_by_device UUID REFERENCES devices(id) ON DELETE SET NULL,
    approved_at        TIMESTAMPTZ,
    revoked_at         TIMESTAMPTZ,
    last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, client_device_id)
);
CREATE INDEX IF NOT EXISTS devices_user_trust_idx ON devices (user_id, trust_state, last_seen_at DESC);

ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS device_id UUID REFERENCES devices(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS sessions_device_idx ON sessions (device_id) WHERE device_id IS NOT NULL;

-- 2. New-device login approval workflow
CREATE TABLE IF NOT EXISTS device_login_requests (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requesting_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    requesting_session_id UUID,
    verification_code    TEXT NOT NULL CHECK (char_length(verification_code) = 6),
    user_agent           TEXT,
    ip_masked            TEXT,
    status               TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'expired')),
    wrapped_sync_key     TEXT,
    approver_device_id   UUID REFERENCES devices(id) ON DELETE SET NULL,
    expires_at           TIMESTAMPTZ NOT NULL,
    decided_at           TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS device_login_requests_user_status_idx
    ON device_login_requests (user_id, status, expires_at DESC);

-- 3. Client-side E2EE key recovery vault (server stores ciphertext only; zero plaintext key exposure)
CREATE TABLE IF NOT EXISTS user_key_backups (
    user_id          UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    wrapped_keys_b64 TEXT NOT NULL,
    kdf_salt_b64     TEXT NOT NULL,
    kdf_iterations   INT NOT NULL DEFAULT 210000 CHECK (kdf_iterations >= 10000),
    key_fingerprint  BYTEA NOT NULL CHECK (octet_length(key_fingerprint) = 32),
    version          INT NOT NULL DEFAULT 1,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 4. Multi-device incremental cursor tracking & per-user storage/sync preferences
CREATE TABLE IF NOT EXISTS device_sync_cursors (
    device_id       UUID PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dm_seq          BIGINT NOT NULL DEFAULT 0,
    group_seq       BIGINT NOT NULL DEFAULT 0,
    channel_seq     BIGINT NOT NULL DEFAULT 0,
    recovery_seq    BIGINT NOT NULL DEFAULT 0,
    phase_a_done    BOOLEAN NOT NULL DEFAULT FALSE,
    phase_b_done    BOOLEAN NOT NULL DEFAULT FALSE,
    last_sync_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS device_storage_policies (
    user_id                    UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    cache_limit_mb             INT NOT NULL DEFAULT 1024 CHECK (cache_limit_mb >= 0),
    auto_download_mode         TEXT NOT NULL DEFAULT 'wifi' CHECK (auto_download_mode IN ('always', 'wifi', 'manual')),
    dm_relay_ttl_hours         INT NOT NULL DEFAULT 168 CHECK (dm_relay_ttl_hours BETWEEN 1 AND 720),
    dm_encrypted_backup        BOOLEAN NOT NULL DEFAULT FALSE,
    require_device_approval    BOOLEAN NOT NULL DEFAULT TRUE,
    prefer_original_upload     BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 5. Privacy-safe Content-Addressable Storage (CAS) objects and rendition variants
CREATE TABLE IF NOT EXISTS media_objects (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cas_scope          TEXT NOT NULL,
    sha256             BYTEA NOT NULL CHECK (octet_length(sha256) = 32),
    plain_sha256       BYTEA CHECK (plain_sha256 IS NULL OR octet_length(plain_sha256) = 32),
    storage_backend    TEXT NOT NULL CHECK (storage_backend IN ('local_cas', 'legacy', 'temp_relay', 's3_ready', 'encrypted_backup', 'peer_only')),
    storage_key        TEXT NOT NULL,
    mime_type          TEXT NOT NULL,
    file_ext           TEXT NOT NULL,
    byte_size          BIGINT NOT NULL CHECK (byte_size >= 0),
    original_byte_size BIGINT NOT NULL CHECK (original_byte_size >= 0),
    width              INT,
    height             INT,
    duration_ms        INT,
    codec              TEXT,
    is_animated        BOOLEAN NOT NULL DEFAULT FALSE,
    ref_count          INT NOT NULL DEFAULT 1 CHECK (ref_count >= 0),
    variant_of         UUID REFERENCES media_objects(id) ON DELETE SET NULL,
    variant_kind       TEXT NOT NULL DEFAULT 'original' CHECK (variant_kind IN ('original', 'optimized', 'thumb', 'poster')),
    expires_at         TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (cas_scope, variant_kind)
);
CREATE INDEX IF NOT EXISTS media_objects_sha_idx ON media_objects (sha256);
CREATE INDEX IF NOT EXISTS media_objects_expires_idx ON media_objects (expires_at) WHERE expires_at IS NOT NULL;

-- 6. Conversation-scoped authorized media references (links messages & RBAC scope to CAS/Relay/Peer objects)
CREATE TABLE IF NOT EXISTS media_references (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope_type           TEXT NOT NULL CHECK (scope_type IN ('dm', 'group', 'channel', 'avatar', 'banner', 'library', 'sticker')),
    scope_id             UUID,
    message_id           UUID,
    object_id            UUID REFERENCES media_objects(id) ON DELETE SET NULL,
    optimized_object_id  UUID REFERENCES media_objects(id) ON DELETE SET NULL,
    thumb_object_id      UUID REFERENCES media_objects(id) ON DELETE SET NULL,
    original_filename    TEXT NOT NULL,
    mime_type            TEXT NOT NULL,
    byte_size            BIGINT NOT NULL CHECK (byte_size >= 0),
    sha256_plain         BYTEA CHECK (sha256_plain IS NULL OR octet_length(sha256_plain) = 32),
    sha256_cipher        BYTEA CHECK (sha256_cipher IS NULL OR octet_length(sha256_cipher) = 32),
    forward_from_ref_id  UUID REFERENCES media_references(id) ON DELETE SET NULL,
    storage_tier         TEXT NOT NULL CHECK (storage_tier IN ('dm_client_preferred', 'server_durable', 'temp_relay', 'encrypted_backup')),
    availability_state   TEXT NOT NULL CHECK (availability_state IN (
        'local_only', 'pending_delivery', 'available_on_peer', 'replicated_multi_device',
        'backed_up_encrypted', 'server_durable', 'restored_from_peer', 'recovery_pending',
        'queued_offline_peer', 'denied_by_peer', 'no_remaining_copies', 'deleted_for_everyone'
    )),
    min_safe_copies      INT NOT NULL DEFAULT 2,
    deleted_for_all      BOOLEAN NOT NULL DEFAULT FALSE,
    relay_expires_at     TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS media_references_scope_idx ON media_references (scope_type, scope_id, created_at DESC);
CREATE INDEX IF NOT EXISTS media_references_owner_idx ON media_references (owner_id, created_at DESC);
CREATE INDEX IF NOT EXISTS media_references_msg_idx ON media_references (message_id) WHERE message_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_references_relay_idx ON media_references (relay_expires_at) WHERE storage_tier = 'temp_relay';

-- 7. Per-device verified copy inventory
CREATE TABLE IF NOT EXISTS media_device_locations (
    ref_id            UUID NOT NULL REFERENCES media_references(id) ON DELETE CASCADE,
    device_id         UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    has_original      BOOLEAN NOT NULL DEFAULT TRUE,
    has_optimized     BOOLEAN NOT NULL DEFAULT FALSE,
    local_tier        TEXT NOT NULL DEFAULT 'opfs' CHECK (local_tier IN ('opfs', 'indexeddb', 'cache_api', 'native_fs', 'server_relay')),
    verified_sha256   BYTEA CHECK (verified_sha256 IS NULL OR octet_length(verified_sha256) = 32),
    last_verified_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (ref_id, device_id)
);
CREATE INDEX IF NOT EXISTS media_device_locations_user_idx ON media_device_locations (user_id, ref_id);

-- 8. Resumable chunked / multipart transfer state
CREATE TABLE IF NOT EXISTS transfer_sessions (
    id                UUID PRIMARY KEY,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id         UUID REFERENCES devices(id) ON DELETE SET NULL,
    scope_type        TEXT NOT NULL CHECK (scope_type IN ('dm', 'group', 'channel', 'recovery_relay', 'encrypted_backup')),
    scope_id          UUID,
    original_filename TEXT NOT NULL,
    mime_type         TEXT NOT NULL,
    total_size        BIGINT NOT NULL CHECK (total_size > 0),
    chunk_size        INT NOT NULL CHECK (chunk_size BETWEEN 4096 AND 16777216),
    total_chunks      INT NOT NULL CHECK (total_chunks > 0),
    expected_sha256   BYTEA CHECK (expected_sha256 IS NULL OR octet_length(expected_sha256) = 32),
    plain_sha256      BYTEA CHECK (plain_sha256 IS NULL OR octet_length(plain_sha256) = 32),
    is_e2ee           BOOLEAN NOT NULL DEFAULT FALSE,
    preserve_original BOOLEAN NOT NULL DEFAULT TRUE,
    status            TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'aborted', 'expired')),
    resulting_ref_id  UUID REFERENCES media_references(id) ON DELETE SET NULL,
    s3_upload_id      TEXT,
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transfer_chunks (
    transfer_id   UUID NOT NULL REFERENCES transfer_sessions(id) ON DELETE CASCADE,
    chunk_index   INT NOT NULL,
    byte_size     INT NOT NULL CHECK (byte_size > 0),
    chunk_sha256  BYTEA NOT NULL CHECK (octet_length(chunk_sha256) = 32),
    s3_etag       TEXT,
    uploaded_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (transfer_id, chunk_index)
);

-- 9. Two-way DM Peer Recovery requests & permission controls
CREATE TABLE IF NOT EXISTS recovery_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ref_id              UUID NOT NULL REFERENCES media_references(id) ON DELETE CASCADE,
    requester_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requester_device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    target_user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_device_id    UUID REFERENCES devices(id) ON DELETE SET NULL,
    transport_mode      TEXT NOT NULL DEFAULT 'webrtc_p2p' CHECK (transport_mode IN ('webrtc_p2p', 'encrypted_relay', 'device_sync')),
    status              TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'queued_offline_peer', 'transferring', 'completed', 'denied', 'no_remaining_copies', 'expired')),
    relay_ref_id        UUID REFERENCES media_references(id) ON DELETE SET NULL,
    reason              TEXT,
    expires_at          TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS recovery_requests_target_idx ON recovery_requests (target_user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS recovery_requests_requester_idx ON recovery_requests (requester_user_id, status, created_at DESC);

ALTER TABLE conversation_prefs
    ADD COLUMN IF NOT EXISTS allow_peer_recovery BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS dm_encrypted_backup BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS media_retention_policy TEXT NOT NULL DEFAULT 'durable' CHECK (media_retention_policy IN ('durable', 'ephemeral', 'no_media')),
    ADD COLUMN IF NOT EXISTS media_max_mb INT NOT NULL DEFAULT 100 CHECK (media_max_mb BETWEEN 0 AND 4096);

-- 10. Storage migration tracker (Local CAS -> S3 / MinIO readiness)
CREATE TABLE IF NOT EXISTS storage_migration_jobs (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_backend   TEXT NOT NULL CHECK (target_backend IN ('s3_ready', 'local_cas')),
    dry_run          BOOLEAN NOT NULL DEFAULT TRUE,
    status           TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    scanned_objects  BIGINT NOT NULL DEFAULT 0,
    migrated_objects BIGINT NOT NULL DEFAULT 0,
    verified_objects BIGINT NOT NULL DEFAULT 0,
    failed_objects   BIGINT NOT NULL DEFAULT 0,
    last_error       TEXT,
    started_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at      TIMESTAMPTZ
);

-- 11. Extend attachments link to media_references for full backward compatibility + metrics
ALTER TABLE attachments
    ADD COLUMN IF NOT EXISTS ref_id UUID REFERENCES media_references(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS scope_type TEXT,
    ADD COLUMN IF NOT EXISTS scope_id UUID;
