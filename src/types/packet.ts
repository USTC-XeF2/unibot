export interface ProtocolPacket {
  packet_id: string;
  bot_id: string;
  profile_id: string;
  protocol_type: string;
  direction: "receive" | "send";
  action_name: string;
  file_path: string;
  related_object_type: string | null;
  related_object_id: string | null;
  is_error: boolean;
  session_id: string;
  created_at: number;
}

export interface PacketFilters {
  bot_id?: string;
  direction?: "receive" | "send";
  action_name?: string;
  since?: number;
  until?: number;
  is_error?: boolean;
  before?: number;
  limit?: number;
}
