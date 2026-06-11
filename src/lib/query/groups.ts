import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { GroupEvent } from "@/types/event";
import type {
  ConversationState,
  GroupCategory,
  GroupMemberProfile,
  GroupProfile,
} from "@/types/group";

export function useGroupsQuery() {
  return useQuery({
    queryKey: queryKeys.groups.all(),
    queryFn: () => invoke<GroupProfile[]>("list_groups"),
    retry: false,
  });
}

export function useUserGroupsQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.groups.byUser(userId),
    enabled: isValidUserId(userId),
    queryFn: () => invoke<GroupProfile[]>("list_user_groups", { userId }),
    retry: false,
  });
}

export function useGroupMembersQuery(
  userId: string,
  groupId: string,
  enabled: boolean,
) {
  return useQuery({
    queryKey: queryKeys.groups.members(userId, groupId),
    queryFn: () =>
      invoke<GroupMemberProfile[]>("list_group_members", { userId, groupId }),
    retry: false,
    enabled: enabled && isValidUserId(userId) && groupId.length > 0,
  });
}

export function useGroupEventHistoryQuery(
  userId: string,
  groupId: string,
  limit: number,
  enabled: boolean,
) {
  return useQuery({
    queryKey: queryKeys.groups.eventHistory(userId, groupId, limit),
    queryFn: () =>
      invoke<GroupEvent[]>("list_group_event_history", {
        userId,
        groupId,
        limit,
      }),
    retry: false,
    enabled,
  });
}

export function invalidateGroupsQuery() {
  return queryClient.invalidateQueries({ queryKey: queryKeys.groups.root() });
}

export function invalidateGroupMembersQuery(userId: string, groupId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.members(userId, groupId),
  });
}

export function invalidateGroupEventHistoryQuery(
  userId: string,
  groupId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.eventHistoryPrefix(userId, groupId),
    refetchType: "active",
  });
}

// === Conversation States ===

export function useConversationStatesQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.conversation.states(userId),
    enabled: isValidUserId(userId),
    queryFn: () =>
      invoke<ConversationState[]>("list_conversation_states", { userId }),
    retry: false,
  });
}

export function invalidateConversationStatesQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.conversation.states(userId),
  });
}

// === Group Categories ===

export function useGroupCategoriesQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.groups.categories(userId),
    enabled: isValidUserId(userId),
    queryFn: () => invoke<GroupCategory[]>("list_group_categories", { userId }),
    retry: false,
  });
}

export function invalidateGroupCategoriesQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.categories(userId),
  });
}
