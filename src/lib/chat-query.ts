import { queryOptions, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/query-client";
import type {
  ChatMessage,
  ChatPoke,
  InternalEventPayload,
  MessageSource,
} from "@/types/chat";

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

type HistoryQueryKey = readonly [
  "chat",
  "history",
  number,
  "private" | "group",
  number,
  number,
];

type PokeHistoryQueryKey = readonly [
  "chat",
  "poke-history",
  number,
  "private" | "group",
  number,
  number,
];

function sourceKey(
  source: MessageSource,
): readonly ["private" | "group", number] {
  if (source.scene === "private") {
    return ["private", source.peer_user_id] as const;
  }
  return ["group", source.group_id] as const;
}

export function messageHistoryQueryKey(
  userId: number,
  source: MessageSource,
  limit: number,
): HistoryQueryKey {
  const [scene, targetId] = sourceKey(source);
  return ["chat", "history", userId, scene, targetId, limit];
}

export function messageHistoryQueryOptions(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return queryOptions({
    queryKey: messageHistoryQueryKey(userId, source, limit),
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

export function pokeHistoryQueryKey(
  userId: number,
  source: MessageSource,
  limit: number,
): PokeHistoryQueryKey {
  const [scene, targetId] = sourceKey(source);
  return ["chat", "poke-history", userId, scene, targetId, limit];
}

export function pokeHistoryQueryOptions(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return queryOptions({
    queryKey: pokeHistoryQueryKey(userId, source, limit),
    queryFn: () =>
      invoke<ChatPoke[]>("list_poke_history", {
        userId,
        source,
        limit,
      }),
    retry: false,
  });
}

export function usePokeHistoryQuery(
  userId: number,
  source: MessageSource,
  limit: number,
) {
  return useQuery(pokeHistoryQueryOptions(userId, source, limit));
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
    queryKey: ["chat", "history", userId],
    refetchType: "active",
  });
}

export function invalidateMessageHistoryQuery(
  userId: number,
  source: MessageSource,
) {
  const [scene, targetId] = sourceKey(source);
  return queryClient.invalidateQueries({
    queryKey: ["chat", "history", userId, scene, targetId],
    refetchType: "active",
  });
}

export function invalidatePokeHistoryQueries(userId: number) {
  return queryClient.invalidateQueries({
    queryKey: ["chat", "poke-history", userId],
    refetchType: "active",
  });
}

export function invalidatePokeHistoryQuery(
  userId: number,
  source: MessageSource,
) {
  const [scene, targetId] = sourceKey(source);
  return queryClient.invalidateQueries({
    queryKey: ["chat", "poke-history", userId, scene, targetId],
    refetchType: "active",
  });
}
