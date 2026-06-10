# Logs 页面定位与群优化设计（v2）

> 本 spec 覆盖 Logs 页面作为统一运行日志入口的定位、一期接入 `protocol_packets` 的方案，以及群功能优化的 ABCD 四个方向。
>
> **v2 修订记录**（基于代码现状核对）：
> 1. 存储根目录改为 `{app_data_dir}/groups/`——现有 packets 就直接在 `app_data_dir` 下，不存在 `data/` 这一级。
> 2. `group_photos.url` 是 `NOT NULL`，migration 0003 必须重建该表才能加 `file_path`，原 ALTER 方案不可行。
> 3. 照片展示链路补全：启用 Tauri assetProtocol + `convertFileSrc`，否则本地图片无法在 `<img>` 中渲染。
> 4. 文件上传/下载改为 dialog 插件 + 路径传递，不再让文件 bytes 走 IPC（50MB `Vec<u8>` 过 serde_json 会内存膨胀且卡死主线程）。新增依赖 `tauri-plugin-dialog`。
> 5. 置顶/免打扰移到 `conversations` 表（该表已有 `is_pinned`/`is_muted` 且覆盖私聊+群聊）；`user_groups` 的同名字段一期不使用。补齐缺失的会话状态读取链路。
> 6. 明确 Logs 与已存在的 `packets.tsx`（接口调试页）的边界。
> 7. 时间筛选参数定为 `since`/`until`，不引入 `from_time`/`to_time` 双轨。
> 8. 修正 `protocol_packets` 字段清单（补 `profile_id`、`related_object_*`），`source` 对 `bot_id` 为空做 fallback。
> 9. `file_hash` 用完整 SHA-256 hex，不截断。
> 10. 分类功能补齐 `group_categories` 的 list/create command——没有它们"移动到分类"无数据可选。

---

## 0. 设计原则

1. **Logs 是统一日志入口**：Logs 页面不等同于协议报文页面；一期先接入 `protocol_packets`，后续可聚合系统日志、群操作日志、审计事件等数据源。
2. **文件存储直接放在 `app_data_dir` 下**：与已有的 `{app_data_dir}/packets/YYYY-MM-DD/` 平级，新增 `{app_data_dir}/groups/`。数据库中存相对 `app_data_dir` 的路径（与 `protocol_packets.file_path` 的惯例一致）。
3. **不改动已有字段的语义**：`url` 保留外部地址语义；本地存储一律走新增的 `file_path`。
4. **UI 一致性**：沿用 shadcn/radix + lucide-react + Card/Sheet/AlertDialog 风格，参考 Dashboard Bot 管理的卡片列表模式。
5. **YAGNI**：一期不做文件预览、图片压缩、断点续传、批量下载。只做上传、列表、下载、删除。

---

## 第一部分：Logs 页面定位与 protocol_packets 接入

### 1.1 目标与边界

让 `src/views/main/logs.tsx` 从空数组（`logs.tsx` 中 `useMemo<LogEntry[]>(() => [], [])`）变成真实数据源，同时明确 Logs 的产品定位。

**与 `packets.tsx`（接口调试页）的边界**——两页消费同一组 hooks，但定位不同：

| | Logs | packets.tsx |
|---|---|---|
| 定位 | 运行日志统一入口（运维视角，只读） | 协议调试（开发视角） |
| 数据源 | 多源聚合（一期仅 packet） | 仅 protocol_packets |
| 展示 | LogEntry 统一模型，按 level/类型/时间筛选 | 报文统计卡片、原始 JSON、session 维度 |

一期 Logs 只接入 `protocol_packets`，因为它已由 `PacketRecorder` 写入，能最快形成可用列表。页面层使用统一的 `LogEntry` view model，`protocol_packets` 通过 adapter 映射进 `LogEntry`。后续新增数据源时只加 adapter，不改交互模型。

### 1.2 Logs 统一视图模型

`LogEntry` 是前端展示模型，不直接等同于任何一张数据库表。`logs.tsx` 现有的 `LogEntry`/`LogLevel`/`EventType` 类型需按下表改写（现有 `EventType` 的 `"message" | "request" | "system" | "group" | "connection"` 枚举与 packet 语义不兼容，直接替换）：

| 字段 | 语义 |
|------|------|
| `id` | 日志唯一 ID，带数据源前缀，如 `packet:{packet_id}` |
| `time` | 毫秒时间戳（`protocol_packets.created_at` 本身就是 `unixepoch()*1000`，不转 ISO 字符串，展示时格式化） |
| `level` | `info` / `error`；类型上预留 `debug` / `warn`，但一期筛选控件只展示 info/error 两项，避免出现永远匹配不到的选项 |
| `eventType` | 一期为 `packet.send` / `packet.receive`；未来扩展 `system.*` / `group.*` |
| `source` | 日志来源：`bot_id`，为空时 fallback 到 `profile_id`，再为空显示 `system` |
| `message` | 一行摘要，一期为 `action_name` |
| `dataSource` | 数据源标识，一期为 `packet` |
| `detailRef` | 详情读取引用，一期为 `packet_id` |

### 1.3 一期数据源：protocol_packets

`protocol_packets` 表结构（已存在，0001_initial_schema.sql）：

| 字段 | 用途 |
|------|------|
| `packet_id` | 唯一 ID |
| `bot_id` | 关联 Bot（**可空**） |
| `profile_id` | 关联协议 profile（可空） |
| `protocol_type` | `"milky"` / 未来 `"onebot_v11"` |
| `direction` | `"send"` / `"receive"`（有 CHECK 约束） |
| `action_name` | 对应 UI 的 `message`，也可作筛选项 |
| `file_path` | 原始 JSON 文件相对路径（相对 `app_data_dir`） |
| `related_object_type` / `related_object_id` | 关联业务对象（一期 Logs 不消费） |
| `is_error` | 对应 `level = "error"` |
| `session_id` | 调试会话（packets.tsx 消费，Logs 不消费） |
| `created_at` | 毫秒时间戳 |

### 1.4 Packet 到 LogEntry 的映射

| LogEntry 字段 | 来源 |
|-------------|------|
| `id` | `packet:` + `packet_id` |
| `time` | `created_at`（毫秒，原样） |
| `level` | `is_error ? "error" : "info"` |
| `eventType` | `packet.` + `direction`；列表项上以 Badge 附带显示 `protocol_type` |
| `source` | `bot_id ?? profile_id ?? "system"` |
| `message` | `action_name` |
| `dataSource` | `"packet"` |
| `detailRef` | `packet_id`，点击展开后经 `read_protocol_packet` 异步读取原始 JSON |

### 1.5 后端改动

- **Command**: 扩展已有 `list_protocol_packets`（`src-tauri/src/commands/packet.rs`）
  - 现有参数：`{ bot_id?, direction?, action_name?, since?, limit? }`
  - 新增参数：`{ protocol_type?: String, is_error?: bool, until?: u64 }`
  - 时间语义：`since` = `created_at >= since`（保持现有名称和语义不变），`until` = `created_at <= until`。**不引入** `from_time`/`to_time` 别名——该 command 的唯一调用方是 `useProtocolPackets`，无兼容负担。
  - 返回：`Vec<ProtocolPacketRecord>`（不新增重复类型）

- **Command**: 复用已有 `read_protocol_packet(packet_id)`
  - 已实现：读 `app_data_dir.join(file_path)` 的 JSON 并返回字符串，文件缺失/不可读时返回 Err。行为符合 `FR-PKT-006` 的懒加载要求，**无需改动**。

- **Repo**: `PacketRepo::list_packets`（`src-tauri/src/persistence/repo/packet.rs`）已用 QueryBuilder 动态拼接，增加 `protocol_type` / `is_error` / `until` 三个可选条件即可。

### 1.6 前端改动

- `src/types/packet.ts`：`PacketFilters` 增加 `protocol_type?` / `is_error?` / `until?`（`ProtocolPacket` 类型已含全部字段，不动）
- `src/lib/query/packets.ts`：`useProtocolPackets(filters)` 透传新筛选参数；`useProtocolPacketDetail` 不动
- `src/views/main/logs.tsx`：
  - 重写 `LogLevel`/`EventType`/`LogEntry` 类型（见 1.2）
  - 新增 packet-to-log adapter：`ProtocolPacket -> LogEntry`（纯函数，放在 logs.tsx 内，后续多数据源时再提取）
  - 时间范围筛选改为**传给后端**（`since`/`until` 由 `15m/1h/24h/7d` 档位换算），不再在客户端过滤——客户端过滤在 `limit` 截断后会丢数据
  - level / eventType 筛选项绑定到 `is_error` / `direction` 查询参数
  - 列表项可点击展开，懒加载原始 JSON（复用 `useProtocolPacketDetail`）
  - 沿用现有 2s `refetchInterval`

### 1.7 错误处理

- 文件读取失败：后端返回 Err，UI 在展开区显示灰色"原始报文文件已丢失或过期"
- 大 JSON 文件：一期直接全量读取展示，不做虚拟滚动（本地文件通常 2-20KB）

---

## 第二部分：群优化

### 2.1 文件存储规划

在 Tauri `app_data_dir()` 下（与 `packets/` 平级，**没有 `data/` 这一层**）：

```
{app_data_dir}/
├── packets/YYYY-MM-DD/           ← 已有（PacketRecorder）
└── groups/
    └── {group_id}/
        ├── files/
        │   └── {file_id}_{sanitized_file_name}
        └── albums/
            └── {album_id}/
                └── {photo_id}_{sanitized_file_name}
```

- 数据库 `file_path` 存相对 `app_data_dir` 的路径（如 `groups/{gid}/files/{fid}_{name}`），与 `protocol_packets.file_path` 惯例一致
- `sanitized_file_name` 规则：
  - 移除 `/ \ : ? * " < > |` 及控制字符
  - 去掉结尾的点和空格（Windows 兼容）
  - 命中 Windows 保留名（`CON`、`PRN`、`AUX`、`NUL`、`COM1-9`、`LPT1-9`，不含扩展名比较）时加 `_` 前缀
  - 限长 120 字节（UTF-8 安全截断，保留扩展名）
  - sanitize 后为空则用 `file_id` 兜底
  - 文件名前缀是 `{file_id}_`（UUID v7），天然不会同名冲突，无需 `_1`/`_2` 后缀
- 路径安全：所有读取/删除本地文件前，把相对路径 join 到 `app_data_dir` 后 canonicalize，校验最终路径仍位于 `{app_data_dir}/groups/` 内
- 写入策略：先写同目录临时文件（`.tmp` 后缀），成功后原子 rename 到目标路径；若数据库写入失败，删除已落盘文件
- 删除群/相册时：数据库行经 FK 级联删除（schema 已声明 `ON DELETE CASCADE`，连接已开 `foreign_keys(true)`）；磁盘文件由应用层删除整个 `groups/{group_id}/` 或 `albums/{album_id}/` 目录，失败仅 `eprintln!` 不阻断（与项目现有错误输出方式一致）

### 2.2 数据库 Migration 0003

文件：`src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql`，在 `migrations/mod.rs` 的 `all_migrations()` 注册（下一编号确为 0003）。

**`group_files`**：直接加列即可。

```sql
ALTER TABLE group_files ADD COLUMN file_path TEXT;
```

**`group_photos`**：现有 `url TEXT NOT NULL`，而本地照片只有 `file_path` 没有外部 url。SQLite 不支持修改列的 NOT NULL 约束，**必须重建表**：

```sql
CREATE TABLE group_photos_new (
    photo_id         TEXT PRIMARY KEY NOT NULL,
    album_id         TEXT NOT NULL,
    url              TEXT,              -- 改为可空：外部地址（保留给未来图床）
    file_path        TEXT,              -- 新增：本地相对路径
    description      TEXT,
    uploader_user_id TEXT NOT NULL,
    file_size        INTEGER,
    created_at       INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (album_id) REFERENCES group_albums(album_id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_user_id) REFERENCES im_accounts(user_id) ON DELETE CASCADE,
    CHECK (url IS NOT NULL OR file_path IS NOT NULL)
);
INSERT INTO group_photos_new (photo_id, album_id, url, description, uploader_user_id, file_size, created_at)
    SELECT photo_id, album_id, url, description, uploader_user_id, file_size, created_at FROM group_photos;
DROP TABLE group_photos;
ALTER TABLE group_photos_new RENAME TO group_photos;
CREATE INDEX IF NOT EXISTS idx_photos_album ON group_photos(album_id, created_at DESC);
```

> 没有任何表 FK 指向 `group_photos`（它只有出向 FK），重建不会触发外键级联问题。

**设计理由**：
- `url` 与 `file_path` 二选一必填（CHECK 保证），语义互斥清晰
- `file_path` 存相对路径，跨平台可迁移

### 2.3 照片/图片展示链路（前置依赖）

本地文件无法直接在 webview `<img src>` 中渲染，需启用 Tauri asset protocol：

- `src-tauri/tauri.conf.json`：
  ```json
  "app": {
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": ["$APPDATA/groups/**"]
      }
    }
  }
  ```
- 前端用 `convertFileSrc(absolutePath)`（`@tauri-apps/api/core`）生成可渲染的 URL
- 需要一个轻量 command `get_app_data_dir()`（或在 photo 列表返回中直接带绝对路径）供前端拼 `convertFileSrc` 入参。**决定：list command 直接返回拼好的绝对路径字段 `abs_path`**，前端无需自己 join
- 当前 `csp` 为 `null`，无需调整 CSP；若未来收紧 CSP，需放行 `asset:` 协议

### 2.4 A. 群文件实际上传/下载

**IPC 原则：文件内容不过 IPC。** 上传/下载都通过 dialog 插件取得路径，由 Rust 侧直接读写磁盘。

**新增依赖**：`tauri-plugin-dialog`（Cargo.toml + package.json `@tauri-apps/plugin-dialog`，二者当前均未安装）。不需要 `plugin-fs`——文件操作全在 Rust command 内用 `std::fs`/`tokio::fs` 完成。

#### 后端

- **Command**: `upload_group_file`
  - 参数：`{ user_id, group_id, parent_folder_id?: String, src_path: String }`（`src_path` 来自前端 dialog open() 的用户选择，文件名从 `src_path` 提取）
  - 限制：一期单文件最大 50MB（Rust 侧 `metadata().len()` 校验），超限返回错误；后续大文件再设计 streaming
  - 流程：
    1. 校验用户是群成员、群存在；`parent_folder_id` 非空时校验文件夹存在且属于该群
    2. 提取文件名并 sanitize（见 2.1）
    3. 流式读取计算 `file_hash`（完整 SHA-256 hex，64 字符——不截断，便于未来秒传/去重）
    4. `file_id = new_db_id()`（已有 helper，UUID v7）
    5. 拷贝到 `groups/{group_id}/files/` 下的临时文件，成功后 rename 到 `{file_id}_{sanitized_name}`
    6. `group_files` INSERT（带相对 `file_path`、`file_size`、`file_hash`、`uploader_user_id`）；repo 已有 `upsert_group_file`，扩展其支持 `file_path` 后复用
    7. 数据库写入失败则删除已落盘文件
  - 返回：`GroupFileEntity`（`models/entities.rs` 中已定义，增加 `file_path` 字段）

- **Command**: `download_group_file`
  - 参数：`{ file_id, dest_path: String }`（`dest_path` 来自前端 dialog save() 的用户选择）
  - 流程：查记录 → 校验 `file_path` 解析后仍在 `{app_data_dir}/groups/` 内 → `std::fs::copy` 到 `dest_path` → `download_count + 1`
  - 返回：`()`；源文件缺失时返回 Err（UI 提示"文件已丢失"）

- **Command**: `delete_group_file`
  - 参数：`{ user_id, file_id }`，校验操作者是上传者本人或群管理员/群主
  - 删除 `group_files` 行；应用层尝试删除磁盘文件，失败仅 `eprintln!`

#### 前端

- 群文件列表页：
  - "上传文件"按钮 → `open()`（plugin-dialog）选文件 → `invoke("upload_group_file", { srcPath })`。**不用** `<input type="file">`——webview 中拿不到真实文件路径
  - 每个文件项显示：文件名、大小、上传者、下载按钮（`save()` 选目标路径后 invoke）、删除按钮（AlertDialog 确认）
  - 文件夹层级用面包屑展示（依赖已有 `parent_folder_id` / `group_folders`）
- 新增 `src/types/group.ts` 类型 `GroupFile`、query hook `useGroupFiles(groupId, parentFolderId)`（`list_group_files` repo 方法已存在，需补 command 暴露）

### 2.5 B. 群相册

#### 后端

- **Repo/Service/Command 新增**：
  - `create_group_album` / `list_group_albums` / `delete_group_album`
  - `upload_group_photo`（同 2.4 的 dialog + `src_path` 模式，限制为图片扩展名，单张上限 20MB）
  - `list_group_photos` / `delete_group_photo`
- **实体/类型新增**：
  - Rust: `GroupAlbumEntity`、`GroupPhotoEntity` 及 row type
  - TypeScript: `GroupAlbum`、`GroupPhoto`
  - Query hooks: `useGroupAlbums`、`useGroupPhotos`
- 照片存储路径：`groups/{group_id}/albums/{album_id}/{photo_id}_{sanitized_name}`
- `list_group_photos` 返回中带 `abs_path`（见 2.3），前端 `convertFileSrc` 渲染
- 相册封面：`list_group_albums` 用**单条 SQL** 一并返回 `photo_count` 和封面（每个相册 `created_at` 最早一张照片的路径，相关子查询即可）——不在前端对每个相册单独发请求（避免 N+1）。`cover_url` 字段保留外部 URL 语义，本地封面不写库

#### 前端

- 群详情页新增"相册" Tab
- 相册以网格卡片展示封面 + 名称 + 照片数
- 进入相册后展示照片网格（`convertFileSrc` 渲染缩略，一期不生成缩略图、直接原图缩放），点击用简单 Modal 查看大图

### 2.6 C. 群管理 UI 优化

> 现状：成员操作（禁言/踢人/设管理/设头衔）已存在于 `chat-main-panel.tsx` 的**消息右键菜单**中；独立的成员列表面板尚不存在，本节是新增 UI 复用已有 mutations。

1. **群成员列表**（新增右侧面板或 Sheet）
   - 显示头像、昵称/名片、角色标签（Owner/Admin/Member，复用 `GroupRole`）
   - 管理员/群主可对成员操作：禁言、设管理、踢人、改名片——**复用消息右键菜单已接的 mutations**，入口用 DropdownMenu
2. **批量操作（轻量版）**
   - 不做复杂批量选择，仅在成员列表项右侧 hover 显示快捷操作图标（禁言时钟、踢人垃圾桶）
3. **群信息编辑**
   - 群名称、公告、简介的编辑入口统一放在群详情顶部"编辑"按钮，Sheet 侧滑表单（名称修改复用已有 `useRenameGroupMutation`）

### 2.7 D. 会话置顶/免打扰 + 群分类

#### 数据归属（v2 关键修正）

**置顶/免打扰落在 `conversations` 表**，不用 `user_groups`：

- `conversations` 已有 `is_pinned` / `is_muted` 字段，且同时覆盖私聊和群聊——放 `user_groups` 会导致私聊永远无法置顶，且产生两份状态
- `conversations` 行在消息写入时由 `repo/message.rs` upsert 产生；**无消息往来的会话行可能不存在**，因此 set 命令必须用 UPSERT，沿用 message.rs 中的 `conversation_id` 生成规则（`{owner}:{scene}:{peer_or_group}`）与 ON CONFLICT 目标
- `user_groups.is_pinned` / `is_muted` 一期闲置不用；`user_groups.category_id` / `sort_order` 用于群分类

#### 后端

- **新增 commands**（会话状态）：
  - `set_conversation_pinned(user_id, scene, peer_or_group_id, is_pinned)` — UPSERT conversations
  - `set_conversation_muted(user_id, scene, peer_or_group_id, is_muted)` — UPSERT conversations
  - `list_conversation_states(user_id)` — 返回 `{ conversation_scene, peer_user_id, group_id, is_pinned, is_muted }[]`。当前**不存在任何读 conversations 的 command**，会话列表是前端从好友+群拼的；一期不改这个推导模式，只新增此轻量状态查询供前端 merge
- **新增 commands**（群分类）：
  - `list_group_categories(user_id)` / `create_group_category(user_id, name)` / `delete_group_category(user_id, category_id)`（表已存在，UNIQUE(owner_user_id, name)）
  - `set_group_category(user_id, group_id, category_id?)` — 更新 `user_groups.category_id`，传 null 表示移出分类

#### 前端

- **会话列表**（`conversation-list.tsx`）：
  - `ConversationItem` 增加 `isPinned` / `isMuted`，由 `list_conversation_states` 结果按 `(scene, id)` merge
  - 排序：置顶组在前，组内仍按 `lastAt` 倒序（当前仅按 `lastAt` 排序）
  - 免打扰会话标题旁显示 `BellOff` 图标（lucide），不用 emoji
  - 右键菜单（已有 ContextMenu）增加："置顶/取消置顶"、"免打扰/开启通知"
- **群会话右键菜单**额外增加："移动到分类"（子菜单列出 `group_categories`，含"新建分类"入口）
- 分类的展示形式（侧边分组/筛选 Tab）一期从简：仅在会话列表顶部加分类筛选下拉，"全部"为默认

---

## 3. 优先级与阶段划分

| 阶段 | 内容 | 预期时间 |
|------|------|---------|
| 1 | Logs：重写 LogEntry 模型 + 扩展 packet 筛选参数 + adapter 接入 | 2-3 天 |
| 2 | migration 0003 + dialog 插件 + 群文件上传/下载 | 3-4 天 |
| 3 | assetProtocol + 群相册（实体、CRUD、照片网格） | 4-5 天 |
| 4 | 群成员列表/信息编辑 UI + 会话置顶/免打扰 + 群分类 | 4-5 天 |

依赖关系：阶段 3 依赖阶段 2 的 migration 与上传基础设施；阶段 1、4 与其他阶段无硬依赖。建议 Logs 先做：先建立统一日志入口的页面模型，用 packet 数据源快速交付可见价值。

---

## 4. 相关文件变更

### Logs 页面
- `src-tauri/src/persistence/repo/packet.rs` — `list_packets` 增加 `protocol_type` / `is_error` / `until` 筛选
- `src-tauri/src/commands/packet.rs` — 扩展 `list_protocol_packets` 参数；`read_protocol_packet` 不动
- `src/types/packet.ts` — `PacketFilters` 扩展
- `src/lib/query/packets.ts` — `useProtocolPackets` 透传新参数
- `src/views/main/logs.tsx` — 重写类型 + packet adapter + 服务端时间筛选

### 群优化
- `src-tauri/Cargo.toml` / `package.json` — 新增 `tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`
- `src-tauri/tauri.conf.json` — 启用 assetProtocol（scope `$APPDATA/groups/**`）
- `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql` — 新增（group_files 加列 + group_photos 重建）
- `src-tauri/src/persistence/migrations/mod.rs` — 注册 0003
- `src-tauri/src/persistence/repo/group/content.rs` — 文件 file_path 支持 + 相册/照片方法
- `src-tauri/src/persistence/repo/message.rs` 或新增 `repo/conversation.rs` — 会话状态 UPSERT/查询
- `src-tauri/src/services/group/content.rs` — 文件/相册服务（落盘、hash、路径校验）
- `src-tauri/src/commands/chat/group.rs` — 文件/相册/分类/会话状态命令
- `src-tauri/src/models/entities.rs` — `GroupFileEntity` 加 `file_path`；新增 `GroupAlbumEntity` / `GroupPhotoEntity`
- `src/types/group.ts` — 新增 `GroupFile` / `GroupAlbum` / `GroupPhoto` / `GroupCategory`
- `src/lib/query/groups.ts` — 新增 `useGroupFiles` / `useGroupAlbums` / `useGroupPhotos` / `useGroupCategories` / 会话状态 hook
- `src/components/chat/conversation-list.tsx` — 置顶排序、免打扰图标、右键菜单项、分类筛选
- `src/views/main/` 或 `src/components/chat/` — 群文件/相册/成员列表/信息编辑 UI

---

## 5. 不在本期范围

- 文件预览（图片外的大文件在线预览）
- 图片压缩、缩略图生成
- 断点续传、拖拽上传、大于 50MB 的文件
- 批量操作的选择模式
- 文件版本历史
- OneBot 协议适配
- 审计日志系统（`audit_events`）
- 数据导出/备份
- `user_groups.is_pinned` / `is_muted` 的启用（已被 `conversations` 表方案取代，字段保留不动）
- 结构化日志设施（tracing 等）——错误输出沿用项目现有 `eprintln!` 风格
