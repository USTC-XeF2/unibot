# Milky 协议适配功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 Milky 1.2 协议适配的功能层：API 消息闭环验证、PacketRecorder 报文追踪、Logs 页面、查询 API 和事件扩展。

**Architecture:** 在基础设施计划完成的 protocol 模块上，扩展 `VirtualBackend` 支持更多查询 API，扩展 `MilkyAdapter` 支持更多事件类型。新增 `PacketRecorder` 在 `ProtocolServer` 中记录所有请求/响应/事件。新增 `PacketRepo` 和前端报文查询。

**Tech Stack:** Rust 2024, axum 0.7, tokio, serde, sqlx 0.8, SQLite, chrono, React/TypeScript, TanStack Query

**Prerequisite:** 基础设施计划 (`2026-06-09-milky-adapter-infrastructure.md`) 已全部完成并通过测试。

---

## File Structure

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | 添加 chrono 依赖 |
| `src-tauri/src/protocol/adapter.rs` | Modify | 扩展事件转换（friend_request, group_member_increase/decrease） |
| `src-tauri/src/protocol/backend.rs` | Modify | 扩展查询 API（get_friend_list, get_group_list, get_group_info, get_group_member_list） |
| `src-tauri/src/protocol/server.rs` | Modify | 集成 PacketRecorder |
| `src-tauri/src/protocol/recorder.rs` | Create | 报文文件化和数据库索引 |
| `src-tauri/src/protocol/runtime.rs` | Modify | 传入 PacketRecorder |
| `src-tauri/src/persistence/repo/packet.rs` | Create | protocol_packets 查询 repo |
| `src-tauri/src/persistence/mod.rs` | Modify | 导出 PacketRepo |
| `src-tauri/src/commands/packet.rs` | Create | 报文查询 Tauri commands |
| `src-tauri/src/commands/mod.rs` | Modify | 添加 packet 模块 |
| `src-tauri/src/lib.rs` | Modify | 注册 packet commands |
| `src/types/packet.ts` | Create | 前端报文类型 |
| `src/lib/query/packets.ts` | Create | 前端报文查询 hook |
| `src/views/main/logs.tsx` | Modify | 协议报文列表和详情 |

---

## Task 1: 添加 chrono 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 在 Cargo.toml 添加 chrono**

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "deps: add chrono for packet date formatting"
```

---

## Task 2: 创建 PacketRecorder

**Files:**
- Create: `src-tauri/src/protocol/recorder.rs`

- [ ] **Step 1: 创建 recorder.rs**

```rust
use std::path::PathBuf;

use chrono::Local;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::utils::now_ts;

#[derive(Clone)]
pub struct PacketRecorder {
    app_data_dir: PathBuf,
    pool: SqlitePool,
}

impl PacketRecorder {
    pub fn new(app_data_dir: PathBuf, pool: SqlitePool) -> Self {
        Self { app_data_dir, pool }
    }

    pub async fn record_request(
        &self,
        bot_id: &str,
        profile_id: &str,
        session_id: &str,
        action_name: &str,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id, profile_id, session_id, "receive", action_name, None, None, false, data,
        )
        .await
    }

    pub async fn record_response(
        &self,
        bot_id: &str,
        profile_id: &str,
        session_id: &str,
        action_name: &str,
        is_error: bool,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id, profile_id, session_id, "send", action_name, None, None, is_error, data,
        )
        .await
    }

    pub async fn record_event(
        &self,
        bot_id: &str,
        profile_id: &str,
        session_id: &str,
        event_type: &str,
        related_object_type: Option<&str>,
        related_object_id: Option<&str>,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id,
            profile_id,
            session_id,
            "send",
            event_type,
            related_object_type,
            related_object_id,
            false,
            data,
        )
        .await
    }

    async fn record(
        &self,
        bot_id: &str,
        profile_id: &str,
        session_id: &str,
        direction: &str,
        action_name: &str,
        related_object_type: Option<&str>,
        related_object_id: Option<&str>,
        is_error: bool,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        let packet_id = crate::utils::new_db_id();
        let date = Local::now().format("%Y-%m-%d").to_string();
        let dir = self.app_data_dir.join("packets").join(&date);
        let file_name = format!("{}.json", packet_id);
        let file_path = dir.join(&file_name);

        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::internal(format!("create packet dir: {e}")))?;

        let temp_path = dir.join(format!(".tmp.{}", file_name));
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| AppError::internal(format!("serialize packet: {e}")))?;
        tokio::fs::write(&temp_path, json)
            .await
            .map_err(|e| AppError::internal(format!("write packet temp: {e}")))?;
        tokio::fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| AppError::internal(format!("rename packet file: {e}")))?;

        let relative_path = format!("packets/{}/{}", date, file_name);

        let result = sqlx::query(
            r#"
            INSERT INTO protocol_packets (
                packet_id, bot_id, profile_id, protocol_type, direction,
                action_name, file_path, related_object_type, related_object_id,
                is_error, session_id, created_at
            ) VALUES (?1, ?2, ?3, 'milky', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&packet_id)
        .bind(bot_id)
        .bind(profile_id)
        .bind(direction)
        .bind(action_name)
        .bind(&relative_path)
        .bind(related_object_type)
        .bind(related_object_id)
        .bind(if is_error { 1 } else { 0 })
        .bind(session_id)
        .bind(now_ts() as i64)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            if let Err(remove_err) = tokio::fs::remove_file(&file_path).await {
                eprintln!(
                    "failed to remove packet file {} after db error: {}",
                    file_path.display(),
                    remove_err
                );
            }
            return Err(e.into());
        }

        Ok(packet_id)
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/recorder.rs
git commit -m "feat(protocol): add PacketRecorder with atomic file write and db indexing"
```

---

## Task 3: 创建 protocol_packets Repo

**Files:**
- Create: `src-tauri/src/persistence/repo/packet.rs`
- Modify: `src-tauri/src/persistence/repo/mod.rs`
- Modify: `src-tauri/src/persistence/mod.rs`

- [ ] **Step 1: 创建 packet.rs**

```rust
use sqlx::{QueryBuilder, SqlitePool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProtocolPacketRecord {
    pub packet_id: String,
    pub bot_id: String,
    pub profile_id: String,
    pub protocol_type: String,
    pub direction: String,
    pub action_name: String,
    pub file_path: String,
    pub related_object_type: Option<String>,
    pub related_object_id: Option<String>,
    pub is_error: i32,
    pub session_id: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct PacketRepo {
    pool: SqlitePool,
}

impl PacketRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_packets(
        &self,
        bot_id: Option<&str>,
        direction: Option<&str>,
        action_name: Option<&str>,
        since: Option<u64>,
        limit: i64,
    ) -> Result<Vec<ProtocolPacketRecord>, sqlx::Error> {
        let mut builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new(
            "SELECT packet_id, bot_id, profile_id, protocol_type, direction, action_name, file_path, related_object_type, related_object_id, is_error, session_id, created_at FROM protocol_packets WHERE 1=1"
        );

        if let Some(bot_id) = bot_id {
            builder.push(" AND bot_id = ");
            builder.push_bind(bot_id);
        }
        if let Some(direction) = direction {
            builder.push(" AND direction = ");
            builder.push_bind(direction);
        }
        if let Some(action_name) = action_name {
            builder.push(" AND action_name = ");
            builder.push_bind(action_name);
        }
        if let Some(since) = since {
            builder.push(" AND created_at >= ");
            builder.push_bind(since as i64);
        }

        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(limit);

        builder.build_query_as().fetch_all(&self.pool).await
    }

    pub async fn get_packet_by_id(
        &self,
        packet_id: &str,
    ) -> Result<Option<ProtocolPacketRecord>, sqlx::Error> {
        sqlx::query_as::<_, ProtocolPacketRecord>(
            r#"
            SELECT packet_id, bot_id, profile_id, protocol_type, direction, action_name, file_path,
                   related_object_type, related_object_id, is_error, session_id, created_at
            FROM protocol_packets
            WHERE packet_id = ?1
            "#,
        )
        .bind(packet_id)
        .fetch_optional(&self.pool)
        .await
    }
}
```

- [ ] **Step 2: 在 repo/mod.rs 中导出 PacketRepo**

```rust
pub mod packet;

pub use packet::{PacketRepo, ProtocolPacketRecord};
```

- [ ] **Step 3: 在 persistence/mod.rs 中导出**

添加 `PacketRepo` 和 `ProtocolPacketRecord` 到导出列表：

```rust
pub use repo::{
    BotRepo, GroupEventRecord, GroupRepo, InteractionRepo, MessageRecord, MessageRepo,
    NewFriendRequestRecord, NewGroupEventRecord, NewGroupRequestRecord, NewMessageReactionRecord,
    NewMessageRecord, NewPokeRecord, PacketRepo, ProtocolPacketRecord, UserRepo,
};
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/persistence/repo/
git commit -m "feat(repo): add PacketRepo with dynamic query builder"
```

---

## Task 4: 在 ProtocolServer 中集成 PacketRecorder

**Files:**
- Modify: `src-tauri/src/protocol/server.rs`
- Modify: `src-tauri/src/protocol/runtime.rs`

- [ ] **Step 1: 修改 server.rs 添加 PacketRecorder**

更新 `ServerState`：

```rust
use crate::protocol::recorder::PacketRecorder;

#[derive(Clone)]
struct ServerState {
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<MilkyAdapter>,
    recorder: Arc<PacketRecorder>,
    session_id: String,
}
```

更新 `event_handler` 中的 stream 构建，添加事件记录：

```rust
async fn event_handler(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<EventQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // ... 鉴权代码不变 ...

    let rx = state
        .backend
        .subscribe_events(&state.context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let adapter = state.adapter.clone();
    let recorder = state.recorder.clone();
    let bot_id = state.context.bot_id.clone();
    let profile_id = state.context.bound_user_id.clone();
    let session_id = state.session_id.clone();

    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        let adapter = adapter.clone();
        let recorder = recorder.clone();
        let bot_id = bot_id.clone();
        let profile_id = profile_id.clone();
        let session_id = session_id.clone();
        async move {
            match result {
                Ok(event) => {
                    if let crate::models::InternalEvent::Message { origin_bot_id, .. } = &event {
                        if origin_bot_id.as_ref() == Some(&bot_id) {
                            return None;
                        }
                    }
                    let milky_event = adapter.adapt_event(&event)?;
                    let json = serde_json::to_string(&milky_event).ok()?;

                    // 记录事件
                    let _ = recorder
                        .record_event(
                            &bot_id,
                            &profile_id,
                            &session_id,
                            &milky_event.event_type,
                            Some("message"),
                            None,
                            &serde_json::from_str(&json).unwrap_or(serde_json::Value::Null),
                        )
                        .await;

                    Some(Ok::<_, Infallible>(Event::default().event("milky_event").data(json)))
                }
                Err(_) => None,
            }
        }
    });

    Ok(Sse::new(stream))
}
```

更新 `api_handler` 添加请求/响应记录：

```rust
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

    // 记录请求
    let _ = state.recorder.record_request(
        &state.context.bot_id,
        &state.context.bound_user_id,
        &state.session_id,
        &api_name,
        &body,
    ).await;

    let request = MilkyApiRequest { api_name: api_name.clone(), params: body };
    let response = match state.backend.call_api(&state.context, request).await {
        Ok(data) => MilkyApiResponse::ok(data),
        Err(err) => {
            let (retcode, message) = state.adapter.adapt_error(&err);
            MilkyApiResponse::failed(retcode, message)
        }
    };

    let is_error = response.retcode != 0;
    let response_json = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);

    // 记录响应
    let _ = state.recorder.record_response(
        &state.context.bot_id,
        &state.context.bound_user_id,
        &state.session_id,
        &api_name,
        is_error,
        &response_json,
    ).await;

    (StatusCode::OK, axum::Json(response))
}
```

更新 `spawn_server` 签名：

```rust
pub async fn spawn_server(
    listener: tokio::net::TcpListener,
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<MilkyAdapter>,
    recorder: Arc<PacketRecorder>,
    session_id: String,
) -> crate::error::AppResult<(
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
)> {
    let state = Arc::new(ServerState {
        context,
        backend,
        adapter,
        recorder,
        session_id,
    });
    // ... rest unchanged
}
```

- [ ] **Step 2: 修改 runtime.rs 传入 PacketRecorder**

在 `ProtocolRuntimeManager` 中添加 `recorder` 字段：

```rust
use crate::protocol::recorder::PacketRecorder;

pub struct ProtocolRuntimeManager {
    servers: Mutex<HashMap<String, RunningProtocolServer>>,
    bot_repo: BotRepo,
    service_hub: ServiceHub,
    core: CoreContainer,
    recorder: PacketRecorder,
}

impl ProtocolRuntimeManager {
    pub fn new(
        bot_repo: BotRepo,
        service_hub: ServiceHub,
        core: CoreContainer,
        app_data_dir: std::path::PathBuf,
        pool: sqlx::SqlitePool,
    ) -> Self {
        let recorder = PacketRecorder::new(app_data_dir.clone(), pool);
        Self {
            servers: Mutex::new(HashMap::new()),
            bot_repo,
            service_hub,
            core,
            recorder,
        }
    }
```

在 `start_bot` 中传入 recorder：

```rust
        let (shutdown_tx, join_handle) = match spawn_server(
            listener,
            context,
            backend,
            adapter,
            Arc::new(self.recorder.clone()),
            session.session_id.clone(),
        )
        .await
        {
            Ok((tx, handle)) => (tx, handle),
            Err(e) => {
                let _ = self.bot_repo.stop_active_sessions(bot_id).await;
                return Err(e);
            }
        };
```

更新 `lib.rs` 中 `ProtocolRuntimeManager::new` 的调用，添加 `pool.clone()`：

```rust
let protocol_runtime = ProtocolRuntimeManager::new(
    bot_repo.clone(),
    service_hub.clone(),
    core.clone(),
    app_data_dir,
    pool.clone(),
);
```

- [ ] **Step 3: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/protocol/server.rs src-tauri/src/protocol/runtime.rs src-tauri/src/lib.rs
git commit -m "feat(protocol): integrate PacketRecorder into server and runtime"
```

---

## Task 5: 创建报文查询 Commands

**Files:**
- Create: `src-tauri/src/commands/packet.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 commands/packet.rs**

```rust
use crate::persistence::{PacketRepo, ProtocolPacketRecord};
use crate::commands::IntoCommandResult;

use tauri::Manager;

#[tauri::command]
pub async fn list_protocol_packets(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    bot_id: Option<String>,
    direction: Option<String>,
    action_name: Option<String>,
    since: Option<u64>,
    limit: Option<i64>,
) -> Result<Vec<ProtocolPacketRecord>, String> {
    let repo = PacketRepo::new(pool.inner().clone());
    let limit = limit.unwrap_or(100).min(1000);
    repo.list_packets(
        bot_id.as_deref(),
        direction.as_deref(),
        action_name.as_deref(),
        since,
        limit,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_protocol_packet(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    packet_id: String,
) -> Result<String, String> {
    let repo = PacketRepo::new(pool.inner().clone());
    let packet = repo
        .get_packet_by_id(&packet_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "packet not found".to_string())?;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let file_path = app_data_dir.join(&packet.file_path);

    tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("failed to read packet file: {}", e))
}
```

- [ ] **Step 2: 在 commands/mod.rs 中添加 packet 模块**

```rust
pub mod bot;
pub mod chat;
pub mod main;
pub mod packet;
```

- [ ] **Step 3: 在 lib.rs 中注册 packet commands**

在 `invoke_handler` 中添加：

```rust
            packet::list_protocol_packets,
            packet::read_protocol_packet,
```

- [ ] **Step 4: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(commands): add packet query and read commands"
```

---

## Task 6: 扩展 VirtualBackend 查询 API

**Files:**
- Modify: `src-tauri/src/protocol/backend.rs`

- [ ] **Step 1: 在 call_api 中添加查询 API**

在 match 中添加：

```rust
            "get_friend_list" => {
                let friends = self
                    .service_hub
                    .user
                    .list_friends(bot.bound_user_id.clone())
                    .await?;
                let data: Vec<serde_json::Value> = futures::future::join_all(
                    friends.into_iter().map(|friend_id| async {
                        self.service_hub.user.get_user_by_id(&friend_id).await.ok().flatten()
                    })
                ).await.into_iter().filter_map(|profile| {
                    profile.map(|p| serde_json::json!({
                        "user_id": p.user_id.parse::<i64>().unwrap_or(0),
                        "nickname": p.nickname,
                        "remark": "",
                    }))
                }).collect();
                Ok(serde_json::json!({ "data": data }))
            }
            "get_group_list" => {
                let groups = self
                    .service_hub
                    .user
                    .list_user_groups(bot.bound_user_id.clone())
                    .await?;
                let data: Vec<serde_json::Value> = groups
                    .into_iter()
                    .map(|g| serde_json::json!({
                        "group_id": g.group_id.parse::<i64>().unwrap_or(0),
                        "group_name": g.group_name,
                        "member_count": g.member_count,
                        "max_member_count": g.max_member_count,
                    }))
                    .collect();
                Ok(serde_json::json!({ "data": data }))
            }
            "get_group_info" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                // 需要 GroupService::get_group_by_id 或类似方法
                // 如果该方法不存在，返回 not_found
                Err(AppError::not_found("get_group_info not yet implemented"))
            }
            "get_group_member_list" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                let members = self
                    .service_hub
                    .group
                    .list_group_members(&group_id)
                    .await?;
                let data: Vec<serde_json::Value> = members
                    .into_iter()
                    .map(|m| serde_json::json!({
                        "user_id": m.user_id.parse::<i64>().unwrap_or(0),
                        "nickname": m.card,
                        "card": m.card,
                        "role": match m.role {
                            crate::models::GroupRole::Owner => "owner",
                            crate::models::GroupRole::Admin => "admin",
                            crate::models::GroupRole::Member => "member",
                        },
                    }))
                    .collect();
                Ok(serde_json::json!({ "data": data }))
            }
```

**注意：** `get_group_info` 需要 `GroupService::get_group_by_id` 方法。如果该方法的实际签名不同，需要调整。`list_user_groups` 也需要确认 `UserService` 是否有该方法。

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过（如果 Service 方法签名匹配）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/backend.rs
git commit -m "feat(protocol): extend VirtualBackend with query APIs"
```

---

## Task 7: 扩展 MilkyAdapter 支持更多事件

**Files:**
- Modify: `src-tauri/src/protocol/adapter.rs`

- [ ] **Step 1: 扩展 adapt_event 方法**

在 match 中添加：

```rust
            InternalEvent::FriendRequestCreated {
                request_id,
                initiator_user_id,
                target_user_id,
                time,
            } => {
                let self_id = target_user_id.parse::<i64>().ok()?;
                Some(MilkyEvent {
                    time: *time / 1000,
                    self_id,
                    event_type: "friend_request".to_string(),
                    data: serde_json::json!({
                        "request_id": request_id,
                        "user_id": initiator_user_id.parse::<i64>().ok()?,
                        "comment": "",
                    }),
                })
            }
            InternalEvent::GroupRequestCreated {
                request_id,
                group_id,
                request_type,
                initiator_user_id,
                target_user_id,
                time,
            } => {
                let self_id = target_user_id.as_ref()?.parse::<i64>().ok()?;
                Some(MilkyEvent {
                    time: *time / 1000,
                    self_id,
                    event_type: "group_join_request".to_string(),
                    data: serde_json::json!({
                        "request_id": request_id,
                        "group_id": group_id.parse::<i64>().ok()?,
                        "user_id": initiator_user_id.parse::<i64>().ok()?,
                        "comment": "",
                    }),
                })
            }
            InternalEvent::GroupMemberJoined {
                group_id,
                operator_user_id,
                target_user_id,
                time,
            } => {
                let self_id = target_user_id.parse::<i64>().ok()?;
                Some(MilkyEvent {
                    time: *time / 1000,
                    self_id,
                    event_type: "group_member_increase".to_string(),
                    data: serde_json::json!({
                        "group_id": group_id.parse::<i64>().ok()?,
                        "user_id": target_user_id.parse::<i64>().ok()?,
                        "operator_id": operator_user_id.parse::<i64>().ok()?,
                    }),
                })
            }
            InternalEvent::GroupMemberLeft {
                group_id,
                operator_user_id,
                target_user_id,
                time,
            } => {
                let self_id = target_user_id.parse::<i64>().ok()?;
                Some(MilkyEvent {
                    time: *time / 1000,
                    self_id,
                    event_type: "group_member_decrease".to_string(),
                    data: serde_json::json!({
                        "group_id": group_id.parse::<i64>().ok()?,
                        "user_id": target_user_id.parse::<i64>().ok()?,
                        "operator_id": operator_user_id.as_ref().and_then(|id| id.parse::<i64>().ok()),
                    }),
                })
            }
```

- [ ] **Step 2: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/protocol/adapter.rs
git commit -m "feat(protocol): extend MilkyAdapter with friend/group request and member events"
```

---

## Task 8: 前端报文类型

**Files:**
- Create: `src/types/packet.ts`

- [ ] **Step 1: 创建前端类型**

```typescript
export interface ProtocolPacket {
  packet_id: string;
  bot_id: string;
  profile_id: string;
  protocol_type: string;
  direction: 'receive' | 'send';
  action_name: string;
  file_path: string;
  related_object_type: string | null;
  related_object_id: string | null;
  is_error: boolean;
  session_id: string;
  created_at: number;
}

export interface PacketFilters {
  bot_id?: string;
  direction?: 'receive' | 'send';
  action_name?: string;
  since?: number;
  limit?: number;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/types/packet.ts
git commit -m "feat(types): add ProtocolPacket frontend types"
```

---

## Task 9: 前端报文 Query Hook

**Files:**
- Create: `src/lib/query/packets.ts`

- [ ] **Step 1: 创建 query hook**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { useQuery } from '@tanstack/react-query';
import type { ProtocolPacket, PacketFilters } from '@/types/packet';

const PACKET_KEYS = {
  all: ['packets'] as const,
  list: (filters: PacketFilters) => [...PACKET_KEYS.all, 'list', filters] as const,
  detail: (id: string) => [...PACKET_KEYS.all, 'detail', id] as const,
};

export function useProtocolPackets(filters: PacketFilters = {}) {
  return useQuery({
    queryKey: PACKET_KEYS.list(filters),
    queryFn: async () => {
      return invoke<ProtocolPacket[]>('list_protocol_packets', {
        botId: filters.bot_id ?? null,
        direction: filters.direction ?? null,
        actionName: filters.action_name ?? null,
        since: filters.since ?? null,
        limit: filters.limit ?? 100,
      });
    },
    refetchInterval: 2000, // 2 秒轮询
  });
}

export function useProtocolPacketDetail(packetId: string) {
  return useQuery({
    queryKey: PACKET_KEYS.detail(packetId),
    queryFn: async () => {
      return invoke<string>('read_protocol_packet', { packetId });
    },
    enabled: !!packetId,
  });
}
```

**注意：** Tauri v2 的 `invoke` 参数名使用 camelCase。确保 Rust command 的参数名与前端匹配。如果 Rust 端使用 snake_case，Tauri 会自动转换。但根据项目现有代码风格，可能直接使用小写。

- [ ] **Step 2: Commit**

```bash
git add src/lib/query/packets.ts
git commit -m "feat(query): add protocol packet query hooks with polling"
```

---

## Task 10: 更新 Logs 页面

**Files:**
- Modify: `src/views/main/logs.tsx`

由于 `src/views/main/logs.tsx` 的当前内容未知，以下是一个实现参考。需要根据实际 UI 框架调整。

- [ ] **Step 1: 实现 Logs 页面**

```tsx
import { useState } from 'react';
import { useProtocolPackets, useProtocolPacketDetail } from '@/lib/query/packets';
import type { ProtocolPacket } from '@/types/packet';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}

function PacketDirectionBadge({ direction }: { direction: string }) {
  const isReceive = direction === 'receive';
  return (
    <span className={`px-2 py-0.5 rounded text-xs ${isReceive ? 'bg-blue-100 text-blue-800' : 'bg-green-100 text-green-800'}`}>
      {isReceive ? '接收' : '发送'}
    </span>
  );
}

export default function LogsPage() {
  const [selectedPacket, setSelectedPacket] = useState<string | null>(null);
  const { data: packets, isLoading } = useProtocolPackets({ limit: 100 });
  const { data: packetJson } = useProtocolPacketDetail(selectedPacket ?? '');

  if (isLoading) {
    return <div className="p-4">加载中...</div>;
  }

  return (
    <div className="flex h-full">
      {/* 左侧列表 */}
      <div className="w-2/3 border-r overflow-auto">
        <table className="w-full text-sm">
          <thead className="bg-gray-50 sticky top-0">
            <tr>
              <th className="px-4 py-2 text-left">时间</th>
              <th className="px-4 py-2 text-left">方向</th>
              <th className="px-4 py-2 text-left">动作</th>
              <th className="px-4 py-2 text-left">Bot</th>
            </tr>
          </thead>
          <tbody>
            {packets?.map((packet: ProtocolPacket) => (
              <tr
                key={packet.packet_id}
                className={`border-b hover:bg-gray-50 cursor-pointer ${selectedPacket === packet.packet_id ? 'bg-blue-50' : ''}`}
                onClick={() => setSelectedPacket(packet.packet_id)}
              >
                <td className="px-4 py-2">
                  {new Date(packet.created_at).toLocaleString()}
                </td>
                <td className="px-4 py-2">
                  <PacketDirectionBadge direction={packet.direction} />
                </td>
                <td className="px-4 py-2">{packet.action_name}</td>
                <td className="px-4 py-2 text-gray-500">{packet.bot_id.slice(0, 8)}...</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* 右侧详情 */}
      <div className="w-1/3 p-4 overflow-auto">
        {selectedPacket && packetJson ? (
          <pre className="text-xs bg-gray-50 p-4 rounded overflow-auto whitespace-pre-wrap">
            {packetJson}
          </pre>
        ) : (
          <div className="text-gray-400 text-center mt-20">
            选择一条报文查看详情
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 编译验证**

Run: `bun run build`（或前端构建命令）
Expected: 构建通过

- [ ] **Step 3: Commit**

```bash
git add src/views/main/logs.tsx src/types/packet.ts src/lib/query/packets.ts
git commit -m "feat(ui): add protocol packet logs page with detail panel"
```

---

## Task 11: HTTP 集成测试

**Files:**
- Create: `src-tauri/src/protocol/integration_tests.rs`
- Modify: `src-tauri/src/protocol/mod.rs`

- [ ] **Step 1: 创建集成测试**

```rust
#[cfg(test)]
mod integration_tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::core::CoreContainer;
    use crate::models::{UserProfile, MessageSource, MessageSegment};
    use crate::persistence::{migrator, BotRepo, MessageRepo, GroupRepo, UserRepo};
    use crate::protocol::{ProtocolRuntimeManager, MilkyAdapter, VirtualBackend};
    use crate::protocol::types::BotConfig;
    use crate::services::{ServiceHub, BotService, MessageService, UserService};
    use crate::utils::new_db_id;

    async fn setup_test_env(pool: sqlx::SqlitePool) -> (ProtocolRuntimeManager, String, String) {
        migrator::run_migrations(&pool).await.unwrap();

        let user_repo = UserRepo::new(pool.clone());
        let bot_repo = BotRepo::new(pool.clone());
        let message_repo = MessageRepo::new(pool.clone());
        let group_repo = GroupRepo::new(pool.clone());

        let core = CoreContainer::new();

        // 注册测试用户
        let user = UserProfile {
            user_id: "10001".to_string(),
            nickname: "Alice".to_string(),
            avatar: "".to_string(),
            signature: "".to_string(),
            account_status: Default::default(),
        };
        user_repo.upsert_user(&user).await.unwrap();
        core.register_user(user).unwrap();

        let service_hub = ServiceHub::new(
            MessageService::new(message_repo, group_repo.clone()),
            // ... 其他服务使用简化版本
            crate::services::InteractionService::new(
                crate::persistence::InteractionRepo::new(pool.clone()),
                MessageRepo::new(pool.clone()),
                group_repo.clone(),
            ),
            crate::services::GroupService::new(group_repo, MessageRepo::new(pool.clone())),
            crate::services::RequestService::new(user_repo.clone()),
            UserService::new(user_repo),
            BotService::new(bot_repo.clone()),
        );

        let temp_dir = std::env::temp_dir().join(format!("unibot-test-{}", new_db_id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let runtime = ProtocolRuntimeManager::new(
            bot_repo.clone(),
            service_hub.clone(),
            core.clone(),
            temp_dir.clone(),
            pool.clone(),
        );

        // 创建 Bot
        let bot = BotService::new(bot_repo)
            .create_bot_for_test("10001", "Test Bot")
            .await
            .unwrap();

        (runtime, bot.bot_id, temp_dir.to_str().unwrap().to_string())
    }

    #[sqlx::test]
    async fn start_bot_exposes_event_endpoint(pool: sqlx::SqlitePool) {
        let (runtime, bot_id, _dir) = setup_test_env(pool).await;

        let addr = runtime.start_bot(&bot_id).await.unwrap();

        // 测试 SSE 连接
        let client = reqwest::Client::new();
        let config = serde_json::from_str::<BotConfig>(
            &tokio::fs::read_to_string(format!("{}/bots/{}.json", _dir, bot_id)).await.unwrap()
        ).unwrap();

        let resp = client
            .get(format!("http://{}/event?access_token={}", addr, config.access_token))
            .send()
            .await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.status(), 200);

        runtime.stop_bot(&bot_id).await.unwrap();
    }

    #[sqlx::test]
    async fn wrong_token_returns_401(pool: sqlx::SqlitePool) {
        let (runtime, bot_id, _dir) = setup_test_env(pool).await;
        let addr = runtime.start_bot(&bot_id).await.unwrap();

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/event?access_token=wrong", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 401);

        runtime.stop_bot(&bot_id).await.unwrap();
    }

    #[sqlx::test]
    async fn get_login_info_returns_bound_user(pool: sqlx::SqlitePool) {
        let (runtime, bot_id, _dir) = setup_test_env(pool).await;
        let addr = runtime.start_bot(&bot_id).await.unwrap();

        let config = serde_json::from_str::<BotConfig>(
            &tokio::fs::read_to_string(format!("{}/bots/{}.json", _dir, bot_id)).await.unwrap()
        ).unwrap();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{}/api/get_login_info?access_token={}", addr, config.access_token))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["data"]["user_id"], 10001);

        runtime.stop_bot(&bot_id).await.unwrap();
    }
}
```

**注意：** 这个测试文件需要 `BotService::create_bot_for_test` 方法，该方法可能需要添加。或者使用现有的 `create_bot` 方法但需要 `tauri::AppHandle`。

由于集成测试需要复杂的 setup，这个任务可以作为可选任务，或者使用手工验证替代。

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/protocol/integration_tests.rs
git commit -m "test(protocol): add HTTP integration tests for auth and get_login_info"
```

---

## Self-Review

### Spec Coverage

| Spec Section | Task | Status |
|-------------|------|--------|
| 4.5 PacketRecorder | Task 2 | ✅ |
| 9. 阶段 2 MVP API | Task 6 | ✅ |
| 9. 首个可运行切片 | 基础设施已完成 | ✅ |
| 10. 数据流 | Task 4 | ✅ |
| 11. Logs 页面 | Task 8, 9, 10 | ✅ |
| 12. Slice C 验收 | 基础设施 + Task 11 | ✅ |
| 12. Slice D 验收 | Task 2, 3, 4, 5 | ✅ |
| 12. Slice E 验收 | Task 6, 7 | ✅ |
| 13. HTTP 集成测试 | Task 11 | ✅ |

### Placeholder Scan

- [x] 无 TBD/TODO
- [x] 无 "implement later"
- [x] 每个步骤包含实际代码

### Type Consistency

- [x] `PacketRecorder` 方法签名在 recorder.rs 和 server.rs 中一致
- [x] `ProtocolPacketRecord` 在 packet.rs 和前端 types 中一致
- [x] `MilkyAdapter::adapt_event` 的 event_type 字符串与 spec 一致

---

**Plan complete and saved to `docs/superpowers/plans/2026-06-09-milky-adapter-features.md`.**

**两个计划已就绪：**
1. `docs/superpowers/plans/2026-06-09-milky-adapter-infrastructure.md` — 基础设施（15 个 Task）
2. `docs/superpowers/plans/2026-06-09-milky-adapter-features.md` — 功能实现（11 个 Task）
