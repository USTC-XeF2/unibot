import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { COMMANDS } from "@/lib/commands";
import {
  invalidateBotStatsQuery,
  invalidateBotsQuery,
  invalidateConversationStatesQuery,
  invalidateFriendCategoriesQuery,
  invalidateFriendRequestsQuery,
  invalidateFriendshipsQuery,
  invalidateFriendsQuery,
  invalidateGroupAlbumsQuery,
  invalidateGroupAnnouncementsQuery,
  invalidateGroupCategoriesQuery,
  invalidateGroupEssenceMessagesQuery,
  invalidateGroupFilesQuery,
  invalidateGroupFoldersQuery,
  invalidateGroupPhotosQuery,
  invalidateGroupRequestsQueries,
  invalidateGroupsQuery,
  invalidateUsersQuery,
  refetchMessageHistoryQuery,
  refetchPokeHistoryQuery,
} from "@/lib/query";
import type { BotProfile, DebugSession } from "@/types/bot";
import type { MessageSegment, MessageSource } from "@/types/chat";
import type {
  GroupAnnouncement,
  GroupEssenceMessage,
  GroupFolder,
} from "@/types/group";
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
      invoke(COMMANDS.deleteFriend, {
        userId,
        friendUserId,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        invalidateUsersQuery(),
        invalidateFriendRequestsQuery(variables.userId),
        invalidateFriendsQuery(variables.userId),
        invalidateFriendshipsQuery(variables.userId),
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
      invoke(COMMANDS.setGroupWholeMute, {
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
      invoke(COMMANDS.renameGroup, {
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
      invoke(COMMANDS.dissolveGroup, {
        userId,
        groupId,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useLeaveGroupMutation() {
  return useMutation({
    mutationFn: ({ userId, groupId }: { userId: string; groupId: string }) =>
      invoke(COMMANDS.leaveGroup, {
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
      invoke(COMMANDS.createFriendRequest, {
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
      invoke(COMMANDS.upsertGroup, {
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
      invoke(COMMANDS.handleFriendRequest, {
        userId,
        requestId,
        state,
      }),
    onSuccess: async (_, variables) => {
      await Promise.all([
        invalidateFriendRequestsQuery(variables.userId),
        invalidateGroupRequestsQueries(variables.userId),
        invalidateFriendsQuery(variables.userId),
        invalidateFriendshipsQuery(variables.userId),
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
      invoke(COMMANDS.handleGroupRequest, {
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
      invoke(COMMANDS.sendMessage, {
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
      invoke(COMMANDS.recallMessage, {
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
      invoke(COMMANDS.pokeUser, {
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
      invoke(COMMANDS.muteGroupMember, {
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
      invoke(COMMANDS.kickGroupMember, {
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
      invoke(COMMANDS.setGroupMemberRole, {
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
      invoke(COMMANDS.setGroupMemberTitle, {
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
      invoke<BotProfile>(COMMANDS.createBot, {
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
      invoke(COMMANDS.deleteBot, { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useStartBotMutation() {
  return useMutation({
    mutationFn: ({ botId }: { botId: string }) =>
      invoke<DebugSession>(COMMANDS.startBot, { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useStopBotMutation() {
  return useMutation({
    mutationFn: ({ botId }: { botId: string }) =>
      invoke(COMMANDS.stopBot, { botId }),
    onSuccess: async () => {
      await Promise.all([invalidateBotsQuery(), invalidateBotStatsQuery()]);
    },
  });
}

export function useSetLogLevelMutation() {
  return useMutation({
    mutationFn: ({ level }: { level: string }) =>
      invoke(COMMANDS.setLogLevel, { level }),
  });
}

export function useSetLogRetentionMutation() {
  return useMutation({
    mutationFn: ({ days }: { days: number }) =>
      invoke(COMMANDS.setLogRetentionDays, { days }),
  });
}

export function useTriggerLogCleanupMutation() {
  return useMutation({
    mutationFn: () =>
      invoke<{ deleted_files: number }>(COMMANDS.triggerLogCleanup),
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
      invoke(COMMANDS.setConversationPinned, {
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
      invoke(COMMANDS.setConversationMuted, {
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
      invoke(COMMANDS.createGroupCategory, { userId, name }),
    onSuccess: (_, variables) =>
      invalidateGroupCategoriesQuery(variables.userId),
  });
}

export function useRenameGroupCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      categoryId,
      name,
    }: {
      userId: string;
      categoryId: string;
      name: string;
    }) => invoke(COMMANDS.renameGroupCategory, { userId, categoryId, name }),
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
    }) => invoke(COMMANDS.deleteGroupCategory, { userId, categoryId }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateGroupCategoriesQuery(variables.userId),
        invalidateGroupsQuery(),
      ]),
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
    }) => invoke(COMMANDS.setGroupCategory, { userId, groupId, categoryId }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateGroupCategoriesQuery(variables.userId),
        invalidateGroupsQuery(),
      ]),
  });
}

export function useCreateFriendCategoryMutation() {
  return useMutation({
    mutationFn: ({ userId, name }: { userId: string; name: string }) =>
      invoke(COMMANDS.createFriendCategory, { userId, name }),
    onSuccess: (_, variables) =>
      invalidateFriendCategoriesQuery(variables.userId),
  });
}

export function useRenameFriendCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      categoryId,
      name,
    }: {
      userId: string;
      categoryId: string;
      name: string;
    }) => invoke(COMMANDS.renameFriendCategory, { userId, categoryId, name }),
    onSuccess: (_, variables) =>
      invalidateFriendCategoriesQuery(variables.userId),
  });
}

export function useDeleteFriendCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      categoryId,
    }: {
      userId: string;
      categoryId: string;
    }) => invoke(COMMANDS.deleteFriendCategory, { userId, categoryId }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateFriendCategoriesQuery(variables.userId),
        invalidateFriendshipsQuery(variables.userId),
      ]),
  });
}

export function useSetFriendCategoryMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      friendUserId,
      categoryId,
    }: {
      userId: string;
      friendUserId: string;
      categoryId: string;
    }) =>
      invoke(COMMANDS.setFriendCategory, {
        userId,
        friendUserId,
        categoryId,
      }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateFriendCategoriesQuery(variables.userId),
        invalidateFriendshipsQuery(variables.userId),
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
      fileName?: string;
      sourcePath: string;
    }) =>
      invoke(COMMANDS.uploadGroupFile, {
        input: {
          user_id: userId,
          group_id: groupId,
          parent_folder_id: parentFolderId ?? null,
          file_name: fileName ?? null,
          source_path: sourcePath,
        },
      }),
    onSuccess: (_, variables) =>
      invalidateGroupFilesQuery(
        variables.userId,
        variables.groupId,
        variables.parentFolderId,
      ),
    onError: (error) => {
      toast.error(`上传文件失败：${error}`);
    },
  });
}

export function useDownloadGroupFileMutation() {
  return useMutation({
    mutationFn: ({
      userId,
      groupId,
      fileId,
      destinationPath,
    }: {
      userId: string;
      groupId: string;
      fileId: string;
      destinationPath: string;
    }) =>
      invoke<string>(COMMANDS.downloadGroupFile, {
        userId,
        groupId,
        fileId,
        destinationPath,
      }),
    onError: (error) => {
      toast.error(`下载文件失败：${error}`);
    },
  });
}

export function useDeleteGroupFileMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      fileId: string;
      parentFolderId?: string;
    }) =>
      invoke(COMMANDS.deleteGroupFile, {
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
    }) => invoke(COMMANDS.createGroupAlbum, { userId, groupId, name }),
    onSuccess: (_, variables) =>
      invalidateGroupAlbumsQuery(variables.userId, variables.groupId),
    onError: (error) => {
      toast.error(`创建相册失败：${error}`);
    },
  });
}

export function useDeleteGroupAlbumMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      albumId: string;
    }) =>
      invoke(COMMANDS.deleteGroupAlbum, {
        userId: params.userId,
        groupId: params.groupId,
        albumId: params.albumId,
      }),
    onSuccess: (_, params) =>
      Promise.all([
        invalidateGroupAlbumsQuery(params.userId, params.groupId),
        invalidateGroupPhotosQuery(params.userId, params.albumId),
      ]),
    onError: (error) => {
      toast.error(`删除相册失败：${error}`);
    },
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
      invoke(COMMANDS.uploadGroupPhoto, {
        input: {
          user_id: userId,
          group_id: groupId,
          album_id: albumId,
          source_path: sourcePath,
          description: description ?? null,
        },
      }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateGroupPhotosQuery(variables.userId, variables.albumId),
        invalidateGroupAlbumsQuery(variables.userId, variables.groupId),
      ]),
    onError: (error) => {
      toast.error(`上传照片失败：${error}`);
    },
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
      invoke(COMMANDS.deleteGroupPhoto, {
        userId: params.userId,
        groupId: params.groupId,
        photoId: params.photoId,
      }),
    onSuccess: (_, params) =>
      Promise.all([
        invalidateGroupPhotosQuery(params.userId, params.albumId),
        invalidateGroupAlbumsQuery(params.userId, params.groupId),
      ]),
    onError: (error) => {
      toast.error(`删除照片失败：${error}`);
    },
  });
}

// === Group Folders ===

export function useUpsertGroupFolderMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      folderId?: string;
      parentFolderId?: string;
      folderName: string;
    }) =>
      invoke<GroupFolder>(COMMANDS.upsertGroupFolder, {
        input: {
          folder_id: input.folderId ?? "",
          group_id: input.groupId,
          parent_folder_id: input.parentFolderId ?? null,
          folder_name: input.folderName,
          creator_user_id: input.userId,
          file_count: 0,
        },
      }),
    onSuccess: (_, variables) =>
      Promise.all([
        invalidateGroupFoldersQuery(variables.userId, variables.groupId),
        invalidateGroupFilesQuery(
          variables.userId,
          variables.groupId,
          variables.parentFolderId,
        ),
      ]),
    onError: (error, variables) => {
      const action = variables.folderId ? "重命名文件夹" : "创建文件夹";
      toast.error(`${action}失败：${error}`);
    },
  });
}

export function useDeleteGroupFolderMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      folderId: string;
      parentFolderId?: string;
    }) =>
      invoke(COMMANDS.deleteGroupFolder, {
        userId: params.userId,
        groupId: params.groupId,
        folderId: params.folderId,
      }),
    onSuccess: (_, variables) =>
      invalidateGroupFoldersQuery(variables.userId, variables.groupId),
    onError: (error) => {
      toast.error(`删除文件夹失败：${error}`);
    },
  });
}

export function useDeleteGroupAnnouncementMutation() {
  return useMutation({
    mutationFn: (params: {
      userId: string;
      groupId: string;
      announcementId: string;
    }) =>
      invoke(COMMANDS.deleteGroupAnnouncement, {
        userId: params.userId,
        groupId: params.groupId,
        announcementId: params.announcementId,
      }),
    onSuccess: (_, params) =>
      invalidateGroupAnnouncementsQuery(params.userId, params.groupId),
    onError: (error) => {
      toast.error(`删除公告失败：${error}`);
    },
  });
}

// === Group Announcements ===

export function useUpsertGroupAnnouncementMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      announcementId?: string;
      content: string;
      imageUrl?: string;
    }) =>
      invoke<GroupAnnouncement>(COMMANDS.upsertGroupAnnouncement, {
        input: {
          announcement_id: input.announcementId ?? "",
          group_id: input.groupId,
          sender_user_id: input.userId,
          content: input.content,
          image_url: input.imageUrl ?? null,
        },
      }),
    onSuccess: (_, variables) =>
      invalidateGroupAnnouncementsQuery(variables.userId, variables.groupId),
    onError: (error, variables) => {
      const action = variables.announcementId ? "编辑公告" : "发布公告";
      toast.error(`${action}失败：${error}`);
    },
  });
}

// === Group Essence ===

/**
 * Mirrors the Rust `EssenceUpdate` tagged enum so set/unset can never carry
 * the wrong identifier: "set" requires a messageId, "unset" requires an
 * essenceId, enforced by the type system on both sides of the Tauri boundary.
 */
export type EssenceUpdate =
  | { kind: "set"; messageId: string }
  | { kind: "unset"; essenceId: string };

export function useSetGroupEssenceMessageMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      update: EssenceUpdate;
    }) =>
      invoke<GroupEssenceMessage>(COMMANDS.setGroupEssenceMessage, {
        userId: input.userId,
        groupId: input.groupId,
        update:
          input.update.kind === "set"
            ? { type: "set", message_id: input.update.messageId }
            : { type: "unset", essence_id: input.update.essenceId },
      }),
    onSuccess: (_, variables) =>
      invalidateGroupEssenceMessagesQuery(variables.userId, variables.groupId),
    onError: (error) => {
      toast.error(`设置精华失败：${error}`);
    },
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
      invoke<BotProfile>(COMMANDS.renameBot, {
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
    mutationFn: () => invoke<boolean>(COMMANDS.openDeveloperTools),
  });
}
