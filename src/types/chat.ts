export type MessageSource =
  | { scene: "private"; peer_user_id: number }
  | { scene: "group"; group_id: number };

export type MessageSegment =
  | { type: "Text"; data: { text: string } }
  | { type: "Image"; data: { file: string; url: string } }
  | { type: "At"; data: { target: number } }
  | { type: "Reply"; data: { message_id: number } }
  | { type: "Face"; data: { id: string } };

export type RequestState = "pending" | "accepted" | "rejected" | "ignored";
export type GroupRequestType = "join" | "invite";
export type NoticeType = "mute" | "kick" | "admin_change";

export type ChatMessage = {
  id: number;
  sender_user_id: number;
  source: MessageSource;
  content: MessageSegment[];
  recall: {
    recalled: boolean;
    recalled_by_user_id?: number | null;
    recalled_at?: number | null;
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

export type GroupEventPayload =
  | {
      type: "member_joined";
      operator_user_id: number;
      joined_user_id: number;
    }
  | {
      type: "member_muted";
      operator_user_id: number;
      target_user_id: number;
      mute_until: number | null;
    }
  | {
      type: "essence_set";
      message_id: number;
      sender_user_id: number;
      operator_user_id: number;
    };

export type GroupEvent = {
  event_id: number;
  group_id: number;
  payload: GroupEventPayload;
  created_at: number;
};

export type InternalEventPayload =
  | {
      kind: "message";
      sender: number;
      group_id: number | null;
      content: MessageSegment[];
      time: number;
    }
  | {
      kind: "message_recalled";
      message_id: number;
      source: MessageSource;
      recalled_by_user_id: number;
      time: number;
    }
  | {
      kind: "message_reaction";
      reaction_id: number;
      message_id: number;
      source: MessageSource;
      operator_user_id: number;
      face_id: string;
      is_add: boolean;
      time: number;
    }
  | {
      kind: "poke";
      poke_id: number;
      source: MessageSource;
      sender_user_id: number;
      target_user_id: number;
      time: number;
    }
  | {
      kind: "friend_request_created";
      request_id: number;
      initiator_user_id: number;
      target_user_id: number;
      time: number;
    }
  | {
      kind: "friend_request_handled";
      request_id: number;
      initiator_user_id: number;
      target_user_id: number;
      operator_user_id: number;
      state: RequestState;
      time: number;
    }
  | {
      kind: "group_request_created";
      request_id: number;
      group_id: number;
      request_type: GroupRequestType;
      initiator_user_id: number;
      target_user_id: number | null;
      time: number;
    }
  | {
      kind: "group_request_handled";
      request_id: number;
      group_id: number;
      request_type: GroupRequestType;
      initiator_user_id: number;
      target_user_id: number | null;
      operator_user_id: number;
      state: RequestState;
      time: number;
    }
  | {
      kind: "group_member_muted";
      group_id: number;
      operator_user_id: number;
      target_user_id: number;
      mute_until: number | null;
      time: number;
    }
  | {
      kind: "group_member_joined";
      group_id: number;
      operator_user_id: number;
      target_user_id: number;
      time: number;
    }
  | {
      kind: "group_member_title_updated";
      group_id: number;
      operator_user_id: number;
      target_user_id: number;
      time: number;
    }
  | {
      kind: "group_whole_mute_updated";
      group_id: number;
      operator_user_id: number;
      muted: boolean;
      mute_until: number | null;
      time: number;
    }
  | {
      kind: "group_announcement_upserted";
      announcement_id: string;
      group_id: number;
      sender_user_id: number;
      time: number;
    }
  | {
      kind: "group_folder_upserted";
      folder_id: string;
      group_id: number;
      creator_user_id: number;
      time: number;
    }
  | {
      kind: "group_file_upserted";
      file_id: string;
      group_id: number;
      uploader_user_id: number;
      time: number;
    }
  | {
      kind: "group_essence_updated";
      essence_id: number;
      group_id: number;
      message_id: number;
      sender_user_id: number;
      operator_user_id: number;
      is_set: boolean;
      time: number;
    }
  | {
      kind: "notice";
      group_id: number;
      actor: number;
      target: number;
      notice_type: NoticeType;
      time: number;
    };
