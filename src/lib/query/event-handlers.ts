import {
  invalidateMessageHistoryQueries,
  invalidateMessageHistoryQuery,
  invalidatePokeHistoryQueries,
  invalidatePokeHistoryQuery,
  sourceFromInternalEvent,
} from "@/lib/query/chat";
import { invalidateFriendsQuery } from "@/lib/query/friends";
import {
  invalidateGroupEventHistoryQuery,
  invalidateGroupsQuery,
} from "@/lib/query/groups";
import {
  invalidateFriendRequestsQuery,
  invalidateGroupRequestsQueries,
} from "@/lib/query/requests";
import type { InternalEventPayload } from "@/types/event";

/**
 * 收到 chat:event 后统一执行的 query 失效逻辑。
 * 由 ChatEventBusProvider 在每个窗口内调用一次。
 */
export function handleQueryInvalidation(
  userId: string,
  payload: InternalEventPayload,
) {
  const source = sourceFromInternalEvent(payload, userId);
  if (source) {
    invalidateMessageHistoryQuery(userId, source);
    invalidatePokeHistoryQuery(userId, source);
    if (source.scene === "group") {
      invalidateGroupEventHistoryQuery(userId, source.group_id);
    }
  } else {
    invalidateMessageHistoryQueries(userId);
    invalidatePokeHistoryQueries(userId);
  }

  if (
    payload.kind === "friend_request_created" ||
    payload.kind === "friend_request_handled" ||
    payload.kind === "group_request_created" ||
    payload.kind === "group_request_handled"
  ) {
    invalidateFriendRequestsQuery(userId);
    invalidateGroupRequestsQueries(userId);

    if (payload.kind === "friend_request_handled") {
      invalidateFriendsQuery(userId);
    }

    if (
      payload.kind === "group_request_handled" &&
      payload.state === "accepted"
    ) {
      const shouldRefreshGroups =
        payload.initiator_user_id === userId ||
        payload.target_user_id === userId;
      if (shouldRefreshGroups) {
        invalidateGroupsQuery();
      }
    }
  }

  if (
    payload.kind === "group_member_joined" &&
    payload.target_user_id === userId
  ) {
    invalidateGroupsQuery();
  }
}
