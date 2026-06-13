# 开发者工具 UI 重设计实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一开发者工具窗口四个面板的布局与交互，修复滚动、结果展示、焦点样式、筛选器体验问题。

**Architecture:** 所有面板统一为“头部工具栏 + 可滚动内容区 + 可选底部栏”的 flex 骨架；日志/事件流筛选器复用主界面日志的 Combobox 多选组件；SQL 面板改为上下分栏，底部固定结果区。

**Tech Stack:** React, Tailwind CSS, shadcn/ui, @base-ui/react Combobox, Tauri 前端。

---

## 文件结构

- `src/views/dev-tools/logs-panel.tsx`：日志面板，改用通用骨架 + Combobox 筛选 + 固定分页。
- `src/views/dev-tools/events-panel.tsx`：事件流面板，改用通用骨架 + Combobox kind 筛选。
- `src/views/dev-tools/schema-panel.tsx`：数据库面板，改用稳定双栏滚动布局。
- `src/views/dev-tools/sql-panel.tsx`：SQL 面板，改为上下分栏，底部结果面板。
- `src/components/ui/tabs.tsx`：已修复 inactive 隐藏，本计划不再改动。
- `src/views/main/logs.tsx`：参考主界面日志的 Combobox 用法。

---

### Task 1: 重构日志面板

**Files:**
- Modify: `src/views/dev-tools/logs-panel.tsx`
- Read: `src/views/main/logs.tsx`（参考 Combobox 用法）

- [ ] **Step 1: 备份当前文件并清空，写入新结构**

`src/views/dev-tools/logs-panel.tsx` 完整替换为：

```tsx
import { ChevronLeft, ChevronRight, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Combobox,
  ComboboxChip,
  ComboboxChips,
  ComboboxChipsInput,
  ComboboxContent,
  ComboboxItem,
  ComboboxList,
  useComboboxAnchor,
} from "@/components/ui/combobox";
import { useSystemLogsQuery } from "@/lib/query";
import type { SystemLogEntry } from "@/types/log";

const PAGE_SIZE = 100;

const LEVEL_OPTIONS = [
  { value: "trace", label: "TRACE" },
  { value: "debug", label: "DEBUG" },
  { value: "info", label: "INFO" },
  { value: "warn", label: "WARN" },
  { value: "error", label: "ERROR" },
] as const;

type LevelValue = (typeof LEVEL_OPTIONS)[number]["value"];

function levelColorClass(level: string): string {
  const lower = level.toLowerCase();
  if (lower === "error") return "text-destructive";
  if (lower === "warn") return "text-amber-600";
  if (lower === "info") return "text-sky-600";
  if (lower === "debug") return "text-violet-600";
  if (lower === "trace") return "text-slate-400";
  return "text-muted-foreground";
}

function levelBadgeClass(level: string): string {
  const lower = level.toLowerCase();
  if (lower === "error") return "border-destructive/30 bg-destructive/10";
  if (lower === "warn") return "border-amber-500/30 bg-amber-500/10";
  if (lower === "info") return "border-sky-500/30 bg-sky-500/10";
  if (lower === "debug") return "border-violet-500/30 bg-violet-500/10";
  if (lower === "trace") return "border-slate-400/30 bg-slate-400/10";
  return "border-border bg-muted/40";
}

function tsToShort(ts: number): string {
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  const now = new Date();
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  if (sameDay) {
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function formatLogMessage(entry: SystemLogEntry): string {
  if (entry.msg) return entry.msg;
  if (!entry.fields) return "";
  const fields = entry.fields;
  if (typeof fields.message === "string" && fields.message) return fields.message;
  if (typeof fields.summary === "string") {
    if (typeof fields.query === "string" && fields.query !== fields.summary) {
      return `${fields.summary}\n${fields.query}`;
    }
    return fields.summary;
  }
  if (typeof fields.query === "string") return fields.query;
  const entries = Object.entries(fields);
  if (entries.length === 0) return "";
  if (entries.length <= 3) {
    return entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
  }
  return `{${entries.length} fields}`;
}

function LevelCombobox({
  value,
  onValueChange,
}: {
  value: LevelValue[];
  onValueChange: (value: LevelValue[]) => void;
}) {
  const anchorRef = useComboboxAnchor();

  return (
    <Combobox
      multiple
      value={value}
      onValueChange={(nextValue) => onValueChange(nextValue as LevelValue[])}
    >
      <ComboboxChips
        ref={anchorRef}
        className="scrollbar-none h-8 flex-nowrap overflow-x-auto overflow-y-hidden whitespace-nowrap"
      >
        {value.map((selected) => {
          const option = LEVEL_OPTIONS.find((o) => o.value === selected);
          return (
            <ComboboxChip key={selected} className="shrink-0">
              {option?.label ?? selected}
            </ComboboxChip>
          );
        })}
        <ComboboxChipsInput
          placeholder={value.length === 0 ? "等级" : ""}
          className="min-w-0 shrink-0"
        />
      </ComboboxChips>

      <ComboboxContent anchor={anchorRef}>
        <ComboboxList>
          {LEVEL_OPTIONS.map((option) => (
            <ComboboxItem key={option.value} value={option.value}>
              {option.label}
            </ComboboxItem>
          ))}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  );
}

export function LogsPanel() {
  const logsQuery = useSystemLogsQuery({ limit: 500 });
  const [keyword, setKeyword] = useState("");
  const [levels, setLevels] = useState<LevelValue[]>([]);
  const [page, setPage] = useState(1);

  const filtered = useMemo(() => {
    const entries = logsQuery.data ?? [];
    const lower = keyword.trim().toLowerCase();
    return entries.filter((entry) => {
      if (levels.length > 0 && !levels.includes(entry.level.toLowerCase() as LevelValue)) {
        return false;
      }
      if (!lower) return true;
      const text = JSON.stringify(entry).toLowerCase();
      return text.includes(lower);
    });
  }, [logsQuery.data, keyword, levels]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const safePage = Math.min(page, totalPages);
  const paginated = useMemo(() => {
    const start = (safePage - 1) * PAGE_SIZE;
    return filtered.slice(start, start + PAGE_SIZE);
  }, [filtered, safePage]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <div className="shrink-0 rounded-xl border bg-card/60 p-3">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="搜索关键字..."
              value={keyword}
              onChange={(e) => {
                setKeyword(e.target.value);
                setPage(1);
              }}
              className="pl-8"
            />
          </div>
          <div className="w-40">
            <LevelCombobox
              value={levels}
              onValueChange={(value) => {
                setLevels(value);
                setPage(1);
              }}
            />
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
        {logsQuery.isPending ? (
          <p className="text-muted-foreground text-sm">读取中...</p>
        ) : logsQuery.isError ? (
          <p className="text-destructive text-sm">读取失败</p>
        ) : paginated.length === 0 ? (
          <p className="text-muted-foreground text-sm">无匹配日志</p>
        ) : (
          <div className="space-y-2">
            {paginated.map((entry) => (
              <div
                key={`${entry.ts}-${entry.target}-${entry.msg}`}
                className="rounded-lg border bg-card px-3 py-2 text-xs"
              >
                <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
                  <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                    {tsToShort(entry.ts)}
                  </span>
                  <span
                    className={`rounded border px-1.5 py-0.5 font-medium ${levelBadgeClass(entry.level)} ${levelColorClass(entry.level)}`}
                  >
                    {entry.level.toUpperCase()}
                  </span>
                  <span className="max-w-50 truncate rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                    {entry.target}
                  </span>
                </div>
                <div className="mt-1.5 whitespace-pre-wrap text-sm leading-relaxed">
                  {formatLogMessage(entry)}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="shrink-0 flex items-center justify-between text-xs">
        <span className="text-muted-foreground">
          共 {filtered.length} 条 · 第 {safePage} / {totalPages} 页
        </span>
        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="icon"
            className="size-7"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={safePage <= 1}
          >
            <ChevronLeft className="size-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            className="size-7"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={safePage >= totalPages}
          >
            <ChevronRight className="size-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 运行前端类型检查**

Run: `npm run build`
Expected: `tsc` 与 `vite build` 通过。

- [ ] **Step 3: 提交 commit**

```bash
git add src/views/dev-tools/logs-panel.tsx
git commit -m "refactor(dev-tools): unify logs panel layout and combobox filter"
```

---

### Task 2: 重构事件流面板

**Files:**
- Modify: `src/views/dev-tools/events-panel.tsx`

- [ ] **Step 1: 完整替换事件流面板**

`src/views/dev-tools/events-panel.tsx` 完整替换为：

```tsx
import { listen } from "@tauri-apps/api/event";
import { Play, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Combobox,
  ComboboxChip,
  ComboboxChips,
  ComboboxChipsInput,
  ComboboxContent,
  ComboboxItem,
  ComboboxList,
  useComboboxAnchor,
} from "@/components/ui/combobox";
import type { DevToolsEventPayload } from "@/types/dev-tools";

type EventItem = {
  id: number;
  receivedAt: number;
  payload: DevToolsEventPayload;
};

const KIND_OPTIONS = [
  { value: "message", label: "message" },
  { value: "poke", label: "poke" },
  { value: "notice", label: "notice" },
] as const;

function kindBadgeClass(kind: string): string {
  if (kind === "message") return "border-sky-500/30 bg-sky-500/10 text-sky-600";
  if (kind === "poke")
    return "border-violet-500/30 bg-violet-500/10 text-violet-600";
  if (kind.startsWith("group_"))
    return "border-amber-500/30 bg-amber-500/10 text-amber-600";
  if (kind === "notice")
    return "border-pink-500/30 bg-pink-500/10 text-pink-600";
  if (kind.includes("request"))
    return "border-emerald-500/30 bg-emerald-500/10 text-emerald-600";
  return "border-border bg-muted/40 text-muted-foreground";
}

function KindCombobox({
  value,
  onValueChange,
}: {
  value: string[];
  onValueChange: (value: string[]) => void;
}) {
  const anchorRef = useComboboxAnchor();

  return (
    <Combobox
      multiple
      value={value}
      onValueChange={onValueChange}
    >
      <ComboboxChips
        ref={anchorRef}
        className="scrollbar-none h-8 flex-nowrap overflow-x-auto overflow-y-hidden whitespace-nowrap"
      >
        {value.map((selected) => (
          <ComboboxChip key={selected} className="shrink-0">{selected}</ComboboxChip>
        ))}
        <ComboboxChipsInput
          placeholder={value.length === 0 ? "kind" : ""}
          className="min-w-0 shrink-0"
        />
      </ComboboxChips>

      <ComboboxContent anchor={anchorRef}>
        <ComboboxList>
          {KIND_OPTIONS.map((option) => (
            <ComboboxItem key={option.value} value={option.value}>
              {option.label}
            </ComboboxItem>
          ))}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  );
}

export function EventsPanel() {
  const [events, setEvents] = useState<EventItem[]>([]);
  const [paused, setPaused] = useState(false);
  const [kinds, setKinds] = useState<string[]>([]);
  const nextIdRef = useRef(0);
  const backlogRef = useRef<EventItem[]>([]);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    listen<DevToolsEventPayload>("devtools:event", (e) => {
      if (cancelled) return;
      const item: EventItem = {
        id: nextIdRef.current++,
        receivedAt: Date.now(),
        payload: e.payload,
      };

      if (pausedRef.current) {
        backlogRef.current.push(item);
        if (backlogRef.current.length > 1000) {
          backlogRef.current.shift();
        }
      } else {
        setEvents((prev) => [...prev, item].slice(-1000));
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (!paused && backlogRef.current.length > 0) {
      const batch = backlogRef.current;
      backlogRef.current = [];
      setEvents((prev) => [...prev, ...batch].slice(-1000));
    }
  }, [paused]);

  const filtered = events.filter(
    (item) =>
      kinds.length === 0 ||
      kinds.includes(item.payload.event.kind),
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <div className="shrink-0 flex items-center gap-3 rounded-xl border bg-card/60 p-3">
        <div className="w-40">
          <KindCombobox value={kinds} onValueChange={setKinds} />
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant={paused ? "default" : "outline"}
            size="sm"
            onClick={() => setPaused((p) => !p)}
          >
            {paused ? "继续" : "暂停"}
          </Button>

          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={() => {
              setEvents([]);
              backlogRef.current = [];
            }}
          >
            <Trash2 className="size-3.5" />
            清空
          </Button>

          {paused && backlogRef.current.length > 0 && (
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              onClick={() => setPaused(false)}
            >
              <Play className="size-3.5" />
              继续 ({backlogRef.current.length})
            </Button>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
        {filtered.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {events.length === 0 ? "暂无事件" : "无匹配事件"}
          </p>
        ) : (
          <div className="space-y-2">
            {filtered.map((item) => (
              <div
                key={item.id}
                className="rounded-lg border bg-card px-3 py-2 text-xs"
              >
                <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
                  <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                    {new Date(item.receivedAt).toLocaleTimeString()}
                  </span>
                  <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                    {item.payload.recipient_user_id}
                  </span>
                  <span
                    className={`rounded border px-1.5 py-0.5 font-medium ${kindBadgeClass(item.payload.event.kind)}`}
                  >
                    {item.payload.event.kind}
                  </span>
                </div>
                <pre className="mt-1.5 max-h-32 overflow-auto rounded bg-muted/50 p-2 text-xs">
                  {JSON.stringify(item.payload.event, null, 2)}
                </pre>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 运行前端类型检查**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 3: 提交 commit**

```bash
git add src/views/dev-tools/events-panel.tsx
git commit -m "refactor(dev-tools): unify events panel layout and combobox filter"
```

---

### Task 3: 重构数据库面板

**Files:**
- Modify: `src/views/dev-tools/schema-panel.tsx`

- [ ] **Step 1: 替换 SchemaPanel 根布局及左右容器**

找到 `export function SchemaPanel()` 及其返回的 JSX，完整替换为：

```tsx
export function SchemaPanel() {
  const schemaQuery = useDbSchemaQuery();
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  const selected = schemaQuery.data?.tables.find(
    (t) => t.name === selectedTable,
  );

  return (
    <div className="flex h-full min-h-0 gap-4">
      <div className="flex h-full w-56 flex-col overflow-hidden rounded-xl border bg-card">
        <div className="flex-1 overflow-auto">
          {schemaQuery.isPending ? (
            <p className="p-3 text-muted-foreground text-sm">读取中...</p>
          ) : schemaQuery.isError ? (
            <p className="p-3 text-destructive text-sm">读取失败</p>
          ) : schemaQuery.data?.tables.length === 0 ? (
            <p className="p-3 text-muted-foreground text-sm">无表</p>
          ) : (
            <ul className="divide-y">
              {schemaQuery.data?.tables.map((table) => (
                <li key={table.name}>
                  <button
                    type="button"
                    className={`block w-full cursor-pointer px-3 py-2 text-left text-sm ${
                      selectedTable === table.name
                        ? "bg-muted font-medium"
                        : "hover:bg-muted/50"
                    }`}
                    onClick={() => setSelectedTable(table.name)}
                  >
                    {table.name}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-4">
        {selected ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="font-semibold text-lg">{selected.name}</h2>
            </div>
            <TableDetail table={selected} />
            <RowPreview tableName={selected.name} />
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">选择一个表查看结构</p>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 给 TableDetail 内的表格和 DDL 加滚动容器**

在 `TableDetail` 中：
- 列表格外层 `div` 已经是 `overflow-auto rounded-xl border bg-card`，保持不变。
- DDL 的 `pre` 已经是 `overflow-auto`，保持不变。
- 索引列表较短，无需额外滚动容器。

- [ ] **Step 3: 给 RowPreview 表格加滚动容器**

`RowPreview` 返回的 `section` 中，表格外层 `div` 已经是 `overflow-auto rounded-xl border bg-card`，保持不变。

- [ ] **Step 4: 运行前端类型检查**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 5: 提交 commit**

```bash
git add src/views/dev-tools/schema-panel.tsx
git commit -m "refactor(dev-tools): make schema panel columns independently scrollable"
```

---

### Task 4: 重构 SQL 面板

**Files:**
- Modify: `src/views/dev-tools/sql-panel.tsx`

- [ ] **Step 1: 完整替换 SQL 面板**

`src/views/dev-tools/sql-panel.tsx` 完整替换为：

```tsx
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { confirmDialog } from "@/lib/modal";
import { checkWriteQuery, useExecuteSqlMutation } from "@/lib/query";
import type { SqlQueryResult } from "@/types/dev-tools";

export function SqlPanel() {
  const [query, setQuery] = useState("SELECT * FROM im_accounts LIMIT 10");
  const [allowWrite, setAllowWrite] = useState(false);
  const [result, setResult] = useState<SqlQueryResult | null>(null);
  const execute = useExecuteSqlMutation();

  const handleExecute = async () => {
    const trimmed = query.trim();
    if (!trimmed) {
      toast.error("SQL 为空");
      return;
    }

    const isWrite = await checkWriteQuery(trimmed);

    if (isWrite && !allowWrite) {
      toast.error("写操作需在上方开启“允许写操作”");
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
          setResult(data);
          toast.success("执行成功");
        },
        onError: (err) => toast.error(`执行失败: ${err}`),
      },
    );
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <div className="shrink-0 rounded-xl border bg-card/60 p-3">
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
            {execute.isPending ? "执行中..." : "执行"}
          </Button>
        </div>

        <Textarea
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="mt-3 min-h-24 font-mono text-sm"
          placeholder="输入 SQL..."
        />
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
        {execute.isError && (
          <div className="mb-3 rounded border border-red-200 bg-red-50 p-2 text-xs text-red-700">
            执行失败: {execute.error?.message || "未知错误"}
          </div>
        )}

        {!result && !execute.isError && (
          <p className="text-muted-foreground text-sm">执行 SQL 后在此查看结果</p>
        )}

        {result && (
          <div className="h-full">
            {result.rows.length === 0 ? (
              <p className="text-muted-foreground text-sm">
                {result.rows_affected !== undefined
                  ? `受影响行数: ${result.rows_affected}`
                  : "无返回数据"}
              </p>
            ) : (
              <div className="overflow-auto">
                <table className="w-full text-xs">
                  <thead className="sticky top-0 bg-muted">
                    <tr>
                      {result.columns.map((col) => (
                        <th
                          key={col}
                          className="px-3 py-2 text-left font-medium"
                        >
                          {col}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {result.rows.map((row) => (
                      <tr key={row.map(String).join("|")}>
                        {row.map((cell, cidx) => {
                          const colName = result.columns[cidx];
                          return (
                            <td
                              key={`${colName}-${String(cell)}`}
                              className="px-3 py-2"
                            >
                              {cell === null ? "NULL" : String(cell)}
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 运行前端类型检查**

Run: `npm run build`
Expected: 通过。

- [ ] **Step 3: 提交 commit**

```bash
git add src/views/dev-tools/sql-panel.tsx
git commit -m "refactor(dev-tools): split sql panel into fixed input and scrollable result pane"
```

---

### Task 5: 最终验证与收尾

**Files:**
- Modify: 无新增文件

- [ ] **Step 1: 确认 TabsContent 修复仍在**

检查 `src/components/ui/tabs.tsx` 中 `TabsContent` 的 className 包含 `data-[state=inactive]:hidden`。
若不存在，按此前提交补齐。

- [ ] **Step 2: 运行完整前端构建**

Run: `npm run build`
Expected: `tsc` 与 `vite build` 通过，无新增类型错误。

- [ ] **Step 3: 运行 Rust 相关测试**

Run: `cd src-tauri && cargo test --lib commands::dev_tools`
Expected: 13 passed。

- [ ] **Step 4: 推送到 PR 分支**

```bash
git push origin feat/developer-mode-pr2
```

---

## Spec Coverage 自查

| 设计点 | 对应 Task |
|--------|-----------|
| 日志滚动 + 分页固定 | Task 1 |
| 日志筛选器改为 Combobox | Task 1 |
| 事件流统一骨架 + kind Combobox | Task 2 |
| 数据库左右栏独立滚动 | Task 3 |
| SQL 结果底部面板 | Task 4 |
| 焦点样式复用默认组件 | Task 1-4 全部移除局部 ring/shadow |
| TabsContent inactive 隐藏 | Task 5 确认 |

## 无占位符检查

- 所有 Task 均给出完整文件路径。
- 所有替换代码均为可直接写入的完整组件。
- 所有验证命令与预期结果明确。

## 执行方式

计划已保存到 `docs/superpowers/plans/2026-06-13-dev-tools-ui-redesign.md`。

两种执行方式：

1. **Subagent-Driven（推荐）**：每个 Task 派一个独立子代理实现并自测，完成后我统一 review。
2. **Inline Execution**：我在当前会话按 Task 顺序直接实现。

选哪种？
