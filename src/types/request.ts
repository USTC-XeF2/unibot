export type RequestState = "pending" | "accepted" | "rejected" | "ignored";
export type GroupRequestType = "join" | "invite";

export type FriendRequestEntity = {
  request_id: string;
  initiator_user_id: string;
  target_user_id: string;
  comment: string;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
};

export type GroupRequestEntity = {
  request_id: string;
  group_id: string;
  request_type: GroupRequestType;
  initiator_user_id: string;
  target_user_id: string | null;
  comment: string | null;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: string | null;
};