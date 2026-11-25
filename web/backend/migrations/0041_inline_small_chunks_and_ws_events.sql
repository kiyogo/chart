-- Spec 165: Unified Database & Storage Optimization
-- 1. Small-chunk inline storage in PostgreSQL (strictly mutually exclusive with Object Storage — ZERO duplication!)
-- 2. Durable WebSocket Event Log for ordered HTTP gap recovery on reconnect or Valkey restart
-- 3. HTTP Idempotency Keys table for retry deduplication without duplicate messages/commits
-- 4. Covering partial indexes for PostgreSQL chunk & message lookup optimization

-- 1. Inline small chunks in PostgreSQL (only when storage_backend = 'pg_inline')
ALTER TABLE message_chunks
    ADD COLUMN IF NOT EXISTS inline_payload BYTEA;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_message_chunks_no_duplicate_payload'
    ) THEN
        ALTER TABLE message_chunks
            ADD CONSTRAINT chk_message_chunks_no_duplicate_payload
            CHECK (
                (storage_backend = 'pg_inline' AND inline_payload IS NOT NULL)
                OR
                (storage_backend <> 'pg_inline' AND inline_payload IS NULL)
            );
    END IF;
END $$;

-- Covering partial index for active chunk retrieval by scope & sequence range
CREATE INDEX IF NOT EXISTS idx_message_chunks_active_scope_seq
    ON message_chunks (scope_type, scope_id, owner_scope_key, seq_start ASC, seq_end ASC)
    WHERE superseded_at IS NULL;

-- Partial index for fast hot-to-cold lifecycle sweep
CREATE INDEX IF NOT EXISTS idx_message_chunks_cold_sweep
    ON message_chunks (storage_tier, last_accessed_at ASC)
    WHERE superseded_at IS NULL;

-- 2. Durable WebSocket Event Log (guarantees zero lost realtime/manifest/chunk/permission events
--    on WebSocket disconnect OR Valkey restart!)
CREATE TABLE IF NOT EXISTS ws_event_log (
    event_id    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type  TEXT NOT NULL,
    scope_type  TEXT,
    scope_id    TEXT,
    message_id  UUID,
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ws_event_log_user_event
    ON ws_event_log (user_id, event_id ASC);

CREATE INDEX IF NOT EXISTS idx_ws_event_log_created_at
    ON ws_event_log (created_at ASC);

-- 3. HTTP Idempotency Keys (prevents duplicate sends/commits across retries & flaky networks)
CREATE TABLE IF NOT EXISTS http_idempotency_keys (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL,
    method          TEXT NOT NULL,
    route_path      TEXT NOT NULL,
    status_code     SMALLINT NOT NULL,
    response_json   JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '24 hours'),
    PRIMARY KEY (user_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_http_idempotency_expires
    ON http_idempotency_keys (expires_at ASC);
