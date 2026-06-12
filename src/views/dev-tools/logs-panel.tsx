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

function levelColorClass(level: string): string {
  const lower = level.toLowerCase();
  if (lower === "error") return "text-destructive";
  if (lower === "warn") return "text-amber-600";
  if (lower === "info") return "text-sky-600";
  if (lower === "debug") return "text-violet-600";
  if (lower === "trace") return "text-slate-400";
  return "text-muted-foreground";
}

function tsToShort(ts: number): string {
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export function LogsPanel() {
  const logsQuery = useSystemLogsQuery({ limit: 500 });
  const [keyword, setKeyword] = useState("");
  const [level, setLevel] = useState("all");

  const filtered = useMemo(() => {
    const entries = logsQuery.data ?? [];
    const lower = keyword.trim().toLowerCase();
    return entries.filter((entry) => {
      if (level !== "all" && entry.level.toLowerCase() !== level) return false;
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
            {filtered.map((entry) => (
              <tr
                key={`${entry.ts}-${entry.target}-${entry.msg}`}
                className="border-t"
              >
                <td className="whitespace-nowrap px-2 py-1">
                  {tsToShort(entry.ts)}
                </td>
                <td
                  className={`px-2 py-1 font-medium ${levelColorClass(entry.level)}`}
                >
                  {entry.level}
                </td>
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
