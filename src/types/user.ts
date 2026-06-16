export type AccountStatus = "active" | "disabled" | "unavailable" | "deleted";

export type UserProfile = {
  user_id: string;
  nickname: string;
  avatar: string;
  signature: string;
  account_status: AccountStatus;
};

export type FriendCategory = {
  category_id: string;
  owner_user_id: string;
  name: string;
  sort_order: number;
  created_at: number;
  updated_at: number;
};

export type Friendship = {
  friend_user_id: string;
  category_id: string | null;
};
