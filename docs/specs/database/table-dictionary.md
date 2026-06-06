# 表字典

26 张表完整字段、类型、约束说明。按数据域组织。

**命名约定**：表名 `snake_case` 复数，字段 `snake_case` 单数。所有 ID 统一为 `TEXT` 类型。

---

## 1. 身份与社交域 (IM_ACCOUNT)（7 表）

### im_accounts

用途：统一 IM 身份。模拟账号、Bot 绑定账号、真实 QQ 联系人缓存均以此表表示。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| user_id | TEXT | NN | — | PK | 账号唯一标识 |
| nickname | TEXT | NN | — | — | 昵称 |
| avatar_url | TEXT | NN | '' | — | 头像 URL |
| signature | TEXT | NN | '' | — | 个性签名 |
| account_source | TEXT | NN | 'simulated' | CHECK IN ('simulated','real') | 环境来源 |
| origin_user_id | TEXT | Y | — | — | 跨环境关联：simulated 行可指向同人 real 行 user_id |
| qid | TEXT | Y | — | — | QID 靓号（非 QQ 号） |
| age | INTEGER | Y | — | — | 年龄 |
| sex | TEXT | Y | — | — | 性别 |
| level | INTEGER | Y | — | — | 等级 |
| bio | TEXT | Y | — | — | 个人简介 |
| created_at | INTEGER | NN | — | — | Unix 毫秒时间戳 |

索引：PK 索引覆盖主查询，无额外索引。

### account_faces

用途：IM 账号的自定义表情（对应 QQ `MarketfaceEntity`）。系统表情走 `faces.json` 不入库。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| face_id | TEXT | NN | — | PK | 表情唯一标识 |
| owner_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 所属账号 |
| face_name | TEXT | Y | — | — | 表情名称 |
| emoji_package_id | INTEGER | Y | — | — | 表情包 ID |
| key | TEXT | Y | — | — | 协议凭证 |
| remote_url | TEXT | Y | — | — | QQ 服务器 URL |
| local_path | TEXT | Y | — | — | 本地缓存路径 |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_account_faces_owner ON account_faces(owner_user_id)`。

约束：`CHECK (face_id GLOB '*[^0-9]*')`——不能是纯数字，与系统表情格式隔离。

### friend_categories

用途：好友分类。per-user 归属，每个账号独立管理。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| category_id | TEXT | NN | — | PK | 分类唯一标识 |
| owner_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 归属账号 |
| name | TEXT | NN | — | — | 分类名称 |
| sort_order | INTEGER | NN | 0 | — | 排序 |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_friend_categories_owner ON friend_categories(owner_user_id)`。

约束说明：`FRIENDSHIP.friend_category_id` 为 NOT NULL 且 ON DELETE RESTRICT——好友必须入组，有好友时不能删分组。

### friendships

用途：好友关系（owner 视角）。A 看 B 和 B 看 A 各一条记录，备注和置顶各自独立。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| owner_user_id | TEXT | NN | — | PK, FK → im_accounts(user_id) ON DELETE CASCADE | 好友关系持有者 |
| friend_user_id | TEXT | NN | — | PK, FK → im_accounts(user_id) ON DELETE CASCADE | 好友方 |
| friend_category_id | TEXT | NN | — | FK → friend_categories(category_id) ON DELETE RESTRICT | 好友分组，必填 |
| remark | TEXT | Y | — | — | 备注名 |
| is_pinned | INTEGER | NN | 0 | — | 0/1 |
| created_at | INTEGER | NN | — | — | — |

唯一约束：`UNIQUE(owner_user_id, friend_user_id)`。

索引：`idx_friendships_friend ON friendships(friend_user_id)`。

### friend_requests

用途：好友申请。状态机：pending → accepted / rejected / ignored。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| request_id | TEXT | NN | — | PK | 申请唯一标识 |
| initiator_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 发起人 |
| target_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 目标人 |
| comment | TEXT | Y | — | — | 申请理由 |
| state | TEXT | NN | 'pending' | CHECK IN ('pending','accepted','rejected','ignored') | — |
| created_at | INTEGER | NN | — | — | — |
| handled_at | INTEGER | Y | — | — | 处理时间 |

唯一约束：`UNIQUE(initiator_user_id, target_user_id) WHERE state = 'pending'`（部分唯一索引，防重复申请）。

索引：`idx_friend_req_target ON friend_requests(target_user_id, state, created_at) WHERE state = 'pending'`（部分索引）。

### group_categories

用途：群分类。per-user 归属，便于按分类组织群组。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| category_id | TEXT | NN | — | PK | — |
| owner_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 归属账号 |
| name | TEXT | NN | — | — | 分类名称 |
| sort_order | INTEGER | NN | 0 | — | 排序 |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_group_categories_owner ON group_categories(owner_user_id)`, `idx_group_categories_name ON group_categories(owner_user_id, name)` (UNIQUE, 同账号下不重名)。

### user_groups

用途：账号视角下的群视图（owner 视角）。承载 per-user 的群分类、置顶、免打扰等属性，解除 `CHAT_GROUP` 上的 per-user 属性耦合。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| owner_user_id | TEXT | NN | — | PK, FK → im_accounts(user_id) ON DELETE CASCADE | 账号视角 |
| group_id | TEXT | NN | — | PK, FK → chat_groups(group_id) ON DELETE CASCADE | 群 |
| category_id | TEXT | Y | — | FK → group_categories(category_id) ON DELETE SET NULL | 该账号下的群分类 |
| is_pinned | INTEGER | NN | 0 | — | 0/1 |
| is_muted | INTEGER | NN | 0 | — | 0/1 |
| sort_order | INTEGER | NN | 0 | — | 排序 |
| joined_at | INTEGER | Y | — | — | 加入时间 |
| last_active_at | INTEGER | Y | — | — | 最后活跃时间 |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_user_groups_category ON user_groups(owner_user_id, category_id)`, `idx_user_groups_group ON user_groups(group_id)`。

触发器：`trg_user_group_category_owner` — 保证 `category_id` 属于同一个 `owner_user_id`。

---

## 2. 群组与内容域 (CHAT_GROUP)（10 表）

### chat_groups

用途：群组基本信息。`group_source` 区分模拟群/真实群，与 `IM_ACCOUNT.account_source` 严格隔离。per-user 属性（分类、置顶、免打扰）由 `user_groups` 承载。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| group_id | TEXT | NN | — | PK | 群唯一标识 |
| group_name | TEXT | NN | — | — | 群名称 |
| group_source | TEXT | NN | 'simulated' | CHECK IN ('simulated','real') | 模拟/真实 |
| avatar_url | TEXT | Y | — | — | 群头像 URL |
| group_owner_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 真实群主 |
| member_count | INTEGER | NN | 0 | CHECK(>= 0 AND <= max_member_count) | 当前成员数（触发器维护） |
| max_member_count | INTEGER | NN | 500 | — | 最大成员数 |
| is_whole_muted | INTEGER | NN | 0 | — | 全员禁言 |
| mute_until | INTEGER | Y | — | — | 全员禁言截止时间 |
| mute_operator_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 禁言操作者 |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_chat_groups_group_owner ON chat_groups(group_owner_user_id)`。

### group_members

用途：群成员关系属性。承载角色、群名片、禁言等。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| group_id | TEXT | NN | — | PK, FK → chat_groups(group_id) ON DELETE CASCADE | — |
| user_id | TEXT | NN | — | PK, FK → im_accounts(user_id) ON DELETE CASCADE | — |
| card | TEXT | NN | '' | — | 群名片 |
| special_title | TEXT | Y | '' | — | 专属头衔 |
| role | TEXT | NN | 'member' | CHECK IN ('owner','admin','member') | 群内角色 |
| joined_at | INTEGER | NN | — | — | 入群时间 |
| last_sent_at | INTEGER | NN | 0 | — | 最后发言时间 |
| mute_until | INTEGER | Y | — | — | 个人禁言截止 |

索引：`idx_group_members_user ON group_members(user_id)`, `idx_group_members_role ON group_members(group_id, role)`。

触发器：AFTER INSERT → `chat_groups.member_count + 1`；AFTER DELETE → `chat_groups.member_count - 1`。BEFORE INSERT → 校验 `account_source` 与 `group_source` 一致。

### group_requests

用途：群通知/申请。需要处理（有状态流转），区别于被动记录的 GROUP_EVENT。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| group_id | TEXT | NN | — | PK, FK → chat_groups(group_id) ON DELETE CASCADE | — |
| notification_seq | TEXT | NN | — | PK | 协议端通知序号 |
| notification_type | TEXT | NN | — | — | join_request / invite |
| initiator_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 发起人 |
| target_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 被邀请人 |
| comment | TEXT | Y | — | — | 备注 |
| state | TEXT | NN | 'pending' | CHECK IN ('pending','accepted','rejected','ignored') | — |
| created_at | INTEGER | NN | — | — | — |
| handled_at | INTEGER | Y | — | — | 处理时间 |
| operator_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 处理人 |

索引：`idx_group_req_initiator ON group_requests(initiator_user_id)`, `idx_group_req_pending ON group_requests(group_id, state, created_at) WHERE state = 'pending'`（部分索引）。

### group_announcements

用途：群公告。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| announcement_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| sender_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 发布者 |
| content | TEXT | NN | — | — | 公告内容 |
| image_url | TEXT | Y | — | — | 图片 URL |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_announcements_group ON group_announcements(group_id, created_at DESC)`。

### group_folders

用途：群文件夹。支持自关联层级结构。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| folder_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| parent_folder_id | TEXT | — | — | FK → group_folders(folder_id) ON DELETE CASCADE | 根目录为 NULL |
| folder_name | TEXT | NN | — | — | 文件夹名 |
| creator_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | — |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_folders_group_parent ON group_folders(group_id, parent_folder_id)`。

### group_files

用途：群文件。支持秒传（file_hash）和过期时间。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| file_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| parent_folder_id | TEXT | Y | — | FK → group_folders(folder_id) ON DELETE CASCADE | NULL=群文件根目录 |
| file_name | TEXT | NN | — | — | 文件名 |
| file_size | INTEGER | NN | — | — | 文件大小（字节） |
| file_hash | TEXT | Y | — | — | 秒传/校验 Hash |
| uploader_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 上传者 |
| created_at | INTEGER | NN | — | — | — |
| expire_at | INTEGER | Y | — | — | 过期时间 |
| download_count | INTEGER | NN | 0 | — | 下载次数 |

索引：`idx_group_files_group_time ON group_files(group_id, created_at DESC)`。

### group_essence_messages

用途：精华消息。取消精华即删除行，不保留历史。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| essence_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| message_id | TEXT | Y | — | FK → messages(message_id) ON DELETE SET NULL | 消息删除后精华保留 |
| sender_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 消息原发送者 |
| operator_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 加精操作者 |
| created_at | INTEGER | NN | — | — | — |

唯一约束：`UNIQUE(group_id, message_id)`——每条消息在一个群内最多加精一次。

索引：`idx_essence_group ON group_essence_messages(group_id, created_at DESC)`。

### group_events

用途：群事件记录。只追加，不修改不删除。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| event_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| event_type | TEXT | NN | — | — | member_join / member_mute / essence_set 等 |
| payload_json | TEXT | NN | — | — | 事件载荷 JSON |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_group_events_group_type ON group_events(group_id, event_type, created_at DESC)`。

### group_albums

用途：群相册。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| album_id | TEXT | NN | — | PK | — |
| group_id | TEXT | NN | — | FK → chat_groups(group_id) ON DELETE CASCADE | — |
| name | TEXT | NN | — | — | 相册名称 |
| cover_url | TEXT | Y | — | — | 封面 URL |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

索引：`idx_albums_group ON group_albums(group_id)`。

### group_photos

用途：群照片。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| photo_id | TEXT | NN | — | PK | — |
| album_id | TEXT | NN | — | FK → group_albums(album_id) ON DELETE CASCADE | — |
| url | TEXT | NN | — | — | 图片 URL |
| description | TEXT | Y | — | — | 描述 |
| uploader_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 上传者 |
| file_size | INTEGER | Y | — | — | 文件大小 |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_photos_album ON group_photos(album_id, created_at DESC)`。

---

## 3. 会话与消息域 (CONVERSATION)（4 表）

### conversations

用途：用户视角的会话列表项。独立于消息表，支持空会话。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| conversation_id | TEXT | NN | — | PK | — |
| owner_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 会话属主 |
| conversation_scene | TEXT | NN | — | CHECK IN ('private','group','temp') | 会话场景 |
| peer_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE CASCADE | 私聊对端（scene='private'/'temp' 时 NN） |
| group_id | TEXT | Y | — | FK → chat_groups(group_id) ON DELETE CASCADE | 群 ID（scene='group' 时 NN） |
| last_message_id | TEXT | Y | — | FK → messages(message_id) ON DELETE SET NULL | 最近消息 |
| last_read_seq | TEXT | Y | — | — | 最后已读消息序号（TEXT） |
| unread_count | INTEGER | NN | 0 | CHECK(>= 0) | 未读数（触发器 + 应用层维护） |
| is_pinned | INTEGER | NN | 0 | — | 0/1 |
| is_muted | INTEGER | NN | 0 | — | 免打扰 |
| updated_at | INTEGER | NN | — | — | Unix 毫秒时间戳 |

部分唯一索引：`uq_conversation_private ON conversations(owner_user_id, conversation_scene, peer_user_id) WHERE conversation_scene IN ('private','temp')`；`uq_conversation_group ON conversations(owner_user_id, conversation_scene, group_id) WHERE conversation_scene = 'group'`。使用部分索引而非普通 UNIQUE 约束——因为 peer_user_id/group_id 互斥为 NULL，普通 UNIQUE 对 nullable 列不保证唯一性。

CHECK 互斥：conversation_scene='private'/'temp' → peer_user_id NN ∧ group_id NULL；conversation_scene='group' → 相反。

索引：`idx_conv_owner_updated ON conversations(owner_user_id, updated_at DESC)`, `idx_conv_unread ON conversations(owner_user_id, unread_count) WHERE unread_count > 0`（部分索引）。

### messages

用途：所有消息（私聊/群聊/临时会话）。消息内容 JSON 存储可变段，核心字段结构化。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| message_id | TEXT | NN | — | PK | — |
| message_scene | TEXT | NN | — | CHECK IN ('private','group','temp') | 消息场景 |
| peer_id | TEXT | NN | — | — | 会话对端标识 |
| message_seq | TEXT | NN | — | — | 消息序号（TEXT） |
| sender_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 发送者；账号删除后置空以保留消息 |
| receiver_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 接收者（私聊） |
| group_id | TEXT | Y | — | FK → chat_groups(group_id) ON DELETE SET NULL | 群 ID |
| bot_id | TEXT | Y | — | — | 反规范化字段：sender 绑定的 Bot ID |
| content_json | TEXT | NN | — | — | 消息段数组 JSON |
| quoted_message_id | TEXT | Y | — | FK → messages(message_id) ON DELETE SET NULL | 引用消息 |
| forward_id | TEXT | Y | — | — | 合并转发 ID（非 FK，兜底补拉用） |
| is_recalled | INTEGER | NN | 0 | — | 撤回标记 |
| recalled_by_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 撤回操作者 |
| recalled_at | INTEGER | Y | — | — | 撤回时间 |
| session_id | TEXT | Y | — | FK → debug_sessions(session_id) ON DELETE SET NULL | 调试会话分组 |
| created_at | INTEGER | NN | — | — | — |

唯一约束：`UNIQUE(message_scene, peer_id, message_seq)`。

CHECK 互斥：message_scene='private'/'temp' → receiver_user_id NN ∧ group_id NULL；message_scene='group' → 相反。

索引：`idx_msg_scene_peer_time ON messages(message_scene, peer_id, created_at DESC)`, `idx_msg_sender_time ON messages(sender_user_id, created_at)`, `idx_msg_bot_time ON messages(bot_id, created_at DESC)`, `idx_msg_quoted ON messages(quoted_message_id)`。

反规范化说明：`bot_id` 可从 `sender_user_id → bots.bound_user_id` 推导（3 表 JOIN），为调试查询性能有意反规范化。

### message_reactions

用途：消息表情回应。使用 `is_add` 标记添加/移除（不 DELETE）。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| reaction_id | TEXT | NN | — | PK | — |
| message_id | TEXT | NN | — | FK → messages(message_id) ON DELETE CASCADE | 被反应消息 |
| operator_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 操作者 |
| face_id | TEXT | NN | — | — | QQ 内置表情 ID（TEXT） |
| is_add | INTEGER | NN | 1 | — | 1=添加 0=移除 |
| created_at | INTEGER | NN | — | — | — |

不建立 `(message_id, operator_user_id, face_id)` 历史唯一约束：添加和移除都需要以 `is_add` 追加记录；"同一用户不能重复添加同一反应"由应用层按当前净状态拦截。

索引：`idx_reactions_msg ON message_reactions(message_id)`。

### pokes

用途：戳一戳互动。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| poke_id | TEXT | NN | — | PK | — |
| sender_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 发起人 |
| target_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 目标人 |
| message_scene | TEXT | NN | — | CHECK IN ('private','group') | 会话场景（不含 temp） |
| peer_id | TEXT | NN | — | — | 会话对端 |
| is_recalled | INTEGER | NN | 0 | — | 撤回标记 |
| recalled_by_user_id | TEXT | Y | — | FK → im_accounts(user_id) ON DELETE SET NULL | 撤回操作者 |
| recalled_at | INTEGER | Y | — | — | 撤回时间 |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_pokes_scene_peer ON pokes(message_scene, peer_id, created_at DESC)`。

---

## 4. Bot 与调试域 (BOT)（2 表）

### bots

用途：被系统托管或调试的机器人实例。连接配置与行为配置存储在 JSON 文件中，`config_path` 指向文件路径。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| bot_id | TEXT | NN | — | PK | Bot 唯一标识 |
| bound_user_id | TEXT | NN | — | FK → im_accounts(user_id) ON DELETE CASCADE | 绑定的 IM 账号 |
| display_name | TEXT | NN | — | — | 展示名称 |
| runtime_status | TEXT | NN | 'stopped' | CHECK IN ('stopped','running','error') | 运行状态 |
| config_path | TEXT | NN | — | — | 配置文件路径（如 `configs/bots/bot_001.json`） |
| created_at | INTEGER | NN | — | — | — |
| updated_at | INTEGER | NN | — | — | — |

唯一索引：`idx_bots_bound_user ON bots(bound_user_id)`，保证一个 IM 账号最多绑定一个 Bot。

设计说明：`config_path` 指向文件系统中的 JSON 配置文件，内容包含连接配置（connections）、私聊行为配置（behavior.private）、群聊行为配置（behavior.group_overrides）、分类级别配置（behavior.category_defaults）等。参考 LangBot `adapter_config` 做法，数据库不存储 JSON 内容本身。

### debug_sessions

用途：调试会话。分组回看一轮测试的所有消息/报文。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| session_id | TEXT | NN | — | PK | — |
| bot_id | TEXT | NN | — | FK → bots(bot_id) ON DELETE CASCADE | 调试目标 Bot |
| session_name | TEXT | NN | — | — | 会话名称 |
| description | TEXT | Y | — | — | 说明 |
| started_at | INTEGER | NN | — | — | 开始时间（DEFAULT unixepoch 毫秒） |
| ended_at | INTEGER | Y | — | CHECK(ended_at IS NULL OR ended_at >= started_at) | 结束时间 |

索引：`idx_debug_sessions_bot ON debug_sessions(bot_id, started_at DESC)`。

---

## 5. 系统与审计域 (System)（3 表）

### protocol_packets

用途：协议报文审计。结构化索引字段存库，原始 JSON 存文件系统，文件路径直接保存在 `file_path`。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| packet_id | TEXT | NN | — | PK | — |
| bot_id | TEXT | Y | — | FK → bots(bot_id) ON DELETE SET NULL | Bot（可空：系统级报文） |
| profile_id | TEXT | Y | — | — | 连接标识（无 FK） |
| protocol_type | TEXT | NN | — | — | mock / Milky / OneBot-v11 / OneBot-v12 等 |
| direction | TEXT | NN | — | CHECK IN ('send','receive') | 方向 |
| action_name | TEXT | NN | — | — | 协议动作名 |
| file_path | TEXT | NN | — | — | 原始 JSON 文件路径 |
| related_object_type | TEXT | Y | — | — | 多态关联类型：message / group_request / group_event |
| related_object_id | TEXT | Y | — | — | 多态关联 ID（非 FK，应用层保证） |
| is_error | INTEGER | NN | 0 | — | 协议错误标记（结构化，去 JSON 解析） |
| session_id | TEXT | Y | — | FK → debug_sessions(session_id) ON DELETE SET NULL | 调试会话分组 |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_packet_bot_time ON protocol_packets(bot_id, created_at DESC)`, `idx_packet_error_time ON protocol_packets(is_error, created_at DESC)`, `idx_packet_related ON protocol_packets(related_object_type, related_object_id)`, `idx_packet_session ON protocol_packets(session_id)`。

原始 JSON 文件存在性采用懒惰检查：查看或导出时读取 `file_path`，读取失败则提示文件已丢失或过期。

### app_settings

用途：应用设置。Key-Value 结构，全局级别。`schema.version` 键管理当前数据库 schema 版本。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| setting_key | TEXT | NN | — | PK | 设置键 |
| setting_value | TEXT | NN | — | — | 设置值 |
| value_type | TEXT | NN | 'string' | CHECK IN ('string','int','bool','json') | 值类型 |
| description | TEXT | Y | — | — | 说明 |
| updated_at | INTEGER | NN | — | — | — |

### audit_events

用途：操作审计日志。记录应用层操作，保留期内只追加，按类型执行保留策略。

| 字段 | 类型 | 空 | 默认值 | 约束 | 说明 |
|---|---|---|---|---|---|
| event_id | TEXT | NN | — | PK | — |
| event_type | TEXT | NN | — | — | bot.start / bot.stop / message.delete 等 |
| actor_user_id | TEXT | Y | — | — | 操作者（系统操作为 NULL，不建 FK） |
| target_type | TEXT | Y | — | CHECK IN ('bot','message','connection','group','user') | 多态，非 FK |
| target_id | TEXT | Y | — | — | 被操作实体 ID |
| detail_json | TEXT | Y | — | — | 结构化详情 JSON |
| created_at | INTEGER | NN | — | — | — |

索引：`idx_audit_type_time ON audit_events(event_type, created_at DESC)`, `idx_audit_actor ON audit_events(actor_user_id, created_at DESC)`。

审计表不对 `actor_user_id`、`target_type`、`target_id` 建外键，避免被审计对象删除后破坏审计日志完整性；应用层负责解析这些引用。

---

## 6. FK 级联策略汇总

| 父表 | 子表 | FK 字段 | ON DELETE |
|---|---|---|---|
| im_accounts | bots | bound_user_id | CASCADE |
| im_accounts | friendships | owner_user_id, friend_user_id | CASCADE |
| im_accounts | friend_requests | initiator_user_id, target_user_id | CASCADE |
| im_accounts | group_members | user_id | CASCADE |
| im_accounts | messages | sender_user_id, receiver_user_id | SET NULL |
| im_accounts | conversations | owner_user_id, peer_user_id | CASCADE |
| im_accounts | group_essence_messages | sender_user_id, operator_user_id | CASCADE |
| im_accounts | chat_groups | group_owner_user_id | SET NULL |
| im_accounts | chat_groups | mute_operator_user_id | SET NULL |
| im_accounts | group_announcements | sender_user_id | CASCADE |
| im_accounts | group_files | uploader_user_id | CASCADE |
| im_accounts | group_folders | creator_user_id | CASCADE |
| im_accounts | group_photos | uploader_user_id | CASCADE |
| im_accounts | pokes | sender_user_id, target_user_id | CASCADE |
| im_accounts | account_faces | owner_user_id | CASCADE |
| im_accounts | friend_categories | owner_user_id | CASCADE |
| im_accounts | group_categories | owner_user_id | CASCADE |
| bots | protocol_packets | bot_id | SET NULL |
| bots | debug_sessions | bot_id | CASCADE |
| chat_groups | group_members | group_id | CASCADE |
| chat_groups | group_requests | group_id | CASCADE |
| chat_groups | group_announcements | group_id | CASCADE |
| chat_groups | group_folders | group_id | CASCADE |
| chat_groups | group_files | group_id | CASCADE |
| chat_groups | group_essence_messages | group_id | CASCADE |
| chat_groups | group_events | group_id | CASCADE |
| chat_groups | conversations | group_id | CASCADE |
| chat_groups | messages | group_id | SET NULL |
| chat_groups | group_albums | group_id | CASCADE |
| group_categories | user_groups | category_id | SET NULL |
| im_accounts | user_groups | owner_user_id | CASCADE |
| chat_groups | user_groups | group_id | CASCADE |
| friend_categories | friendships | friend_category_id | RESTRICT |
| messages | message_reactions | message_id | CASCADE |
| messages | group_essence_messages | message_id | SET NULL |
| messages | messages | quoted_message_id | SET NULL |
| messages | conversations | last_message_id | SET NULL |
| group_folders | group_files | parent_folder_id | CASCADE |
| group_folders | group_folders | parent_folder_id | CASCADE |
| group_albums | group_photos | album_id | CASCADE |
| debug_sessions | protocol_packets | session_id | SET NULL |
| debug_sessions | messages | session_id | SET NULL |

关键 SET NULL 场景：
- `messages.sender_user_id` SET NULL → 用户删除后消息保留（调试核心资产）
- `group_essence_messages.message_id` SET NULL → 消息删除后精华记录保留
- `friend_categories.friend_category_id` RESTRICT → 有好友时不能删除分组
