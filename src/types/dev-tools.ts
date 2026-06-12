import type { InternalEventPayload } from "@/types/event";

export type DbColumn = {
  cid: number;
  name: string;
  type_name: string;
  not_null: boolean;
  default_value: string | null;
  primary_key: boolean;
};

export type DbIndex = {
  seq: number;
  name: string;
  unique: boolean;
  origin: string;
  partial: boolean;
  columns: string[];
};

export type DbTable = {
  name: string;
  sql: string | null;
  columns: DbColumn[];
  indexes: DbIndex[];
};

export type DbSchema = {
  tables: DbTable[];
};

export type TableRowPreview = {
  columns: string[];
  rows: (string | number | boolean | null)[][];
};

export type DevToolsEventPayload = {
  event: InternalEventPayload;
};
