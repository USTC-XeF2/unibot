import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";
import type { GroupProfile } from "@/types/group";

const groupsQueryKey = ["groups"] as const;

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

export function invalidateGroupsQuery() {
  return queryClient.invalidateQueries({ queryKey: groupsQueryKey });
}
