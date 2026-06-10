# 群功能优化设计 Spec

> 本 spec 覆盖群文件实际上传下载（A）、群相册（B）、群管理 UI（C）、会话置顶/免打扰 + 群分类（D）四个方向。
>
> 基于代码现状核对（2026-06-10）：`group_albums`/`group_photos`/`group_categories`/`conversations` 四张表已在 0001 schema 中，但缺少对应 command/前端 UI。`group_files` 有表但无 `file_path` 列，现有 `upsert_group_file` 是纯 DB 记录，不落盘文件。

---

## 0. 现状与缺口

### 0.1 已存在的表结构

| 表 | 状态 | 已有字段 | 缺失 |
|---|---|---|---|
| `group_files` | ⚠️ 不完整 | `file_id`, `group_id`, `parent_folder_id`, `file_name`, `file_size`, `file_hash`, `uploader_user_id`, `created_at`, `expire_at`, `download_count` | `file_path` |
| `group_photos` | ⚠️ 不完整 | `photo_id`, `album_id`, `url NOT NULL`, `description`, `uploader_user_id`, `file_size`, `created_at` | `file_path`；`url` 需改可空 |
| `group_albums` | ✅ 完整 | `album_id`, `group_id`, `name`, `cover_url`, `created_at`, `updated_at` | — |
| `group_categories` | ✅ 完整 | `category_id`, `owner_user_id`, `name`, `sort_order`, `created_at`, `updated_at` | — |
| `conversations` | ✅ 完整 | `is_pinned`, `is_muted` | 无读写 command |

### 0.2 已有但未暴露的功能

- `upsert_group_file` / `list_group_files`：service/repo/command 三层都有，但**只写 DB 记录，不实际读写磁盘文件**。`file_hash` 由调用方传入，无校验机制。
- `list_group_folders`：已有，返回含 `file_count`。
- `upsert_group_folder`：已有。
- `group_announcements`：三层完整，但**前端无 UI**（不在本 spec 范围内）。
- `group_essence_messages`：三层完整，前端在 chat-main-panel 的事件流中展示 `"xxx 的消息被设为了精华消息"`。

### 0.3 前端现状

- **群管理操作**：全部藏在 `chat-main-panel.tsx` 的**消息头像右键菜单**里（禁言/踢人/设管理/设头衔）。无独立成员列表面板。
- **群文件按钮**：composer 工具栏有 `<File className="size-4" />` 按钮（第 912 行），**无 onClick 处理**。
- **图片按钮**：composer 工具栏有 `<Image className="size-4" />` 按钮（第 909 行），**无 onClick 处理**。
- **会话列表**：`conversation-list.tsx` 按 `lastAt` 倒序排列，无置顶分组、无免打扰图标。

---

## 1. 阶段划分

| 阶段 | 内容 | 预计时间 | 前置依赖 |
|------|------|---------|---------|
| **Phase 1** | migration 0003 + tauri-plugin-dialog + assetProtocol | 1 天 | — |
| **Phase 2** | 群文件实际上传/下载/删除 | 2-3 天 | Phase 1 |
| **Phase 3** | 群相册（相册 CRUD + 照片上传/展示） | 2-3 天 | Phase 1+2 |
| **Phase 4** | 会话置顶/免打扰 + 群分类 | 2 天 | — |
| **Phase 5** | 群成员列表面板 + 群信息编辑 UI | 2 天 | Phase 4 |

> Phase 1 和 Phase 4 无依赖，可并行。建议 Phase 1 先做（基础设施），然后 Phase 2+3 串行，同时 Phase 4 并行。

---

## Phase 1：基础设施

### 1.1 新增依赖

**Cargo.toml** 添加：
```toml
tauri-plugin-dialog = "2"
```

**package.json** 添加：
```json
"@tauri-apps/plugin-dialog": "^2"
```

> `plugin-fs` 不需要——文件读写全在 Rust command 内用 `std::fs`/`tokio::fs` 完成。

### 1.2 Tauri 配置

**tauri.conf.json** — 启用 assetProtocol：
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

### 1.3 Migration 0003

文件：`src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql`

在 `migrations/mod.rs` 的 `all_migrations()` 注册（下一编号确为 0003）。

**`group_files`**：直接加列。
```sql
ALTER TABLE group_files ADD COLUMN file_path TEXT;
```

**`group_photos`**：现有 `url TEXT NOT NULL`，需改可空并加 `file_path`。SQLite 不支持 ALTER COLUMN，必须重建表：
```sql
CREATE TABLE group_photos_new (
    photo_id         TEXT PRIMARY KEY NOT NULL,
    album_id         TEXT NOT NULL,
    url              TEXT,              -- 改为可空：保留外部地址语义
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

> 没有任何表 FK 指向 `group_photos`（只有出向 FK），重建不会触发级联问题。

### 1.4 Rust 实体更新

**`GroupFileEntity`**（`src-tauri/src/models/entities.rs:202`）：
```rust
pub struct GroupFileEntity {
    pub file_id: String,
    pub group_id: DbId,
    pub parent_folder_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub file_path: Option<String>,      // ← 新增
    pub uploader_user_id: DbId,
    pub uploaded_at: u64,
    pub expire_at: Option<u64>,
    pub download_count: u32,
}
```

**新增 `GroupAlbumEntity`**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupAlbumEntity {
    pub album_id: DbId,
    pub group_id: DbId,
    pub name: String,
    pub cover_url: Option<String>,
    pub photo_count: u32,              // ← 运行时计算，不存表
    pub cover_file_path: Option<String>, // ← 运行时计算
    pub created_at: u64,
    pub updated_at: u64,
}
```

**新增 `GroupPhotoEntity`**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupPhotoEntity {
    pub photo_id: DbId,
    pub album_id: DbId,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub abs_path: Option<String>,      // ← 后端拼接 app_data_dir + file_path
    pub description: Option<String>,
    pub uploader_user_id: DbId,
    pub file_size: Option<u64>,
    pub created_at: u64,
}
```

**新增 `GroupCategoryEntity`**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupCategoryEntity {
    pub category_id: DbId,
    pub owner_user_id: DbId,
    pub name: String,
    pub sort_order: i32,
    pub created_at: u64,
    pub updated_at: u64,
}
```

---

## Phase 2：群文件（A）

### 2.1 文件存储规划

目录结构（`app_data_dir` 下，与 `packets/` 平级）：
```
{app_data_dir}/
├── packets/YYYY-MM-DD/
└── groups/
    └── {group_id}/
        ├── files/
        │   └── {file_id}_{sanitized_name}
        └── albums/
            └── {album_id}/
                └── {photo_id}_{sanitized_name}
```

数据库 `file_path` 存相对 `app_data_dir` 的路径（如 `groups/{gid}/files/{fid}_{name}`）。

### 2.2 文件名 sanitize

规则：
1. 移除 `\ / : ? * " < > |` 及控制字符（`\x00-\x1f`）
2. 去掉结尾的 `.` 和空格（Windows 兼容）
3. Windows 保留名（`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`，**不含扩展名比较**）时加 `_` 前缀
4. 限长 120 字节（UTF-8 安全截断，保留扩展名）
5. sanitize 后为空则用 `file_id` 兜底
6. 文件名前缀是 `{file_id}_`（UUID v7），天然不会同名冲突

### 2.3 路径安全

所有读取/删除本地文件前：
1. `app_data_dir.join(file_path)`
2. `.canonicalize()` 解析真实路径
3. 校验最终路径仍位于 `{app_data_dir}/groups/` 内
4. 越界则返回 `AppError::validation("invalid file path")`

### 2.4 写入策略

1. 先写同目录临时文件（`.tmp` 后缀）
2. `std::fs::rename` 原子移动到目标路径
3. 数据库写入失败则删除已落盘文件（rollback）

### 2.5 后端 Command

#### `upload_group_file`
```rust
#[tauri::command]
pub async fn upload_group_file(
    app: tauri::AppHandle,
    services: tauri::State<'_, crate::services::ServiceHub>,
    core: tauri::State<'_, crate::core::CoreContainer>,
    user_id: String,
    group_id: String,
    parent_folder_id: Option<String>,
    src_path: String,              // ← 前端 dialog open() 返回的绝对路径
) -> Result<GroupFileEntity, String>
```

流程：
1. 校验用户是群成员
2. `parent_folder_id` 非空时校验文件夹存在且属于该群
3. `tokio::fs::metadata(&src_path).await` 取文件大小，超限（50MB）返回 Err
4. 提取原文件名并 sanitize → `{file_id}_{sanitized_name}`
5. 流式读取计算 SHA-256 hex（64 字符完整 hash）
6. 拷贝到 `groups/{group_id}/files/` 临时文件 → rename
7. `group_files` INSERT（含相对 `file_path`、`file_size`、`file_hash`、`uploader_user_id`）
8. DB 失败 → 删除已落盘文件
9. emit `GroupFileUpserted` 事件

#### `download_group_file`
```rust
#[tauri::command]
pub async fn download_group_file(
    app: tauri::AppHandle,
    services: tauri::State<'_, crate::services::ServiceHub>,
    file_id: String,
    dest_path: String,             // ← 前端 dialog save() 返回的绝对路径
) -> Result<(), String>
```

流程：
1. 查记录 → 取 `file_path`
2. 路径安全校验（canonicalize + 前缀检查）
3. `std::fs::copy(src, dest_path)`
4. `download_count + 1`
5. 源文件缺失返回 Err

#### `delete_group_file`
```rust
#[tauri::command]
pub async fn delete_group_file(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    file_id: String,
) -> Result<(), String>
```

流程：
1. 查记录 → 校验操作者是上传者本人或群管理员/群主
2. DELETE `group_files` 行
3. 尝试删除磁盘文件，失败仅 `eprintln!` 不阻断

### 2.6 Repo 变更

**`src-tauri/src/persistence/repo/group/content.rs`**：

- `upsert_group_file`：INSERT 语句加 `file_path` 列
- `list_group_files`：SELECT 加 `file_path`
- 新增 `get_group_file_by_id(file_id) -> Option<GroupFileEntity>`
- 新增 `delete_group_file(file_id) -> Result<(), sqlx::Error>`
- `list_group_files` 增加 `parent_folder_id` 过滤参数（当前查询返回整个群的全部文件，不分文件夹）

**Row type 更新**：`GroupFileRow` 加 `file_path: Option<String>`。

### 2.7 前端

**新增 `src/types/group.ts` 类型**：
```typescript
export type GroupFile = {
  file_id: string;
  group_id: string;
  parent_folder_id: string;
  file_name: string;
  file_size: number;
  file_hash: string | null;
  file_path: string | null;
  uploader_user_id: string;
  uploaded_at: number;
  expire_at: number | null;
  download_count: number;
};
```

**新增 query hooks**（`src/lib/query/groups.ts`）：
- `useGroupFiles(groupId, parentFolderId?)` → `list_group_files`
- `invalidateGroupFilesQuery(groupId)`

**新增 mutations**（`src/lib/mutations.ts`）：
- `useUploadGroupFileMutation()`
- `useDownloadGroupFileMutation()`
- `useDeleteGroupFileMutation()`

**前端 dialog 使用**：
```typescript
import { open, save } from "@tauri-apps/plugin-dialog";

// 上传
const srcPath = await open({ multiple: false });
if (srcPath) await uploadMutation.mutateAsync({ groupId, srcPath });

// 下载
const destPath = await save({ defaultPath: file.file_name });
if (destPath) await downloadMutation.mutateAsync({ fileId, destPath });
```

> **不用** `<input type="file">`——webview 中拿不到真实文件路径。

### 2.8 UI 规划

群文件列表页（新增组件，建议放在群详情 Tab 内）：
- 顶部：面包屑（文件夹层级）+ "上传文件" 按钮
- 列表项：文件名、大小、上传者、下载次数、下载按钮、删除按钮（AlertDialog 确认）
- 文件夹可点击进入子文件夹

---

## Phase 3：群相册（B）

### 3.1 存储路径

```
groups/{group_id}/albums/{album_id}/{photo_id}_{sanitized_name}
```

照片上传限制：
- 图片扩展名：`.jpg`, `.jpeg`, `.png`, `.gif`, `.webp`, `.bmp`
- 单张上限 20MB

### 3.2 后端 Command

#### 相册 CRUD
```rust
#[tauri::command]
pub async fn create_group_album(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    group_id: String,
    name: String,
) -> Result<GroupAlbumEntity, String>

#[tauri::command]
pub async fn list_group_albums(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupAlbumEntity>, String>

#[tauri::command]
pub async fn delete_group_album(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    album_id: String,
) -> Result<(), String>
```

`list_group_albums` 用**单条 SQL** 返回每个相册的 `photo_count` 和封面（`created_at` 最早的照片的 `file_path`）：
```sql
SELECT
    a.album_id, a.group_id, a.name, a.cover_url, a.created_at, a.updated_at,
    (SELECT COUNT(*) FROM group_photos p WHERE p.album_id = a.album_id) AS photo_count,
    (SELECT p.file_path FROM group_photos p
     WHERE p.album_id = a.album_id AND p.file_path IS NOT NULL
     ORDER BY p.created_at ASC LIMIT 1) AS cover_file_path
FROM group_albums a
WHERE a.group_id = ?1
ORDER BY a.updated_at DESC
```

#### 照片上传
```rust
#[tauri::command]
pub async fn upload_group_photo(
    app: tauri::AppHandle,
    services: tauri::State<'_, crate::services::ServiceHub>,
    core: tauri::State<'_, crate::core::CoreContainer>,
    user_id: String,
    group_id: String,
    album_id: String,
    src_path: String,
    description: Option<String>,
) -> Result<GroupPhotoEntity, String>
```

流程与群文件上传基本一致，区别：
1. 校验扩展名为图片类型
2. 上限 20MB
3. 存储路径为 `groups/{gid}/albums/{aid}/{pid}_{name}`
4. `list_group_photos` 返回中带 `abs_path`（后端拼接 `app_data_dir.join(file_path)`）

#### 照片列表/删除
```rust
#[tauri::command]
pub async fn list_group_photos(
    app: tauri::AppHandle,
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    album_id: String,
) -> Result<Vec<GroupPhotoEntity>, String>

#[tauri::command]
pub async fn delete_group_photo(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    photo_id: String,
) -> Result<(), String>
```

### 3.3 照片展示链路

前端用 `convertFileSrc`（`@tauri-apps/api/core`）渲染本地图片：
```typescript
import { convertFileSrc } from "@tauri-apps/api/core";

<img src={photo.abs_path ? convertFileSrc(photo.abs_path) : photo.url} />
```

`abs_path` 由后端在 `list_group_photos` 中拼接好返回，前端无需自己 join。

### 3.4 Repo 新增

**`src-tauri/src/persistence/repo/group/content.rs`** 新增：
- `create_group_album(album) -> GroupAlbumEntity`
- `list_group_albums(group_id) -> Vec<GroupAlbumEntity>`
- `delete_group_album(album_id) -> ()`（CASCADE 会删照片记录，应用层删磁盘目录）
- `create_group_photo(photo) -> GroupPhotoEntity`
- `list_group_photos(album_id) -> Vec<GroupPhotoEntity>`
- `get_group_photo_by_id(photo_id) -> Option<GroupPhotoEntity>`
- `delete_group_photo(photo_id) -> ()`

**新增 Row types**：`GroupAlbumRow`, `GroupPhotoRow`。

### 3.5 前端

**新增类型**（`src/types/group.ts`）：
```typescript
export type GroupAlbum = {
  album_id: string;
  group_id: string;
  name: string;
  cover_url: string | null;
  photo_count: number;
  cover_file_path: string | null;
  cover_abs_path: string | null;
  created_at: number;
  updated_at: number;
};

export type GroupPhoto = {
  photo_id: string;
  album_id: string;
  url: string | null;
  file_path: string | null;
  abs_path: string | null;
  description: string | null;
  uploader_user_id: string;
  file_size: number | null;
  created_at: number;
};
```

**新增 query hooks**：
- `useGroupAlbums(groupId)`
- `useGroupPhotos(albumId)`
- `invalidateGroupAlbumsQuery(groupId)`
- `invalidateGroupPhotosQuery(albumId)`

**新增 mutations**：
- `useCreateGroupAlbumMutation()`
- `useDeleteGroupAlbumMutation()`
- `useUploadGroupPhotoMutation()`
- `useDeleteGroupPhotoMutation()`

**UI 规划**：
- 群详情页新增"相册" Tab
- 相册网格卡片：封面 + 名称 + 照片数
- 进入相册：照片网格（原图缩放，一期不生成缩略图）
- 点击照片：简单 Modal 查看大图

---

## Phase 4：会话置顶/免打扰 + 群分类（D）

### 4.1 数据归属

**置顶/免打扰落在 `conversations` 表**，不用 `user_groups`：
- `conversations` 已有 `is_pinned`/`is_muted`，且同时覆盖私聊和群聊
- `conversations` 行在消息写入时由 `repo/message.rs` 的 `upsert_conversation` 产生
- 无消息往来的会话行可能不存在 → **set 命令必须用 UPSERT**
- `user_groups.is_pinned` / `is_muted` **一期闲置不用**
- `user_groups.category_id` / `sort_order` 用于群分类

### 4.2 后端 Command

#### 会话状态
```rust
#[tauri::command]
pub async fn set_conversation_pinned(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    scene: String,              // "private" | "group"
    peer_user_id: Option<String>,
    group_id: Option<String>,
    is_pinned: bool,
) -> Result<(), String>

#[tauri::command]
pub async fn set_conversation_muted(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    scene: String,
    peer_user_id: Option<String>,
    group_id: Option<String>,
    is_muted: bool,
) -> Result<(), String>

#[tauri::command]
pub async fn list_conversation_states(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
) -> Result<Vec<ConversationState>, String>
```

`ConversationState`：
```rust
#[derive(Serialize)]
pub struct ConversationState {
    pub conversation_scene: String,
    pub peer_user_id: Option<String>,
    pub group_id: Option<String>,
    pub is_pinned: bool,
    pub is_muted: bool,
}
```

UPSERT 使用 `conversation_id` 生成规则：`{owner}:{scene}:{peer_or_group}`（与 `repo/message.rs` 中的规则一致）。

```sql
INSERT INTO conversations (conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id, is_pinned, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
    WHERE conversation_scene IN ('private', 'temp')
DO UPDATE SET is_pinned = excluded.is_pinned, updated_at = excluded.updated_at
ON CONFLICT(owner_user_id, conversation_scene, group_id)
    WHERE conversation_scene = 'group'
DO UPDATE SET is_pinned = excluded.is_pinned, updated_at = excluded.updated_at
```

> 注意：SQLite 3.35.0+ 才支持 `UPSERT` 多 conflict target；Tauri 自带的 SQLite 版本需确认。若不支持多 conflict，fallback 为 `INSERT OR REPLACE` 整行替换（会丢 `last_message_id`/`unread_count`，需先 SELECT 再 REPLACE）。

#### 群分类
```rust
#[tauri::command]
pub async fn list_group_categories(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
) -> Result<Vec<GroupCategoryEntity>, String>

#[tauri::command]
pub async fn create_group_category(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    name: String,
) -> Result<GroupCategoryEntity, String>

#[tauri::command]
pub async fn delete_group_category(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    category_id: String,
) -> Result<(), String>

#[tauri::command]
pub async fn set_group_category(
    services: tauri::State<'_, crate::services::ServiceHub>,
    user_id: String,
    group_id: String,
    category_id: Option<String>,
) -> Result<(), String>
```

### 4.3 Repo 新增

**新建 `src-tauri/src/persistence/repo/conversation.rs`**（或放在 `message.rs` 旁）：
- `upsert_conversation_pinned(owner_user_id, scene, peer_user_id, group_id, is_pinned) -> ()`
- `upsert_conversation_muted(...) -> ()`
- `list_conversation_states(owner_user_id) -> Vec<ConversationStateRow>`

**`src-tauri/src/persistence/repo/group/basic.rs`** 新增：
- `list_group_categories(owner_user_id) -> Vec<GroupCategoryEntity>`
- `create_group_category(owner_user_id, name) -> GroupCategoryEntity`
- `delete_group_category(category_id) -> ()`
- `set_group_category(owner_user_id, group_id, category_id) -> ()`

### 4.4 前端

**新增类型**（`src/types/group.ts`）：
```typescript
export type GroupCategory = {
  category_id: string;
  owner_user_id: string;
  name: string;
  sort_order: number;
  created_at: number;
  updated_at: number;
};

export type ConversationState = {
  conversation_scene: "private" | "group" | "temp";
  peer_user_id: string | null;
  group_id: string | null;
  is_pinned: boolean;
  is_muted: boolean;
};
```

**新增 query hooks**：
- `useConversationStates(userId)`
- `useGroupCategories(userId)`

**新增 mutations**：
- `useSetConversationPinnedMutation()`
- `useSetConversationMutedMutation()`
- `useCreateGroupCategoryMutation()`
- `useDeleteGroupCategoryMutation()`
- `useSetGroupCategoryMutation()`

**会话列表变更**（`conversation-list.tsx`）：
1. 拉取 `list_conversation_states` 按 `(scene, id)` merge 到 `ConversationItem`
2. 排序：置顶组在前，组内仍按 `lastAt` 倒序
3. 免打扰会话标题旁显示 `BellOff` 图标（lucide）
4. 右键菜单新增：
   - "置顶/取消置顶"
   - "免打扰/开启通知"
5. 群会话右键菜单额外增加：
   - "移动到分类"（子菜单列出 `group_categories`，含"新建分类"入口）
6. 分类展示：会话列表顶部加分类筛选下拉，"全部"为默认

---

## Phase 5：群管理 UI（C）

### 5.1 群成员列表面板

在 `chat-main-panel.tsx` 的 ResizablePanelGroup 中新增右侧面板（或 Sheet 侧滑），展示群成员列表：
- 头像（Avatar）、昵称/名片、角色标签（Owner/Admin/Member）
- 管理员/群主可对成员操作：禁言、设管理、踢人、改名片
- **复用已有 mutations**（`useMuteGroupMemberMutation`, `useKickGroupMemberMutation`, `useSetGroupMemberRoleMutation`, `useSetGroupMemberTitleMutation`）
- 入口用 DropdownMenu

### 5.2 批量操作（轻量版）

不做复杂批量选择，仅在成员列表项右侧 hover 显示快捷操作图标：
- 禁言时钟图标
- 踢人垃圾桶图标

### 5.3 群信息编辑

群详情顶部增加"编辑"按钮，Sheet 侧滑表单：
- 群名称（复用 `useRenameGroupMutation`）
- 群公告（复用 `upsert_group_announcement`，但前端目前无 UI）
- 群简介（`chat_groups` 表无 `description` 字段，**需 migration 加列**——建议作为 Phase 5 的子任务）

> 群公告的 UI 展示和编辑不在本 spec 核心范围，但群信息编辑 Sheet 应预留入口。

---

## 6. 不在本期范围

- 文件预览（图片外的大文件在线预览）
- 图片压缩、缩略图生成
- 断点续传、拖拽上传、大于 50MB 的文件
- 批量操作的选择模式（checkbox 批量选）
- 文件版本历史
- 群公告的完整前端展示（只保留编辑入口）
- `user_groups.is_pinned` / `is_muted` 的启用（已被 `conversations` 方案取代，字段保留不动）
- 群文件/相册的在线播放（视频/音频）

---

## 7. 文件变更清单

### Phase 1
- `src-tauri/Cargo.toml` — 加 `tauri-plugin-dialog`
- `package.json` — 加 `@tauri-apps/plugin-dialog`
- `src-tauri/tauri.conf.json` — 启用 assetProtocol
- `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql` — 新增
- `src-tauri/src/persistence/migrations/mod.rs` — 注册 0003
- `src-tauri/src/models/entities.rs` — `GroupFileEntity` 加 `file_path`；新增 `GroupAlbumEntity`, `GroupPhotoEntity`, `GroupCategoryEntity`

### Phase 2
- `src-tauri/src/persistence/repo/group/content.rs` — `file_path` 支持；新增 `get_group_file_by_id`, `delete_group_file`
- `src-tauri/src/persistence/repo/group/mod.rs` — 新增 Row types
- `src-tauri/src/services/group/content.rs` — 实际上传/下载/删除逻辑
- `src-tauri/src/commands/chat/group.rs` — 新增 `upload_group_file`, `download_group_file`, `delete_group_file`
- `src-tauri/src/lib.rs` — 注册新 command
- `src/types/group.ts` — 新增 `GroupFile`
- `src/lib/query/groups.ts` — 新增 `useGroupFiles`
- `src/lib/mutations.ts` — 新增文件 mutations
- `src/components/chat/` — 新增群文件列表组件

### Phase 3
- `src-tauri/src/persistence/repo/group/content.rs` — 相册/照片 CRUD
- `src-tauri/src/services/group/content.rs` — 照片上传/删除
- `src-tauri/src/commands/chat/group.rs` — 相册/照片 commands
- `src-tauri/src/lib.rs` — 注册新 command
- `src/types/group.ts` — 新增 `GroupAlbum`, `GroupPhoto`
- `src/lib/query/groups.ts` — 新增相册/照片 hooks
- `src/lib/mutations.ts` — 新增相册 mutations
- `src/components/chat/` — 新增相册/照片网格组件

### Phase 4
- `src-tauri/src/persistence/repo/conversation.rs` — 新建，会话状态 UPSERT/查询
- `src-tauri/src/persistence/repo/group/basic.rs` — 群分类 CRUD
- `src-tauri/src/services/` — 会话状态服务、群分类服务
- `src-tauri/src/commands/chat/` — 会话状态 commands、群分类 commands
- `src-tauri/src/lib.rs` — 注册新 command
- `src/types/group.ts` — 新增 `GroupCategory`, `ConversationState`
- `src/lib/query/groups.ts` — 新增会话状态、群分类 hooks
- `src/lib/mutations.ts` — 新增 mutations
- `src/components/chat/conversation-list.tsx` — 置顶排序、免打扰图标、右键菜单、分类筛选

### Phase 5
- `src/components/chat/chat-main-panel.tsx` — 新增群成员列表面板/Sheet
- `src/components/chat/` — 新增群信息编辑 Sheet
- `src-tauri/src/persistence/migrations/0004_group_description.sql` — 可选（给 chat_groups 加 description）
