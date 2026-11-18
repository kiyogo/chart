-- Spec 007: "Chính mình" stores notes to yourself as direct messages where the
-- sender is also the recipient. The original schema forbade that.
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_check;
