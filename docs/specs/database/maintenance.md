# 数据库可视化与维护指南

本文说明 UniBot 本地 SQLite 数据库在开发、调试、课程展示和后续演进中的维护方式。重点区分两件事：

- **可视化查看**：用于理解表结构、检查业务数据、辅助调试。
- **数据库维护**：用于保证 schema 演进、数据完整性、备份恢复和长期可维护性。

可视化工具可以帮助观察数据库，但不应绕过应用服务层随意修改业务数据。正常业务状态变更应优先通过 Repo/Service/API 完成。

## 1. 数据库位置

开发环境下，Tauri 应用会在应用数据目录创建 SQLite 文件：

```text
%APPDATA%\dev.xef2.unibot\unibot.db
```

同目录下可能还会出现 SQLite WAL 相关文件：

```text
unibot.db-wal
unibot.db-shm
```

这些文件属于同一个数据库运行状态。复制或备份数据库时，应在应用退出后一起处理，避免只复制主 `unibot.db` 导致 WAL 中尚未 checkpoint 的数据丢失。

## 2. 推荐可视化工具

### 2.1 DB Browser for SQLite

适合课程展示和轻量调试：

- 查看表结构、索引、触发器。
- 浏览表数据。
- 手动执行只读 SQL。
- 导出查询结果。

推荐用于快速说明当前数据库中有哪些用户、群、消息、请求和系统设置。

### 2.2 DBeaver

适合更完整的数据库维护视角：

- 查看表关系和外键。
- 运行 SQL 脚本。
- 对比 schema。
- 分析索引和查询结果。

如果需要展示 ER 关系、外键约束和表间关联，DBeaver 更合适。

### 2.3 DataGrip

适合长期开发，但不是必需：

- SQL 编辑体验更好。
- schema 导航更强。
- 适合频繁写查询和维护脚本。

## 3. 建议重点查看的表

| 表 | 维护/展示意义 |
|---|---|
| `app_settings` | 查看 `schema.version`，确认当前迁移版本。 |
| `im_accounts` | 查看用户资料和 `account_status` 生命周期状态。 |
| `chat_groups` | 查看群资料和 `group_status` 生命周期状态。 |
| `group_members` | 查看群成员关系和角色。 |
| `friend_requests` | 查看好友请求状态流转。 |
| `group_requests` | 查看入群、邀请等群请求状态流转。 |
| `messages` | 查看消息事实记录、场景、发送者、接收者/群关系。 |
| `conversations` | 查看 owner 视角下的会话状态、最近消息和未读数。 |
| `message_reactions` | 查看表情回应操作历史。 |
| `pokes` | 查看戳一戳互动历史。 |
| `protocol_packets` | 查看协议包结构化索引和原始文件路径。 |
| `audit_events` | 查看系统维护和关键操作审计记录。 |

这些表能体现数据库在系统中的核心作用：不只是保存数据，还负责约束业务事实、保留历史、支撑调试和追踪。

## 4. 只读检查 SQL

### 4.1 查看 schema 版本

```sql
SELECT setting_value
FROM app_settings
WHERE setting_key = 'schema.version';
```

当前 baseline 期望值为：

```text
0001
```

### 4.2 查看表数量

```sql
SELECT COUNT(*) AS table_count
FROM sqlite_master
WHERE type = 'table';
```

当前 baseline 期望创建 26 张业务表。

### 4.3 查看最近消息

```sql
SELECT
    message_id,
    message_scene,
    peer_id,
    sender_user_id,
    receiver_user_id,
    group_id,
    created_at
FROM messages
ORDER BY created_at DESC
LIMIT 20;
```

### 4.4 查看账号和群生命周期状态

```sql
SELECT user_id, nickname, account_status, deleted_at, unavailable_at
FROM im_accounts
ORDER BY updated_at DESC;
```

```sql
SELECT group_id, group_name, group_owner_user_id, group_status, dissolved_at, unavailable_at
FROM chat_groups
ORDER BY updated_at DESC;
```

### 4.5 检查外键完整性

```sql
PRAGMA foreign_key_check;
```

期望结果为空。若返回行，说明存在外键破坏，需要先定位来源，不能直接忽略。

### 4.6 检查数据库整体完整性

```sql
PRAGMA integrity_check;
```

期望结果为：

```text
ok
```

## 5. Schema 演进维护规则

运行时 schema 的权威来源是：

```text
src-tauri/src/persistence/migrations/
```

当前 baseline 文件是：

```text
src-tauri/src/persistence/migrations/0001_initial_schema.sql
```

后续修改数据库结构时，应遵守以下规则：

1. 已经合入和发布过的迁移文件不直接修改。
2. 新增结构变更时创建新的迁移文件，例如 `0002_add_xxx.sql`。
3. 将新迁移注册到 `src-tauri/src/persistence/migrations/mod.rs`。
4. 每个迁移应能在干净数据库和已有旧版本数据库上稳定执行。
5. 修改 schema 后同步更新 `docs/specs/database/**` 下的设计文档。
6. 修改 schema 后必须运行迁移测试、Repo smoke tests 和构建检查。

推荐验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml persistence::migrator
cargo test --manifest-path src-tauri/Cargo.toml persistence::repo::tests
cargo check --manifest-path src-tauri/Cargo.toml
bun.cmd run build
```

在 PowerShell 中，如果 `bun run build` 被脚本执行策略拦截，可以使用 `bun.cmd run build`。

## 6. 数据维护原则

### 6.1 不物理删除核心身份行

正常业务流中，账号注销和群解散不应物理删除核心身份行：

- 用户注销：更新 `im_accounts.account_status = 'deleted'`，保留用户行。
- 用户不可查询：更新 `im_accounts.account_status = 'unavailable'`。
- 群解散：更新 `chat_groups.group_status = 'dissolved'`，保留群行。
- 群不可查询：更新 `chat_groups.group_status = 'unavailable'`。

这样可以保证历史消息、会话、群事件和调试记录仍然能关联到原始身份事实。

### 6.2 不绕过业务层修复数据

可视化工具中不要随手 `UPDATE` / `DELETE` 业务表。特别是这些表需要谨慎：

- `im_accounts`
- `chat_groups`
- `messages`
- `conversations`
- `friend_requests`
- `group_requests`
- `group_members`

如果确实需要修复数据，应优先：

1. 编写一次性维护脚本或新迁移。
2. 在测试数据库验证。
3. 运行 `PRAGMA foreign_key_check;` 和相关 Repo tests。
4. 记录修复原因、SQL 和验证结果。

### 6.3 保持会话状态 owner-scoped

`conversations` 是用户视角下的状态表，不是消息事实表。维护时应注意：

- `messages` 记录消息事实。
- `conversations` 记录某个 `owner_user_id` 看到的会话状态。
- 未读数、置顶、免打扰等状态应由服务层维护，不应通过通用消息触发器自动推导。

## 7. 备份与恢复

### 7.1 开发环境备份

备份前建议退出应用，然后复制：

```text
%APPDATA%\dev.xef2.unibot\unibot.db
%APPDATA%\dev.xef2.unibot\unibot.db-wal
%APPDATA%\dev.xef2.unibot\unibot.db-shm
```

如果应用已经退出且 WAL 已 checkpoint，可能只存在 `unibot.db`。

### 7.2 恢复后检查

恢复数据库后，至少执行：

```sql
PRAGMA integrity_check;
PRAGMA foreign_key_check;
SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version';
```

然后运行应用核心流程，确认用户、好友、群、消息列表可以正常读取。

## 8. 清理与压缩

SQLite 删除大量数据后，文件大小不会一定立刻变小。需要压缩时可以使用：

```sql
VACUUM;
```

注意：

- `VACUUM` 会重写数据库文件，执行前应备份。
- 执行时不要让应用同时写入数据库。
- 一般开发期不需要频繁执行。

如果启用了 WAL 模式，可在维护窗口执行 checkpoint：

```sql
PRAGMA wal_checkpoint(TRUNCATE);
```

## 9. 建议的应用内维护面板

后续可以在开发模式或设置页中加入只读数据库维护面板，用于减少手动打开数据库的频率。

建议显示：

- 当前数据库路径。
- 当前 `schema.version`。
- 表数量。
- 用户数、群数、消息数、会话数。
- 最近 20 条消息。
- 最近好友请求和群请求。
- `PRAGMA foreign_key_check` 结果。
- `PRAGMA integrity_check` 结果。

建议提供的操作：

- 复制数据库路径。
- 打开数据库所在目录。
- 导出只读诊断报告。
- 触发一次完整性检查。

不建议在第一版维护面板中提供任意 SQL 执行器。任意 SQL 执行器容易绕过服务层约束，带来不可追踪的数据损坏。

## 10. 课程展示口径

可以这样描述数据库维护能力：

> UniBot 使用 SQLite 作为本地持久化数据库。应用启动时通过迁移系统自动创建和升级 schema，当前版本记录在 `app_settings.schema.version` 中。数据库通过外键、CHECK 约束、唯一索引和触发器维护核心数据完整性，并通过账号/群生命周期状态保留历史消息的可追溯性。开发阶段可使用 SQLite 可视化工具查看表结构和业务数据；长期维护通过新增 migration、运行完整性检查和 Repo smoke tests 完成，而不是直接手改运行时数据库。

