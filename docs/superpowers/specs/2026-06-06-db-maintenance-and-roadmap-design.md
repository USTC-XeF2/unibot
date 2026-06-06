# UniBot 现状盘点与数据库维护策略设计

> 本文档总结当前代码与设计的差距，并确定数据库长期维护的务实实施路线。

---

## 1. 当前已实现 vs 未实现功能

### 1.1 已实现（较完整）

| 模块 | 功能 |
|------|------|
| **用户系统** | 注册、列表、删除（软删标记）、更新资料、头像 URL |
| **好友系统** | 发送好友请求、处理请求（接受/拒绝）、删除好友、列表好友 |
| **消息系统** | 私聊/群聊发送消息、消息历史、消息撤回、引用消息、表情/文字/@/@全体 |
| **互动系统** | 消息 Reaction（表情反应）、戳一戳（Poke） |
| **群组系统** | 创建群、加群请求、处理请求、踢人、禁言成员、全员禁言、设置角色/头衔、退群、解散群 |
| **群内容** | 公告、文件夹、文件（仅元数据表）、精华消息 |
| **群事件** | 群事件历史记录 |
| **虚拟客户端** | 多用户聊天窗口、会话列表、消息展示、侧边栏导航 |
| **数据持久化** | SQLite + sqlx + 自定义迁移器（0001 初始 schema）、WAL 模式、FK 约束 |

### 1.2 未实现 / 仅占位

| 模块 | 现状 | 说明 |
|------|------|------|
| **OneBot v11/v12 协议** | ❌ 完全缺失 | README 核心卖点，代码里零实现 |
| **Milky 协议** | ❌ 完全缺失 | 同上 |
| **协议报文审计** | 🔶 只有表 | `protocol_packets` 表存在，但没有任何代码读写 |
| **日志系统** | 🔶 UI 空壳 | Logs 页面有完整筛选 UI，但数据源是空数组 `[]` |
| **文件上传/下载** | 🔶 只有元数据 | `group_files` 表存在，但没有实际上传/存储/下载逻辑 |
| **本地图片存储** | 🔶 只有 URL | `MessageSegment::Image` 只支持 URL，没有本地文件引用 |
| **头像上传** | 🔶 只有 URL | 创建用户时头像填的是外部 URL（如 QQ 头像接口） |
| **设置系统** | 🔶 空白页面 | `SettingsView` 是空的 `<div></div>` |
| **Dashboard 统计** | 🔶 两个 null | "总消息数"和"在线机器人数"没有接入数据 |
| **JSON 双轨存储** | ❌ 未实现 | README 提到 SQLite + JSON 双轨，代码里只有 SQLite |
| **语音/视频消息** | ❌ 未实现 | `MessageSegment` 没有这些类型 |
| **机器人/Webhook** | ❌ 未实现 | 没有 HTTP/WebSocket 监听端口，没有机器人回调机制 |
| **数据备份/导出** | ❌ 未实现 | 没有任何导入导出或备份机制 |
| **Bot 管理** | ❌ 未实现 | `bots` 表已创建但无业务代码使用 |
| **调试会话** | ❌ 未实现 | `debug_sessions` 表已创建但无业务代码使用 |
| **审计日志** | ❌ 未实现 | `audit_events` 表已创建但无业务代码使用 |
| **会话表** | ❌ 未实现 | `conversations` 表已创建但无业务代码使用 |

### 1.3 已知数据完整性缺陷

| 缺陷 | 位置 | 说明 |
|------|------|------|
| **CTE MAX+1 ID 生成竞态** | 7 个 repo 的 INSERT 语句 | `WITH next_id AS (SELECT MAX(...) + 1)` 在 WAL + 多连接下会导致两个事务生成相同 ID，触发 `UNIQUE` 约束冲突 |

涉及文件：

- `src-tauri/src/persistence/repo/message.rs` — `insert_message`
- `src-tauri/src/persistence/repo/interaction.rs` — `insert_message_reaction`、`insert_poke`
- `src-tauri/src/persistence/repo/user/friends.rs` — `create_friend_request`
- `src-tauri/src/persistence/repo/group/requests.rs` — `create_group_request`
- `src-tauri/src/persistence/repo/group/content.rs` — `create_group_essence_message`
- `src-tauri/src/persistence/repo/group/events.rs` — `insert_group_event`

> 修复方案：Rust 端生成 UUID v7（时间有序），彻底消除 DB 层竞态。详见 [docs/superpowers/plans/2026-05-22-id-generation-atomicity.md](../plans/2026-05-22-id-generation-atomicity.md)。

---

## 2. 核心判断：文档领先于实现

项目的数据库设计文档（`docs/specs/database/`）非常详尽，覆盖了：
- 26 张表的完整 DDL
- 38 个索引设计（含部分索引）
- 完整的迁移路线规划（0001-011）
- 数据生命周期与清理策略（TTL、VACUUM、孤儿文件处理）
- 备份恢复、完整性检查、文件一致性协议

**但代码只实现了 0001 migration 和核心 IM 功能。**

这意味着：如果现在按文档把所有维护机制（TTL 清理、VACUUM 自动化、审计日志、复杂监控面板）都做出来，会陷入**"维护一个空数据库"**的尴尬——没有真实协议报文可清理，没有审计事件可保留，没有 Bot 调试数据可追踪。

---

## 3. 数据库维护策略：务实派

### 3.1 已够用的维护基线（不需要再投工作量）

| 项目 | 状态 | 说明 |
|------|------|------|
| Migration 机制 | ✅ 已够用 | 自定义 migrator + SQL 分词器 + 事务保证 |
| WAL 模式 | ✅ 已够用 | `journal_mode = WAL`, `synchronous = NORMAL` |
| FK 约束 | ✅ 已够用 | `foreign_keys = ON`, `busy_timeout = 5000ms` |
| Schema 版本管理 | ✅ 已够用 | `app_settings.schema.version` |
| Repo 测试 | ✅ 已够用 | `#[sqlx::test]` + smoke tests |

### 3.2 必须补齐的维护安全网

**唯一真正的风险：schema 演进时老用户数据安全。**

当前只有 0001，未来加新表/改字段时，需要验证 migrator 能平滑升级。因此必须做：

1. **Migration 兼容性测试**
   - 构造一个"纯 0001 的数据库"，运行 migrator，验证能正常升级到最新
   - 以后每加一个新 migration，都必须通过这个测试范式
   - 这是后续 0002+ 迁移的**安全网**

2. **Settings 页面"数据库状态"卡片**
   - 当前 `SettingsView` 完全空白
   - 最小展示：schema 版本、表数量、数据库文件大小、`integrity_check` 按钮
   - 最小操作："导出数据库备份"按钮（复制 `.db` + `.db-wal` + `.db-shm`）
   - **不做**：SQL 执行器、VACUUM 按钮、清理按钮

> 以上两项工作量小，但直接服务于 schema 演进和用户数据保护。

### 3.3 明确推迟的维护项

| 维护项 | 推迟理由 |
|--------|---------|
| Protocol packets TTL 清理 | `protocol_packets` 还没有真实写入 |
| VACUUM 自动化 | 数据库文件目前很小，开发期不需要 |
| 审计日志系统 | `audit_events` 还没有真实写入 |
| 复杂监控面板 | 没有复杂数据需要监控 |
| 完整性检查自动化 | 手动按钮足够，自动化是 P2 |

---

## 4. 功能推进策略：Bot 管理优先于假协议数据

### 4.1 不做"虚拟协议报文"

之前考虑过在虚拟 IM 消息发送时伪造 protocol_packets，但此方案被否决：
- 假报文对 Bot 开发者没有调试价值
- 制造了真实的文件系统维护负担（目录管理、磁盘增长、一致性）
- 真协议适配进来时，格式和逻辑大概率要重写

### 4.2 推荐的最小垂直切片

**目标**：让 `bots`、`debug_sessions` 产生真实业务数据，让 Dashboard 统计活起来。

具体工作：

1. **Bot 实体 CRUD**
   - 允许把已有虚拟账号"标记为 Bot"
   - 写入 `bots` 表（表已存在，只是没代码用）
   - `config_path` 先指向一个空 JSON 文件占位

2. **调试会话最小实现**
   - "启动 Bot" → 创建 `debug_session`（记录 `started_at`）
   - "停止 Bot" → 更新 `ended_at`
   - Dashboard"在线机器人数" = 有活跃 session 的 Bot 数

3. **消息归属到 Bot**
   - Bot 账号发送/接收消息时，填充 `messages.bot_id`
   - 支持按 Bot 过滤消息历史

> 这个切片的好处：所有数据都是真实业务数据，不是伪造的。当未来接入真协议时，Bot 管理和调试会话的表结构、查询逻辑完全复用。

---

## 5. 阶段化实施路线

### 阶段 1（立即）：维护安全网 + Bot 管理

**数据完整性（必须先修）：**

- [ ] UUID v7 替换 CTE MAX+1 ID 生成（消除并发竞态）
  - 加 `uuid` crate（feature `v7`）
  - 改 7 个 repo 的 INSERT：去掉 `WITH next_id` CTE，Rust 侧生成 ID 后绑定

**维护侧：**
- [ ] Migration 兼容性测试（0001 → latest）
- [ ] Settings 页面数据库状态卡片（schema 版本、表数、文件大小、完整性检查按钮、备份按钮）

**功能侧：**
- [ ] Bot 管理 CRUD（标记虚拟账号为 Bot、解绑）
- [ ] 调试会话生命周期（启动/停止记录）
- [ ] Dashboard 统计接入（用户数、群数、消息数、在线 Bot 数）

### 阶段 2（有真实 Bot 数据后）：协议追踪基础设施

**功能侧：**
- [ ] 接入真实 OneBot v11 HTTP 适配器（最小可用）
- [ ] 真实协议事件 → `protocol_packets` 写入
- [ ] 消息 ↔ 协议报文双向追踪

**维护侧：**
- [ ] Protocol packets TTL 清理（此时有真数据可清理）
- [ ] 文件一致性检查（此时有真文件需要管理）

### 阶段 3（有协议追踪后）：审计与高级维护

**功能侧：**
- [ ] `audit_events` 写入（Bot 启动/停止、配置变更）
- [ ] 数据导出（会话消息、协议报文）

**维护侧：**
- [ ] 审计日志按类型保留策略
- [ ] 应用内维护面板增强（存储估算、最近日志）

---

## 6. 设计原则总结

1. **维护机制服务于真实数据**：不做"维护空数据库"的过度设计
2. **Schema 升级安全网优先**：migration 兼容性测试是维护工作的核心
3. **Bot 管理先于协议适配**：先让 `bots` 表活起来，再接入真实协议
4. **推迟假数据**：虚拟协议报文等真协议适配进来后再考虑是否还需要
5. **Settings 页面从空白到可用**：这是用户感知维护能力的唯一入口

---

## 7. 相关文档索引

| 文档 | 内容 | 本文档对其态度 |
|------|------|---------------|
| [docs/specs/database/migrations.md](../database/migrations.md) | 迁移规范、重建表流程 | ✅ 遵循，已在代码中实现 |
| [docs/specs/database/lifecycle-and-cleanup.md](../database/lifecycle-and-cleanup.md) | TTL、文件一致性、VACUUM | 🔶 设计正确，但**推迟到阶段 2 后实现** |
| [docs/specs/database/maintenance.md](../database/maintenance.md) | 维护面板、备份恢复、完整性检查 | 🔶 设计正确，但**精简为最小版本在阶段 1 实现** |
| [docs/specs/database/indexes.md](../database/indexes.md) | 38 个索引设计 | ✅ 已在 0001 migration 中落地 |
| [docs/specs/unibot-database-design-zh.md](../unibot-database-design-zh.md) | 总体架构 | ✅ 长期目标，分阶段逼近 |

---

## 附录 A：UUID v7 替换 CTE MAX+1 实施细节

### 依赖

`src-tauri/Cargo.toml` 新增：

```toml
uuid = { version = "1", features = ["v7"] }
```

### 工具函数

`src-tauri/src/utils.rs` 新增：

```rust
pub fn new_db_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
```

### 改写示例（以 `message.rs` 为例）

**之前：**

```rust
sqlx::query_as(
    r#"
    WITH next_id(value) AS (
        SELECT CAST(COALESCE(MAX(CAST(message_id AS INTEGER)), 0) + 1 AS TEXT)
        FROM messages
    )
    INSERT INTO messages (message_id, sender_user_id, source_type, source_id, content_json, quoted_message_id, created_at)
    SELECT value, ?1, ?2, ?3, ?4, ?5, ?6
    FROM next_id
    RETURNING id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
    "#,
)
.bind(&record.sender_user_id)
// ...
```

**之后：**

```rust
let id = crate::utils::new_db_id();
sqlx::query_as(
    r#"
    INSERT INTO messages (message_id, sender_user_id, source_type, source_id, content_json, quoted_message_id, created_at)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    RETURNING message_id AS id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
    "#,
)
.bind(&id)
.bind(&record.sender_user_id)
.bind(&record.source_type)
.bind(&record.source_id)
.bind(&record.content_json)
.bind(&record.quoted_message_id)
.bind(record.created_at as i64)
.fetch_one(&self.pool)
.await
```

### 受影响的 7 个 INSERT 语句

| 文件 | 方法 | 绑定参数变化 |
|------|------|-------------|
| `repo/message.rs` | `insert_message` | 去掉 CTE，Rust 侧生成 `message_id` |
| `repo/interaction.rs` | `insert_message_reaction` | 去掉 CTE，Rust 侧生成 `reaction_id` |
| `repo/interaction.rs` | `insert_poke` | 去掉 CTE，Rust 侧生成 `poke_id` |
| `repo/user/friends.rs` | `create_friend_request` | 去掉 CTE，Rust 侧生成 `request_id` |
| `repo/group/requests.rs` | `create_group_request` | 去掉 CTE，Rust 侧生成 `notification_seq` |
| `repo/group/content.rs` | `create_group_essence_message` | 去掉 CTE，Rust 侧生成 `essence_id` |
| `repo/group/events.rs` | `insert_group_event` | 去掉 CTE，Rust 侧生成 `event_id` |

### 验证方式

1. `cargo test --manifest-path src-tauri/Cargo.toml` 通过（repo smoke tests 不应因 ID 格式变化而失败，因为 ID 类型本来就是 `String`）
2. 手动验证：连续快速发送多条消息，不应再出现 `UNIQUE constraint failed` 错误
