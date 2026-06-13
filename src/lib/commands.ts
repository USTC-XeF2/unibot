/**
 * 所有 Tauri 命令名的单一真相源（前端侧）。
 *
 * 命令名是 stringly-typed：前端 `invoke("name")` 的字符串必须和后端
 * `#[tauri::command]` 注册名逐字一致，拼错一个字母会到运行时才暴露
 * （参见 SQL 面板曾因 `is_write_query` vs `is_write_query_command` 失败）。
 *
 * 把命令名集中到这里有两个作用：
 * 1. 前端多处调用同一命令时只有一处真相，拼写不再各自维护。
 * 2. 后端有一个 Rust 测试（frontend_commands_are_all_registered）读取本文件的字符串值，
 *    断言每个都在 `lib.rs` 的 `generate_handler!` 注册表里，于是前后端漂移
 *    会在 `cargo test` 失败，而不是运行时。
 *
 * 维护约定：值必须和后端注册名完全一致；新增命令时同时更新这里和后端。
 */
export const COMMANDS = {
  // main
  registerUser: "register_user",
  listUsers: "list_users",
  deleteUser: "delete_user",
  openUserChatWindow: "open_user_chat_window",
  getDbStatus: "get_db_status",
  getStats: "get_stats",

  // log
  listSystemLogs: "list_system_logs",
  getLogSettings: "get_log_settings",
  setLogLevel: "set_log_level",
  setLogRetentionDays: "set_log_retention_days",
  triggerLogCleanup: "trigger_log_cleanup",

  // bot
  createBot: "create_bot",
  getBotConfig: "get_bot_config",
  listBots: "list_bots",
  renameBot: "rename_bot",
  deleteBot: "delete_bot",
  startBot: "start_bot",
  stopBot: "stop_bot",

  // user
  updateUserProfile: "update_user_profile",
  listFriends: "list_friends",
  listUserGroups: "list_user_groups",

  // message
  sendMessage: "send_message",
  listMessageHistory: "list_message_history",
  recallMessage: "recall_message",
  pokeUser: "poke_user",
  listPokeHistory: "list_poke_history",

  // friend request
  createFriendRequest: "create_friend_request",
  listFriendRequests: "list_friend_requests",
  handleFriendRequest: "handle_friend_request",
  deleteFriend: "delete_friend",

  // group
  listGroups: "list_groups",
  upsertGroup: "upsert_group",
  listGroupMembers: "list_group_members",
  renameGroup: "rename_group",
  dissolveGroup: "dissolve_group",
  leaveGroup: "leave_group",
  kickGroupMember: "kick_group_member",
  muteGroupMember: "mute_group_member",
  setGroupWholeMute: "set_group_whole_mute",
  setGroupMemberRole: "set_group_member_role",
  setGroupMemberTitle: "set_group_member_title",
  handleGroupRequest: "handle_group_request",
  listGroupRequests: "list_group_requests",
  listGroupEventHistory: "list_group_event_history",

  // group category
  listGroupCategories: "list_group_categories",
  createGroupCategory: "create_group_category",
  deleteGroupCategory: "delete_group_category",
  setGroupCategory: "set_group_category",

  // group content: album / photo / file
  listGroupAlbums: "list_group_albums",
  createGroupAlbum: "create_group_album",
  deleteGroupAlbum: "delete_group_album",
  listGroupPhotos: "list_group_photos",
  uploadGroupPhoto: "upload_group_photo",
  deleteGroupPhoto: "delete_group_photo",
  listGroupFiles: "list_group_files",
  uploadGroupFile: "upload_group_file",
  downloadGroupFile: "download_group_file",
  deleteGroupFile: "delete_group_file",

  // group content: folder
  listGroupFolders: "list_group_folders",
  upsertGroupFolder: "upsert_group_folder",

  // group content: announcement / essence
  listGroupAnnouncements: "list_group_announcements",
  upsertGroupAnnouncement: "upsert_group_announcement",
  listGroupEssenceMessages: "list_group_essence_messages",
  setGroupEssenceMessage: "set_group_essence_message",

  // group content: window
  openGroupFilesWindow: "open_group_files_window",
  openGroupAlbumsWindow: "open_group_albums_window",

  // conversation
  listConversationStates: "list_conversation_states",
  setConversationMuted: "set_conversation_muted",
  setConversationPinned: "set_conversation_pinned",

  // packet
  listProtocolPackets: "list_protocol_packets",
  readProtocolPacket: "read_protocol_packet",

  // dev tools
  openDeveloperTools: "open_developer_tools",
  getDbSchema: "get_db_schema",
  previewTableRows: "preview_table_rows",
  executeSql: "execute_sql",
  isWriteQueryCommand: "is_write_query_command",
} as const;

export type CommandName = (typeof COMMANDS)[keyof typeof COMMANDS];
