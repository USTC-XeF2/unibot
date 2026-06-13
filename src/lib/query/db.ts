import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { queryClient } from "@/lib/query-client";
import type { DbStatus } from "@/types/db";
import { queryKeys } from "./keys";

export function useDbStatusQuery() {
  return useQuery({
    queryKey: queryKeys.db.status(),
    queryFn: () => invoke<DbStatus>(COMMANDS.getDbStatus),
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
}

export function invalidateDbStatusQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.db.status() });
}
