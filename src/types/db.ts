export interface DbStatus {
  schema_version: string;
  table_count: number;
  db_size_bytes: number;
  integrity_check: string;
  foreign_key_check: string[];
}
