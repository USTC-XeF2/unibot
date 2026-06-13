import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { queryClient } from "@/lib/query-client";
import type { LogSettings, SystemLogEntry } from "@/types/log";
import { queryKeys } from "./keys";

export function useSystemLogsQuery(
  params: { since?: number; before?: number; limit?: number } = {},
) {
  return useQuery({
    queryKey: queryKeys.logs.system(params),
    queryFn: () =>
      invoke<SystemLogEntry[]>(COMMANDS.listSystemLogs, {
        since: params.since ?? null,
        before: params.before ?? null,
        before_seq: null,
        limit: params.limit ?? 100,
      }),
    retry: false,
    refetchOnWindowFocus: false,
  });
}

/**
 * Cursor-paginated system logs for infinite scroll, with server-side filtering.
 *
 * The backend returns entries sorted newest-first and treats `before` as an
 * exclusive upper bound on `ts`, so the cursor for the next (older) page is the
 * `ts` of the last entry in the current page. A page shorter than `pageSize`
 * means there is no more history to load. `keyword` and `levels` are applied on
 * the backend across the full on-disk history; changing either restarts paging
 * from the newest entries because they are part of the query key.
 */
export function useSystemLogsInfiniteQuery(params: {
  pageSize?: number;
  keyword?: string;
  levels?: string[];
}) {
  const { pageSize = 100, keyword = "", levels = [] } = params;
  const trimmedKeyword = keyword.trim();

  return useInfiniteQuery({
    queryKey: queryKeys.logs.systemInfinite({
      limit: pageSize,
      keyword: trimmedKeyword,
      levels,
    }),
    initialPageParam: null as { ts: number; seq: number } | null,
    queryFn: ({ pageParam }) =>
      invoke<SystemLogEntry[]>(COMMANDS.listSystemLogs, {
        since: null,
        before: pageParam?.ts ?? null,
        before_seq: pageParam?.seq ?? null,
        limit: pageSize,
        keyword: trimmedKeyword || null,
        levels,
      }),
    getNextPageParam: (lastPage) => {
      if (lastPage.length < pageSize) return undefined;
      const oldest = lastPage[lastPage.length - 1];
      return oldest ? { ts: oldest.ts, seq: oldest.seq } : undefined;
    },
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
