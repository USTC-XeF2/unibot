import type { MessageSource } from "@/types/chat";
import type { PacketFilters } from "@/types/packet";

function sourceTuple(
  source: MessageSource,
): readonly ["private" | "group", string] {
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
    byUser: (userId: string) => ["friends", userId] as const,
  },
  groups: {
    root: () => ["groups"] as const,
    all: () => ["groups", "all"] as const,
    byUser: (userId: string) => ["groups", "user", userId] as const,
    eventHistoryPrefix: (userId: string, groupId: string) =>
      ["groups", "event-history", userId, groupId] as const,
    eventHistory: (userId: string, groupId: string, limit: number) =>
      ["groups", "event-history", userId, groupId, limit] as const,
    members: (userId: string, groupId: string) =>
      ["groups", "members", userId, groupId] as const,
    categories: (userId: string) => ["groups", "categories", userId] as const,
  },
  conversation: {
    states: (userId: string) => ["conversation", "states", userId] as const,
  },
  requests: {
    friendByUser: (userId: string) => ["requests", "friend", userId] as const,
    manageableGroupPrefix: (userId: string) =>
      ["requests", "group-manageable", userId] as const,
    manageableGroup: (userId: string, groupIdsKey: string) =>
      ["requests", "group-manageable", userId, groupIdsKey] as const,
  },
  db: {
    status: () => ["db", "status"] as const,
  },
  logs: {
    system: (params: { before?: number; limit?: number }) =>
      ["logs", "system", params] as const,
    settings: () => ["logs", "settings"] as const,
  },
  bots: {
    all: () => ["bots"] as const,
    stats: () => ["stats"] as const,
    config: (botId: string) => ["bots", "config", botId] as const,
  },
  packets: {
    all: () => ["packets"] as const,
    list: (filters: PacketFilters) => ["packets", "list", filters] as const,
    detail: (id: string) => ["packets", "detail", id] as const,
  },
  chat: {
    historyByUser: (userId: string) => ["chat", "history", userId] as const,
    historyPrefix: (userId: string, source: MessageSource) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "history", userId, scene, targetId] as const;
    },
    history: (userId: string, source: MessageSource, limit: number) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "history", userId, scene, targetId, limit] as const;
    },
    pokeByUser: (userId: string) => ["chat", "poke-history", userId] as const,
    pokePrefix: (userId: string, source: MessageSource) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "poke-history", userId, scene, targetId] as const;
    },
    poke: (userId: string, source: MessageSource, limit: number) => {
      const [scene, targetId] = sourceTuple(source);
      return ["chat", "poke-history", userId, scene, targetId, limit] as const;
    },
  },
} as const;
