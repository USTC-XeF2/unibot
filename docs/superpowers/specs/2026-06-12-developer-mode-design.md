# UniBot 开发者模式设计

## 背景

当前 Settings 页面有一个 "DEBUG 模式" 开关，仅用于把日志级别切换到 debug。随着项目复杂度增加（多窗口、事件总线、SQLite 状态），core contributor 在排查问题时需要同时查看日志、实时事件流、数据库状态和执行 SQL。本设计把 "DEBUG 模式" 升级为 "开发者模式"，并新增一个独立的"开发者工具"窗口，集中提供这些调试能力。

## 目标用户

UniBot 核心贡献者（core contributors），即直接修改 UniBot 前端、后端、数据库的人。

## 设计范围

### 包含在第一个 PR（MVP）

1. Settings 页面 "DEBUG 模式" 重命名为 "开发者模式"。
2. 开启开发者模式后显示"打开开发者工具"按钮。
3. 新增独立 Tauri 窗口 `developer-tools`（默认 1200x800）。
4. 开发者工具窗口包含四个标签页：
   - **日志**：复用 `list_system_logs`，支持关键字搜索和按 level/target 过滤。
   - **事件流**：实时显示后端 `chat:event` 事件，含时间戳、目标窗口、事件 kind、JSON payload；支持暂停/继续、清空、按 kind 过滤。
   - **数据库**：展示 SQLite 表结构（表名、列、索引），点击表名可预览前 N 行。
   - **SQL**：SQL 执行器，默认只读；执行写操作需要 UI 二次确认 + 后端 `allow_write` 参数双重校验。

### 不包含（后续可扩展）

- React Query 缓存状态查看器。
- 性能分析（FPS、内存、Tauri 命令耗时）。
- 协议包导出/重放。

## 架构

### 后端

新增 `src-tauri/src/commands/dev_tools.rs`：

| 命令 | 说明 |
|------|------|
| `get_db_schema()` | 读取 `sqlite_master`、`PRAGMA table_info(...)`、`PRAGMA index_list(...)`，返回表、列、索引结构。 |
| `execute_sql(query: String, allow_write: bool)` | 执行 SQL。若语句含写操作关键字且 `allow_write=false`，返回 `AppError::Validation`。返回行数组（每行是列名到值的 map）。 |
| `open_developer_tools(app)` | 后端命令，创建或聚焦 `developer-tools` 窗口（与 `open_user_chat_window` 模式一致）。 |

事件流：
- 后端 `core.rs` 在把事件 `emit_to` 给 `chat-{user_id}` 窗口时，若 `developer-tools` 窗口存在，额外 `emit_to("developer-tools", "chat:event", event)` 一份。
- 开发者工具窗口前端全局监听 `chat:event`（不指定 target），接收所有用户的事件。

### 前端

- 新路由 `/devtools`，入口组件 `src/views/dev-tools/dev-tools-window.tsx`。
- 使用 shadcn Tabs 组织四个面板：
  - `src/views/dev-tools/logs-panel.tsx`
  - `src/views/dev-tools/events-panel.tsx`
  - `src/views/dev-tools/schema-panel.tsx`
  - `src/views/dev-tools/sql-panel.tsx`
- Settings 页面修改：
  - "DEBUG 模式" 改为 "开发者模式"。
  - 开启后显示 "打开开发者工具" 按钮，调用后端命令 `open_developer_tools` 打开窗口。

### 权限

新增 `src-tauri/capabilities/devtools.json`：
- 仅对 `developer-tools` 窗口生效。
- 允许调用 dev tools 相关命令。
- 限制 `execute_sql` 等敏感命令只能在该窗口调用。

## 数据流

### SQL 执行器

1. 用户在 SQL 面板输入 SQL，点击执行。
2. 前端默认以 `allow_write=false` 调用 `execute_sql`。
3. 后端解析 SQL，若含写操作关键字且 `allow_write=false`，返回验证错误。
4. 若用户开启"允许写操作"开关并确认对话框，前端以 `allow_write=true` 重新调用。
5. 后端执行 SQL，返回行数组（`SELECT`/`PRAGMA`）或空数组（写操作/`EXPLAIN` 等），前端用表格展示。

### 事件流

1. 开发者工具窗口 mount 时调用 `listen("chat:event", callback)` 全局监听。
2. 后端 `core.rs` 转发事件时同步 emit 到 `developer-tools` 窗口。
3. 前端将事件追加到列表；每条事件显示：时间戳、目标用户/窗口、source（private/group）、发送者、事件 kind、折叠的 JSON payload。支持暂停/继续、清空、按 kind 过滤。若处于暂停状态则缓存，恢复后继续追加。

### 日志

1. 前端调用 `list_system_logs({ limit: 500 })` 获取最近日志。
2. 用户输入关键字或选择 level/target 过滤，前端在内存中过滤。
3. 后续可扩展为后端分页（参考已有 logs 设计 spec）。

## 安全

- **Capability 隔离**：dev tools 命令只对 `developer-tools` 窗口暴露。
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
- 手动验证：事件流标签页能实时显示聊天窗口触发的事件。

### 安全

- 手动验证：dev tools 命令在非 `developer-tools` 窗口调用会被 Tauri capability 拒绝。

## 影响范围

- `src/views/main/settings.tsx`：重命名开关，增加打开按钮。
- `src-tauri/src/commands/dev_tools.rs`：新增命令文件。
- `src-tauri/src/lib.rs`：注册新命令到 invoke handler。
- `src-tauri/src/core.rs`：事件转发逻辑增加 dev tools 窗口投递。
- `src-tauri/capabilities/devtools.json`：新增 capability。
- `src/views/dev-tools/**`：新增开发者工具窗口和面板。
- `src/lib/query/dev-tools.ts`：新增前端 query hooks。
- `src/App.tsx`：新增 `/devtools` 路由。

## 开放问题

- 是否需要为开发者工具窗口提供深色/浅色主题切换？（复用应用当前主题即可。）
- 事件流是否需要显示事件原始来源用户 ID？（推荐显示，便于排查多窗口问题。）
- SQL 执行器返回的 BLOB 类型如何展示？（MVP 中可显示为 `<BLOB>` 占位符。）
