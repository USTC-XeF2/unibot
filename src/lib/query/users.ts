import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { UserProfile } from "@/types/user";

export function useUsersQuery() {
  return useQuery({
    queryKey: queryKeys.users.all(),
    queryFn: () => invoke<UserProfile[]>(COMMANDS.listUsers),
    retry: false,
  });
}

export function invalidateUsersQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.users.all() });
}
