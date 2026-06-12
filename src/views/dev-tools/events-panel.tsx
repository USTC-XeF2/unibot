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
    <Combobox multiple value={value} onValueChange={onValueChange}>
      <ComboboxChips
        ref={anchorRef}
        className="scrollbar-none h-8 flex-nowrap overflow-x-auto overflow-y-hidden whitespace-nowrap"
      >
        {value.map((selected) => (
          <ComboboxChip key={selected} className="shrink-0">
            {selected}
          </ComboboxChip>
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
    (item) => kinds.length === 0 || kinds.includes(item.payload.event.kind),
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
