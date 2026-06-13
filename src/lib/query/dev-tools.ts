import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
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
    queryFn: () => invoke<DbSchema>(COMMANDS.getDbSchema),
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
    mutationFn: () => invoke<boolean>(COMMANDS.openDeveloperTools),
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
      invoke<SqlQueryResult>(COMMANDS.executeSql, {
        query,
        allowWrite,
      }),
  });
}

export async function checkWriteQuery(query: string): Promise<boolean> {
  return invoke<boolean>(COMMANDS.isWriteQueryCommand, { query });
}

export function useTableRowPreviewQuery(table: string | null, limit = 50) {
  return useQuery({
    queryKey: queryKeys.devTools.previewRows(table ?? "", limit),
    queryFn: () =>
      invoke<TableRowPreview>(COMMANDS.previewTableRows, { table, limit }),
    enabled: !!table,
    retry: false,
  });
}
