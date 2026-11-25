-- Spec 161: Allow 'completing' atomic state in transfer_sessions and clean up any legacy cold-compressed media_objects cas_scope collisions.
ALTER TABLE transfer_sessions
    DROP CONSTRAINT IF EXISTS transfer_sessions_status_check;

ALTER TABLE transfer_sessions
    ADD CONSTRAINT transfer_sessions_status_check
    CHECK (status IN ('active', 'completing', 'completed', 'aborted', 'expired'));

-- Ensure any pre-existing cold-compressed objects don't occupy active dedup cas_scope slots or dedup index rows.
UPDATE media_objects
SET cas_scope = 'cold15d:' || id::text
WHERE cold_compressed_at IS NOT NULL
  AND cas_scope NOT LIKE 'cold15d:%';

DELETE FROM media_dedup_indexes
WHERE object_id IN (SELECT id FROM media_objects WHERE cold_compressed_at IS NOT NULL);
