# 群内容前端接入设计

**日期：** 2026-06-13  
**主题：** 将后端已就绪的群文件、群相册、群公告、精华消息能力接入前端 UI  
**状态：** 待实现

## 背景

UniBot 后端已经完整实现了群内容相关命令、服务、实体和事件：

- 群文件：`upsert_group_file`、`list_group_files`、`upload_group_file`、`download_group_file`、`delete_group_file`
- 群文件夹：`upsert_group_folder`、`list_group_folders`
- 群相册：`create_group_album`、`list_group_albums`、`delete_group_album`
- 群照片：`upload_group_photo`、`list_group_photos`、`delete_group_photo`
- 群公告：`upsert_group_announcement`、`list_group_announcements`
- 精华消息：`set_group_essence_message`、`list_group_essence_messages`

前端已经存在对应的 TanStack Query hooks 和 mutations（`src/lib/query/groups.ts`、`src/lib/mutations.ts`）以及类型定义（`src/types/group.ts`），但**没有任何 UI 组件调用它们**。本次设计目标是把这四种群内容能力真正可用的前端界面做出来，并保持与现有项目风格一致。

## 范围

### 包含

1. 聊天窗口标题栏新增「群工具」入口，提供四个菜单项。
2. **群文件浏览器**：独立 Tauri 窗口，支持文件夹层级、上传、下载、删除、新建文件夹。
3. **群相册浏览器**：独立 Tauri 窗口，支持相册网格 → 照片网格 → Lightbox 查看，支持新建相册、上传照片、删除。
4. **群公告面板**：聊天窗口右侧 Sheet，支持发布公告、列表展示。
5. **精华消息面板**：聊天窗口右侧 Sheet，支持从消息右键菜单设置/取消精华、列表展示。
6. 跨窗口实时同步：文件/相册/公告/精华变更后，所有相关窗口自动刷新。
7. 统一的错误处理和权限校验。

### 不包含

- 回收站功能（界面预留 Tab，后端暂不实现）。
- 文件/相册的批量选择、拖拽上传、移动、重命名（可后续扩展）。
- 群容量配额硬限制（先展示已用空间统计）。
- 公告图片上传（第一版仅支持文本/URL）。
- 真正的「修改人/修改时间」追踪（后端目前只有上传者/上传时间，第一版 UI 显示为修改人/修改时间，数据先回退为上传数据）。

## 入口设计

### 位置

在聊天窗口标题栏右侧新增两个图标：

- **九宫格（`LayoutGrid`）**：点击后下拉菜单展示「群文件 / 群相册 / 群公告 / 精华消息」。
- **更多（`MoreHorizontal`）**：保持现有行为，打开 `GroupInfoSheet` 群信息详情页。

这样分工清晰：九宫格 = 群内容工具快捷入口；更多 = 群信息/成员/设置详情页。

### 菜单实现

使用项目已有的 shadcn/ui `DropdownMenu`，配合 Lucide 图标。菜单对齐方式使用 `align="end"`，使菜单右侧与按钮右侧对齐，避免贴窗口右边缘被截断。

## 窗口架构

### 独立窗口：群文件、群相册

群文件和群相册使用独立 Tauri 窗口，以获得更多浏览空间，并允许与聊天窗口并行使用。

#### 窗口标识

- 文件窗口：`group-files-{user_id}-{group_id}`
- 相册窗口：`group-albums-{user_id}-{group_id}`

#### 创建方式

与现有 chat 窗口创建方式保持一致：由 Rust 后端统一创建。前端点击菜单项后调用 invoke：

```ts
invoke("open_group_files_window", { userId, groupId });
invoke("open_group_albums_window", { userId, groupId });
```

Rust 命令内部：

1. 按规则生成 label。
2. 检查该 label 窗口是否已存在；若存在则 `show` / `unminimize` / `set_focus`。
3. 若不存在，使用 `WebviewWindowBuilder` 创建新窗口。

#### 窗口参数

参考 chat 窗口参数：

- `inner_size(960.0, 680.0)`
- `min_inner_size(520.0, 420.0)`
- `.center()`
- title: `群文件 · {group_name}` / `群相册 · {group_name}`

#### 路由

新窗口加载前端路由：

- `index.html#/group-files?userId=xxx&groupId=yyy`
- `index.html#/group-albums?userId=xxx&groupId=yyy`

在 `App.tsx` 增加：

```tsx
<Route path="/group-files" element={<GroupFilesWindow />} />
<Route path="/group-albums" element={<GroupAlbumsWindow />} />
```

窗口组件从 URL query 读取 `userId` 和 `groupId`。

#### 能力（Capabilities）

新增 `src-tauri/capabilities/group-content.json`：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "group-content",
  "description": "Capability for group content windows",
  "windows": ["group-files-*", "group-albums-*"],
  "permissions": ["core:default"]
}
```

独立窗口创建由 Rust 命令完成，不依赖前端 capability；新窗口本身通过上述 capability 获得 `core:default` 权限。

### 右侧 Sheet：群公告、精华消息

群公告和精华消息在聊天窗口内通过右侧 Sheet 展示，不新建窗口，因为内容相对轻量。

Sheet 使用项目已有的 shadcn/ui `Sheet` 组件，保持与 `GroupInfoSheet` 一致的动画和宽度。

## 群文件浏览器

### 布局

借鉴 QQ 群盘的排版，但使用项目 shadcn/ui 风格（圆角、灰阶、outline 按钮、柔和边框）：

```
┌────────────────────────────────────────────────────┐
│ [文件] [回收站]     [+ 新建] [上传] [列表]          │
├────────────────────────────────────────────────────┤
│ 返回上一级 | 全部 > 2026RoboGame交流群_群盘资料  共 7 条│
├────────────────────────────────────────────────────┤
│ □  名称                大小    修改人    修改时间   操作 │
│ □  📁 PPT（包括...）    -      Alice   2025-12-24  ⋮  │
│ □  📄 设计稿.pdf       2.3 MB  Alice   2025-12-24  ⋮  │
│ □  📄 会议纪要.docx    156 KB  Bob     2025-12-24  ⋮  │
├────────────────────────────────────────────────────┤
│ 已用 128 MB / 10 GB                                │
└────────────────────────────────────────────────────┘
```

### 顶部工具栏

- **Tab**：`文件` / `回收站`（回收站第一版禁用或占位）。
- **+ 新建**：下拉菜单，当前只有「新建文件夹」。
- **上传**：打开系统文件选择器，上传到当前文件夹。
- **列表**：视图切换（第一版只有列表视图，为后续网格视图留扩展点）。

### 面包屑

- `返回上一级` 按钮。
- `全部 > 文件夹 A > 文件夹 B` 路径，可点击跳转。
- 右侧显示 `共 N 条`。

### 文件列表

表格列：

1. Checkbox（为后续批量操作预留）
2. 名称（文件夹/文件图标 + 名）
3. 大小
4. 修改人
5. 修改时间
6. 操作

行操作：

- 文件夹：点击进入。
- 文件：下载按钮 + 更多菜单（删除）。

### 底部工具栏

- **状态信息**：左下角显示已用空间，例如 `已用 128 MB / 10 GB`。
- 主要操作按钮（上传、新建文件夹、刷新）已集中在顶部工具栏，底部不再重复放置。

### 数据

- Query: `useGroupFilesQuery(userId, groupId, parentFolderId)`
- Query: `useGroupFoldersQuery(userId, groupId)`
- Mutation: `useUploadGroupFileMutation`
- Mutation: `useDownloadGroupFileMutation`
- Mutation: `useDeleteGroupFileMutation`
- Mutation: `useUpsertGroupFolderMutation`（用于新建文件夹）

## 群相册浏览器

### 层级

同一个独立窗口内两级视图：

1. **相册网格页**：展示所有相册。
2. **照片网格页**：点入相册后展示该相册的照片缩略图。

### 相册网格页

```
┌──────────────────────────────────────────────┐
│ 群相册 · {群名}        [新建相册] [上传] [刷新] [×]│
├──────────────────────────────────────────────┤
│ ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│ │  封面   │  │  封面   │  │    +    │        │
│ │ 活动照片 │  │ 表情包  │  │ 新建相册 │        │
│ │ 128 张  │  │ 45 张   │  │         │        │
│ └─────────┘  └─────────┘  └─────────┘        │
├──────────────────────────────────────────────┤
│ 已用 45 MB / 10 GB                           │
└──────────────────────────────────────────────┘
```

### 照片网格页

```
┌──────────────────────────────────────────────┐
│ ← 相册 / 活动照片      [上传照片] [刷新]     [×]│
├──────────────────────────────────────────────┤
│ ┌───┐ ┌───┐ ┌───┐ ┌───┐                     │
│ │ 🖼 │ │ 🖼 │ │ 🖼 │ │ 🖼 │                     │
│ └───┘ └───┘ └───┘ └───┘                     │
├──────────────────────────────────────────────┤
│ 128 张照片                                   │
└──────────────────────────────────────────────┘
```

### 交互

- 点击相册卡片进入照片网格页。
- 点击照片打开 Lightbox 查看大图，大图使用 `convertFileSrc()` 加载本地 `file_path`。
- 照片缩略图也使用 `convertFileSrc()` 从 `file_path` 或 `url` 加载。
- 右键/长按照片显示删除菜单。
- 相册卡片右键可删除相册。
- 主要操作按钮集中在顶部工具栏，底部仅显示统计信息。

### 数据

- Query: `useGroupAlbumsQuery(userId, groupId)`
- Query: `useGroupPhotosQuery(userId, groupId, albumId)`
- Mutation: `useCreateGroupAlbumMutation`
- Mutation: `useUploadGroupPhotoMutation`
- Mutation: `useDeleteGroupAlbumMutation`
- Mutation: `useDeleteGroupPhotoMutation`

## 群公告面板

### 入口

聊天窗口标题栏九宫格菜单 → 群公告 → 右侧 Sheet。

### 布局

```
┌──────────────────────────────────────┐
│ 群公告                           [×] │
├──────────────────────────────────────┤
│ [发布公告]  （仅群主/管理员可见）      │
├──────────────────────────────────────┤
│ 📢 本周活动安排                       │
│    本周六晚 8 点线上会议...           │
│    — Alice · 2025-12-20              │
│                                      │
│ 📢 新成员须知                         │
│    请大家先阅读群规...                │
│    — Bob · 2025-12-18                │
└──────────────────────────────────────┘
```

### 功能

- **发布公告**：弹窗输入标题 + 内容，可选图片 URL，调用 `upsert_group_announcement`。
- **公告列表**：按时间倒序展示标题、内容摘要、发布者、时间。
- **权限**：仅群主/管理员可发布/编辑/删除；普通成员只读。

### 数据

- Query: `useGroupAnnouncementsQuery(userId, groupId)`（需新增）
- Mutation: `useUpsertGroupAnnouncementMutation`（需新增）

## 精华消息面板

### 入口

- 列表：聊天窗口标题栏九宫格菜单 → 精华消息 → 右侧 Sheet。
- 设置/取消：聊天消息右键菜单增加「设为精华」/「取消精华」。

### 布局

```
┌──────────────────────────────────────┐
│ 精华消息                         [×] │
├──────────────────────────────────────┤
│ ⭐ Alice: 这个方案我觉得可以...        │
│    2025-12-20 14:32                  │
│                                      │
│ ⭐ Bob: 会议记录见附件                │
│    2025-12-19 09:15                  │
└──────────────────────────────────────┘
```

### 功能

- 按时间倒序展示精华消息。
- 点击某条可尝试定位到聊天中的原始消息（可选）。
- 群主/管理员可取消精华。

### 数据

- Query: `useGroupEssenceMessagesQuery(userId, groupId)`（需新增）
- Mutation: `useSetGroupEssenceMessageMutation`（需新增）

## 数据流与实时同步

### 后端事件

群内容操作完成后，后端已经 emits 以下 `InternalEvent`：

```rust
GroupFolderUpserted { folder_id, group_id, ... }
GroupFileUpserted { file_id, group_id, ... }
GroupFileDeleted { file_id, group_id, ... }
GroupAlbumCreated { album_id, group_id, ... }
GroupAlbumDeleted { album_id, group_id, ... }
GroupPhotoUploaded { photo_id, album_id, group_id, ... }
GroupPhotoDeleted { photo_id, album_id, group_id, ... }
GroupAnnouncementUpserted { announcement_id, group_id, ... }
GroupEssenceUpdated { essence_id, group_id, ... }
```

### 推送目标

目前 `core.rs` 只把事件推到 `chat-{user_id}`。新增文件/相册窗口后，同一个用户可能同时打开：

- `chat-{user_id}`
- `group-files-{user_id}-{group_id}`
- `group-albums-{user_id}-{group_id}`

需要在 `core.rs` 中把群内容事件同时推到这三个标签（如果窗口存在）。推荐新增一个 helper，使用项目已有的 `emit_to` 模式避免全局广播：

```rust
fn emit_group_content_event(
    app: &tauri::AppHandle,
    user_id: &str,
    group_id: &str,
    event: &InternalEvent,
) {
    for label in [
        format!("chat-{user_id}"),
        format!("group-files-{user_id}-{group_id}"),
        format!("group-albums-{user_id}-{group_id}"),
    ] {
        let _ = app.emit_to(&label, "chat:event", event);
    }
}
```

### 前端监听

- 独立窗口里也挂载 `useChatEventBus(userId)`，事件到达后按类型失效对应 query。
- Sheet 里的公告/精华面板直接复用 chat 窗口已有的 hook。
- 这样所有窗口/query 都能自动刷新，无需轮询。

## 错误处理

统一使用项目现有模式：

- 后端：`AppError::Validation` / `NotFound` / `Conflict` / `Storage` / `Internal`，通过 `IntoCommandResult` 转成前端 `String`。
- 前端 mutations：失败时用 `toast.error()` 提示。
- 权限不足：后端拒绝，前端 toast「无权限」。
- 文件操作异常：下载/上传路径错误、磁盘满等映射为 `Storage` 错误。
- 窗口创建失败：Rust 命令返回错误，前端 toast 提示。

### 关键错误场景

| 场景 | 处理 |
|------|------|
| 非管理员发公告 | 后端拒绝，前端 toast |
| 删除他人文件 | 后端校验 uploader/role，拒绝 |
| 下载文件不存在 | `NotFound` + toast |
| 相册窗口已存在 | focus 已有窗口，不报错 |
| 网络/磁盘错误 | `Storage` / `Internal` + toast |

## 测试策略

### 后端

- 为新增命令添加 `#[sqlx::test]` 测试：
  - `open_group_files_window` / `open_group_albums_window` 的创建与 focus 行为。
  - 文件上传、下载、删除的权限校验。
  - 相册/照片 CRUD。
  - 事件是否正确广播到多个窗口标签。
- 使用临时 SQLite 数据库，符合现有测试习惯。

### 前端

- 项目目前无前端单元测试配置，本轮以手动验证为主：
  - 文件上传、进入文件夹、下载、删除。
  - 相册创建、进入相册、上传照片、删除。
  - 公告发布、列表刷新。
  - 精华设置/取消、列表刷新。
  - 多窗口同时打开时的实时同步。
- 后续可考虑为浏览器组件补 Storybook 或简单组件测试。

## 未来扩展

- 回收站功能（需要后端支持）。
- 文件/相册批量选择、拖拽上传、移动、重命名。
- 群容量配额硬限制与清理策略。
- 公告图片上传。
- 真正的修改人/修改时间追踪（`updated_by` / `updated_at`）。
- 文件网格视图、相册封面自定义。
- 精华消息点击定位到原始消息。

## 相关文件

- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/chat.json`
- `src-tauri/src/core.rs`
- `src-tauri/src/commands/chat/group/content.rs`
- `src-tauri/src/services/group/content.rs`
- `src-tauri/src/models/entities.rs`
- `src-tauri/src/models/internal.rs`
- `src/App.tsx`
- `src/components/chat/chat-main-panel.tsx`
- `src/lib/query/groups.ts`
- `src/lib/query/keys.ts`
- `src/lib/mutations.ts`
- `src/lib/commands.ts`
- `src/types/group.ts`
- `src/hooks/use-chat-event-bus.ts`
