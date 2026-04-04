export type RequestState = "pending" | "accepted" | "rejected" | "ignored";
export type GroupRequestType = "join" | "invite";

export type FriendRequestEntity = {
  request_id: number;
  initiator_user_id: number;
  target_user_id: number;
  comment: string;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: number | null;
};

export type GroupRequestEntity = {
  request_id: number;
  group_id: number;
  request_type: GroupRequestType;
  initiator_user_id: number;
  target_user_id: number | null;
  comment: string | null;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: number | null;
};
