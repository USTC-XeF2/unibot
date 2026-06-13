import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { FriendRequestEntity, GroupRequestEntity } from "@/types/request";

export function useFriendRequestsQuery(userId: string, enabled = true) {
  return useQuery({
    queryKey: queryKeys.requests.friendByUser(userId),
    enabled: enabled && isValidUserId(userId),
    retry: false,
    queryFn: () =>
      invoke<FriendRequestEntity[]>(COMMANDS.listFriendRequests, {
        userId,
      }),
  });
}

export function useGroupRequestsQuery(userId: string, enabled = true) {
  return useQuery({
    queryKey: queryKeys.requests.manageableGroup(userId, "all"),
    enabled: enabled && isValidUserId(userId),
    retry: false,
    queryFn: () =>
      invoke<GroupRequestEntity[]>(COMMANDS.listGroupRequests, {
        userId,
      }),
  });
}

export function invalidateFriendRequestsQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.requests.friendByUser(userId),
  });
}

export function invalidateGroupRequestsQueries(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.requests.manageableGroupPrefix(userId),
  });
}
