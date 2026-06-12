import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  invalidateBotStatsQuery,
  invalidateBotsQuery,
  invalidateConversationStatesQuery,
  invalidateFriendRequestsQuery,
  invalidateFriendsQuery,
  invalidateGroupAlbumsQuery,
  invalidateGroupCategoriesQuery,
  invalidateGroupFilesQuery,
  invalidateGroupPhotosQuery,
  invalidateGroupRequestsQueries,
  invalidateGroupsQuery,
  invalidateUsersQuery,
  refetchMessageHistoryQuery,
  refetchPokeHistoryQuery,
} from "@/lib/query";
import type { BotProfile, DebugSession } from "@/types/bot";
import type { MessageSegment, MessageSource } from "@/types/chat";
import type { RequestState } from "@/types/request";

type RequestActionState = Extract<RequestState, "accepted" | "rejected">;

export function useDeleteFriendMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      friendUserId,
    }: {
      userId: string;
      friendUserId: string;
    }) =>
      invoke("delete_friend", {
        userId,
        friendUserId,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        invalidateUsersQuery(),
        invalidateFriendRequestsQuery(variables.userId),
        invalidateFriendsQuery(variables.userId),
      ]);
    },
  });
}

export function useSetGroupWholeMuteMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      durationSeconds,
    }: {
      userId: string;
      groupId: string;
      durationSeconds: number;
    }) =>
      invoke("set_group_whole_mute", {
        userId,
        groupId,
        durationSeconds,
      }),
  });
}

export function useRenameGroupMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      groupName,
    }: {
      userId: string;
      groupId: string;
      groupName: string;
    }) =>
      invoke("rename_group", {
        userId,
        groupId,
        groupName,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useDissolveGroupMutation() {
  return useMutation({
    mutationFn: ({ userId, groupId }: { userId: string; groupId: string }) =>
      invoke("dissolve_group", {
        userId,
        groupId,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useLeaveGroupMutation() {
  return useMutation({
    mutationFn: ({ userId, groupId }: { userId: string; groupId: string }) =>
      invoke("leave_group", {
        userId,
        groupId,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useCreateFriendRequestMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      targetUserId,
      comment,
    }: {
      userId: string;
      targetUserId: string;
      comment?: string;
    }) =>
      invoke("create_friend_request", {
        userId,
        targetUserId,
        comment: comment ?? "",
      }),
    onSuccess: (_, variables) =>
      invalidateFriendRequestsQuery(variables.userId),
  });
}

export function useCreateGroupMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      groupName,
      initialMemberUserIds,
    }: {
      userId: string;
      groupId: string;
      groupName: string;
      initialMemberUserIds: string[];
    }) =>
      invoke("upsert_group", {
        userId,
        groupId,
        groupName,
        maxMemberCount: 500,
        initialMemberUserIds,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useHandleFriendRequestMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      requestId,
      state,
    }: {
      userId: string;
      requestId: string;
      state: RequestActionState;
    }) =>
      invoke("handle_friend_request", {
        userId,
        requestId,
        state,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        invalidateFriendRequestsQuery(variables.userId),
        invalidateGroupRequestsQueries(variables.userId),
        invalidateFriendsQuery(variables.userId),
      ]);
    },
  });
}

export function useHandleGroupRequestMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      requestId,
      state,
    }: {
      userId: string;
      requestId: string;
      state: RequestActionState;
    }) =>
      invoke("handle_group_request", {
        userId,
        requestId,
        state,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        invalidateGroupsQuery(),
        invalidateGroupRequestsQueries(variables.userId),
      ]);
    },
  });
}

export function useSendMessageMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      source,
      content,
      quotedMessageId,
    }: {
      userId: string;
      source: MessageSource;
      content: MessageSegment[];
      quotedMessageId: string | null;
    }) =>
      invoke("send_message", {
        userId,
        source,
        content,
        quotedMessageId,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        refetchMessageHistoryQuery(variables.userId, variables.source),
        invalidateBotStatsQuery(),
      ]);
    },
  });
}

export function useRecallMessageMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      messageId,
      source: _source,
    }: {
      userId: string;
      messageId: string;
      source: MessageSource;
    }) =>
      invoke("recall_message", {
        userId,
        messageId,
      }),
    onSuccess: (_, variables) =>
      refetchMessageHistoryQuery(variables.userId, variables.source),
  });
}

export function usePokeUserMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      source,
      targetUserId,
    }: {
      userId: string;
      source: MessageSource;
      targetUserId: string;
    }) =>
      invoke("poke_user", {
        userId,
        source,
        targetUserId,
      }),
    onSuccess: (_, variables) =>
      refetchPokeHistoryQuery(variables.userId, variables.source),
  });
}

export function useMuteGroupMemberMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      targetUserId,
      durationSeconds,
    }: {
      userId: string;
      groupId: string;
      targetUserId: string;
      durationSeconds: number;
    }) =>
      invoke("mute_group_member", {
        userId,
        groupId,
        targetUserId,
        durationSeconds,
      }),
  });
}

export function useKickGroupMemberMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      targetUserId,
    }: {
      userId: string;
      groupId: string;
      targetUserId: string;
    }) =>
      invoke("kick_group_member", {
        userId,
        groupId,
        targetUserId,
      }),
  });
}

export function useSetGroupMemberRoleMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      targetUserId,
      isAdmin,
    }: {
      userId: string;
      groupId: string;
      targetUserId: string;
      isAdmin: boolean;
    }) =>
      invoke("set_group_member_role", {
        userId,
        groupId,
        targetUserId,
        isAdmin,
      }),
  });
}

export function useSetGroupMemberTitleMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      targetUserId,
      title,
    }: {
      userId: string;
      groupId: string;
      targetUserId: string;
      title: string;
    }) =>
      invoke("set_group_member_title", {
        userId,
        groupId,
        targetUserId,
        title,
      }),
  });
}

export function useCreateBotMutation() {
  return useMutation({
    mutationFn: ({
      boundUserId,
      displayName,
    }: {
      boundUserId: string;
      displayName: string;
    }) =>
      invoke<BotProfile>("create_bot", {
        boundUserId,
        displayName,
      }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useDeleteBotMutation() {
  return useMutation({
    mutationFn: ({ botId }: { botId: string }) =>
      invoke("delete_bot", { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useStartBotMutation() {
  return useMutation({
    mutationFn: ({ botId }: { botId: string }) =>
      invoke<DebugSession>("start_bot", { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useStopBotMutation() {
  return useMutation({
    mutationFn: ({ botId }: { botId: string }) => invoke("stop_bot", { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useSetLogLevelMutation() {
  return useMutation({
    mutationFn: ({ level }: { level: string }) =>
      invoke("set_log_level", { level }),
  });
}

export function useSetLogRetentionMutation() {
  return useMutation({
    mutationFn: ({ days }: { days: number }) =>
      invoke("set_log_retention_days", { days }),
  });
}

export function useTriggerLogCleanupMutation() {
  return useMutation({
    mutationFn: () => invoke<{ deleted_files: number }>("trigger_log_cleanup"),
  });
}

export function useSetConversationPinnedMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      scene,
      peerUserId,
      groupId,
      isPinned,
    }: {
      userId: string;
      scene: "private" | "group" | "temp";
      peerUserId: string | null;
      groupId: string | null;
      isPinned: boolean;
    }) =>
      invoke("set_conversation_pinned", {
        userId,
        scene,
        peerUserId,
        groupId,
        isPinned,
      }),
    onSuccess: (_, variables) =>
      invalidateConversationStatesQuery(variables.userId),
  });
}

export function useSetConversationMutedMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      scene,
      peerUserId,
      groupId,
      isMuted,
    }: {
      userId: string;
      scene: "private" | "group" | "temp";
      peerUserId: string | null;
      groupId: string | null;
      isMuted: boolean;
    }) =>
      invoke("set_conversation_muted", {
        userId,
        scene,
        peerUserId,
        groupId,
        isMuted,
      }),
    onSuccess: (_, variables) =>
      invalidateConversationStatesQuery(variables.userId),
  });
}

export function useCreateGroupCategoryMutation() {
  return useMutation({
    mutationFn: ({ userId, name }: { userId: string; name: string }) =>
      invoke("create_group_category", { userId, name }),
    onSuccess: (_, variables) =>
      invalidateGroupCategoriesQuery(variables.userId),
  });
}

export function useDeleteGroupCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      categoryId,
    }: {
      userId: string;
      categoryId: string;
    }) => invoke("delete_group_category", { userId, categoryId }),
    onSuccess: (_, variables) =>
      invalidateGroupCategoriesQuery(variables.userId),
  });
}

export function useSetGroupCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      categoryId,
    }: {
      userId: string;
      groupId: string;
      categoryId: string | null;
    }) => invoke("set_group_category", { userId, groupId, categoryId }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateGroupCategoriesQuery(variables.userId),
        invalidateGroupsQuery(),
      ]),
  });
}

export function useUploadGroupFileMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      parentFolderId,
      fileName,
      sourcePath,
    }: {
      userId: string;
      groupId: string;
      parentFolderId?: string;
      fileName: string;
      sourcePath: string;
    }) =>
      invoke("upload_group_file", {
        userId,
        groupId,
        parentFolderId: parentFolderId ?? null,
        fileName,
        sourcePath,
      }),
    onSuccess: (_, variables) =>
      invalidateGroupFilesQuery(
        variables.userId,
        variables.groupId,
        variables.parentFolderId,
      ),
  });
}

export function useDownloadGroupFileMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      fileId,
    }: {
      userId: string;
      groupId: string;
      fileId: string;
    }) =>
      invoke<string>("download_group_file", {
        userId,
        groupId,
        fileId,
      }),
  });
}

export function useDeleteGroupFileMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      fileId: string;
      parentFolderId: string;
    }) =>
      invoke("delete_group_file", {
        userId: params.userId,
        groupId: params.groupId,
        fileId: params.fileId,
      }),
    onSuccess: (_, params) =>
      invalidateGroupFilesQuery(
        params.userId,
        params.groupId,
        params.parentFolderId,
      ),
  });
}

export function useCreateGroupAlbumMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      name,
    }: {
      userId: string;
      groupId: string;
      name: string;
    }) => invoke("create_group_album", { userId, groupId, name }),
    onSuccess: (_, variables) =>
      invalidateGroupAlbumsQuery(variables.userId, variables.groupId),
  });
}

export function useDeleteGroupAlbumMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      albumId: string;
    }) =>
      invoke("delete_group_album", {
        userId: params.userId,
        groupId: params.groupId,
        albumId: params.albumId,
      }),
    onSuccess: (_, params) =>
      invalidateGroupAlbumsQuery(params.userId, params.groupId),
  });
}

export function useUploadGroupPhotoMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      albumId,
      sourcePath,
      description,
    }: {
      userId: string;
      groupId: string;
      albumId: string;
      sourcePath: string;
      description?: string;
    }) =>
      invoke("upload_group_photo", {
        userId,
        groupId,
        albumId,
        sourcePath,
        description: description ?? null,
      }),
    onSuccess: (_, variables) =>
      invalidateGroupPhotosQuery(variables.userId, variables.albumId),
  });
}

export function useDeleteGroupPhotoMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      albumId: string;
      photoId: string;
    }) =>
      invoke("delete_group_photo", {
        userId: params.userId,
        groupId: params.groupId,
        photoId: params.photoId,
      }),
    onSuccess: (_, params) =>
      invalidateGroupPhotosQuery(params.userId, params.albumId),
  });
}

export function useRenameBotMutation() {
  return useMutation({
    mutationFn: ({
      botId,
      displayName,
    }: {
      botId: string;
      displayName: string;
    }) =>
      invoke<BotProfile>("rename_bot", {
        botId,
        displayName,
      }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useOpenDeveloperToolsMutation() {
  return useMutation({
    mutationFn: () => invoke<boolean>("open_developer_tools"),
  });
}
