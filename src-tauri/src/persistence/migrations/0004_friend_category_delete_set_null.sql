-- ============================================================================
-- Migration 0004: relax friend_category_id and align friend categories with groups
--
-- Fixes two issues from the 0001 schema:
--   1. friendships.friend_category_id was NOT NULL with ON DELETE RESTRICT, so a
--      friend category could not be deleted while any friend referenced it. Rebuild
--      the table to make the column nullable with ON DELETE SET NULL, matching how
--      user_groups.category_id already behaves.
--   2. friend_categories lacked the UNIQUE(owner_user_id, name) index that
--      group_categories has, forcing name-uniqueness to be checked in code. Add the
--      index so the database is the single source of truth, mirroring groups.
-- ============================================================================

CREATE TABLE friendships_new (
    owner_user_id      TEXT NOT NULL,
    friend_user_id     TEXT NOT NULL,
    friend_category_id TEXT,
    remark             TEXT,
    is_pinned          INTEGER NOT NULL DEFAULT 0,
    created_at         INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (owner_user_id, friend_user_id),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (friend_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (friend_category_id) REFERENCES friend_categories(category_id) ON DELETE SET NULL
);

INSERT INTO friendships_new (
    owner_user_id,
    friend_user_id,
    friend_category_id,
    remark,
    is_pinned,
    created_at
)
SELECT
    owner_user_id,
    friend_user_id,
    friend_category_id,
    remark,
    is_pinned,
    created_at
FROM friendships;

DROP TABLE friendships;
ALTER TABLE friendships_new RENAME TO friendships;

CREATE INDEX idx_friendships_friend ON friendships(friend_user_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_friend_categories_name
    ON friend_categories(owner_user_id, name);
