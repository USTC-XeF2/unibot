import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { isValidUserId } from "@/lib/query/common";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { GroupEvent } from "@/types/event";
import type {
  ConversationState,
  GroupAlbum,
  GroupAnnouncement,
  GroupCategory,
  GroupEssenceMessage,
  GroupFile,
  GroupFolder,
  GroupMemberProfile,
  GroupPhoto,
  GroupProfile,
} from "@/types/group";

export function useGroupsQuery() {
  return useQuery({
    queryKey: queryKeys.groups.all(),
    queryFn: () => invoke<GroupProfile[]>(COMMANDS.listGroups),
    retry: false,
  });
}

export function useUserGroupsQuery(userId: string) {
  return useQuery({
    queryKey: queryKeys.groups.byUser(userId),
    enabled: isValidUserId(userId),
    queryFn: () => invoke<GroupProfile[]>(COMMANDS.listUserGroups, { userId }),
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
      invoke<GroupMemberProfile[]>(COMMANDS.listGroupMembers, {
        userId,
        groupId,
      }),
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
      invoke<GroupEvent[]>(COMMANDS.listGroupEventHistory, {
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
      invoke<ConversationState[]>(COMMANDS.listConversationStates, { userId }),
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
    queryFn: () =>
      invoke<GroupCategory[]>(COMMANDS.listGroupCategories, { userId }),
    retry: false,
  });
}

export function invalidateGroupCategoriesQuery(userId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.categories(userId),
  });
}

// === Group Files ===

export function useGroupFilesQuery(
  userId: string,
  groupId: string,
  parentFolderId?: string,
) {
  return useQuery({
    queryKey: queryKeys.groups.files(userId, groupId, parentFolderId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupFile[]>(COMMANDS.listGroupFiles, {
        userId,
        groupId,
        parentFolderId: parentFolderId || null,
      }),
    retry: false,
  });
}

export function invalidateGroupFilesQuery(
  userId: string,
  groupId: string,
  parentFolderId?: string,
) {
  return queryClient.invalidateQueries({
    queryKey:
      parentFolderId === undefined
        ? ["groups", "files", userId, groupId]
        : queryKeys.groups.files(userId, groupId, parentFolderId),
  });
}

// === Group Albums ===

export function useGroupAlbumsQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.albums(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupAlbum[]>(COMMANDS.listGroupAlbums, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupAlbumsQuery(userId: string, groupId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.albums(userId, groupId),
  });
}

// === Group Photos ===

export function useGroupPhotosQuery(
  userId: string,
  groupId: string,
  albumId: string,
) {
  return useQuery({
    queryKey: queryKeys.groups.photos(userId, albumId),
    enabled: isValidUserId(userId) && groupId.length > 0 && albumId.length > 0,
    queryFn: () =>
      invoke<GroupPhoto[]>(COMMANDS.listGroupPhotos, {
        userId,
        groupId,
        albumId,
      }),
    retry: false,
  });
}

export function invalidateGroupPhotosQuery(userId: string, albumId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.photos(userId, albumId),
  });
}

// === Group Folders ===

export function useGroupFoldersQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.folders(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupFolder[]>(COMMANDS.listGroupFolders, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupFoldersQuery(userId: string, groupId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.folders(userId, groupId),
  });
}

// === Group Announcements ===

export function useGroupAnnouncementsQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.announcements(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupAnnouncement[]>(COMMANDS.listGroupAnnouncements, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupAnnouncementsQuery(
  userId: string,
  groupId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.announcements(userId, groupId),
  });
}

// === Group Essence Messages ===

export function useGroupEssenceMessagesQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.essence(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupEssenceMessage[]>(COMMANDS.listGroupEssenceMessages, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupEssenceMessagesQuery(
  userId: string,
  groupId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.essence(userId, groupId),
  });
}
