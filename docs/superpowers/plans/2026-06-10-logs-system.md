# Logs 系统 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Scope:** 仅 Logs 系统（协议报文 + 系统日志）。群文件、群优化、OneBot 等全部延后，不在本期。
>
> **Baseline spec:** [docs/superpowers/specs/2026-06-10-logs-system-design.md](../../../docs/superpowers/specs/2026-06-10-logs-system-design.md)

**Goal:** 让 Logs 页面成为统一的运行日志入口，同时展示协议报文（`protocol_packets`）和系统日志（`tracing` JSON Lines 文件），支持按时间窗口查询、等级筛选、数据源切换、游标分页；Settings 页面增加 DEBUG 开关和日志保留期配置。

**Tech Stack:** Rust edition 2024, `tracing` + `tracing-subscriber` (registry/env-filter/reload) + `tracing-appender`, sqlx 0.8, SQLite, Tauri 2, React/TypeScript, TanStack Query, shadcn/ui

---

## 1. 分页设计

### 1.1 游标分页模型

Logs 数据量大（protocol_packets 可能百万级）、时间序列天然有序，用 offset 分页会导致跳页不一致。采用**时间戳游标**（`before` + `limit`）：

| 参数 | 类型 | 说明 |
|------|------|------|
| `since` | `u64` | 毫秒时间戳，起始边界（包含） |
| `until` | `u64` | 毫秒时间戳，结束边界（包含） |
| `before` | `Option<u64>` | 游标：返回 `time < before` 的条目。首次查询传 `None`（或 `until`），翻页传最后一条的 `time` |
| `limit` | `u32` | 每页条数，默认 50，上限 200 |

- 排序固定 DESC（最新在前）
- 翻页逻辑：取上一页最后一条的 `time` 作为下一页 `before`
- **时间戳碰撞**：多条记录 `time` 相同，用 `before` 严格小于会导致漏行。后端实际用 `time < before OR (time = before AND id < last_id)` 兜底。前端不感知，只传 `before = last_time`

### 1.2 单源 vs 跨源分页

| 数据源 | 分页方式 |
|--------|---------|
| `packet` | 直接用 `created_at < before` 游标 |
| `system` | 遍历日期文件 → 合并 → 全局排序后按 `before` 截断 |
| `all` | **一期放弃精确跨源分页**。原因：两源独立查询后合并，跨源游标无法定位。一期 `all` 模式只展示固定条数（`limit * 2`，各源 limit 条合并），不做翻页。翻页仅支持单源模式（`packet` 或 `system`） |

### 1.3 分页响应结构

```rust
#[derive(serde::Serialize)]
pub struct LogPage<T> {
    pub items: Vec<T>,
    pub has_more: bool,      // 后端判断：返回条数 == limit 时 true
    pub next_before: Option<u64>, // 最后一条的 time，供前端直接传下一页
}
```

前端：
```typescript
export interface LogPage<T> {
  items: T[];
  has_more: boolean;
  next_before: number | null;
}
```

---

## 2. 文件结构变更

### 后端

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/Cargo.toml` | 修改 | 添加 `tracing`/`tracing-subscriber`/`tracing-appender` 依赖 |
| `src-tauri/src/logging.rs` | **新建** | 自定义 JSON Layer + visitor；`LogGuard`/`LogReloadHandle` State 包装；日志清理函数 |
| `src-tauri/src/lib.rs` | 修改 | `setup` 中初始化 tracing（只调一次 `set_global_default`）、manage guard/handle、启动清理 task；追加 5 个 command |
| `src-tauri/src/utils.rs` | 修改 | 2 处 `eprintln!` → `tracing`；加日志清理辅助函数 |
| `src-tauri/src/persistence/repo/settings.rs` | **新建** | `SettingsRepo`：`get_setting` / `set_setting` |
| `src-tauri/src/persistence/repo/mod.rs` | 修改 | `pub mod settings;` + `pub use settings::SettingsRepo;` |
| `src-tauri/src/persistence/repo/packet.rs` | 修改 | `list_packets` 增加 `until`、`is_error`、`before`、返回游标分页结构 |
| `src-tauri/src/persistence/repo/system_log.rs` | **新建** | `SystemLogReader`：读日志文件，支持 `since`/`until`/`level`/`before`/`limit` |
| `src-tauri/src/commands/main.rs` | 修改 | 新增 `list_system_logs` / `get_log_settings` / `set_log_level` / `set_log_retention` / `trigger_log_cleanup` |
| `src-tauri/src/commands/packet.rs` | 修改 | `list_protocol_packets` 透传 `until` / `is_error` / `before`，返回 `LogPage` |
| `src-tauri/src/services/bot.rs` | 修改 | Bot 启动/停止/创建/删除埋点；1 处 `eprintln!` → `tracing` |
| `src-tauri/src/services/user.rs` | 修改 | 用户注册/删除埋点 |
| `src-tauri/src/services/group/basic.rs` | 修改 | 群创建埋点 |
| `src-tauri/src/services/group/management.rs` | 修改 | 群解散埋点 |
| `src-tauri/src/services/message.rs` | 修改 | 消息发送失败埋点 |
| `src-tauri/src/protocol/runtime.rs` | 修改 | 3 处 `eprintln!` → `tracing` |
| `src-tauri/src/protocol/recorder.rs` | 重写 | 1 处 `eprintln!` → `tracing`；**分层写入**：错误/关键操作立即 DB，普通操作批量缓冲，低价值只写文件 |
| `src-tauri/src/protocol/server.rs` | 修改 | 2 处 `eprintln!` → `tracing`；协议适配器错误埋点 |

### 前端

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/types/log.ts` | **新建** | `LogEntry` / `LogLevel` / `LogDataSource` / `SystemLogEntry` / `LogFilters` / `LogPage` |
| `src/types/settings.ts` | **新建** | `LogSettings` |
| `src/types/packet.ts` | 修改 | `PacketFilters` 增加 `until` / `is_error` / `before` |
| `src/lib/query/logs.ts` | **新建** | `useSystemLogsQuery`（含翻页） / `useLogsQuery`（合并 + adapter，all 模式不分页） |
| `src/lib/query/settings.ts` | **新建** | `useLogSettingsQuery` / `useSetLogLevelMutation` / `useSetLogRetentionMutation` |
| `src/lib/query/keys.ts` | 修改 | 增加 `logs` / `settings` query keys |
| `src/lib/query/index.ts` | 修改 | 导出新增的两个文件 |
| `src/lib/mutations.ts` | 修改 | 追加 settings 相关 mutation（或直接在 `query/settings.ts` 导出） |
| `src/views/main/logs.tsx` | 重写 | 接入真实数据：数据源 Select、时间范围传 `since`/`until`、等级/eventType 筛选、单源翻页 |
| `src/views/main/settings.tsx` | 修改 | 新增"日志设置"卡片：DEBUG 开关、保留期下拉、立即清理按钮 |

---

## 3. 实现顺序

必须按顺序执行：Phase 1 → Phase 2 是后续所有步骤的前置；Phase 3-7 可部分并行。

### Phase 1: `app_settings` 通用读写设施

**Files:**
- `src-tauri/src/persistence/repo/settings.rs` (新建)
- `src-tauri/src/persistence/repo/mod.rs` (修改)
- `src/lib/query/settings.ts` (新建)
- `src/lib/query/keys.ts` (修改)
- `src/lib/query/index.ts` (修改)
- `src/types/settings.ts` (新建)

- [x] **Step 1.1: 创建 `SettingsRepo`**

  在 `src-tauri/src/persistence/repo/settings.rs` 实现：

  ```rust
  use sqlx::SqlitePool;

  #[derive(Clone)]
  pub struct SettingsRepo {
      pool: SqlitePool,
  }

  impl SettingsRepo {
      pub fn new(pool: SqlitePool) -> Self {
          Self { pool }
      }

      pub async fn get_setting(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
          sqlx::query_scalar::<_, String>(
              "SELECT setting_value FROM app_settings WHERE setting_key = ?1"
          )
          .bind(key)
          .fetch_optional(&self.pool)
          .await
      }

      pub async fn set_setting(
          &self,
          key: &str,
          value: &str,
          value_type: &str,
      ) -> Result<(), sqlx::Error> {
          sqlx::query(
              "INSERT INTO app_settings (setting_key, setting_value, value_type, updated_at)
               VALUES (?1, ?2, ?3, unixepoch() * 1000)
               ON CONFLICT(setting_key) DO UPDATE SET
                 setting_value = excluded.setting_value,
                 value_type = excluded.value_type,
                 updated_at = excluded.updated_at"
          )
          .bind(key)
          .bind(value)
          .bind(value_type)
          .execute(&self.pool)
          .await?;
          Ok(())
      }
  }
  ```

- [x] **Step 1.2: 注册 `SettingsRepo`**

  修改 `src-tauri/src/persistence/repo/mod.rs`：
  - 加 `pub mod settings;`
  - 加 `pub use settings::SettingsRepo;`

- [x] **Step 1.3: 前端 settings 类型与 query**

  新建 `src/types/settings.ts`：

  ```typescript
  export interface LogSettings {
    level: "debug" | "info";
    retentionDays: number;
  }
  ```

  新建 `src/lib/query/settings.ts`，导出：
  - `useLogSettingsQuery`
  - `useSetLogLevelMutation`
  - `useSetLogRetentionMutation`
  - `invalidateLogSettingsQuery`

  修改 `src/lib/query/keys.ts` 增加：

  ```typescript
  settings: {
    log: () => ["settings", "log"] as const;
  },
  ```

  修改 `src/lib/query/index.ts` 导出新增文件。

**Acceptance:**
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过
- 前端能成功调用 `get_log_settings` / `set_log_level` / `set_log_retention`

---

### Phase 2: `tracing` 基础设施（自定义 JSON Layer + reload + 清理）

**Files:**
- `src-tauri/Cargo.toml` (修改)
- `src-tauri/src/logging.rs` (新建)
- `src-tauri/src/lib.rs` (修改)
- `src-tauri/src/utils.rs` (修改)

- [x] **Step 2.1: 添加依赖**

  修改 `src-tauri/Cargo.toml`：

  ```toml
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["registry", "env-filter", "reload"] }
  tracing-appender = "0.2"
  ```

- [x] **Step 2.2: 实现自定义 JSON Layer**

  新建 `src-tauri/src/logging.rs`。核心内容：

  1. `struct JsonLayer<W: MakeWriter + 'static>` 实现 `tracing_subscriber::Layer<S>`
  2. `on_event` 中构造 visitor 收集字段，把 `message` 字段提升为顶层 `msg`
  3. 输出格式：
     ```json
     {"ts":1718035200000,"level":"INFO","target":"unibot::services::message","msg":"...","fields":{"k":"v"}}
     ```
  4. `LogGuard(pub WorkerGuard)` — `WorkerGuard` 本身不是 `Clone`，也不需 derive。直接 struct 包装，Tauri State 会持有它
  5. `LogReloadHandle(pub reload::Handle<EnvFilter, Registry>)` — 这个可以 `#[derive(Clone)]`，用于热切换

  关键实现约束：
  - writer 用 `tracing_appender::rolling::RollingFileAppender` daily + `non_blocking`
  - 文件命名：`Builder::filename_prefix("unibot").filename_suffix("log")` → `unibot.YYYY-MM-DD.log`
  - `WorkerGuard` 必须被 `app.manage(LogGuard(guard))` 持有整个进程生命周期

- [x] **Step 2.3: 在 `lib.rs` setup 中初始化 tracing**

  在 `app.manage(pool.clone());` 之前：

  1. 从数据库读取 `log.level`（缺省 `"info"`）和 `log.retention_days`（缺省 `"7"`）
  2. 构造 `RollingFileAppender`，目录 `{app_data_dir}/logs`
  3. `non_blocking(appender)` → writer + guard
  4. `app.manage(LogGuard(guard))`
  5. 构造 `EnvFilter`，默认级别从 setting 读取；用 `reload::Layer::new(filter)` 得到 handle
  6. `tracing_subscriber::registry().with(filter_layer).with(json_layer).init()`
  7. `app.manage(LogReloadHandle(handle))`
  8. 启动 tokio task：延迟 5 分钟执行一次清理，之后每 24 小时一次

  **注意：** `tracing_subscriber::registry()...init()` 只能调用一次。Tauri 主进程只跑一次 setup，安全。测试环境用 `#[sqlx::test]` 不走 setup，不冲突。

- [x] **Step 2.4: 日志清理函数**

  在 `src-tauri/src/utils.rs` 增加：

  ```rust
  pub async fn cleanup_old_logs(log_dir: &std::path::Path, retention_days: i64) -> std::io::Result<()> {
      // 匹配 unibot.YYYY-MM-DD.log，解析日期，删除过期的
  }
  ```

  文件名解析可用正则 `^unibot\.(\d{4})-(\d{2})-(\d{2})\.log$`，失败项跳过。

**Acceptance:**
- `bunx tauri dev` 启动后 `{app_data_dir}/logs/unibot.YYYY-MM-DD.log` 出现
- 在 service 中写一条 `tracing::info!`，文件中能看到正确 JSON
- 切换 DEBUG 开关后，新日志等级实时生效

---

### Phase 3: 埋点 + `eprintln!` 迁移

**Files:**
- `src-tauri/src/services/bot.rs`
- `src-tauri/src/services/user.rs`
- `src-tauri/src/services/group/basic.rs`
- `src-tauri/src/services/group/management.rs`
- `src-tauri/src/services/message.rs`
- `src-tauri/src/protocol/runtime.rs`
- `src-tauri/src/protocol/recorder.rs`
- `src-tauri/src/protocol/server.rs`
- `src-tauri/src/utils.rs`
- `src-tauri/src/lib.rs`

- [x] **Step 3.1: Service 层埋点**

  按 spec 1.5 的表格插入 `tracing` 调用：

  | 位置 | 调用 |
  |------|------|
  | `services/bot.rs` `start_bot` success 分支 | `tracing::info!(target: "unibot::bot", bot_id = %bot_id, "bot started");` |
  | `services/bot.rs` `stop_bot` success 分支 | `tracing::info!(target: "unibot::bot", bot_id = %bot_id, "bot stopped");` |
  | `services/bot.rs` `create_bot` 返回前 | `tracing::info!(target: "unibot::bot", bot_id = %bot.bot_id, bound_user_id = %bot.bound_user_id, "bot created");` |
  | `services/bot.rs` `delete_bot` 返回前 | `tracing::info!(target: "unibot::bot", bot_id = %bot_id, "bot deleted");` |
  | `services/group/basic.rs` `upsert_group` 新建群 | `tracing::info!(target: "unibot::group", group_id = %group_id, owner_user_id = %user_id, "group created");` |
  | `services/group/management.rs` `dissolve_group` 成功 | `tracing::info!(target: "unibot::group", group_id = %group_id, "group dissolved");` |
  | `services/user.rs` `register_user` 返回前 | `tracing::info!(target: "unibot::user", user_id = %profile.user_id, "user registered");` |
  | `services/user.rs` `delete_user` 返回前 | `tracing::info!(target: "unibot::user", user_id = %user_id, "user deleted");` |
  | `services/message.rs` send 错误分支 | `tracing::error!(target: "unibot::message", user_id = %user_id, error = %err, "message send failed");` |
  | `protocol/server.rs` `is_error` 判定处 | `tracing::error!(target: "unibot::protocol", bot_id = %bot_id, error = %action_name, "protocol adapter error");`（或按实际错误内容） |

- [x] **Step 3.2: `eprintln!` 迁移**

  全部 10 处改为对应 `tracing::warn!` / `error!`：
  - `utils.rs` 2 处 group member 查询失败 → `tracing::warn!`
  - `lib.rs:108` shutdown timeout → `tracing::warn!`
  - `services/bot.rs:128` config 删除失败 → `tracing::warn!`
  - `protocol/runtime.rs` 3 处 → `tracing::error!` / `warn!`
  - `protocol/recorder.rs:172` → `tracing::error!`
  - `protocol/server.rs` 2 处 → `tracing::error!`

**Acceptance:**
- `grep -rn "eprintln!" src-tauri/src/` 返回空
- 执行 Bot 启停、用户注册、群创建后日志文件中有对应 JSON 行

---

### Phase 3.5: 协议日志分层写入

**Files:**
- `src-tauri/src/protocol/recorder.rs` (重写)
- `src-tauri/src/protocol/runtime.rs` (修改)

**分层策略：**

| 层级 | 判定条件 | 文件写入 | 数据库索引 | 写入方式 |
|------|---------|---------|-----------|---------|
| **Critical** | `is_error = true` | ✅ 同步 | ✅ **立即 INSERT** | 和现状一样，保证错误立即可查 |
| **Normal** | `action_name` ∈ 白名单（见下） | ✅ 同步 | ✅ **批量缓冲** | 积累到 50 条或 100ms 后批量 INSERT |
| **Low** | 其他（心跳、普通事件等） | ✅ 同步 | ❌ **不入 DB** | 只写文件，需要时直接读文件 |

**Normal 白名单**（`action_name` 匹配）：
```rust
const NORMAL_ACTIONS: &[&str] = &[
    "send_message", "recall_message", "poke_user",
    "create_friend_request", "handle_friend_request",
    "upsert_group", "dissolve_group", "leave_group",
    "mute_group_member", "kick_group_member",
    "set_group_whole_mute", "set_group_member_role",
];
```

- [x] **Step 3.5.1: 重写 `PacketRecorder` 内部结构**

  ```rust
  struct PacketRecorder {
      app_data_dir: PathBuf,
      pool: SqlitePool,
      // 批量缓冲通道
      batch_tx: mpsc::UnboundedSender<IndexRecord>,
      // 后台 flush task 的 join handle（shutdown 时等待）
      flush_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
  }

  struct IndexRecord {
      packet_id: String,
      bot_id: String,
      profile_id: Option<String>,
      protocol_type: String,
      direction: String,
      action_name: String,
      file_path: String,
      related_object_type: Option<String>,
      related_object_id: Option<String>,
      is_error: bool,
      session_id: Option<String>,
      created_at: i64,
  }
  ```

  内部逻辑：
  1. `record()` 先同步写文件（不变）
  2. 根据 `is_error` 和 `action_name` 判定层级：
     - `Critical` → 立即 `INSERT INTO protocol_packets`
     - `Normal` → `batch_tx.send(index_record)`，立即返回
     - `Low` → 什么都不做（文件已存）
  3. 后台 task 每 **100ms** 或积累 **50 条** 时：
     ```rust
     INSERT INTO protocol_packets (...) VALUES (?,...,?), (?,...,?), ...
     ```
     使用 `QueryBuilder` 构造多行 VALUES。
  4. `flush()` 方法（供 shutdown 调用）：drain channel 剩余条目，立即执行一次批量 INSERT

- [x] **Step 3.5.2: `shutdown_all` 时 flush buffer**

  修改 `runtime.rs` `shutdown_all`：在关闭所有 server 后，调用 `recorder.flush().await`。

  注意：`ProtocolRuntimeManager::stop_bot` 也应在 stop 后 flush（如果 channel 中有该 bot 的 pending 记录）。

- [x] **Step 3.5.3: 前端默认过滤**

  `list_protocol_packets` 默认行为不变——返回 `protocol_packets` 表中的数据。由于 Normal 记录通过缓冲批量入表，查询结果会延迟最多 100ms，对用户体验无影响。

  `LogFilters` 增加可选的 `include_all_packets?: boolean`，但一期不加。Logs 页面默认只展示 DB 中有的记录（Critical + Normal），Low 记录需要手动读文件排查。

**Acceptance:**
- `cargo test` 通过
- 大量心跳请求下数据库 INSERT 频率显著降低（可通过日志观察）
- Bot stop 后 pending buffer 正确 flush 到 DB
- 错误报文（`is_error=true`）立即可在 Logs 页面查到

---

### Phase 4: 后端读取与设置 command

**Files:**
- `src-tauri/src/commands/main.rs`
- `src-tauri/src/commands/packet.rs`
- `src-tauri/src/persistence/repo/packet.rs`
- `src-tauri/src/persistence/repo/system_log.rs` (新建)
- `src-tauri/src/lib.rs`

- [ ] **Step 4.1: 扩展 `PacketRepo::list_packets`（支持游标分页）**

  签名改为：

  ```rust
  #[derive(serde::Serialize)]
  pub struct LogPage<T> {
      pub items: Vec<T>,
      pub has_more: bool,
      pub next_before: Option<u64>,
  }

  pub async fn list_packets(
      &self,
      bot_id: Option<&str>,
      direction: Option<&str>,
      action_name: Option<&str>,
      is_error: Option<bool>,
      since: Option<u64>,
      until: Option<u64>,
      before: Option<u64>,
      limit: i64,
  ) -> Result<LogPage<ProtocolPacketRecord>, sqlx::Error>
  ```

  查询逻辑：
  1. `since` → `created_at >= ?`
  2. `until` → `created_at <= ?`
  3. `before` → `created_at < ?`（与 `until` 取 min）
  4. `is_error` → `is_error = ?`（`bool → i32` 转换）
  5. `ORDER BY created_at DESC LIMIT ?`
  6. 返回条数 == limit → `has_more = true`，`next_before = 最后一条.created_at`

- [ ] **Step 4.2: 扩展 `list_protocol_packets` command**

  透传 `is_error`、`until`、`before`，返回 `LogPage<ProtocolPacketRecord>`。

- [x] **Step 4.3: 新建 `SystemLogReader`**

  新建 `src-tauri/src/persistence/repo/system_log.rs`：

  ```rust
  pub struct SystemLogReader;

  impl SystemLogReader {
      pub async fn list_logs(
          log_dir: &std::path::Path,
          since: u64,
          until: u64,
          level: Option<&str>,
          before: Option<u64>,
          limit: usize,
      ) -> std::io::Result<LogPage<SystemLogEntry>> {
          // 1. 根据 [since, until] 推算覆盖的日期列表
          // 2. 只打开命中的 unibot.YYYY-MM-DD.log
          // 3. 逐行解析 JSON，跳过损坏行
          // 4. 按 ts 范围、level、before 过滤
          // 5. 收集到 Vec，按 ts DESC 排序
          // 6. 截断 limit，计算 has_more / next_before
      }
  }
  ```

  日期推算用 `chrono::NaiveDate` 从 `since/until` 提取，迭代 `succ_opt()`。

- [x] **Step 4.4: 新增系统日志与设置 command**

  在 `src-tauri/src/commands/main.rs` 增加：

  - `list_system_logs(app: AppHandle, since, until, level, before, limit)` → `LogPage<SystemLogEntry>`
  - `get_log_settings(pool: State<SqlitePool>)` → `{ level: String, retention_days: i64 }`
  - `set_log_level(level: String, handle: State<LogReloadHandle>, pool: State<SqlitePool>)` → `()`
  - `set_log_retention(days: i64, pool: State<SqlitePool>)` → `()`
  - `trigger_log_cleanup(app: AppHandle)` → `()`

  注意点：
  - `list_system_logs` 需要 `AppHandle` 取 `app_data_dir`
  - `set_log_level` 需要 `State<LogReloadHandle>` 热切换 + 持久化到 `app_settings`
  - `set_log_retention` 只持久化，不立即执行清理
  - `trigger_log_cleanup` 立即按当前 retention_days 执行一次清理

- [x] **Step 4.5: 注册 command**

  修改 `src-tauri/src/lib.rs:129` 的 `generate_handler!`，追加 5 个新 command。

**Acceptance:**
- `cargo test` 通过
- `list_protocol_packets` 首次返回 50 条，`has_more` 正确
- `list_system_logs` 能正确按 `before` 翻页
- DEBUG 开关能在运行时改变日志级别

---

### Phase 5: 前端 Logs 页面

**Files:**
- `src/types/log.ts` (新建)
- `src/types/packet.ts` (修改)
- `src/lib/query/logs.ts` (新建)
- `src/lib/query/keys.ts` (修改)
- `src/lib/query/index.ts` (修改)
- `src/views/main/logs.tsx` (重写)

- [x] **Step 5.1: 类型定义**

  新建 `src/types/log.ts`：

  ```typescript
  export type LogLevel = "debug" | "info" | "warn" | "error";
  export type LogDataSource = "all" | "packet" | "system";

  export interface LogEntry {
    id: string;
    time: number;
    level: LogLevel;
    eventType: string;
    source: string;
    message: string;
    dataSource: "packet" | "system";
    detailRef: string | null;
  }

  export interface SystemLogEntry {
    ts: number;
    level: string;
    target: string;
    msg: string;
    fields: Record<string, unknown>;
  }

  export interface LogFilters {
    since: number;
    until: number;
    levels?: LogLevel[];
    eventTypes?: string[];
    limit?: number;
  }

  export interface LogPage<T> {
    items: T[];
    has_more: boolean;
    next_before: number | null;
  }
  ```

- [x] **Step 5.2: 前端 query hooks**

  新建 `src/lib/query/logs.ts`：

  - `useSystemLogsQuery(filters, before)` → TanStack Query，`queryKey` 包含 `before` 和 `filters` 的 `since/until`
  - `useLogsQuery(dataSource, filters, before?)`：
    - `"packet"`：调用 `useProtocolPacketsQuery`（含 `before` 翻页）
    - `"system"`：调用 `useSystemLogsQuery`（含 `before` 翻页）
    - `"all"`：两者都调用 **第一页**（不传 `before`，各取 limit 条），合并排序，**不支持翻页**
    - adapter 转 `LogEntry`：
      - packet：`id = packet:${p.packet_id}`，`time = p.created_at`，`level = p.is_error ? "error" : "info"`，`eventType = "packet.${p.direction}"`，`source = p.bot_id ?? p.profile_id ?? "system"`，`message = p.action_name`，`dataSource = "packet"`，`detailRef = p.packet_id`
      - system：`id = system:${entry.ts}:${seq}`，`time = entry.ts`，`level = entry.level.toLowerCase()`，`eventType = inferSystemEventType(entry.target, entry.msg)`，`source = entry.target`，`message = entry.msg`，`dataSource = "system"`，`detailRef = JSON.stringify({ target: entry.target, fields: entry.fields })`
    - 轮询 5s

  修改 `keys.ts` 增加 `logs` 相关 key。

- [x] **Step 5.3: 重写 Logs 页面**

  修改 `src/views/main/logs.tsx`：

  - 增加"日志来源" Select：`全部 / 协议日志 / 系统日志`
  - 时间范围下拉 `15m / 1h / 24h / 7d`，换算成 `since = Date.now() - windowMs`，`until = Date.now()`
  - 等级筛选保持 MultiSelectCombobox
  - eventType 筛选简化为粗粒度：`packet` / `system`（按前缀匹配 `eventType`）
  - **翻页 UI**：
    - `packet` / `system` 模式：展示"加载更多"按钮，点击传 `next_before` 到 query
    - `all` 模式：不展示翻页按钮（只展示固定条数）
  - 用 `useLogsQuery` 取真实数据
  - 展示：`time` 用 `formatMessageTimestamp`、`level` badge、`eventType` badge、`source`、message

**Acceptance:**
- Logs 页面能展示 protocol_packets 数据
- 切换到"系统日志"只展示 JSON Lines 文件内容
- `packet` / `system` 模式下"加载更多"正常工作
- `all` 模式下展示合并后的固定条数（各源 limit 条），无翻页按钮
- 等级/eventType 筛选正常工作

---

### Phase 6: 前端 Settings 页面扩展

**Files:**
- `src/views/main/settings.tsx`
- `src/lib/query/settings.ts`

- [x] **Step 6.1: 新增"日志设置"卡片**

  在 `src/views/main/settings.tsx` 增加第二个 `Card`：

  - **启用 DEBUG 日志**：Switch，绑定 `logSettings.level === "debug"`
    - onChange → `setLogLevelMutation.mutate(checked ? "debug" : "info")`
  - **日志保留期**：Select，选项 `1 / 7 / 30` 天
    - onChange → `setLogRetentionMutation.mutate(days)`
  - **立即清理**：Button
    - onClick → `invoke("trigger_log_cleanup")`
    - 成功后 toast 或简单 alert

- [x] **Step 6.2: 接入 query**

  使用 `useLogSettingsQuery` 读取当前设置；mutation 成功后 `invalidateLogSettingsQuery()`。

**Acceptance:**
- Settings 页面能看到当前日志等级和保留期
- 切换 DEBUG 开关后，新日志立即改变（验证：触发一个 `tracing::debug!` 事件）
- 保留期改变持久化到 `app_settings`

---

## 4. 验收标准（Definition of Done）

- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 全部通过
- [x] `bunx --bun @biomejs/biome check --write` 无错误
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml` 已执行
- [ ] 应用启动后 `{app_data_dir}/logs/unibot.YYYY-MM-DD.log` 正确生成
- [ ] Bot 启停、用户注册、群创建能在日志文件中看到对应 JSON 行
- [ ] Logs 页面"全部"模式下能同时看到协议日志和系统日志，按时间倒序排列
- [ ] `packet` / `system` 单源模式下"加载更多"翻页正常工作
- [ ] 切换"日志来源"、"时间范围"、"等级"筛选均正常工作
- [ ] Settings 页面 DEBUG 开关实时生效（无需重启应用）
- [ ] 保留期改变后，下次清理任务按新配置执行
- [x] 代码库中无残留 `eprintln!`

---

## 5. 风险与注意事项

1. **`tracing_subscriber::registry()...init()` 只能调用一次**
   - Tauri `setup` 只跑一次，安全。#[sqlx::test] 不走 setup，不冲突。

2. **`WorkerGuard` 生命周期**
   - 必须被 `app.manage` 成 State 持有。不要放在局部变量里。`WorkerGuard` 本身不实现 `Clone`，直接 struct 包装即可。

3. **`reload::Handle` 线程安全**
   - `reload::Handle` 是 `Clone + Send + Sync`，可以 `#[derive(Clone)]` 包装后 manage。

4. **日志文件解析容错**
   - `SystemLogReader` 遇到损坏行应跳过，不要 panic。返回能解析的行即可。

5. **时间窗口查询性能**
   - 系统日志需要按 `since/until` 推算日期、打开对应文件。7 天窗口最多打开 7 个文件，可控。单文件逐行解析在 5s 轮询下可接受。

6. **清理 task 与测试**
   - 清理 task 在 `setup` 中 spawn，#[sqlx::test] 不会走 Tauri setup，因此不影响测试。清理函数本身可单独写单元测试。

7. **`list_system_logs` 的日期推算**
   - 用 `chrono::DateTime::from_timestamp_millis(since).unwrap().date_naive()` 提取日期边界，`NaiveDate::succ_opt()` 迭代到 `end_date`。

8. **`is_error` bool→i32 转换**
   - `ProtocolPacketRecord.is_error` 是 `i32`（sqlx FromRow 映射 SQLite INTEGER）。repo 层查询绑定需 `if is_error { 1 } else { 0 }`。
