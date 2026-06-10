import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";
import type { LogSettings, SystemLogEntry } from "@/types/log";
import { queryKeys } from "./keys";

export function useSystemLogsQuery(
  params: { before?: number; limit?: number } = {},
) {
  return useQuery({
    queryKey: queryKeys.logs.system(params),
    queryFn: () =>
      invoke<SystemLogEntry[]>("list_system_logs", {
        before: params.before ?? null,
        limit: params.limit ?? 100,
      }),
    retry: false,
    refetchOnWindowFocus: false,
  });
}

export function useLogSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.logs.settings(),
    queryFn: () => invoke<LogSettings>("get_log_settings"),
    retry: false,
    refetchOnWindowFocus: false,
  });
}

export function invalidateLogSettingsQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.logs.settings() });
}
