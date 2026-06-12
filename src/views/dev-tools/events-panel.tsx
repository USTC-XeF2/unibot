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
                <td className="whitespace-nowrap px-2 py-1">
                  {new Date(item.receivedAt).toLocaleTimeString()}
                </td>
                <td className="px-2 py-1">{item.payload.recipient_user_id}</td>
                <td className="px-2 py-1">{item.payload.event.kind}</td>
                <td className="max-w-md truncate px-2 py-1">
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
