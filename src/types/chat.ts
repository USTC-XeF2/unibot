export type MessageSource =
  | { scene: "private"; peer_user_id: string }
  | { scene: "group"; group_id: string };

export type MessageSegment =
  | { type: "Text"; data: { text: string } }
  | { type: "Image"; data: { file: string; url: string } }
  | { type: "At"; data: { target: string } }
  | { type: "AtAll" }
  | { type: "Face"; data: { id: string } };

export type ChatMessage = {
  id: string;
  sender_user_id: string;
  source: MessageSource;
  content: MessageSegment[];
  quoted_message_id: string | null;
  recall: {
    recalled: boolean;
    recalled_by_user_id?: string | null;
  };
  created_at: number;
};

export type ChatPoke = {
  poke_id: string;
  source: MessageSource;
  sender_user_id: string;
  target_user_id: string;
  created_at: number;
};