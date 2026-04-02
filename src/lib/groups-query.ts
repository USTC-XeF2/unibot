import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";
import type { GroupEvent } from "@/types/chat";
import type { GroupProfile } from "@/types/group";

const groupsQueryKey = ["groups"] as const;

type GroupEventHistoryQueryKey = readonly [
  "groups",
  "event-history",
  number,
  number,
  number,
];

function userGroupsQueryKey(userId: number) {
  return [...groupsQueryKey, "user", userId] as const;
}

export const groupsQueryOptions = queryOptions({
  queryKey: [...groupsQueryKey, "all"] as const,
  queryFn: () => invoke<GroupProfile[]>("list_groups"),
  retry: false,
});

export function userGroupsQueryOptions(userId: number) {
  return queryOptions({
    queryKey: userGroupsQueryKey(userId),
    enabled: Number.isInteger(userId) && userId > 0,
    queryFn: () => invoke<GroupProfile[]>("list_user_groups", { userId }),
    retry: false,
  });
}

export function useGroupsQuery() {
  return useQuery(groupsQueryOptions);
}

export function useUserGroupsQuery(userId: number) {
  const options = userGroupsQueryOptions(userId);
  return useQuery(options);
}

export function groupEventHistoryQueryKey(
  userId: number,
  groupId: number,
  limit: number,
): GroupEventHistoryQueryKey {
  return ["groups", "event-history", userId, groupId, limit];
}

export function groupEventHistoryQueryOptions(
  userId: number,
  groupId: number,
  limit: number,
) {
  return queryOptions({
    queryKey: groupEventHistoryQueryKey(userId, groupId, limit),
    queryFn: () =>
      invoke<GroupEvent[]>("list_group_event_history", {
        userId,
        groupId,
        limit,
      }),
    retry: false,
  });
}

export function useGroupEventHistoryQuery(
  userId: number,
  groupId: number,
  limit: number,
  enabled: boolean,
) {
  return useQuery({
    ...groupEventHistoryQueryOptions(userId, groupId, limit),
    enabled,
  });
}

export function invalidateGroupsQuery() {
  return queryClient.invalidateQueries({ queryKey: groupsQueryKey });
}

export function invalidateGroupEventHistoryQuery(
  userId: number,
  groupId: number,
) {
  return queryClient.invalidateQueries({
    queryKey: ["groups", "event-history", userId, groupId],
    refetchType: "active",
  });
}
