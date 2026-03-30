import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";

function friendsQueryKey(userId: number) {
  return ["friends", userId] as const;
}

export function friendsQueryOptions(userId: number) {
  return queryOptions({
    queryKey: friendsQueryKey(userId),
    queryFn: () => invoke<number[]>("list_friends", { userId }),
    retry: false,
    enabled: Number.isInteger(userId) && userId > 0,
  });
}

export function useFriendsQuery(userId: number) {
  return useQuery(friendsQueryOptions(userId));
}

export function invalidateFriendsQuery(userId: number) {
  return queryClient.invalidateQueries({ queryKey: friendsQueryKey(userId) });
}
