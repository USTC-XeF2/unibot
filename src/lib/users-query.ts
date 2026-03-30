import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";
import type { UserProfile } from "@/types/user";

const usersQueryKey = ["users"] as const;

export const usersQueryOptions = queryOptions({
  queryKey: usersQueryKey,
  queryFn: () => invoke<UserProfile[]>("list_users"),
  retry: false,
});

export function useUsersQuery() {
  return useQuery(usersQueryOptions);
}

export function invalidateUsersQuery() {
  return queryClient.invalidateQueries({ queryKey: usersQueryKey });
}
