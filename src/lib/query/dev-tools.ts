import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { DbSchema } from "@/types/dev-tools";

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
