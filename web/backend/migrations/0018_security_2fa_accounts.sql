-- Spec 011: two-step verification (TOTP + backup codes), login challenges and a security log.
-- Additive only. Rollback: DROP TABLE security_events, twofa_challenges, backup_codes;
--   ALTER TABLE users DROP COLUMN totp_secret_enc, DROP COLUMN totp_pending_enc, DROP COLUMN totp_enabled_at, DROP COLUMN totp_last_step;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret_enc BYTEA;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_pending_enc BYTEA;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_enabled_at TIMESTAMPTZ;
-- Last accepted 30-second step, so a code can't be replayed.
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_last_step BIGINT;

CREATE TABLE IF NOT EXISTS backup_codes (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash  BYTEA NOT NULL CHECK (octet_length(code_hash) = 32),
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS backup_codes_user_idx ON backup_codes (user_id);

CREATE TABLE IF NOT EXISTS twofa_challenges (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_agent TEXT,
    ip_masked  TEXT,
    attempts   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS security_events (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (char_length(kind) <= 40),
    device_label TEXT CHECK (char_length(device_label) <= 80),
    user_agent   TEXT CHECK (char_length(user_agent) <= 300),
    ip_masked    TEXT CHECK (char_length(ip_masked) <= 64),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS security_events_user_idx ON security_events (user_id, created_at DESC);
