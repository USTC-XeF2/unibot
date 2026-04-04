import type { MessageSource } from "@/types/chat";

function sourceTuple(
  source: MessageSource,
): readonly ["private" | "group", number] {
  if (source.scene === "private") {
    return ["private", source.peer_user_id] as const;
  }
  return ["group", source.group_id] as const;
}

export const queryKeys = {
  users: {
    all: () => ["users"] as const,
  },
  friends: {
    byUser: (userId: number) => ["friends", userId] as const,
  },
  groups: {
    root: () => ["groups"] as const,
    all: () => ["groups", "all"] as const,
    byUser: (userId: number) => ["groups", "user", userId] as const,
    eventHistoryPrefix: (userId: number, groupId: number) =>
      ["groups", "event-history", userId, groupId] as const,
    eventHistory: (userId: number, groupId: number, limit: number) =>
      ["groups", "event-history", userId, groupId, limit] as const,
    members: (userId: number, groupId: number) =>
      ["groups", "members", userId, groupId] as const,
  },
  requests: {
    friendByUser: (userId: number) => ["requests", "friend", userId] as const,
    manageableGroupPrefix: (userId: number) =>
      ["requests", "group-manageable", userId] as const,
    manageableGroup: (userId: number, groupIdsKey: string) =>
      ["requests", "group-manageable", userId, groupIdsKey] as const,
  },
  chat: {
    historyByUser: (userId: number) => ["chat", "history", userId] as const,
    historyPrefix: (userId: number, source: MessageSource) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "history", userId, scene, targetId] as const;
    },
    history: (userId: number, source: MessageSource, limit: number) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "history", userId, scene, targetId, limit] as const;
    },
    pokeByUser: (userId: number) => ["chat", "poke-history", userId] as const,
    pokePrefix: (userId: number, source: MessageSource) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "poke-history", userId, scene, targetId] as const;
    },
    poke: (userId: number, source: MessageSource, limit: number) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "poke-history", userId, scene, targetId, limit] as const;
    },
  },
} as const;
