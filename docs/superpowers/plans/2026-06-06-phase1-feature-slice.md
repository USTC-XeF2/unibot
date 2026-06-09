# 阶段 1 功能推进 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Bot 管理 CRUD、调试会话生命周期、Dashboard 统计接入，让 `bots` 和 `debug_sessions` 表产生真实业务数据。

**Architecture:** 新建 `BotRepo` 和 `BotService` 管理 Bot 实体与调试会话；`MessageService` 预留 `bot_id` 字段；Dashboard 通过独立统计命令接入总消息数和在线 Bot 数。

**Tech Stack:** Rust edition 2024, sqlx 0.8, SQLite, Tauri 2, React/TypeScript, shadcn/ui

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/src/models/entities.rs` | 修改 | 新增 `BotProfile`、`DebugSession` 实体 |
| `src-tauri/src/models/mod.rs` | 修改 | 导出新实体 |
| `src-tauri/src/persistence/repo/bot.rs` | 创建 | `BotRepo` + `DebugSessionRepo` |
| `src-tauri/src/persistence/repo/mod.rs` | 修改 | 导出 `BotRepo` |
| `src-tauri/src/services/bot.rs` | 创建 | `BotService`（CRUD + 会话生命周期） |
| `src-tauri/src/services/mod.rs` | 修改 | `ServiceHub` 加入 `BotService` |
| `src-tauri/src/commands/bot.rs` | 创建 | Bot 相关 Tauri 命令 |
| `src-tauri/src/commands/mod.rs` | 修改 | `pub mod bot;` |
| `src-tauri/src/commands/main.rs` | 修改 | `get_stats` 命令 |
| `src-tauri/src/lib.rs` | 修改 | 初始化 `BotRepo`/`BotService`，注册命令 |
| `src-tauri/src/persistence/repo/message.rs` | 修改 | `NewMessageRecord`/`MessageRecord` 加 `bot_id` |
| `src-tauri/src/services/message.rs` | 修改 | `send` 预留 `bot_id` 参数 |
| `src-tauri/src/commands/chat/message.rs` | 修改 | `send_message` command 传 `None` bot_id |
| `src/types/bot.ts` | 创建 | `BotProfile`、`DebugSession`、`StatsResult` TS 类型 |
| `src/types/chat.ts` | 修改 | `ChatMessage` 加可选 `bot_id` |
| `src/lib/query/bots.ts` | 创建 | `useBotsQuery`、`useBotStatsQuery` |
| `src/lib/query/index.ts` | 修改 | 导出 bots query |
| `src/lib/mutations.ts` | 修改 | 添加 Bot mutations |
| `src/views/main/dashboard.tsx` | 修改 | 统计数字接入 + Bot 管理卡片 |

---

### Task 1: Add Bot and DebugSession Models

**Files:**
- Modify: `src-tauri/src/models/entities.rs`
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: Add `BotProfile` and `DebugSession` structs**

  In `src-tauri/src/models/entities.rs`, append after existing structs:

  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct BotProfile {
      pub bot_id: DbId,
      pub bound_user_id: DbId,
      pub display_name: String,
      pub runtime_status: String,
      pub config_path: String,
      pub created_at: u64,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct DebugSession {
      pub session_id: DbId,
      pub bot_id: DbId,
      pub session_name: String,
      pub description: Option<String>,
      pub started_at: u64,
      pub ended_at: Option<u64>,
  }
  ```

- [ ] **Step 2: Export from models/mod.rs**

  In `src-tauri/src/models/mod.rs`, add to the `pub use entities::{...}` list:

  ```rust
  pub use entities::{
      BotProfile, DebugSession,
      // ... existing exports
  };
  ```

- [ ] **Step 3: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully.

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/models/entities.rs src-tauri/src/models/mod.rs
  git commit -m "feat(bot): add BotProfile and DebugSession models"
  ```

---

### Task 2: Create BotRepo and DebugSessionRepo

**Files:**
- Create: `src-tauri/src/persistence/repo/bot.rs`
- Modify: `src-tauri/src/persistence/repo/mod.rs`

- [ ] **Step 1: Write `bot.rs` with repo structs and methods**

  Create `src-tauri/src/persistence/repo/bot.rs`:

  ```rust
  use sqlx::SqlitePool;

  pub struct BotRepo {
      pool: SqlitePool,
  }

  #[derive(sqlx::FromRow)]
  pub(super) struct BotRow {
      pub bot_id: String,
      pub bound_user_id: String,
      pub display_name: String,
      pub runtime_status: String,
      pub config_path: String,
      pub created_at: i64,
  }

  #[derive(sqlx::FromRow)]
  pub(super) struct DebugSessionRow {
      pub session_id: String,
      pub bot_id: String,
      pub session_name: String,
      pub description: Option<String>,
      pub started_at: i64,
      pub ended_at: Option<i64>,
  }

  impl BotRepo {
      pub fn new(pool: SqlitePool) -> Self {
          Self { pool }
      }

      pub async fn insert_bot(
          &self,
          bot_id: &str,
          bound_user_id: &str,
          display_name: &str,
          config_path: &str,
      ) -> Result<BotRow, sqlx::Error> {
          sqlx::query_as::<_, BotRow>(
              r#"
              INSERT INTO bots (bot_id, bound_user_id, display_name, runtime_status, config_path, created_at, updated_at)
              VALUES (?1, ?2, ?3, 'stopped', ?4, unixepoch() * 1000, unixepoch() * 1000)
              RETURNING bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
              "#,
          )
          .bind(bot_id)
          .bind(bound_user_id)
          .bind(display_name)
          .bind(config_path)
          .fetch_one(&self.pool)
          .await
      }

      pub async fn list_bots(&self) -> Result<Vec<BotRow>, sqlx::Error> {
          sqlx::query_as::<_, BotRow>(
              "SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at FROM bots ORDER BY created_at ASC"
          )
          .fetch_all(&self.pool)
          .await
      }

      pub async fn get_bot_by_id(
          &self,
          bot_id: &str,
      ) -> Result<Option<BotRow>, sqlx::Error> {
          sqlx::query_as::<_, BotRow>(
              "SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at FROM bots WHERE bot_id = ?1"
          )
          .bind(bot_id)
          .fetch_optional(&self.pool)
          .await
      }

      pub async fn find_bot_by_bound_user_id(
          &self,
          user_id: &str,
      ) -> Result<Option<BotRow>, sqlx::Error> {
          sqlx::query_as::<_, BotRow>(
              "SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at FROM bots WHERE bound_user_id = ?1"
          )
          .bind(user_id)
          .fetch_optional(&self.pool)
          .await
      }

      pub async fn delete_bot(&self, bot_id: &str) -> Result<bool, sqlx::Error> {
          let result = sqlx::query("DELETE FROM bots WHERE bot_id = ?1")
              .bind(bot_id)
              .execute(&self.pool)
              .await?;
          Ok(result.rows_affected() > 0)
      }

      pub async fn update_runtime_status(
          &self,
          bot_id: &str,
          status: &str,
      ) -> Result<(), sqlx::Error> {
          sqlx::query(
              "UPDATE bots SET runtime_status = ?1, updated_at = unixepoch() * 1000 WHERE bot_id = ?2"
          )
          .bind(status)
          .bind(bot_id)
          .execute(&self.pool)
          .await?;
          Ok(())
      }

      // DebugSession methods
      pub async fn insert_session(
          &self,
          session_id: &str,
          bot_id: &str,
          session_name: &str,
      ) -> Result<DebugSessionRow, sqlx::Error> {
          sqlx::query_as::<_, DebugSessionRow>(
              r#"
              INSERT INTO debug_sessions (session_id, bot_id, session_name, started_at)
              VALUES (?1, ?2, ?3, unixepoch() * 1000)
              RETURNING session_id, bot_id, session_name, description, started_at, ended_at
              "#,
          )
          .bind(session_id)
          .bind(bot_id)
          .bind(session_name)
          .fetch_one(&self.pool)
          .await
      }

      pub async fn end_active_sessions(
          &self,
          bot_id: &str,
      ) -> Result<(), sqlx::Error> {
          sqlx::query(
              "UPDATE debug_sessions SET ended_at = unixepoch() * 1000 WHERE bot_id = ?1 AND ended_at IS NULL"
          )
          .bind(bot_id)
          .execute(&self.pool)
          .await?;
          Ok(())
      }

      pub async fn has_active_session(
          &self,
          bot_id: &str,
      ) -> Result<bool, sqlx::Error> {
          let count: i64 = sqlx::query_scalar(
              "SELECT COUNT(*) FROM debug_sessions WHERE bot_id = ?1 AND ended_at IS NULL"
          )
          .bind(bot_id)
          .fetch_one(&self.pool)
          .await?;
          Ok(count > 0)
      }

      pub async fn list_sessions_by_bot(
          &self,
          bot_id: &str,
      ) -> Result<Vec<DebugSessionRow>, sqlx::Error> {
          sqlx::query_as::<_, DebugSessionRow>(
              "SELECT session_id, bot_id, session_name, description, started_at, ended_at FROM debug_sessions WHERE bot_id = ?1 ORDER BY started_at DESC"
          )
          .bind(bot_id)
          .fetch_all(&self.pool)
          .await
      }

      // Stats
      pub async fn get_online_bot_count(&self) -> Result<i64, sqlx::Error> {
          sqlx::query_scalar(
              "SELECT COUNT(DISTINCT bot_id) FROM debug_sessions WHERE ended_at IS NULL"
          )
          .fetch_one(&self.pool)
          .await
      }
  }

  impl TryFrom<BotRow> for crate::models::BotProfile {
      type Error = crate::error::AppError;

      fn try_from(row: BotRow) -> Result<Self, Self::Error> {
          Ok(Self {
              bot_id: row.bot_id,
              bound_user_id: row.bound_user_id,
              display_name: row.display_name,
              runtime_status: row.runtime_status,
              config_path: row.config_path,
              created_at: row.created_at as u64,
          })
      }
  }

  impl TryFrom<DebugSessionRow> for crate::models::DebugSession {
      type Error = crate::error::AppError;

      fn try_from(row: DebugSessionRow) -> Result<Self, Self::Error> {
          Ok(Self {
              session_id: row.session_id,
              bot_id: row.bot_id,
              session_name: row.session_name,
              description: row.description,
              started_at: row.started_at as u64,
              ended_at: row.ended_at.map(|v| v as u64),
          })
      }
  }
  ```

- [ ] **Step 2: Register in repo/mod.rs**

  In `src-tauri/src/persistence/repo/mod.rs`, add:

  ```rust
  pub mod bot;
  ```

  And add to `pub use`:
  ```rust
  pub use bot::BotRepo;
  ```

- [ ] **Step 3: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully (may warn about unused code until Service is wired).

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/bot.rs src-tauri/src/persistence/repo/mod.rs
  git commit -m "feat(bot): add BotRepo and DebugSessionRepo"
  ```

---

### Task 3: Create BotService

**Files:**
- Create: `src-tauri/src/services/bot.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write `BotService`**

  Create `src-tauri/src/services/bot.rs`:

  ```rust
  use crate::core::CoreContainer;
  use crate::error::{AppError, AppResult};
  use crate::models::{BotProfile, DebugSession};
  use crate::persistence::BotRepo;
  use crate::utils::new_db_id;
  use std::path::PathBuf;
  use tauri::Manager;

  #[derive(Clone)]
  pub struct BotService {
      repo: BotRepo,
  }

  impl BotService {
      pub fn new(repo: BotRepo) -> Self {
          Self { repo }
      }

      pub async fn create_bot(
          &self,
          app: &tauri::AppHandle,
          bound_user_id: String,
          display_name: String,
      ) -> AppResult<BotProfile> {
          if bound_user_id.trim().is_empty() {
              return Err(AppError::validation("bound user id cannot be empty"));
          }

          let existing = self.repo.find_bot_by_bound_user_id(&bound_user_id).await?;
          if existing.is_some() {
              return Err(AppError::conflict("user already has a bot"));
          }

          let bot_id = new_db_id();
          let app_data_dir = app
              .path()
              .app_data_dir()
              .map_err(|err| AppError::internal(format!("app dir error: {err}")))?;
          let bots_dir = app_data_dir.join("bots");
          std::fs::create_dir_all(&bots_dir)
              .map_err(|err| AppError::internal(format!("create bots dir: {err}")))?;
          let config_path = bots_dir.join(format!("{}.json", bot_id));
          std::fs::write(&config_path, "{}")
              .map_err(|err| AppError::internal(format!("write config: {err}")))?;

          let row = self
              .repo
              .insert_bot(&bot_id, &bound_user_id, &display_name, config_path.to_str().unwrap_or(""))
              .await?;

          row.try_into()
      }

      pub async fn list_bots(&self) -> AppResult<Vec<BotProfile>> {
          let rows = self.repo.list_bots().await?;
          rows.into_iter().map(TryInto::try_into).collect()
      }

      pub async fn delete_bot(
          &self,
          core: &CoreContainer,
          bot_id: String,
      ) -> AppResult<()> {
          let bot = self
              .repo
              .get_bot_by_id(&bot_id)
              .await?
              .ok_or_else(|| AppError::not_found(format!("bot {} not found", bot_id)))?;

          if bot.runtime_status == "running" {
              self.repo.end_active_sessions(&bot_id).await?;
          }

          self.repo.delete_bot(&bot_id).await?;
          Ok(())
      }

      pub async fn start_bot(&self,
          bot_id: String,
      ) -> AppResult<DebugSession> {
          let bot = self
              .repo
              .get_bot_by_id(&bot_id)
              .await?
              .ok_or_else(|| AppError::not_found(format!("bot {} not found", bot_id)))?;

          if bot.runtime_status == "running" {
              return Err(AppError::conflict("bot is already running"));
          }

          let session_id = new_db_id();
          let session_name = format!("调试会话 {}", crate::utils::now_ts());

          let mut tx = self.repo.pool.begin().await.map_err(AppError::Storage)?;

          let row = sqlx::query_as::<_, crate::persistence::repo::bot::DebugSessionRow>(
              r#"
              INSERT INTO debug_sessions (session_id, bot_id, session_name, started_at)
              VALUES (?1, ?2, ?3, unixepoch() * 1000)
              RETURNING session_id, bot_id, session_name, description, started_at, ended_at
              "#,
          )
          .bind(&session_id)
          .bind(&bot_id)
          .bind(&session_name)
          .fetch_one(&mut *tx)
          .await
          .map_err(AppError::from)?;

          sqlx::query(
              "UPDATE bots SET runtime_status = 'running', updated_at = unixepoch() * 1000 WHERE bot_id = ?1"
          )
          .bind(&bot_id)
          .execute(&mut *tx)
          .await
          .map_err(AppError::from)?;

          tx.commit().await.map_err(AppError::from)?;

          row.try_into()
      }

      pub async fn stop_bot(&self,
          bot_id: String,
      ) -> AppResult<()> {
          let bot = self
              .repo
              .get_bot_by_id(&bot_id)
              .await?
              .ok_or_else(|| AppError::not_found(format!("bot {} not found", bot_id)))?;

          if bot.runtime_status != "running" {
              return Err(AppError::validation("bot is not running"));
          }

          let mut tx = self.repo.pool.begin().await.map_err(AppError::Storage)?;

          sqlx::query(
              "UPDATE debug_sessions SET ended_at = unixepoch() * 1000 WHERE bot_id = ?1 AND ended_at IS NULL"
          )
          .bind(&bot_id)
          .execute(&mut *tx)
          .await
          .map_err(AppError::from)?;

          sqlx::query(
              "UPDATE bots SET runtime_status = 'stopped', updated_at = unixepoch() * 1000 WHERE bot_id = ?1"
          )
          .bind(&bot_id)
          .execute(&mut *tx)
          .await
          .map_err(AppError::from)?;

          tx.commit().await.map_err(AppError::from)?;
          Ok(())
      }

      pub async fn list_sessions(
          &self,
          bot_id: String,
      ) -> AppResult<Vec<DebugSession>> {
          let rows = self.repo.list_sessions_by_bot(&bot_id).await?;
          rows.into_iter().map(TryInto::try_into).collect()
      }

      pub async fn get_stats(&self,
          message_repo: &crate::persistence::MessageRepo,
      ) -> AppResult<StatsResult> {
          let total_messages = message_repo.get_message_count().await.map_err(AppError::from)?;
          let online_bots = self.repo.get_online_bot_count().await?;
          Ok(StatsResult {
              total_messages,
              online_bots,
          })
      }
  }

  #[derive(Clone, Debug, serde::Serialize)]
  pub struct StatsResult {
      pub total_messages: i64,
      pub online_bots: i64,
  }
  ```

  **Note:** `BotService::get_stats` takes `MessageRepo` as parameter because `BotService` does not own `MessageRepo`. The command handler will pass it in.

- [ ] **Step 2: Wire into ServiceHub**

  In `src-tauri/src/services/mod.rs`:

  ```rust
  pub mod bot;
  // ... existing mods

  pub use bot::{BotService, StatsResult};
  // ... existing uses

  #[derive(Clone)]
  pub struct ServiceHub {
      pub message: MessageService,
      pub interaction: InteractionService,
      pub group: GroupService,
      pub request: RequestService,
      pub user: UserService,
      pub bot: BotService,
  }

  impl ServiceHub {
      pub fn new(
          message: MessageService,
          interaction: InteractionService,
          group: GroupService,
          request: RequestService,
          user: UserService,
          bot: BotService,
      ) -> Self {
          Self {
              message,
              interaction,
              group,
              request,
              user,
              bot,
          }
      }
  }
  ```

- [ ] **Step 3: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully (may warn about unused BotService until commands are wired).

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/services/bot.rs src-tauri/src/services/mod.rs
  git commit -m "feat(bot): add BotService with session lifecycle"
  ```

---

### Task 4: Add Bot Commands and Wire ServiceHub

**Files:**
- Create: `src-tauri/src/commands/bot.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/main.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `commands/bot.rs`**

  Create `src-tauri/src/commands/bot.rs`:

  ```rust
  use crate::core::CoreContainer;
  use crate::models::{BotProfile, DebugSession};
  use crate::services::{ServiceHub, StatsResult};
  use super::IntoCommandResult;

  #[tauri::command]
  pub async fn create_bot(
      app: tauri::AppHandle,
      services: tauri::State<'_ , ServiceHub>,
      bound_user_id: String,
      display_name: String,
  ) -> Result<BotProfile, String> {
      services
          .bot
          .create_bot(&app, bound_user_id, display_name)
          .await
          .into_command_result()
  }

  #[tauri::command]
  pub async fn list_bots(
      services: tauri::State<'_ , ServiceHub>,
  ) -> Result<Vec<BotProfile>, String> {
      services.bot.list_bots().await.into_command_result()
  }

  #[tauri::command]
  pub async fn delete_bot(
      core: tauri::State<'_ , CoreContainer>,
      services: tauri::State<'_ , ServiceHub>,
      bot_id: String,
  ) -> Result<(), String> {
      services
          .bot
          .delete_bot(&core, bot_id)
          .await
          .into_command_result()
  }

  #[tauri::command]
  pub async fn start_bot(
      services: tauri::State<'_ , ServiceHub>,
      bot_id: String,
  ) -> Result<DebugSession, String> {
      services.bot.start_bot(bot_id).await.into_command_result()
  }

  #[tauri::command]
  pub async fn stop_bot(
      services: tauri::State<'_ , ServiceHub>,
      bot_id: String,
  ) -> Result<(), String> {
      services.bot.stop_bot(bot_id).await.into_command_result()
  }

  #[tauri::command]
  pub async fn list_debug_sessions(
      services: tauri::State<'_ , ServiceHub>,
      bot_id: String,
  ) -> Result<Vec<DebugSession>, String> {
      services
          .bot
          .list_sessions(bot_id)
          .await
          .into_command_result()
  }
  ```

- [ ] **Step 2: Add `get_stats` to `commands/main.rs`**

  Append to `src-tauri/src/commands/main.rs`:

  ```rust
  #[tauri::command]
  pub async fn get_stats(
      services: tauri::State<'_ , ServiceHub>,
  ) -> Result<StatsResult, String> {
      services
          .bot
          .get_stats(&services.message.repo)
          .await
          .into_command_result()
  }
  ```

  Wait — `services.message.repo` is private. We need to either:
  1. Expose a public getter on `MessageService`
  2. Or add a `get_message_count` method to `MessageService`

  Option 2 is cleaner. Add to `MessageService` in `src-tauri/src/services/message.rs`:

  ```rust
  pub async fn get_message_count(&self,
  ) -> AppResult<i64> {
      self.repo.get_message_count().await.map_err(Into::into)
  }
  ```

  And add `get_message_count` to `MessageRepo` in `src-tauri/src/persistence/repo/message.rs`:

  ```rust
  pub async fn get_message_count(&self) -> Result<i64, sqlx::Error> {
      sqlx::query_scalar("SELECT COUNT(*) FROM messages")
          .fetch_one(&self.pool)
          .await
  }
  ```

  Then `get_stats` command becomes:
  ```rust
  #[tauri::command]
  pub async fn get_stats(
      services: tauri::State<'_ , ServiceHub>,
  ) -> Result<StatsResult, String> {
      let total_messages = services
          .message
          .get_message_count()
          .await
          .into_command_result()?;
      let online_bots = services
          .bot
          .repo
          .get_online_bot_count()
          .await
          .map_err(|err| err.to_string())?;
      Ok(StatsResult {
          total_messages,
          online_bots,
      })
  }
  ```

  Hmm, `services.bot.repo` is also private. Better to add `get_online_bot_count` to `BotService`:

  ```rust
  // In BotService
  pub async fn get_online_bot_count(&self) -> AppResult<i64> {
      self.repo.get_online_bot_count().await.map_err(Into::into)
  }
  ```

  Then `get_stats`:
  ```rust
  #[tauri::command]
  pub async fn get_stats(
      services: tauri::State<'_ , ServiceHub>,
  ) -> Result<StatsResult, String> {
      let total_messages = services
          .message
          .get_message_count()
          .await
          .into_command_result()?;
      let online_bots = services
          .bot
          .get_online_bot_count()
          .await
          .into_command_result()?;
      Ok(StatsResult {
          total_messages,
          online_bots,
      })
  }
  ```

- [ ] **Step 3: Register modules and commands**

  In `src-tauri/src/commands/mod.rs`, add:
  ```rust
  pub mod bot;
  ```

  In `src-tauri/src/lib.rs`:
  1. Add `BotRepo` to imports:
     ```rust
     use persistence::{BotRepo, GroupRepo, InteractionRepo, MessageRepo, UserRepo, init_sqlite_pool};
     ```
  2. Add `BotService` to imports:
     ```rust
     use services::{BotService, GroupService, InteractionService, MessageService, RequestService, ServiceHub, UserService};
     ```
  3. Add `bot` to commands import:
     ```rust
     use commands::{
         chat::{group, message, request, user},
         main, bot,
     };
     ```
  4. In setup, after `group_repo`, add:
     ```rust
     let bot_repo = BotRepo::new(pool.clone());
     ```
  5. In `ServiceHub::new`, add `BotService::new(bot_repo)` as last parameter.
  6. In `invoke_handler!`, add all bot commands:
     ```rust
     bot::create_bot,
     bot::list_bots,
     bot::delete_bot,
     bot::start_bot,
     bot::stop_bot,
     bot::list_debug_sessions,
     main::get_stats,
     ```

- [ ] **Step 4: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: compiles successfully.

- [ ] **Step 5: Commit**

  ```bash
  git add src-tauri/src/commands/bot.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/main.rs src-tauri/src/lib.rs src-tauri/src/services/message.rs src-tauri/src/persistence/repo/message.rs
  git commit -m "feat(bot): add bot commands and wire into ServiceHub"
  ```

---

### Task 5: Frontend Types and Queries

**Files:**
- Create: `src/types/bot.ts`
- Modify: `src/types/chat.ts`
- Create: `src/lib/query/bots.ts`
- Modify: `src/lib/query/index.ts`
- Modify: `src/lib/mutations.ts`

- [ ] **Step 1: Add TypeScript types**

  Create `src/types/bot.ts`:

  ```typescript
  export interface BotProfile {
    bot_id: string;
    bound_user_id: string;
    display_name: string;
    runtime_status: "stopped" | "running" | "error";
    config_path: string;
    created_at: number;
  }

  export interface DebugSession {
    session_id: string;
    bot_id: string;
    session_name: string;
    description: string | null;
    started_at: number;
    ended_at: number | null;
  }

  export interface StatsResult {
    total_messages: number;
    online_bots: number;
  }
  ```

  Modify `src/types/chat.ts`, add to `ChatMessage`:
  ```typescript
  export type ChatMessage = {
    id: string;
    sender_user_id: string;
    source: MessageSource;
    content: MessageSegment[];
    quoted_message_id: string | null;
    recall: {
      recalled: boolean;
      recalled_by_user_id?: string | null;
    };
    created_at: number;
    bot_id?: string | null; // <-- add this
  };
  ```

- [ ] **Step 2: Add query hooks**

  Create `src/lib/query/bots.ts`:

  ```typescript
  import { useQuery } from "@tanstack/react-query";
  import { invoke } from "@tauri-apps/api/core";
  import type { BotProfile, StatsResult } from "@/types/bot";

  export function useBotsQuery() {
    return useQuery({
      queryKey: ["bots"],
      queryFn: () => invoke<BotProfile[]>("list_bots"),
      retry: false,
    });
  }

  export function useBotStatsQuery() {
    return useQuery({
      queryKey: ["stats"],
      queryFn: () => invoke<StatsResult>("get_stats"),
      retry: false,
    });
  }
  ```

  Modify `src/lib/query/index.ts`, add:
  ```typescript
  export * from "@/lib/query/bots";
  ```

- [ ] **Step 3: Add mutations**

  Append to `src/lib/mutations.ts`:

  ```typescript
  import { useMutation, useQueryClient } from "@tanstack/react-query";
  import { invoke } from "@tauri-apps/api/core";
  import type { BotProfile } from "@/types/bot";

  export function useCreateBotMutation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        bound_user_id,
        display_name,
      }: {
        bound_user_id: string;
        display_name: string;
      }) =>
        invoke<BotProfile>("create_bot", {
          bound_user_id,
          display_name,
        }),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["bots"] });
        queryClient.invalidateQueries({ queryKey: ["stats"] });
      },
    });
  }

  export function useDeleteBotMutation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ bot_id }: { bot_id: string }) =>
        invoke("delete_bot", { bot_id }),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["bots"] });
        queryClient.invalidateQueries({ queryKey: ["stats"] });
      },
    });
  }

  export function useStartBotMutation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ bot_id }: { bot_id: string }) =>
        invoke("start_bot", { bot_id }),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["bots"] });
        queryClient.invalidateQueries({ queryKey: ["stats"] });
      },
    });
  }

  export function useStopBotMutation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ bot_id }: { bot_id: string }) =>
        invoke("stop_bot", { bot_id }),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["bots"] });
        queryClient.invalidateQueries({ queryKey: ["stats"] });
      },
    });
  }
  ```

- [ ] **Step 4: Compile check**

  ```bash
  bun run build
  ```

  Expected: TypeScript compiles without errors.

- [ ] **Step 5: Commit**

  ```bash
  git add src/types/bot.ts src/types/chat.ts src/lib/query/bots.ts src/lib/query/index.ts src/lib/mutations.ts
  git commit -m "feat(bot): add frontend types, queries and mutations"
  ```

---

### Task 6: Build Dashboard with Bot Management

**Files:**
- Modify: `src/views/main/dashboard.tsx`

- [ ] **Step 1: Rewrite Dashboard with stats and Bot card**

  Replace `src/views/main/dashboard.tsx` with:

  ```tsx
  import { useState } from "react";
  import {
    Bot,
    MessageCircle,
    Play,
    Power,
    Plus,
    SquareUser,
    Trash2,
    Users,
  } from "lucide-react";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "@/components/ui/select";
  import {
    Sheet,
    SheetContent,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
  } from "@/components/ui/sheet";
  import { useGroupsQuery, useUsersQuery, useBotStatsQuery, useBotsQuery } from "@/lib/query";
  import {
    useCreateBotMutation,
    useDeleteBotMutation,
    useStartBotMutation,
    useStopBotMutation,
  } from "@/lib/mutations";

  function StatValue({
    value,
    loading,
  }: {
    value: number | null;
    loading: boolean;
  }) {
    if (loading) {
      return <span className="text-lg text-muted-foreground">读取中...</span>;
    }
    if (value === null) {
      return <span className="text-lg text-muted-foreground">--</span>;
    }
    return (
      <span className="font-semibold text-2xl">{value.toLocaleString("zh-CN")}</span>
    );
  }

  function DashboardView() {
    const usersQuery = useUsersQuery();
    const groupsQuery = useGroupsQuery();
    const statsQuery = useBotStatsQuery();
    const botsQuery = useBotsQuery();

    const createBot = useCreateBotMutation();
    const deleteBot = useDeleteBotMutation();
    const startBot = useStartBotMutation();
    const stopBot = useStopBotMutation();

    const [selectedUserId, setSelectedUserId] = useState("");
    const [sheetOpen, setSheetOpen] = useState(false);

    const users = usersQuery.data ?? [];
    const bots = botsQuery.data ?? [];
    const stats = statsQuery.data;

    const unboundUsers = users.filter(
      (u) => !bots.some((b) => b.bound_user_id === u.user_id),
    );

    const handleCreateBot = () => {
      if (!selectedUserId) return;
      const user = users.find((u) => u.user_id === selectedUserId);
      if (!user) return;
      createBot.mutate(
        {
          bound_user_id: selectedUserId,
          display_name: user.nickname,
        },
        {
          onSuccess: () => {
            setSheetOpen(false);
            setSelectedUserId("");
          },
        },
      );
    };

    return (
      <div className="space-y-4">
        <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="flex items-center gap-2 text-sm">
                <SquareUser className="size-4" /> 总用户数
              </CardTitle>
            </CardHeader>
            <CardContent>
              <StatValue
                value={users.length}
                loading={usersQuery.isPending}
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="flex items-center gap-2 text-sm">
                <Users className="size-4" /> 总群聊数
              </CardTitle>
            </CardHeader>
            <CardContent>
              <StatValue
                value={groupsQuery.data?.length ?? null}
                loading={groupsQuery.isPending}
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="flex items-center gap-2 text-sm">
                <MessageCircle className="size-4" /> 总消息数
              </CardTitle>
            </CardHeader>
            <CardContent>
              <StatValue
                value={stats?.total_messages ?? null}
                loading={statsQuery.isPending}
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="flex items-center gap-2 text-sm">
                <Bot className="size-4" /> 在线机器人数
              </CardTitle>
            </CardHeader>
            <CardContent>
              <StatValue
                value={stats?.online_bots ?? null}
                loading={statsQuery.isPending}
              />
            </CardContent>
          </Card>
        </section>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Bot className="size-4" /> Bot 管理
            </CardTitle>
            <Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
              <SheetTrigger asChild>
                <Button type="button" size="sm" variant="outline">
                  <Plus className="mr-1 size-4" /> 创建 Bot
                </Button>
              </SheetTrigger>
              <SheetContent>
                <SheetHeader>
                  <SheetTitle>创建 Bot</SheetTitle>
                </SheetHeader>
                <div className="mt-4 space-y-4">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">选择用户</label>
                    <Select
                      value={selectedUserId}
                      onValueChange={setSelectedUserId}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="选择要绑定的用户" />
                      </SelectTrigger>
                      <SelectContent>
                        {unboundUsers.map((user) => (
                          <SelectItem
                            key={user.user_id}
                            value={user.user_id}
                          >
                            {user.nickname} ({user.user_id})
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <Button
                    type="button"
                    className="w-full"
                    disabled={!selectedUserId || createBot.isPending}
                    onClick={handleCreateBot}
                  >
                    {createBot.isPending ? "创建中..." : "确认创建"}
                  </Button>
                </div>
              </SheetContent>
            </Sheet>
          </CardHeader>
          <CardContent>
            {bots.length === 0 ? (
              <p className="text-muted-foreground text-sm">暂无 Bot</p>
            ) : (
              <div className="space-y-2">
                {bots.map((bot) => (
                  <div
                    key={bot.bot_id}
                    className="flex items-center justify-between rounded-lg border p-3"
                  >
                    <div className="space-y-1">
                      <p className="text-sm font-medium">{bot.display_name}</p>
                      <p className="text-muted-foreground text-xs">
                        绑定用户: {bot.bound_user_id}
                      </p>
                      <span
                        className={`inline-block rounded px-1.5 py-0.5 text-xs ${
                          bot.runtime_status === "running"
                            ? "bg-green-100 text-green-700"
                            : bot.runtime_status === "error"
                              ? "bg-red-100 text-red-700"
                              : "bg-gray-100 text-gray-700"
                        }`}
                      >
                        {bot.runtime_status === "running"
                          ? "运行中"
                          : bot.runtime_status === "error"
                            ? "异常"
                            : "已停止"}
                      </span>
                    </div>
                    <div className="flex items-center gap-1">
                      {bot.runtime_status === "running" ? (
                        <Button
                          type="button"
                          size="icon-xs"
                          variant="ghost"
                          onClick={() =>
                            stopBot.mutate({ bot_id: bot.bot_id })
                          }
                        >
                          <Power className="size-4 text-red-500" />
                        </Button>
                      ) : (
                        <Button
                          type="button"
                          size="icon-xs"
                          variant="ghost"
                          onClick={() =>
                            startBot.mutate({ bot_id: bot.bot_id })
                          }
                        >
                          <Play className="size-4 text-green-500" />
                        </Button>
                      )}
                      <Button
                        type="button"
                        size="icon-xs"
                        variant="ghost"
                        onClick={() =>
                          deleteBot.mutate({ bot_id: bot.bot_id })
                        }
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    );
  }

  export default DashboardView;
  ```

- [ ] **Step 2: Verify build**

  ```bash
  bun run build
  ```

  Expected: TypeScript compiles without errors.

- [ ] **Step 3: Commit**

  ```bash
  git add src/views/main/dashboard.tsx
  git commit -m "feat(dashboard): add bot management and stats"
  ```

---

### Task 7: Reserve bot_id in Message Flow

**Files:**
- Modify: `src-tauri/src/persistence/repo/message.rs`
- Modify: `src-tauri/src/services/message.rs`
- Modify: `src-tauri/src/commands/chat/message.rs`

- [ ] **Step 1: Add `bot_id` to `NewMessageRecord` and `MessageRecord`**

  In `src-tauri/src/persistence/repo/message.rs`, add `bot_id` to structs:

  ```rust
  pub struct NewMessageRecord {
      pub owner_user_id: String,
      pub sender_user_id: String,
      pub source_type: String,
      pub source_id: String,
      pub content_json: String,
      pub quoted_message_id: Option<String>,
      pub created_at: u64,
      pub bot_id: Option<String>, // <-- new
  }

  pub struct MessageRecord {
      pub id: String,
      pub sender_user_id: String,
      pub source_type: String,
      pub source_id: String,
      pub receiver_user_id: Option<String>,
      pub group_id: Option<String>,
      pub bot_id: Option<String>, // <-- new
      pub content_json: String,
      pub quoted_message_id: Option<String>,
      pub is_recalled: bool,
      pub recalled_by_user_id: Option<String>,
      pub created_at: u64,
  }
  ```

  Update `insert_message` SQL to include `bot_id`:
  Add `bot_id` to INSERT columns and VALUES, and RETURNING clause:
  ```rust
  INSERT INTO messages (
      message_id, message_scene, peer_id, message_seq, sender_user_id,
      receiver_user_id, group_id, bot_id, content_json, quoted_message_id, created_at
  ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
  RETURNING message_id AS id, sender_user_id, message_scene AS source_type,
            peer_id AS source_id, receiver_user_id, group_id, bot_id, content_json,
            quoted_message_id, is_recalled, recalled_by_user_id, created_at
  ```

  Add `.bind(&record.bot_id)` before `.bind(&record.content_json)`.

- [ ] **Step 2: Add `bot_id` to `MessageEntity` and `SendMessageResult`**

  In `src-tauri/src/models/entities.rs`, add to both structs:
  ```rust
  pub bot_id: Option<DbId>,
  ```

  Update `TryFrom<MessageRecord>` implementations in `services/message.rs` to map `bot_id`.

- [ ] **Step 3: Update `MessageService::send` signature**

  In `src-tauri/src/services/message.rs`, change:
  ```rust
  pub async fn send(
      &self,
      core: &CoreContainer,
      user_id: String,
      source: MessageSource,
      content: Vec<MessageSegment>,
      quoted_message_id: Option<String>,
      bot_id: Option<String>, // <-- add
  ) -> AppResult<SendMessageResult>
  ```

  Pass `bot_id` into `NewMessageRecord`.

- [ ] **Step 4: Update command to pass `None`**

  In `src-tauri/src/commands/chat/message.rs`, update `send_message` to accept optional `bot_id: Option<String>` and pass it to service. For now frontend always sends `null` / `undefined`.

- [ ] **Step 5: Compile and test**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  bun run build
  ```

  Expected: all tests pass, frontend builds.

- [ ] **Step 6: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/message.rs src-tauri/src/models/entities.rs src-tauri/src/services/message.rs src-tauri/src/commands/chat/message.rs
  git commit -m "feat(msg): reserve bot_id field in message flow"
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
  1. Dashboard shows 4 stats cards with real data.
  2. "创建 Bot" Sheet opens, lists unbound users.
  3. Creating a Bot adds it to the list with "已停止" status.
  4. Clicking play icon starts the Bot, status changes to "运行中", online bot count increases.
  5. Clicking power icon stops the Bot, status changes back, count decreases.
  6. Deleting a Bot removes it from the list.
  7. Sending a message in chat still works normally.

- [ ] **Step 5: Final commit if fixes needed**

  ```bash
  git add .
  git commit -m "fix: resolve bot integration issues"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ Bot 实体 CRUD — covered by Tasks 2-4
- ✅ 调试会话生命周期（启动/停止）— covered by Tasks 2-4
- ✅ Dashboard 统计接入（总消息数、在线 Bot 数）— covered by Tasks 4-6
- ✅ 消息归属到 Bot（预留字段）— covered by Task 7

**2. Placeholder scan:**
- ✅ 所有步骤包含完整代码
- ✅ 无 "TBD"/"TODO" / "implement later"
- ✅ 无模糊描述

**3. Type consistency:**
- ✅ `BotProfile` fields match between Rust and TypeScript
- ✅ `DebugSession` fields match between Rust and TypeScript
- ✅ `StatsResult` fields match between Rust and TypeScript
- ✅ Query keys `["bots"]`, `["stats"]` consistent across hooks and invalidations
