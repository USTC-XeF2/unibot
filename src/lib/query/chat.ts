import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryKeys } from "@/lib/query/keys";
import { queryClient } from "@/lib/query-client";
import type { ChatMessage, ChatPoke, MessageSource } from "@/types/chat";
import type { InternalEventPayload } from "@/types/event";

function normalizePrivateSourceFromEvent(
  source: MessageSource,
  currentUserId: number,
  peerCandidate: number | null,
): MessageSource {
  if (source.scene !== "private") {
    return source;
  }

  if (source.peer_user_id !== currentUserId) {
    return source;
  }

  if (!peerCandidate || peerCandidate <= 0 || peerCandidate === currentUserId) {
    return source;
  }

  return { scene: "private", peer_user_id: peerCandidate };
}

export function messageHistoryQueryOptions(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return queryOptions({
    queryKey: queryKeys.chat.history(userId, source, limit),
    queryFn: () =>
      invoke<ChatMessage[]>("list_message_history", {
        userId,
        source,
        limit,
      }),
    retry: false,
  });
}

export function useMessageHistoryQuery(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return useQuery(messageHistoryQueryOptions(userId, source, limit));
}

export function usePokeHistoryQuery(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return useQuery({
    queryKey: queryKeys.chat.poke(userId, source, limit),
    queryFn: () =>
      invoke<ChatPoke[]>("list_poke_history", {
        userId,
        source,
        limit,
      }),
    retry: false,
  });
}

export function sourceFromInternalEvent(
  payload: InternalEventPayload,
  currentUserId: number,
): MessageSource | null {
  if (payload.kind === "message") {
    if (payload.group_id) {
      return { scene: "group", group_id: payload.group_id };
    }

    if (payload.sender !== currentUserId) {
      return { scene: "private", peer_user_id: payload.sender };
    }
    return null;
  }

  if (payload.kind === "message_recalled") {
    return normalizePrivateSourceFromEvent(
      payload.source,
      currentUserId,
      payload.recalled_by_user_id,
    );
  }

  if (payload.kind === "message_reaction") {
    return normalizePrivateSourceFromEvent(
      payload.source,
      currentUserId,
      payload.operator_user_id,
    );
  }

  if (payload.kind === "poke") {
    const peerCandidate =
      payload.sender_user_id === currentUserId
        ? payload.target_user_id
        : payload.sender_user_id;
    return normalizePrivateSourceFromEvent(
      payload.source,
      currentUserId,
      peerCandidate,
    );
  }

  if (
    payload.kind === "friend_request_created" ||
    payload.kind === "friend_request_handled"
  ) {
    const peerUserId =
      payload.initiator_user_id === currentUserId
        ? payload.target_user_id
        : payload.initiator_user_id;
    return { scene: "private", peer_user_id: peerUserId };
  }

  if (
    payload.kind === "group_request_created" ||
    payload.kind === "group_request_handled" ||
    payload.kind === "group_member_joined" ||
    payload.kind === "group_member_muted" ||
    payload.kind === "group_member_title_updated" ||
    payload.kind === "group_whole_mute_updated" ||
    payload.kind === "group_announcement_upserted" ||
    payload.kind === "group_folder_upserted" ||
    payload.kind === "group_file_upserted" ||
    payload.kind === "group_essence_updated" ||
    payload.kind === "notice"
  ) {
    return { scene: "group", group_id: payload.group_id };
  }

  return null;
}

export function invalidateMessageHistoryQueries(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.chat.historyByUser(userId),
    refetchType: "active",
  });
}

export function invalidateMessageHistoryQuery(
  userId: number,
  source: MessageSource,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.chat.historyPrefix(userId, source),
    refetchType: "active",
  });
}

export function invalidatePokeHistoryQueries(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.chat.pokeByUser(userId),
    refetchType: "active",
  });
}

export function invalidatePokeHistoryQuery(
  userId: number,
  source: MessageSource,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.chat.pokePrefix(userId, source),
    refetchType: "active",
  });
}
