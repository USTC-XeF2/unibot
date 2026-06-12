import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type {
  DbSchema,
  SqlQueryResult,
  TableRowPreview,
} from "@/types/dev-tools";

export function useDbSchemaQuery() {
  return useQuery({
    queryKey: queryKeys.devTools.schema(),
    queryFn: () => invoke<DbSchema>("get_db_schema"),
    retry: false,
  });
}

export function invalidateDevToolsSchemaQuery() {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.devTools.schema(),
  });
}

export function useOpenDeveloperToolsMutation() {
  return useMutation({
    mutationFn: () => invoke<boolean>("open_developer_tools"),
  });
}

export function useExecuteSqlMutation() {
  return useMutation({
    mutationFn: ({
      query,
      allowWrite,
    }: {
      query: string;
      allowWrite: boolean;
    }) =>
      invoke<SqlQueryResult>("execute_sql", {
        query,
        allowWrite,
      }),
  });
}

export async function checkWriteQuery(query: string): Promise<boolean> {
  return invoke<boolean>("is_write_query", { query });
}

export function useTableRowPreviewQuery(table: string | null, limit = 50) {
  return useQuery({
    queryKey: queryKeys.devTools.previewRows(table ?? "", limit),
    queryFn: () =>
      invoke<TableRowPreview>("preview_table_rows", { table, limit }),
    enabled: !!table,
    retry: false,
  });
}
