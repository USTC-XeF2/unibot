# 阶段 1 基础设施修复 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 CTE MAX+1 ID 生成竞态，建立 migration 兼容性测试基线，并在 Settings 页面提供数据库状态可见性。

**Architecture:** Rust 端使用 UUID v7 生成所有持久化 ID，彻底消除 SQLite 层的并发竞态；migration 测试确保 schema 演进安全；Settings 面板通过 Tauri 命令暴露数据库元数据。

**Tech Stack:** Rust edition 2024, sqlx 0.8, SQLite, Tauri 2, uuid v7, React/TypeScript

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/Cargo.toml` | 修改 | 添加 `uuid` crate 依赖 |
| `src-tauri/src/utils.rs` | 修改 | 新增 `new_db_id()` 工具函数 |
| `src-tauri/src/persistence/repo/message.rs` | 修改 | `insert_message` 去掉 CTE，使用 Rust 生成 ID |
| `src-tauri/src/persistence/repo/interaction.rs` | 修改 | `insert_message_reaction`、`insert_poke` 去掉 CTE |
| `src-tauri/src/persistence/repo/user/friends.rs` | 修改 | `create_friend_request` 去掉 CTE |
| `src-tauri/src/persistence/repo/group/requests.rs` | 修改 | `create_group_request` 去掉 CTE |
| `src-tauri/src/persistence/repo/group/content.rs` | 修改 | `create_group_essence_message` 去掉 CTE |
| `src-tauri/src/persistence/repo/group/events.rs` | 修改 | `insert_group_event` 去掉 CTE |
| `src-tauri/src/persistence/migrator.rs` | 修改 | 添加 migration 兼容性测试 |
| `src-tauri/src/commands/main.rs` | 修改 | 添加 `get_db_status` 命令 |
| `src-tauri/src/lib.rs` | 修改 | 注册 `get_db_status` 命令 |
| `src-tauri/src/services/mod.rs` | 修改 | 暴露需要的 repo 方法或新建查询逻辑 |
| `src/types/db.ts` | 创建 | `DbStatus` TypeScript 类型 |
| `src/lib/query/db.ts` | 创建 | `useDbStatusQuery` hook |
| `src/views/main/settings.tsx` | 修改 | 数据库状态卡片 + 备份按钮 |

---

### Task 1: Add UUID Dependency and ID Generator

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/utils.rs`

- [ ] **Step 1: Add `uuid` crate to Cargo.toml**

  In `src-tauri/Cargo.toml`, add to `[dependencies]`:

  ```toml
  uuid = { version = "1", features = ["v7"] }
  ```

  Run:
  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully (downloads uuid crate).

- [ ] **Step 2: Add `new_db_id()` helper to utils.rs**

  In `src-tauri/src/utils.rs`, append after `now_ts()`:

  ```rust
  pub fn new_db_id() -> String {
      uuid::Uuid::now_v7().to_string()
  }
  ```

  Run:
  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully.

- [ ] **Step 3: Commit**

  ```bash
  git add src-tauri/Cargo.toml src-tauri/src/utils.rs
  git commit -m "feat(db): add uuid v7 id generator"
  ```

---

### Task 2: Replace CTE in MessageRepo

**Files:**
- Modify: `src-tauri/src/persistence/repo/message.rs`

- [ ] **Step 1: Read current `insert_message` implementation**

  The current SQL uses:
  ```rust
  WITH next_id(value) AS (
      SELECT CAST(COALESCE(MAX(CAST(message_id AS INTEGER)), 0) + 1 AS TEXT)
      FROM messages
  )
  INSERT INTO messages (
      message_id, message_scene, peer_id, message_seq, sender_user_id,
      receiver_user_id, group_id, content_json, quoted_message_id, created_at
  ) SELECT
      value, ?1, ?2, value, ?3, ?4, ?5, ?6, ?7, ?8
  FROM next_id
  RETURNING message_id AS id, sender_user_id, message_scene AS source_type,
            peer_id AS source_id, receiver_user_id, group_id, content_json,
            quoted_message_id, is_recalled, recalled_by_user_id, created_at
  ```

  Note: `message_seq` is set to the same generated value as `message_id`.

- [ ] **Step 2: Replace with plain INSERT using `new_db_id()`**

  In `src-tauri/src/persistence/repo/message.rs`, replace the `let row = sqlx::query_as...` block inside `insert_message` with:

  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, MessageRecord>(
      r#"
      INSERT INTO messages (
          message_id, message_scene, peer_id, message_seq, sender_user_id,
          receiver_user_id, group_id, content_json, quoted_message_id, created_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
      RETURNING message_id AS id, sender_user_id, message_scene AS source_type,
                peer_id AS source_id, receiver_user_id, group_id, content_json,
                quoted_message_id, is_recalled, recalled_by_user_id, created_at
      "#,
  )
  .bind(&id)
  .bind(&record.source_type)
  .bind(&record.source_id)
  .bind(&id)  // message_seq = message_id
  .bind(&record.sender_user_id)
  .bind(receiver_user_id)
  .bind(group_id)
  .bind(&record.content_json)
  .bind(&record.quoted_message_id)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

  Keep the `receiver_user_id` and `group_id` computation logic above the query unchanged.

- [ ] **Step 3: Run repo tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml persistence::repo::tests::smoke_crud_messages
  ```

  Expected: passes (message insert and history still work).

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/message.rs
  git commit -m "refactor(db): use uuid v7 for message ids"
  ```

---

### Task 3: Replace CTE in InteractionRepo

**Files:**
- Modify: `src-tauri/src/persistence/repo/interaction.rs`

- [ ] **Step 1: Replace `insert_message_reaction` CTE**

  Current SQL:
  ```rust
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

  Replace with:
  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, MessageReactionIdRow>(
      r#"
      INSERT INTO message_reactions (
          reaction_id, message_id, operator_user_id, face_id, is_add, created_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
      RETURNING reaction_id AS id
      "#,
  )
  .bind(&id)
  .bind(&record.message_id)
  .bind(&record.operator_user_id)
  .bind(&record.face_id)
  .bind(record.is_add)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

- [ ] **Step 2: Replace `insert_poke` CTE**

  Current SQL:
  ```rust
  WITH next_id(value) AS (
      SELECT CAST(COALESCE(MAX(CAST(poke_id AS INTEGER)), 0) + 1 AS TEXT)
      FROM pokes
  )
  INSERT INTO pokes (
      poke_id, message_scene, peer_id, sender_user_id, target_user_id, created_at
  ) SELECT value, ?1, ?2, ?3, ?4, ?5
  FROM next_id
  RETURNING poke_id AS id, message_scene AS source_type, peer_id AS source_id,
            sender_user_id, target_user_id, created_at
  ```

  Replace with:
  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, PokeRow>(
      r#"
      INSERT INTO pokes (
          poke_id, message_scene, peer_id, sender_user_id, target_user_id, created_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
      RETURNING poke_id AS id, message_scene AS source_type, peer_id AS source_id,
                sender_user_id, target_user_id, created_at
      "#,
  )
  .bind(&id)
  .bind(&record.source_type)
  .bind(&record.source_id)
  .bind(&record.sender_user_id)
  .bind(&record.target_user_id)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

- [ ] **Step 3: Run repo tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml persistence::repo::tests::smoke_crud_interactions
  ```

  Expected: passes.

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/interaction.rs
  git commit -m "refactor(db): use uuid v7 for reaction and poke ids"
  ```

---

### Task 4: Replace CTE in User and Group Repos

**Files:**
- Modify: `src-tauri/src/persistence/repo/user/friends.rs`
- Modify: `src-tauri/src/persistence/repo/group/requests.rs`
- Modify: `src-tauri/src/persistence/repo/group/content.rs`
- Modify: `src-tauri/src/persistence/repo/group/events.rs`

- [ ] **Step 1: Replace `create_friend_request` in `user/friends.rs`**

  Replace the CTE block with:
  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, FriendRequestRow>(
      r#"
      INSERT INTO friend_requests (
          request_id, initiator_user_id, target_user_id, comment, state, created_at
      ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5)
      RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
      "#,
  )
  .bind(&id)
  .bind(&record.initiator_user_id)
  .bind(&record.target_user_id)
  .bind(&record.comment)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

- [ ] **Step 2: Replace `create_group_request` in `group/requests.rs`**

  Replace the CTE block with:
  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, GroupRequestRow>(
      r#"
      INSERT INTO group_requests (
          group_id, notification_seq, notification_type, initiator_user_id,
          target_user_id, comment, state, created_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)
      RETURNING notification_seq AS id, group_id, notification_type AS request_type,
                initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
      "#,
  )
  .bind(&record.group_id)
  .bind(&id)
  .bind(codecs::group_request_type_to_db(record.request_type))
  .bind(&record.initiator_user_id)
  .bind(record.target_user_id.as_deref())
  .bind(&record.comment)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

- [ ] **Step 3: Replace `create_group_essence_message` in `group/content.rs`**

  Replace the `WITH next_id` inside `if is_set` with:
  ```rust
  let id = crate::utils::new_db_id();
  let row = sqlx::query_as::<_, GroupEssenceRow>(
      r#"
      INSERT INTO group_essence_messages (
          essence_id, group_id, message_id, sender_user_id, operator_user_id, created_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
      ON CONFLICT(group_id, message_id) DO UPDATE SET
          sender_user_id = excluded.sender_user_id,
          operator_user_id = excluded.operator_user_id,
          created_at = excluded.created_at
      RETURNING essence_id AS id, group_id, message_id, sender_user_id, operator_user_id, 1 AS is_set, created_at
      "#,
  )
  .bind(&id)
  .bind(group_id)
  .bind(message_id)
  .bind(sender_user_id)
  .bind(operator_user_id)
  .bind(created_at as i64)
  .fetch_one(&self.pool)
  .await?;
  ```

- [ ] **Step 4: Replace `insert_group_event` in `group/events.rs`**

  Replace the CTE block with:
  ```rust
  let id = crate::utils::new_db_id();
  sqlx::query_as::<_, GroupEventRecord>(
      r#"
      INSERT INTO group_events (event_id, group_id, event_type, payload_json, created_at)
      VALUES (?1, ?2, 'generic', ?3, ?4)
      RETURNING event_id AS id, group_id, payload_json AS payload, created_at
      "#,
  )
  .bind(&id)
  .bind(&record.group_id)
  .bind(&record.payload)
  .bind(record.created_at as i64)
  .fetch_one(&self.pool)
  .await
  ```

- [ ] **Step 5: Run all repo tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

  Expected: all tests pass (smoke_crud_users, smoke_crud_friends, smoke_crud_groups, smoke_crud_messages, smoke_crud_interactions, smoke_group_requests, smoke_account_deletion_retains_rows, smoke_group_events).

- [ ] **Step 6: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/user/friends.rs \
          src-tauri/src/persistence/repo/group/requests.rs \
          src-tauri/src/persistence/repo/group/content.rs \
          src-tauri/src/persistence/repo/group/events.rs
  git commit -m "refactor(db): use uuid v7 for friend/group request, essence, and event ids"
  ```

---

### Task 5: Add Migration Compatibility Test

**Files:**
- Modify: `src-tauri/src/persistence/migrator.rs`

- [ ] **Step 1: Add test for clean-database migration**

  At the end of `src-tauri/src/persistence/migrator.rs` (inside the existing `#[cfg(test)]` mod), add:

  ```rust
  #[sqlx::test]
  async fn migrates_from_blank_to_latest(pool: SqlitePool) -> Result<(), sqlx::Error> {
      // Simulate a pre-migration database by ensuring app_settings does not exist
      let table_exists: i64 = sqlx::query_scalar(
          "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_settings'",
      )
      .fetch_one(&pool)
      .await?;
      assert_eq!(table_exists, 0);

      run_migrations(&pool).await.map_err(sqlx::Error::Protocol)?;

      let version: String = sqlx::query_scalar(
          "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
      )
      .fetch_one(&pool)
      .await?;
      assert_eq!(version, "0001");

      let table_count: i64 =
          sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'")
              .fetch_one(&pool)
              .await?;
      assert_eq!(table_count, 26);

      Ok(())
  }
  ```

- [ ] **Step 2: Run the new test**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml migrates_from_blank_to_latest
  ```

  Expected: passes.

- [ ] **Step 3: Commit**

  ```bash
  git add src-tauri/src/persistence/migrator.rs
  git commit -m "test(db): add migration compatibility test for clean db"
  ```

---

### Task 6: Add Backend DB Status Command

**Files:**
- Modify: `src-tauri/src/commands/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/types/db.ts`

- [ ] **Step 1: Add `DbStatus` Rust struct**

  In `src-tauri/src/commands/main.rs`, add after existing imports:

  ```rust
  use sqlx::SqlitePool;
  use tauri::Manager;

  #[derive(serde::Serialize)]
  pub struct DbStatus {
      schema_version: String,
      table_count: i64,
      db_size_bytes: u64,
      integrity_check: String,
      foreign_key_check: Vec<String>,
  }
  ```

  Add the command function at the end of `main.rs`:

  ```rust
  #[tauri::command]
  pub async fn get_db_status(app: tauri::AppHandle) -> Result<DbStatus, String> {
      let pool = app
          .state::<SqlitePool>()
          .inner()
          .clone();

      let schema_version: String = sqlx::query_scalar(
          "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
      )
      .fetch_one(&pool)
      .await
      .map_err(|err| format!("failed to read schema version: {err}"))?;

      let table_count: i64 = sqlx::query_scalar(
          "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
      )
      .fetch_one(&pool)
      .await
      .map_err(|err| format!("failed to count tables: {err}"))?;

      let db_path = app
          .path()
          .app_data_dir()
          .map_err(|err| format!("failed to get app data dir: {err}"))?
          .join("unibot.db");
      let db_size_bytes = std::fs::metadata(&db_path)
          .map(|m| m.len())
          .unwrap_or(0);

      let integrity_check: String = sqlx::query_scalar("PRAGMA integrity_check")
          .fetch_one(&pool)
          .await
          .map_err(|err| format!("integrity check failed: {err}"))?;

      let fk_issues: Vec<String> = sqlx::query_scalar("PRAGMA foreign_key_check")
          .fetch_all(&pool)
          .await
          .map_err(|err| format!("foreign key check failed: {err}"))?;

      Ok(DbStatus {
          schema_version,
          table_count,
          db_size_bytes,
          integrity_check,
          foreign_key_check: fk_issues,
      })
  }
  ```

  **Note:** `app.state::<SqlitePool>()` requires `SqlitePool` to be managed as Tauri state. Currently `init_sqlite_pool` creates the pool but it's only stored in repos. We need to manage it in the app.

  Wait — currently the pool is created in `lib.rs` setup and passed to repos directly. It's not managed as app state. We need to either:
  1. Manage the pool as app state
  2. Or access db path from app and run PRAGMA via a temporary connection

  Option 1 is cleaner. Modify `lib.rs` setup to also `app.manage(pool.clone())`.

- [ ] **Step 2: Manage pool as Tauri state in lib.rs**

  In `src-tauri/src/lib.rs`, after `let pool = tauri::async_runtime::block_on(init_sqlite_pool(...))?;`, add:

  ```rust
  app.manage(pool.clone());
  ```

  Register the new command in `invoke_handler!`:

  ```rust
  .invoke_handler(tauri::generate_handler![
      main::register_user,
      main::list_users,
      main::list_groups,
      main::delete_user,
      main::open_user_chat_window,
      main::get_db_status,  // <-- add this
      // ... rest unchanged
  ])
  ```

- [ ] **Step 3: Add TypeScript type**

  Create `src/types/db.ts`:

  ```typescript
  export interface DbStatus {
    schema_version: string;
    table_count: number;
    db_size_bytes: number;
    integrity_check: string;
    foreign_key_check: string[];
  }
  ```

- [ ] **Step 4: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully.

- [ ] **Step 5: Commit**

  ```bash
  git add src-tauri/src/commands/main.rs src-tauri/src/lib.rs src/types/db.ts
  git commit -m "feat(db): add get_db_status backend command"
  ```

---

### Task 7: Build Settings Database Status Panel

**Files:**
- Create: `src/lib/query/db.ts`
- Modify: `src/views/main/settings.tsx`

- [ ] **Step 1: Add `useDbStatusQuery` hook**

  Create `src/lib/query/db.ts`:

  ```typescript
  import { useQuery } from "@tanstack/react-query";
  import { invoke } from "@tauri-apps/api/core";
  import type { DbStatus } from "@/types/db";

  export function useDbStatusQuery() {
    return useQuery({
      queryKey: ["db", "status"],
      queryFn: () => invoke<DbStatus>("get_db_status"),
      retry: false,
    });
  }
  ```

  Export from `src/lib/query/index.ts` by adding:
  ```typescript
  export * from "@/lib/query/db";
  ```

- [ ] **Step 2: Build Settings UI**

  Replace `src/views/main/settings.tsx` with:

  ```tsx
  import { useState } from "react";
  import { Database, Shield, Download } from "lucide-react";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import { useDbStatusQuery } from "@/lib/query";
  import { queryClient } from "@/lib/query-client";

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
  }

  function SettingsView() {
    const dbStatus = useDbStatusQuery();
    const [isExporting, setIsExporting] = useState(false);
    const data = dbStatus.data;

    const handleExport = async () => {
      setIsExporting(true);
      try {
        await invoke("export_db_backup");
      } catch (err) {
        alert(`导出失败: ${err}`);
      } finally {
        setIsExporting(false);
      }
    };

    return (
      <div className="space-y-4">
        <h1 className="font-semibold text-xl">设置</h1>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Database className="size-4" />
              数据库状态
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {dbStatus.isPending ? (
              <p className="text-muted-foreground text-sm">读取中...</p>
            ) : dbStatus.isError ? (
              <p className="text-destructive text-sm">
                读取失败: {String(dbStatus.error)}
              </p>
            ) : data ? (
              <>
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div>
                    <span className="text-muted-foreground">Schema 版本</span>
                    <p className="font-medium">{data.schema_version}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">表数量</span>
                    <p className="font-medium">{data.table_count}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">数据库大小</span>
                    <p className="font-medium">{formatBytes(data.db_size_bytes)}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Shield className="size-4" />
                    <span
                      className={
                        data.integrity_check === "ok"
                          ? "text-green-600"
                          : "text-destructive"
                      }
                    >
                      {data.integrity_check === "ok"
                        ? "完整性正常"
                        : `完整性异常: ${data.integrity_check}`}
                    </span>
                  </div>
                </div>

                {data.foreign_key_check.length > 0 && (
                  <div className="rounded border border-destructive/30 bg-destructive/10 p-2 text-destructive text-xs">
                    外键约束异常: {data.foreign_key_check.join(", ")}
                  </div>
                )}

                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    queryClient.invalidateQueries({ queryKey: ["db", "status"] })
                  }
                >
                  刷新状态
                </Button>
              </>
            ) : null}
          </CardContent>
        </Card>
      </div>
    );
  }

  export default SettingsView;
  ```

  **Note:** The `export_db_backup` command referenced above does not exist yet. Remove the export button from this first pass, or add a simple alert("待实现"). For minimal viable implementation, remove the export button and `isExporting` state entirely, keep only the status display.

  Revised minimal SettingsView without export:

  ```tsx
  import { Database, Shield } from "lucide-react";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import { useDbStatusQuery } from "@/lib/query";
  import { queryClient } from "@/lib/query-client";

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
  }

  function SettingsView() {
    const dbStatus = useDbStatusQuery();
    const data = dbStatus.data;

    return (
      <div className="space-y-4">
        <h1 className="font-semibold text-xl">设置</h1>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Database className="size-4" />
              数据库状态
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {dbStatus.isPending ? (
              <p className="text-muted-foreground text-sm">读取中...</p>
            ) : dbStatus.isError ? (
              <p className="text-destructive text-sm">
                读取失败: {String(dbStatus.error)}
              </p>
            ) : data ? (
              <>
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div>
                    <span className="text-muted-foreground">Schema 版本</span>
                    <p className="font-medium">{data.schema_version}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">表数量</span>
                    <p className="font-medium">{data.table_count}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">数据库大小</span>
                    <p className="font-medium">{formatBytes(data.db_size_bytes)}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Shield className="size-4" />
                    <span
                      className={
                        data.integrity_check === "ok"
                          ? "text-green-600"
                          : "text-destructive"
                      }
                    >
                      {data.integrity_check === "ok"
                        ? "完整性正常"
                        : `完整性异常: ${data.integrity_check}`}
                    </span>
                  </div>
                </div>

                {data.foreign_key_check.length > 0 && (
                  <div className="rounded border border-destructive/30 bg-destructive/10 p-2 text-destructive text-xs">
                    外键约束异常: {data.foreign_key_check.join(", ")}
                  </div>
                )}

                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    queryClient.invalidateQueries({ queryKey: ["db", "status"] })
                  }
                >
                  刷新状态
                </Button>
              </>
            ) : null}
          </CardContent>
        </Card>
      </div>
    );
  }

  export default SettingsView;
  ```

- [ ] **Step 3: Verify build**

  ```bash
  bun run build
  ```

  Expected: TypeScript compiles without errors.

- [ ] **Step 4: Commit**

  ```bash
  git add src/lib/query/db.ts src/lib/query/index.ts src/views/main/settings.tsx src/types/db.ts
  git commit -m "feat(settings): add database status panel"
  ```

---

### Task 8: Full Verification

**Files:** no source edits expected.

- [ ] **Step 1: Format Rust**

  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml
  ```

- [ ] **Step 2: Run all Rust tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

  Expected: all tests pass.

- [ ] **Step 3: Build frontend**

  ```bash
  bun run build
  ```

  Expected: Vite build passes.

- [ ] **Step 4: Manual integration test**

  ```bash
  bunx tauri dev
  ```

  Verify:
  1. App starts without errors.
  2. Navigate to Settings page — database status card shows schema version `0001`, table count `26`, integrity check "正常".
  3. Send a message in a chat — message appears without `UNIQUE constraint failed` error.
  4. Create a friend request — request is created successfully.

- [ ] **Step 5: Final commit if any fixes needed**

  ```bash
  git add .
  git commit -m "fix: resolve integration issues after uuid v7 migration"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ UUID v7 替换 CTE MAX+1 — covered by Tasks 1-4
- ✅ Migration 兼容性测试 — covered by Task 5
- ✅ Settings 数据库状态卡片 — covered by Tasks 6-7
- ⚠️ 数据库备份导出按钮 — **未包含**。理由：需要额外的 Tauri 命令和文件系统权限，作为 P2 延后到功能计划 B 或后续迭代。

**2. Placeholder scan:**
- ✅ 所有步骤包含完整代码或明确命令
- ✅ 无 "TBD"/"TODO" / "implement later"
- ✅ 无 "Add appropriate error handling" 等模糊描述

**3. Type consistency:**
- ✅ `new_db_id()` returns `String`, matching all `TEXT` ID columns
- ✅ `DbStatus` fields match between Rust struct and TypeScript interface
- ✅ Query key `["db", "status"]` consistent across hook and invalidation
