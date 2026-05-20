-- ============================================================================
-- UniBot 初始 Schema DDL (SQLite)
-- 5 域 26 表 + 6 触发器 + 全量索引
-- 适用：全新安装
-- ============================================================================

-- ============================================================================
-- 域 1：身份与社交域 (IM_ACCOUNT) — 7 表
-- ============================================================================

-- 1. 统一 IM 身份
CREATE TABLE IF NOT EXISTS im_accounts (
    user_id         TEXT PRIMARY KEY NOT NULL,
    nickname        TEXT NOT NULL,
    avatar_url      TEXT NOT NULL DEFAULT '',
    signature       TEXT NOT NULL DEFAULT '',
    account_source  TEXT NOT NULL DEFAULT 'simulated'
                    CHECK (account_source IN ('simulated', 'real')),
    origin_user_id  TEXT,
    qid             TEXT,
    age             INTEGER,
    sex             TEXT,
    level           INTEGER,
    bio             TEXT,
    account_status  TEXT NOT NULL DEFAULT 'active'
                    CHECK (account_status IN ('active', 'disabled', 'unavailable', 'deleted')),
    unavailable_at  INTEGER,
    deleted_at      INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    CHECK (
        (account_status = 'unavailable' AND unavailable_at IS NOT NULL)
        OR account_status != 'unavailable'
    ),
    CHECK (
        (account_status = 'deleted' AND deleted_at IS NOT NULL)
        OR account_status != 'deleted'
    )
);

-- 2. 自定义表情
CREATE TABLE IF NOT EXISTS account_faces (
    face_id           TEXT PRIMARY KEY NOT NULL,
    owner_user_id     TEXT NOT NULL,
    face_name         TEXT,
    emoji_package_id  INTEGER,
    key               TEXT,
    remote_url        TEXT,
    local_path        TEXT,
    created_at        INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_account_faces_owner ON account_faces(owner_user_id);

-- 3. 好友分类 (per-user)
CREATE TABLE IF NOT EXISTS friend_categories (
    category_id   TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_friend_categories_owner ON friend_categories(owner_user_id);

-- 4. 好友关系 (owner 视角)
CREATE TABLE IF NOT EXISTS friendships (
    owner_user_id      TEXT NOT NULL,
    friend_user_id     TEXT NOT NULL,
    friend_category_id TEXT NOT NULL,
    remark             TEXT,
    is_pinned          INTEGER NOT NULL DEFAULT 0,
    created_at         INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (owner_user_id, friend_user_id),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (friend_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (friend_category_id) REFERENCES friend_categories(category_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_friendships_friend ON friendships(friend_user_id);

-- 5. 好友申请
CREATE TABLE IF NOT EXISTS friend_requests (
    request_id        TEXT PRIMARY KEY NOT NULL,
    initiator_user_id TEXT NOT NULL,
    target_user_id    TEXT NOT NULL,
    comment           TEXT,
    state             TEXT NOT NULL DEFAULT 'pending'
                      CHECK (state IN ('pending', 'accepted', 'rejected', 'ignored')),
    created_at        INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    handled_at        INTEGER,
    FOREIGN KEY (initiator_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (target_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_friend_req_dedup
    ON friend_requests(initiator_user_id, target_user_id) WHERE state = 'pending';
CREATE INDEX IF NOT EXISTS idx_friend_req_target
    ON friend_requests(target_user_id, state, created_at) WHERE state = 'pending';

-- 6. 群分类 (per-user)
CREATE TABLE IF NOT EXISTS group_categories (
    category_id   TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_group_categories_owner ON group_categories(owner_user_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_group_categories_name
    ON group_categories(owner_user_id, name);

-- 7. 账号-群视图 (owner 视角，per-user)
CREATE TABLE IF NOT EXISTS user_groups (
    owner_user_id   TEXT NOT NULL,
    group_id        TEXT NOT NULL,
    category_id     TEXT,
    is_pinned       INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0, 1)),
    is_muted        INTEGER NOT NULL DEFAULT 0 CHECK (is_muted IN (0, 1)),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    joined_at       INTEGER,
    last_active_at  INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (owner_user_id, group_id),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES group_categories(category_id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_user_groups_category ON user_groups(owner_user_id, category_id);
CREATE INDEX IF NOT EXISTS idx_user_groups_group ON user_groups(group_id);

-- ============================================================================
-- 域 2：群组与内容域 (CHAT_GROUP) — 10 表
-- ============================================================================

-- 8. 群组基本信息
CREATE TABLE IF NOT EXISTS chat_groups (
    group_id            TEXT PRIMARY KEY NOT NULL,
    group_name          TEXT NOT NULL,
    group_source        TEXT NOT NULL DEFAULT 'simulated'
                        CHECK (group_source IN ('simulated', 'real')),
    avatar_url          TEXT,
    group_owner_user_id TEXT NOT NULL,
    member_count        INTEGER NOT NULL DEFAULT 0,
    max_member_count    INTEGER NOT NULL DEFAULT 500,
    is_whole_muted      INTEGER NOT NULL DEFAULT 0,
    mute_until          INTEGER,
    mute_operator_user_id TEXT,
    group_status        TEXT NOT NULL DEFAULT 'active'
                        CHECK (group_status IN ('active', 'dissolved', 'unavailable')),
    dissolved_at        INTEGER,
    unavailable_at      INTEGER,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_owner_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
    FOREIGN KEY (mute_operator_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    CHECK (member_count >= 0 AND member_count <= max_member_count),
    CHECK (
        (group_status = 'dissolved' AND dissolved_at IS NOT NULL)
        OR group_status != 'dissolved'
    ),
    CHECK (
        (group_status = 'unavailable' AND unavailable_at IS NOT NULL)
        OR group_status != 'unavailable'
    )
);
CREATE INDEX IF NOT EXISTS idx_chat_groups_group_owner ON chat_groups(group_owner_user_id);

-- 9. 群成员
CREATE TABLE IF NOT EXISTS group_members (
    group_id      TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    card          TEXT NOT NULL DEFAULT '',
    special_title TEXT DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'member'
                  CHECK (role IN ('owner', 'admin', 'member')),
    joined_at     INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    last_sent_at  INTEGER NOT NULL DEFAULT 0,
    mute_until    INTEGER,
    PRIMARY KEY (group_id, user_id),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_group_members_user ON group_members(user_id);
CREATE INDEX IF NOT EXISTS idx_group_members_role ON group_members(group_id, role);

-- 10. 群通知/申请
CREATE TABLE IF NOT EXISTS group_requests (
    group_id          TEXT NOT NULL,
    notification_seq  TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    initiator_user_id TEXT NOT NULL,
    target_user_id    TEXT,
    comment           TEXT,
    state             TEXT NOT NULL DEFAULT 'pending'
                      CHECK (state IN ('pending', 'accepted', 'rejected', 'ignored')),
    created_at        INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    handled_at        INTEGER,
    operator_user_id  TEXT,
    PRIMARY KEY (group_id, notification_seq),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (initiator_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (target_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    FOREIGN KEY (operator_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_group_req_initiator ON group_requests(initiator_user_id);
CREATE INDEX IF NOT EXISTS idx_group_req_pending
    ON group_requests(group_id, state, created_at) WHERE state = 'pending';

-- 11. 群公告
CREATE TABLE IF NOT EXISTS group_announcements (
    announcement_id TEXT PRIMARY KEY NOT NULL,
    group_id        TEXT NOT NULL,
    sender_user_id  TEXT NOT NULL,
    content         TEXT NOT NULL,
    image_url       TEXT,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_announcements_group ON group_announcements(group_id, created_at DESC);

-- 12. 群文件夹
CREATE TABLE IF NOT EXISTS group_folders (
    folder_id        TEXT PRIMARY KEY NOT NULL,
    group_id         TEXT NOT NULL,
    parent_folder_id TEXT,
    folder_name      TEXT NOT NULL,
    creator_user_id  TEXT NOT NULL,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (parent_folder_id) REFERENCES group_folders(folder_id) ON DELETE CASCADE,
    FOREIGN KEY (creator_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_folders_group_parent ON group_folders(group_id, parent_folder_id);

-- 13. 群文件
CREATE TABLE IF NOT EXISTS group_files (
    file_id          TEXT PRIMARY KEY NOT NULL,
    group_id         TEXT NOT NULL,
    parent_folder_id TEXT,
    file_name        TEXT NOT NULL,
    file_size        INTEGER NOT NULL,
    file_hash        TEXT,
    uploader_user_id TEXT NOT NULL,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    expire_at        INTEGER,
    download_count   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (parent_folder_id) REFERENCES group_folders(folder_id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_group_files_group_time ON group_files(group_id, created_at DESC);

-- 14. 精华消息
CREATE TABLE IF NOT EXISTS group_essence_messages (
    essence_id       TEXT PRIMARY KEY NOT NULL,
    group_id         TEXT NOT NULL,
    message_id       TEXT,
    sender_user_id   TEXT NOT NULL,
    operator_user_id TEXT NOT NULL,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(message_id) ON DELETE SET NULL,
    FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (operator_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    UNIQUE (group_id, message_id)
);
CREATE INDEX IF NOT EXISTS idx_essence_group ON group_essence_messages(group_id, created_at DESC);

-- 15. 群事件记录 (append-only)
CREATE TABLE IF NOT EXISTS group_events (
    event_id     TEXT PRIMARY KEY NOT NULL,
    group_id     TEXT NOT NULL,
    event_type   TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_group_events_group_type
    ON group_events(group_id, event_type, created_at DESC);

-- 16. 群相册
CREATE TABLE IF NOT EXISTS group_albums (
    album_id   TEXT PRIMARY KEY NOT NULL,
    group_id   TEXT NOT NULL,
    name       TEXT NOT NULL,
    cover_url  TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_albums_group ON group_albums(group_id);

-- 17. 群照片
CREATE TABLE IF NOT EXISTS group_photos (
    photo_id         TEXT PRIMARY KEY NOT NULL,
    album_id         TEXT NOT NULL,
    url              TEXT NOT NULL,
    description      TEXT,
    uploader_user_id TEXT NOT NULL,
    file_size        INTEGER,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (album_id) REFERENCES group_albums(album_id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_photos_album ON group_photos(album_id, created_at DESC);

-- ============================================================================
-- 域 3：会话与消息域 (CONVERSATION) — 4 表
-- ============================================================================

-- 18. 会话
CREATE TABLE IF NOT EXISTS conversations (
    conversation_id    TEXT PRIMARY KEY NOT NULL,
    owner_user_id      TEXT NOT NULL,
    conversation_scene TEXT NOT NULL CHECK (conversation_scene IN ('private', 'group', 'temp')),
    peer_user_id       TEXT,
    group_id           TEXT,
    last_message_id    TEXT,
    last_read_seq      TEXT,
    unread_count       INTEGER NOT NULL DEFAULT 0 CHECK (unread_count >= 0),
    is_pinned          INTEGER NOT NULL DEFAULT 0,
    is_muted           INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (owner_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (peer_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE CASCADE,
    FOREIGN KEY (last_message_id) REFERENCES messages(message_id) ON DELETE SET NULL,
    CHECK (
        (conversation_scene IN ('private', 'temp') AND peer_user_id IS NOT NULL AND group_id IS NULL)
        OR (conversation_scene = 'group' AND group_id IS NOT NULL AND peer_user_id IS NULL)
    )
);
CREATE INDEX IF NOT EXISTS idx_conv_owner_updated
    ON conversations(owner_user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conv_unread
    ON conversations(owner_user_id, unread_count) WHERE unread_count > 0;
CREATE UNIQUE INDEX IF NOT EXISTS uq_conversation_private
    ON conversations(owner_user_id, conversation_scene, peer_user_id)
    WHERE conversation_scene IN ('private', 'temp');
CREATE UNIQUE INDEX IF NOT EXISTS uq_conversation_group
    ON conversations(owner_user_id, conversation_scene, group_id)
    WHERE conversation_scene = 'group';

-- 19. 消息
CREATE TABLE IF NOT EXISTS messages (
    message_id          TEXT PRIMARY KEY NOT NULL,
    message_scene       TEXT NOT NULL CHECK (message_scene IN ('private', 'group', 'temp')),
    peer_id             TEXT NOT NULL,
    message_seq         TEXT NOT NULL,
    sender_user_id      TEXT NOT NULL,
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
    FOREIGN KEY (quoted_message_id) REFERENCES messages(message_id) ON DELETE SET NULL,
    FOREIGN KEY (recalled_by_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    FOREIGN KEY (session_id) REFERENCES debug_sessions(session_id) ON DELETE SET NULL,
    UNIQUE (message_scene, peer_id, message_seq),
    CHECK (
        (message_scene IN ('private', 'temp') AND receiver_user_id IS NOT NULL AND group_id IS NULL)
        OR (message_scene = 'group' AND group_id IS NOT NULL AND receiver_user_id IS NULL)
    )
);
CREATE INDEX IF NOT EXISTS idx_msg_scene_peer_time
    ON messages(message_scene, peer_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_msg_sender_time
    ON messages(sender_user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_msg_bot_time
    ON messages(bot_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_msg_quoted
    ON messages(quoted_message_id);

-- 20. 消息表情回应
CREATE TABLE IF NOT EXISTS message_reactions (
    reaction_id      TEXT PRIMARY KEY NOT NULL,
    message_id       TEXT NOT NULL,
    operator_user_id TEXT NOT NULL,
    face_id          TEXT NOT NULL,
    is_add           INTEGER NOT NULL DEFAULT 1,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (message_id) REFERENCES messages(message_id) ON DELETE CASCADE,
    FOREIGN KEY (operator_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reaction_dedup
    ON message_reactions(message_id, operator_user_id, face_id);
CREATE INDEX IF NOT EXISTS idx_reactions_msg ON message_reactions(message_id);

-- 21. 戳一戳
CREATE TABLE IF NOT EXISTS pokes (
    poke_id            TEXT PRIMARY KEY NOT NULL,
    sender_user_id     TEXT NOT NULL,
    target_user_id     TEXT NOT NULL,
    message_scene      TEXT NOT NULL CHECK (message_scene IN ('private', 'group')),
    peer_id            TEXT NOT NULL,
    is_recalled        INTEGER NOT NULL DEFAULT 0,
    recalled_by_user_id TEXT,
    recalled_at        INTEGER,
    created_at         INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (target_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    FOREIGN KEY (recalled_by_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_pokes_scene_peer
    ON pokes(message_scene, peer_id, created_at DESC);

-- ============================================================================
-- 域 4：Bot 与调试域 (BOT) — 2 表
-- ============================================================================

-- 22. 机器人实例
CREATE TABLE IF NOT EXISTS bots (
    bot_id          TEXT PRIMARY KEY NOT NULL,
    bound_user_id   TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    runtime_status  TEXT NOT NULL DEFAULT 'stopped'
                    CHECK (runtime_status IN ('stopped', 'running', 'error')),
    config_path     TEXT NOT NULL,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (bound_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_bots_bound_user ON bots(bound_user_id);

-- 23. 调试会话
CREATE TABLE IF NOT EXISTS debug_sessions (
    session_id   TEXT PRIMARY KEY NOT NULL,
    bot_id       TEXT NOT NULL,
    session_name TEXT NOT NULL,
    description  TEXT,
    started_at   INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    ended_at     INTEGER,
    FOREIGN KEY (bot_id) REFERENCES bots(bot_id) ON DELETE CASCADE,
    CHECK (ended_at >= started_at OR ended_at IS NULL)
);
CREATE INDEX IF NOT EXISTS idx_debug_sessions_bot ON debug_sessions(bot_id, started_at DESC);

-- ============================================================================
-- 域 5：系统与审计域 (System) — 3 表
-- ============================================================================

-- 24. 协议报文 (raw_json 不存库，文件路径直接存 file_path)
CREATE TABLE IF NOT EXISTS protocol_packets (
    packet_id           TEXT PRIMARY KEY NOT NULL,
    bot_id              TEXT,
    profile_id          TEXT,
    protocol_type       TEXT NOT NULL,
    direction           TEXT NOT NULL CHECK (direction IN ('send', 'receive')),
    action_name         TEXT NOT NULL,
    file_path           TEXT NOT NULL,
    related_object_type TEXT,
    related_object_id   TEXT,
    is_error            INTEGER NOT NULL DEFAULT 0,
    session_id          TEXT,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (bot_id) REFERENCES bots(bot_id) ON DELETE SET NULL,
    FOREIGN KEY (session_id) REFERENCES debug_sessions(session_id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_packet_bot_time ON protocol_packets(bot_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_packet_error_time ON protocol_packets(is_error, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_packet_related ON protocol_packets(related_object_type, related_object_id);
CREATE INDEX IF NOT EXISTS idx_packet_session ON protocol_packets(session_id);

-- 25. 应用设置
CREATE TABLE IF NOT EXISTS app_settings (
    setting_key   TEXT PRIMARY KEY NOT NULL,
    setting_value TEXT NOT NULL,
    value_type    TEXT NOT NULL DEFAULT 'string'
                  CHECK (value_type IN ('string', 'int', 'bool', 'json')),
    description   TEXT,
    updated_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- 26. 操作审计日志 (append-only)
CREATE TABLE IF NOT EXISTS audit_events (
    event_id      TEXT PRIMARY KEY NOT NULL,
    event_type    TEXT NOT NULL,
    actor_user_id TEXT,
    target_type   TEXT CHECK (target_type IS NULL OR target_type IN ('bot', 'message', 'connection', 'group', 'user')),
    target_id     TEXT,
    detail_json   TEXT,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (actor_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_type_time ON audit_events(event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_events(actor_user_id, created_at DESC);

-- ============================================================================
-- 触发器
-- ============================================================================

-- 环境隔离：simulated 账号不能加入 real 群，反之亦然
CREATE TRIGGER IF NOT EXISTS trg_group_member_isolation
BEFORE INSERT ON group_members
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'environment isolation violation: account_source must match group_source')
    WHERE (
        SELECT account_source FROM im_accounts WHERE user_id = NEW.user_id
    ) != (
        SELECT group_source FROM chat_groups WHERE group_id = NEW.group_id
    );
END;

-- 成员计数 +1
CREATE TRIGGER IF NOT EXISTS trg_member_count_inc
AFTER INSERT ON group_members
FOR EACH ROW
BEGIN
    UPDATE chat_groups SET member_count = member_count + 1
    WHERE group_id = NEW.group_id;
END;

-- 成员计数 -1
CREATE TRIGGER IF NOT EXISTS trg_member_count_dec
AFTER DELETE ON group_members
FOR EACH ROW
BEGIN
    UPDATE chat_groups SET member_count = member_count - 1
    WHERE group_id = OLD.group_id;
END;

-- USER_GROUP.category_id 必须属于同一个 owner_user_id
CREATE TRIGGER IF NOT EXISTS trg_user_group_category_owner
BEFORE INSERT ON user_groups
FOR EACH ROW
WHEN NEW.category_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'category owner mismatch: category must belong to same owner_user_id')
    WHERE NOT EXISTS (
        SELECT 1 FROM group_categories
        WHERE category_id = NEW.category_id AND owner_user_id = NEW.owner_user_id
    );
END;

CREATE TRIGGER IF NOT EXISTS trg_user_group_category_owner_upd
BEFORE UPDATE OF owner_user_id, category_id ON user_groups
FOR EACH ROW
WHEN NEW.category_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'category owner mismatch: category must belong to same owner_user_id')
    WHERE NOT EXISTS (
        SELECT 1 FROM group_categories
        WHERE category_id = NEW.category_id AND owner_user_id = NEW.owner_user_id
    );
END;

-- 未读计数 +1（新消息到达时自动增加会话未读数）
CREATE TRIGGER IF NOT EXISTS trg_unread_inc
AFTER INSERT ON messages
FOR EACH ROW
BEGIN
    UPDATE conversations
    SET unread_count = unread_count + 1,
        updated_at = NEW.created_at
    WHERE conversation_scene = NEW.message_scene
    AND (
        (NEW.message_scene IN ('private', 'temp') AND peer_user_id = NEW.sender_user_id)
        OR (NEW.message_scene = 'group' AND group_id = NEW.group_id)
    );
END;

-- ============================================================================
-- 初始数据
-- ============================================================================

INSERT INTO app_settings (setting_key, setting_value, value_type, description) VALUES
    ('schema.version', '0001', 'string', '当前数据库 schema 迁移版本'),
    ('audit.retention_days', '30', 'int', '报文/审计日志保留天数，0=永不过期'),
    ('audit.cleanup_enabled', 'true', 'bool', '是否启用定时清理');