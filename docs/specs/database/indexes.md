# 索引设计

全量索引设计，按数据域和查询场景组织。SQLite 自动为 PRIMARY KEY 和 UNIQUE 约束创建索引，本文档仅列出需要显式创建的附加索引。

## 1. 设计原则

| 原则 | 说明 |
|---|---|
| FK 不自动索引 | SQLite 不为 FK 自动创建索引，高频 JOIN 的 FK 列需手动建索引 |
| 部分索引优先 | 查询只关心特定子集时（如 state='pending'），用 WHERE 子句缩小索引体积 |
| 调试查询优先 | 协议追踪、Bot 消息过滤等调试场景的索引优先级最高 |
| 复合索引列顺序 | 等值条件在前，范围/排序条件在后 |

## 2. 身份与社交域（账号与表情）

### im_accounts

PK (`user_id`) 已有隐式索引。无额外索引——账号查询以主键为主，全表扫描可接受（账号数量级 ≤ 千级）。

### account_faces

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_account_faces_owner` | `(owner_user_id)` | 普通 | 列出某账号的所有自定义表情 |

```sql
CREATE INDEX idx_account_faces_owner ON account_faces(owner_user_id);
```

## 3. 身份与社交域（好友与分类）

### friendships

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_friendships_friend` | `(friend_user_id)` | 普通 | 反向查找：某人被哪些人加为好友 |

```sql
CREATE INDEX idx_friendships_friend ON friendships(friend_user_id);
```

说明：`(owner_user_id, friend_user_id)` 联合 PK 已覆盖"owner 的所有好友"查询，仅需补充反向查找。

### friend_requests

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_friend_req_target` | `(target_user_id, state, created_at) WHERE state = 'pending'` | 部分索引 | 某人收到的待处理好友申请 |

```sql
CREATE INDEX idx_friend_req_target ON friend_requests(target_user_id, state, created_at) WHERE state = 'pending';
```

说明：pending 状态的申请远少于总数，部分索引大幅缩小体积。`handled_at` 为 NULL 的已处理申请查询走全表扫描（低频操作）。

### friend_categories

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_friend_categories_owner` | `(owner_user_id)` | 普通 | 列出某账号的所有好友分组 |

```sql
CREATE INDEX idx_friend_categories_owner ON friend_categories(owner_user_id);
```

### group_categories

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_group_categories_owner` | `(owner_user_id)` | 普通 | 列出某账号的所有群分类 |
| `idx_group_categories_name` | `(owner_user_id, name)` | 唯一 | 同账号下分类不重名 |

```sql
CREATE INDEX idx_group_categories_owner ON group_categories(owner_user_id);
CREATE UNIQUE INDEX idx_group_categories_name ON group_categories(owner_user_id, name);
```

### user_groups

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_user_groups_category` | `(owner_user_id, category_id)` | 普通 | 按分类筛选该账号下的群 |
| `idx_user_groups_group` | `(group_id)` | 普通 | 查找某群被哪些账号订阅 |

```sql
CREATE INDEX idx_user_groups_category ON user_groups(owner_user_id, category_id);
CREATE INDEX idx_user_groups_group ON user_groups(group_id);
```

说明：`(owner_user_id, group_id)` 联合 PK 已有隐式索引，覆盖"某账号的所有群视图"查询。

## 4. 群组与内容域

### chat_groups

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_chat_groups_group_owner` | `(group_owner_user_id)` | 普通 | 某人拥有的群列表 |

```sql
CREATE INDEX idx_chat_groups_group_owner ON chat_groups(group_owner_user_id);
```

### group_members

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_group_members_user` | `(user_id)` | 普通 | 查找某人加入了哪些群 |
| `idx_group_members_role` | `(group_id, role)` | 普通 | 群管理查询：列出某群所有管理员 |

```sql
CREATE INDEX idx_group_members_user ON group_members(user_id);
CREATE INDEX idx_group_members_role ON group_members(group_id, role);
```

说明：`(group_id, user_id)` 联合 PK 已覆盖"群的所有成员"查询。

### group_requests

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_group_req_initiator` | `(initiator_user_id)` | 普通 | 某人发起的所有群申请 |
| `idx_group_req_pending` | `(group_id, state, created_at) WHERE state = 'pending'` | 部分索引 | 某群待处理的申请列表 |

```sql
CREATE INDEX idx_group_req_initiator ON group_requests(initiator_user_id);
CREATE INDEX idx_group_req_pending ON group_requests(group_id, state, created_at) WHERE state = 'pending';
```

### group_announcements

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_announcements_group` | `(group_id, created_at DESC)` | 普通 | 某群的公告列表（最新在前） |

```sql
CREATE INDEX idx_announcements_group ON group_announcements(group_id, created_at DESC);
```

### group_folders

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_folders_group_parent` | `(group_id, parent_folder_id)` | 普通 | 某群某文件夹下的子文件夹列表 |

```sql
CREATE INDEX idx_folders_group_parent ON group_folders(group_id, parent_folder_id);
```

### group_files

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_group_files_group_time` | `(group_id, created_at DESC)` | 普通 | 某群的文件列表（最新在前） |

```sql
CREATE INDEX idx_group_files_group_time ON group_files(group_id, created_at DESC);
```

### group_essence_messages

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_essence_group` | `(group_id, created_at DESC)` | 普通 | 某群的精华消息列表（最新在前） |

```sql
CREATE INDEX idx_essence_group ON group_essence_messages(group_id, created_at DESC);
```

说明：UNIQUE(group_id, message_id) 已有隐式索引。

### group_events

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_group_events_group_type` | `(group_id, event_type, created_at DESC)` | 普通 | 某群某类事件的历史记录 |

```sql
CREATE INDEX idx_group_events_group_type ON group_events(group_id, event_type, created_at DESC);
```

### group_albums

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_albums_group` | `(group_id)` | 普通 | 某群的相册列表 |

```sql
CREATE INDEX idx_albums_group ON group_albums(group_id);
```

### group_photos

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_photos_album` | `(album_id, created_at DESC)` | 普通 | 某相册的照片列表（最新在前） |

```sql
CREATE INDEX idx_photos_album ON group_photos(album_id, created_at DESC);
```

## 5. 会话与消息域

### conversations

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_conv_owner_updated` | `(owner_user_id, updated_at DESC)` | 普通 | 会话列表（按最近活跃排序）——最高频查询 |
| `idx_conv_unread` | `(owner_user_id, unread_count) WHERE unread_count > 0` | 部分索引 | 未读会话统计/角标 |
| `uq_conversation_private` | `(owner_user_id, conversation_scene, peer_user_id) WHERE conversation_scene IN ('private','temp')` | 部分唯一 | 私聊/temp 会话唯一性 |
| `uq_conversation_group` | `(owner_user_id, conversation_scene, group_id) WHERE conversation_scene = 'group'` | 部分唯一 | 群会话唯一性 |

```sql
CREATE INDEX idx_conv_owner_updated ON conversations(owner_user_id, updated_at DESC);
CREATE INDEX idx_conv_unread ON conversations(owner_user_id, unread_count) WHERE unread_count > 0;
CREATE UNIQUE INDEX uq_conversation_private ON conversations(owner_user_id, conversation_scene, peer_user_id) WHERE conversation_scene IN ('private', 'temp');
CREATE UNIQUE INDEX uq_conversation_group ON conversations(owner_user_id, conversation_scene, group_id) WHERE conversation_scene = 'group';
```

说明：未读索引用部分索引——绝大多数会话 unread_count=0，索引体积约为全量索引的 10-20%。唯一约束使用部分索引而非普通 UNIQUE——因为 peer_user_id/group_id 互斥为 NULL，普通 UNIQUE 对 nullable 列不保证唯一性。

### messages

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_msg_scene_peer_time` | `(message_scene, peer_id, created_at DESC)` | 普通 | 消息列表（按会话+时间排序）——最高频查询 |
| `idx_msg_sender_time` | `(sender_user_id, created_at)` | 普通 | 按发送者追溯消息 |
| `idx_msg_bot_time` | `(bot_id, created_at DESC)` | 普通 | 按 Bot 过滤消息（调试核心查询） |
| `idx_msg_quoted` | `(quoted_message_id)` | 普通 | 查找某消息的引用者 |

```sql
CREATE INDEX idx_msg_scene_peer_time ON messages(message_scene, peer_id, created_at DESC);
CREATE INDEX idx_msg_sender_time ON messages(sender_user_id, created_at);
CREATE INDEX idx_msg_bot_time ON messages(bot_id, created_at DESC);
CREATE INDEX idx_msg_quoted ON messages(quoted_message_id);
```

说明：UNIQUE(message_scene, peer_id, message_seq) 已有隐式索引。bot_id 索引是调试场景的核心优化——`WHERE bot_id='X'` 从 3 表 JOIN 降为单表索引扫描。

### message_reactions

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_reactions_msg` | `(message_id)` | 普通 | 某消息的所有表情回应 |

```sql
CREATE INDEX idx_reactions_msg ON message_reactions(message_id);
```

说明：表情回应保留添加/移除操作历史，不建立 `(message_id, operator_user_id, face_id)` 唯一索引；当前重复添加由应用层按净状态拦截。

### pokes

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_pokes_scene_peer` | `(message_scene, peer_id, created_at DESC)` | 普通 | 某会话的戳一戳历史 |

```sql
CREATE INDEX idx_pokes_scene_peer ON pokes(message_scene, peer_id, created_at DESC);
```

## 6. Bot 与协议调试域

### bots

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_bots_bound_user` | `(bound_user_id)` | 唯一 | 通过绑定的 IM 账号查找 Bot，并保证一个账号最多绑定一个 Bot |

```sql
CREATE UNIQUE INDEX idx_bots_bound_user ON bots(bound_user_id);
```

### debug_sessions

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_debug_sessions_bot` | `(bot_id, started_at DESC)` | 普通 | 某 Bot 的调试会话列表（最近在前） |

```sql
CREATE INDEX idx_debug_sessions_bot ON debug_sessions(bot_id, started_at DESC);
```

### protocol_packets

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_packet_bot_time` | `(bot_id, created_at DESC)` | 普通 | 某 Bot 的协议报文审计 |
| `idx_packet_error_time` | `(is_error, created_at DESC)` | 普通 | 协议错误筛选——调试高频 |
| `idx_packet_related` | `(related_object_type, related_object_id)` | 普通 | 业务对象溯源：从消息/群申请反查报文 |
| `idx_packet_session` | `(session_id)` | 普通 | 按调试会话分组查看报文 |

```sql
CREATE INDEX idx_packet_bot_time ON protocol_packets(bot_id, created_at DESC);
CREATE INDEX idx_packet_error_time ON protocol_packets(is_error, created_at DESC);
CREATE INDEX idx_packet_related ON protocol_packets(related_object_type, related_object_id);
CREATE INDEX idx_packet_session ON protocol_packets(session_id);
```

## 7. 系统治理域

### audit_events

| 索引 | 列 | 类型 | 场景 |
|---|---|---|---|
| `idx_audit_type_time` | `(event_type, created_at DESC)` | 普通 | 按事件类型筛选审计日志 |
| `idx_audit_actor` | `(actor_user_id, created_at DESC)` | 普通 | 某操作者的审计历史 |

```sql
CREATE INDEX idx_audit_type_time ON audit_events(event_type, created_at DESC);
CREATE INDEX idx_audit_actor ON audit_events(actor_user_id, created_at DESC);
```

### app_settings

PK (`setting_key`) 已有隐式索引。无额外索引——设置键数量少，全表扫描可接受。

## 8. 索引汇总

| 序号 | 表 | 索引名 | 列 | 类型 |
|---|---|---|---|---|
| 1 | account_faces | `idx_account_faces_owner` | owner_user_id | 普通 |
| 2 | bots | `idx_bots_bound_user` | bound_user_id | 唯一 |
| 3 | friendships | `idx_friendships_friend` | friend_user_id | 普通 |
| 4 | friend_requests | `idx_friend_req_target` | (target_user_id, state, created_at) WHERE state='pending' | 部分 |
| 5 | friend_categories | `idx_friend_categories_owner` | owner_user_id | 普通 |
| 6 | group_categories | `idx_group_categories_owner` | owner_user_id | 普通 |
| 7 | group_categories | `idx_group_categories_name` | (owner_user_id, name) | 唯一 |
| 8 | user_groups | `idx_user_groups_category` | (owner_user_id, category_id) | 普通 |
| 9 | user_groups | `idx_user_groups_group` | group_id | 普通 |
| 10 | chat_groups | `idx_chat_groups_group_owner` | group_owner_user_id | 普通 |
| 11 | group_members | `idx_group_members_user` | user_id | 普通 |
| 12 | group_members | `idx_group_members_role` | (group_id, role) | 普通 |
| 13 | group_requests | `idx_group_req_initiator` | initiator_user_id | 普通 |
| 14 | group_requests | `idx_group_req_pending` | (group_id, state, created_at) WHERE state='pending' | 部分 |
| 15 | group_announcements | `idx_announcements_group` | (group_id, created_at DESC) | 普通 |
| 16 | group_folders | `idx_folders_group_parent` | (group_id, parent_folder_id) | 普通 |
| 17 | group_files | `idx_group_files_group_time` | (group_id, created_at DESC) | 普通 |
| 18 | group_essence_messages | `idx_essence_group` | (group_id, created_at DESC) | 普通 |
| 19 | group_events | `idx_group_events_group_type` | (group_id, event_type, created_at DESC) | 普通 |
| 20 | group_albums | `idx_albums_group` | group_id | 普通 |
| 21 | group_photos | `idx_photos_album` | (album_id, created_at DESC) | 普通 |
| 22 | conversations | `idx_conv_owner_updated` | (owner_user_id, updated_at DESC) | 普通 |
| 23 | conversations | `idx_conv_unread` | (owner_user_id, unread_count) WHERE unread_count > 0 | 部分 |
| 24 | conversations | `uq_conversation_private` | (owner_user_id, conversation_scene, peer_user_id) WHERE conversation_scene IN ('private','temp') | 部分唯一 |
| 25 | conversations | `uq_conversation_group` | (owner_user_id, conversation_scene, group_id) WHERE conversation_scene = 'group' | 部分唯一 |
| 26 | messages | `idx_msg_scene_peer_time` | (message_scene, peer_id, created_at DESC) | 普通 |
| 27 | messages | `idx_msg_sender_time` | (sender_user_id, created_at) | 普通 |
| 28 | messages | `idx_msg_bot_time` | (bot_id, created_at DESC) | 普通 |
| 29 | messages | `idx_msg_quoted` | quoted_message_id | 普通 |
| 30 | message_reactions | `idx_reactions_msg` | message_id | 普通 |
| 31 | pokes | `idx_pokes_scene_peer` | (message_scene, peer_id, created_at DESC) | 普通 |
| 32 | debug_sessions | `idx_debug_sessions_bot` | (bot_id, started_at DESC) | 普通 |
| 33 | protocol_packets | `idx_packet_bot_time` | (bot_id, created_at DESC) | 普通 |
| 34 | protocol_packets | `idx_packet_error_time` | (is_error, created_at DESC) | 普通 |
| 35 | protocol_packets | `idx_packet_related` | (related_object_type, related_object_id) | 普通 |
| 36 | protocol_packets | `idx_packet_session` | session_id | 普通 |
| 37 | audit_events | `idx_audit_type_time` | (event_type, created_at DESC) | 普通 |
| 38 | audit_events | `idx_audit_actor` | (actor_user_id, created_at DESC) | 普通 |

共 38 个显式索引，其中 5 个部分索引（含 2 个部分唯一索引）。

## 9. 查询场景验证

### 高频查询（必须命中索引）

| 查询 | 使用索引 | 索引类型 |
|---|---|---|
| 会话列表（按更新时间排序） | `idx_conv_owner_updated` | 复合，覆盖排序 |
| 某会话的消息列表（分页） | `idx_msg_scene_peer_time` | 复合，覆盖排序 |
| 未读会话统计 | `idx_conv_unread` | 部分索引 |
| Bot X 的所有消息 | `idx_msg_bot_time` | 复合，覆盖排序 |
| 协议错误列表 | `idx_packet_error_time` | 复合，覆盖排序 |

### 中频查询

| 查询 | 使用索引 | 索引类型 |
|---|---|---|
| 某用户的待处理好友申请 | `idx_friend_req_target` | 部分索引 |
| 某群的待处理申请 | `idx_group_req_pending` | 部分索引 |
| 某群的管理员列表 | `idx_group_members_role` | 复合 |
| 从消息反查协议报文 | `idx_packet_related` | 复合 |
| 按事件类型筛选审计日志 | `idx_audit_type_time` | 复合 |

### 低频查询（索引覆盖但非关键）

| 查询 | 使用索引 |
|---|---|
| 某好友被哪些人添加 | `idx_friendships_friend` |
| 某人发起的所有群申请 | `idx_group_req_initiator` |
| 某消息的引用者列表 | `idx_msg_quoted` |

## 10. 部分索引说明

5 个部分索引的 WHERE 子句筛选了远小于全表的数据子集：

| 部分索引 | 筛选条件 | 典型占比 | 收益 |
|---|---|---|---|
| `idx_conv_unread` | `unread_count > 0` | 10-20% | 未读查询只需扫描活跃会话 |
| `idx_friend_req_target` | `state = 'pending'` | 5-15% | 已完成申请不出现在索引中 |
| `idx_group_req_pending` | `state = 'pending'` | 5-15% | 同上 |
| `uq_conversation_private` | `conversation_scene IN ('private','temp')` | 66% | 唯一约束只对私聊/temp 会话生效 |
| `uq_conversation_group` | `conversation_scene = 'group'` | 33% | 唯一约束只对群会话生效 |

部分索引的写入开销仅在新行匹配 WHERE 条件时产生，对非匹配行的 INSERT/UPDATE 无额外开销。
