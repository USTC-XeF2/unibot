import type { MessageSegment, MessageSource } from "./chat";
import type { GroupRequestType, RequestState } from "./request";

export type NoticeType = "mute" | "kick" | "admin_change";

export type GroupEventPayload =
  | {
      type: "member_joined";
      operator_user_id: string;
      joined_user_id: string;
    }
  | {
      type: "member_muted";
      operator_user_id: string;
      target_user_id: string;
      mute_until: number | null;
    }
  | {
      type: "essence_set";
      message_id: string;
      sender_user_id: string;
      operator_user_id: string;
    };

export type GroupEvent = {
  event_id: string;
  group_id: string;
  payload: GroupEventPayload;
  created_at: number;
};

export type InternalEventPayload =
  | {
      kind: "message";
      sender: string;
      group_id: string | null;
      content: MessageSegment[];
      time: number;
    }
  | {
      kind: "message_recalled";
      message_id: string;
      source: MessageSource;
      recalled_by_user_id: string;
      time: number;
    }
  | {
      kind: "message_reaction";
      reaction_id: string;
      message_id: string;
      source: MessageSource;
      operator_user_id: string;
      face_id: string;
      is_add: boolean;
      time: number;
    }
  | {
      kind: "poke";
      poke_id: string;
      source: MessageSource;
      sender_user_id: string;
      target_user_id: string;
      time: number;
    }
  | {
      kind: "friend_request_created";
      request_id: string;
      initiator_user_id: string;
      target_user_id: string;
      time: number;
    }
  | {
      kind: "friend_request_handled";
      request_id: string;
      initiator_user_id: string;
      target_user_id: string;
      operator_user_id: string;
      state: RequestState;
      time: number;
    }
  | {
      kind: "group_request_created";
      request_id: string;
      group_id: string;
      request_type: GroupRequestType;
      initiator_user_id: string;
      target_user_id: string | null;
      time: number;
    }
  | {
      kind: "group_request_handled";
      request_id: string;
      group_id: string;
      request_type: GroupRequestType;
      initiator_user_id: string;
      target_user_id: string | null;
      operator_user_id: string;
      state: RequestState;
      time: number;
    }
  | {
      kind: "group_member_muted";
      group_id: string;
      operator_user_id: string;
      target_user_id: string;
      mute_until: number | null;
      time: number;
    }
  | {
      kind: "group_member_joined";
      group_id: string;
      operator_user_id: string;
      target_user_id: string;
      time: number;
    }
  | {
      kind: "group_member_title_updated";
      group_id: string;
      operator_user_id: string;
      target_user_id: string;
      time: number;
    }
  | {
      kind: "group_whole_mute_updated";
      group_id: string;
      operator_user_id: string;
      muted: boolean;
      mute_until: number | null;
      time: number;
    }
  | {
      kind: "group_announcement_upserted";
      announcement_id: string;
      group_id: string;
      sender_user_id: string;
      time: number;
    }
  | {
      kind: "group_folder_upserted";
      folder_id: string;
      group_id: string;
      creator_user_id: string;
      time: number;
    }
  | {
      kind: "group_file_upserted";
      file_id: string;
      group_id: string;
      uploader_user_id: string;
      time: number;
    }
  | {
      kind: "group_essence_updated";
      essence_id: string;
      group_id: string;
      message_id: string;
      sender_user_id: string;
      operator_user_id: string;
      is_set: boolean;
      time: number;
    }
  | {
      kind: "notice";
      group_id: string;
      actor: string;
      target: string;
      notice_type: NoticeType;
      time: number;
    };