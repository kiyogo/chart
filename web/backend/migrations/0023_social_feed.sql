-- Spec 016: Threads-style Social Feed ("Mới")
CREATE TABLE IF NOT EXISTS feed_posts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id    UUID REFERENCES feed_posts(id) ON DELETE CASCADE,
    repost_of_id UUID REFERENCES feed_posts(id) ON DELETE CASCADE,
    content      TEXT NOT NULL CHECK (char_length(content) <= 1000),
    media_url    TEXT CHECK (media_url IS NULL OR media_url LIKE '/uploads/%'),
    visibility   TEXT NOT NULL DEFAULT 'friends' CHECK (visibility IN ('friends', 'public', 'self')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS feed_posts_timeline_idx
    ON feed_posts (created_at DESC)
    WHERE parent_id IS NULL;

CREATE INDEX IF NOT EXISTS feed_posts_author_idx
    ON feed_posts (author_id, created_at DESC);

CREATE INDEX IF NOT EXISTS feed_posts_parent_idx
    ON feed_posts (parent_id, created_at ASC)
    WHERE parent_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS feed_likes (
    post_id    UUID NOT NULL REFERENCES feed_posts(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (post_id, user_id)
);
