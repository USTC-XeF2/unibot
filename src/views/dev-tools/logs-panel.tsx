import { ChevronLeft, ChevronRight, Search } from "lucide-react";
import { useMemo, useState } from "react";
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
import { Input } from "@/components/ui/input";
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
  if (typeof fields.message === "string" && fields.message)
    return fields.message;
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
      if (
        levels.length > 0 &&
        !levels.includes(entry.level.toLowerCase() as LevelValue)
      ) {
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
