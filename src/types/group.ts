export type GroupStatus = "active" | "dissolved" | "unavailable";

export type GroupProfile = {
  group_id: string;
  group_name: string;
  owner_user_id: string;
  member_count: number;
  max_member_count: number;
  group_status: GroupStatus;
};

export type GroupRole = "owner" | "admin" | "member";

export type GroupMemberProfile = {
  group_id: string;
  user_id: string;
  card: string;
  title: string;
  role: GroupRole;
  joined_at: number;
  last_sent_at: number;
  mute_until: number | null;
};