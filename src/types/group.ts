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

export type GroupCategory = {
  category_id: string;
  owner_user_id: string;
  name: string;
  sort_order: number;
  created_at: number;
  updated_at: number;
};

export type ConversationState = {
  conversation_scene: "private" | "group" | "temp";
  peer_user_id: string | null;
  group_id: string | null;
  is_pinned: boolean;
  is_muted: boolean;
};

export type GroupFile = {
  file_id: string;
  group_id: string;
  parent_folder_id: string;
  file_name: string;
  file_size: number;
  file_hash: string | null;
  uploader_user_id: string;
  uploaded_at: number;
  expire_at: number | null;
  download_count: number;
  file_path: string | null;
};

export type GroupFolder = {
  folder_id: string;
  group_id: string;
  parent_folder_id: string;
  folder_name: string;
  creator_user_id: string;
  created_at: number;
  updated_at: number;
  file_count: number;
};

export type GroupAlbum = {
  album_id: string;
  group_id: string;
  name: string;
  cover_url: string | null;
  photo_count: number;
  created_at: number;
  updated_at: number;
};

export type GroupPhoto = {
  photo_id: string;
  album_id: string;
  group_id: string;
  url: string;
  file_path: string | null;
  description: string | null;
  uploader_user_id: string;
  file_size: number | null;
  created_at: number;
};

export type GroupAnnouncement = {
  announcement_id: string;
  group_id: string;
  sender_user_id: string;
  content: string;
  image_url: string | null;
  created_at: number;
  updated_at: number;
};

export type GroupEssenceMessage = {
  essence_id: string;
  group_id: string;
  message_id: string;
  sender_user_id: string;
  operator_user_id: string;
  is_set: boolean;
  created_at: number;
};
