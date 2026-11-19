-- Spec 013: allow "event" channels. Rollback: restore the previous list after converting event channels to text.
ALTER TABLE channels DROP CONSTRAINT IF EXISTS channels_channel_type_check;
ALTER TABLE channels ADD CONSTRAINT channels_channel_type_check CHECK (channel_type IN ('text', 'voice', 'announcement', 'forum', 'event'));
