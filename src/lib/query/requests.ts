import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { FriendRequestEntity, GroupRequestEntity } from "@/types/request";

export function useFriendRequestsQuery(userId: number, enabled = true) {
  return useQuery({
    queryKey: queryKeys.requests.friendByUser(userId),
    enabled: enabled && isValidUserId(userId),
    retry: false,
    queryFn: () =>
      invoke<FriendRequestEntity[]>("list_friend_requests", {
        userId,
      }),
  });
}

export function useGroupRequestsQuery(userId: number, enabled = true) {
  return useQuery({
    queryKey: queryKeys.requests.manageableGroup(userId, "all"),
    enabled: enabled && isValidUserId(userId),
    retry: false,
    queryFn: () =>
      invoke<GroupRequestEntity[]>("list_group_requests", {
        userId,
      }),
  });
}

export function invalidateFriendRequestsQuery(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.requests.friendByUser(userId),
  });
}

export function invalidateGroupRequestsQueries(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.requests.manageableGroupPrefix(userId),
  });
}
