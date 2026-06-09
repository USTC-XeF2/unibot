-- 消息序列号计数器表
CREATE TABLE message_seq_counter (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    next_seq INTEGER NOT NULL DEFAULT 1
);
INSERT INTO message_seq_counter (id, next_seq) VALUES (1, 1);

-- 重建 messages 表：将 message_seq 从 TEXT 改为 INTEGER
CREATE TABLE messages_new (
    message_id          TEXT PRIMARY KEY NOT NULL,
    message_scene       TEXT NOT NULL CHECK (message_scene IN ('private', 'group', 'temp')),
    peer_id             TEXT NOT NULL,
    message_seq         INTEGER,
    sender_user_id      TEXT,
    receiver_user_id    TEXT,
    group_id            TEXT,
    bot_id              TEXT,
    content_json        TEXT NOT NULL,
    quoted_message_id   TEXT,
    forward_id          TEXT,
    is_recalled         INTEGER NOT NULL DEFAULT 0,
    recalled_by_user_id TEXT,
    recalled_at         INTEGER,
    session_id          TEXT,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
    FOREIGN KEY (receiver_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE RESTRICT,
    FOREIGN KEY (quoted_message_id) REFERENCES messages_new(message_id) ON DELETE SET NULL,
    FOREIGN KEY (recalled_by_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    FOREIGN KEY (session_id) REFERENCES debug_sessions(session_id) ON DELETE SET NULL,
    UNIQUE (message_scene, peer_id, message_seq),
    CHECK (
        (message_scene IN ('private', 'temp') AND receiver_user_id IS NOT NULL AND group_id IS NULL)
        OR (message_scene = 'group' AND group_id IS NOT NULL AND receiver_user_id IS NULL)
    )
);

-- 迁移数据，回填 message_seq
INSERT INTO messages_new (
    message_id, message_scene, peer_id, message_seq, sender_user_id,
    receiver_user_id, group_id, bot_id, content_json, quoted_message_id,
    forward_id, is_recalled, recalled_by_user_id, recalled_at,
    session_id, created_at
)
SELECT
    message_id, message_scene, peer_id,
    ROW_NUMBER() OVER (ORDER BY created_at, message_id),
    sender_user_id, receiver_user_id, group_id, bot_id,
    content_json, quoted_message_id, forward_id, is_recalled,
    recalled_by_user_id, recalled_at, session_id, created_at
FROM messages;

-- 替换旧表
DROP TABLE messages;
ALTER TABLE messages_new RENAME TO messages;

-- 重建索引
CREATE INDEX idx_msg_scene_peer_time ON messages(message_scene, peer_id, created_at DESC);
CREATE INDEX idx_msg_sender_time ON messages(sender_user_id, created_at);
CREATE INDEX idx_msg_bot_time ON messages(bot_id, created_at DESC);
CREATE INDEX idx_msg_quoted ON messages(quoted_message_id);

-- 更新计数器
UPDATE message_seq_counter
SET next_seq = (SELECT COALESCE(MAX(message_seq), 0) + 1 FROM messages)
WHERE id = 1;
