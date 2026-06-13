import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { queryKeys } from "@/lib/query/keys";
import type { PacketFilters, ProtocolPacket } from "@/types/packet";

export function useProtocolPackets(filters: PacketFilters = {}) {
  return useQuery({
    queryKey: queryKeys.packets.list(filters),
    queryFn: async () => {
      return invoke<ProtocolPacket[]>(COMMANDS.listProtocolPackets, {
        botId: filters.bot_id ?? null,
        direction: filters.direction ?? null,
        actionName: filters.action_name ?? null,
        since: filters.since ?? null,
        limit: filters.limit ?? 100,
      });
    },
    refetchInterval: 2000,
    retry: false,
  });
}

export function useProtocolPacketDetail(packetId: string) {
  return useQuery({
    queryKey: queryKeys.packets.detail(packetId),
    queryFn: async () => {
      return invoke<string>(COMMANDS.readProtocolPacket, { packetId });
    },
    enabled: !!packetId,
    retry: false,
  });
}
