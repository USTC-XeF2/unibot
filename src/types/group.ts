export type GroupProfile = {
  group_id: number;
  group_name: string;
  owner_user_id: number;
  member_count: number;
  max_member_count: number;
};

export type GroupRole = "owner" | "admin" | "member";

export type GroupMemberProfile = {
  group_id: number;
  user_id: number;
  card: string;
  title: string;
  role: GroupRole;
  joined_at: number;
  last_sent_at: number;
  mute_until: number | null;
};
