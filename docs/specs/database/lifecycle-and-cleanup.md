# 数据生命周期与清理策略

数据保留、过期清理、文件一致性维护。涵盖协议报文 TTL、审计日志轮转、孤儿文件检测、数据库 VACUUM。

## 1. 数据分类与保留策略

按变更频率和保留需求，26 张表分为三类：

### 1.1 持久数据（不自动清理）

| 表 | 保留策略 | 说明 |
|---|---|---|
| im_accounts | 永久 | 账号身份，手动管理 |
| bots | 永久 | Bot 实例，手动管理 |
| account_faces | 永久 | 自定义表情，手动管理 |
| friendships | 永久 | 好友关系，手动管理 |
| friend_requests | 永久 | 申请历史，手动管理 |
| friend_categories | 永久 | 分组，手动管理 |
| group_categories | 永久 | 群分类，手动管理 |
| user_groups | 永久 | 账号-群视图，手动管理 |
| chat_groups | 永久 | 群组，手动管理 |
| group_members | 永久 | 成员关系，手动管理 |
| group_announcements | 永久 | 公告历史，手动管理 |
| group_folders | 永久 | 文件夹结构，手动管理 |
| group_files | 永久 | 文件元数据，手动管理 |
| group_essence_messages | 永久 | 精华记录，手动管理 |
| group_albums | 永久 | 相册，手动管理 |
| group_photos | 永久 | 照片元数据，手动管理 |
| conversations | 永久 | 会话，手动管理 |
| app_settings | 永久 | 配置项，手动管理 |

### 1.2 长期保留（消息类，不自动清理）

| 表 | 保留策略 | 说明 |
|---|---|---|
| messages | 长期保留，默认不自动删除 | `sender_user_id` ON DELETE SET NULL，消息体保留 |
| message_reactions | 长期保留 | 随消息 CASCADE 删除 |
| pokes | 长期保留 | 手动删除 |
| group_events | 长期保留 | 只追加不删除 |
| group_requests | 长期保留 | 申请历史不自动清理 |

**设计理由**：消息是调试核心资产——Bot 开发者需要回溯历史消息来排查问题。消息不随协议报文 TTL 自动删除，确保"协议报文可能已清理，但消息记录仍在"。

### 1.3 可过期数据（支持 TTL 清理）

| 表 | 保留策略 | 默认 TTL | 说明 |
|---|---|---|---|
| protocol_packets | TTL 清理 | 30 天 | 协议报文结构化索引 |
| audit_events | 按类型 TTL 清理 | 普通 90 天；安全关键永久 | 操作审计日志 |
| debug_sessions | 手动清理 | — | 调试会话分组，完成后可手动删除 |

## 2. 协议报文 TTL 清理

### 2.1 配置项

通过 `app_settings` 控制清理行为：

| setting_key | 默认值 | 说明 |
|---|---|---|
| `protocol_packet.retention_days` | `30` | 协议包和协议包文件保留天数，0 = 永不过期 |
| `protocol_packet.cleanup_enabled` | `true` | 是否启用协议包定时清理 |
| `audit.retention_days` | `90` | 普通操作审计保留天数 |
| `audit.security_retention_days` | `0` | 安全关键审计保留天数，0 = 永不过期 |

设置 `protocol_packet.retention_days = 0` 或 `protocol_packet.cleanup_enabled = false` 时关闭协议包自动清理（调试工具的常见需求）。

### 2.2 清理流程

报文清理涉及数据库行和磁盘文件，需在事务内协调：

```text
BEGIN TRANSACTION;

-- 1. 查询待清理报文的文件路径
SELECT packet_id, file_path
FROM protocol_packets
WHERE created_at < :cutoff_ms;

-- 2. 删除数据库行
DELETE FROM protocol_packets
WHERE created_at < :cutoff_ms;

COMMIT;

-- 3. 事务提交后删除磁盘文件（文件删除失败不影响数据库一致性）
FOR EACH (file_path FROM step 1):
    如果文件存在 → 删除
    如果文件不存在 → 忽略（已被其他方式清理）
    如果删除失败 → 记录清理待处理报告，后台任务重试
```

步骤顺序的考量：
- 步骤 1 必须在 DELETE 之前执行——删除报文索引后将无法再从数据库获取文件路径
- 步骤 3 在事务提交后执行——如果文件删除失败，数据库已提交，生成清理待处理报告；后台任务或孤儿扫描后续处理

### 2.3 清理触发时机

| 触发方式 | 时机 | 说明 |
|---|---|---|
| 应用启动 | 启动后 5 分钟 | 延迟执行，避免影响启动速度 |
| 定时任务 | 每 6 小时 | 长期运行时的定期清理 |
| 手动触发 | 用户操作 | 设置页面提供"立即清理"按钮 |

清理逻辑包含防重入保护——同一时刻只允许一个清理任务运行。

### 2.4 审计事件清理

审计事件的清理不涉及文件系统，但必须按事件类型区分普通审计与安全关键审计。安全关键事件默认永久保留，普通事件默认 90 天：

```sql
DELETE FROM audit_events
WHERE created_at < :cutoff_ms
  AND event_type NOT IN (:security_critical_event_types);
```

`:security_critical_event_types` 由应用层维护，例如账号彻底清除、数据库恢复、敏感导出等安全关键操作。

## 3. 文件一致性

### 3.1 问题模型

数据库和文件系统之间缺乏原子性保证，存在两类一致性问题：

```text
问题一：孤儿文件（Orphan Files）
  协议报文过期 → DB 行已删，但磁盘 JSON 文件仍在
  原因：清理流程步骤 3 失败（磁盘满、权限变更、进程崩溃）

问题二：悬空引用（Dangling References）
  磁盘 JSON 文件被外部删除/损坏 → DB 行仍指向该路径
  原因：用户手动清理磁盘、杀毒软件隔离、磁盘故障
```

### 3.2 解决方案：file_path + 懒惰检查

`PROTOCOL_PACKET.file_path` 是每个报文原始 JSON 的唯一数据库指针。UniBot 是本地桌面调试工具，不维护额外文件元数据或全量校验状态：

- **孤儿文件**：正常的清理流程（2.2）负责删除。如果清理流程失败留下孤儿，后续目录清理或用户手动清理即可处理；孤儿文件不污染数据库查询结果。
- **悬空引用**：不做定时全量扫描。用户查看或导出原始报文时，业务代码直接读取 `file_path`；若文件不存在、不可读或 JSON 解析失败，则 UI 提示“文件已丢失或过期”。

### 3.3 文件写入协议（最终一致性）

```text
写入报文文件的标准流程：

1. 写临时文件
   path = data/packets/YYYY-MM-DD/{packet_id}.json.tmp

2. 原子重命名
   path → data/packets/YYYY-MM-DD/{packet_id}.json
   (同文件系统内的 rename 是原子操作)

3. 事务内写数据库
   BEGIN;
   INSERT INTO protocol_packets (..., file_path, ...)
        VALUES (..., :path, ...);
   COMMIT;

4. 异常处理
   如果步骤 3 回滚 → 删除步骤 2 的 JSON 文件
   如果步骤 3 提交后文件被外部删除 → 下次查看原文时提示文件已丢失
```

关键设计：文件先落盘（步骤 2），数据库行后写入（步骤 3）。如果 DB 写入失败，留下一个孤儿文件——这是比 DB 行指向不存在的文件更安全的选择（孤儿文件不污染查询结果，仅是磁盘占用）。

### 3.4 懒惰读取检查

协议包详情页和导出流程统一使用懒惰读取检查：

```text
1. 从 protocol_packets 查询 file_path
2. 尝试读取文件
   → 成功：展示或导出原始 JSON
   → NotFound / PermissionDenied / JSON 解析失败：展示“文件已丢失或过期”，不修改协议包索引
```

该策略避免后台任务扫描大量本地文件，也避免在每次写入时计算额外文件元数据。

## 4. 手动删除级联

用户手动删除实体时的级联行为（详见 [table-dictionary.md](table-dictionary.md) FK 级联汇总）：

### 4.1 完全级联（CASCADE）

删除父实体时同步清除所有子数据：

- 删除 `IM_ACCOUNT` → 清除 Bot、好友关系、好友申请、群成员关系、会话、自定义表情、分组、USER_GROUP
- 删除 `CHAT_GROUP` → 清除成员、申请、公告、文件夹、文件、精华、事件、相册、照片、USER_GROUP
- 删除 `BOT` → 清除调试会话
- 删除 `GROUP_ALBUM` → 清除所有照片
- 删除 `GROUP_FOLDER` → 清除子文件夹和文件

### 4.2 保留标记（SET NULL）

删除父实体时保留子数据，标记关联已失效：

- 删除 `IM_ACCOUNT` → 保留消息（sender_user_id/receiver_user_id 置 NULL）
- 删除 `BOT` → 保留协议报文（bot_id 置 NULL）
- 删除 `MESSAGE` → 保留精华记录（message_id 置 NULL）
- 删除 `MESSAGE` → 保留引用关系（quoted_message_id 置 NULL）

### 4.3 阻止删除（RESTRICT）

- 删除 `FRIEND_CATEGORY` → 如果仍有好友在此分组中，阻止删除

## 5. 存储空间管理

### 5.1 SQLite VACUUM

协议报文清理后，SQLite 文件不会自动收缩——DELETE 只是标记页面可复用。建议：

- 每次清理任务完成后执行 `PRAGMA auto_vacuum = INCREMENTAL;`
- 数据库空闲时（用户无操作 30 秒）执行 `PRAGMA incremental_vacuum;`
- 应用退出时执行完整 `VACUUM;`（仅当空闲空间 > 50MB 时，避免退出延迟）

### 5.2 报文目录按日期组织

```text
data/packets/
├── 2026-05-01/
├── 2026-05-02/
├── ...
└── 2026-05-15/
```

按日期分目录的好处：TTL 清理可以直接删除整个过期目录（如 30 天前的目录），比逐个删除文件快几个数量级。如果当天仍有保留的报文（可能因为 `protocol_packet.retention_days` 不是严格的按天计算），则回退到逐文件删除。

### 5.3 存储估算

| 数据 | 单条大小 | 日增量（典型） | 保留期总量（典型） | 说明 |
|---|---|---|---|---|
| protocol_packets 索引行 | ~240 B | 500-2000 条 | 4-15 MB | 结构化字段 + file_path |
| 报文 JSON 文件 | 2-20 KB/个 | 500-2000 个 | 30-1200 MB | 取决于报文大小和频率 |
| messages 行 | ~500 B | 100-1000 条 | 1.5-15 MB | 消息（不受 TTL 影响） |
| audit_events 行 | ~200 B | 10-100 条 | 0.18-1.8 MB（普通 90 天） | 审计日志；安全关键事件永久保留但数量很少 |

典型协议包 30 天保留时，总存储：SQLite 数据库 ~20-50 MB，报文文件 ~100-500 MB。清理主要回收文件系统空间。
