# 需求追踪矩阵

功能需求（FR）→ 表 → 字段的可追踪映射。本文档只追踪当前需求规格中的 FR 编号，不使用旧版按好友、群组、会话、系统治理单独拆分的编号体系。

## 1. 功能需求索引

| FR 编号 | 需求领域 | 需求概述 |
|---|---|---|
| FR-ACC | 账号管理 | 创建和管理 simulated/real IM 账号，维护账号资料与自定义表情 |
| FR-BOT | Bot 实例管理 | Bot 注册、账号绑定、运行状态与仪表盘 |
| FR-MSG | 会话与消息 | 消息持久化、会话列表、渲染、撤回、引用、反应和戳一戳 |
| FR-SOC | 好友、群与成员资料镜像 | 好友关系、群资料、群成员、群内容资产与环境隔离 |
| FR-REQ | 请求与事件处理 | 好友申请、群申请/邀请和群事件记录 |
| FR-PKT | 协议包追踪 | 协议报文索引、原始 JSON 文件、业务对象关联与完整性检查 |
| FR-DBG | 调试会话管理 | 调试会话生命周期、消息/报文聚合与历史回看 |
| FR-CFG | 配置管理 | Bot JSON 配置文件读写、配置审计、群组行为配置 |
| FR-AUD | 审计、导出与系统维护 | 操作审计、清理、导出、完整性检查、备份恢复和应用设置 |

## 2. 需求 → 表 → 字段映射

### FR-ACC：账号管理

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-ACC-001 创建与管理 IM 账号 | im_accounts | user_id, nickname, avatar_url, signature, account_source, qid, age, sex, level, bio | 账号身份与资料字段 |
| FR-ACC-002 模拟/真实环境隔离 | im_accounts, chat_groups, group_members | account_source, group_source, (group_id, user_id) | 账号和群分别标记来源，成员写入时校验同源 |
| FR-ACC-003 账号表情管理 | account_faces | face_id, owner_user_id, face_name, emoji_package_id, key, remote_url, local_path | 仅自定义 Marketface 入库；系统表情从 `faces.json` 加载 |

### FR-BOT：Bot 实例管理

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-BOT-001 Bot 注册与账号绑定 | bots | bot_id, bound_user_id, display_name, config_path | Bot 必须绑定一个 IM 账号；配置仅保存文件路径 |
| FR-BOT-001 账号最多绑定一个 Bot | bots | bound_user_id | 唯一索引保证 1:0..1 绑定关系 |
| FR-BOT-002 运行状态管理 | bots, audit_events | runtime_status, event_type, target_type, target_id | 状态枚举为 stopped/running/error；状态变更写审计 |
| FR-BOT-003 Bot 仪表盘 | bots, im_accounts, debug_sessions | display_name, runtime_status, bound_user_id, nickname, started_at | 绑定账号昵称从 `im_accounts` 取，最近启动时间可由最新调试会话推导 |

### FR-MSG：会话与消息

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-MSG-001 消息接收与持久化 | messages | message_id, message_scene, peer_id, message_seq, sender_user_id, receiver_user_id, group_id, content_json, created_at | scene + peer_id + message_seq 业务唯一 |
| FR-MSG-001 关联会话更新 | conversations | conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id, last_message_id, updated_at | 首条消息创建会话，后续消息更新最近消息与时间 |
| FR-MSG-002 会话列表管理 | conversations | last_read_seq, unread_count, is_pinned, is_muted | 会话级状态独立于消息表 |
| FR-MSG-003 消息内容渲染 | messages, account_faces | content_json, face_id, local_path | 消息段 JSON 保存可变内容；自定义表情元数据由 `account_faces` 支撑 |
| FR-MSG-004 引用与撤回 | messages | quoted_message_id, is_recalled, recalled_by_user_id, recalled_at | 撤回标记而非物理删除；引用消息删除后 SET NULL |
| FR-MSG-005 消息与协议包关联 | messages, protocol_packets | bot_id, related_object_type, related_object_id | `messages.bot_id` 反规范化；协议包通过多态字段反查消息 |
| FR-MSG-006 表情回应 | message_reactions | reaction_id, message_id, operator_user_id, face_id, is_add, created_at | 用 `is_add` 记录添加/移除操作历史 |
| FR-MSG-007 戳一戳互动 | pokes | poke_id, sender_user_id, target_user_id, message_scene, peer_id, is_recalled, recalled_at | Poke 与 Message 同处会话语境，但不是消息 |

### FR-SOC：好友、群与成员资料镜像

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-SOC-001 群组资料镜像 | chat_groups | group_id, group_name, avatar_url, group_owner_user_id, member_count, max_member_count, group_source, is_whole_muted, mute_until | 群基本资料、成员数和全员禁言状态；per-user 属性由 user_groups 承载 |
| FR-SOC-002 群成员信息缓存 | group_members | group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until | 基础身份为 P0，扩展资料为 P1 |
| FR-SOC-003 模拟/真实群组环境隔离 | chat_groups, im_accounts, group_members | group_source, account_source, (group_id, user_id) | 成员只能来自同源环境 |
| FR-SOC-004 好友关系缓存 | friendships, friend_categories | owner_user_id, friend_user_id, friend_category_id, remark, is_pinned, category_id, name | owner 视角；好友必须归属一个分组 |
| FR-SOC-005 群分类管理 | group_categories, user_groups | category_id, owner_user_id, name, sort_order | per-user 群分类；通过 user_groups 关联群 |
| FR-SOC-010 群视图管理 | user_groups | owner_user_id, group_id, category_id, is_pinned, is_muted, sort_order, joined_at, last_active_at | 账号视角下的群置顶/免打扰/分类 |
| FR-SOC-006 群公告缓存 | group_announcements | announcement_id, group_id, sender_user_id, content, image_url, created_at | 按群和时间查看公告 |
| FR-SOC-007 群文件与文件夹缓存 | group_folders, group_files | folder_id, parent_folder_id, file_id, file_name, file_size, file_hash, uploader_user_id, expire_at, download_count | 文件夹支持层级，文件保存元数据 |
| FR-SOC-008 群相册与照片缓存 | group_albums, group_photos | album_id, name, cover_url, photo_id, url, description, uploader_user_id, file_size | 低频镜像数据 |
| FR-SOC-009 群精华消息缓存 | group_essence_messages | essence_id, group_id, message_id, sender_user_id, operator_user_id | 同一消息同一群最多一条精华记录 |

### FR-REQ：请求与事件处理

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-REQ-001 好友申请管理 | friend_requests, friendships | request_id, initiator_user_id, target_user_id, comment, state, handled_at | pending 唯一；接受后应用层事务创建双向好友关系 |
| FR-REQ-002 群通知/申请管理 | group_requests, group_members | group_id, notification_seq, notification_type, initiator_user_id, target_user_id, operator_user_id, state | 处理人可为群主或管理员；接受后可插入成员 |
| FR-REQ-003 群事件记录 | group_events | event_id, group_id, event_type, payload_json, created_at | 事件只追加，不修改不删除 |

### FR-PKT：协议包追踪

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-PKT-001 协议报文全量记录 | protocol_packets | packet_id, bot_id, profile_id, protocol_type, direction, action_name, file_path, is_error, session_id, created_at | 数据库存结构化索引和原始 JSON 文件路径，原文在文件系统 |
| FR-PKT-002 协议包与业务对象关联 | protocol_packets | related_object_type, related_object_id | 多态关联 message / group_request / group_event，非 FK |
| FR-PKT-003 原始协议报文查看与导出 | protocol_packets | file_path | 从文件系统懒惰读取或导出原始 JSON；读取失败时提示文件已丢失 |
| FR-PKT-004 多维度筛选 | protocol_packets | bot_id, session_id, protocol_type, direction, action_name, is_error, created_at, related_object_type | 索引覆盖 Bot、错误、关联对象和会话筛选 |
| FR-PKT-005 模拟端协议包记录 | protocol_packets, bots | profile_id, protocol_type, bot_id, config_path | 通过 Bot 配置文件中的连接 profile 区分 mock/simulated 与真实协议端 |
| FR-PKT-006 协议文件懒惰检查 | protocol_packets | packet_id, file_path | 查看或导出原文时读取文件；缺失、不可读或解析失败时提示异常 |

### FR-DBG：调试会话管理

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-DBG-001 自动创建与生命周期 | debug_sessions | session_id, bot_id, session_name, description, started_at, ended_at | Bot 启动创建，停止或异常退出写结束时间 |
| FR-DBG-002 会话内聚合 | messages, protocol_packets | session_id | 消息与协议包都按调试会话分组 |
| FR-DBG-003 调试会话列表与回看 | debug_sessions | bot_id, started_at, ended_at | 按 Bot 和时间范围筛选历史会话 |

### FR-CFG：配置管理

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-CFG-001 Bot 配置文件管理 | bots | config_path | 连接配置和行为配置均在外部 JSON 文件，数据库不拆配置子表 |
| FR-CFG-002 配置变更记录 | audit_events | event_type, actor_user_id, target_type, target_id, detail_json | 配置变更写审计摘要 |
| FR-CFG-003 Bot 群组行为配置 | bots | config_path | 群级、分类级和兜底行为策略都在配置 JSON 中 |

### FR-AUD：审计、导出与系统维护

| FR 子项 | 表 | 字段 | 说明 |
|---|---|---|---|
| FR-AUD-001 操作审计日志 | audit_events | event_id, event_type, actor_user_id, target_type, target_id, detail_json, created_at | 保留期内只追加 |
| FR-AUD-002 协议包与文件定时清理 | app_settings, protocol_packets, audit_events | protocol_packet.retention_days, protocol_packet.cleanup_enabled, packet_id, file_path | 默认协议包/文件 30 天；清理操作写审计 |
| FR-AUD-003 数据导出 | protocol_packets, messages | 协议包元数据、file_path、content_json | 支持协议包列表/原始 JSON 和会话消息历史导出 |
| FR-AUD-004 数据库完整性检查 | protocol_packets | file_path | 结合 SQLite `integrity_check`、FK 检查和按需文件读取错误生成报告 |
| FR-AUD-005 数据库备份与恢复 | app_settings, protocol_packets, bots | schema.version, file_path, config_path | 备份包包含 SQLite 文件、Bot 配置文件和协议包文件路径对应文件 |
| FR-AUD-006 应用设置管理 | app_settings | setting_key, setting_value, value_type, description, updated_at | 存 schema 版本、保留策略和 UI 偏好 |

## 3. 表 → 需求反向映射

| 表 | 覆盖的 FR | 说明 |
|---|---|---|
| im_accounts | FR-ACC, FR-BOT, FR-MSG, FR-SOC | 账号身份、Bot 绑定、消息归属和社交关系基础 |
| account_faces | FR-ACC, FR-MSG | 自定义表情元数据与渲染缓存 |
| friend_categories | FR-SOC | 好友分组 |
| friendships | FR-SOC, FR-REQ | owner 视角好友关系 |
| friend_requests | FR-REQ | 好友申请状态机 |
| group_categories | FR-SOC | per-user 群分类 |
| user_groups | FR-SOC | 账号视角群视图：分类、置顶、免打扰 |
| chat_groups | FR-SOC, FR-MSG | 群资料、群会话和环境隔离（per-user 属性已移入 user_groups） |
| group_members | FR-SOC, FR-REQ | 群成员资料与入群申请处理结果 |
| group_requests | FR-REQ, FR-PKT | 群申请/邀请及协议包多态关联目标 |
| group_announcements | FR-SOC | 群公告缓存 |
| group_folders | FR-SOC | 群文件夹层级 |
| group_files | FR-SOC | 群文件元数据 |
| group_essence_messages | FR-SOC | 群精华消息 |
| group_events | FR-REQ, FR-PKT | 群事件记录及协议包多态关联目标 |
| group_albums | FR-SOC | 群相册 |
| group_photos | FR-SOC | 群照片 |
| conversations | FR-MSG | 会话列表和会话级状态 |
| messages | FR-MSG, FR-PKT, FR-DBG | 核心消息实体、协议追踪对象和调试会话聚合对象 |
| message_reactions | FR-MSG | 表情回应操作历史 |
| pokes | FR-MSG | 戳一戳互动 |
| bots | FR-BOT, FR-CFG, FR-PKT, FR-DBG | Bot 身份、配置文件路径、协议包归属和调试会话归属 |
| debug_sessions | FR-DBG, FR-PKT | 调试会话及协议包分组 |
| protocol_packets | FR-PKT, FR-MSG, FR-AUD | 协议报文索引、消息追溯和导出清理对象 |
| app_settings | FR-AUD | schema 版本、保留策略和全局设置 |
| audit_events | FR-BOT, FR-CFG, FR-AUD | Bot 状态、配置变更和系统操作审计 |

26 张表全部有当前需求来源覆盖，无孤立表。

## 4. 协议事件 → 实体写入映射

| 协议事件 | 写入实体 | 操作 | 覆盖 FR |
|---|---|---|---|
| `message_receive` (private/group/temp) | messages, conversations, protocol_packets | INSERT + UPSERT/UPDATE | FR-MSG, FR-PKT |
| `message_recall` | messages | UPDATE is_recalled/recalled_by_user_id/recalled_at | FR-MSG |
| `message_read` | conversations | UPDATE last_read_seq/unread_count | FR-MSG |
| `friend_request` | friend_requests, protocol_packets | INSERT | FR-REQ, FR-PKT |
| `friend_request_accept` | friend_requests, friendships | UPDATE + INSERT x2 | FR-REQ, FR-SOC |
| `friend_add` | friendships | INSERT x2 | FR-SOC |
| `group_member_join` | group_events, group_members, chat_groups | INSERT + INSERT + trigger/update | FR-SOC, FR-REQ |
| `group_member_leave` | group_events, group_members, chat_groups | INSERT + DELETE + trigger/update | FR-SOC, FR-REQ |
| `group_member_mute` | group_members, group_events | UPDATE + INSERT | FR-SOC, FR-REQ |
| `group_whole_mute` | chat_groups, group_events | UPDATE + INSERT | FR-SOC, FR-REQ |
| `group_request` | group_requests, protocol_packets | INSERT | FR-REQ, FR-PKT |
| `group_request_handle` | group_requests, group_members | UPDATE + optional INSERT | FR-REQ, FR-SOC |
| `group_announcement` | group_announcements | INSERT/UPSERT | FR-SOC |
| `group_file_upload` | group_files | INSERT/UPSERT | FR-SOC |
| `group_essence_set` | group_essence_messages, group_events | INSERT + INSERT | FR-SOC, FR-REQ |
| `group_essence_cancel` | group_essence_messages, group_events | DELETE + INSERT | FR-SOC, FR-REQ |
| `message_reaction` | message_reactions | INSERT operation row | FR-MSG |
| `poke` | pokes | INSERT | FR-MSG |
| `poke_recall` | pokes | UPDATE is_recalled/recalled_by_user_id/recalled_at | FR-MSG |
| `bot_start` / `bot_stop` | bots, debug_sessions, audit_events | UPDATE + INSERT/UPDATE + INSERT | FR-BOT, FR-DBG, FR-AUD |
| `protocol_send` / `protocol_receive` | protocol_packets | INSERT | FR-PKT |

## 5. 关键设计决策追踪

| 设计决策 | 驱动 FR / DR / NFR | 驱动因素 |
|---|---|---|
| SQLite + WAL | NFR-REL, NFR-PERF, NFR-CMP | 单用户本地桌面应用，读写并发和崩溃恢复由 SQLite/WAL 支撑 |
| ID 统一 TEXT 类型 | FR-ACC, FR-MSG, NFR-CMP | 外部协议 ID 可能超出 JS 安全整数范围 |
| Bot 配置外置 `config_path` | FR-BOT-001, FR-CFG-001, FR-CFG-003 | 连接和行为配置属于应用层 JSON，不拆成数据库子表 |
| `messages.bot_id` 反规范化 | FR-MSG-005, FR-PKT-004 | 高频调试查询从多表 JOIN 降为单表索引扫描 |
| `friendships` owner 视角 | FR-SOC-004 | 与协议端好友列表模型一致，支持各自备注和置顶 |
| `conversations` 独立建表 | FR-MSG-002 | 支持空会话、未读数、置顶和免打扰，避免从消息表临时聚合 |
| 消息默认长期保留 | DR-CLN-003, NFR-SEC-006 | 消息是调试核心资产；账号删除默认 SET NULL 保留历史 |
| simulated/real 环境隔离 | FR-ACC-002, FR-SOC-003 | 通过 account_source/group_source 标记，应用层和成员写入校验阻止混用 |
| 协议原文文件化 | FR-PKT-001, FR-PKT-003, NFR-PERF | 大 JSON 不进 SQLite，数据库只存 `file_path` 和结构化索引 |
| 协议文件懒惰检查 | FR-PKT-006, FR-AUD-004, V-DC-001 | 本地桌面场景下不维护文件元数据表；查看/导出时按 `file_path` 读取并处理缺失 |
| 协议包 TTL 清理 | DR-CLN-001, FR-AUD-002 | 协议包/文件默认 30 天清理，消息不随协议包清理 |
| 审计按类型保留 | DR-CLN-004, FR-AUD-001 | 普通审计默认 90 天，安全关键审计默认永久保留 |
| 部分索引 | FR-MSG-002, FR-REQ-001, FR-REQ-002 | 缩小 pending/未读场景索引体积 |
| `user_groups` 账号视角群视图 | FR-SOC-005, FR-SOC-010 | per-user 群分类/置顶/免打扰从 CHAT_GROUP 移入关联表，解除全局群表上的 per-user 属性耦合 |
