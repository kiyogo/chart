ALTER TABLE call_sessions
    ADD COLUMN IF NOT EXISTS meet_code VARCHAR(32) UNIQUE,
    ADD COLUMN IF NOT EXISTS meet_title VARCHAR(120);

CREATE INDEX IF NOT EXISTS call_sessions_meet_code_idx
    ON call_sessions (meet_code)
    WHERE meet_code IS NOT NULL;
