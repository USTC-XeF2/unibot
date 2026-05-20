import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";

export function useFriendsQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.friends.byUser(userId),
    queryFn: () => invoke<string[]>("list_friends", { userId }),
    retry: false,
    enabled: isValidUserId(userId),
  });
}

export function invalidateFriendsQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.friends.byUser(userId),
  });
}