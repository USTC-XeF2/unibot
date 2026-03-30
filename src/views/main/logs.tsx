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

type LogLevel = "debug" | "info" | "warn" | "error";
type EventType = "message" | "request" | "system" | "group" | "connection";

type LogEntry = {
  id: string;
  time: string | null;
  level: LogLevel;
  eventType: EventType;
  source: string;
  message: string;
};

const levelOptions: Array<{ value: LogLevel; label: string }> = [
  { value: "debug", label: "DEBUG" },
  { value: "info", label: "INFO" },
  { value: "warn", label: "WARN" },
  { value: "error", label: "ERROR" },
];

const eventOptions: Array<{ value: EventType; label: string }> = [
  { value: "message", label: "消息" },
  { value: "request", label: "请求" },
  { value: "system", label: "系统" },
  { value: "group", label: "群组" },
  { value: "connection", label: "连接" },
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
  if (level === "error") {
    return "text-destructive";
  }
  if (level === "warn") {
    return "text-amber-600";
  }
  if (level === "info") {
    return "text-sky-600";
  }
  return "text-muted-foreground";
}

function levelBadgeClass(level: LogLevel) {
  if (level === "error") {
    return "border-destructive/30 bg-destructive/10";
  }
  if (level === "warn") {
    return "border-amber-500/30 bg-amber-500/10";
  }
  if (level === "info") {
    return "border-sky-500/30 bg-sky-500/10";
  }
  return "border-border bg-muted/40";
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
      <ComboboxChips ref={anchorRef}>
        {value.map((selected) => {
          const selectedOption = options.find(
            (option) => option.value === selected,
          );
          return (
            <ComboboxChip key={selected}>
              {selectedOption?.label ?? selected}
            </ComboboxChip>
          );
        })}
        <ComboboxChipsInput placeholder={placeholder} />
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

function LogsView() {
  const [levels, setLevels] = useState<LogLevel[]>([]);
  const [eventTypes, setEventTypes] = useState<EventType[]>([]);
  const [timeRange, setTimeRange] = useState<
    "all" | "15m" | "1h" | "24h" | "7d"
  >("24h");

  const logs = useMemo<LogEntry[]>(() => [], []);

  const filteredLogs = useMemo(() => {
    const now = Date.now();
    const windowMsMap: Record<"15m" | "1h" | "24h" | "7d", number> = {
      "15m": 15 * 60 * 1000,
      "1h": 60 * 60 * 1000,
      "24h": 24 * 60 * 60 * 1000,
      "7d": 7 * 24 * 60 * 60 * 1000,
    };

    return logs.filter((log) => {
      if (levels.length > 0 && !levels.includes(log.level)) {
        return false;
      }
      if (eventTypes.length > 0 && !eventTypes.includes(log.eventType)) {
        return false;
      }
      if (timeRange !== "all") {
        if (!log.time) {
          return false;
        }
        const time = new Date(log.time).getTime();
        if (Number.isNaN(time) || now - time > windowMsMap[timeRange]) {
          return false;
        }
      }
      return true;
    });
  }, [eventTypes, levels, logs, timeRange]);

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
              <SelectTrigger className="w-full">
                <SelectValue placeholder="选择时间范围" />
              </SelectTrigger>
              <SelectContent>
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
        </div>

        <div className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
          {filteredLogs.map((log) => (
            <div
              key={log.id}
              className="rounded-lg border bg-card px-3 py-2 text-xs"
            >
              <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
                <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                  {log.time || "-"}
                </span>
                <span
                  className={`rounded border px-1.5 py-0.5 font-medium ${levelBadgeClass(log.level)} ${levelColor(log.level)}`}
                >
                  {log.level.toUpperCase()}
                </span>
                <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-muted-foreground">
                  {log.eventType}
                </span>
                <span className="text-muted-foreground">{log.source}</span>
              </div>
              <p className="mt-1.5 break-all text-sm leading-relaxed">
                {log.message}
              </p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default LogsView;
