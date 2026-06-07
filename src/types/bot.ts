export interface BotProfile {
  bot_id: string;
  bound_user_id: string;
  display_name: string;
  runtime_status: "stopped" | "running" | "error";
  config_path: string;
  created_at: number;
}

export interface DebugSession {
  session_id: string;
  bot_id: string;
  session_name: string;
  description: string | null;
  started_at: number;
  ended_at: number | null;
}

export interface StatsResult {
  total_messages: number;
  online_bots: number;
}
