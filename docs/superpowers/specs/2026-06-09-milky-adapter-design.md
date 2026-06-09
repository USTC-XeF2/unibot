# UniBot Milky 1.2 协议适配与协议追踪设计

> 本文档定义阶段 2 的虚拟 Milky 协议端，并为阶段 3 的真实协议代理预留稳定扩展点。
>
> Milky 协议基线固定为 **Milky 1.2**。协议名称、请求参数、响应结构和事件结构均以官方文档为准，不混用 OneBot 11 命名。

---

## 1. 目标与边界

### 1.1 阶段 2 目标

1. 每个运行中的 Bot 提供一个独立的 Milky HTTP 服务端口。
2. Bot 框架通过 Milky API 操作 UniBot 虚拟 IM。
3. 绑定用户收到的虚拟 IM 事件通过 Milky 1.2 SSE 推送给 Bot 框架。
4. API 请求、API 响应和事件推送写入 `protocol_packets` 表及文件系统。
5. Logs 页面能够查询并持续刷新协议报文。
6. Bot 的启动、停止、删除和应用退出能够正确管理协议服务生命周期。

### 1.2 阶段 2 MVP 不包含

- WebSocket 和 WebHook 事件推送。
- Milky 全量 API 和全量事件。
- 真实 QQ 协议连接。
- 上游 Milky 代理。
- 多协议同时运行。
- 公网监听和 TLS。

MVP 只监听 `127.0.0.1`，只实现 SSE，以压缩首个可验证切片的范围。

---

## 2. 协议约束

### 2.1 HTTP 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/event` | Milky SSE 事件流 |
| `POST` | `/api/:api` | Milky API 调用 |

所有请求都检查以下任一种凭据：

- `Authorization: Bearer {access_token}`
- Query 参数 `access_token={access_token}`，仅供无法设置 Header 的 SSE 客户端使用

鉴权失败返回 HTTP `401`。未知 API 返回 HTTP `404`。不支持的 Content-Type 返回 HTTP `415`。API 业务失败仍返回 HTTP `200`，并使用 Milky 失败响应。

### 2.2 标准响应

```json
{
  "status": "ok",
  "retcode": 0,
  "data": {}
}
```

```json
{
  "status": "failed",
  "retcode": -400,
  "message": "invalid request"
}
```

### 2.3 SSE 格式

```text
event: milky_event
data: {"time":1710000000,"self_id":10001,"event_type":"message_receive","data":{}}

```

`self_id` 是 Bot 绑定用户的数字 QQ ID，即 `bots.bound_user_id`，不是内部 UUID `bot_id`。

Milky 的 `time` 字段使用 Unix 秒。UniBot 内部实体和数据库继续使用毫秒；Adapter 在协议边界统一执行毫秒到秒的转换。

---

## 3. 总体架构

阶段 2 采用五个边界清晰的模块：

| 模块 | 职责 |
|------|------|
| `ProtocolRuntimeManager` | 管理所有 Bot 协议服务实例及生命周期 |
| `ProtocolServer` | 提供鉴权、API 路由和 SSE 连接 |
| `ProtocolBackend` | 提供事件订阅和 API 执行能力 |
| `MilkyAdapter` | Milky 类型、内部类型和错误之间的转换 |
| `PacketRecorder` | 报文文件及索引记录 |

```text
Bot Framework
    | POST /api/:api             ^ GET /event (SSE)
    v                            |
ProtocolServer ------------------+
    |
    v
MilkyAdapter
    |
    v
ProtocolBackend
    |
    +---- VirtualBackend ---- Virtual IM services + bound user's event bus
    |
    `---- ProxyBackend ------ upstream Milky endpoint (stage 3)

ProtocolServer / MilkyAdapter
    |
    `---- PacketRecorder ---- JSON file + protocol_packets
```

阶段 3 的差异不只在事件来源。事件订阅和 API 执行都必须由同一个 Backend 抽象承载。

---

## 4. 模块设计

### 4.1 ProtocolRuntimeManager

**文件：** `src-tauri/src/protocol/runtime.rs`

```rust
pub struct ProtocolRuntimeManager {
    servers: tokio::sync::Mutex<HashMap<String, RunningProtocolServer>>,
}

pub struct RunningProtocolServer {
    pub bot_id: String,
    pub session_id: String,
    pub bound_addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}
```

职责：

1. 保证同一 Bot 最多运行一个协议服务。
2. 启动时占用监听端口并注册运行实例。
3. 停止时发送 graceful shutdown，并等待任务结束。
4. 删除 Bot 前停止对应服务。
5. 应用退出时停止全部服务。
6. 启动流程任一步失败时回滚已创建的 listener、任务和 debug session。

`ProtocolRuntimeManager` 由 Tauri `manage` 持有。`BotService` 不直接保存 `JoinHandle`。

### 4.2 ProtocolServer

**文件：** `src-tauri/src/protocol/server.rs`

每个 Bot 对应一个 axum server。Server 只负责传输层：

- 解析鉴权信息。
- 解析 API 名称和 JSON body。
- 将 API 调用交给 `ProtocolBackend`。
- 将 Backend 事件交给 `MilkyAdapter` 后通过 SSE 广播。
- 调用 `PacketRecorder` 记录请求、响应和事件。
- 将内部错误转换为 Milky HTTP/JSON 响应。

Server 不直接访问 SQLite，也不直接调用具体 Service。

### 4.3 ProtocolBackend

**文件：** `src-tauri/src/protocol/backend.rs`

```rust
#[async_trait::async_trait]
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
```

阶段 2 使用 `VirtualBackend`：

- 订阅 `bound_user_id` 对应的 `UserContext` 事件总线。
- 通过现有 `ServiceHub` 操作虚拟 IM。
- 不绕过 Service 层直接写 SQLite。

阶段 3 使用 `ProxyBackend`：

- 订阅上游 Milky SSE/WebSocket。
- 将 API 请求转发给上游。
- Server、MilkyAdapter 和 PacketRecorder 保持不变。

### 4.4 MilkyAdapter

**文件：**

- `src-tauri/src/protocol/adapter.rs`
- `src-tauri/src/protocol/types.rs`

职责：

- Milky 1.2 请求 JSON -> 强类型请求。
- 内部实体 -> Milky API 返回实体。
- `InternalEvent` -> Milky Event。
- `MessageSegment` <-> Milky Incoming/Outgoing Segment。
- `AppError` -> Milky `retcode` 和 `message`。

Milky 类型使用 serde 的 tagged enum 或明确 DTO，禁止通过动态 JSON 字符串拼接协议。

### 4.5 PacketRecorder

**文件：**

- `src-tauri/src/protocol/recorder.rs`
- `src-tauri/src/persistence/repo/packet.rs`

写入顺序：

1. 生成 `packet_id = new_db_id()`。
2. 创建日期目录。
3. 将完整 JSON 原子写入临时文件。
4. rename 为最终 `{packet_id}.json`。
5. 插入 `protocol_packets` 索引记录。
6. 数据库插入失败时删除最终文件；删除失败记录警告。

文件路径：

```text
{app_data_dir}/packets/YYYY-MM-DD/{packet_id}.json
```

数据库只保存相对路径，例如：

```text
packets/2026-06-09/{packet_id}.json
```

字段映射：

| 字段 | 来源 |
|------|------|
| `packet_id` | UUID v7 |
| `bot_id` | 内部 Bot UUID |
| `profile_id` | `bound_user_id` |
| `protocol_type` | `"milky"` |
| `direction` | `"receive"`：框架 -> UniBot；`"send"`：UniBot -> 框架 |
| `action_name` | API 名或 Milky `event_type` |
| `file_path` | 相对 app data dir 的 JSON 路径 |
| `related_object_type` | `message`、`friend_request`、`group_request` 等 |
| `related_object_id` | 对应内部实体 ID |
| `is_error` | API 失败响应为 `1`，其余为 `0` |
| `session_id` | 当前 debug session |
| `created_at` | 毫秒 Unix 时间戳 |

当前 schema 已包含所需字段，不新增不存在的 `source` 或 `config_json` 字段。

---

## 5. Bot 配置和端口

`bots.config_path` 指向：

```json
{
  "version": 1,
  "protocol": "milky",
  "http": {
    "host": "127.0.0.1",
    "port": 3001
  },
  "access_token": "random-generated-token",
  "event_transport": "sse"
}
```

规则：

1. 创建 Bot 时写入完整默认配置，不再写入 `{}`。
2. `bound_user_id` 必须可解析为正数 Milky `int64 self_id`。
3. `access_token` 创建时随机生成，不使用固定示例 token。
4. 从应用保留范围中为 Bot 分配稳定端口，默认从 `3001` 开始。
5. 端口写入配置后保持稳定。
6. 启动时端口已被外部进程占用则返回明确冲突，不静默改写端口。
7. 配置解析失败不改变 Bot 状态，也不创建 debug session。

---

## 6. 内部事件契约

当前 `InternalEvent::Message` 信息不足，阶段 2 开始前需改为携带已经持久化的消息身份和完整来源：

```rust
InternalEvent::Message {
    message_id: DbId,
    message_seq: i64,
    sender_user_id: DbId,
    source: MessageSource,
    content: Vec<MessageSegment>,
    origin_bot_id: Option<DbId>,
    time: u64,
}
```

其他事件必须携带转换 Milky Event 所需的数据，不能由 Adapter 猜测。允许 Adapter 按 ID 查询补充展示实体，但事件类型、操作者、目标对象和协议序列必须在事件中明确。

新增：

```rust
InternalEvent::GroupMemberLeft {
    group_id: DbId,
    operator_user_id: Option<DbId>,
    target_user_id: DbId,
    time: u64,
}
```

### 6.1 消息序列

Milky 的 `message_seq` 是 `int64`，当前 UUID 字符串不能直接暴露。

为 `messages` 增加独立的 `milky_message_seq INTEGER`，并创建唯一索引：

- 增量迁移先添加可空列，再按 `created_at, message_id` 为历史消息稳定回填。
- 回填完成后创建 `UNIQUE INDEX`。
- SQLite 的 `ALTER TABLE ADD COLUMN` 限制使该列无法直接增加 `NOT NULL` 约束；迁移测试和 repo 写入路径保证所有记录非空。
- 新消息在数据库写事务中分配全局单调递增序列。
- 内部 UUID `message_id` 继续作为数据库主键。
- Milky API 和事件只暴露数字 `milky_message_seq`。
- 通过 repo 方法完成 UUID 与 Milky sequence 的双向查询。

该 schema 改动必须新增增量迁移，不修改已经发布的迁移来模拟升级。

### 6.2 事件路由和 Echo

Bot 订阅绑定用户的事件总线，而不是按内部 `bot_id` 新建平行总线。

事件路由规则：

1. 只向事件接收者对应的 Bot 推送。
2. API 发送消息时设置 `origin_bot_id = Some(current_bot_id)`。
3. 当前 Bot 默认不接收自己 API 操作产生的 `message_receive` Echo，避免框架循环。
4. 同一事件仍可推送给其他接收用户绑定的 Bot。

---

## 7. Bot 生命周期

### 7.1 启动

```text
读取 Bot 和配置
  -> 校验 bound_user_id、token、host、port
  -> RuntimeManager 为 bot_id 建立启动占位
  -> bind TCP listener
  -> repo.start_session 事务设置 running 并创建 session
  -> 创建 VirtualBackend 和 PacketRecorder
  -> spawn ProtocolServer
  -> 将 RunningProtocolServer 写入运行表
```

失败补偿：

- bind 失败：释放启动占位，不改变数据库。
- session 创建失败：释放 listener。
- spawn/注册失败：关闭 server，并结束 session。

### 7.2 停止

```text
RuntimeManager 移除运行实例
  -> graceful shutdown
  -> 等待 server task
  -> repo.stop_active_sessions 事务设置 stopped
```

重复停止返回“Bot 未运行”，不创建新 session。

### 7.3 删除

```text
先停止 RuntimeManager 中的服务
  -> repo.delete_bot_with_sessions
  -> best-effort 删除配置文件
```

数据库删除是主操作；配置清理失败记录警告，不恢复已删除的 Bot。

### 7.4 应用退出

应用退出钩子调用 `shutdown_all()`，停止所有 listener，并结束仍活跃的 debug session。

---

## 8. 阶段 2 MVP API

Milky API 名称严格使用官方名称：

| API | 请求核心字段 | 虚拟 IM 操作 |
|-----|-------------|-------------|
| `get_login_info` | `{}` | 返回绑定用户的 `uin` 和 `nickname` |
| `get_friend_list` | `{}` | 查询绑定用户好友及完整资料 |
| `get_group_list` | `{ no_cache? }` | 查询绑定用户加入的群 |
| `get_group_info` | `{ group_id, no_cache? }` | 查询单群资料 |
| `get_group_member_list` | `{ group_id, no_cache? }` | 查询群成员 |
| `send_private_message` | `{ user_id, message }` | 绑定用户发送私聊消息 |
| `send_group_message` | `{ group_id, message }` | 绑定用户发送群消息 |

不提供非 Milky 的 `send_msg` 聚合 API。

首个可运行切片只实现：

1. `get_login_info`
2. `send_private_message`
3. `send_group_message`

其余查询 API 在第二个 API 切片补齐。

---

## 9. 阶段 2 MVP 事件

| Milky `event_type` | 触发时机 | 内部事件 |
|--------------------|---------|---------|
| `message_receive` | 绑定用户收到私聊或群消息 | `InternalEvent::Message` |
| `friend_request` | 绑定用户收到好友请求 | `FriendRequestCreated` |
| `group_join_request` | 绑定用户作为管理员收到入群申请 | `GroupRequestCreated` |
| `group_member_increase` | 绑定用户所在群新增成员 | `GroupMemberJoined` |
| `group_member_decrease` | 绑定用户所在群成员退出或被踢 | `GroupMemberLeft` |

首个事件切片只实现 `message_receive`。其余事件在消息链路稳定后逐项加入。

---

## 10. 数据流

### 10.1 虚拟 IM -> Bot 框架

```text
MessageService::send
  -> 事务持久化消息和 milky_message_seq
  -> emit InternalEvent::Message
  -> bound user's VirtualBackend receiver
  -> 过滤 origin_bot_id
  -> MilkyAdapter -> message_receive
  -> PacketRecorder 记录 send event
  -> ProtocolServer SSE 推送
```

### 10.2 Bot 框架 -> 虚拟 IM

```text
POST /api/send_group_message
  -> 鉴权和 JSON 解析
  -> PacketRecorder 记录 receive request
  -> VirtualBackend 调用 MessageService::send
       bot_id = Some(current_bot_id)
  -> MilkyAdapter 生成 API response
  -> PacketRecorder 记录 send response
  -> HTTP 200 response
```

当前 Bot 不接收该消息的 Echo；其他接收用户绑定的 Bot 可以收到对应 `message_receive`。

---

## 11. Logs 页面

后端增加：

- `list_protocol_packets(bot_id?, direction?, action_name?, since?, limit?)`
- `read_protocol_packet(packet_id)`

前端增加：

- `src/types/packet.ts`
- `src/lib/query/packets.ts`
- Logs 表格数据源及过滤条件
- 点击一条报文打开 JSON 详情

实时性的 MVP 定义为仅在 Logs 页面可见时每 2 秒刷新。协议服务不直接依赖 Tauri window emitter。后续如果轮询成本明显，再增加独立实时通知。

---

## 12. 实现切片与验收

### Slice A：协议基础类型和消息序列

- Milky 1.2 DTO 和 serde fixture。
- `InternalEvent::Message` 契约升级。
- `milky_message_seq` 增量迁移、回填和 repo 查询。
- 消息段双向转换。

验收：

- 官方示例 JSON 可反序列化并原样语义往返。
- 并发插入消息不会产生重复 sequence。
- 旧数据库升级后历史消息 sequence 非空且稳定。

### Slice B：运行时、配置、鉴权和 SSE

- 默认 Bot 配置生成。
- `ProtocolRuntimeManager`。
- 单 Bot axum server。
- Bearer/query token 鉴权。
- SSE 连接和 graceful shutdown。

验收：

- 启动 Bot 后 `/event` 可建立连接。
- 错误 token 返回 `401`。
- 同一 Bot 不能重复启动。
- 停止、删除和退出后端口释放。
- 端口冲突不改变 Bot 运行状态。

### Slice C：首个 API 与消息闭环

- `get_login_info`。
- `send_private_message`。
- `send_group_message`。
- `message_receive`。
- 同源 Echo 过滤。

验收：

- NoneBot Milky adapter 或协议 fixture client 可调用三个 API。
- UI 消息能够通过 SSE 到达 Bot 框架。
- Bot API 发出的消息出现在虚拟 IM 中。
- 当前 Bot 不收到自己的 API Echo。

### Slice D：PacketRecorder 和 Logs

- 文件原子写入和数据库索引。
- 请求、响应和事件三类记录。
- packet 查询命令。
- Logs 列表和 JSON 详情。

验收：

- 每次 API 调用有 request 和 response 记录。
- 每次 SSE 事件有 send 记录。
- 文件或数据库写入失败不会留下不可定位的半成品。
- Logs 可按 Bot、方向、动作和时间查询。

### Slice E：查询 API 和其余事件

- 好友、群和群成员查询 API。
- 好友请求、群请求、成员增减事件。

每个事件单独增加转换 fixture 和端到端测试。

---

## 13. 测试策略

### 单元测试

- Milky DTO serde。
- MessageSegment 转换。
- InternalEvent -> Milky Event。
- AppError -> Milky response。
- 配置校验。
- PacketRecorder 补偿删除。

### Repo 和迁移测试

- 从上一 schema version 升级。
- 历史消息 sequence 回填。
- sequence 并发唯一性。
- protocol_packets 插入、过滤和分页。

### HTTP 集成测试

- 鉴权。
- 未知 API 和错误 Content-Type。
- `get_login_info`。
- 私聊和群聊发送。
- SSE 连接、事件格式和断开。
- graceful shutdown 和端口释放。

### 手工兼容验证

使用真实 Milky 客户端适配器连接本地服务：

1. 使用 token 建立 SSE。
2. 调用 `get_login_info`。
3. 调用私聊和群聊发送 API。
4. 在 UniBot UI 发送消息并观察 `message_receive`。
5. 在 Logs 页面核对 request、response 和 event。

---

## 14. 关键文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `src-tauri/src/protocol/mod.rs` | 创建 | 协议模块入口 |
| `src-tauri/src/protocol/types.rs` | 创建 | Milky 1.2 DTO |
| `src-tauri/src/protocol/adapter.rs` | 创建 | 类型和错误转换 |
| `src-tauri/src/protocol/backend.rs` | 创建 | Backend trait 和 VirtualBackend |
| `src-tauri/src/protocol/server.rs` | 创建 | axum、鉴权、API、SSE |
| `src-tauri/src/protocol/runtime.rs` | 创建 | 多 Bot 生命周期管理 |
| `src-tauri/src/protocol/recorder.rs` | 创建 | 报文文件化 |
| `src-tauri/src/persistence/repo/packet.rs` | 创建 | protocol_packets repo |
| `src-tauri/src/models/internal.rs` | 修改 | 完整消息和成员离开事件 |
| `src-tauri/src/persistence/migrations/*.sql` | 创建 | 数字消息序列增量迁移 |
| `src-tauri/src/persistence/repo/message.rs` | 修改 | sequence 分配和双向查询 |
| `src-tauri/src/services/bot.rs` | 修改 | 委托 RuntimeManager 管理生命周期 |
| `src-tauri/src/services/message.rs` | 修改 | 发送完整 InternalEvent |
| `src-tauri/src/lib.rs` | 修改 | 注册 runtime 和 packet commands |
| `src-tauri/Cargo.toml` | 修改 | axum、async-trait 等依赖 |
| `src/views/main/logs.tsx` | 修改 | 协议报文列表和详情 |
| `src/lib/query/packets.ts` | 创建 | 报文查询 |
| `src/types/packet.ts` | 创建 | 前端报文类型 |

---

## 15. 设计原则

1. **遵循 Milky 1.2**：不引入 OneBot 命名或非标准聚合 API。
2. **Transport 与 Backend 分离**：Server 不直接操作数据库或具体业务 Service。
3. **虚拟和代理行为同时抽象**：事件订阅与 API 调用统一由 Backend 提供。
4. **内部 ID 与协议 ID 分离**：UUID 用于内部实体，数字 sequence 用于 Milky。
5. **生命周期有单一所有者**：所有 listener 和 task 归 RuntimeManager 管理。
6. **配置稳定**：端口和 token 创建时确定，启动时不静默改写。
7. **报文可追踪**：大 JSON 文件化，SQLite 保存可过滤索引。
8. **先完成消息闭环**：SSE、三个 API 和 `message_receive` 验证后再扩展功能。
