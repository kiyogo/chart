-- Users. A row is created on first Google login; username stays NULL until the user picks one.
CREATE TABLE users (
    id            UUID PRIMARY KEY,
    google_sub    TEXT NOT NULL UNIQUE CHECK (char_length(google_sub) BETWEEN 1 AND 255),
    username      TEXT CHECK (char_length(username) BETWEEN 2 AND 32),
    -- C collation lets the unique btree index serve `LIKE 'prefix%'` searches.
    username_norm TEXT COLLATE "C" UNIQUE CHECK (char_length(username_norm) BETWEEN 2 AND 64),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((username IS NULL) = (username_norm IS NULL))
);

-- Sessions store only a keyed hash (HMAC-SHA-256) of the random cookie token.
CREATE TABLE sessions (
    token_hash BYTEA PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_expires_idx ON sessions (expires_at);

-- Public keys only (uncompressed SEC1 P-256 points). One immutable key set per account.
CREATE TABLE device_keys (
    user_id      UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    ecdh_public  BYTEA NOT NULL CHECK (octet_length(ecdh_public) = 65),
    ecdsa_public BYTEA NOT NULL CHECK (octet_length(ecdsa_public) = 65),
    fingerprint  BYTEA NOT NULL CHECK (octet_length(fingerprint) = 32),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE friend_requests (
    id          UUID PRIMARY KEY,
    sender_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('pending', 'accepted')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (sender_id <> receiver_id),
    UNIQUE (sender_id, receiver_id)
);
CREATE INDEX friend_requests_receiver_pending_idx
    ON friend_requests (receiver_id, created_at) WHERE status = 'pending';

-- One row per friendship, stored with user_a < user_b.
CREATE TABLE friendships (
    user_a     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_b     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_a, user_b),
    CHECK (user_a < user_b)
);
CREATE INDEX friendships_user_b_idx ON friendships (user_b);

-- Messages hold only end-to-end encrypted envelopes. id is the client-generated
-- message id (idempotency key); seq gives a stable order for cursor pagination.
CREATE TABLE messages (
    id                 UUID PRIMARY KEY,
    seq                BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    sender_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_envelope BYTEA NOT NULL CHECK (octet_length(recipient_envelope) BETWEEN 255 AND 10000),
    sender_envelope    BYTEA NOT NULL CHECK (octet_length(sender_envelope) BETWEEN 255 AND 10000),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (sender_id <> receiver_id)
);
CREATE INDEX messages_conversation_idx
    ON messages (LEAST(sender_id, receiver_id), GREATEST(sender_id, receiver_id), seq);
