-- Spec 027: .kdata Encrypted Backup, Strong Export Re-Authentication, Locked Unencrypted ZIP & Cloud Sync/Recovery

CREATE TABLE IF NOT EXISTS user_export_credentials (
    user_id      UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_phc TEXT NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS passkeys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   TEXT NOT NULL UNIQUE,
    public_key_spki BYTEA NOT NULL,
    sign_count      BIGINT NOT NULL DEFAULT 0,
    label           TEXT NOT NULL DEFAULT 'Thiết bị tin cậy',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS passkeys_user_idx ON passkeys (user_id);

CREATE TABLE IF NOT EXISTS passkey_challenges (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_b64 TEXT NOT NULL,
    purpose       TEXT NOT NULL CHECK (purpose IN ('register', 'reauth')),
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reauth_attempts (
    user_id           UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    failed_count      INT NOT NULL DEFAULT 0,
    window_started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until      TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS export_authorizations (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash        BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    scope             TEXT NOT NULL CHECK (scope IN ('kdata', 'zip_encrypted', 'zip_unencrypted')),
    reauth_method     TEXT NOT NULL,
    risk_acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    preview_viewed    BOOLEAN NOT NULL DEFAULT FALSE,
    step_confirmed    BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at        TIMESTAMPTZ NOT NULL,
    used_at           TIMESTAMPTZ,
    revoked_at        TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS export_auth_user_idx ON export_authorizations (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS backup_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       UUID REFERENCES devices(id) ON DELETE SET NULL,
    job_kind        TEXT NOT NULL CHECK (job_kind IN ('export', 'import', 'cloud_backup', 'cloud_restore')),
    format          TEXT NOT NULL CHECK (format IN ('kdata', 'zip_aes256', 'zip_unencrypted', 'kchunk_v1')),
    status          TEXT NOT NULL DEFAULT 'preparing' CHECK (status IN ('preparing', 'running', 'paused', 'completed', 'verified', 'failed', 'cancelled')),
    total_bytes     BIGINT NOT NULL DEFAULT 0,
    processed_bytes BIGINT NOT NULL DEFAULT 0,
    total_files     INT NOT NULL DEFAULT 0,
    processed_files INT NOT NULL DEFAULT 0,
    missing_files   INT NOT NULL DEFAULT 0,
    speed_bps       BIGINT NOT NULL DEFAULT 0,
    checksum_sha256 TEXT,
    honest_notice   TEXT,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS backup_jobs_user_idx ON backup_jobs (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS cloud_connections (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL CHECK (provider IN ('google_drive', 'onedrive', 'dropbox', 'nextcloud', 'webdav', 's3', 'r2', 'minio', 'nas', 'icloud')),
    account_label     TEXT NOT NULL,
    remote_account    TEXT,
    config_enc        BYTEA NOT NULL,
    status            TEXT NOT NULL DEFAULT 'connected' CHECK (status IN ('connected', 'auth_error', 'disconnected', 'needs_admin_config')),
    quota_total_bytes BIGINT,
    quota_used_bytes  BIGINT,
    last_error        TEXT,
    last_backup_at    TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS cloud_connections_user_idx ON cloud_connections (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS cloud_oauth_states (
    state             TEXT PRIMARY KEY,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL,
    code_verifier_enc BYTEA NOT NULL,
    redirect_uri      TEXT NOT NULL,
    scope_used        TEXT NOT NULL,
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cloud_backup_policies (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    enabled              BOOLEAN NOT NULL DEFAULT FALSE,
    connection_id        UUID REFERENCES cloud_connections(id) ON DELETE SET NULL,
    schedule_mode        TEXT NOT NULL DEFAULT 'manual' CHECK (schedule_mode IN ('manual', 'daily', 'weekly', 'on_change')),
    wifi_only            BOOLEAN NOT NULL DEFAULT TRUE,
    charging_only        BOOLEAN NOT NULL DEFAULT FALSE,
    bandwidth_limit_kbps INT NOT NULL DEFAULT 0 CHECK (bandwidth_limit_kbps >= 0),
    storage_limit_mb     INT NOT NULL DEFAULT 0 CHECK (storage_limit_mb >= 0),
    retain_snapshots     INT NOT NULL DEFAULT 5 CHECK (retain_snapshots BETWEEN 1 AND 50),
    paused               BOOLEAN NOT NULL DEFAULT FALSE,
    last_run_at          TIMESTAMPTZ,
    next_run_at          TIMESTAMPTZ,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cloud_snapshots (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    connection_id        UUID NOT NULL REFERENCES cloud_connections(id) ON DELETE CASCADE,
    snapshot_seq         BIGINT NOT NULL,
    parent_snapshot_id   UUID REFERENCES cloud_snapshots(id) ON DELETE SET NULL,
    remote_manifest_key  TEXT NOT NULL,
    manifest_sha256      TEXT NOT NULL,
    total_plain_bytes    BIGINT NOT NULL DEFAULT 0,
    uploaded_delta_bytes BIGINT NOT NULL DEFAULT 0,
    file_count           INT NOT NULL DEFAULT 0,
    status               TEXT NOT NULL DEFAULT 'uploading' CHECK (status IN ('uploading', 'verifying', 'verified', 'failed', 'deleted')),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_at          TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS cloud_snapshots_conn_idx ON cloud_snapshots (user_id, connection_id, snapshot_seq DESC);

CREATE TABLE IF NOT EXISTS cloud_chunks (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    connection_id UUID NOT NULL REFERENCES cloud_connections(id) ON DELETE CASCADE,
    chunk_id_hex  TEXT NOT NULL,
    remote_key    TEXT NOT NULL,
    plain_sha256  TEXT NOT NULL,
    cipher_sha256 TEXT NOT NULL,
    byte_size     BIGINT NOT NULL CHECK (byte_size >= 0),
    ref_count     INT NOT NULL DEFAULT 1 CHECK (ref_count >= 0),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, connection_id, chunk_id_hex)
);

CREATE TABLE IF NOT EXISTS cloud_snapshot_chunks (
    snapshot_id UUID NOT NULL REFERENCES cloud_snapshots(id) ON DELETE CASCADE,
    chunk_id    UUID NOT NULL REFERENCES cloud_chunks(id) ON DELETE CASCADE,
    PRIMARY KEY (snapshot_id, chunk_id)
);
