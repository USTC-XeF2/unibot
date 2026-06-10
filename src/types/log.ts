export interface SystemLogEntry {
  ts: number;
  level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  target: string;
  msg: string;
  fields?: Record<string, unknown> | null;
}

export interface LogSettings {
  level: string;
  retention_days: number;
}

// TODO: Phase 1 does not enable cursor pagination; keep the type for Phase 2.
export interface LogPage<T> {
  items: T[];
  next_cursor: number | null;
  has_more: boolean;
}
