-- Spec 167: Contacts Directory Hub ("Danh bạ")
-- Adds fast sender-side pending friend request lookup and direct group/community invitations.

CREATE INDEX IF NOT EXISTS friend_requests_sender_pending_idx
    ON friend_requests (sender_id, created_at DESC)
    WHERE status = 'pending';

CREATE TABLE IF NOT EXISTS direct_target_invites (
    id          UUID PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('group', 'server')),
    target_id   UUID NOT NULL,
    code        TEXT NOT NULL,
    inviter_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (inviter_id <> invitee_id),
    UNIQUE (kind, target_id, invitee_id)
);

CREATE INDEX IF NOT EXISTS direct_target_invites_invitee_pending_idx
    ON direct_target_invites (invitee_id, created_at DESC)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS direct_target_invites_inviter_pending_idx
    ON direct_target_invites (inviter_id, created_at DESC)
    WHERE status = 'pending';
