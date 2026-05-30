import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  invalidateFriendRequestsQuery,
  invalidateFriendsQuery,
  invalidateGroupRequestsQueries,
  invalidateGroupsQuery,
  invalidateMessageHistoryQuery,
  invalidatePokeHistoryQuery,
  invalidateUsersQuery,
} from "@/lib/query";
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
    onSuccess: (_, variables) =>
      invalidateMessageHistoryQuery(variables.userId, variables.source),
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
      invalidateMessageHistoryQuery(variables.userId, variables.source),
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
      invalidatePokeHistoryQuery(variables.userId, variables.source),
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