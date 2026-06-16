import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { FriendCategory, Friendship } from "@/types/user";

export function useFriendsQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.friends.byUser(userId),
    queryFn: () => invoke<string[]>(COMMANDS.listFriends, { userId }),
    retry: false,
    enabled: isValidUserId(userId),
  });
}

export function useFriendshipsQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.friends.friendships(userId),
    queryFn: () => invoke<Friendship[]>(COMMANDS.listFriendships, { userId }),
    retry: false,
    enabled: isValidUserId(userId),
  });
}

export function useFriendCategoriesQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.friends.categories(userId),
    queryFn: () =>
      invoke<FriendCategory[]>(COMMANDS.listFriendCategories, { userId }),
    retry: false,
    enabled: isValidUserId(userId),
  });
}

export function invalidateFriendsQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.friends.byUser(userId),
  });
}

export function invalidateFriendshipsQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.friends.friendships(userId),
  });
}

export function invalidateFriendCategoriesQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.friends.categories(userId),
  });
}
