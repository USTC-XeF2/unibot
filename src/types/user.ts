export type AccountStatus =
  | "active"
  | "disabled"
  | "unavailable"
  | "deleted";

export type UserProfile = {
  user_id: string;
  nickname: string;
  avatar: string;
  signature: string;
  account_status: AccountStatus;
};