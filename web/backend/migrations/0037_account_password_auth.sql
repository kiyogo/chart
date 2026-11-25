-- Spec 160: Account password authentication for password-only zero-friction multi-device sync
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS password_hash TEXT;
