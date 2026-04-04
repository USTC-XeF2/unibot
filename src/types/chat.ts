export type MessageSource =
  | { scene: "private"; peer_user_id: number }
  | { scene: "group"; group_id: number };

export type MessageSegment =
  | { type: "Text"; data: { text: string } }
  | { type: "Image"; data: { file: string; url: string } }
  | { type: "At"; data: { target: number } }
  | { type: "AtAll" }
  | { type: "Face"; data: { id: string } };

export type ChatMessage = {
  id: number;
  sender_user_id: number;
  source: MessageSource;
  content: MessageSegment[];
  quoted_message_id: number | null;
  recall: {
    recalled: boolean;
    recalled_by_user_id?: number | null;
  };
  created_at: number;
};

export type ChatPoke = {
  poke_id: number;
  source: MessageSource;
  sender_user_id: number;
  target_user_id: number;
  created_at: number;
};
