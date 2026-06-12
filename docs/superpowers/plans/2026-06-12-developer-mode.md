# UniBot 开发者模式实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有 "DEBUG 模式" 升级为 "开发者模式"，新增独立 `developer-tools` 窗口，提供日志、实时事件流、数据库结构浏览和 SQL 执行器四个调试面板。

**Architecture:** 后端在 `CoreContainer` 维护一个 `devtools_tx` firehose channel，所有事件经 `emit_to_users` 时同步写入；`open_developer_tools` 命令创建独立窗口并启动单个转发循环，把 firehose 中的事件经 `devtools:event` 推送到该窗口。前端独立窗口通过路由 `/developer-tools` 渲染四个 shadcn Tabs 面板。

**Tech Stack:** Tauri v2, Rust, React 19, TypeScript, shadcn/ui, sqlx, SQLite.

---

## 文件结构

### 新增文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/models/dev_tools.rs` | `DevToolsEvent` 类型定义。 |
| `src-tauri/src/commands/dev_tools.rs` | `open_developer_tools`、`get_db_schema`、`execute_sql` 三个 Tauri 命令。 |
| `src-tauri/capabilities/devtools.json` | `developer-tools` 窗口的 capability。 |
| `src/types/dev-tools.ts` | 前端 DevTools 相关类型（`DbSchema`、`DevToolsEventPayload` 等）。 |
| `src/lib/query/dev-tools.ts` | 前端 query hooks（`useDbSchemaQuery`、`useOpenDeveloperTools` 等）。 |
| `src/views/dev-tools/dev-tools-window.tsx` | 开发者工具窗口入口，含 Tabs 切换。 |
| `src/views/dev-tools/logs-panel.tsx` | 日志面板。 |
| `src/views/dev-tools/events-panel.tsx` | 实时事件流面板。 |
| `src/views/dev-tools/schema-panel.tsx` | 数据库结构浏览器面板。 |
| `src/views/dev-tools/sql-panel.tsx` | SQL 执行器面板（PR2）。 |

### 修改文件

| 文件 | 修改 |
|------|------|
| `src-tauri/src/core.rs` | `CoreContainer` 新增 `devtools_tx` firehose channel。 |
| `src-tauri/src/utils.rs` | `emit_to_users` 同步写入 `devtools_tx`。 |
| `src-tauri/src/lib.rs` | 注册 dev tools 命令、引入 `models/dev_tools`。 |
| `src-tauri/src/models/mod.rs` | 导出 `dev_tools` 模块。 |
| `src/App.tsx` | 新增 `/developer-tools` 路由。 |
| `src/views/main/settings.tsx` | "DEBUG 模式" 改 "开发者模式"，开启后显示"打开开发者工具"按钮。 |
| `src/lib/mutations.ts` | 新增 `useOpenDeveloperToolsMutation`。 |

---

## PR1：基础开发者工具窗口（日志 / 事件流 / 数据库）

### Task 1: 新增 `DevToolsEvent` 类型

**Files:**
- Create: `src-tauri/src/models/dev_tools.rs`
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: 创建 `DevToolsEvent` 类型**

```rust
use serde::{Deserialize, Serialize};

use crate::models::InternalEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevToolsEvent {
    pub recipient_user_id: String,
    pub event: InternalEvent,
}
```

- [ ] **Step 2: 在 `models/mod.rs` 中导出**

假设 `src-tauri/src/models/mod.rs` 已有 `pub mod entities; pub mod internal;`，新增：

```rust
pub mod dev_tools;
```

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/models/dev_tools.rs src-tauri/src/models/mod.rs
git commit -m "feat(dev-tools): add DevToolsEvent type"
```

---

### Task 2: CoreContainer 新增 devtools firehose

**Files:**
- Modify: `src-tauri/src/core.rs`

- [ ] **Step 1: 导入 `DevToolsEvent` 和广播 channel**

在 `src-tauri/src/core.rs` 顶部添加：

```rust
use crate::models::{DevToolsEvent, InternalEvent, UserProfile};
```

- [ ] **Step 2: 在 `CoreContainer` 中新增 `devtools_tx`**

```rust
pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 256;
pub const DEFAULT_DEV_TOOLS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct CoreContainer {
    users: Arc<RwLock<HashMap<String, UserContext>>>,
    devtools_tx: broadcast::Sender<DevToolsEvent>,
}
```

- [ ] **Step 3: 在 `new()` 中初始化 channel**

```rust
impl CoreContainer {
    pub fn new() -> Self {
        let (devtools_tx, _) = broadcast::channel(DEFAULT_DEV_TOOLS_CAPACITY);
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            devtools_tx,
        }
    }
}
```

- [ ] **Step 4: 添加订阅方法**

在 `impl CoreContainer` 中新增：

```rust
pub fn subscribe_devtools_events(&self) -> broadcast::Receiver<DevToolsEvent> {
    self.devtools_tx.subscribe()
}
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/core.rs
git commit -m "feat(dev-tools): add devtools firehose channel to CoreContainer"
```

---

### Task 3: `emit_to_users` 写入 firehose

**Files:**
- Modify: `src-tauri/src/utils.rs`

- [ ] **Step 1: 修改 `emit_to_users` 同步写入 `devtools_tx`**

```rust
pub fn emit_to_users<I, S>(core: &CoreContainer, user_ids: I, event: InternalEvent)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for user_id in user_ids {
        if let Ok(ctx) = core.require_user_context(user_id.as_ref()) {
            let _ = ctx.event_tx.send(event.clone());
        };
        let _ = core.devtools_tx.send(DevToolsEvent {
            recipient_user_id: user_id.as_ref().to_string(),
            event: event.clone(),
        });
    }
}
```

- [ ] **Step 2: 导入 `DevToolsEvent`**

```rust
use crate::models::{DevToolsEvent, InternalEvent, MessageSource};
```

- [ ] **Step 3: 运行 Rust 测试确保无编译错误**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --no-run
```

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/utils.rs
git commit -m "feat(dev-tools): forward events to devtools firehose"
```

---

### Task 4: 后端命令 `open_developer_tools`

**Files:**
- Create: `src-tauri/src/commands/dev_tools.rs`
- Modify: `src-tauri/src/commands/mod.rs`（如果存在，否则跳过）

- [ ] **Step 1: 创建命令文件**

```rust
use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::DevToolsEvent;

#[tauri::command]
pub fn open_developer_tools(
    app: tauri::AppHandle,
    core: tauri::State<CoreContainer>,
) -> AppResult<bool> {
    let label = "developer-tools";

    if let Some(existing) = app.get_webview_window(label) {
        existing.show().map_err(|e| {
            AppError::internal(format!("failed to show developer tools window: {e}"))
        })?;
        existing.unminimize().map_err(|e| {
            AppError::internal(format!("failed to unminimize developer tools window: {e}"))
        })?;
        existing.set_focus().map_err(|e| {
            AppError::internal(format!("failed to focus developer tools window: {e}"))
        })?;
        return Ok(false);
    }

    let webview_url = tauri::WebviewUrl::App(format!("index.html#/developer-tools").into());
    let window = tauri::WebviewWindowBuilder::new(&app, label, webview_url)
        .title("开发者工具")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .build()
        .map_err(|e| AppError::internal(format!("failed to create developer tools window: {e}")))?;

    let mut devtools_rx = core.subscribe_devtools_events();
    let app_handle = app.clone();
    let label_owned = label.to_string();

    tauri::async_runtime::spawn(async move {
        loop {
            if app_handle.get_webview_window(&label_owned).is_none() {
                break;
            }

            match devtools_rx.recv().await {
                Ok(DevToolsEvent {
                    recipient_user_id,
                    event,
                }) => {
                    if app_handle.get_webview_window(&label_owned).is_none() {
                        break;
                    }
                    let payload = serde_json::json!({
                        "recipient_user_id": recipient_user_id,
                        "event": event,
                    });
                    if let Err(e) = app_handle.emit_to(&label_owned, "devtools:event", payload) {
                        tracing::error!(target: "dev_tools", "emit_to developer-tools failed: {}", e);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    let app_handle_for_close = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            if let Some(w) = app_handle_for_close.get_webview_window(label) {
                let _ = w.close();
            }
        }
    });

    Ok(true)
}
```

- [ ] **Step 2: 在 `commands/mod.rs` 导出（如存在）**

若 `src-tauri/src/commands/mod.rs` 存在，新增：

```rust
pub mod dev_tools;
```

- [ ] **Step 3: 运行 `cargo check`**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands/dev_tools.rs src-tauri/src/commands/mod.rs
git commit -m "feat(dev-tools): add open_developer_tools command with event forwarding loop"
```

---

### Task 5: 后端命令 `get_db_schema`

**Files:**
- Modify: `src-tauri/src/commands/dev_tools.rs`

- [ ] **Step 1: 定义返回类型**

在 `src-tauri/src/commands/dev_tools.rs` 顶部追加：

```rust
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbColumn {
    pub cid: i64,
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbIndex {
    pub seq: i64,
    pub name: String,
    pub unique: bool,
    pub origin: String,
    pub partial: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTable {
    pub name: String,
    pub sql: Option<String>,
    pub columns: Vec<DbColumn>,
    pub indexes: Vec<DbIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSchema {
    pub tables: Vec<DbTable>,
}
```

- [ ] **Step 2: 实现 `get_db_schema` 命令**

```rust
#[tauri::command]
pub async fn get_db_schema(pool: tauri::State<'_, sqlx::SqlitePool>) -> AppResult<DbSchema> {
    let tables: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::storage(format!("failed to list tables: {e}")))?;

    let mut result = Vec::new();
    for (name, sql) in tables {
        let columns: Vec<DbColumn> = sqlx::query(
            "SELECT cid, name, type, notnull, dflt_value, pk FROM pragma_table_info(?)",
        )
        .bind(&name)
        .fetch_all(&**pool)
        .await
        .map_err(|e| AppError::storage(format!("failed to get table info for {name}: {e}")))?
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| DbColumn {
            cid: row.get::<i64, _>("cid"),
            name: row.get::<String, _>("name"),
            type_name: row.get::<String, _>("type"),
            not_null: row.get::<bool, _>("notnull"),
            default_value: row.get::<Option<String>, _>("dflt_value"),
            primary_key: row.get::<bool, _>("pk"),
        })
        .collect();

        let index_list: Vec<(i64, String, bool, String, bool)> = sqlx::query_as(
            "SELECT seq, name, \"unique\", origin, partial FROM pragma_index_list(?)",
        )
        .bind(&name)
        .fetch_all(&**pool)
        .await
        .map_err(|e| AppError::storage(format!("failed to list indexes for {name}: {e}")))?;

        let mut indexes = Vec::new();
        for (seq, idx_name, unique, origin, partial) in index_list {
            let index_columns: Vec<String> = sqlx::query_as(
                "SELECT name FROM pragma_index_info(?)",
            )
            .bind(&idx_name)
            .fetch_all(&**pool)
            .await
            .map_err(|e| AppError::storage(format!("failed to get index info for {idx_name}: {e}")))?
            .into_iter()
            .map(|(name,): (String,)| name)
            .collect();

            indexes.push(DbIndex {
                seq,
                name: idx_name,
                unique,
                origin,
                partial,
                columns: index_columns,
            });
        }

        result.push(DbTable {
            name,
            sql,
            columns,
            indexes,
        });
    }

    Ok(DbSchema { tables: result })
}
```

- [ ] **Step 3: 运行 `cargo check`**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands/dev_tools.rs
git commit -m "feat(dev-tools): add get_db_schema command"
```

---

### Task 6: 注册命令与 capability

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/capabilities/devtools.json`

- [ ] **Step 1: 在 `lib.rs` 中导入 dev_tools 命令**

在 `src-tauri/src/lib.rs` 顶部已有 `mod commands;` 的模块声明处，确保 `commands` 模块包含 `dev_tools`。若 `commands/mod.rs` 已导出 `pub mod dev_tools;`，则无需额外导入；否则在 `lib.rs` 的 `commands` 模块下添加：

```rust
mod dev_tools;
```

- [ ] **Step 2: 将命令注册到 invoke handler**

在 `tauri::generate_handler![...]` 数组末尾追加：

```rust
dev_tools::open_developer_tools,
dev_tools::get_db_schema,
```

- [ ] **Step 3: 创建 capability 文件**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "devtools",
  "description": "Permissions for the developer tools window",
  "windows": ["developer-tools"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-unminimize",
    "core:window:allow-set-focus",
    "core:window:allow-close"
  ]
}
```

- [ ] **Step 4: 运行构建以验证 capability 配置**

```bash
bunx tauri build --no-bundle
```

或仅构建前端+检查：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/lib.rs src-tauri/capabilities/devtools.json
git commit -m "feat(dev-tools): register commands and add devtools capability"
```

---

### Task 7: 前端类型与 query hooks

**Files:**
- Create: `src/types/dev-tools.ts`
- Create: `src/lib/query/dev-tools.ts`
- Modify: `src/lib/mutations.ts`

- [ ] **Step 1: 创建前端类型**

```typescript
import type { InternalEventPayload } from "@/types/event";

export type DbColumn = {
  cid: number;
  name: string;
  type_name: string;
  not_null: boolean;
  default_value: string | null;
  primary_key: boolean;
};

export type DbIndex = {
  seq: number;
  name: string;
  unique: boolean;
  origin: string;
  partial: boolean;
  columns: string[];
};

export type DbTable = {
  name: string;
  sql: string | null;
  columns: DbColumn[];
  indexes: DbIndex[];
};

export type DbSchema = {
  tables: DbTable[];
};

export type DevToolsEventPayload = {
  recipient_user_id: string;
  event: InternalEventPayload;
};

export type SqlQueryResult = {
  columns: string[];
  rows: (string | number | boolean | null)[][];
  rows_affected?: number;
};
```

- [ ] **Step 2: 创建 query hooks**

```typescript
import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import type { DbSchema, SqlQueryResult } from "@/types/dev-tools";

export function useDbSchemaQuery() {
  return useQuery({
    queryKey: queryKeys.devTools.schema(),
    queryFn: () => invoke<DbSchema>("get_db_schema"),
    retry: false,
  });
}

export function useOpenDeveloperToolsMutation() {
  return useMutation({
    mutationFn: () => invoke<boolean>("open_developer_tools"),
  });
}

export function useExecuteSqlMutation() {
  return useMutation({
    mutationFn: ({
      query,
      allowWrite,
    }: {
      query: string;
      allowWrite: boolean;
    }) =>
      invoke<SqlQueryResult>("execute_sql", {
        query,
        allowWrite,
      }),
  });
}
```

- [ ] **Step 3: 在 `src/lib/query/keys.ts` 新增 devTools key**

```typescript
export const queryKeys = {
  // ...existing keys
  devTools: {
    schema: () => ["dev-tools", "schema"] as const,
  },
} as const;
```

- [ ] **Step 4: 在 `src/lib/query/index.ts` 导出**

```typescript
export * from "@/lib/query/dev-tools";
```

- [ ] **Step 5: 提交**

```bash
git add src/types/dev-tools.ts src/lib/query/dev-tools.ts src/lib/query/keys.ts src/lib/query/index.ts
git commit -m "feat(dev-tools): add frontend types and query hooks"
```

---

### Task 8: 新增 `/developer-tools` 路由与窗口入口

**Files:**
- Modify: `src/App.tsx`
- Create: `src/views/dev-tools/dev-tools-window.tsx`

- [ ] **Step 1: 在 App.tsx 注册路由**

```typescript
import DevToolsWindow from "@/views/dev-tools/dev-tools-window";
```

在 router 数组中 `ChatWindowView` 同级新增：

```typescript
{
  path: "/developer-tools",
  element: <DevToolsWindow />,
},
```

- [ ] **Step 2: 创建窗口入口组件**

```typescript
import { useState } from "react";
import { LogsPanel } from "@/views/dev-tools/logs-panel";
import { EventsPanel } from "@/views/dev-tools/events-panel";
import { SchemaPanel } from "@/views/dev-tools/schema-panel";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export default function DevToolsWindow() {
  const [activeTab, setActiveTab] = useState("logs");

  return (
    <div className="flex h-screen w-screen flex-col bg-background p-4">
      <header className="mb-4 flex items-center justify-between">
        <h1 className="font-semibold text-lg">开发者工具</h1>
      </header>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex flex-1 flex-col">
        <TabsList className="mb-4 self-start">
          <TabsTrigger value="logs">日志</TabsTrigger>
          <TabsTrigger value="events">事件流</TabsTrigger>
          <TabsTrigger value="schema">数据库</TabsTrigger>
          <TabsTrigger value="sql" disabled>
            SQL（PR2）
          </TabsTrigger>
        </TabsList>

        <TabsContent value="logs" className="flex-1 overflow-hidden">
          <LogsPanel />
        </TabsContent>
        <TabsContent value="events" className="flex-1 overflow-hidden">
          <EventsPanel />
        </TabsContent>
        <TabsContent value="schema" className="flex-1 overflow-hidden">
          <SchemaPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
```

- [ ] **Step 3: 提交**

```bash
git add src/App.tsx src/views/dev-tools/dev-tools-window.tsx
git commit -m "feat(dev-tools): add developer-tools route and window shell"
```

---

### Task 9: 日志面板

**Files:**
- Create: `src/views/dev-tools/logs-panel.tsx`

- [ ] **Step 1: 复用现有日志视图逻辑**

将 `src/views/main/logs.tsx` 的核心渲染逻辑精简后复制到 `logs-panel.tsx`，或提取为可复用组件。MVP 最简单做法：新建面板，调用 `useSystemLogsQuery` 并展示。

```typescript
import { useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSystemLogsQuery } from "@/lib/query";

const LEVELS = ["all", "trace", "debug", "info", "warn", "error"];

export function LogsPanel() {
  const logsQuery = useSystemLogsQuery({ limit: 500 });
  const [keyword, setKeyword] = useState("");
  const [level, setLevel] = useState("all");

  const filtered = useMemo(() => {
    const entries = logsQuery.data ?? [];
    const lower = keyword.trim().toLowerCase();
    return entries.filter((entry) => {
      if (level !== "all" && entry.level !== level) return false;
      if (!lower) return true;
      const text = JSON.stringify(entry).toLowerCase();
      return text.includes(lower);
    });
  }, [logsQuery.data, keyword, level]);

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex gap-2">
        <Input
          placeholder="搜索关键字..."
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          className="flex-1"
        />
        <Select value={level} onValueChange={setLevel}>
          <SelectTrigger className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {LEVELS.map((l) => (
              <SelectItem key={l} value={l}>
                {l.toUpperCase()}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex-1 overflow-auto rounded border font-mono text-xs">
        <table className="w-full">
          <thead className="sticky top-0 bg-muted">
            <tr>
              <th className="px-2 py-1 text-left">时间</th>
              <th className="px-2 py-1 text-left">级别</th>
              <th className="px-2 py-1 text-left">Target</th>
              <th className="px-2 py-1 text-left">消息</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((entry, idx) => (
              <tr key={idx} className="border-t">
                <td className="px-2 py-1 whitespace-nowrap">{entry.ts}</td>
                <td className="px-2 py-1">{entry.level}</td>
                <td className="px-2 py-1">{entry.target}</td>
                <td className="px-2 py-1">{entry.msg}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add src/views/dev-tools/logs-panel.tsx
git commit -m "feat(dev-tools): add logs panel"
```

---

### Task 10: 事件流面板

**Files:**
- Create: `src/views/dev-tools/events-panel.tsx`

- [ ] **Step 1: 监听 `devtools:event`**

```typescript
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { DevToolsEventPayload } from "@/types/dev-tools";

type EventItem = {
  id: number;
  receivedAt: number;
  payload: DevToolsEventPayload;
};

export function EventsPanel() {
  const [events, setEvents] = useState<EventItem[]>([]);
  const [paused, setPaused] = useState(false);
  const [kindFilter, setKindFilter] = useState("");
  const nextIdRef = useRef(0);
  const backlogRef = useRef<EventItem[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listen<DevToolsEventPayload>("devtools:event", (e) => {
      const item: EventItem = {
        id: nextIdRef.current++,
        receivedAt: Date.now(),
        payload: e.payload,
      };

      if (paused) {
        backlogRef.current.push(item);
      } else {
        setEvents((prev) => [...prev, item].slice(-1000));
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [paused]);

  useEffect(() => {
    if (!paused && backlogRef.current.length > 0) {
      const batch = backlogRef.current;
      backlogRef.current = [];
      setEvents((prev) => [...prev, ...batch].slice(-1000));
    }
  }, [paused]);

  const filtered = events.filter(
    (item) =>
      !kindFilter ||
      item.payload.event.kind
        .toLowerCase()
        .includes(kindFilter.toLowerCase()),
  );

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-3">
        <Input
          placeholder="按 kind 过滤..."
          value={kindFilter}
          onChange={(e) => setKindFilter(e.target.value)}
          className="w-48"
        />
        <div className="flex items-center gap-2">
          <Switch checked={paused} onCheckedChange={setPaused} id="pause" />
          <label htmlFor="pause" className="text-sm">
            暂停
          </label>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setEvents([]);
            backlogRef.current = [];
          }}
        >
          清空
        </Button>
      </div>

      <div className="flex-1 overflow-auto rounded border font-mono text-xs">
        <table className="w-full">
          <thead className="sticky top-0 bg-muted">
            <tr>
              <th className="px-2 py-1 text-left">时间</th>
              <th className="px-2 py-1 text-left">接收用户</th>
              <th className="px-2 py-1 text-left">Kind</th>
              <th className="px-2 py-1 text-left">Payload</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((item) => (
              <tr key={item.id} className="border-t">
                <td className="px-2 py-1 whitespace-nowrap">
                  {new Date(item.receivedAt).toLocaleTimeString()}
                </td>
                <td className="px-2 py-1">{item.payload.recipient_user_id}</td>
                <td className="px-2 py-1">{item.payload.event.kind}</td>
                <td className="px-2 py-1 max-w-md truncate">
                  {JSON.stringify(item.payload.event)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add src/views/dev-tools/events-panel.tsx
git commit -m "feat(dev-tools): add events panel"
```

---

### Task 11: 数据库结构面板

**Files:**
- Create: `src/views/dev-tools/schema-panel.tsx`

- [ ] **Step 1: 展示表结构与预览**

```typescript
import { useState } from "react";
import { useDbSchemaQuery } from "@/lib/query";
import type { DbTable } from "@/types/dev-tools";

function TableDetail({ table }: { table: DbTable }) {
  return (
    <div className="space-y-3">
      <div>
        <h3 className="font-semibold text-sm">列</h3>
        <table className="w-full text-sm">
          <thead className="bg-muted">
            <tr>
              <th className="px-2 py-1 text-left">名</th>
              <th className="px-2 py-1 text-left">类型</th>
              <th className="px-2 py-1 text-left">非空</th>
              <th className="px-2 py-1 text-left">默认值</th>
              <th className="px-2 py-1 text-left">PK</th>
            </tr>
          </thead>
          <tbody>
            {table.columns.map((col) => (
              <tr key={col.name} className="border-t">
                <td className="px-2 py-1">{col.name}</td>
                <td className="px-2 py-1">{col.type_name}</td>
                <td className="px-2 py-1">{col.not_null ? "是" : ""}</td>
                <td className="px-2 py-1">{col.default_value ?? "-"}</td>
                <td className="px-2 py-1">{col.primary_key ? "是" : ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {table.indexes.length > 0 && (
        <div>
          <h3 className="font-semibold text-sm">索引</h3>
          <ul className="list-disc pl-5 text-sm">
            {table.indexes.map((idx) => (
              <li key={idx.name}>
                {idx.name} ({idx.unique ? "唯一" : "非唯一"}) →{" "}
                {idx.columns.join(", ")}
              </li>
            ))}
          </ul>
        </div>
      )}

      {table.sql && (
        <div>
          <h3 className="font-semibold text-sm">DDL</h3>
          <pre className="rounded bg-muted p-2 text-xs">{table.sql}</pre>
        </div>
      )}
    </div>
  );
}

export function SchemaPanel() {
  const schemaQuery = useDbSchemaQuery();
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  const selected = schemaQuery.data?.tables.find(
    (t) => t.name === selectedTable,
  );

  return (
    <div className="flex h-full gap-4">
      <div className="w-56 overflow-auto rounded border">
        {schemaQuery.isPending ? (
          <p className="p-2 text-sm text-muted-foreground">读取中...</p>
        ) : schemaQuery.isError ? (
          <p className="p-2 text-sm text-destructive">读取失败</p>
        ) : (
          <ul className="divide-y text-sm">
            {schemaQuery.data?.tables.map((table) => (
              <li
                key={table.name}
                className={`cursor-pointer px-3 py-2 ${
                  selectedTable === table.name ? "bg-muted" : "hover:bg-muted/50"
                }`}
                onClick={() => setSelectedTable(table.name)}
              >
                {table.name}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="flex-1 overflow-auto rounded border p-3">
        {selected ? (
          <TableDetail table={selected} />
        ) : (
          <p className="text-muted-foreground text-sm">选择一个表查看结构</p>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add src/views/dev-tools/schema-panel.tsx
git commit -m "feat(dev-tools): add database schema panel"
```

---

### Task 12: Settings 页面改造

**Files:**
- Modify: `src/views/main/settings.tsx`
- Modify: `src/lib/mutations.ts`

- [ ] **Step 1: 在 `src/lib/mutations.ts` 添加 mutation**

```typescript
import { useOpenDeveloperToolsMutation as useOpenDeveloperToolsMutationImpl } from "@/lib/query/dev-tools";

export function useOpenDeveloperToolsMutation() {
  return useOpenDeveloperToolsMutationImpl();
}
```

或者如果 `src/lib/mutations.ts` 直接从 `@tanstack/react-query` 定义，则直接写：

```typescript
export function useOpenDeveloperToolsMutation() {
  return useMutation({
    mutationFn: () => invoke<boolean>("open_developer_tools"),
  });
}
```

- [ ] **Step 2: 修改 Settings 页面**

导入：

```typescript
import { Bug } from "lucide-react";
import { useOpenDeveloperToolsMutation } from "@/lib/mutations";
```

在组件内新增 mutation：

```typescript
const openDevTools = useOpenDeveloperToolsMutation();

const handleOpenDevTools = () => {
  openDevTools.mutate(undefined, {
    onError: (err) => toast.error(`打开失败: ${err}`),
  });
};
```

把原来的 "DEBUG 模式" 区域改为"开发者模式"：

```typescript
<div className="flex items-center justify-between gap-3">
  <div>
    <p className="font-medium text-sm">开发者模式</p>
    <p className="text-muted-foreground text-xs">
      开启后可查看 DEBUG 日志并打开开发者工具窗口
    </p>
  </div>
  <Switch
    checked={isDebugEnabled}
    onCheckedChange={handleDebugToggle}
    disabled={setLogLevel.isPending}
  />
</div>

{isDebugEnabled && (
  <Button
    type="button"
    variant="outline"
    size="sm"
    className="gap-1.5"
    onClick={handleOpenDevTools}
    disabled={openDevTools.isPending}
  >
    <Bug className="size-3.5" />
    打开开发者工具
  </Button>
)}
```

同时把 toast 文案从 "DEBUG 模式" 改为 "开发者模式"。

- [ ] **Step 3: 运行前端 lint/build 检查**

```bash
bunx --bun @biomejs/biome check --write
bun run build
```

- [ ] **Step 4: 提交**

```bash
git add src/views/main/settings.tsx src/lib/mutations.ts
git commit -m "feat(dev-tools): rename debug mode and add open developer tools button"
```

---

### Task 13: PR1 手动验收

- [ ] **Step 1: 运行完整构建**

```bash
bunx tauri build --no-bundle
```

- [ ] **Step 2: 启动应用并验证**

```bash
bunx tauri dev
```

- [ ] **Step 3: 验收清单**

- [ ] Settings 页面显示"开发者模式"开关，开启后出现"打开开发者工具"按钮。
- [ ] 点击按钮打开独立窗口，标题为"开发者工具"，尺寸约 1200x800。
- [ ] 日志标签页能加载并过滤系统日志。
- [ ] 事件流标签页能实时显示聊天窗口触发的事件；打开 dev tools 后再注册新用户，其事件也能出现。
- [ ] 数据库标签页能列出所有表，点击表名显示列、索引、DDL。
- [ ] 关闭 dev tools 窗口后重新打开，事件流继续工作，无重复事件。

- [ ] **Step 4: 提交 PR1**

```bash
git push origin feat/developer-mode-pr1
```

---

## PR2：SQL 执行器

### Task 14: 后端命令 `execute_sql` 与写保护

**Files:**
- Modify: `src-tauri/src/commands/dev_tools.rs`

- [ ] **Step 1: 添加 SQL 写操作检测函数与 Rust 返回类型**

在 `src-tauri/src/commands/dev_tools.rs` 顶部追加：

```rust
use sqlx::Column;

const WRITE_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "REPLACE", "DROP", "CREATE", "ALTER", "TRUNCATE",
];

fn is_write_query(query: &str) -> bool {
    let normalized = query.to_uppercase();
    WRITE_KEYWORDS.iter().any(|kw| normalized.contains(kw))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: Option<u64>,
}
```

- [ ] **Step 2: 实现 `execute_sql` 命令**

```rust
#[tauri::command]
pub async fn execute_sql(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    query: String,
    allow_write: bool,
) -> AppResult<SqlQueryResult> {
    if !allow_write && is_write_query(&query) {
        return Err(AppError::validation(
            "write queries require allow_write=true",
        ));
    }

    if query.trim().is_empty() {
        return Err(AppError::validation("query is empty"));
    }

    let rows = sqlx::query(&query)
        .fetch_all(&**pool)
        .await
        .map_err(|e| AppError::storage(format!("failed to execute sql: {e}")))?;

    if rows.is_empty() {
        return Ok(SqlQueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: None,
        });
    }

    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let mut result_rows = Vec::new();
    for row in rows {
        let mut values = Vec::new();
        for (idx, _) in columns.iter().enumerate() {
            let value: serde_json::Value = if let Ok(v) = row.try_get::<i64, _>(idx) {
                serde_json::Value::Number(v.into())
            } else if let Ok(v) = row.try_get::<f64, _>(idx) {
                serde_json::Number::from_f64(v)
                    .map_or(serde_json::Value::Null, serde_json::Value::Number)
            } else if let Ok(v) = row.try_get::<String, _>(idx) {
                serde_json::Value::String(v)
            } else if let Ok(v) = row.try_get::<bool, _>(idx) {
                serde_json::Value::Bool(v)
            } else if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
                serde_json::Value::String(format!("<BLOB {} bytes>", v.len()))
            } else {
                serde_json::Value::Null
            };
            values.push(value);
        }
        result_rows.push(values);
    }

    Ok(SqlQueryResult {
        columns,
        rows: result_rows,
        rows_affected: None,
    })
}
```

- [ ] **Step 3: 在 `lib.rs` 注册 `execute_sql`**

```rust
dev_tools::execute_sql,
```

- [ ] **Step 4: 运行 `cargo check`**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands/dev_tools.rs src-tauri/src/lib.rs
git commit -m "feat(dev-tools): add execute_sql command with write guard"
```

---

### Task 15: SQL 写保护单元测试

**Files:**
- Create: `src-tauri/src/commands/dev_tools_tests.rs` 或在 `commands/dev_tools.rs` 底部加 `#[cfg(test)]` 模块

- [ ] **Step 1: 添加测试模块**

在 `src-tauri/src/commands/dev_tools.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::is_write_query;

    #[test]
    fn detects_write_queries() {
        let writes = [
            "INSERT INTO users VALUES ('x')",
            "UPDATE users SET name = 'x'",
            "DELETE FROM users",
            "REPLACE INTO users VALUES ('x')",
            "DROP TABLE users",
            "CREATE TABLE foo (id INTEGER)",
            "ALTER TABLE users ADD COLUMN x TEXT",
            "TRUNCATE TABLE users",
        ];
        for q in writes {
            assert!(is_write_query(q), "expected write: {q}");
        }
    }

    #[test]
    fn read_queries_are_not_write() {
        let reads = [
            "SELECT * FROM users",
            "PRAGMA table_info(users)",
            "EXPLAIN SELECT * FROM users",
            "SELECT 'INSERT' FROM users",
        ];
        for q in reads {
            assert!(!is_write_query(q), "expected read: {q}");
        }
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml dev_tools::tests
```

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/commands/dev_tools.rs
git commit -m "test(dev-tools): add SQL write guard tests"
```

---

### Task 16: 前端 SQL 面板

**Files:**
- Create: `src/views/dev-tools/sql-panel.tsx`
- Modify: `src/views/dev-tools/dev-tools-window.tsx`

- [ ] **Step 1: 创建 SQL 面板**

```typescript
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useExecuteSqlMutation } from "@/lib/query";
import { confirmDialog } from "@/lib/modal";

export function SqlPanel() {
  const [query, setQuery] = useState("SELECT * FROM users LIMIT 10");
  const [allowWrite, setAllowWrite] = useState(false);
  const [result, setResult] = useState<{
    columns: string[];
    rows: (string | number | boolean | null)[][];
  } | null>(null);
  const execute = useExecuteSqlMutation();

  const handleExecute = async () => {
    const trimmed = query.trim();
    if (!trimmed) {
      toast.error("SQL 为空");
      return;
    }

    const writeKeywords = ["INSERT", "UPDATE", "DELETE", "REPLACE", "DROP", "CREATE", "ALTER", "TRUNCATE"];
    const isWrite = writeKeywords.some((kw) =>
      trimmed.toUpperCase().includes(kw),
    );

    if (isWrite && !allowWrite) {
      toast.error("写操作需在上方开启"允许写操作"");
      return;
    }

    if (isWrite && allowWrite) {
      const confirmed = await confirmDialog({
        title: "确认执行写操作",
        description: "该 SQL 可能修改数据库，确定继续？",
        confirmText: "执行",
      });
      if (!confirmed) return;
    }

    execute.mutate(
      { query: trimmed, allowWrite },
      {
        onSuccess: (data) => {
          setResult({ columns: data.columns, rows: data.rows });
          toast.success("执行成功");
        },
        onError: (err) => toast.error(`执行失败: ${err}`),
      },
    );
  };

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Switch
            checked={allowWrite}
            onCheckedChange={setAllowWrite}
            id="allow-write"
          />
          <label htmlFor="allow-write" className="text-sm">
            允许写操作
          </label>
        </div>
        <Button
          onClick={handleExecute}
          disabled={execute.isPending}
          size="sm"
        >
          执行
        </Button>
      </div>

      <Textarea
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="min-h-[120px] font-mono text-sm"
        placeholder="输入 SQL..."
      />

      {result && (
        <div className="flex-1 overflow-auto rounded border">
          <table className="w-full text-xs">
            <thead className="sticky top-0 bg-muted">
              <tr>
                {result.columns.map((col) => (
                  <th key={col} className="px-2 py-1 text-left">
                    {col}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {result.rows.map((row, idx) => (
                <tr key={idx} className="border-t">
                  {row.map((cell, cidx) => (
                    <td key={cidx} className="px-2 py-1">
                      {cell === null ? "NULL" : String(cell)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: 在窗口入口启用 SQL 标签页**

在 `dev-tools-window.tsx` 中把 `TabsTrigger value="sql" disabled` 改为启用，并添加 `TabsContent`：

```typescript
import { SqlPanel } from "@/views/dev-tools/sql-panel";

<TabsTrigger value="sql">SQL</TabsTrigger>

<TabsContent value="sql" className="flex-1 overflow-hidden">
  <SqlPanel />
</TabsContent>
```

- [ ] **Step 3: 运行 lint/build**

```bash
bunx --bun @biomejs/biome check --write
bun run build
```

- [ ] **Step 4: 提交**

```bash
git add src/views/dev-tools/sql-panel.tsx src/views/dev-tools/dev-tools-window.tsx
git commit -m "feat(dev-tools): add SQL executor panel"
```

---

### Task 17: PR2 手动验收

- [ ] **Step 1: 运行完整构建**

```bash
bunx tauri build --no-bundle
```

- [ ] **Step 2: 启动应用并验证**

```bash
bunx tauri dev
```

- [ ] **Step 3: 验收清单**

- [ ] SQL 面板执行 `SELECT * FROM users LIMIT 10` 返回结果表格。
- [ ] 执行 `INSERT INTO ...` 时，若未开启"允许写操作"，前端提示错误，后端拒绝。
- [ ] 开启"允许写操作"后执行写 SQL，弹出确认对话框，确认后成功执行。
- [ ] 在 main/chat 窗口中无法调用 `execute_sql`（Tauri capability 拒绝）。

- [ ] **Step 4: 提交 PR2**

```bash
git push origin feat/developer-mode-pr2
```

---

## 自评检查

### Spec 覆盖

- [x] 开发者模式开关重命名 → Task 12
- [x] 独立开发者工具窗口 → Task 4, Task 8
- [x] 日志面板 → Task 9
- [x] 实时事件流 → Task 3 (firehose), Task 4 (forwarding), Task 10 (UI)
- [x] 数据库结构浏览器 → Task 5, Task 11
- [x] SQL 执行器 → Task 14, Task 16
- [x] 安全：capability 隔离 → Task 6
- [x] 安全：写操作双重校验 → Task 14, Task 16

### Placeholder 扫描

- [x] 无 "TBD"/"TODO"
- [x] 所有代码片段包含完整实现
- [x] 所有命令与类型命名一致（`DevToolsEvent`、`execute_sql`、`get_db_schema`、`open_developer_tools`）

### 类型一致性

- [x] 后端 `DevToolsEvent` 与前端 `DevToolsEventPayload` 字段匹配（`recipient_user_id` + `event`）
- [x] `SqlQueryResult` 前后端字段一致（`columns`、`rows`、`rows_affected`）
- [x] `DbSchema` / `DbTable` / `DbColumn` / `DbIndex` 前后端一致

---

## 执行交接

**Plan complete and saved to `docs/superpowers/plans/2026-06-12-developer-mode.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
