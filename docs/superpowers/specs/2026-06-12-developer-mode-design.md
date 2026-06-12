# UniBot 开发者模式设计

## 背景

当前 Settings 页面有一个 "DEBUG 模式" 开关，仅用于把日志级别切换到 debug。随着项目复杂度增加（多窗口、事件总线、SQLite 状态），core contributor 在排查问题时需要同时查看日志、实时事件流、数据库状态和执行 SQL。本设计把 "DEBUG 模式" 升级为 "开发者模式"，并新增一个独立的"开发者工具"窗口，集中提供这些调试能力。

## 目标用户

UniBot 核心贡献者（core contributors），即直接修改 UniBot 前端、后端、数据库的人。

## 设计范围

> 实现时拆为两个任务（见 writing plans）：
>
> - **PR1（基础）**：开发者模式开关 + 窗口框架 + 日志/事件流/数据库三个只读面板。
> - **PR2（SQL 执行器）**：SQL 面板，因其安全面最大、动态返回类型最复杂，单独交付。

整体范围如下：

1. Settings 页面 "DEBUG 模式" 重命名为 "开发者模式"。
2. 开启开发者模式后显示"打开开发者工具"按钮。
3. 新增独立 Tauri 窗口 `developer-tools`（默认 1200x800）。
4. 开发者工具窗口包含四个标签页：
   - **日志**（PR1）：复用 `list_system_logs`，支持关键字搜索和按 level/target 过滤。
   - **事件流**（PR1）：实时显示后端广播的 `InternalEvent`（经 `devtools:event` 转发），含接收用户 ID、事件 `kind`、折叠的 JSON payload；支持暂停/继续、清空、按 kind 过滤。
   - **数据库**（PR1）：展示 SQLite 表结构（表名、列、索引），点击表名可预览前 N 行。
   - **SQL**（PR2）：SQL 执行器，默认只读；执行写操作需要 UI 二次确认 + 后端 `allow_write` 参数双重校验。

### 不包含（后续可扩展）

- React Query 缓存状态查看器。
- 性能分析（FPS、内存、Tauri 命令耗时）。
- 协议包导出/重放。

## 架构

### 后端

新增 `src-tauri/src/commands/dev_tools.rs`。这些命令直接注入 `tauri::State<'_, sqlx::SqlitePool>`（`pool` 已在 `lib.rs` 通过 `app.manage(pool.clone())` 注册），不走 repo 层：

| 命令 | 说明 |
|------|------|
| `get_db_schema(pool)` | 读取 `sqlite_master`、`PRAGMA table_info(...)`、`PRAGMA index_list(...)`，返回表、列、索引结构。 |
| `execute_sql(pool, query: String, allow_write: bool)` | 执行 SQL。若语句含写操作关键字且 `allow_write=false`，返回 `AppError::Validation`。返回值见下方"返回类型"。 |
| `open_developer_tools(app, core)` | 注入 `tauri::AppHandle` 与 `tauri::State<CoreContainer>`。创建或聚焦 `developer-tools` 窗口（参考 `open_user_chat_window` 的 show/unminimize/set_focus 模式），并起事件转发循环（见下"事件流"）。 |

**`execute_sql` 返回类型**：SQLite 列值类型不定（INTEGER/TEXT/REAL/BLOB/NULL），用 `sqlx::query(...)` 取动态行后按列遍历 `sqlx::Row`。返回结构为 `{ columns: Vec<String>, rows: Vec<Vec<serde_json::Value>> }`：

- INTEGER/REAL → JSON number，TEXT → string，NULL → `null`。
- BLOB → 字符串占位符 `"<BLOB>"`（MVP 不解析内容）。
- 写操作返回空 `rows` 并附 `rows_affected`；`EXPLAIN`/`PRAGMA` 等有结果集的语句照常返回 `{ columns, rows }`。

事件流（实时，覆盖运行期新增用户）：

事件发布的唯一 choke point 是 `utils.rs` 的 `emit_to_users`（`emit_to_group_members` 内部也调它，服务层无任何地方绕过它直接 `event_tx.send`）。据此采用集中 firehose 方案，而非 per-user 转发循环：

- `CoreContainer` 新增一个独立的 `devtools_tx: broadcast::Sender<DevToolsEvent>`，在 `new()` 中用 `broadcast::channel` 初始化。`DevToolsEvent` 形如 `{ recipient_user_id: String, event: InternalEvent }`。
- `emit_to_users` 在向每个 user 的 `event_tx.send` 的同时，额外 `core.devtools_tx.send(DevToolsEvent { recipient_user_id, event })`。它已持有 `&CoreContainer`，无需改签名、无需 `AppHandle`，**所有调用点零改动**。新注册用户的事件同样流经 `emit_to_users`，因此自动被覆盖。
- `open_developer_tools` 创建窗口后，先 `core.devtools_tx.subscribe()` 拿到 owned `Receiver`、`app.clone()` 拿到 owned `AppHandle`，再 `tauri::async_runtime::spawn` 一个**单个**转发循环 move 进它们（`tauri::State` 是借用，不能直接进 `'static` 闭包——同 `open_user_chat_window` 的做法）。循环把 `DevToolsEvent` 经 `emit_to("developer-tools", "devtools:event", ...)` 投递，以 dev-tools 窗口是否存在为存活条件（窗口关闭则 `break`）。
- **每个收件人一条，不去重（有意为之）**：`emit_to_users` 对每个 recipient 各 `event_tx.send` 一次，因此一条群消息发给 N 个成员时，dev-tools 也会收到 N 条（各带不同 `recipient_user_id`）。这正是"观察每个 user 实际收到什么"的预期，前端不做去重。
- **firehose channel 容量与滞后**：`devtools_tx` 用有界 `broadcast::channel(N)`（N 取较大值，如 1024）。转发循环对 `RecvError::Lagged` 直接 `continue`（参考 `core.rs` 现有 chat 循环的处理），丢弃溢出事件而非阻塞业务；dev-tools 窗口未打开时无订阅者，`send` 失败被忽略，不影响主流程。
- **接收用户 ID 由 `DevToolsEvent` 显式携带**：`InternalEvent`（见 `models/internal.rs`）是 `#[serde(tag = "kind")]` 的扁平枚举，payload 本身不含"这条发给哪个用户"的信息——那是 `emit_to_users` 的循环上下文，故包装在 `DevToolsEvent.recipient_user_id` 里。
- **没有统一的"发送者"字段**：各变体的用户字段名不同（`Message.sender_user_id`、`Poke.sender_user_id`/`target_user_id`、`Notice.actor`/`target`、`GroupMemberMuted.operator_user_id` 等），部分群事件无单一发送者。前端只展示 `kind` + 完整 JSON payload，不试图抽取统一发送者字段。
- **使用独立事件名 `devtools:event`，不复用 `chat:event`**：chat 窗口的 `use-chat-event-bus` 期望裸 `InternalEvent` payload，dev-tools 转发的是包装后的 `DevToolsEvent`，形状不同，故隔离事件名避免误用。

### 前端

- 新路由 `/developer-tools`（与窗口 label `developer-tools` 保持一致），入口组件 `src/views/dev-tools/dev-tools-window.tsx`。窗口 URL 为 `index.html#/developer-tools`。
- 使用 shadcn Tabs 组织四个面板：
  - `src/views/dev-tools/logs-panel.tsx`
  - `src/views/dev-tools/events-panel.tsx`
  - `src/views/dev-tools/schema-panel.tsx`
  - `src/views/dev-tools/sql-panel.tsx`
- Settings 页面修改：
  - "DEBUG 模式" 改为 "开发者模式"。
  - 开启后显示 "打开开发者工具" 按钮，调用后端命令 `open_developer_tools` 打开窗口。

### 权限

新增 `src-tauri/capabilities/devtools.json`（结构参考 `chat.json`）：

- `"windows": ["developer-tools"]`，仅对该窗口生效。
- `"permissions"` 包含 `core:default` 与 dev tools 命令所需权限。
- **命令的窗口隔离靠 capability 的命令声明实现**：Tauri 按窗口授权命令集，没有"命令级窗口白名单"。`execute_sql`/`get_db_schema` 等只在 `devtools.json` 声明，**不**出现在 `default.json`/`chat.json`，从而 main/chat 窗口无法调用。

## 数据流

### SQL 执行器

1. 用户在 SQL 面板输入 SQL，点击执行。
2. 前端默认以 `allow_write=false` 调用 `execute_sql`。
3. 后端解析 SQL，若含写操作关键字且 `allow_write=false`，返回验证错误。
4. 若用户开启"允许写操作"开关并确认对话框，前端以 `allow_write=true` 重新调用。
5. 后端执行 SQL，返回 `{ columns, rows }`（`SELECT`/`PRAGMA`）或空 `rows`（写操作/`EXPLAIN` 等，附 `rows_affected`），前端用表格展示。

### 事件流

1. 开发者工具窗口 mount 时调用 `listen("devtools:event", callback)` 监听本窗口收到的事件。
2. 服务层任意事件经 `emit_to_users` 发布时，同步写入 `CoreContainer.devtools_tx` firehose；`open_developer_tools` 起的单个转发循环 `subscribe()` 该 firehose，把 `DevToolsEvent` `emit_to("developer-tools", "devtools:event", ...)`；循环以 dev-tools 窗口存在为存活条件。
3. 前端将事件追加到列表；每条事件显示：接收时间戳（前端本地）、`recipient_user_id`、事件 `kind`、折叠的完整 JSON payload。支持暂停/继续、清空、按 kind 过滤。若处于暂停状态则缓存，恢复后继续追加。

### 日志

1. 前端调用 `list_system_logs({ limit: 500 })` 获取最近日志。
2. 用户输入关键字或选择 level/target 过滤，前端在内存中过滤。
3. 后续可扩展为后端分页（参考已有 logs 设计 spec）。

## 安全

- **Capability 隔离**：dev tools 命令只在 `devtools.json` 声明、不出现在 `default.json`/`chat.json`，因此只有 `developer-tools` 窗口能调用（Tauri 按窗口授权命令集）。
- **写操作双重校验**：UI 二次确认 + 后端 `allow_write` 参数校验。
- **本地使用**：所有数据都在本地，不存在网络暴露风险。
- **关键字检测而非完整 SQL 解析**：足够用于本地调试场景，不追求阻止所有恶意输入（用户本身就是 core contributor）。存在极小误报可能（如 SELECT 语句的字符串字面量中包含 INSERT），可接受。

## 测试计划

### 后端

- 单元测试：SQL 写操作关键字检测覆盖 INSERT/UPDATE/DELETE/REPLACE/DROP/CREATE/ALTER/TRUNCATE 与 SELECT/PRAGMA/EXPLAIN。
- `#[sqlx::test]` 集成测试：`get_db_schema` 返回当前迁移后的表结构。

### 前端

- 手动验证：开启开发者模式后 Settings 出现"打开开发者工具"按钮，点击打开独立窗口。
- 手动验证：SQL 执行器默认拒绝写操作，开启写模式并确认后才能执行。
- 手动验证：事件流标签页能实时显示任意用户触发的事件；dev-tools 窗口打开后再新注册的用户，其事件也能即时出现（验证 firehose 覆盖新用户）。

### 安全

- 手动验证：dev tools 命令在非 `developer-tools` 窗口调用会被 Tauri capability 拒绝。

## 影响范围

- `src/views/main/settings.tsx`：重命名开关，增加打开按钮。
- `src-tauri/src/commands/dev_tools.rs`：新增命令文件（`get_db_schema`、`execute_sql`、`open_developer_tools`）。
- `src-tauri/src/lib.rs`：注册新命令到 invoke handler。
- `src-tauri/src/core.rs`：`CoreContainer` 新增 `devtools_tx` firehose channel；`open_developer_tools` 订阅它起单个转发循环（不改动现有 per-user chat 转发循环）。
- `src-tauri/src/utils.rs`：`emit_to_users` 在发布事件时同步写入 `devtools_tx`。
- `src-tauri/src/models/internal.rs`（或 `dev_tools.rs`）：新增 `DevToolsEvent { recipient_user_id, event }` 类型。
- `src-tauri/capabilities/devtools.json`：新增 capability。
- `src/views/dev-tools/**`：新增开发者工具窗口和面板。
- `src/lib/query/dev-tools.ts`：新增前端 query hooks。
- `src/App.tsx`：新增 `/developer-tools` 路由。

## 开放问题

- 是否需要为开发者工具窗口提供深色/浅色主题切换？（复用应用当前主题即可。）
