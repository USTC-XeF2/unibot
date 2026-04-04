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
      userId: number;
      friendUserId: number;
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
      userId: number;
      groupId: number;
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
      userId: number;
      groupId: number;
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
    mutationFn: ({ userId, groupId }: { userId: number; groupId: number }) =>
      invoke("dissolve_group", {
        userId,
        groupId,
      }),
    onSuccess: () => invalidateGroupsQuery(),
  });
}

export function useLeaveGroupMutation() {
  return useMutation({
    mutationFn: ({ userId, groupId }: { userId: number; groupId: number }) =>
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
      userId: number;
      targetUserId: number;
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
      userId: number;
      groupId: number;
      groupName: string;
      initialMemberUserIds: number[];
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
      userId: number;
      requestId: number;
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
      userId: number;
      requestId: number;
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
      userId: number;
      source: MessageSource;
      content: MessageSegment[];
      quotedMessageId: number | null;
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
      userId: number;
      messageId: number;
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
      userId: number;
      source: MessageSource;
      targetUserId: number;
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
      userId: number;
      groupId: number;
      targetUserId: number;
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
      userId: number;
      groupId: number;
      targetUserId: number;
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
      userId: number;
      groupId: number;
      targetUserId: number;
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
      userId: number;
      groupId: number;
      targetUserId: number;
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
