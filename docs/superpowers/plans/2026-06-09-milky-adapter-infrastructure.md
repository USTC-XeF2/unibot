# Milky 协议适配基础设施实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 Milky 1.2 协议适配的基础设施层：消息序列号、DTO 类型、Backend trait、RuntimeManager、axum Server，使每个 Bot 能够独立启动 HTTP 服务。

**Architecture:** 在现有 Tauri + SQLite 架构上引入 `protocol/` 模块。`ProtocolRuntimeManager` 管理每个 Bot 的 axum 服务器生命周期。`VirtualBackend` 通过 `ServiceHub` 操作虚拟 IM。`MilkyAdapter` 负责内部类型和 Milky 1.2 协议格式之间的转换。

**Tech Stack:** Rust 2024, axum 0.7, tower, tokio-stream, async-trait, serde, sqlx 0.8, SQLite

---

## File Structure

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | 添加 axum, tower, tokio-stream, async-trait 依赖 |
| `src-tauri/src/protocol/mod.rs` | Create | 协议模块入口，导出公共类型 |
| `src-tauri/src/protocol/types.rs` | Create | Milky 1.2 DTO 和消息段类型 |
| `src-tauri/src/protocol/adapter.rs` | Create | 内部类型与 Milky 协议之间的转换 |
| `src-tauri/src/protocol/backend.rs` | Create | ProtocolBackend trait 和 VirtualBackend |
| `src-tauri/src/protocol/runtime.rs` | Create | ProtocolRuntimeManager，Bot 生命周期管理 |
| `src-tauri/src/protocol/server.rs` | Create | axum HTTP server，鉴权，SSE |
| `src-tauri/src/models/internal.rs` | Modify | 扩展 InternalEvent::Message，新增 GroupMemberLeft |
| `src-tauri/src/models/entities.rs` | Modify | 添加 BotConfig 结构 |
| `src-tauri/src/persistence/migrations/0002_message_seq.sql` | Create | 消息序列号增量迁移 |
| `src-tauri/src/persistence/migrations/mod.rs` | Modify | 注册 0002 迁移 |
| `src-tauri/src/persistence/repo/message.rs` | Modify | 添加 milky_message_seq 分配和查询 |
| `src-tauri/src/services/message.rs` | Modify | 发送完整 InternalEvent::Message |
| `src-tauri/src/services/bot.rs` | Modify | 生成完整 Bot 配置，委托 RuntimeManager |
| `src-tauri/src/commands/bot.rs` | Modify | 注入 ProtocolRuntimeManager state |
| `src-tauri/src/lib.rs` | Modify | 注册 protocol 模块和 RuntimeManager |

---

## Task 1: 添加协议依赖到 Cargo.toml

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 axum 和相关依赖**

在 `[dependencies]` 节添加：

```toml
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
tokio-stream = { version = "0.1", features = ["sync"] }
async-trait = "0.1"
```

修改 `tokio` features，添加 `"net"`：

```toml
tokio = { version = "1", features = ["sync", "fs", "net"] }
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 成功编译，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "deps: add axum, tower, tokio-stream, async-trait for protocol server"
```

---

## Task 2: 消息序列号数据库迁移

**Files:**
- Create: `src-tauri/src/persistence/migrations/0002_message_seq.sql`
- Modify: `src-tauri/src/persistence/migrations/mod.rs`

- [ ] **Step 1: 创建 0002_message_seq.sql**

```sql
-- 消息序列号计数器表
CREATE TABLE message_seq_counter (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    next_seq INTEGER NOT NULL DEFAULT 1
);
INSERT INTO message_seq_counter (id, next_seq) VALUES (1, 1);

-- 为 messages 表添加 Milky 协议序列号列
ALTER TABLE messages ADD COLUMN milky_message_seq INTEGER;

-- 为历史消息回填稳定序列号（按 created_at, message_id 排序）
UPDATE messages
SET milky_message_seq = (
    SELECT next_seq - 1 + row_num
    FROM (
        SELECT message_id, ROW_NUMBER() OVER (ORDER BY created_at, message_id) AS row_num
        FROM messages
    ) numbered
    WHERE numbered.message_id = messages.message_id
)
WHERE milky_message_seq IS NULL;

-- 更新计数器为最大值 + 1
UPDATE message_seq_counter
SET next_seq = (SELECT COALESCE(MAX(milky_message_seq), 0) + 1 FROM messages)
WHERE id = 1;

-- 创建唯一索引
CREATE UNIQUE INDEX idx_messages_milky_seq ON messages(milky_message_seq);
```

- [ ] **Step 2: 注册迁移到 migrations/mod.rs**

```rust
pub fn all_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: "0001",
            sql: include_str!("0001_initial_schema.sql"),
        },
        Migration {
            version: "0002",
            sql: include_str!("0002_message_seq.sql"),
        },
    ]
}
```

- [ ] **Step 3: 运行迁移测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migrates_from_blank_to_latest`
Expected: 测试通过，验证 schema_version 为 "0002"

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/persistence/migrations/
git commit -m "feat(db): add milky_message_seq migration with backfill and unique index"
```

---

## Task 3: Repo 支持消息序列号分配和查询

**Files:**
- Modify: `src-tauri/src/persistence/repo/message.rs`

- [ ] **Step 1: 在 MessageRecord 添加 milky_message_seq 字段**

修改 `MessageRecord` 结构体（在第 17 行 `created_at` 之前添加）：

```rust
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub sender_user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub receiver_user_id: Option<String>,
    pub group_id: Option<String>,
    pub bot_id: Option<String>,
    pub content_json: String,
    pub quoted_message_id: Option<String>,
    pub is_recalled: bool,
    pub recalled_by_user_id: Option<String>,
    pub milky_message_seq: i64,
    pub created_at: u64,
}
```

- [ ] **Step 2: 将 insert_message 改为事务并分配序列号**

替换 `insert_message` 方法的完整实现：

```rust
    pub async fn insert_message(
        &self,
        record: NewMessageRecord,
    ) -> Result<MessageRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // 分配全局单调递增序列号
        let seq: i64 = sqlx::query_scalar(
            "UPDATE message_seq_counter SET next_seq = next_seq + 1 WHERE id = 1 RETURNING next_seq - 1"
        )
        .fetch_one(&mut *tx)
        .await?;

        let is_private = record.source_type == "private" || record.source_type == "temp";
        let receiver_user_id: Option<&str> = if is_private {
            if record.sender_user_id == record.owner_user_id {
                Some(&record.source_id)
            } else {
                Some(&record.owner_user_id)
            }
        } else {
            None
        };
        let group_id: Option<&str> = if !is_private {
            Some(&record.source_id)
        } else {
            None
        };

        let id = crate::utils::new_db_id();
        let row = sqlx::query_as::<_, MessageRecord>(
            r#"
            INSERT INTO messages (
                message_id, message_scene, peer_id, message_seq, sender_user_id,
                receiver_user_id, group_id, bot_id, content_json, quoted_message_id, created_at,
                milky_message_seq
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            RETURNING message_id AS id,
                      sender_user_id,
                      message_scene AS source_type,
                      peer_id AS source_id,
                      receiver_user_id,
                      group_id,
                      bot_id,
                      content_json,
                      quoted_message_id,
                      is_recalled,
                      recalled_by_user_id,
                      milky_message_seq,
                      created_at
            "#,
        )
        .bind(&id)
        .bind(&record.source_type)
        .bind(&record.source_id)
        .bind(&id) // message_seq = message_id (UUID)
        .bind(&record.sender_user_id)
        .bind(receiver_user_id)
        .bind(group_id)
        .bind(record.bot_id.as_deref())
        .bind(&record.content_json)
        .bind(record.quoted_message_id.as_deref())
        .bind(record.created_at as i64)
        .bind(seq)
        .fetch_one(&mut *tx)
        .await?;

        let conversation_id = if is_private {
            format!(
                "{}:{}:{}",
                record.owner_user_id, record.source_type, record.source_id
            )
        } else {
            format!("{}:group:{}", record.owner_user_id, record.source_id)
        };

        if is_private {
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    last_message_id, unread_count, updated_at
                ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, 0, ?6)
                ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
                WHERE conversation_scene IN ('private', 'temp')
                DO UPDATE SET
                    last_message_id = excluded.last_message_id,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(&record.owner_user_id)
            .bind(&record.source_type)
            .bind(&record.source_id)
            .bind(&row.id)
            .bind(record.created_at as i64)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    last_message_id, unread_count, updated_at
                ) VALUES (?1, ?2, 'group', NULL, ?3, ?4, 0, ?5)
                ON CONFLICT(owner_user_id, conversation_scene, group_id)
                WHERE conversation_scene = 'group'
                DO UPDATE SET
                    last_message_id = excluded.last_message_id,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(&record.owner_user_id)
            .bind(&record.source_id)
            .bind(&row.id)
            .bind(record.created_at as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(row)
    }
```

- [ ] **Step 3: 添加序列号查询方法**

在 `get_message_count` 方法之后添加：

```rust
    pub async fn get_message_by_milky_seq(
        &self,
        seq: i64,
    ) -> Result<Option<MessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT message_id AS id,
                   sender_user_id,
                   message_scene AS source_type,
                   COALESCE(group_id, peer_id) AS source_id,
                   receiver_user_id,
                   group_id,
                   bot_id,
                   content_json,
                   quoted_message_id,
                   is_recalled,
                   recalled_by_user_id,
                   milky_message_seq,
                   created_at
            FROM messages
            WHERE milky_message_seq = ?1
            "#,
        )
        .bind(seq)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_milky_seq_by_message_id(
        &self,
        message_id: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT milky_message_seq FROM messages WHERE message_id = ?1")
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await
    }
```

- [ ] **Step 4: 更新其他查询的 RETURNING/SELECT 添加 milky_message_seq**

修改 `mark_message_recalled` 的 RETURNING（在 `recalled_by_user_id` 后添加 `milky_message_seq,`）：

```sql
RETURNING message_id AS id,
          sender_user_id,
          message_scene AS source_type,
          peer_id AS source_id,
          receiver_user_id,
          group_id,
          bot_id,
          content_json,
          quoted_message_id,
          is_recalled,
          recalled_by_user_id,
          milky_message_seq,
          created_at
```

修改 `get_message_by_id` 的 SELECT（在 `recalled_by_user_id` 后添加 `milky_message_seq,`）：

```sql
SELECT message_id AS id,
       sender_user_id,
       message_scene AS source_type,
       COALESCE(group_id, peer_id) AS source_id,
       receiver_user_id,
       group_id,
       bot_id,
       content_json,
       quoted_message_id,
       is_recalled,
       recalled_by_user_id,
       milky_message_seq,
       created_at
FROM messages
WHERE message_id = ?1
```

修改 `list_messages` 的 private 分支和 group 分支的 SELECT（都在 `recalled_by_user_id` 后添加 `milky_message_seq,`）。

- [ ] **Step 5: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有现有测试通过（包括 BotService 测试）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/persistence/repo/message.rs
git commit -m "feat(repo): allocate milky_message_seq in transaction, add lookup methods"
```

---

## Task 4: 升级 InternalEvent 契约

**Files:**
- Modify: `src-tauri/src/models/internal.rs`

- [ ] **Step 1: 扩展 InternalEvent::Message 字段**

替换 `InternalEvent::Message` variant：

```rust
    Message {
        message_id: DbId,
        message_seq: i64,
        sender_user_id: DbId,
        source: MessageSource,
        content: Vec<MessageSegment>,
        origin_bot_id: Option<DbId>,
        time: u64,
    },
```

- [ ] **Step 2: 新增 GroupMemberLeft 事件**

在 `GroupMemberJoined` 之后添加：

```rust
    GroupMemberLeft {
        group_id: DbId,
        operator_user_id: Option<DbId>,
        target_user_id: DbId,
        time: u64,
    },
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过。`MessageService::send` 中的事件构造会报错，将在 Task 5 修复。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models/internal.rs
git commit -m "feat(models): expand InternalEvent::Message with full identity, add GroupMemberLeft"
```

---

## Task 5: 更新 MessageService 发送完整事件

**Files:**
- Modify: `src-tauri/src/services/message.rs`

- [ ] **Step 1: 更新 send 方法中的事件构造**

找到 `send` 方法中的事件构造代码（约第 130-141 行），替换为：

```rust
        let event = InternalEvent::Message {
            message_id: saved.id.clone(),
            message_seq: saved.milky_message_seq,
            sender_user_id: user_id.clone(),
            source: source.clone(),
            content: content.clone(),
            origin_bot_id: bot_id.clone(),
            time: now,
        };
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有测试通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/message.rs
git commit -m "feat(message): emit complete InternalEvent::Message with milky seq and origin"
```

---

## Task 6: 创建 Milky 1.2 DTO 类型

**Files:**
- Create: `src-tauri/src/protocol/mod.rs`
- Create: `src-tauri/src/protocol/types.rs`

- [ ] **Step 1: 创建 protocol/mod.rs**

```rust
pub mod adapter;
pub mod backend;
pub mod runtime;
pub mod server;
pub mod types;

pub use adapter::MilkyAdapter;
pub use backend::{ProtocolBackend, VirtualBackend};
pub use runtime::ProtocolRuntimeManager;
pub use types::*;
```

- [ ] **Step 2: 创建 protocol/types.rs**

```rust
use serde::{Deserialize, Serialize};

// ========== API 请求/响应基座 ==========

#[derive(Debug, Clone, Deserialize)]
pub struct MilkyApiRequest {
    pub api_name: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MilkyApiResponse {
    pub status: String,
    pub retcode: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl MilkyApiResponse {
    pub fn ok(data: impl Serialize) -> Self {
        Self {
            status: "ok".to_string(),
            retcode: 0,
            data: serde_json::to_value(data).ok(),
            message: None,
        }
    }

    pub fn failed(retcode: i32, message: impl Into<String>) -> Self {
        Self {
            status: "failed".to_string(),
            retcode,
            data: None,
            message: Some(message.into()),
        }
    }
}

pub type MilkyApiData = serde_json::Value;

// ========== Milky 事件 ==========

#[derive(Debug, Clone, Serialize)]
pub struct MilkyEvent {
    pub time: u64,
    pub self_id: i64,
    pub event_type: String,
    pub data: serde_json::Value,
}

// ========== Milky 消息段 ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum MilkySegment {
    Text { text: String },
    Image { file: String, url: String },
    At { qq: String },
    AtAll {},
    Face { id: String },
}

// ========== Bot 运行时上下文 ==========

#[derive(Debug, Clone)]
pub struct BotRuntimeContext {
    pub bot_id: String,
    pub bound_user_id: String,
    pub access_token: String,
    pub listen_addr: std::net::SocketAddr,
}

// ========== Bot 配置 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub version: i32,
    pub protocol: String,
    pub http: HttpConfig,
    pub access_token: String,
    pub event_transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            version: 1,
            protocol: "milky".to_string(),
            http: HttpConfig {
                host: "127.0.0.1".to_string(),
                port: 3001,
            },
            access_token: uuid::Uuid::new_v4().to_string(),
            event_transport: "sse".to_string(),
        }
    }
}
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/protocol/
git commit -m "feat(protocol): add Milky 1.2 DTO types, BotConfig, and runtime context"
```

---

## Task 7: 创建 MessageSegment 转换

**Files:**
- Modify: `src-tauri/src/protocol/adapter.rs`

- [ ] **Step 1: 创建 adapter.rs 的 segment 转换**

```rust
use crate::models::MessageSegment;
use crate::protocol::types::MilkySegment;

pub fn internal_to_milky_segment(seg: &MessageSegment) -> MilkySegment {
    match seg {
        MessageSegment::Text { text } => MilkySegment::Text { text: text.clone() },
        MessageSegment::Image { file, url } => MilkySegment::Image {
            file: file.clone(),
            url: url.clone(),
        },
        MessageSegment::At { target } => MilkySegment::At { qq: target.clone() },
        MessageSegment::AtAll => MilkySegment::AtAll {},
        MessageSegment::Face { id } => MilkySegment::Face { id: id.clone() },
    }
}

pub fn milky_to_internal_segment(seg: &MilkySegment) -> MessageSegment {
    match seg {
        MilkySegment::Text { text } => MessageSegment::Text { text: text.clone() },
        MilkySegment::Image { file, url } => MessageSegment::Image {
            file: file.clone(),
            url: url.clone(),
        },
        MilkySegment::At { qq } => MessageSegment::At { target: qq.clone() },
        MilkySegment::AtAll {} => MessageSegment::AtAll,
        MilkySegment::Face { id } => MessageSegment::Face { id: id.clone() },
    }
}

pub fn internal_to_milky_segments(segments: &[MessageSegment]) -> Vec<MilkySegment> {
    segments.iter().map(internal_to_milky_segment).collect()
}

pub fn milky_to_internal_segments(segments: &[MilkySegment]) -> Vec<MessageSegment> {
    segments.iter().map(milky_to_internal_segment).collect()
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/protocol/adapter.rs
git commit -m "feat(protocol): add MessageSegment <-> MilkySegment bidirectional conversion"
```

---

## Task 8: 创建 MilkyAdapter 核心转换

**Files:**
- Modify: `src-tauri/src/protocol/adapter.rs`

- [ ] **Step 1: 在 adapter.rs 中添加 MilkyAdapter 结构**

在文件末尾追加：

```rust
use crate::error::AppError;
use crate::models::{InternalEvent, MessageSource, UserProfile};
use crate::protocol::types::{MilkyApiResponse, MilkyEvent, MilkySegment};

#[derive(Debug, Clone, Default)]
pub struct MilkyAdapter;

impl MilkyAdapter {
    pub fn new() -> Self {
        Self
    }

    /// 将 AppError 映射为 Milky retcode 和 message
    pub fn adapt_error(&self, err: &AppError) -> (i32, String) {
        match err {
            AppError::Validation(msg) => (-400, msg.clone()),
            AppError::NotFound(msg) => (-404, msg.clone()),
            AppError::Conflict(msg) => (-409, msg.clone()),
            AppError::Storage(msg) => (-500, msg.clone()),
            AppError::Internal(msg) => (-500, msg.clone()),
        }
    }

    /// 将内部事件转换为 Milky Event
    pub fn adapt_event(&self, event: &InternalEvent) -> Option<MilkyEvent> {
        match event {
            InternalEvent::Message {
                message_seq,
                sender_user_id,
                source,
                content,
                time,
                ..
            } => {
                let self_id = sender_user_id.parse::<i64>().ok()?;
                let segments = internal_to_milky_segments(content);
                let data = match source {
                    MessageSource::Private { peer_user_id } => {
                        serde_json::json!({
                            "message_type": "private",
                            "user_id": peer_user_id.parse::<i64>().ok()?,
                            "message_seq": message_seq,
                            "message": segments,
                        })
                    }
                    MessageSource::Group { group_id } => {
                        serde_json::json!({
                            "message_type": "group",
                            "group_id": group_id.parse::<i64>().ok()?,
                            "user_id": self_id,
                            "message_seq": message_seq,
                            "message": segments,
                        })
                    }
                };
                Some(MilkyEvent {
                    time: *time / 1000, // ms -> s
                    self_id,
                    event_type: "message_receive".to_string(),
                    data,
                })
            }
            _ => None,
        }
    }

    /// 生成 get_login_info 响应数据
    pub fn adapt_login_info(&self, user: &UserProfile) -> serde_json::Value {
        serde_json::json!({
            "user_id": user.user_id.parse::<i64>().unwrap_or(0),
            "nickname": user.nickname,
        })
    }

    /// 生成 send_message 响应数据
    pub fn adapt_message_send(&self, message_id: &str, message_seq: i64) -> serde_json::Value {
        serde_json::json!({
            "message_id": message_id,
            "message_seq": message_seq,
        })
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/adapter.rs
git commit -m "feat(protocol): add MilkyAdapter for error, event, and API response conversion"
```

---

## Task 9: 更新 BotService 生成完整配置

**Files:**
- Modify: `src-tauri/src/services/bot.rs`
- Modify: `src-tauri/src/models/entities.rs`

- [ ] **Step 1: 在 entities.rs 添加 BotConfig 导入**

在 `src-tauri/src/models/entities.rs` 顶部添加：

```rust
pub use crate::protocol::types::{BotConfig, HttpConfig};
```

- [ ] **Step 2: 修改 BotService::create_bot 生成完整配置**

替换 `BotService::create_bot` 方法中写配置文件的逻辑（约第 47-64 行）：

```rust
        let bot_id = new_db_id();
        let bots_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| AppError::internal(format!("app dir error: {err}")))?
            .join("bots");
        let config_path = bots_dir.join(format!("{bot_id}.json"));
        let config_path_string = config_path
            .to_str()
            .ok_or_else(|| AppError::internal("bot config path is not valid UTF-8"))?
            .to_string();

        // 分配最小可用端口（从 3001 开始）
        let port = self.allocate_port().await?;
        let config = BotConfig {
            version: 1,
            protocol: "milky".to_string(),
            http: HttpConfig {
                host: "127.0.0.1".to_string(),
                port,
            },
            access_token: uuid::Uuid::new_v4().to_string(),
            event_transport: "sse".to_string(),
        };
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::internal(format!("serialize config: {e}")))?;

        tokio::fs::create_dir_all(&bots_dir)
            .await
            .map_err(|err| AppError::internal(format!("create bots dir: {err}")))?;
        tokio::fs::write(&config_path, config_json)
            .await
            .map_err(|err| AppError::internal(format!("write config: {err}")))?;
```

- [ ] **Step 3: 添加 allocate_port 辅助方法**

在 `BotService` impl 中添加：

```rust
    async fn allocate_port(&self) -> AppResult<u16> {
        let bots = self.repo.list_bots().await?;
        let mut used = Vec::new();
        for bot in bots {
            if let Ok(text) = tokio::fs::read_to_string(&bot.config_path).await {
                if let Ok(cfg) = serde_json::from_str::<BotConfig>(&text) {
                    used.push(cfg.http.port);
                }
            }
        }
        for port in 3001..=65535 {
            if !used.contains(&port) {
                return Ok(port);
            }
        }
        Err(AppError::validation("no available port in range 3001-65535"))
    }
```

- [ ] **Step 4: 添加 BotConfig 导入**

在 `src-tauri/src/services/bot.rs` 顶部添加：

```rust
use crate::models::BotConfig;
use crate::protocol::types::{HttpConfig};
```

- [ ] **Step 5: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有测试通过（包括 delete_bot 测试）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/bot.rs src-tauri/src/models/entities.rs
git commit -m "feat(bot): generate full BotConfig with allocated port and random token"
```

---

## Task 10: 创建 ProtocolBackend trait 和 VirtualBackend

**Files:**
- Create: `src-tauri/src/protocol/backend.rs`

- [ ] **Step 1: 创建 backend.rs**

```rust
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{InternalEvent, MessageSegment, MessageSource};
use crate::protocol::types::{BotRuntimeContext, MilkyApiData, MilkyApiRequest};
use crate::services::ServiceHub;
use crate::utils::now_ts;

#[async_trait]
pub trait ProtocolBackend: Send + Sync {
    fn subscribe_events(
        &self,
        bot: &BotRuntimeContext,
    ) -> AppResult<broadcast::Receiver<InternalEvent>>;

    async fn call_api(
        &self,
        bot: &BotRuntimeContext,
        api: MilkyApiRequest,
    ) -> AppResult<MilkyApiData>;
}

#[derive(Clone)]
pub struct VirtualBackend {
    service_hub: ServiceHub,
    core: CoreContainer,
}

impl VirtualBackend {
    pub fn new(service_hub: ServiceHub, core: CoreContainer) -> Self {
        Self {
            service_hub,
            core,
        }
    }
}

#[async_trait]
impl ProtocolBackend for VirtualBackend {
    fn subscribe_events(
        &self,
        bot: &BotRuntimeContext,
    ) -> AppResult<broadcast::Receiver<InternalEvent>> {
        let ctx = self
            .core
            .user_context(&bot.bound_user_id)
            .ok_or_else(|| AppError::not_found("bound user not registered"))?;
        Ok(ctx.event_tx.subscribe())
    }

    async fn call_api(
        &self,
        bot: &BotRuntimeContext,
        api: MilkyApiRequest,
    ) -> AppResult<MilkyApiData> {
        match api.api_name.as_str() {
            "get_login_info" => {
                let user = self
                    .service_hub
                    .user
                    .get_user_by_id(&bot.bound_user_id)
                    .await?
                    .ok_or_else(|| AppError::not_found("bound user not found"))?;
                Ok(serde_json::json!({
                    "user_id": user.user_id.parse::<i64>().unwrap_or(0),
                    "nickname": user.nickname,
                }))
            }
            "send_private_message" => {
                let user_id = api
                    .params
                    .get("user_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing user_id"))?
                    .to_string();
                let message = api
                    .params
                    .get("message")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| AppError::validation("missing message"))?;
                let segments: Vec<MessageSegment> = message
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                let result = self
                    .service_hub
                    .message
                    .send(
                        &self.core,
                        bot.bound_user_id.clone(),
                        MessageSource::Private {
                            peer_user_id: user_id,
                        },
                        segments,
                        None,
                        Some(bot.bot_id.clone()),
                    )
                    .await?;
                Ok(serde_json::json!({
                    "message_id": result.id,
                    "message_seq": self
                        .service_hub
                        .message
                        .get_milky_seq(&result.id)
                        .await
                        .unwrap_or(0),
                }))
            }
            "send_group_message" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                let message = api
                    .params
                    .get("message")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| AppError::validation("missing message"))?;
                let segments: Vec<MessageSegment> = message
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                let result = self
                    .service_hub
                    .message
                    .send(
                        &self.core,
                        bot.bound_user_id.clone(),
                        MessageSource::Group { group_id },
                        segments,
                        None,
                        Some(bot.bot_id.clone()),
                    )
                    .await?;
                Ok(serde_json::json!({
                    "message_id": result.id,
                    "message_seq": self
                        .service_hub
                        .message
                        .get_milky_seq(&result.id)
                        .await
                        .unwrap_or(0),
                }))
            }
            _ => Err(AppError::not_found(format!("unknown api: {}", api.api_name))),
        }
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过。VirtualBackend 中引用了 `get_milky_seq` 方法，将在 Task 11 中添加。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/backend.rs
git commit -m "feat(protocol): add ProtocolBackend trait and VirtualBackend impl"
```

---

## Task 11: 添加 get_milky_seq 到 MessageService

**Files:**
- Modify: `src-tauri/src/services/message.rs`

- [ ] **Step 1: 添加 get_milky_seq 方法**

在 `MessageService` impl 中添加：

```rust
    pub async fn get_milky_seq(&self, message_id: &str) -> AppResult<i64> {
        self.repo
            .get_milky_seq_by_message_id(message_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("message {message_id} not found")))
    }
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/message.rs
git commit -m "feat(message): add get_milky_seq helper for protocol adapter"
```

---

## Task 12: 创建 ProtocolRuntimeManager

**Files:**
- Create: `src-tauri/src/protocol/runtime.rs`

- [ ] **Step 1: 创建 runtime.rs**

```rust
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::BotProfile;
use crate::persistence::BotRepo;
use crate::protocol::adapter::MilkyAdapter;
use crate::protocol::backend::VirtualBackend;
use crate::protocol::server::spawn_server;
use crate::protocol::types::{BotConfig, BotRuntimeContext};
use crate::services::ServiceHub;
use crate::utils::now_ts;

pub struct ProtocolRuntimeManager {
    servers: Mutex<HashMap<String, RunningProtocolServer>>,
    bot_repo: BotRepo,
    service_hub: ServiceHub,
    core: CoreContainer,
    app_data_dir: std::path::PathBuf,
}

pub struct RunningProtocolServer {
    pub bot_id: String,
    pub session_id: String,
    pub bound_addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl ProtocolRuntimeManager {
    pub fn new(
        bot_repo: BotRepo,
        service_hub: ServiceHub,
        core: CoreContainer,
        app_data_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            bot_repo,
            service_hub,
            core,
            app_data_dir,
        }
    }

    pub async fn start_bot(&self, bot_id: &str) -> AppResult<SocketAddr> {
        let mut servers = self.servers.lock().await;
        if servers.contains_key(bot_id) {
            return Err(AppError::conflict("bot is already running"));
        }

        let bot = self
            .bot_repo
            .get_bot_by_id(bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        let config_str = tokio::fs::read_to_string(&bot.config_path)
            .await
            .map_err(|e| AppError::internal(format!("read config: {e}")))?;
        let config: BotConfig = serde_json::from_str(&config_str)
            .map_err(|e| AppError::internal(format!("parse config: {e}")))?;

        let addr = format!("{}:{}", config.http.host, config.http.port)
            .parse::<SocketAddr>()
            .map_err(|e| AppError::internal(format!("invalid bind address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| AppError::internal(format!("port {} in use: {}", config.http.port, e)))?;
        let bound_addr = listener
            .local_addr()
            .map_err(|e| AppError::internal(format!("local_addr: {e}")))?;

        let session_id = crate::utils::new_db_id();
        let session_name = format!("调试会话 {}", now_ts());

        let session = self
            .bot_repo
            .start_session(&session_id, bot_id, &session_name)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::conflict("bot is already running or does not exist")
                }
                _ => e.into(),
            })?;

        let backend = Arc::new(VirtualBackend::new(
            self.service_hub.clone(),
            self.core.clone(),
        ));
        let adapter = Arc::new(MilkyAdapter::new());

        let context = BotRuntimeContext {
            bot_id: bot_id.to_string(),
            bound_user_id: bot.bound_user_id,
            access_token: config.access_token,
            listen_addr: bound_addr,
        };

        let (shutdown_tx, join_handle) = match spawn_server(
            listener,
            context,
            backend,
            adapter,
        )
        .await
        {
            Ok((tx, handle)) => (tx, handle),
            Err(e) => {
                let _ = self.bot_repo.stop_active_sessions(bot_id).await;
                return Err(e);
            }
        };

        servers.insert(
            bot_id.to_string(),
            RunningProtocolServer {
                bot_id: bot_id.to_string(),
                session_id: session.session_id,
                bound_addr,
                shutdown_tx: Some(shutdown_tx),
                join_handle,
            },
        );

        Ok(bound_addr)
    }

    pub async fn stop_bot(&self, bot_id: &str) -> AppResult<()> {
        let mut servers = self.servers.lock().await;
        let running = servers
            .remove(bot_id)
            .ok_or_else(|| AppError::validation("bot is not running"))?;

        if let Some(tx) = running.shutdown_tx {
            let _ = tx.send(());
        }

        let _ = running.join_handle.await;
        self.bot_repo.stop_active_sessions(bot_id).await?;
        Ok(())
    }

    pub async fn shutdown_all(&self) {
        let mut servers = self.servers.lock().await;
        for (_, mut running) in servers.drain() {
            if let Some(tx) = running.shutdown_tx.take() {
                let _ = tx.send(());
            }
        }
        // Drop lock before awaiting handles
        let handles: Vec<_> = servers
            .drain()
            .map(|(_, running)| running.join_handle)
            .collect();
        drop(servers);
        for handle in handles {
            let _ = handle.await;
        }
    }

    pub async fn is_running(&self, bot_id: &str) -> bool {
        let servers = self.servers.lock().await;
        servers.contains_key(bot_id)
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过（server::spawn_server 将在 Task 13 中定义）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/runtime.rs
git commit -m "feat(protocol): add ProtocolRuntimeManager for bot lifecycle management"
```

---

## Task 13: 创建 ProtocolServer (axum)

**Files:**
- Create: `src-tauri/src/protocol/server.rs`

- [ ] **Step 1: 创建 server.rs**

```rust
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use axum::response::sse::Event;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::AppError;
use crate::protocol::adapter::MilkyAdapter;
use crate::protocol::backend::ProtocolBackend;
use crate::protocol::types::{
    BotRuntimeContext, MilkyApiRequest, MilkyApiResponse, MilkyEvent,
};

#[derive(Clone)]
struct ServerState {
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<MilkyAdapter>,
}

#[derive(serde::Deserialize)]
struct EventQuery {
    access_token: Option<String>,
}

fn extract_token(
    headers: &axum::http::HeaderMap,
    query: &EventQuery,
) -> Option<String> {
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    query.access_token.clone()
}

async fn event_handler(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<EventQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let token = extract_token(&headers, &query).ok_or(StatusCode::UNAUTHORIZED)?;
    if token != state.context.access_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let rx = state
        .backend
        .subscribe_events(&state.context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let adapter = state.adapter.clone();
    let bot_id = state.context.bot_id.clone();

    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        let adapter = adapter.clone();
        let bot_id = bot_id.clone();
        async move {
            match result {
                Ok(event) => {
                    // 过滤当前 Bot 自己发送的消息 Echo
                    if let crate::models::InternalEvent::Message { origin_bot_id, .. } = &event {
                        if origin_bot_id.as_ref() == Some(&bot_id) {
                            return None;
                        }
                    }
                    let milky_event = adapter.adapt_event(&event)?;
                    let json = serde_json::to_string(&milky_event).ok()?;
                    Some(Ok::<_, Infallible>(Event::default().event("milky_event").data(json)))
                }
                Err(_) => None,
            }
        }
    });

    Ok(Sse::new(stream))
}

async fn api_handler(
    State(state): State<Arc<ServerState>>,
    Path(api_name): Path<String>,
    Query(query): Query<EventQuery>,
    headers: axum::http::HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let token = match extract_token(&headers, &query) {
        Some(t) => t,
        None => {
            return (StatusCode::UNAUTHORIZED, axum::Json(MilkyApiResponse::failed(-401, "unauthorized")));
        }
    };
    if token != state.context.access_token {
        return (StatusCode::UNAUTHORIZED, axum::Json(MilkyApiResponse::failed(-401, "unauthorized")));
    }

    let request = MilkyApiRequest { api_name, params: body };
    match state.backend.call_api(&state.context, request).await {
        Ok(data) => {
            (StatusCode::OK, axum::Json(MilkyApiResponse::ok(data)))
        }
        Err(err) => {
            let (retcode, message) = state.adapter.adapt_error(&err);
            (StatusCode::OK, axum::Json(MilkyApiResponse::failed(retcode, message)))
        }
    }
}

pub async fn spawn_server(
    listener: tokio::net::TcpListener,
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<MilkyAdapter>,
) -> crate::error::AppResult<(
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
)> {
    let state = Arc::new(ServerState {
        context,
        backend,
        adapter,
    });

    let app = Router::new()
        .route("/event", get(event_handler))
        .route("/api/:api", post(api_handler))
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let graceful = server.with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = graceful.await {
            eprintln!("protocol server error: {e}");
        }
    });

    Ok((shutdown_tx, handle))
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/server.rs
git commit -m "feat(protocol): add axum ProtocolServer with auth, SSE, and API routing"
```

---

## Task 14: 集成生命周期到 BotService、Commands 和 lib.rs

**Files:**
- Modify: `src-tauri/src/services/bot.rs`
- Modify: `src-tauri/src/commands/bot.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 修改 BotService 委托 RuntimeManager**

在 `src-tauri/src/services/bot.rs` 中，修改 `start_bot` 和 `stop_bot`：

```rust
    pub async fn start_bot(
        &self,
        runtime: &crate::protocol::ProtocolRuntimeManager,
        bot_id: String,
    ) -> AppResult<crate::models::DebugSession> {
        // 先启动 runtime（它会创建 session）
        let _addr = runtime.start_bot(&bot_id).await?;

        // 返回最新的 active session
        let sessions = self.repo.list_sessions_by_bot(&bot_id).await?;
        let session = sessions
            .into_iter()
            .find(|s| s.ended_at.is_none())
            .ok_or_else(|| AppError::internal("session not found after start"))?;
        session.try_into()
    }

    pub async fn stop_bot(
        &self,
        runtime: &crate::protocol::ProtocolRuntimeManager,
        bot_id: String,
    ) -> AppResult<()> {
        runtime.stop_bot(&bot_id).await
    }

    pub async fn delete_bot(
        &self,
        runtime: &crate::protocol::ProtocolRuntimeManager,
        bot_id: String,
    ) -> AppResult<()> {
        // 如果正在运行，先停止
        if runtime.is_running(&bot_id).await {
            let _ = runtime.stop_bot(&bot_id).await;
        }

        let bot = self
            .repo
            .delete_bot_with_sessions(&bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        match tokio::fs::remove_file(&bot.config_path).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                eprintln!(
                    "failed to delete bot config file at {} after deleting bot {bot_id}: {err}",
                    bot.config_path
                );
            }
        }

        Ok(())
    }
```

删除旧的 `start_bot` 和 `stop_bot` 实现（只保留上面的新版本）。

- [ ] **Step 2: 修改 commands/bot.rs 注入 RuntimeManager**

```rust
use crate::models::{BotProfile, DebugSession};
use crate::protocol::ProtocolRuntimeManager;
use crate::services::ServiceHub;

use super::IntoCommandResult;

#[tauri::command]
pub async fn create_bot(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    bound_user_id: String,
    display_name: String,
) -> Result<BotProfile, String> {
    services
        .bot
        .create_bot(&app, bound_user_id, display_name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_bots(services: tauri::State<'_, ServiceHub>) -> Result<Vec<BotProfile>, String> {
    services.bot.list_bots().await.into_command_result()
}

#[tauri::command]
pub async fn delete_bot(
    runtime: tauri::State<'_, ProtocolRuntimeManager>,
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<(), String> {
    services
        .bot
        .delete_bot(&runtime, bot_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn start_bot(
    runtime: tauri::State<'_, ProtocolRuntimeManager>,
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<DebugSession, String> {
    services
        .bot
        .start_bot(&runtime, bot_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn stop_bot(
    runtime: tauri::State<'_, ProtocolRuntimeManager>,
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<(), String> {
    services
        .bot
        .stop_bot(&runtime, bot_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_debug_sessions(
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<Vec<DebugSession>, String> {
    services
        .bot
        .list_sessions(bot_id)
        .await
        .into_command_result()
}
```

- [ ] **Step 3: 修改 lib.rs 注册 protocol 模块和 RuntimeManager**

在 `src-tauri/src/lib.rs` 顶部添加 `mod protocol;`：

```rust
mod commands;
mod core;
mod error;
mod models;
mod persistence;
mod protocol;
mod services;
mod utils;
```

在 `setup` 闭包中，创建 `ProtocolRuntimeManager` 并 manage：

```rust
use protocol::ProtocolRuntimeManager;

// ... 在 app.manage(service_hub); 之前添加：

let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|err| format!("failed to get app data dir: {err}"))?;
let protocol_runtime = ProtocolRuntimeManager::new(
    bot_repo.clone(),
    service_hub.clone(),
    core.clone(),
    app_data_dir,
);
app.manage(protocol_runtime);
```

在 `invoke_handler` 中确保 `bot` commands 已注册（已存在，无需修改）。

- [ ] **Step 4: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 5: 运行测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有测试通过（BotService 测试需要更新以适配新签名）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/bot.rs src-tauri/src/commands/bot.rs src-tauri/src/lib.rs
git commit -m "feat(protocol): integrate ProtocolRuntimeManager into Bot lifecycle"
```

---

## Task 15: 迁移和集成测试

**Files:**
- Modify: `src-tauri/src/persistence/migrator.rs`
- Create: `src-tauri/src/protocol/tests.rs`

- [ ] **Step 1: 更新迁移兼容性测试**

在 `src-tauri/src/persistence/migrator.rs` 中，更新 `migrates_from_blank_to_latest` 测试：

```rust
    #[sqlx::test]
    async fn migrates_from_blank_to_latest(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        run_migrations(&pool)
            .await
            .map_err(sqlx::Error::Protocol)?;

        let version: String = sqlx::query_scalar(
            "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(version, "0002");

        let table_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'")
                .fetch_one(&pool)
                .await?;

        // 26 tables from 0001 + 1 message_seq_counter from 0002 = 27
        assert_eq!(table_count, 27);

        // Verify message_seq_counter exists and is initialized
        let next_seq: i64 =
            sqlx::query_scalar("SELECT next_seq FROM message_seq_counter WHERE id = 1")
                .fetch_one(&pool)
                .await?;
        assert_eq!(next_seq, 1);

        Ok(())
    }
```

- [ ] **Step 2: 创建 protocol 模块测试**

在 `src-tauri/src/protocol/tests.rs` 中：

```rust
#[cfg(test)]
mod tests {
    use crate::protocol::adapter::{internal_to_milky_segment, milky_to_internal_segment};
    use crate::models::MessageSegment;
    use crate::protocol::types::MilkySegment;

    #[test]
    fn text_segment_roundtrip() {
        let internal = MessageSegment::Text { text: "hello".to_string() };
        let milky = internal_to_milky_segment(&internal);
        let back = milky_to_internal_segment(&milky);
        assert_eq!(internal, back);
    }

    #[test]
    fn at_segment_qq_conversion() {
        let internal = MessageSegment::At { target: "12345".to_string() };
        let milky = internal_to_milky_segment(&internal);
        match milky {
            MilkySegment::At { qq } => assert_eq!(qq, "12345"),
            _ => panic!("expected At segment"),
        }
    }

    #[test]
    fn milky_response_serialization() {
        use crate::protocol::types::MilkyApiResponse;
        let ok = MilkyApiResponse::ok(serde_json::json!({"user_id": 10001}));
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"retcode\":0"));

        let failed = MilkyApiResponse::failed(-400, "bad request");
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"retcode\":-400"));
    }
}
```

在 `src-tauri/src/protocol/mod.rs` 底部添加：

```rust
#[cfg(test)]
mod tests;
```

- [ ] **Step 3: 运行全部测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 所有测试通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/persistence/migrator.rs src-tauri/src/protocol/tests.rs src-tauri/src/protocol/mod.rs
git commit -m "test(protocol): add migration compatibility and segment conversion tests"
```

---

## Self-Review

### Spec Coverage

| Spec Section | Task | Status |
|-------------|------|--------|
| 4.1 ProtocolRuntimeManager | Task 12 | ✅ |
| 4.2 ProtocolServer | Task 13 | ✅ |
| 4.3 ProtocolBackend / VirtualBackend | Task 10 | ✅ |
| 4.4 MilkyAdapter | Task 7, 8 | ✅ |
| 5. Bot 配置和端口 | Task 9 | ✅ |
| 6. 内部事件契约 | Task 4, 5 | ✅ |
| 6.1 消息序列 | Task 2, 3 | ✅ |
| 7.1 启动生命周期 | Task 12, 14 | ✅ |
| 7.2 停止生命周期 | Task 12, 14 | ✅ |
| 7.3 删除生命周期 | Task 14 | ✅ |
| 7.4 应用退出 | Task 12 (shutdown_all) | ✅ |
| 12. Slice A 验收 | Task 2, 3, 15 | ✅ |
| 12. Slice B 验收 | Task 12, 13, 14 | ✅ |
| 13. 测试策略 | Task 15 | ✅ |

### Placeholder Scan

- [x] 无 TBD/TODO
- [x] 无 "implement later"
- [x] 无 "add appropriate error handling"（具体错误处理已写出）
- [x] 无 "Similar to Task N"
- [x] 每个步骤包含实际代码

### Type Consistency

- [x] `MessageRecord.milky_message_seq: i64` 在所有查询中一致
- [x] `InternalEvent::Message` 字段名在 models/internal.rs 和 services/message.rs 中一致
- [x] `BotRuntimeContext` 在 types.rs、backend.rs、server.rs、runtime.rs 中一致
- [x] `MilkyApiData = serde_json::Value` 在 types.rs 和 backend.rs 中一致

---

**Plan complete and saved to `docs/superpowers/plans/2026-06-09-milky-adapter-infrastructure.md`.**

Next: Write the features plan (Slice C + D + E) for API implementation, message loop, PacketRecorder, and Logs UI.
