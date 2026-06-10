export interface SystemLogEntry {
  ts: number;
  level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  target: string;
  msg: string;
  fields?: Record<string, unknown>;
}

export interface LogSettings {
  level: string;
  retention_days: number;
}

export interface LogPage<T> {
  items: T[];
  next_cursor: number | null;
  has_more: boolean;
}
