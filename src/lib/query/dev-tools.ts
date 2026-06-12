import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import type { DbSchema, SqlQueryResult } from "@/types/dev-tools";

export function useDbSchemaQuery() {
  return useQuery({
    queryKey: queryKeys.devTools.schema(),
    queryFn: () => invoke<DbSchema>("get_db_schema"),
    retry: false,
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
