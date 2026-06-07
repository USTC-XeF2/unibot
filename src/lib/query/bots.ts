import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { BotProfile, StatsResult } from "@/types/bot";

export function useBotsQuery() {
  return useQuery({
    queryKey: queryKeys.bots.all(),
    queryFn: () => invoke<BotProfile[]>("list_bots"),
    retry: false,
  });
}

export function useBotStatsQuery() {
  return useQuery({
    queryKey: queryKeys.bots.stats(),
    queryFn: () => invoke<StatsResult>("get_stats"),
    retry: false,
  });
}

export function invalidateBotsQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.bots.all() });
}

export function invalidateBotStatsQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.bots.stats() });
}
