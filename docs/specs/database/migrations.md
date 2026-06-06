# Schema 版本管理

数据库 schema 版本管理与迁移执行规范。采用递增序号迁移 + `app_settings(schema.version)` 管理版本。

## 1. 版本模型

### 1.1 版本存储

Schema 版本通过 `app_settings` 中的 `schema.version` 键管理——这是唯一的版本真相来源。当前设计中该值使用三位迁移序号字符串（如 `001`、`011`），方便和迁移文件名前缀直接比较；应用发布版本如需记录，应使用独立设置键。

```sql
-- 读取当前版本
SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version';
```

UniBot 是桌面端调试工具——单实例运行、版本线性演进——因此不需要独立的迁移历史表。`app_settings(schema.version)` 直接记录当前已应用的最新迁移编号。

### 1.2 版本读取与更新

```text
应用启动时:
  current_version = SELECT setting_value FROM app_settings
                    WHERE setting_key = 'schema.version';

  遍历 migrations/ 目录中所有迁移文件，
  选出序号大于 current_version 的迁移编号;

  FOR EACH migration IN pending_migrations (按序号升序):
    执行迁移;
    UPDATE app_settings SET setting_value = :new_version, updated_at = :now
    WHERE setting_key = 'schema.version';
```

迁移执行后立即更新版本号——即使后续迁移失败，已成功的迁移也不会重复执行。

## 2. 迁移文件规范

### 2.1 文件命名

```
{序号}_{描述}.sql

示例：
001_initial_schema.sql
002_add_bot_table.sql
003_add_bot_id_to_messages.sql
004_rebuild_messages_for_text_ids.sql
005_add_debug_session.sql
```

序号为三位自增数字，不使用时间戳（时间戳在多人协作时可能冲突）。描述使用 snake_case 英文。

### 2.2 文件结构

```sql
-- Migration: 002_add_bot_table
-- Description: 新增 BOT 实体表，Bot 配置走 config_path JSON 文件
-- Created: 2026-05-15

-- ============================================================
-- UP
-- ============================================================

CREATE TABLE IF NOT EXISTS bots (
    bot_id          TEXT PRIMARY KEY,
    bound_user_id   TEXT NOT NULL REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    display_name    TEXT NOT NULL,
    runtime_status  TEXT NOT NULL DEFAULT 'stopped'
                    CHECK (runtime_status IN ('stopped', 'running', 'error')),
    config_path     TEXT NOT NULL,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_bots_bound_user ON bots(bound_user_id);
```

迁移脚本只写 UP（正向迁移），不写 ROLLBACK。回退通过写新的 UP 迁移实现。

### 2.3 规则

| 规则 | 说明 |
|---|---|
| **单向迁移** | 只写 UP，不写 ROLLBACK。需回退时写新的 UP 迁移把结构改回去 |
| **每迁移一事务** | 一个迁移脚本 = 一个 `BEGIN...COMMIT`，全部成功或全部回滚 |
| **幂等** | 所有 DDL 使用 `IF NOT EXISTS` / `IF EXISTS` |
| **DDL 与种子数据分离** | 迁移脚本只含 DDL 和必要的数据回填 INSERT...SELECT，不含测试种子数据 |
| **无交互** | 迁移脚本不包含任何需用户输入的语句 |
| **不修改已执行的迁移** | 已提交到代码仓库的迁移文件不可修改。如需修正，写新的迁移 |

## 3. SQLite ALTER TABLE 限制

SQLite 的 ALTER TABLE 能力有限（在 3.35 之前不支持 DROP COLUMN）：

| 操作 | 支持程度 | 处理方式 |
|---|---|---|
| ADD COLUMN | 支持（只能加在末尾） | 直接 ALTER TABLE ADD COLUMN |
| RENAME TABLE | 支持 | ALTER TABLE RENAME TO |
| RENAME COLUMN | 支持（3.25+） | ALTER TABLE RENAME COLUMN |
| DROP COLUMN | 支持（3.35+） | ALTER TABLE DROP COLUMN（或重建表兼容旧版） |
| ALTER COLUMN (改类型) | 不支持 | 重建表 |

### 3.1 重建表标准流程

需要修改列类型或删除列（兼容旧版 SQLite）时的 4 步流程：

```sql
BEGIN;

-- 1. 创建新表（含目标 schema）
CREATE TABLE messages_new (
    message_id      TEXT PRIMARY KEY,
    message_scene   TEXT NOT NULL CHECK (message_scene IN ('private', 'group', 'temp')),
    peer_id         TEXT NOT NULL,
    message_seq     TEXT NOT NULL,
    sender_user_id  TEXT REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    receiver_user_id TEXT REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    group_id        TEXT REFERENCES chat_groups(group_id) ON DELETE SET NULL,
    bot_id          TEXT,
    content_json    TEXT NOT NULL,
    quoted_message_id TEXT REFERENCES messages(message_id) ON DELETE SET NULL,
    forward_id      TEXT,
    is_recalled     INTEGER NOT NULL DEFAULT 0,
    recalled_by_user_id TEXT REFERENCES im_accounts(user_id) ON DELETE SET NULL,
    recalled_at     INTEGER,
    session_id      TEXT REFERENCES debug_sessions(session_id) ON DELETE SET NULL,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    UNIQUE(message_scene, peer_id, message_seq)
);

-- 2. 迁移数据
INSERT INTO messages_new
SELECT message_id, message_scene, peer_id, message_seq,
       sender_user_id, receiver_user_id, group_id,
       NULL AS bot_id,
       content_json, quoted_message_id,
       NULL AS forward_id,
       is_recalled, recalled_by_user_id, recalled_at,
       NULL AS session_id,
       created_at
FROM messages;

-- 3. 替换表
DROP TABLE messages;
ALTER TABLE messages_new RENAME TO messages;

-- 4. 重建索引
CREATE INDEX idx_msg_scene_peer_time ON messages(message_scene, peer_id, created_at DESC);
CREATE INDEX idx_msg_sender_time ON messages(sender_user_id, created_at);
CREATE INDEX idx_msg_bot_time ON messages(bot_id, created_at DESC);
CREATE INDEX idx_msg_quoted ON messages(quoted_message_id);

COMMIT;
```

### 3.2 外键处理

重建表期间需要临时关闭外键检查。SQLite 不允许在事务内部切换 `foreign_keys`，因此必须在 `BEGIN` 之前关闭，`COMMIT` 之后恢复并立即校验：

```sql
PRAGMA foreign_keys = OFF;
BEGIN;
-- ... 重建表步骤 ...
COMMIT;
PRAGMA foreign_keys = ON;
PRAGMA foreign_key_check;  -- 提交后手动校验
```

## 4. 当前项目迁移路线

基于当前代码（`src-tauri/src/persistence/`）到目标设计的差距，推荐迁移顺序：

| 序号 | 内容 | 类型 | 说明 |
|---|---|---|---|
| 001 | 初始 schema（26 表） | 新建 | 全新安装的完整 DDL |
| 002 | ID 类型 INTEGER→TEXT | 重建 | 重建 users/friendships/groups/messages 等核心表 |
| 003 | 消息标识重构 | 重建 | source_type+source_id → message_scene+peer_id+message_seq |
| 004 | 新增 BOT 表 | 新建 | 含 config_path |
| 005 | 新增 CONVERSATION | 新建 + ALTER | 会话独立建表 |
| 006 | 扩展 CHAT_GROUP | ALTER | group_source, category_id, avatar_url, 合并 is_whole_muted |
| 007 | 重构 GROUP_REQUEST | 重建 | 引入 notification_seq 联合 PK，废弃旧 group_requests |
| 008 | MESSAGE 加 bot_id + session_id | ALTER | 反规范化字段 + 索引 |
| 009 | PROTOCOL_PACKET 重构 | ALTER | 移除 raw_json, 加 is_error/session_id/file_path |
| 010 | 新增系统治理表 | 新建 | app_settings, debug_sessions, audit_events |
| 011 | 新增群内容表 | 新建 | GROUP_CATEGORY, GROUP_ALBUM, GROUP_PHOTO |

从旧版本升级的用户依次执行 002-011；全新安装只执行 001（含完整 DDL）。

## 5. 迁移文件目录

```
src-tauri/src/persistence/migrations/
├── 001_initial_schema.sql          # 全新安装：完整 26 表 DDL
├── 002_change_ids_to_text.sql      # INTEGER → TEXT 重建核心表
├── 003_rebuild_message_identity.sql # source_type+source_id → scene+peer+seq
├── 004_add_bot_table.sql           # 新增 BOT 实体
├── 005_add_conversation.sql        # 新增 CONVERSATION + 约束
├── 006_extend_chat_group.sql       # CHAT_GROUP 扩展字段
├── 007_rebuild_group_request.sql   # 重建 GROUP_REQUEST（notification_seq PK）
├── 008_add_bot_id_to_messages.sql  # MESSAGE.bot_id + session_id + forward_id + 索引
├── 009_rebuild_protocol_packet.sql # 移除 raw_json, 加 is_error/session_id/file_path
├── 010_add_system_tables.sql       # app_settings, debug_sessions, audit_events
├── 011_add_group_content.sql       # GROUP_CATEGORY, GROUP_ALBUM, GROUP_PHOTO
└── seed_test.sql                   # 测试种子数据（独立，不混入迁移序列）
```

## 6. 种子数据

测试/演示用种子数据独立于迁移脚本：

```sql
-- seed_test.sql
-- 模拟环境种子数据：创建 3 个模拟账号 + 1 个 Bot + 群组 + 示例消息
-- 由应用启动时通过 --seed 参数触发加载

INSERT INTO app_settings (setting_key, setting_value, value_type, description)
VALUES ('schema.version', '001', 'string', '当前数据库 schema 迁移版本');

-- ... 种子数据 INSERT 语句 ...
```

种子数据永远不混入迁移脚本。应用启动时通过命令行参数（`--seed`）或 debug 构建配置按需加载。

## 7. 迁移执行流程

```text
┌────────────────────────────────────────────────────┐
│ 应用启动                                             │
│   │                                                  │
│   ├── 检查 app_settings 是否存在 schema.version       │
│   │   ├── 存在 → 读取版本号                           │
│   │   └── 不存在 → 版本 = 'v0.0.0'（新数据库）        │
│   │                                                  │
│   ├── 扫描 migrations/ 目录，收集 .sql 文件            │
│   │   └── 排序：按序号升序                            │
│   │                                                  │
│   ├── 筛选待执行迁移（序号 > 当前版本）                 │
│   │   ├── 空 → 跳过                                   │
│   │   └── 非空 → 逐文件执行（每个文件一个事务）         │
│   │                                                  │
│   ├── 每执行完一个迁移                                 │
│   │   └── UPDATE app_settings SET setting_value=版本   │
│   │                                                  │
│   └── 全部完成 → 进入正常应用逻辑                      │
│                                                      │
│ 迁移失败处理：                                        │
│   ├── 当前迁移回滚（事务保证）                         │
│   ├── 已成功的迁移不回滚（版本号已更新）                │
│   ├── 记录错误日志                                    │
│   └── 弹窗提示用户：数据库迁移失败，提供"重试"/"退出"   │
└────────────────────────────────────────────────────┘
```

## 8. WAL 模式

应用启动时确保 WAL 模式已启用：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;   -- WAL 模式下 NORMAL 足够安全
PRAGMA foreign_keys = ON;       -- 强制 FK 约束
PRAGMA busy_timeout = 5000;    -- 5 秒忙等待超时
```

WAL 模式的好处：
- 读写并发：写操作不阻塞读操作
- 更好的写入性能（顺序写入 WAL 文件，而非随机写入主文件）
- 桌面应用的 SQLite 标准配置
