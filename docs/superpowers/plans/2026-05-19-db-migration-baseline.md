# 数据库迁移与 Repo 新 Schema 适配 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将数据库初始化切换为迁移驱动，并在同一阶段把后端 ID 类型与 `src-tauri/src/persistence/migrations/0001_initial_schema.sql` 的 TEXT schema 对齐，保证当前应用功能在干净数据库上可启动、可读写。

**Architecture:** 启动时由 `db_pool.rs` 创建 SQLite 连接池并运行 `migrator::run_migrations()`，迁移 SQL 通过 `include_str!` 编译进二进制。后端 Rust 持久化 ID 统一使用 `String`，与数据库 `TEXT` ID 对齐；时间戳、计数、大小等非 ID 数值字段继续保留数字类型。现有 `UserRepo`、`GroupRepo`、`MessageRepo`、`InteractionRepo` 不再依赖旧的 `users/groups/source_type/source_id` schema，而是直接读写 `im_accounts/chat_groups/messages` 及其关联目标表。

**Tech Stack:** Rust edition 2024, sqlx 0.8, SQLite, Tauri 2, React/Vite frontend

**Architecture Boundary:** Runtime database schema must live under `src-tauri/src/persistence/migrations/*.sql` and be embedded from there with `include_str!`. Files under `docs/specs/database/**` are specification/course documentation and may be kept in sync, but app startup, tests, and build scripts must not load schema SQL from `docs/`.

---

## Scope

本阶段包含：

- 迁移目录与 `0001_initial_schema.sql` 落地。
- 启动时运行迁移并设置 WAL/FK/busy_timeout/synchronous。
- 删除旧的分散 `init_schema` 启动流程。
- 将后端 model/internal event/service/command/repo 中的持久化 ID 从 `u64`/`i64` 调整为 `String`，与数据库 `TEXT` ID 对齐。
- 适配当前已有 Repo 方法到新 schema，并更新前端 TypeScript ID 类型和 invoke payload，使 Tauri 边界传递字符串 ID。
- 添加迁移和 Repo smoke tests，验证干净数据库可建表且现有核心读写路径可用。

本阶段不包含：

- Bot 管理、协议包追踪、调试会话 UI 的完整实现。
- 旧用户数据库的 002-011 数据迁移链。
- 协议包文件落盘、清理、导出和备份恢复。
- 用户注销时的旧数据迁移；本阶段只处理干净数据库 baseline。

---

## File Structure

- Rename: `docs/specs/database/ddl/001_initial_schema.sql` -> `docs/specs/database/ddl/0001_initial_schema.sql`
- Modify: `docs/specs/database/table-dictionary.md`
- Modify: `docs/specs/database/er-model.md`
- Modify: `src-tauri/src/models/entities.rs`
- Modify: `src-tauri/src/models/internal.rs`
- Modify: `src-tauri/src/core.rs`
- Modify: `src-tauri/src/utils.rs`
- Modify: `src-tauri/src/commands/main.rs`
- Modify: `src-tauri/src/commands/chat/*.rs`
- Modify: `src-tauri/src/services/**/*.rs`
- Modify: `src/store/use-auth-store.ts`
- Modify: `src/types/*.ts`
- Modify: `src/lib/**/*.ts`
- Modify: `src/hooks/**/*.ts`
- Modify: `src/components/**/*.tsx`
- Modify: `src/views/**/*.tsx`
- Create: `src-tauri/src/persistence/migrations/0001_initial_schema.sql`
- Create: `src-tauri/src/persistence/migrations/mod.rs`
- Create: `src-tauri/src/persistence/migrator.rs`
- Create: `src-tauri/src/persistence/repo/codecs.rs`
- Modify: `src-tauri/src/persistence/mod.rs`
- Modify: `src-tauri/src/persistence/db_pool.rs`
- Modify: `src-tauri/src/persistence/repo/mod.rs`
- Modify: `src-tauri/src/persistence/repo/user/mod.rs`
- Modify: `src-tauri/src/persistence/repo/user/types.rs`
- Modify: `src-tauri/src/persistence/repo/user/profile.rs`
- Modify: `src-tauri/src/persistence/repo/user/friends.rs`
- Delete: `src-tauri/src/persistence/repo/user/schema.rs`
- Modify: `src-tauri/src/persistence/repo/group/mod.rs`
- Modify: `src-tauri/src/persistence/repo/group/types.rs`
- Modify: `src-tauri/src/persistence/repo/group/basic.rs`
- Modify: `src-tauri/src/persistence/repo/group/requests.rs`
- Modify: `src-tauri/src/persistence/repo/group/content.rs`
- Modify: `src-tauri/src/persistence/repo/group/events.rs`
- Delete: `src-tauri/src/persistence/repo/group/schema.rs`
- Modify: `src-tauri/src/persistence/repo/message.rs`
- Modify: `src-tauri/src/persistence/repo/interaction.rs`

---

### Task 1: Add Migration SQL And Registry

**Files:**
- Rename: `docs/specs/database/ddl/001_initial_schema.sql` -> `docs/specs/database/ddl/0001_initial_schema.sql`
- Modify: `docs/specs/database/table-dictionary.md`
- Modify: `docs/specs/database/er-model.md`
- Create: `src-tauri/src/persistence/migrations/0001_initial_schema.sql`
- Create: `src-tauri/src/persistence/migrations/mod.rs`
- Modify: `src-tauri/src/persistence/mod.rs`

- [ ] **Step 1: Create migration directory**

Run:

```powershell
New-Item -ItemType Directory -Force -Path "src-tauri/src/persistence/migrations"
```

Expected: directory exists.

- [ ] **Step 2: Fix DDL lifecycle status fields, then copy and normalize initial DDL**

Do not model account or group disappearance as physical deletion in this baseline. Retain `im_accounts` and `chat_groups` rows for historical messages, conversations, and protocol debugging. State changes are represented with status columns.

Add lifecycle columns to `im_accounts`:

```sql
account_status TEXT NOT NULL DEFAULT 'active'
               CHECK (account_status IN ('active', 'disabled', 'unavailable', 'deleted')),
unavailable_at INTEGER,
deleted_at     INTEGER,
updated_at     INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
CHECK (
    (account_status = 'unavailable' AND unavailable_at IS NOT NULL)
    OR account_status != 'unavailable'
),
CHECK (
    (account_status = 'deleted' AND deleted_at IS NOT NULL)
    OR account_status != 'deleted'
)
```

Add lifecycle columns to `chat_groups`:

```sql
group_status TEXT NOT NULL DEFAULT 'active'
             CHECK (group_status IN ('active', 'dissolved', 'unavailable')),
dissolved_at   INTEGER,
unavailable_at INTEGER,
CHECK (
    (group_status = 'dissolved' AND dissolved_at IS NOT NULL)
    OR group_status != 'dissolved'
),
CHECK (
    (group_status = 'unavailable' AND unavailable_at IS NOT NULL)
    OR group_status != 'unavailable'
)
```

Require a retained owner row for every group. The owner can be `account_status = 'deleted'` or `account_status = 'unavailable'`, but the FK target row remains:

```sql
-- Before
group_owner_user_id TEXT,
FOREIGN KEY (group_owner_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,

-- After
group_owner_user_id TEXT NOT NULL,
FOREIGN KEY (group_owner_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
```

Keep message identity FKs strict so accidental physical deletes fail instead of erasing fact links:

```sql
-- Before
FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
FOREIGN KEY (receiver_user_id) REFERENCES im_accounts(user_id) ON DELETE SET NULL,
FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE SET NULL,

-- After
FOREIGN KEY (sender_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
FOREIGN KEY (receiver_user_id) REFERENCES im_accounts(user_id) ON DELETE RESTRICT,
FOREIGN KEY (group_id) REFERENCES chat_groups(group_id) ON DELETE RESTRICT,
```

Keep `messages.sender_user_id`, `messages.receiver_user_id`, and group-message `messages.group_id` shape constraints strict. Historical private and group messages continue to reference retained `im_accounts` and `chat_groups` rows, so this CHECK remains valid:

```sql
CHECK (
    (message_scene IN ('private', 'temp') AND receiver_user_id IS NOT NULL AND group_id IS NULL)
    OR (message_scene = 'group' AND group_id IS NOT NULL AND receiver_user_id IS NULL)
)
```

Update `docs/specs/database/table-dictionary.md` and `docs/specs/database/er-model.md` to describe status-based lifecycle behavior:

- `im_accounts.account_status = 'deleted'` means the account was注销/removed from the protocol perspective, but the row remains for message facts and debug traceability.
- `im_accounts.account_status = 'unavailable'` means the account currently cannot be queried from the protocol side.
- `chat_groups.group_status = 'dissolved'` means the group is dissolved, but the row remains for historical messages, conversations, and protocol traces.
- `chat_groups.group_status = 'unavailable'` means group metadata currently cannot be queried from the protocol side.
- Application code must not physically delete `im_accounts` or `chat_groups` for normal account注销/group dissolution flows.

Rename the docs DDL to `docs/specs/database/ddl/0001_initial_schema.sql`, then apply the lifecycle and FK edits above to both `docs/specs/database/ddl/0001_initial_schema.sql` and `src-tauri/src/persistence/migrations/0001_initial_schema.sql`. Treat the `src-tauri` migration file as the runtime authority; the `docs` DDL is a spec mirror for documentation and course deliverables. Then make these migration-specific edits:

```sql
-- Remove these four lines from the migration file.
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

Change the seed setting at the end:

```sql
-- Before
('schema.version', 'v1.0.0', 'string', '当前数据库 schema 版本'),

-- After
('schema.version', '0001', 'string', '当前数据库 schema 迁移版本'),
```

Enforce group content parentage in the database:

```sql
-- group_folders must expose a composite target for same-group parent checks.
UNIQUE (folder_id, group_id)

-- group_files must not be able to point at a parent folder from another group.
FOREIGN KEY (parent_folder_id, group_id) REFERENCES group_folders(folder_id, group_id) ON DELETE CASCADE
```

Remove any automatic `trg_unread_inc` trigger. `conversations.unread_count` is owner-scoped business state; it must be updated only from repo/service paths that know the target `owner_user_id`.

Keep `group_essence_messages` as a snapshot table, not a pure projection of `messages`. Store `group_id`, `message_id`, `sender_user_id`, `operator_user_id`, and `created_at` intentionally so essence entries can still render after recall, sender departure, or `message_id` being detached by `ON DELETE SET NULL`. Do not remove these snapshot columns just to satisfy a normalized view of `messages`.

After the lifecycle/FK edits, pragma removal, schema version seed change, same-group content constraints, unread trigger removal, and essence snapshot comments above, keep unrelated table, index, trigger, and setting DDL unchanged.

- [ ] **Step 3: Add migration registry**

Write `src-tauri/src/persistence/migrations/mod.rs`:

```rust
pub struct Migration {
    pub version: &'static str,
    pub description: &'static str,
    pub sql: &'static str,
}

pub fn all_migrations() -> Vec<Migration> {
    vec![Migration {
        version: "0001",
        description: "initial_schema",
        sql: include_str!("0001_initial_schema.sql"),
    }]
}
```

- [ ] **Step 4: Register modules**

Update `src-tauri/src/persistence/mod.rs`:

```rust
pub mod db_pool;
pub mod migrations;
pub mod migrator;
pub mod repo;

pub use db_pool::init_sqlite_pool;
pub use repo::{
    GroupEventRecord, GroupRepo, InteractionRepo, MessageRecord, MessageRepo,
    NewFriendRequestRecord, NewGroupEventRecord, NewGroupRequestRecord, NewMessageReactionRecord,
    NewMessageRecord, NewPokeRecord, UserRepo,
};
```

- [ ] **Step 5: Verify compile sees the embedded SQL**

Run:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: fails only because `migrator` module is not implemented yet, not because the SQL file path is missing.

- [ ] **Step 6: Commit**

```bash
git add docs/specs/database/ddl/0001_initial_schema.sql docs/specs/database/table-dictionary.md docs/specs/database/er-model.md src-tauri/src/persistence/migrations src-tauri/src/persistence/mod.rs
git commit -m "feat(db): add initial migration registry"
```

---

### Task 2: Implement Migrator With Correct SQL Statement Splitting

**Files:**
- Create: `src-tauri/src/persistence/migrator.rs`

- [ ] **Step 1: Implement migrator**

Write `src-tauri/src/persistence/migrator.rs` with these requirements:

- `run_migrations(pool)` reads `app_settings.schema.version`; if `app_settings` does not exist, current version is `"0000"`.
- Pending migrations are those whose version is greater than current version.
- Each migration runs inside one transaction.
- After a migration succeeds, update `app_settings.setting_value`, `description`, and `updated_at`.
- Do not use a naive `;` split. The splitter must preserve `CREATE TRIGGER ... BEGIN ... END;` blocks and ignore semicolons inside `'...'`, `"..."`, `-- ...`, and `/* ... */`.

Use this skeleton:

```rust
use sqlx::SqlitePool;

use super::migrations::{self, Migration};

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), String> {
    let current_version = read_schema_version(pool).await?;
    let pending: Vec<Migration> = migrations::all_migrations()
        .into_iter()
        .filter(|migration| migration.version > current_version.as_str())
        .collect();

    for migration in pending {
        apply_migration(pool, &migration).await?;
    }

    Ok(())
}

async fn read_schema_version(pool: &SqlitePool) -> Result<String, String> {
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_settings'",
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("failed to check app_settings existence: {err}"))?;

    if table_exists == 0 {
        return Ok("0000".to_string());
    }

    let version: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("failed to read schema version: {err}"))?;

    Ok(version.unwrap_or_else(|| "0000".to_string()))
}

async fn apply_migration(pool: &SqlitePool, migration: &Migration) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| format!("migration {}: failed to begin tx: {err}", migration.version))?;

    for (index, statement) in split_sql_statements(migration.sql).iter().enumerate() {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                format!(
                    "migration {} statement {} failed: {err}\n{statement}",
                    migration.version,
                    index + 1
                )
            })?;
    }

    sqlx::query(
        "UPDATE app_settings
         SET setting_value = ?1,
             description = '当前数据库 schema 迁移版本',
             updated_at = unixepoch() * 1000
         WHERE setting_key = 'schema.version'",
    )
    .bind(migration.version)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("migration {}: failed to update version: {err}", migration.version))?;

    tx.commit()
        .await
        .map_err(|err| format!("migration {}: failed to commit: {err}", migration.version))?;

    Ok(())
}
```

Implement `split_sql_statements(sql: &str) -> Vec<String>` as a state machine with a lexical state plus a separate trigger-body flag. `in_trigger_body` must not be encoded as another lexical state, because trigger bodies can contain quoted strings and comments.

```rust
enum ScanState {
    Normal,
    SingleQuoted,
    DoubleQuoted,
    LineComment,
    BlockComment,
}
```

Maintain:

```rust
let mut state = ScanState::Normal;
let mut in_trigger_body = false;
```

Set `in_trigger_body = true` when the current statement starts with `CREATE TRIGGER`, ignoring leading comments and whitespace. While `in_trigger_body` is true, do not split on semicolons unless `state == ScanState::Normal` and the semicolon immediately follows a standalone top-level `END` token. Semicolons inside trigger strings, line comments, or block comments are ignored by the splitter.

- [ ] **Step 2: Add splitter tests**

Add tests in `migrator.rs`:

```rust
#[test]
fn splits_simple_statements() {
    assert_eq!(split_sql_statements("SELECT 1; SELECT 2;"), vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn preserves_trigger_body_semicolons() {
    let sql = "CREATE TRIGGER foo AFTER INSERT ON t FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'END; still string'); -- END; comment\nUPDATE t SET a = 1; END;";
    let result = split_sql_statements(sql);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("RAISE(ABORT, 'END; still string');"));
    assert!(result[0].contains("UPDATE t SET a = 1;"));
}

#[test]
fn ignores_semicolons_in_strings_and_comments() {
    let sql = "SELECT ';'; -- comment ;\nSELECT \"x;y\"; /* block ; */ SELECT 3;";
    let result = split_sql_statements(sql);
    assert_eq!(result.len(), 3);
}

#[test]
fn parses_initial_schema_as_expected() {
    let statements = split_sql_statements(crate::persistence::migrations::all_migrations()[0].sql);
    assert!(statements.iter().any(|stmt| stmt.contains("CREATE TABLE IF NOT EXISTS im_accounts")));
    assert!(statements.iter().any(|stmt| stmt.contains("CREATE TRIGGER IF NOT EXISTS trg_member_count_inc")));
    assert!(statements.iter().any(|stmt| stmt.contains("FOREIGN KEY (parent_folder_id, group_id) REFERENCES group_folders(folder_id, group_id)")));
    assert!(!statements.iter().any(|stmt| stmt.contains("CREATE TRIGGER IF NOT EXISTS trg_unread_inc")));
    assert!(statements.iter().any(|stmt| stmt.contains("INSERT INTO app_settings")));
}
```

- [ ] **Step 3: Add in-memory migration test**

Add this async test in `migrator.rs`:

```rust
#[sqlx::test]
async fn applies_initial_schema(pool: SqlitePool) -> Result<(), sqlx::Error> {
    run_migrations(&pool).await.map_err(sqlx::Error::Protocol)?;

    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(table_count, 26);

    let version: String = sqlx::query_scalar(
        "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(version, "0001");

    run_migrations(&pool).await.map_err(sqlx::Error::Protocol)?;
    Ok(())
}
```

- [ ] **Step 4: Run migrator tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml persistence::migrator
```

Expected: all migrator tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/persistence/migrator.rs
git commit -m "feat(db): add migration runner"
```

---

### Task 3: Switch Database Startup To Migrator

**Files:**
- Modify: `src-tauri/src/persistence/db_pool.rs`

- [ ] **Step 1: Replace scattered schema initialization**

Update `db_pool.rs`:

```rust
use std::fs;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use tauri::Manager;

use super::migrator;

pub async fn init_sqlite_pool(app: &tauri::AppHandle) -> Result<SqlitePool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data dir: {err}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|err| format!("failed to create app data dir: {err}"))?;

    let db_path = app_data_dir.join("unibot.db");
    let connect_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .map_err(|err| format!("failed to connect sqlite: {err}"))?;

    migrator::run_migrations(&pool)
        .await
        .map_err(|err| format!("database migration failed: {err}"))?;

    Ok(pool)
}
```

- [ ] **Step 2: Run compile check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compile reaches Repo errors that are caused by old schema assumptions. Those are fixed in later tasks.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/persistence/db_pool.rs
git commit -m "refactor(db): initialize database through migrator"
```

---

### Task 4: Align Backend And Frontend ID Types To TEXT

**Files:**
- Create: `src-tauri/src/persistence/repo/codecs.rs`
- Modify: `src-tauri/src/persistence/repo/mod.rs`
- Modify: `src-tauri/src/models/entities.rs`
- Modify: `src-tauri/src/models/internal.rs`
- Modify: `src-tauri/src/core.rs`
- Modify: `src-tauri/src/utils.rs`
- Modify: `src-tauri/src/commands/main.rs`
- Modify: `src-tauri/src/commands/chat/user.rs`
- Modify: `src-tauri/src/commands/chat/request.rs`
- Modify: `src-tauri/src/commands/chat/message.rs`
- Modify: `src-tauri/src/commands/chat/group.rs`
- Modify: `src-tauri/src/services/**/*.rs`
- Modify: `src/store/use-auth-store.ts`
- Modify: `src/types/*.ts`
- Modify: `src/lib/**/*.ts`
- Modify: `src/hooks/**/*.ts`
- Modify: `src/components/**/*.tsx`
- Modify: `src/views/**/*.tsx`

- [ ] **Step 1: Change persistent Rust IDs to `String`**

In `src-tauri/src/models/entities.rs`, change every persistent ID field that maps to a database `TEXT` ID column from `u64` or `i64` to `String`.

```rust
pub type DbId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Disabled,
    Unavailable,
    Deleted,
}

impl Default for AccountStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    Active,
    Dissolved,
    Unavailable,
}

impl Default for GroupStatus {
    fn default() -> Self {
        Self::Active
    }
}
```

Use `DbId` for:

- `UserProfile.user_id`
- `GroupProfile.group_id`, `GroupProfile.owner_user_id`
- `GroupMemberProfile.group_id`, `GroupMemberProfile.user_id`
- `MessageSource::Private.peer_user_id`, `MessageSource::Group.group_id`
- `MessageEntity.message_id`, `MessageEntity.quoted_message_id`, `MessageEntity.recall.recalled_by_user_id`
- `MessageEntity.sender_user_id`
- `MessageReactionEntity.reaction_id`, `message_id`, `operator_user_id`
- `PokeEntity.poke_id`, `sender_user_id`, `target_user_id`
- `GroupAnnouncementEntity.group_id`, `sender_user_id`
- `GroupFileEntity.group_id`, `uploader_user_id`
- `GroupFolderEntity.group_id`, `creator_user_id`
- `FriendRequestEntity.request_id`, `initiator_user_id`, `target_user_id`
- `GroupRequestEntity.request_id`, `group_id`, `initiator_user_id`, `target_user_id`, `operator_user_id`
- `GroupWholeMuteState.group_id`, `operator_user_id`
- `GroupEssenceMessageEntity.essence_id`, `group_id`, `message_id`, `sender_user_id`, `operator_user_id`
- all ID fields inside `GroupEventPayload`
- `GroupEventEntity.event_id`, `group_id`

Add lifecycle status to the service-facing profiles so history and debug UI can render retained rows correctly:

```rust
pub struct UserProfile {
    pub user_id: DbId,
    pub nickname: String,
    pub avatar: String,
    pub signature: String,
    #[serde(default)]
    pub account_status: AccountStatus,
}

pub struct GroupProfile {
    pub group_id: DbId,
    pub group_name: String,
    pub owner_user_id: DbId,
    #[serde(default)]
    pub member_count: u32,
    pub max_member_count: u32,
    #[serde(default)]
    pub group_status: GroupStatus,
}
```

Update DB-backed enums that are now stored as `TEXT` to avoid direct integer SQL mapping. Remove `sqlx::Type`, `#[repr(i64)]`, and `#[sqlx(type_name = "INTEGER")]` from `GroupRole`, `RequestState`, and `GroupRequestType`; keep them as serde enums and map them through `repo::codecs`:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    Pending,
    Accepted,
    Rejected,
    Ignored,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupRequestType {
    Join,
    Invite,
}
```

Keep these fields numeric:

- timestamps such as `created_at`, `updated_at`, `handled_at`
- counters such as `member_count`, `max_member_count`, `file_count`, `download_count`
- sizes such as `file_size`
- non-persistent values such as `duration_seconds`, `limit`, `face_id`, and booleans

Update `MessageSource` helpers to use borrowed string IDs:

```rust
impl MessageSource {
    pub fn to_db_parts(&self) -> (&'static str, &str) {
        match self {
            MessageSource::Private { peer_user_id } => ("private", peer_user_id.as_str()),
            MessageSource::Group { group_id } => ("group", group_id.as_str()),
        }
    }
}

impl TryFrom<(&str, String)> for MessageSource {
    type Error = String;

    fn try_from(value: (&str, String)) -> Result<Self, Self::Error> {
        let (source_type, source_id) = value;
        match source_type {
            "private" => Ok(MessageSource::Private {
                peer_user_id: source_id,
            }),
            "group" => Ok(MessageSource::Group {
                group_id: source_id,
            }),
            _ => Err(format!("unknown source type: {source_type}")),
        }
    }
}
```

Row mapping code should pass owned `String` source IDs into `MessageSource::try_from((row.source_type.as_str(), row.source_id))`.

- [ ] **Step 2: Change core, utilities, internal events, commands, and services to String IDs**

In `src-tauri/src/models/internal.rs`, change every event payload ID that references an account, group, message, request, file, folder, reaction, poke, or essence row to `String`.

In `src-tauri/src/core.rs`, change registered user storage from numeric keys to string keys:

```rust
users: RwLock<HashMap<String, UserContext>>
```

Use `String` for owned IDs and `&str` for lookup-only arguments:

```rust
pub fn unregister_user(&self, user_id: &str) -> Option<UserContext>
pub fn user_context(&self, user_id: &str) -> Option<UserContext>
pub fn require_user_context(&self, user_id: &str) -> AppResult<UserContext>
```

`open_user_chat_window` should accept `user_id: String` and reject only `user_id.trim().is_empty()`.

In `src-tauri/src/utils.rs`, change recipient helpers to string IDs:

```rust
pub fn emit_to_users<I, S>(core: &CoreContainer, user_ids: I, event: InternalEvent)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
```

`recipients_for_source` should return `HashSet<String>`, accept `actor_user_id: &str` and `other_user_id: Option<&str>`, and clone member IDs into the set.

In all Tauri command functions under `src-tauri/src/commands`, change ID parameters from `u64`/`i64` to `String` where the parameter maps to a persistent database ID:

```rust
pub async fn get_user_by_id(user_id: String, state: State<'_, CoreContainer>) -> AppResult<Option<UserProfile>>
```

Do the same for service and repo method signatures that receive or return persistent IDs. The app no longer parses user/group/message/request IDs as numbers in backend code.

- [ ] **Step 3: Update frontend ID types and command payloads**

In `src/types/*.ts`, change persistent ID fields from `number` to `string`. For example:

```ts
export type AccountStatus = "active" | "disabled" | "unavailable" | "deleted";
export type GroupStatus = "active" | "dissolved" | "unavailable";

export type MessageSource =
  | { scene: "private"; peer_user_id: string }
  | { scene: "group"; group_id: string };

export interface UserProfile {
  user_id: string;
  nickname: string;
  avatar: string;
  signature: string;
  account_status: AccountStatus;
}

export interface GroupProfile {
  group_id: string;
  group_name: string;
  owner_user_id: string;
  member_count: number;
  max_member_count: number;
  group_status: GroupStatus;
}
```

Update frontend call sites so generated or user-entered IDs are strings before invoking Tauri commands. Remove `Number(...)`, `parseInt(...)`, and numeric sentinels for persistent IDs. Use `""` or `null` only where the data model already permits absence.

In `src/store/use-auth-store.ts`, change auth state to string IDs:

```ts
type AuthState = {
  currentUserId: string | null;
  setCurrentUserId: (userId: string | null) => void;
};
```

Replace `currentUserId ?? -1` and `Number.isInteger(currentUserId)` guards with string presence checks such as `if (!currentUserId)`.

- [ ] **Step 4: Add enum codec helpers**

Create `src-tauri/src/persistence/repo/codecs.rs` for enum/string mapping only:

```rust
use crate::models::{AccountStatus, GroupRequestType, GroupRole, GroupStatus, RequestState};

pub fn account_status_from_db(value: &str) -> Result<AccountStatus, sqlx::Error> {
    match value {
        "active" => Ok(AccountStatus::Active),
        "disabled" => Ok(AccountStatus::Disabled),
        "unavailable" => Ok(AccountStatus::Unavailable),
        "deleted" => Ok(AccountStatus::Deleted),
        _ => Err(sqlx::Error::Protocol(format!("unknown account status: {value}"))),
    }
}

pub fn group_status_from_db(value: &str) -> Result<GroupStatus, sqlx::Error> {
    match value {
        "active" => Ok(GroupStatus::Active),
        "dissolved" => Ok(GroupStatus::Dissolved),
        "unavailable" => Ok(GroupStatus::Unavailable),
        _ => Err(sqlx::Error::Protocol(format!("unknown group status: {value}"))),
    }
}

pub fn group_role_to_db(role: GroupRole) -> &'static str {
    match role {
        GroupRole::Owner => "owner",
        GroupRole::Admin => "admin",
        GroupRole::Member => "member",
    }
}

pub fn group_role_from_db(value: &str) -> Result<GroupRole, sqlx::Error> {
    match value {
        "owner" => Ok(GroupRole::Owner),
        "admin" => Ok(GroupRole::Admin),
        "member" => Ok(GroupRole::Member),
        _ => Err(sqlx::Error::Protocol(format!("unknown group role: {value}"))),
    }
}

pub fn request_state_to_db(state: RequestState) -> &'static str {
    match state {
        RequestState::Pending => "pending",
        RequestState::Accepted => "accepted",
        RequestState::Rejected => "rejected",
        RequestState::Ignored => "ignored",
    }
}

pub fn request_state_from_db(value: &str) -> Result<RequestState, sqlx::Error> {
    match value {
        "pending" => Ok(RequestState::Pending),
        "accepted" => Ok(RequestState::Accepted),
        "rejected" => Ok(RequestState::Rejected),
        "ignored" => Ok(RequestState::Ignored),
        _ => Err(sqlx::Error::Protocol(format!("unknown request state: {value}"))),
    }
}

pub fn group_request_type_to_db(value: GroupRequestType) -> &'static str {
    match value {
        GroupRequestType::Join => "join",
        GroupRequestType::Invite => "invite",
    }
}

pub fn group_request_type_from_db(value: &str) -> Result<GroupRequestType, sqlx::Error> {
    match value {
        "join" => Ok(GroupRequestType::Join),
        "invite" => Ok(GroupRequestType::Invite),
        _ => Err(sqlx::Error::Protocol(format!("unknown group request type: {value}"))),
    }
}
```

- [ ] **Step 5: Document repo-wide ID SQL rules in the task notes**

Use these rules in every Repo task below:

- Bind backend ID parameters directly as `String` into target `TEXT` ID columns.
- Select target `TEXT` IDs directly into `String` row fields. Do not `CAST(... AS INTEGER)` for persistent IDs.
- `Option<String>` is used only for nullable ID columns, such as `quoted_message_id`, `recalled_by_user_id`, optional request targets, and nullable operator fields.
- For generated IDs, compute the ID once in the same write statement with a CTE and reuse that CTE value. Do not repeat `MAX(CAST(...)) + 1` inline inside one `INSERT`.

Use this CTE shape for generated string primary keys that remain sequential for compatibility with existing demo flows:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(id_column AS INTEGER)), 0) + 1 AS TEXT)
    FROM table_name
)
INSERT INTO table_name (id_column, other_column)
SELECT value, ?1
FROM next_id
RETURNING id_column AS id
```

- [ ] **Step 6: Register helper module**

Update `src-tauri/src/persistence/repo/mod.rs`:

```rust
mod codecs;
pub mod group;
pub mod interaction;
pub mod message;
pub mod user;
```

- [ ] **Step 7: Run focused checks**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
bun run build
```

Expected: remaining Rust errors are old Repo SQL/schema references that are fixed in later tasks; TypeScript has no number/string ID type mismatch in already-updated call sites.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/models src-tauri/src/commands src-tauri/src/services src-tauri/src/persistence/repo/codecs.rs src-tauri/src/persistence/repo/mod.rs src/types src/lib src/hooks src/components src/views
git commit -m "refactor(db): align app ids with text schema"
```

---

### Task 5: Adapt UserRepo To im_accounts And Owner-View Friendships

**Files:**
- Modify: `src-tauri/src/persistence/repo/user/mod.rs`
- Modify: `src-tauri/src/persistence/repo/user/types.rs`
- Modify: `src-tauri/src/persistence/repo/user/profile.rs`
- Modify: `src-tauri/src/persistence/repo/user/friends.rs`
- Delete: `src-tauri/src/persistence/repo/user/schema.rs`

- [ ] **Step 1: Remove old schema module**

Delete `src-tauri/src/persistence/repo/user/schema.rs`, then remove `mod schema;` from `user/mod.rs`.

- [ ] **Step 2: Update user row types**

Change `UserRow` fields to DB text IDs and target column names:

```rust
pub(super) struct UserRow {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: String,
    pub signature: String,
    pub account_status: String,
}
```

Change `FriendRequestRow` so `request_id`, user IDs, and `state` are strings:

```rust
pub(super) struct FriendRequestRow {
    pub request_id: String,
    pub initiator_user_id: String,
    pub target_user_id: String,
    pub comment: Option<String>,
    pub state: String,
    pub created_at: u64,
    pub handled_at: Option<u64>,
}
```

Long-term clean friend request model: do not store or expose `operator_user_id` on `friend_requests` / `FriendRequestEntity` / TypeScript friend request types. A pending request has no handler; after handling, the handler is derivable from the invariant that only `target_user_id` can handle the request. Service/event code may use the current `user_id` at handling time for immediate notification payloads, but this is not persisted as a separate database fact.

Apply that model concretely:

- In `src-tauri/src/models/entities.rs`, remove `operator_user_id` from `FriendRequestEntity`. Keep `operator_user_id` on `InternalEvent::FriendRequestHandled`, because the service has the current handler `user_id` when emitting the event.
- In `src-tauri/src/persistence/repo/user/types.rs`, remove `operator_user_id` from `FriendRequestRow` and from `TryFrom<FriendRequestRow> for FriendRequestEntity`.
- In `src-tauri/src/persistence/repo/user/friends.rs`, remove the `operator_user_id` parameter from `handle_friend_request_for_target`. The repo should update only `state` and `handled_at`, guarded by `target_user_id`; it should not write, select, return, or synthesize `operator_user_id`.
- In `src-tauri/src/services/request.rs`, keep using the current `user_id` as the handler for `InternalEvent::FriendRequestHandled`, but do not expect it in the returned `FriendRequestEntity`.
- In `src/types/request.ts`, remove `operator_user_id` from the frontend `FriendRequestEntity` type. Keep `operator_user_id` on group request and event types where it is an independent fact.

Implement `TryFrom<UserRow> for UserProfile` by moving string IDs directly and converting `account_status` through `codecs::account_status_from_db`. Implement `TryFrom<FriendRequestRow> for FriendRequestEntity` only for enum conversion with `codecs::request_state_from_db`; keep request and user IDs as `String`.

- [ ] **Step 3: Adapt profile SQL**

Use `im_accounts` in `profile.rs`:

```sql
INSERT INTO im_accounts (
    user_id, nickname, avatar_url, signature, account_source,
    account_status, unavailable_at, deleted_at
)
VALUES (?1, ?2, ?3, ?4, 'simulated', 'active', NULL, NULL)
ON CONFLICT(user_id) DO UPDATE SET
    nickname = excluded.nickname,
    avatar_url = excluded.avatar_url,
    signature = excluded.signature,
    account_status = 'active',
    unavailable_at = NULL,
    deleted_at = NULL,
    updated_at = unixepoch() * 1000
```

After upserting an account, ensure default friend and group categories exist:

```sql
INSERT OR IGNORE INTO friend_categories (category_id, owner_user_id, name, sort_order)
VALUES (?1, ?2, '默认分组', 0)
```

Use `category_id = format!("{user_id}:friend:default")`.

Also create the default group category:

```sql
INSERT OR IGNORE INTO group_categories (category_id, owner_user_id, name, sort_order)
VALUES (?1, ?2, '默认分组', 0)
```

Use `category_id = format!("{user_id}:group:default")`.

List/get should read from `im_accounts`, selecting `avatar_url` and `account_status`. Default list queries should exclude `account_status = 'deleted'` so normal account pickers do not show logged-out/removed accounts. Direct `get_user_by_id` should not filter by status, because history/debug views need the retained row.

List users:

```sql
SELECT user_id, nickname, avatar_url, signature, account_status
FROM im_accounts
WHERE account_status != 'deleted'
ORDER BY created_at ASC
```

Get one user:

```sql
SELECT user_id, nickname, avatar_url, signature, account_status
FROM im_accounts
WHERE user_id = ?1
```

`delete_user` should not delete the row. It should mark the account as deleted and keep the row available for message FKs and debug traceability:

```sql
UPDATE im_accounts
SET account_status = 'deleted',
    deleted_at = unixepoch() * 1000,
    updated_at = unixepoch() * 1000
WHERE user_id = ?1
```

In the same transaction, dissolve groups owned by that account. This implements the rule that an account注销/removal dissolves owned groups, while disabled or temporarily unavailable accounts do not:

```sql
UPDATE chat_groups
SET group_status = 'dissolved',
    dissolved_at = COALESCE(dissolved_at, unixepoch() * 1000),
    updated_at = unixepoch() * 1000
WHERE group_owner_user_id = ?1
  AND group_status != 'dissolved'
```

Runtime unregister/logout code may remove in-memory session state, but persistence must retain `im_accounts.user_id`.

- [ ] **Step 4: Adapt friend request SQL**

Use `friend_requests.request_id TEXT` and text states:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(request_id AS INTEGER)), 0) + 1 AS TEXT)
    FROM friend_requests
)
INSERT INTO friend_requests (
    request_id, initiator_user_id, target_user_id, comment, state, created_at
) SELECT value, ?1, ?2, ?3, 'pending', ?4
FROM next_id
RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
```

In `handle_friend_request_for_target`, update only columns that exist in the DDL. Do not add or synthesize `operator_user_id` for friend requests, and do not accept `operator_user_id` as a repo parameter:

```sql
UPDATE friend_requests
SET state = ?2,
    handled_at = ?3
WHERE request_id = ?1
  AND state = 'pending'
  AND target_user_id = ?4
RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
```

Bind request state with `codecs::request_state_to_db(state)`. Bind user IDs as `String`. Keep the current handling `user_id` in service scope for `InternalEvent::FriendRequestHandled`, but do not route it through repo rows.

- [ ] **Step 5: Adapt friendships to owner-view rows**

When accepting a friend request, insert two rows:

```sql
INSERT OR IGNORE INTO friendships (
    owner_user_id, friend_user_id, friend_category_id, created_at
) VALUES (?1, ?2, ?3, ?4)
```

Use category IDs:

```rust
format!("{owner_user_id}:friend:default")
```

`are_friends(user_a, user_b)` should check:

```sql
SELECT EXISTS(
    SELECT 1 FROM friendships
    WHERE owner_user_id = ?1 AND friend_user_id = ?2
)
```

`remove_friendship_pair(user_a, user_b)` should delete both owner-view rows.

- [ ] **Step 6: Run focused check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: remaining compile errors are in group/message/interaction repos, not user repo.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/persistence/repo/user
git commit -m "refactor(db): adapt user repo to im_accounts schema"
```

---

### Task 6: Adapt GroupRepo Basic Data To chat_groups

**Files:**
- Modify: `src-tauri/src/persistence/repo/group/mod.rs`
- Modify: `src-tauri/src/persistence/repo/group/types.rs`
- Modify: `src-tauri/src/persistence/repo/group/basic.rs`
- Delete: `src-tauri/src/persistence/repo/group/schema.rs`

- [ ] **Step 1: Remove old schema module**

Delete `src-tauri/src/persistence/repo/group/schema.rs`, then remove `mod schema;` from `group/mod.rs`.

- [ ] **Step 2: Update row types**

Use text IDs and text role:

```rust
pub(super) struct GroupRow {
    pub group_id: String,
    pub group_name: String,
    pub owner_user_id: String,
    pub member_count: u32,
    pub max_member_count: u32,
    pub group_status: String,
}

pub(super) struct GroupMemberRow {
    pub group_id: String,
    pub user_id: String,
    pub card: String,
    pub special_title: String,
    pub role: String,
    pub joined_at: u64,
    pub last_sent_at: u64,
    pub mute_until: Option<u64>,
}

pub(super) struct GroupWholeMuteRow {
    pub group_id: String,
    pub muted: bool,
    pub mute_until: Option<u64>,
    pub operator_user_id: Option<String>,
    pub updated_at: u64,
}
```

Convert group rows through `TryFrom<GroupRow> for GroupProfile`, moving string IDs directly and converting `group_status` through `codecs::group_status_from_db`. Convert member rows by moving string IDs directly and mapping role through `codecs::group_role_from_db`. `owner_user_id` is non-null because the owner account row is retained and lifecycle changes are stored through `im_accounts.account_status`.

- [ ] **Step 3: Adapt group CRUD SQL**

Use `chat_groups`:

```sql
INSERT INTO chat_groups (
    group_id, group_name, group_owner_user_id, group_source, max_member_count,
    group_status, dissolved_at, unavailable_at
)
VALUES (?1, ?2, ?3, 'simulated', ?4, 'active', NULL, NULL)
ON CONFLICT(group_id) DO UPDATE SET
    group_name = excluded.group_name,
    group_owner_user_id = excluded.group_owner_user_id,
    max_member_count = excluded.max_member_count,
    group_status = 'active',
    dissolved_at = NULL,
    unavailable_at = NULL,
    updated_at = unixepoch() * 1000
```

`list_groups`, `list_user_groups`, `get_group`, `update_group_name`, and `delete_group` should use `chat_groups`. Default list queries should exclude `group_status = 'dissolved'` so normal group lists do not show dissolved groups. Direct `get_group` should not filter by status, because history/debug views need the retained row.

List all groups:

```sql
SELECT group_id,
       group_name,
       group_owner_user_id AS owner_user_id,
       member_count,
       max_member_count,
       group_status
FROM chat_groups
WHERE group_status != 'dissolved'
ORDER BY created_at ASC
```

List groups for a user:

```sql
SELECT g.group_id,
       g.group_name,
       g.group_owner_user_id AS owner_user_id,
       g.member_count,
       g.max_member_count,
       g.group_status
FROM user_groups ug
JOIN chat_groups g ON g.group_id = ug.group_id
WHERE ug.owner_user_id = ?1
  AND g.group_status != 'dissolved'
ORDER BY ug.sort_order ASC, g.created_at ASC
```

Get one group:

```sql
SELECT group_id,
       group_name,
       group_owner_user_id AS owner_user_id,
       member_count,
       max_member_count,
       group_status
FROM chat_groups
WHERE group_id = ?1
```

`delete_group` should represent group dissolution by status update instead of physical deletion:

```sql
UPDATE chat_groups
SET group_status = 'dissolved',
    dissolved_at = unixepoch() * 1000,
    updated_at = unixepoch() * 1000
WHERE group_id = ?1
```

Historical messages keep `messages.group_id` pointing at the retained `chat_groups` row.

- [ ] **Step 4: Adapt group member SQL**

Use `special_title` and role strings:

```sql
INSERT INTO group_members (
    group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
ON CONFLICT(group_id, user_id) DO UPDATE SET
    card = excluded.card,
    special_title = excluded.special_title,
    role = excluded.role,
    joined_at = excluded.joined_at,
    last_sent_at = excluded.last_sent_at,
    mute_until = excluded.mute_until
```

Bind `role` with `codecs::group_role_to_db(member.role)`. Bind `group_id`, `user_id`, and `mute_operator_user_id` as `String`.

All member queries should select:

```sql
SELECT group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
FROM group_members
```

- [ ] **Step 5: Adapt whole-group mute**

The new schema stores whole mute on `chat_groups`; remove use of `group_whole_mute`.

Update:

```sql
UPDATE chat_groups
SET is_whole_muted = ?2,
    mute_until = ?3,
    mute_operator_user_id = ?4,
    updated_at = ?5
WHERE group_id = ?1
RETURNING group_id, is_whole_muted AS muted, mute_until, mute_operator_user_id AS operator_user_id, updated_at
```

Read:

```sql
SELECT group_id, is_whole_muted AS muted, mute_until, mute_operator_user_id AS operator_user_id, updated_at
FROM chat_groups
WHERE group_id = ?1
```

- [ ] **Step 6: Run focused check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: remaining compile errors are in group requests/content/events, message, or interaction repos.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/persistence/repo/group
git commit -m "refactor(db): adapt group basics to chat_groups schema"
```

---

### Task 7: Adapt Group Requests, Events, And Content Tables

**Files:**
- Modify: `src-tauri/src/persistence/repo/group/types.rs`
- Modify: `src-tauri/src/persistence/repo/group/requests.rs`
- Modify: `src-tauri/src/persistence/repo/group/content.rs`
- Modify: `src-tauri/src/persistence/repo/group/events.rs`

- [ ] **Step 1: Adapt group request rows**

Represent `notification_seq` as the existing public `request_id`:

```rust
pub(super) struct GroupRequestRow {
    pub id: String,
    pub group_id: String,
    pub request_type: String,
    pub initiator_user_id: String,
    pub target_user_id: Option<String>,
    pub comment: Option<String>,
    pub state: String,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<String>,
}
```

SQL aliases must map:

```sql
notification_seq AS id,
notification_type AS request_type
```

Convert through `codecs::group_request_type_from_db` and `codecs::request_state_from_db`.

- [ ] **Step 2: Adapt group request insert/update**

Insert:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(notification_seq AS INTEGER)), 0) + 1 AS TEXT)
    FROM group_requests
)
INSERT INTO group_requests (
    group_id, notification_seq, notification_type, initiator_user_id,
    target_user_id, comment, state, created_at
) SELECT
    ?1,
    value,
    ?2, ?3, ?4, ?5, 'pending', ?6
FROM next_id
RETURNING notification_seq AS id, group_id, notification_type AS request_type,
          initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
```

Handle by `notification_seq = ?1` and `state = 'pending'`, then insert group member using role string `'member'`.

- [ ] **Step 3: Adapt group events**

Use `group_events(event_id, event_type, payload_json)`:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(event_id AS INTEGER)), 0) + 1 AS TEXT)
    FROM group_events
)
INSERT INTO group_events (event_id, group_id, event_type, payload_json, created_at)
SELECT value, ?1, 'generic', ?2, ?3
FROM next_id
RETURNING event_id AS id, group_id, payload_json AS payload, created_at
```

List:

```sql
SELECT event_id AS id, group_id, payload_json AS payload, created_at
FROM group_events
WHERE group_id = ?1
ORDER BY created_at DESC
LIMIT ?2
```

- [ ] **Step 4: Adapt content table column names**

Required mappings:

- `group_files.created_at AS uploaded_at`
- `group_files.download_count` should be selected and preserved in `GroupFileEntity`
- `group_folders.parent_folder_id` is nullable; bind `None` when the incoming parent is `""` or `"/"`
- `group_folders.parent_folder_id` should be selected as `COALESCE(parent_folder_id, '') AS parent_folder_id`
- `group_folders` and `group_files` must preserve the database same-group invariant: a file/folder parent may only point at a folder in the same `group_id`.
- `group_essence_messages.essence_id` is TEXT; select `essence_id AS id`
- `group_essence_messages` has no `is_set`; list rows as `1 AS is_set`
- `group_essence_messages` is a snapshot table. Keep `group_id` and `sender_user_id` as stored snapshot columns even though `message_id` can often derive them while the source message exists.

For `create_group_essence_message`:

- If `is_set == true`, upsert a row with generated string `essence_id` using the same one-CTE ID rule from Task 4.
- If `is_set == false`, delete the `(group_id, message_id)` row and return a synthetic `GroupEssenceMessageEntity` with `is_set: false`.

- [ ] **Step 5: Run focused check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: group repo compiles; remaining errors are in message or interaction repos.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/persistence/repo/group
git commit -m "refactor(db): adapt group request and content repos"
```

---

### Task 8: Adapt MessageRepo To messages And conversations

**Files:**
- Modify: `src-tauri/src/persistence/repo/message.rs`
- Modify: `src-tauri/src/services/message.rs`

- [ ] **Step 1: Remove old init_schema**

Delete the `MessageRepo::init_schema` method. Startup schema now comes only from migrations.

- [ ] **Step 2: Update record shape**

Change `MessageRecord` and `NewMessageRecord` persistent ID fields to `String`, matching Task 4. Both write and read records keep sender IDs non-null because account rows are retained with `account_status` instead of being physically deleted:

```rust
pub struct NewMessageRecord {
    pub owner_user_id: String,
    pub sender_user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub content_json: String,
    pub quoted_message_id: Option<String>,
    pub created_at: u64,
}

pub struct MessageRecord {
    pub id: String,
    pub sender_user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub receiver_user_id: Option<String>,
    pub group_id: Option<String>,
    pub content_json: String,
    pub quoted_message_id: Option<String>,
    pub is_recalled: bool,
    pub recalled_by_user_id: Option<String>,
    pub created_at: u64,
}
```

`NewMessageRecord.owner_user_id` is the conversation owner whose conversation row should be updated. In the current send-message service path it is the current user ID and usually equals `sender_user_id`; keep it separate so future received/imported protocol messages can have `sender_user_id != owner_user_id`. `NewMessageRecord.sender_user_id` and `MessageRecord.sender_user_id` stay non-null because sending and reading historical messages both rely on retained `im_accounts` rows. A deleted or unavailable account is rendered from `account_status`, not from a missing sender FK.

`MessageRecord.receiver_user_id` and `MessageRecord.group_id` are internal row fields used for validation and owner-scoped source reconstruction. They are not exposed in `MessageEntity` or `SendMessageResult`.

Update `MessageService::send_message` construction of `NewMessageRecord`:

```rust
NewMessageRecord {
    owner_user_id: user_id.clone(),
    sender_user_id: user_id.clone(),
    source_type: source_type.to_string(),
    source_id: source_id.to_string(),
    content_json,
    quoted_message_id,
    created_at: now,
}
```

Query aliases must map new columns without numeric casts:

```sql
message_id AS id,
message_scene AS source_type,
peer_id AS source_id
```

For owner-scoped private history, do not return raw `peer_id` as `source_id`. Compute the other participant relative to the requested owner so `MessageSource::Private.peer_user_id` remains the conversation peer from the caller's perspective.

- [ ] **Step 3: Insert messages into new schema**

For private messages:

- `message_scene = 'private'`
- `peer_id = source_id`
- `message_seq = generated string id`
- `sender_user_id = record.sender_user_id`
- `receiver_user_id = source_id` when `record.sender_user_id == record.owner_user_id`
- `receiver_user_id = record.owner_user_id` when the sender is the peer and the owner is receiving/importing the message
- `group_id = NULL`

For group messages:

- `message_scene = 'group'`
- `peer_id = source_id`
- `message_seq = generated string id`
- `sender_user_id = record.sender_user_id`
- `receiver_user_id = NULL`
- `group_id = source_id`

Use one transaction:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(message_id AS INTEGER)), 0) + 1 AS TEXT)
    FROM messages
)
INSERT INTO messages (
    message_id, message_scene, peer_id, message_seq, sender_user_id,
    receiver_user_id, group_id, content_json, quoted_message_id, created_at
) SELECT
    value,
    ?1,
    ?2,
    value,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8
FROM next_id
RETURNING message_id AS id,
          sender_user_id,
          message_scene AS source_type,
          peer_id AS source_id,
          receiver_user_id,
          group_id,
          content_json,
          quoted_message_id,
          is_recalled,
          recalled_by_user_id,
          created_at
```

Use deterministic `conversation_id` values so conversation IDs do not depend on numeric allocation:

```rust
format!("{owner_user_id}:{scene}:{peer_or_group_id}")
```

Bind `owner_user_id` from `record.owner_user_id`; do not derive it from `sender_user_id` inside the repo.

Use separate upserts that match the partial unique indexes in the DDL.

Private/temp conversation:

```sql
INSERT INTO conversations (
    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
    last_message_id, unread_count, updated_at
) VALUES (?1, ?2, ?3, ?4, NULL, ?5, 0, ?6)
ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
WHERE conversation_scene IN ('private', 'temp')
DO UPDATE SET
    last_message_id = excluded.last_message_id,
    updated_at = excluded.updated_at
```

Group conversation:

```sql
INSERT INTO conversations (
    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
    last_message_id, unread_count, updated_at
) VALUES (?1, ?2, 'group', NULL, ?3, ?4, 0, ?5)
ON CONFLICT(owner_user_id, conversation_scene, group_id)
WHERE conversation_scene = 'group'
DO UPDATE SET
    last_message_id = excluded.last_message_id,
    updated_at = excluded.updated_at
```

For private conversation uniqueness, set `peer_user_id = source_id` and `group_id = NULL`. For group conversation, set `peer_user_id = NULL` and `group_id = source_id`. Bind all ID parameters as `String`.

- [ ] **Step 4: Adapt recall and read queries**

Recall:

```sql
UPDATE messages
SET is_recalled = 1,
    recalled_by_user_id = ?2,
    recalled_at = unixepoch() * 1000
WHERE message_id = ?1 AND is_recalled = 0
RETURNING message_id AS id,
          sender_user_id,
          message_scene AS source_type,
          peer_id AS source_id,
          receiver_user_id,
          group_id,
          content_json,
          quoted_message_id,
          is_recalled,
          recalled_by_user_id,
          created_at
```

Owner-scoped single message lookup for quote validation, recall, and reactions:

```sql
SELECT message_id AS id,
       sender_user_id,
       message_scene AS source_type,
       CASE
           WHEN message_scene IN ('private', 'temp') AND sender_user_id = ?2 THEN receiver_user_id
           WHEN message_scene IN ('private', 'temp') THEN sender_user_id
           ELSE group_id
       END AS source_id,
       receiver_user_id,
       group_id,
       content_json,
       quoted_message_id,
       is_recalled,
       recalled_by_user_id,
       created_at
FROM messages
WHERE message_id = ?1
  AND (
      message_scene = 'group'
      OR sender_user_id = ?2
      OR receiver_user_id = ?2
  )
```

Use this owner-scoped lookup in `MessageService` quote validation and recall, and in `InteractionService::react_to_message`. Keep a raw `get_message_by_id` only for internal group checks such as group essence validation; raw group lookup should return `group_id AS source_id`.

Private history:

```sql
SELECT message_id AS id,
       sender_user_id,
       message_scene AS source_type,
       CASE
           WHEN sender_user_id = ?1 THEN receiver_user_id
           ELSE sender_user_id
       END AS source_id,
       receiver_user_id,
       group_id,
       content_json,
       quoted_message_id,
       is_recalled,
       recalled_by_user_id,
       created_at
FROM messages
WHERE message_scene = 'private'
  AND (
    (sender_user_id = ?1 AND receiver_user_id = ?2)
    OR (sender_user_id = ?2 AND receiver_user_id = ?1)
  )
ORDER BY created_at DESC
LIMIT ?3
```

Group history:

```sql
SELECT message_id AS id,
       sender_user_id,
       message_scene AS source_type,
       group_id AS source_id,
       receiver_user_id,
       group_id,
       content_json,
       quoted_message_id,
       is_recalled,
       recalled_by_user_id,
       created_at
FROM messages
WHERE message_scene = 'group' AND group_id = ?1
ORDER BY created_at DESC
LIMIT ?2
```

Use `group_id` for group history lookup because group rows are retained with `group_status` and `messages.group_id` remains a valid FK. UI display should render deleted or unavailable senders/groups from their lifecycle status labels.

- [ ] **Step 5: Run focused check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: message repo compiles; remaining errors are in interaction repo.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/persistence/repo/message.rs
git commit -m "refactor(db): adapt message repo to target schema"
```

---

### Task 9: Adapt InteractionRepo To message_reactions And pokes

**Files:**
- Modify: `src-tauri/src/persistence/repo/interaction.rs`
- Modify: `src-tauri/src/services/interaction.rs`

- [ ] **Step 1: Remove old init_schema**

Delete `InteractionRepo::init_schema`.

- [ ] **Step 2: Update reaction row mapping**

The new `message_reactions` table does not store source fields. Select source fields by joining `messages`:

Change reaction record shapes so only the read row carries reconstructed source fields:

```rust
struct MessageReactionRow {
    id: String,
    message_id: String,
    source_type: String,
    source_id: String,
    operator_user_id: String,
    face_id: String,
    is_add: bool,
    created_at: u64,
}

pub struct NewMessageReactionRecord {
    pub message_id: String,
    pub operator_user_id: String,
    pub face_id: String,
    pub is_add: bool,
    pub created_at: u64,
}
```

Update `InteractionService::react_to_message` to build `NewMessageReactionRecord` without `source_type` or `source_id`; the returned `MessageReactionEntity` gets its source from the post-insert join below.

```sql
SELECT
    r.reaction_id AS id,
    r.message_id,
    m.message_scene AS source_type,
    CASE
        WHEN m.message_scene IN ('private', 'temp') AND m.sender_user_id = r.operator_user_id THEN m.receiver_user_id
        WHEN m.message_scene IN ('private', 'temp') THEN m.sender_user_id
        ELSE m.group_id
    END AS source_id,
    r.operator_user_id,
    r.face_id,
    r.is_add,
    r.created_at
FROM message_reactions r
INNER JOIN messages m ON m.message_id = r.message_id
```

Insert:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(reaction_id AS INTEGER)), 0) + 1 AS TEXT)
    FROM message_reactions
)
INSERT INTO message_reactions (
    reaction_id, message_id, operator_user_id, face_id, is_add, created_at
) SELECT value, ?1, ?2, ?3, ?4, ?5
FROM next_id
RETURNING reaction_id AS id
```

Use a follow-up select with the join to return `MessageReactionEntity`. Bind `message_id` and `operator_user_id` as `String`.

- [ ] **Step 3: Adapt poke SQL**

Change poke row and write records to string IDs:

```rust
struct PokeRow {
    id: String,
    source_type: String,
    source_id: String,
    sender_user_id: String,
    target_user_id: String,
    created_at: u64,
}

pub struct NewPokeRecord {
    pub source_type: String,
    pub source_id: String,
    pub sender_user_id: String,
    pub target_user_id: String,
    pub created_at: u64,
}
```

Insert into new columns:

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(poke_id AS INTEGER)), 0) + 1 AS TEXT)
    FROM pokes
)
INSERT INTO pokes (
    poke_id, message_scene, peer_id, sender_user_id, target_user_id, created_at
) SELECT value, ?1, ?2, ?3, ?4, ?5
FROM next_id
RETURNING poke_id AS id,
          message_scene AS source_type,
          peer_id AS source_id,
          sender_user_id,
          target_user_id,
          created_at
```

List private pokes by sender/target pair; for private poke history, return `source_id` as the other participant relative to the requested owner, using the same `CASE` shape as private message history. List group pokes by `message_scene = 'group' AND peer_id = ?` and return `peer_id AS source_id`. Bind all poke ID parameters as `String`.

- [ ] **Step 4: Run focused check**

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: backend compiles.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/persistence/repo/interaction.rs
git commit -m "refactor(db): adapt interaction repo to target schema"
```

---

### Task 10: Add Repository Smoke Tests

**Files:**
- Modify: `src-tauri/src/persistence/repo/mod.rs`
- Create: `src-tauri/src/persistence/repo/tests.rs`

- [ ] **Step 1: Register test module**

In `repo/mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

- [ ] **Step 2: Add smoke tests**

Create tests that:

1. Run `migrator::run_migrations(&pool)`.
2. Upsert two users with string IDs such as `"10001"` and `"10002"`.
3. Create a group with a string ID such as `"20001"` and add both users as members.
4. Insert one private message and one group message.
5. List private and group history.
6. Create and accept a friend request.
7. Create and handle a group request.
8. Insert a reaction and a poke.
9. Call the owner account deletion path, verify the owner row becomes `account_status = 'deleted'`, the owned group row becomes `group_status = 'dissolved'`, and both rows remain while the previously inserted group message still references `messages.group_id` and is returned by group history.

Use `#[sqlx::test]` and the existing repo public methods. This test should exercise the same service-facing API that the app uses.

- [ ] **Step 3: Run repo tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml persistence::repo::tests
```

Expected: all repo smoke tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/persistence/repo/mod.rs src-tauri/src/persistence/repo/tests.rs
git commit -m "test(db): cover repos on migrated schema"
```

---

### Task 11: Full Verification

**Files:** no source edits expected.

- [ ] **Step 1: Format**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
```

Expected: formatting completes.

- [ ] **Step 2: Rust tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

- [ ] **Step 3: Frontend/backend build checks**

```powershell
bun run build
```

Expected: TypeScript and Vite build pass.

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: Rust check passes without schema-related errors.

- [ ] **Step 4: Manual clean database startup**

Use a temporary profile or remove the dev DB for the app identifier in `src-tauri/tauri.conf.json`:

```powershell
$dbDir = "$env:APPDATA\dev.xef2.unibot"
if (Test-Path "$dbDir\unibot.db") { Remove-Item "$dbDir\unibot.db" }
if (Test-Path "$dbDir\unibot.db-wal") { Remove-Item "$dbDir\unibot.db-wal" }
if (Test-Path "$dbDir\unibot.db-shm") { Remove-Item "$dbDir\unibot.db-shm" }
```

Then start the app:

```powershell
bun tauri dev
```

Expected:

- App starts without DB migration error.
- A new `unibot.db` is created.
- `app_settings.schema.version` is `0001`.
- Current user/group/message flows do not fail with `no such table` or missing column errors.

- [ ] **Step 5: Commit verification-only fixes if needed**

If verification requires small fixes:

```bash
git add src-tauri src
git commit -m "fix(db): resolve migrated schema integration issues"
```

---

## Final Checklist

- [ ] `src-tauri/src/persistence/migrations/0001_initial_schema.sql` is the only startup schema source.
- [ ] `docs/specs/database/ddl/0001_initial_schema.sql` is documentation/specification only and is not read by app startup, tests, or build scripts.
- [ ] Source and migration DDL keep `messages.sender_user_id`, private `messages.receiver_user_id`, and group `messages.group_id` non-null in their valid message shapes.
- [ ] Source and migration DDL make `chat_groups.group_owner_user_id` `NOT NULL` with `ON DELETE RESTRICT`.
- [ ] Source and migration DDL retain `im_accounts` and `chat_groups` rows through `account_status`, `group_status`, `deleted_at`, `dissolved_at`, and `unavailable_at` instead of physical deletes.
- [ ] Source and migration DDL keep historical group messages linked through non-null `messages.group_id`.
- [ ] Source and migration DDL keep `group_essence_messages` as snapshot records with stored `group_id`, nullable `message_id`, stored `sender_user_id`, `operator_user_id`, and `created_at`.
- [ ] Source and migration DDL enforce same-group parentage for group files/folders through `(parent_folder_id, group_id)` -> `(folder_id, group_id)`.
- [ ] Source and migration DDL do not define `trg_unread_inc`; unread counts are updated only by owner-scoped repo/service logic.
- [ ] Account deletion marks owned groups as `group_status = 'dissolved'`; disabled/unavailable accounts do not dissolve owned groups.
- [ ] Old `Repo::init_schema` methods are removed from startup and deleted where they only contained old DDL.
- [ ] `db_pool.rs` uses `migrator::run_migrations`.
- [ ] SQLite options include `foreign_keys(true)`, WAL, `synchronous(NORMAL)`, and `busy_timeout(5000ms)`.
- [ ] SQL splitter preserves trigger bodies.
- [ ] Backend persistent ID fields and command parameters use `String`/`Option<String>` instead of `u64`/`i64`.
- [ ] `UserProfile` and `GroupProfile` expose lifecycle status fields, and repo row mapping converts `account_status`/`group_status` through codec helpers.
- [ ] `friend_requests` and `FriendRequestEntity` do not store or expose `operator_user_id`; handled friend requests derive the handler from `target_user_id` and service uses current `user_id` only for immediate events.
- [ ] `GroupRole`, `RequestState`, and `GroupRequestType` no longer use integer `sqlx::Type` mapping; repo SQL converts them through text codec helpers.
- [ ] `src-tauri/src/models/entities.rs` changes `MessageSource` IDs to `String`/`DbId`, `to_db_parts()` returns `(&'static str, &str)`, and `TryFrom<(&str, String)>` reconstructs sources without numeric parsing.
- [ ] Message and interaction services use owner-scoped message lookup when reconstructing private `MessageSource`; raw message lookup is limited to internal group checks.
- [ ] Frontend TypeScript persistent ID fields and invoke payloads use `string` instead of `number`.
- [ ] Repo SQL selects target `TEXT` IDs directly without numeric casts.
- [ ] User repo uses `im_accounts`.
- [ ] Group repo uses `chat_groups` and stores whole mute on `chat_groups`.
- [ ] Message repo uses `messages.message_scene/peer_id/message_id` and updates `conversations` with explicit partial-index `ON CONFLICT` targets.
- [ ] Interaction repo no longer assumes `message_reactions.source_type/source_id`.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes.
- [ ] `bun run build` passes.
- [ ] Clean database startup creates 26 tables and `schema.version = '0001'`.

---

## Risks And Follow-Up Work

- Public Rust and TypeScript IDs become strings in this phase. Any frontend code that currently parses route params or form values as numbers must keep them as strings.
- `group_requests` uses `(group_id, notification_seq)` as PK, while the current API exposes a single `request_id`. This plan treats `notification_seq` as globally generated string text for compatibility.
- Bot/debug/protocol tables are created but not yet used by services. They should be implemented in the next P0 vertical slice.
