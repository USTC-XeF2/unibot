# Logs 系统完整设计（v4）

> 本 spec 覆盖 Logs 页面作为统一运行日志入口的完整实现，包括协议报文（`protocol_packets`）和系统日志（JSON Lines 文件）。群优化从本期移除，延后实施。
>
> **v4 修订记录**（基于代码现状核对，修正 v3 的设计硬伤）：
> 1. **DEBUG 热切换**：`tracing` 全局 subscriber 只能 `set_global_default` 一次，无法"重新初始化"。改用 `tracing_subscriber::reload::Handle` 运行时热切换 level，Handle 作为 Tauri State 管理。
> 2. **自定义 JSON Layer**：内置 `json` formatter 给不出 1.1 的精确 schema（它产出 `timestamp` RFC3339 字符串 + `fields.message`）。改为自定义 `Layer` 输出 `{ts, level, target, msg, fields}`，明确列为实现任务。
> 3. **app_settings 是从零搭的一层**：表存在但没有任何通用读写设施（repo/command/前端 hook 全无）。本 spec 显式包含 `SettingsRepo` + 通用 get/set command + 前端 query/mutation。
> 4. **全部模式按时间窗口查**：放弃"两源各取 limit 条再合并"（会导致时间窗口错位、无法分页）。改为统一 `since`/`until` 时间窗口查询，一期不做跨源精确分页。
> 5. `WorkerGuard` 保活、appender 文件名对齐清理逻辑、补 `clear_logs` command、eventType 筛选语义、`LogEntry.time` 改毫秒数字等若干修正。

---

## 0. 设计原则

1. **Logs 是统一日志入口**：协议日志（packet）和系统日志（runtime）在同一个页面展示，按时间合并。
2. **协议日志走数据库索引 + 文件系统**：沿用 `protocol_packets` 已有设计。
3. **系统日志走文件系统**：JSON Lines 格式，不进入数据库，避免写入热点。
4. **DEBUG 级别受控且热切换**：默认 INFO，设置页面开关 DEBUG，运行时生效无需重启。
5. **UI 一致性**：沿用 shadcn/radix + lucide-react 风格。
6. **顺带统一错误输出**：现有 10 处 `eprintln!` 迁移到 `tracing`，避免两套日志并存（见 1.7）。

---

## 第一部分：系统日志基础设施

### 1.1 存储格式

`tracing-appender` 的 `RollingFileAppender` 按天轮转，文件名是 `{prefix}.{date}` 形式（日期是**后缀**，不是 `2026-06-10.log`）。用 `Builder::filename_prefix("unibot").filename_suffix("log")` 得到：

```
{app_data_dir}/
├── logs/
│   ├── unibot.2026-06-10.log   ← 当天日志
│   ├── unibot.2026-06-09.log
│   └── ...
├── packets/YYYY-MM-DD/         ← 已有
└── ...
```

> 清理逻辑（1.2）必须按 appender 的实际命名 `unibot.YYYY-MM-DD.log` 匹配，不是 `*.log` 后随便 glob。

每行一个 JSON（由**自定义 Layer** 产出，见 1.4）：

```json
{"ts":1718035200000,"level":"INFO","target":"unibot::services::message","msg":"message sent","fields":{"user_id":"10001","group_id":"20001"}}
```

字段：
- `ts`：毫秒时间戳（自定义 Layer 用 `now_ts()` 或等价，**数字**，非 RFC3339 字符串）
- `level`：`ERROR` / `WARN` / `INFO` / `DEBUG`
- `target`：Rust 模块路径
- `msg`：日志消息（event 的 message 字段，提到顶层）
- `fields`：其余结构化字段（key-value，不含 message）

### 1.2 日志保留策略

- **保留期**：默认 7 天，由 `app_settings` 的 `log.retention_days` 控制
- **清理时机**：应用启动后延迟 5 分钟执行一次；之后每 24 小时执行一次（tokio task + interval）
- **清理方式**：扫描 `logs/` 下匹配 `unibot.YYYY-MM-DD.log` 的文件，按文件名日期解析，删除超过 retention_days 的
- **不叠加 appender 的 `max_log_files`**：清理只走应用层这一套，避免两套删除逻辑互相干扰

### 1.3 依赖

`src-tauri/Cargo.toml` 新增（`chrono` 已存在，复用）：

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["registry", "env-filter"] }
tracing-appender = "0.2"
```

> 注意：**不需要** `json` feature——自定义 Layer 自己序列化，内置 json formatter 的 schema 对不上 1.1。`registry` 用于挂载多 Layer，`env-filter`/`reload` 用于热切换 level。

### 1.4 日志初始化（含自定义 JSON Layer + 热切换）

在 `src-tauri/src/lib.rs` 的 `setup` 中（在 `app.manage` 之前）：

1. 从 `app_settings` 读取 `log.level`（缺省 `"info"`）和 `log.retention_days`（缺省 `"7"`）
2. 构造 `tracing-appender` 的 `RollingFileAppender`（daily，prefix=`unibot`，suffix=`log`，dir=`{app_data_dir}/logs`）
3. `tracing_appender::non_blocking(appender)` → 得到 `(non_blocking_writer, WorkerGuard)`
   - **`WorkerGuard` 必须保活**：一旦 drop，后台写线程停止、缓冲日志丢失。将其 `app.manage(LogGuard(guard))` 成 State 持有整个进程生命周期
4. 构造**自定义 JSON Layer**（见下）写入 `non_blocking_writer`
5. 用 `reload::Layer::new(level_filter)` 包一层 filter，得到 `reload_handle`
6. `tracing_subscriber::registry().with(filter_layer).with(json_layer).init()`（`init` 内部即 `set_global_default`，全进程只调一次）
7. `app.manage(LogReloadHandle(reload_handle))` 成 State，供 `set_log_level` 运行时调用 `handle.modify(|f| *f = new_level)` 热切换
8. 启动清理 tokio task

**自定义 JSON Layer**（新文件 `src-tauri/src/logging.rs`）：
- 实现 `tracing_subscriber::Layer` 的 `on_event`
- 用一个实现了 `tracing::field::Visit` 的 visitor 收集 event 字段：`message` 字段单独提取为 `msg`，其余进 `fields` map
- 组装 `serde_json::json!({ "ts": now_ts(), "level": ..., "target": metadata.target(), "msg": ..., "fields": ... })`，`writeln!` 到 writer
- 这是本期一项明确的实现任务，不是"开 feature 即得"

### 1.5 写日志的位置（一期）

`tracing::info!` / `tracing::error!` 是**全局宏，不依赖 AppHandle/State**，可直接在 service 层任意位置调用——这正是它优于"emit event 到前端"的地方。

| 事件 | 等级 | target | fields | 插入位置 |
|------|------|--------|--------|---------|
| Bot 启动 | INFO | `unibot::bot` | `bot_id`, `bound_user_id` | `services/bot.rs` `start_bot` |
| Bot 停止 | INFO | `unibot::bot` | `bot_id`, `session_id` | `services/bot.rs` `stop_bot` |
| Bot 创建 | INFO | `unibot::bot` | `bot_id`, `bound_user_id` | `services/bot.rs` `create_bot` |
| Bot 删除 | INFO | `unibot::bot` | `bot_id` | `services/bot.rs` `delete_bot` |
| 群创建 | INFO | `unibot::group` | `group_id`, `owner_user_id` | `services/group/basic.rs` `upsert_group` |
| 群解散 | INFO | `unibot::group` | `group_id` | `services/group/management.rs` `dissolve_group` |
| 用户注册 | INFO | `unibot::user` | `user_id` | `services/user.rs` `register_user` |
| 用户删除 | INFO | `unibot::user` | `user_id` | `services/user.rs` `delete_user` |
| 消息发送失败 | ERROR | `unibot::message` | `user_id`, `error` | `services/message.rs` send 错误分支 |
| 协议适配器错误 | ERROR | `unibot::protocol` | `bot_id`, `error` | `protocol/server.rs` `is_error` 判定处 |

> 移除了 v3 的"文件上传失败"——群文件功能已延后，本期无此路径。

### 1.6 设置项

`app_settings` 新增（通过 1.4 的种子或首次读取时写入默认值）：

| key | 默认值 | value_type | 说明 |
|-----|--------|-----------|------|
| `log.level` | `"info"` | `string` | `"debug"` 或 `"info"` |
| `log.retention_days` | `"7"` | `int` | 保留天数（存为字符串，读取时 parse） |

> `app_settings.setting_value` 列始终是 TEXT，`value_type` 仅作语义标注，代码读取时自行 parse（现状如此，见 [migrator.rs](src-tauri/src/persistence/migrator.rs) 都按 String 读）。

### 1.7 app_settings 通用读写设施（新建，非"可能需要"）

调研确认：`app_settings` 表存在（[0001_initial_schema.sql:483](src-tauri/src/persistence/migrations/0001_initial_schema.sql#L483)），但**没有任何通用读写抽象**——仅 migrator 和 `get_db_status` 用裸 SQL 读 `schema.version`，前端零 settings hook。本期需新建：

- **后端 `SettingsRepo`**（`src-tauri/src/persistence/repo/settings.rs`，新文件）：
  - `get_setting(key) -> Option<String>`
  - `set_setting(key, value, value_type) -> ()`（UPSERT，更新 `updated_at`）
  - 接入 `ServiceHub` 或直接由 command 持有 pool 调用
- **前端**：`src/lib/query/settings.ts`（新文件）、`src/types/settings.ts`（新文件）

### 1.8 现有 eprintln! 迁移

现有 10 处 `eprintln!`（`utils.rs`、`lib.rs`、`protocol/runtime.rs`×3、`protocol/recorder.rs`、`services/bot.rs`、`protocol/server.rs`×2）改为对应 `tracing::warn!`/`error!`。这样所有运行时日志统一进系统日志文件，Logs 页面才能真正"统一"。

---

## 第二部分：Logs 页面设计

### 2.1 数据源切换

Logs 页面顶部增加"日志来源"筛选（新增一个 `Select`，与现有时间范围 Select 同风格）：

- **全部**：`protocol_packets` + `logs/*.log`，按统一时间窗口查询后合并（见 2.3）
- **协议日志**：只读 `protocol_packets`
- **系统日志**：只读 `logs/*.log`

### 2.2 统一视图模型

`logs.tsx` 现有的 `LogEntry`/`LogLevel`/`EventType` 类型需改写（现有 `time: string | null` 改为毫秒 `number`；现有 `EventType` 枚举"消息/请求/系统/群组/连接"与新体系不兼容，直接替换）。类型移到新文件 `src/types/log.ts`：

| 字段 | 语义 |
|------|------|
| `id` | `packet:{packet_id}` 或 `system:{ts}:{seq}`（seq 为同一文件内行号，保证唯一） |
| `time` | 毫秒时间戳（`number`，与前端惯例一致，见 [time-format.ts](src/lib/time-format.ts)） |
| `level` | `info` / `warn` / `error` / `debug` |
| `eventType` | 点分多级：`packet.send` / `packet.receive` / `system.bot` / `system.group` / `system.user` / `system.protocol` 等 |
| `source` | packet 源：`bot_id ?? profile_id ?? "system"`；system 源：`target` |
| `message` | 摘要（packet 用 `action_name`，system 用 `msg`） |
| `dataSource` | `"packet"` / `"system"` |
| `detailRef` | packet：`packet_id`（懒加载原始 JSON）；system：整行 JSON 字符串（直接展示 `fields`） |

**level 映射**：packet 无 level 字段（[packet.ts](src/types/packet.ts) 只有 `is_error`），映射 `is_error ? "error" : "info"`；system 直接用日志行的 `level` 小写。

**eventType 筛选语义**：筛选控件按**顶级前缀**分组匹配（选 `system` 命中所有 `system.*`，选 `packet` 命中所有 `packet.*`），不做精确多级匹配。一期筛选项仅 `packet` / `system` 两个粗粒度选项 + level 筛选，避免列出一堆永远匹配不到的细分。

### 2.3 系统日志读取与合并

- **Command**: `list_system_logs`（`src-tauri/src/commands/main.rs`）
  - 参数：`{ since?: u64, until?: u64, level?: String, limit: u32 }`
  - 实现：根据 `[since, until]` 推算覆盖的日期 → 只打开命中的 `unibot.YYYY-MM-DD.log` 文件 → 逐行解析 JSON → 按 `ts` 范围和 `level` 过滤 → 按 `ts` 倒序 → 截断 `limit`
  - 返回：`Vec<SystemLogEntry>`（Rust 结构体，字段对齐 1.1 的 JSON）

- **全部模式的合并策略（修正 v3 的分页缺陷）**：
  - 两个源**都按同一 `since`/`until` 时间窗口查**，而非各取 `limit` 条
  - 时间窗口由前端档位（15m/1h/24h/7d）换算成 `since = now - window`、`until = now`
  - 各源在窗口内仍各有 `limit` 上限（防御性截断，避免极端情况下内存爆掉），但正常密度下时间窗口足够小、不会触顶
  - 合并：前端 concat 两源结果，按 `time` 倒序排序
  - **一期明确放弃跨源精确分页 / 无限滚动**——时间窗口模型下"加载更多"等于拉宽窗口，本期不做

- **前端 hook**（`src/lib/query/logs.ts`，新文件）：
  - `useSystemLogsQuery(filters)` → `invoke("list_system_logs", ...)`
  - `useLogsQuery(dataSource, filters)`：根据 `dataSource` 决定调用 `useProtocolPackets` / `useSystemLogsQuery` / 两者，并在 `useMemo` 里 adapter 转 `LogEntry` + 合并排序
  - **轮询**：系统日志读取比 packet 重（解析整日文件），轮询间隔设 **5s**（不沿用 packets 的 2s）；loading 合并取"任一源首次 pending"

### 2.4 设置页面扩展

[settings.tsx](src/views/main/settings.tsx) 现状是单个只读"数据库状态"卡片。新增"日志设置"卡片：

- **日志等级**：Switch "启用 DEBUG 日志"（默认关）→ `set_log_level(level)` command → 后端 `reload_handle.modify` 热切换 + 写 `app_settings`
- **日志保留期**：下拉 1/7/30 天 → 写 `app_settings` 的 `log.retention_days`（影响下次清理 task，不影响 appender）
- **立即清理**：Button → `clear_logs` command（手动触发一次清理逻辑）

### 2.5 新增 command 清单

| command | 文件 | 作用 |
|---------|------|------|
| `list_system_logs` | `commands/main.rs` | 读系统日志文件 |
| `get_log_settings` | `commands/main.rs` | 读 `log.level` / `log.retention_days` |
| `set_log_level` | `commands/main.rs` | 热切换 + 持久化 level |
| `set_log_retention` | `commands/main.rs` | 持久化 retention_days |
| `clear_logs` | `commands/main.rs` | 手动触发清理 |

> 全部需在 `lib.rs` 的 `tauri::generate_handler!` 列表追加（[lib.rs:129-184](src-tauri/src/lib.rs#L129)）。

---

## 第三部分：不在本期范围

- 群文件上传/下载、群相册、群管理 UI、群分类/置顶/免打扰
- OneBot 协议适配
- 审计日志系统（`audit_events`）
- 数据导出/备份
- 跨数据源的精确分页 / 无限滚动
- 日志全文搜索

---

## 第四部分：相关文件变更

### 后端
- `src-tauri/Cargo.toml` — 加 `tracing` / `tracing-subscriber`(registry, env-filter, reload) / `tracing-appender`
- `src-tauri/src/logging.rs` — **新建**：自定义 JSON Layer + visitor + `LogGuard`/`LogReloadHandle` State 包装
- `src-tauri/src/lib.rs` — 初始化 tracing（reload handle + 自定义 layer）、manage `WorkerGuard` 和 reload handle、启动清理 task、追加 5 个 command 到 handler 列表
- `src-tauri/src/utils.rs` — 加日志清理函数（按 `unibot.YYYY-MM-DD.log` 解析日期）；现有 2 处 `eprintln!` → `tracing`
- `src-tauri/src/persistence/repo/settings.rs` — **新建**：`SettingsRepo` get/set
- `src-tauri/src/commands/main.rs` — 加 `list_system_logs` / `get_log_settings` / `set_log_level` / `set_log_retention` / `clear_logs`
- `src-tauri/src/services/bot.rs`、`services/user.rs`、`services/group/basic.rs`、`services/group/management.rs`、`services/message.rs` — 插 `tracing` 调用
- `src-tauri/src/protocol/{runtime,recorder,server}.rs`、`src-tauri/src/lib.rs` — 现有 `eprintln!` → `tracing`

### 前端
- `src/types/log.ts` — **新建**：`LogEntry` / `LogLevel` / `SystemLogEntry` / `LogDataSource`
- `src/types/settings.ts` — **新建**：日志设置相关 type
- `src/lib/query/logs.ts` — **新建**：`useSystemLogsQuery` / `useLogsQuery`（合并逻辑 + adapter）
- `src/lib/query/settings.ts` — **新建**：`useLogSettingsQuery` + set mutation
- `src/views/main/logs.tsx` — 重写：类型移到 log.ts、加"日志来源"切换、接入真实数据、时间筛选传后端
- `src/views/main/settings.tsx` — 加"日志设置"卡片

---

## 第五部分：实现顺序建议

1. **app_settings 设施**（1.7）：SettingsRepo + 前端 query/mutation——后续 DEBUG 开关依赖它
2. **tracing 基础设施**（1.3/1.4）：依赖 + 自定义 Layer + reload + WorkerGuard 保活——先让日志能落盘
3. **埋点 + eprintln 迁移**（1.5/1.8）：在各 service 插 tracing
4. **清理 task**（1.2）
5. **后端读取 command**（2.3/2.5）：`list_system_logs` 等
6. **前端 Logs 页面**（2.1/2.2/2.3）：数据源切换 + 合并
7. **设置页面**（2.4）：DEBUG 开关 + 保留期 + 立即清理

> 顺序 1-2 是其余一切的前置；4 可与 5-7 并行。
