-- Spec 058: E2EE v2 Identity Key History, Device Prekeys, Argon2id Backup Metadata & Multi-Device Envelopes

CREATE TABLE IF NOT EXISTS identity_key_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    ecdh_public BYTEA NOT NULL,
    ecdsa_public BYTEA NOT NULL,
    fingerprint BYTEA NOT NULL,
    action TEXT NOT NULL DEFAULT 'register',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_identity_key_history_user_created
    ON identity_key_history(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS device_prekeys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    signed_prekey_id INT NOT NULL DEFAULT 1,
    signed_prekey_pub BYTEA NOT NULL,
    signed_prekey_sig BYTEA NOT NULL,
    one_time_prekeys JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE user_key_backups
    ADD COLUMN IF NOT EXISTS kdf_alg TEXT NOT NULL DEFAULT 'pbkdf2-sha256-v1',
    ADD COLUMN IF NOT EXISTS kdf_mem_kib INT NOT NULL DEFAULT 19456,
    ADD COLUMN IF NOT EXISTS kdf_time_cost INT NOT NULL DEFAULT 2,
    ADD COLUMN IF NOT EXISTS kdf_parallelism INT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS root_key_wrapped_b64 TEXT;

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS device_envelopes JSONB;

-- Ensure no marketplace package is classified as a parallel built-in plugin
UPDATE plugin_packages SET is_builtin = false WHERE is_builtin = true;

