# Logs 页面与群文件 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Logs 页面展示 protocol_packets 数据，并完成群文件的上传/下载功能。

**Architecture:** Logs 复用已有的 packet query hooks，通过 adapter 映射为 LogEntry view model；群文件通过 tauri-plugin-dialog 做文件选择，存储到 `{app_data_dir}/groups/{group_id}/files/`，数据库通过 migration 0003 新增 file_path 字段。

**Tech Stack:** Rust edition 2024, sqlx 0.8, SQLite, Tauri 2, tauri-plugin-dialog, axum (已有), React/TypeScript, shadcn/ui

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/Cargo.toml` | 修改 | 添加 `tauri-plugin-dialog` |
| `src-tauri/tauri.conf.json` | 修改 | 启用 `assetProtocol`，scope `$APPDATA/groups/**` |
| `src-tauri/src/lib.rs` | 修改 | 初始化 dialog 插件 |
| `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql` | 创建 | migration：group_files 加 file_path，group_photos 重建 |
| `src-tauri/src/persistence/migrations/mod.rs` | 修改 | 注册 0003 |
| `src-tauri/src/persistence/repo/packet.rs` | 修改 | `list_packets` 增加 protocol_type / is_error / until |
| `src-tauri/src/commands/packet.rs` | 修改 | `list_protocol_packets` 增加筛选参数 |
| `src-tauri/src/persistence/repo/group/content.rs` | 修改 | 新增群文件上传/下载/删除方法 |
| `src-tauri/src/services/group/content.rs` | 修改 | 文件上传/下载服务层 |
| `src-tauri/src/commands/chat/group.rs` | 修改 | 新增 upload/download/delete group file 命令 |
| `src-tauri/src/models/entities.rs` | 修改 | `GroupFileEntity` 加 `file_path` |
| `src/types/packet.ts` | 修改 | `PacketFilters` 增加筛选字段 |
| `src/types/group.ts` | 修改 | `GroupFileEntity` 加 `file_path` |
| `src/lib/query/packets.ts` | 修改 | 透传新筛选参数 |
| `src/lib/query/logs.ts` | 创建 | `useLogsQuery` hook |
| `src/views/main/logs.tsx` | 重写 | Logs 页面 |
| `src/components/chat/group-files.tsx` | 创建 | 群文件列表组件 |

---

### Task 1: Extend Packet Query with New Filters

**Files:**
- Modify: `src-tauri/src/persistence/repo/packet.rs`
- Modify: `src-tauri/src/commands/packet.rs`
- Modify: `src/types/packet.ts`
- Modify: `src/lib/query/packets.ts`

- [ ] **Step 1: Extend `PacketRepo::list_packets`**

  In `src-tauri/src/persistence/repo/packet.rs`, modify `list_packets` signature and body:

  ```rust
  pub async fn list_packets(
      &self,
      bot_id: Option<&str>,
      direction: Option<&str>,
      action_name: Option<&str>,
      protocol_type: Option<&str>,
      is_error: Option<bool>,
      since: Option<u64>,
      until: Option<u64>,
      limit: i64,
  ) -> Result<Vec<ProtocolPacketRecord>, sqlx::Error> {
      let limit = limit.min(1000);

      let mut builder: QueryBuilder<'_, sqlx::Sqlite> =
          QueryBuilder::new("SELECT * FROM protocol_packets");

      let mut has_where = false;

      if let Some(bot_id) = bot_id {
          builder.push(" WHERE bot_id = ");
          builder.push_bind(bot_id);
          has_where = true;
      }

      if let Some(direction) = direction {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("direction = ");
          builder.push_bind(direction);
      }

      if let Some(action_name) = action_name {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("action_name = ");
          builder.push_bind(action_name);
      }

      if let Some(protocol_type) = protocol_type {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("protocol_type = ");
          builder.push_bind(protocol_type);
      }

      if let Some(is_error) = is_error {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("is_error = ");
          builder.push_bind(if is_error { 1 } else { 0 });
      }

      if let Some(since) = since {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("created_at >= ");
          builder.push_bind(since as i64);
      }

      if let Some(until) = until {
          if has_where { builder.push(" AND "); } else { builder.push(" WHERE "); has_where = true; }
          builder.push("created_at <= ");
          builder.push_bind(until as i64);
      }

      builder.push(" ORDER BY created_at DESC LIMIT ");
      builder.push_bind(limit);

      builder.build_query_as::<ProtocolPacketRecord>()
          .fetch_all(&self.pool)
          .await
  }
  ```

- [ ] **Step 2: Extend `list_protocol_packets` command**

  In `src-tauri/src/commands/packet.rs`, modify the command:

  ```rust
  #[tauri::command]
  pub async fn list_protocol_packets(
      pool: tauri::State<'_, sqlx::SqlitePool>,
      bot_id: Option<String>,
      direction: Option<String>,
      action_name: Option<String>,
      protocol_type: Option<String>,
      is_error: Option<bool>,
      since: Option<u64>,
      until: Option<u64>,
      limit: Option<i64>,
  ) -> Result<Vec<ProtocolPacketRecord>, String> {
      let repo = PacketRepo::new(pool.inner().clone());
      let limit = limit.unwrap_or(100).min(1000);
      repo.list_packets(
          bot_id.as_deref(),
          direction.as_deref(),
          action_name.as_deref(),
          protocol_type.as_deref(),
          is_error,
          since,
          until,
          limit,
      )
      .await
      .map_err(|e| e.to_string())
  }
  ```

- [ ] **Step 3: Update TypeScript types**

  In `src/types/packet.ts`, modify `PacketFilters`:

  ```typescript
  export interface PacketFilters {
    bot_id?: string;
    direction?: "receive" | "send";
    action_name?: string;
    protocol_type?: string;
    is_error?: boolean;
    since?: number;
    until?: number;
    limit?: number;
  }
  ```

- [ ] **Step 4: Update query hook**

  In `src/lib/query/packets.ts`, modify `useProtocolPackets`:

  ```typescript
  export function useProtocolPackets(filters: PacketFilters = {}) {
    return useQuery({
      queryKey: queryKeys.packets.list(filters),
      queryFn: async () => {
        return invoke<ProtocolPacket[]>("list_protocol_packets", {
          botId: filters.bot_id ?? null,
          direction: filters.direction ?? null,
          actionName: filters.action_name ?? null,
          protocolType: filters.protocol_type ?? null,
          isError: filters.is_error ?? null,
          since: filters.since ?? null,
          until: filters.until ?? null,
          limit: filters.limit ?? 100,
        });
      },
      refetchInterval: 2000,
      retry: false,
    });
  }
  ```

- [ ] **Step 5: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  bun run build
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add src-tauri/src/persistence/repo/packet.rs src-tauri/src/commands/packet.rs src/types/packet.ts src/lib/query/packets.ts
  git commit -m "feat(logs): extend packet query with protocol_type, is_error, until filters"
  ```

---

### Task 2: Rewrite Logs Page

**Files:**
- Create: `src/lib/query/logs.ts`
- Modify: `src/views/main/logs.tsx`
- Modify: `src/lib/query/keys.ts`

- [ ] **Step 1: Add logs query key**

  In `src/lib/query/keys.ts`, add to `queryKeys`:

  ```typescript
  logs: {
    list: (filters: Record<string, unknown>) => ["logs", "list", JSON.stringify(filters)] as const,
    detail: (ref: string) => ["logs", "detail", ref] as const,
  },
  ```

  Add to `packets` section:
  ```typescript
  packets: {
    list: (filters: Record<string, unknown>) => ["packets", "list", JSON.stringify(filters)] as const,
    detail: (packetId: string) => ["packets", "detail", packetId] as const,
  },
  ```

  (Note: if `queryKeys.packets` already exists, just verify it covers list/detail.)

- [ ] **Step 2: Add `useLogsQuery` hook**

  Create `src/lib/query/logs.ts`:

  ```typescript
  import { useQuery } from "@tanstack/react-query";
  import { invoke } from "@tauri-apps/api/core";
  import { queryKeys } from "@/lib/query/keys";
  import type { PacketFilters } from "@/types/packet";

  export interface LogEntry {
    id: string;
    time: number;
    level: "info" | "error" | "debug" | "warn";
    eventType: string;
    source: string;
    message: string;
    dataSource: string;
    detailRef: string;
  }

  function packetToLogEntry(packet: {
    packet_id: string;
    created_at: number;
    is_error: boolean;
    protocol_type: string;
    direction: string;
    bot_id: string | null;
    profile_id: string | null;
    action_name: string;
  }): LogEntry {
    return {
      id: `packet:${packet.packet_id}`,
      time: packet.created_at,
      level: packet.is_error ? "error" : "info",
      eventType: `packet.${packet.direction}`,
      source: packet.bot_id || packet.profile_id || "system",
      message: packet.action_name,
      dataSource: "packet",
      detailRef: packet.packet_id,
    };
  }

  export function useLogsQuery(filters: PacketFilters = {}) {
    return useQuery({
      queryKey: queryKeys.logs.list(filters),
      queryFn: async () => {
        const packets = await invoke<
          Array<{
            packet_id: string;
            created_at: number;
            is_error: boolean;
            protocol_type: string;
            direction: string;
            bot_id: string | null;
            profile_id: string | null;
            action_name: string;
          }>
        >("list_protocol_packets", {
          botId: filters.bot_id ?? null,
          direction: filters.direction ?? null,
          actionName: filters.action_name ?? null,
          protocolType: filters.protocol_type ?? null,
          isError: filters.is_error ?? null,
          since: filters.since ?? null,
          until: filters.until ?? null,
          limit: filters.limit ?? 100,
        });
        return packets.map(packetToLogEntry);
      },
      refetchInterval: 2000,
      retry: false,
    });
  }

  export function useLogDetailQuery(detailRef: string | null) {
    return useQuery({
      queryKey: queryKeys.logs.detail(detailRef ?? ""),
      queryFn: async () => {
        if (!detailRef) return null;
        return invoke<string>("read_protocol_packet", { packetId: detailRef });
      },
      enabled: !!detailRef,
      retry: false,
    });
  }
  ```

- [ ] **Step 3: Rewrite `logs.tsx`**

  Replace `src/views/main/logs.tsx` with:

  ```tsx
  import { useState } from "react";
  import {
    ScrollText,
    Shield,
    ChevronDown,
    ChevronUp,
    AlertCircle,
    Info,
  } from "lucide-react";
  import { Button } from "@/components/ui/button";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "@/components/ui/select";
  import { useLogsQuery, useLogDetailQuery } from "@/lib/query/logs";

  type TimeRange = "15m" | "1h" | "24h" | "7d" | "all";

  function timeRangeToMs(range: TimeRange): number | null {
    if (range === "all") return null;
    const now = Date.now();
    const map: Record<Exclude<TimeRange, "all">, number> = {
      "15m": 15 * 60 * 1000,
      "1h": 60 * 60 * 1000,
      "24h": 24 * 60 * 60 * 1000,
      "7d": 7 * 24 * 60 * 60 * 1000,
    };
    return now - map[range];
  }

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function LogLevelBadge({ level }: { level: string }) {
    if (level === "error") {
      return (
        <span className="inline-flex items-center gap-1 rounded border border-destructive/30 bg-destructive/10 px-1.5 py-0.5 text-destructive text-xs font-medium">
          <AlertCircle className="size-3" />
          ERROR
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 rounded border border-sky-500/30 bg-sky-500/10 px-1.5 py-0.5 text-sky-600 text-xs font-medium">
        <Info className="size-3" />
        INFO
      </span>
    );
  }

  function LogDetail({ detailRef }: { detailRef: string }) {
    const detail = useLogDetailQuery(detailRef);

    if (detail.isPending) {
      return <p className="text-muted-foreground text-xs">加载中...</p>;
    }
    if (detail.isError) {
      return (
        <p className="text-muted-foreground text-xs">
          原始报文文件已丢失或过期
        </p>
      );
    }

    return (
      <pre className="mt-2 max-h-64 overflow-auto rounded bg-muted/50 p-2 text-xs">
        {detail.data}
      </pre>
    );
  }

  export default function LogsView() {
    const [level, setLevel] = useState<"all" | "info" | "error">("all");
    const [timeRange, setTimeRange] = useState<TimeRange>("24h");
    const [expandedId, setExpandedId] = useState<string | null>(null);

    const since = timeRangeToMs(timeRange);
    const isError = level === "all" ? undefined : level === "error";

    const logsQuery = useLogsQuery({
      since: since ?? undefined,
      is_error: isError,
      limit: 100,
    });
    const logs = logsQuery.data ?? [];

    return (
      <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
        <div className="space-y-3 rounded-xl border bg-card/60 p-3">
          <div className="flex items-center gap-2 text-sm">
            <ScrollText className="size-4" />
            <span className="font-medium">运行日志</span>
          </div>

          <div className="grid gap-2 md:grid-cols-3">
            <div className="space-y-1 text-xs">
              <span className="text-muted-foreground">等级</span>
              <Select
                value={level}
                onValueChange={(v) => setLevel(v as typeof level)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">全部</SelectItem>
                  <SelectItem value="info">INFO</SelectItem>
                  <SelectItem value="error">ERROR</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1 text-xs">
              <span className="text-muted-foreground">时间范围</span>
              <Select
                value={timeRange}
                onValueChange={(v) => setTimeRange(v as TimeRange)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="15m">最近 15 分钟</SelectItem>
                  <SelectItem value="1h">最近 1 小时</SelectItem>
                  <SelectItem value="24h">最近 24 小时</SelectItem>
                  <SelectItem value="7d">最近 7 天</SelectItem>
                  <SelectItem value="all">全部时间</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1 text-xs">
              <span className="text-muted-foreground">匹配数</span>
              <p className="flex h-9 items-center text-sm">
                {logs.length} 条日志
              </p>
            </div>
          </div>
        </div>

        <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card/60">
          <div className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
            {logsQuery.isPending ? (
              <p className="text-muted-foreground text-sm">读取中...</p>
            ) : logs.length === 0 ? (
              <p className="text-muted-foreground text-sm">暂无日志</p>
            ) : (
              logs.map((log) => {
                const isExpanded = expandedId === log.id;
                return (
                  <div
                    key={log.id}
                    className="rounded-lg border bg-card px-3 py-2 text-xs"
                  >
                    <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
                      <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                        {formatTime(log.time)}
                      </span>
                      <LogLevelBadge level={log.level} />
                      <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                        {log.eventType}
                      </span>
                      <span className="text-muted-foreground">{log.source}</span>
                    </div>
                    <p className="mt-1.5 break-all text-sm leading-relaxed">
                      {log.message}
                    </p>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="mt-1 h-6 px-1 text-xs"
                      onClick={() =>
                        setExpandedId(isExpanded ? null : log.id)
                      }
                    >
                      {isExpanded ? (
                        <>
                          <ChevronUp className="mr-1 size-3" />
                          收起详情
                        </>
                      ) : (
                        <>
                          <ChevronDown className="mr-1 size-3" />
                          查看详情
                        </>
                      )}
                    </Button>
                    {isExpanded && (
                      <LogDetail detailRef={log.detailRef} />
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    );
  }
  ```

- [ ] **Step 4: Verify build**

  ```bash
  bun run build
  ```

- [ ] **Step 5: Commit**

  ```bash
  git add src/lib/query/logs.ts src/views/main/logs.tsx src/lib/query/keys.ts
  git commit -m "feat(logs): rewrite logs page with protocol_packets integration"
  ```

---

### Task 3: Add Migration 0003 for Group File Paths

**Files:**
- Create: `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql`
- Modify: `src-tauri/src/persistence/migrations/mod.rs`

- [ ] **Step 1: Write migration 0003**

  Create `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql`:

  ```sql
  -- Migration 0003: Add file_path to group_files, rebuild group_photos with nullable url + file_path

  -- group_files: add file_path column
  ALTER TABLE group_files ADD COLUMN file_path TEXT;

  -- group_photos: url is NOT NULL, SQLite cannot ALTER COLUMN, must rebuild
  CREATE TABLE group_photos_new (
      photo_id         TEXT PRIMARY KEY NOT NULL,
      album_id         TEXT NOT NULL,
      url              TEXT,              -- was NOT NULL, now nullable: external URL
      file_path        TEXT,              -- new: local relative path
      description      TEXT,
      uploader_user_id TEXT NOT NULL,
      file_size        INTEGER,
      created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
      FOREIGN KEY (album_id) REFERENCES group_albums(album_id) ON DELETE CASCADE,
      FOREIGN KEY (uploader_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
      CHECK (url IS NOT NULL OR file_path IS NOT NULL)
  );

  INSERT INTO group_photos_new (
      photo_id, album_id, url, description, uploader_user_id, file_size, created_at
  )
  SELECT
      photo_id, album_id, url, description, uploader_user_id, file_size, created_at
  FROM group_photos;

  DROP TABLE group_photos;
  ALTER TABLE group_photos_new RENAME TO group_photos;

  CREATE INDEX idx_photos_album ON group_photos(album_id, created_at DESC);
  ```

- [ ] **Step 2: Register migration**

  In `src-tauri/src/persistence/migrations/mod.rs`:

  ```rust
  pub fn all_migrations() -> Vec<Migration> {
      vec![
          Migration {
              version: "0001",
              sql: include_str!("0001_initial_schema.sql"),
          },
          Migration {
              version: "0002",
              sql: include_str!("0002_message_seq.sql"),
          },
          Migration {
              version: "0003",
              sql: include_str!("0003_group_file_photo_paths.sql"),
          },
      ]
  }
  ```

- [ ] **Step 3: Run tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml migrates_from_blank_to_latest
  ```

  Expected: passes (now creates all 3 migrations' tables).

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql src-tauri/src/persistence/migrations/mod.rs
  git commit -m "feat(db): add migration 0003 for group file/photo paths"
  ```

---

### Task 4: Add tauri-plugin-dialog and Asset Protocol

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add dependency**

  In `src-tauri/Cargo.toml`, add to `[dependencies]`:

  ```toml
  tauri-plugin-dialog = "2"
  sha2 = "0.10"
  hex = "0.4"
  ```

- [ ] **Step 2: Configure tauri.conf.json**

  In `src-tauri/tauri.conf.json`, modify `app.security`:

  ```json
  "app": {
    "windows": [
      {
        "title": "UniBot",
        "width": 1024,
        "height": 768,
        "minWidth": 800,
        "minHeight": 600,
        "decorations": false
      }
    ],
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": ["$APPDATA/**"]
      }
    }
  },
  ```

- [ ] **Step 3: Initialize plugin in lib.rs**

  In `src-tauri/src/lib.rs`, add to `.plugin(...)` chain:

  ```rust
  .plugin(tauri_plugin_dialog::init())
  ```

- [ ] **Step 4: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

  Expected: downloads tauri-plugin-dialog, compiles.

- [ ] **Step 5: Commit**

  ```bash
  git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/src/lib.rs
  git commit -m "feat: add tauri-plugin-dialog and enable assetProtocol"
  ```

---

### Task 5: Implement Group File Upload/Download Backend

**Files:**
- Modify: `src-tauri/src/models/entities.rs`
- Modify: `src-tauri/src/persistence/repo/group/content.rs`
- Modify: `src-tauri/src/persistence/repo/group/types.rs`
- Modify: `src-tauri/src/services/group/content.rs`
- Modify: `src-tauri/src/commands/chat/group.rs`

- [ ] **Step 1: Add `file_path` to `GroupFileEntity`**

  In `src-tauri/src/models/entities.rs`, add to `GroupFileEntity`:

  ```rust
  pub file_path: Option<String>,
  ```

- [ ] **Step 2: Update `GroupFileRow` and mapping**

  In `src-tauri/src/persistence/repo/group/types.rs`, add `file_path` to the row struct and `TryFrom` mapping.

- [ ] **Step 3: Add file storage helpers**

  Add to `src-tauri/src/utils.rs`:

  ```rust
  use std::path::Path;

  pub fn sanitize_file_name(name: &str) -> String {
      let mut s = name
          .replace(['/', '\\', ':', '?', '*', '"', '<', '>', '|'], "_")
          .replace(|c: char| c.is_control(), "");
      // Remove trailing dots and spaces (Windows compatibility)
      while s.ends_with('.') || s.ends_with(' ') {
          s.pop();
      }
      // Windows reserved names
      let reserved = ["CON", "PRN", "AUX", "NUL",
          "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
          "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
      let upper = s.to_uppercase();
      if reserved.iter().any(|&r| upper == r) {
          s = format!("_{}", s);
      }
      // Length limit
      if s.len() > 120 {
          if let Some(pos) = s.rfind('.') {
              let ext = &s[pos..];
              let name = &s[..pos.min(120 - ext.len())];
              s = format!("{}{}", name, ext);
          } else {
              s.truncate(120);
          }
      }
      if s.is_empty() {
          s = "file".to_string();
      }
      s
  }

  pub fn ensure_within_app_data(app_data_dir: &Path, relative: &str) -> Option<PathBuf> {
      let full = app_data_dir.join(relative);
      let canonical = std::fs::canonicalize(&full).ok()?;
      let canonical_base = std::fs::canonicalize(app_data_dir).ok()?;
      if canonical.starts_with(&canonical_base) {
          Some(canonical)
      } else {
          None
      }
  }
  ```

- [ ] **Step 4: Add upload/download/delete commands**

  In `src-tauri/src/commands/chat/group.rs`, append:

  ```rust
  use tauri_plugin_dialog::DialogExt;

  #[tauri::command]
  pub async fn upload_group_file(
      app: tauri::AppHandle,
      services: tauri::State<'_, ServiceHub>,
      user_id: String,
      group_id: String,
      parent_folder_id: Option<String>,
  ) -> Result<GroupFileEntity, String> {
      // Open file picker via dialog plugin
      let file_path = app.dialog()
          .file()
          .pick_file()
          .await
          .ok_or("no file selected")?;

      let path = std::path::PathBuf::from(file_path.to_string());
      let file_name = path.file_name()
          .and_then(|n| n.to_str())
          .unwrap_or("unknown")
          .to_string();

      services
          .group
          .upload_group_file(&app, user_id, group_id, parent_folder_id.unwrap_or_default(), file_name, &path)
          .await
          .map_err(|e| e.to_string())
  }

  #[tauri::command]
  pub async fn download_group_file(
      app: tauri::AppHandle,
      services: tauri::State<'_, ServiceHub>,
      user_id: String,
      file_id: String,
  ) -> Result<(), String> {
      services
          .group
          .download_group_file(&app, user_id, file_id)
          .await
          .map_err(|e| e.to_string())
  }

  #[tauri::command]
  pub async fn delete_group_file(
      services: tauri::State<'_, ServiceHub>,
      user_id: String,
      file_id: String,
  ) -> Result<(), String> {
      services
          .group
          .delete_group_file(user_id, file_id)
          .await
          .map_err(|e| e.to_string())
  }
  ```

- [ ] **Step 5: Add service methods**

  In `src-tauri/src/services/group/content.rs`, add:

  ```rust
  use std::path::Path;
  use tauri::Manager;

  pub async fn upload_group_file(
      &self,
      app: &tauri::AppHandle,
      user_id: String,
      group_id: String,
      parent_folder_id: String,
      file_name: String,
      source_path: &Path,
  ) -> AppResult<GroupFileEntity> {
      core.require_user_context(&user_id)?;
      self.ensure_group_member(&group_id, &user_id).await?;

      let app_data_dir = app.path().app_data_dir()
          .map_err(|e| AppError::Internal(format!("app dir: {e}")))?;

      let file_id = crate::utils::new_db_id();
      let sanitized = crate::utils::sanitize_file_name(&file_name);
      let dest_name = format!("{file_id}_{sanitized}");
      let relative_dir = format!("groups/{group_id}/files");
      let dest_dir = app_data_dir.join(&relative_dir);
      std::fs::create_dir_all(&dest_dir)
          .map_err(|e| AppError::Storage(format!("create dir: {e}")))?;

      let dest_path = dest_dir.join(&dest_name);
      let temp_path = dest_dir.join(format!(".{dest_name}.tmp"));

      // Copy source to temp, then rename
      std::fs::copy(source_path, &temp_path)
          .map_err(|e| AppError::Storage(format!("copy file: {e}")))?;
      std::fs::rename(&temp_path, &dest_path)
          .map_err(|e| {
              let _ = std::fs::remove_file(&temp_path);
              AppError::Storage(format!("rename: {e}"))
          })?;

      let relative_path = format!("{relative_dir}/{dest_name}");
      let file_size = std::fs::metadata(&dest_path)
          .map(|m| m.len())
          .unwrap_or(0);

      let file_hash = {
          let bytes = std::fs::read(&dest_path)
              .map_err(|e| AppError::Storage(format!("read for hash: {e}")))?;
          use sha2::{Sha256, Digest};
          let hash = Sha256::digest(&bytes);
          hex::encode(hash)
      };

      let file = GroupFileEntity {
          file_id,
          group_id: group_id.clone(),
          parent_folder_id,
          file_name,
          file_size,
          file_hash,
          uploader_user_id: user_id,
          uploaded_at: crate::utils::now_ts(),
          expire_at: None,
          download_count: 0,
          file_path: Some(relative_path),
      };

      self.repo.upsert_group_file(&file).await?;
      Ok(file)
  }

  pub async fn download_group_file(
      &self,
      app: &tauri::AppHandle,
      user_id: String,
      file_id: String,
  ) -> AppResult<()> {
      core.require_user_context(&user_id)?;
      let file = self.repo.get_group_file(&file_id).await?
          .ok_or_else(|| AppError::not_found(format!("file {file_id} not found")))?;
      self.ensure_group_member(&file.group_id, &user_id).await?;

      let app_data_dir = app.path().app_data_dir()
          .map_err(|e| AppError::Internal(format!("app dir: {e}")))?;

      let relative = file.file_path
          .ok_or_else(|| AppError::Internal("file has no local path".to_string()))?;
      let full_path = crate::utils::ensure_within_app_data(&app_data_dir, &relative)
          .ok_or_else(|| AppError::Internal("invalid file path".to_string()))?;

      // Let user pick save location
      let save_path = app.dialog()
          .file()
          .set_file_name(&file.file_name)
          .save_file()
          .await
          .ok_or_else(|| AppError::validation("no save location selected"))?;

      std::fs::copy(&full_path, std::path::PathBuf::from(save_path.to_string()))
          .map_err(|e| AppError::Storage(format!("copy: {e}")))?;

      // Increment download count
      self.repo.increment_download_count(&file_id).await?;
      Ok(())
  }

  pub async fn delete_group_file(
      &self,
      user_id: String,
      file_id: String,
  ) -> AppResult<()> {
      core.require_user_context(&user_id)?;
      let file = self.repo.get_group_file(&file_id).await?
          .ok_or_else(|| AppError::not_found(format!("file {file_id} not found")))?;
      self.ensure_group_member(&file.group_id, &user_id).await?;

      // Check permission: uploader or owner/admin
      let member = self.group_repo.get_group_member(&file.group_id, &user_id).await?
          .ok_or_else(|| AppError::validation("not a group member"))?;
      if file.uploader_user_id != user_id && !matches!(member.role, GroupRole::Owner | GroupRole::Admin) {
          return Err(AppError::validation("no permission to delete this file"));
      }

      // Delete DB row (cascade handles related)
      self.repo.delete_group_file(&file_id).await?;
      Ok(())
  }
  ```

- [ ] **Step 6: Add repo methods**

  In `src-tauri/src/persistence/repo/group/content.rs`, add:

  ```rust
  pub async fn get_group_file(
      &self,
      file_id: &str,
  ) -> Result<Option<GroupFileRow>, sqlx::Error> {
      sqlx::query_as::<_, GroupFileRow>(
          "SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash, uploader_user_id, created_at AS uploaded_at, expire_at, download_count, file_path FROM group_files WHERE file_id = ?1"
      )
      .bind(file_id)
      .fetch_optional(&self.pool)
      .await
  }

  pub async fn delete_group_file(
      &self,
      file_id: &str,
  ) -> Result<bool, sqlx::Error> {
      let result = sqlx::query("DELETE FROM group_files WHERE file_id = ?1")
          .bind(file_id)
          .execute(&self.pool)
          .await?;
      Ok(result.rows_affected() > 0)
  }

  pub async fn increment_download_count(
      &self,
      file_id: &str,
  ) -> Result<(), sqlx::Error> {
      sqlx::query("UPDATE group_files SET download_count = download_count + 1 WHERE file_id = ?1")
          .bind(file_id)
          .execute(&self.pool)
          .await?;
      Ok(())
  }
  ```

- [ ] **Step 7: Register commands in lib.rs**

  Add `group::upload_group_file`, `group::download_group_file`, `group::delete_group_file` to `invoke_handler!`.

- [ ] **Step 8: Compile check**

  ```bash
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

- [ ] **Step 9: Commit**

  ```bash
  git add src-tauri/src/models/entities.rs src-tauri/src/persistence/repo/group/ src-tauri/src/services/group/content.rs src-tauri/src/commands/chat/group.rs src-tauri/src/utils.rs src-tauri/src/lib.rs
  git commit -m "feat(group): add file upload/download/delete backend"
  ```

---

### Task 6: Build Group File Frontend

**Files:**
- Modify: `src/types/group.ts`
- Create: `src/components/chat/group-files.tsx`
- Modify: `src/lib/mutations.ts`

- [ ] **Step 1: Update TypeScript types**

  In `src/types/group.ts`, add to `GroupFileEntity`:

  ```typescript
  export interface GroupFileEntity {
    file_id: string;
    group_id: string;
    parent_folder_id: string;
    file_name: string;
    file_size: number;
    file_hash: string;
    uploader_user_id: string;
    uploaded_at: number;
    expire_at: number | null;
    download_count: number;
    file_path?: string | null;  // <-- new
  }
  ```

- [ ] **Step 2: Add mutations**

  In `src/lib/mutations.ts`, add:

  ```typescript
  export function useUploadGroupFileMutation() {
    return useMutation({
      mutationFn: ({
        userId,
        groupId,
        parentFolderId,
      }: {
        userId: string;
        groupId: string;
        parentFolderId?: string;
      }) =>
        invoke<GroupFileEntity>("upload_group_file", {
          userId,
          groupId,
          parentFolderId: parentFolderId ?? null,
        }),
    });
  }

  export function useDeleteGroupFileMutation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ userId, fileId }: { userId: string; fileId: string }) =>
        invoke("delete_group_file", { userId, fileId }),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["group_files"] });
      },
    });
  }
  ```

- [ ] **Step 3: Build GroupFiles component**

  Create `src/components/chat/group-files.tsx`:

  ```tsx
  import { Upload, Download, Trash2, FileText } from "lucide-react";
  import { Button } from "@/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
  import type { GroupFileEntity } from "@/types/group";

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
  }

  interface GroupFilesProps {
    userId: string;
    groupId: string;
    files: GroupFileEntity[];
    onUpload: () => void;
    onDownload: (fileId: string) => void;
    onDelete: (fileId: string) => void;
    isUploading: boolean;
  }

  export function GroupFiles({
    userId,
    groupId,
    files,
    onUpload,
    onDownload,
    onDelete,
    isUploading,
  }: GroupFilesProps) {
    return (
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="flex items-center gap-2 text-sm">
            <FileText className="size-4" />
            群文件
          </CardTitle>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={isUploading}
            onClick={onUpload}
          >
            <Upload className="mr-1 size-4" />
            {isUploading ? "上传中..." : "上传"}
          </Button>
        </CardHeader>
        <CardContent>
          {files.length === 0 ? (
            <p className="text-muted-foreground text-sm">暂无文件</p>
          ) : (
            <div className="space-y-2">
              {files.map((file) => (
                <div
                  key={file.file_id}
                  className="flex items-center justify-between rounded-lg border p-3"
                >
                  <div className="min-w-0 flex-1 space-y-1">
                    <p className="truncate text-sm font-medium">
                      {file.file_name}
                    </p>
                    <p className="text-muted-foreground text-xs">
                      {formatBytes(file.file_size)} · 下载 {file.download_count} 次
                    </p>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      type="button"
                      size="icon-xs"
                      variant="ghost"
                      onClick={() => onDownload(file.file_id)}
                    >
                      <Download className="size-4" />
                    </Button>
                    <Button
                      type="button"
                      size="icon-xs"
                      variant="ghost"
                      onClick={() => onDelete(file.file_id)}
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
    );
  }
  ```

- [ ] **Step 4: Verify build**

  ```bash
  bun run build
  ```

- [ ] **Step 5: Commit**

  ```bash
  git add src/types/group.ts src/components/chat/group-files.tsx src/lib/mutations.ts
  git commit -m "feat(group): add group file upload/download frontend"
  ```

---

### Task 7: Full Verification

- [ ] **Step 1: Run all Rust tests**

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

- [ ] **Step 2: Build frontend**

  ```bash
  bun run build
  ```

- [ ] **Step 3: Manual test**

  ```bash
  bunx tauri dev
  ```

  Verify:
  1. Logs 页面显示 protocol_packets 数据（如果有 Bot 运行产生报文）
  2. 筛选等级/时间范围正常工作
  3. 点击查看详情能展开原始 JSON
  4. 群文件上传按钮弹出文件选择器
  5. 上传后文件出现在列表中
  6. 下载按钮保存文件到选择的位置

- [ ] **Step 4: Format and final commit**

  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml
  bunx --bun @biomejs/biome check --write
  git add .
  git commit -m "style: format code"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ Logs 页面接入 protocol_packets — Tasks 1-2
- ✅ 筛选参数扩展（protocol_type, is_error, until）— Task 1
- ✅ Migration 0003 — Task 3
- ✅ tauri-plugin-dialog + assetProtocol — Task 4
- ✅ 群文件上传/下载/删除后端 — Task 5
- ✅ 群文件前端 — Task 6
- ⚠️ 群相册不在本期范围（按 spec 阶段划分）

**2. Placeholder scan:**
- ✅ `file_hash` 使用 `sha2::Sha256` 计算完整 hex（已添加 `sha2` + `hex` 依赖）
- ✅ 文件选择器使用 `dialog().file().pick_file()` / `.save_file()` 标准 async API

**3. Type consistency:**
- ✅ `GroupFileEntity` fields match between Rust and TypeScript
- ✅ `file_path` is `Option<String>` / `string | undefined`
- ✅ Packet filter fields consistent across backend/frontend
