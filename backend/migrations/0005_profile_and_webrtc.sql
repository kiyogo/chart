-- Phase 5: Profile Customization & WebRTC Voice/Video Signaling

-- 1. Profile customization columns on users
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT CHECK (char_length(bio) <= 256);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_text TEXT CHECK (char_length(status_text) <= 128);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status_emoji TEXT CHECK (char_length(status_emoji) <= 32);
ALTER TABLE users ADD COLUMN IF NOT EXISTS presence TEXT NOT NULL DEFAULT 'offline' CHECK (presence IN ('online', 'idle', 'dnd', 'offline'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_banner_url TEXT;

-- 2. Voice Channel States (Live participants in voice channels)
CREATE TABLE IF NOT EXISTS voice_states (
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    self_mute  BOOLEAN NOT NULL DEFAULT FALSE,
    self_deaf  BOOLEAN NOT NULL DEFAULT FALSE,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, user_id)
);
CREATE INDEX IF NOT EXISTS voice_states_channel_idx ON voice_states (channel_id);

