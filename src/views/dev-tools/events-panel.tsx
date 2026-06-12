import { listen } from "@tauri-apps/api/event";
import { Play, Trash2 } from "lucide-react";
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

export function EventsPanel() {
  const [events, setEvents] = useState<EventItem[]>([]);
  const [paused, setPaused] = useState(false);
  const [kindFilter, setKindFilter] = useState("");
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
      !kindFilter ||
      item.payload.event.kind.toLowerCase().includes(kindFilter.toLowerCase()),
  );

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-3">
        <Input
          placeholder="按 kind 过滤..."
          value={kindFilter}
          onChange={(e) => setKindFilter(e.target.value)}
          className="w-56"
        />
        <div className="flex items-center gap-2">
          <Switch checked={paused} onCheckedChange={setPaused} id="pause" />
          <label htmlFor="pause" className="text-sm">
            {paused ? "已暂停" : "暂停"}
          </label>
        </div>
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

      <div className="flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
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
