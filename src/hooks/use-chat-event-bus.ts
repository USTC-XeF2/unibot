import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import {
  invalidateFriendRequestsQuery,
  invalidateFriendsQuery,
  invalidateGroupEventHistoryQuery,
  invalidateGroupRequestsQueries,
  invalidateGroupsQuery,
  invalidateMessageHistoryQueries,
  invalidateMessageHistoryQuery,
  invalidatePokeHistoryQueries,
  invalidatePokeHistoryQuery,
  sourceFromInternalEvent,
} from "@/lib/query";
import type { InternalEventPayload } from "@/types/event";

type ChatEventSubscriber = (payload: InternalEventPayload) => void;

const subscribers = new Set<ChatEventSubscriber>();
let activeUserId: number | null = null;
let activeUnlisten: UnlistenFn | null = null;
let setupPromise: Promise<void> | null = null;

function dispatchToSubscribers(payload: InternalEventPayload) {
  for (const subscriber of subscribers) {
    subscriber(payload);
  }
}

function handleQueryInvalidation(
  userId: number,
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

function detachListener() {
  if (activeUnlisten) {
    activeUnlisten();
    activeUnlisten = null;
  }
  activeUserId = null;
}

async function ensureListener(userId: number) {
  if (activeUserId === userId && activeUnlisten) {
    return;
  }

  if (setupPromise) {
    await setupPromise;
    if (activeUserId === userId && activeUnlisten) {
      return;
    }
  }

  setupPromise = (async () => {
    detachListener();
    activeUserId = userId;
    activeUnlisten = await listen<InternalEventPayload>(
      "chat:event",
      (event) => {
        const payload = event.payload;
        if (!payload) {
          return;
        }

        handleQueryInvalidation(userId, payload);
        dispatchToSubscribers(payload);
      },
    );
  })();

  try {
    await setupPromise;
  } finally {
    setupPromise = null;
  }
}

export function useChatEventBus(
  userId: number,
  onEvent?: (payload: InternalEventPayload) => void,
) {
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  const subscriberRef = useRef<ChatEventSubscriber | null>(null);
  if (!subscriberRef.current) {
    subscriberRef.current = (payload) => {
      onEventRef.current?.(payload);
    };
  }

  useEffect(() => {
    if (!Number.isInteger(userId) || userId <= 0) {
      return;
    }

    const subscriber = subscriberRef.current;
    if (!subscriber) {
      return;
    }

    subscribers.add(subscriber);

    ensureListener(userId);

    return () => {
      subscribers.delete(subscriber);

      if (subscribers.size === 0) {
        detachListener();
      }
    };
  }, [userId]);
}
