import { ScrollText } from "lucide-react";
import { useMemo, useState } from "react";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useProtocolPackets } from "@/lib/query";
import { useSystemLogsQuery } from "@/lib/query/logs";

type LogLevel = "trace" | "debug" | "info" | "warn" | "error";
type EventType = "system" | "protocol" | "message" | "request" | "group";

interface LogEntry {
  id: string;
  time: number;
  timeLabel: string;
  level: LogLevel;
  eventType: EventType;
  source: string;
  message: string;
}

const levelOptions: Array<{ value: LogLevel; label: string }> = [
  { value: "trace", label: "TRACE" },
  { value: "debug", label: "DEBUG" },
  { value: "info", label: "INFO" },
  { value: "warn", label: "WARN" },
  { value: "error", label: "ERROR" },
];

const eventOptions: Array<{ value: EventType; label: string }> = [
  { value: "system", label: "系统" },
  { value: "protocol", label: "协议" },
  { value: "message", label: "消息" },
  { value: "request", label: "请求" },
  { value: "group", label: "群组" },
];

const rangeOptions: Array<{
  value: "all" | "15m" | "1h" | "24h" | "7d";
  label: string;
}> = [
  { value: "all", label: "全部时间" },
  { value: "15m", label: "最近 15 分钟" },
  { value: "1h", label: "最近 1 小时" },
  { value: "24h", label: "最近 24 小时" },
  { value: "7d", label: "最近 7 天" },
];

function levelColor(level: LogLevel) {
  if (level === "error") return "text-destructive";
  if (level === "warn") return "text-amber-600";
  if (level === "info") return "text-sky-600";
  if (level === "debug") return "text-violet-600";
  if (level === "trace") return "text-slate-400";
  return "text-muted-foreground";
}

function levelBadgeClass(level: LogLevel) {
  if (level === "error") return "border-destructive/30 bg-destructive/10";
  if (level === "warn") return "border-amber-500/30 bg-amber-500/10";
  if (level === "info") return "border-sky-500/30 bg-sky-500/10";
  if (level === "debug") return "border-violet-500/30 bg-violet-500/10";
  if (level === "trace") return "border-slate-400/30 bg-slate-400/10";
  return "border-border bg-muted/40";
}

/** Show time compactly: HH:MM:SS for today, MM-DD HH:MM otherwise */
function tsToShort(ts: number): string {
  const now = new Date();
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  const sameYear = d.getFullYear() === now.getFullYear();
  const sameDay =
    sameYear &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  if (sameDay) {
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Format a tracing log message, falling back to fields when msg is empty. */
function formatLogMessage(
  msg: string,
  fields?: Record<string, unknown>,
): string {
  if (msg) return msg;
  if (!fields) return "";

  // Some events put the message in fields.message
  const fieldsMsg = fields.message;
  if (typeof fieldsMsg === "string" && fieldsMsg) {
    return fieldsMsg;
  }

  // sqlx::query style: summary + query
  const summary = fields.summary;
  const query = fields.query;
  if (typeof summary === "string") {
    if (typeof query === "string" && query !== summary) {
      return `${summary}\n${query}`;
    }
    return summary;
  }
  if (typeof query === "string") return query;

  // Generic: show key: value for simple fields (≤3)
  const entries = Object.entries(fields).filter(([k]) => k !== "message");
  if (entries.length === 0) return "";
  if (entries.length <= 3) {
    return entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
  }

  // Too many fields → summary
  return `{${entries.length} fields}`;
}

type MultiSelectComboboxProps<T extends string> = {
  value: T[];
  onValueChange: (value: T[]) => void;
  options: Array<{ value: T; label: string }>;
  placeholder: string;
};

function MultiSelectCombobox<T extends string>({
  value,
  onValueChange,
  options,
  placeholder,
}: MultiSelectComboboxProps<T>) {
  const anchorRef = useComboboxAnchor();

  return (
    <Combobox
      multiple
      value={value}
      onValueChange={(nextValue) => onValueChange(nextValue as T[])}
    >
      <ComboboxChips
        ref={anchorRef}
        className="scrollbar-none overflow-x-auto overflow-y-hidden whitespace-nowrap flex-nowrap h-8"
      >
        {value.map((selected) => {
          const selectedOption = options.find(
            (option) => option.value === selected,
          );
          return (
            <ComboboxChip key={selected} className="shrink-0">
              {selectedOption?.label ?? selected}
            </ComboboxChip>
          );
        })}
        <ComboboxChipsInput
          placeholder={value.length === 0 ? placeholder : ""}
          className="shrink-0 min-w-0"
        />
      </ComboboxChips>

      <ComboboxContent anchor={anchorRef}>
        <ComboboxList>
          {options.map((option) => (
            <ComboboxItem key={option.value} value={option.value}>
              {option.label}
            </ComboboxItem>
          ))}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  );
}

function useLogEntries() {
  const systemQuery = useSystemLogsQuery({ limit: 500 });
  const protocolQuery = useProtocolPackets({ limit: 500 });

  const systemEntries = useMemo<LogEntry[]>(() => {
    if (!systemQuery.data) return [];
    return systemQuery.data.map((log, i) => {
      const level = log.level.toLowerCase() as LogLevel;
      return {
        id: `sys-${log.ts}-${i}`,
        time: log.ts,
        timeLabel: tsToShort(log.ts),
        level,
        eventType: "system" as EventType,
        source: log.target,
        message: formatLogMessage(log.msg, log.fields ?? undefined),
      };
    });
  }, [systemQuery.data]);

  const protocolEntries = useMemo<LogEntry[]>(() => {
    if (!protocolQuery.data) return [];
    return protocolQuery.data.map((pkt) => {
      const level: LogLevel = pkt.is_error ? "error" : "info";
      return {
        id: `pkt-${pkt.packet_id}`,
        time: pkt.created_at,
        timeLabel: tsToShort(pkt.created_at),
        level,
        eventType: "protocol" as EventType,
        source: (pkt.bot_id ?? "system").slice(0, 8),
        message: `${pkt.direction === "receive" ? "←" : "→"} ${pkt.action_name}`,
      };
    });
  }, [protocolQuery.data]);

  const isLoading = systemQuery.isLoading || protocolQuery.isLoading;

  const entries = useMemo<LogEntry[]>(() => {
    const merged = [...systemEntries, ...protocolEntries];
    merged.sort(
      (a, b) => new Date(b.time).getTime() - new Date(a.time).getTime(),
    );
    return merged;
  }, [systemEntries, protocolEntries]);

  return { entries, isLoading };
}

/** Single log row. Short messages show inline; long ones get an expand button. */
function LogRow({ log }: { log: LogEntry }) {
  const [expanded, setExpanded] = useState(false);
  const MAX_CHARS = 200;
  const hasNewlines = log.message.includes("\n");
  const isLong = log.message.length > MAX_CHARS || hasNewlines;

  return (
    <div className="rounded-lg border bg-card px-3 py-2 text-xs">
      <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
        <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
          {log.timeLabel}
        </span>
        <span
          className={`rounded border px-1.5 py-0.5 font-medium ${levelBadgeClass(log.level)} ${levelColor(log.level)}`}
        >
          {log.level.toUpperCase()}
        </span>
        <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
          {log.eventType}
        </span>
        <span className="max-w-50 truncate rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
          {log.source}
        </span>
      </div>
      <div className="mt-1.5 text-sm leading-relaxed">
        {isLong ? (
          <>
            <div className={!expanded ? "line-clamp-2" : ""}>
              <span className="whitespace-pre-wrap">
                {!expanded && log.message.length > MAX_CHARS
                  ? `${log.message.slice(0, MAX_CHARS)}…`
                  : log.message}
              </span>
            </div>
            {!expanded ? (
              <button
                type="button"
                className="mt-0.5 cursor-pointer text-muted-foreground text-xs hover:text-primary"
                onClick={() => setExpanded(true)}
              >
                展开
              </button>
            ) : (
              <button
                type="button"
                className="mt-0.5 cursor-pointer text-muted-foreground text-xs hover:text-primary"
                onClick={() => setExpanded(false)}
              >
                收起
              </button>
            )}
          </>
        ) : (
          <span className="whitespace-pre-wrap">{log.message}</span>
        )}
      </div>
    </div>
  );
}

function LogsView() {
  const [levels, setLevels] = useState<LogLevel[]>([]);
  const [eventTypes, setEventTypes] = useState<EventType[]>([]);
  const [timeRange, setTimeRange] = useState<
    "all" | "15m" | "1h" | "24h" | "7d"
  >("24h");

  const { entries: allEntries, isLoading } = useLogEntries();

  const filteredLogs = useMemo(() => {
    const now = Date.now();
    const windowMsMap: Record<"15m" | "1h" | "24h" | "7d", number> = {
      "15m": 15 * 60 * 1000,
      "1h": 60 * 60 * 1000,
      "24h": 24 * 60 * 60 * 1000,
      "7d": 7 * 24 * 60 * 60 * 1000,
    };

    return allEntries.filter((log) => {
      if (levels.length > 0 && !levels.includes(log.level)) {
        return false;
      }
      if (eventTypes.length > 0 && !eventTypes.includes(log.eventType)) {
        return false;
      }
      if (timeRange !== "all") {
        if (now - log.time > windowMsMap[timeRange]) {
          return false;
        }
      }
      return true;
    });
  }, [allEntries, eventTypes, levels, timeRange]);

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
            <MultiSelectCombobox
              value={levels}
              onValueChange={setLevels}
              options={levelOptions}
              placeholder="选择等级"
            />
          </div>

          <div className="space-y-1 text-xs">
            <span className="text-muted-foreground">时间范围</span>
            <Select
              value={timeRange}
              onValueChange={(value) =>
                setTimeRange(value as "all" | "15m" | "1h" | "24h" | "7d")
              }
            >
              <SelectTrigger className="h-8 w-full min-w-30">
                <SelectValue placeholder="选择时间范围" />
              </SelectTrigger>
              <SelectContent position="popper" align="start">
                {rangeOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1 text-xs">
            <span className="text-muted-foreground">事件类型</span>
            <MultiSelectCombobox
              value={eventTypes}
              onValueChange={setEventTypes}
              options={eventOptions}
              placeholder="选择事件类型"
            />
          </div>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card/60">
        <div className="border-b px-3 py-2 text-muted-foreground text-xs">
          已匹配 {filteredLogs.length} 条日志
          {isLoading && " · 加载中…"}
        </div>

        <div className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
          {filteredLogs.map((log) => (
            <LogRow key={log.id} log={log} />
          ))}
        </div>
      </div>
    </div>
  );
}

export default LogsView;
