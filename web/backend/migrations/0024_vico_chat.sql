-- Spec 016: Standalone Vico AI Chat Conversations & Optional User API Config
CREATE TABLE IF NOT EXISTS vico_conversations (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      TEXT NOT NULL DEFAULT 'Cuộc trò chuyện mới' CHECK (char_length(title) BETWEEN 1 AND 120),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS vico_conversations_user_idx
    ON vico_conversations (user_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS vico_messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES vico_conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 16000),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS vico_messages_conv_idx
    ON vico_messages (conversation_id, created_at ASC);

CREATE TABLE IF NOT EXISTS vico_user_configs (
    user_id      UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    api_endpoint TEXT,
    api_key      TEXT,
    model_name   TEXT NOT NULL DEFAULT 'gpt-4o-mini',
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
