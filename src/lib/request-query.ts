import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { GroupProfile } from "@/types/group";
import { queryClient } from "./query-client";

export type RequestState = "pending" | "accepted" | "rejected" | "ignored";

export type FriendRequestEntity = {
  request_id: number;
  initiator_user_id: number;
  target_user_id: number;
  comment: string;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: number | null;
};

export type GroupRequestEntity = {
  request_id: number;
  group_id: number;
  request_type: "join" | "invite";
  initiator_user_id: number;
  target_user_id: number | null;
  comment: string | null;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: number | null;
};

type GroupMemberProfile = {
  group_id: number;
  user_id: number;
  role: "owner" | "admin" | "member";
};

export type PendingGroupRequestItem = GroupRequestEntity & {
  group_name: string;
};

function validUser(userId: number): boolean {
  return Number.isInteger(userId) && userId > 0;
}

export function friendRequestsQueryKey(userId: number) {
  return ["requests", "friend", userId] as const;
}

export function manageableGroupRequestsQueryKey(
  userId: number,
  groupIdsKey: string,
) {
  return ["requests", "group-manageable", userId, groupIdsKey] as const;
}

export function friendRequestsQueryOptions(userId: number) {
  return queryOptions({
    queryKey: friendRequestsQueryKey(userId),
    enabled: validUser(userId),
    retry: false,
    queryFn: () =>
      invoke<FriendRequestEntity[]>("list_friend_requests", {
        userId,
      }),
  });
}

export function useFriendRequestsQuery(userId: number, enabled = true) {
  const options = friendRequestsQueryOptions(userId);
  return useQuery({
    ...options,
    enabled: options.enabled && enabled,
  });
}

export function useManageableGroupRequestsQuery(
  userId: number,
  groups: GroupProfile[],
  enabled = true,
) {
  const groupIdsKey = groups
    .map((group) => group.group_id)
    .sort((a, b) => a - b)
    .join(",");

  return useQuery({
    queryKey: manageableGroupRequestsQueryKey(userId, groupIdsKey),
    enabled: enabled && validUser(userId),
    retry: false,
    queryFn: async () => {
      const allPendingGroupRequests = (
        await Promise.all(
          groups.map(async (group) => {
            try {
              const [members, requests] = await Promise.all([
                invoke<GroupMemberProfile[]>("list_group_members", {
                  userId,
                  groupId: group.group_id,
                }),
                invoke<GroupRequestEntity[]>("list_group_requests", {
                  userId,
                  groupId: group.group_id,
                }),
              ]);

              const myMember = members.find(
                (member) => member.user_id === userId,
              );
              const canManageByRole =
                myMember !== undefined && myMember.role !== "member";

              return requests
                .filter((request) => request.state === "pending")
                .filter((request) => {
                  if (request.request_type === "join") {
                    return canManageByRole;
                  }

                  return request.target_user_id === userId || canManageByRole;
                })
                .map(
                  (request): PendingGroupRequestItem => ({
                    ...request,
                    group_name:
                      group.group_name?.trim() || `群 ${request.group_id}`,
                  }),
                );
            } catch {
              return [] as PendingGroupRequestItem[];
            }
          }),
        )
      )
        .flat()
        .sort((a, b) => b.created_at - a.created_at);

      return allPendingGroupRequests;
    },
  });
}

export function invalidateFriendRequestsQuery(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: friendRequestsQueryKey(userId),
  });
}

export function invalidateManageableGroupRequestsQueries(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: ["requests", "group-manageable", userId],
  });
}
