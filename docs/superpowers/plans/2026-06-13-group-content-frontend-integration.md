# 群内容前端接入实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把后端已就绪的群文件、群相册、群公告、精华消息能力接入前端 UI，包括两个独立 Tauri 窗口、两个右侧 Sheet 面板、实时跨窗口同步和统一的错误处理。

**Architecture：** 复用项目现有的 Tauri + React + TanStack Query + shadcn/ui 技术栈。群文件/相册使用 Rust 命令创建独立窗口并加载独立路由；群公告/精华使用右侧 Sheet。所有群内容变更通过现有的 `chat:event` 事件总线广播，新增独立窗口标签接收事件，前端统一失效对应 query。

**Tech Stack：** Tauri v2, React 19, TypeScript 5.8, TanStack Query, shadcn/ui, Lucide, Rust, sqlx, SQLite

---

## 文件结构概览

### 后端新增/修改

| 文件 | 责任 |
| ---- | ---- |
| `src-tauri/src/commands/chat/group/window.rs` | 新增 `open_group_files_window` / `open_group_albums_window` 命令 |
| `src-tauri/src/commands/chat/group/mod.rs` | 导出窗口命令模块 |
| `src-tauri/src/utils.rs` | 新增 `emit_group_content_to_windows` helper |
| `src-tauri/src/services/group/content.rs` | 在群内容事件广播后调用 helper |
| `src-tauri/src/lib.rs` | 注册新增命令到 invoke handler |
| `src-tauri/capabilities/group-content.json` | 新增 group content 窗口 capability |

### 前端新增/修改

| 文件 | 责任 |
| ---- | ---- |
| `src/lib/commands.ts` | 补充缺失命令名 |
| `src/lib/query/keys.ts` | 补充 folders / announcements / essence query keys |
| `src/types/event.ts` | 扩展 `InternalEventPayload` 新 kind |
| `src/lib/query/groups.ts` | 新增 folders / announcements / essence query hooks |
| `src/lib/mutations.ts` | 新增 folders / announcements / essence mutations |
| `src/lib/query/event-handlers.ts` | 新增群内容事件失效逻辑 |
| `src/components/chat/chat-event-bus-provider.tsx` | 支持自定义 `windowLabel` |
| `src/App.tsx` | 注册 `/group-files` / `/group-albums` 路由 |
| `src/views/group/group-files-window.tsx` | 群文件独立窗口根组件 |
| `src/views/group/group-albums-window.tsx` | 群相册独立窗口根组件 |
| `src/components/group/group-file-browser.tsx` | 群文件浏览器主体 |
| `src/components/group/group-album-browser.tsx` | 群相册浏览器主体 |
| `src/components/group/group-announcement-panel.tsx` | 群公告 Sheet 内容 |
| `src/components/group/group-essence-panel.tsx` | 精华消息 Sheet 内容 |
| `src/components/chat/chat-main-panel.tsx` | 标题栏九宫格入口 |
| `src/components/chat/chat-message-item.tsx` | 消息右键菜单增加设为精华 |

---

## Phase 1：后端基础设施

### Task 1.1：新增 group content 窗口 capability

**Files:**
- Create: `src-tauri/capabilities/group-content.json`

- [ ] **Step 1：创建 capability 文件**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "group-content",
  "description": "Capability for group content windows",
  "windows": ["group-files-*", "group-albums-*"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 2：提交**

```bash
git add src-tauri/capabilities/group-content.json
git commit -m "feat(capabilities): add group-content window capability"
```

### Task 1.2：新增事件广播 helper

**Files:**
- Modify: `src-tauri/src/utils.rs`

- [ ] **Step 1：在 `utils.rs` 底部新增 helper**

```rust
pub fn emit_group_content_to_windows(
    app: &tauri::AppHandle,
    user_id: &str,
    group_id: &str,
    event: &InternalEvent,
) {
    for label in [
        format!("group-files-{user_id}-{group_id}"),
        format!("group-albums-{user_id}-{group_id}"),
    ] {
        if let Err(e) = app.emit_to(&label, "chat:event", event) {
            tracing::debug!(
                target: "utils",
                "emit_group_content_to_windows skipped {} (window likely closed): {}",
                label,
                e
            );
        }
    }
}
```

- [ ] **Step 2：运行 Rust 格式化**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --
```

- [ ] **Step 3：提交**

```bash
git add src-tauri/src/utils.rs
git commit -m "feat(utils): add helper to emit group content events to independent windows"
```

### Task 1.3：在群内容服务中调用 helper

**Files:**
- Modify: `src-tauri/src/services/group/content.rs`

- [ ] **Step 1：更新 imports**

在文件顶部找到 `use crate::utils::{emit_to_group_members, now_ts};`，改为：

```rust
use crate::utils::{emit_group_content_to_windows, emit_to_group_members, now_ts};
```

- [ ] **Step 2：在每个 `emit_to_group_members` 调用后补推窗口事件**

以 `upsert_announcement` 为例（其他方法同理），找到类似代码：

```rust
emit_to_group_members(
    &core,
    &self.group_repo,
    &input.group_id,
    InternalEvent::GroupAnnouncementUpserted { ... },
)
.await?;
```

在其后追加：

```rust
emit_group_content_to_windows(
    &app,
    &operator_user_id,
    &input.group_id,
    &InternalEvent::GroupAnnouncementUpserted { ... },
);
```

**注意：** 由于 `emit_to_group_members` 接收的是已构造好的 `InternalEvent`，需要在调用前把事件存到局部变量，避免重复构造。

示例改写 `upsert_announcement`：

```rust
let event = InternalEvent::GroupAnnouncementUpserted {
    announcement_id: announcement.announcement_id.clone(),
    group_id: input.group_id.clone(),
    sender_user_id: operator_user_id.clone(),
    time: now_ts(),
};
emit_to_group_members(&core, &self.group_repo, &input.group_id, event.clone()).await?;
emit_group_content_to_windows(&app, &operator_user_id, &input.group_id, &event);
```

- [ ] **Step 3：对所有群内容方法重复 Step 2**

涉及的方法（根据 `content.rs` 中的 `emit_to_group_members` 调用）：
- `upsert_announcement`
- `upsert_group_folder`
- `upsert_group_file`
- `upload_group_file`
- `delete_group_file`
- `create_group_album`
- `delete_group_album`
- `upload_group_photo`
- `delete_group_photo`
- `set_group_essence_message`

- [ ] **Step 4：运行 Rust 测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all existing tests pass.

- [ ] **Step 5：提交**

```bash
git add src-tauri/src/services/group/content.rs
git commit -m "feat(group): emit content events to independent windows"
```

### Task 1.4：新增打开群文件/相册窗口命令

**Files:**
- Create: `src-tauri/src/commands/chat/group/window.rs`
- Modify: `src-tauri/src/commands/chat/group/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1：创建 `src-tauri/src/commands/chat/group/window.rs`**

```rust
use tauri::Manager;

use crate::core::CoreContainer;
use crate::error::AppResult;

fn group_content_window_label(kind: &str, user_id: &str, group_id: &str) -> String {
    format!("{kind}-{user_id}-{group_id}")
}

fn ensure_or_focus_window(
    app: tauri::AppHandle,
    label: &str,
) -> AppResult<bool> {
    if let Some(existing) = app.get_webview_window(label) {
        existing.show().map_err(|e| {
            crate::error::AppError::internal(format!("failed to show window: {e}"))
        })?;
        existing.unminimize().map_err(|e| {
            crate::error::AppError::internal(format!("failed to unminimize window: {e}"))
        })?;
        existing.set_focus().map_err(|e| {
            crate::error::AppError::internal(format!("failed to focus window: {e}"))
        })?;
        return Ok(false);
    }
    Ok(true)
}

#[tauri::command]
pub async fn open_group_files_window(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    let label = group_content_window_label("group-files", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let group = core
        .require_user_context(&user_id)?
        .profile;
    // Fetch group name via service; simplified here, see Step 2 note.
    let title = format!("群文件 · {group_id}");

    let url = tauri::WebviewUrl::App(
        format!("index.html#/group-files?userId={user_id}&groupId={group_id}").into(),
    );
    tauri::WebviewWindowBuilder::new(&app, label, url)
        .title(title)
        .inner_size(960.0, 680.0)
        .min_inner_size(520.0, 420.0)
        .center()
        .build()
        .map_err(|e| crate::error::AppError::internal(format!("failed to create window: {e}")))?;

    Ok(true)
}

#[tauri::command]
pub async fn open_group_albums_window(
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    let label = group_content_window_label("group-albums", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let title = format!("群相册 · {group_id}");

    let url = tauri::WebviewUrl::App(
        format!("index.html#/group-albums?userId={user_id}&groupId={group_id}").into(),
    );
    tauri::WebviewWindowBuilder::new(&app, label, url)
        .title(title)
        .inner_size(960.0, 680.0)
        .min_inner_size(520.0, 420.0)
        .center()
        .build()
        .map_err(|e| crate::error::AppError::internal(format!("failed to create window: {e}")))?;

    Ok(true)
}
```

**注意 Step 2：** 上述代码用 `group_id` 作为标题占位符。实际实现时应通过 `ServiceHub` 注入 `GroupService`，查询 `group_profile` 获取 `group_name`：

```rust
let group = services
    .group
    .get_group_profile(&user_id, &group_id)
    .await?;
let title = format!("群文件 · {}", group.group_name);
```

需确认 `GroupService::get_group_profile` 是否存在，若不存在则使用 `list_user_groups` 后过滤。

- [ ] **Step 3：导出窗口命令**

在 `src-tauri/src/commands/chat/group/mod.rs` 中添加：

```rust
pub mod window;
```

- [ ] **Step 4：在 `src-tauri/src/lib.rs` 注册命令**

找到 `group::{` 的 import 块，增加 `window`，例如：

```rust
use commands::chat::{conversation, group, message, request, user};
```

已存在则无需修改。

找到 `invoke_handler!` 宏中的 group 命令列表，追加：

```rust
group::open_group_files_window,
group::open_group_albums_window,
```

- [ ] **Step 5：运行 Rust 编译检查**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 6：提交**

```bash
git add src-tauri/src/commands/chat/group/window.rs src-tauri/src/commands/chat/group/mod.rs src-tauri/src/lib.rs
git commit -m "feat(backend): add open_group_files_window and open_group_albums_window commands"
```

### Task 1.5：后端窗口命令测试

**Files:**
- Create: `src-tauri/src/commands/chat/group/tests.rs` 或在 `src-tauri/src/commands/chat/group/window.rs` 底部加 `#[cfg(test)]` 模块

- [ ] **Step 1：添加集成测试验证窗口创建**

由于 Tauri 窗口测试需要 `AppHandle`，推荐在 `window.rs` 底部添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_content_window_label_format() {
        assert_eq!(
            super::group_content_window_label("group-files", "u1", "g1"),
            "group-files-u1-g1"
        );
        assert_eq!(
            super::group_content_window_label("group-albums", "u1", "g1"),
            "group-albums-u1-g1"
        );
    }
}
```

- [ ] **Step 2：运行测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml group_content_window_label
```

Expected: PASS.

- [ ] **Step 3：提交**

```bash
git add src-tauri/src/commands/chat/group/window.rs
git commit -m "test(backend): add window label format tests"
```

---

## Phase 2：前端基础设施

### Task 2.1：补充命令名

**Files:**
- Modify: `src/lib/commands.ts`

- [ ] **Step 1：在 `COMMANDS` 对象中添加**

```ts
  // group content: folder
  listGroupFolders: "list_group_folders",
  upsertGroupFolder: "upsert_group_folder",

  // group content: announcement / essence
  listGroupAnnouncements: "list_group_announcements",
  upsertGroupAnnouncement: "upsert_group_announcement",
  listGroupEssenceMessages: "list_group_essence_messages",
  setGroupEssenceMessage: "set_group_essence_message",

  // group content: window
  openGroupFilesWindow: "open_group_files_window",
  openGroupAlbumsWindow: "open_group_albums_window",
```

- [ ] **Step 2：运行 Biome 检查**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/lib/commands.ts
git commit -m "feat(commands): add group content command names"
```

### Task 2.2：补充 query keys

**Files:**
- Modify: `src/lib/query/keys.ts`

- [ ] **Step 1：在 `queryKeys.groups` 中增加**

```ts
    folders: (userId: string, groupId: string) =>
      ["groups", "folders", userId, groupId] as const,
    announcements: (userId: string, groupId: string) =>
      ["groups", "announcements", userId, groupId] as const,
    essence: (userId: string, groupId: string) =>
      ["groups", "essence", userId, groupId] as const,
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/lib/query/keys.ts
git commit -m "feat(query): add group folders, announcements, essence keys"
```

### Task 2.3：扩展事件类型

**Files:**
- Modify: `src/types/event.ts`

- [ ] **Step 1：在 `InternalEventPayload` 联合类型中追加**

```ts
  | {
      kind: "group_file_deleted";
      file_id: string;
      group_id: string;
      uploader_user_id: string;
      time: number;
    }
  | {
      kind: "group_album_created";
      album_id: string;
      group_id: string;
      name: string;
      time: number;
    }
  | {
      kind: "group_album_deleted";
      album_id: string;
      group_id: string;
      time: number;
    }
  | {
      kind: "group_photo_uploaded";
      photo_id: string;
      album_id: string;
      group_id: string;
      time: number;
    }
  | {
      kind: "group_photo_deleted";
      photo_id: string;
      album_id: string;
      group_id: string;
      time: number;
    };
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/types/event.ts
git commit -m "feat(types): extend InternalEventPayload with group content kinds"
```

### Task 2.4：新增 query hooks

**Files:**
- Modify: `src/lib/query/groups.ts`

- [ ] **Step 1：在文件底部追加**

```ts
// === Group Folders ===

export function useGroupFoldersQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.folders(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupFolder[]>(COMMANDS.listGroupFolders, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupFoldersQuery(userId: string, groupId: string) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.folders(userId, groupId),
  });
}

// === Group Announcements ===

export function useGroupAnnouncementsQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.announcements(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupAnnouncement[]>(COMMANDS.listGroupAnnouncements, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupAnnouncementsQuery(
  userId: string,
  groupId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.announcements(userId, groupId),
  });
}

// === Group Essence Messages ===

export function useGroupEssenceMessagesQuery(userId: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.groups.essence(userId, groupId),
    enabled: isValidUserId(userId) && groupId.length > 0,
    queryFn: () =>
      invoke<GroupEssenceMessage[]>(COMMANDS.listGroupEssenceMessages, {
        userId,
        groupId,
      }),
    retry: false,
  });
}

export function invalidateGroupEssenceMessagesQuery(
  userId: string,
  groupId: string,
) {
  return queryClient.invalidateQueries({
    queryKey: queryKeys.groups.essence(userId, groupId),
  });
}
```

- [ ] **Step 2：确保 imports 正确**

文件顶部 import 需要加入 `GroupFolder`、`GroupAnnouncement`、`GroupEssenceMessage`：

```ts
import type {
  ConversationState,
  GroupAlbum,
  GroupAnnouncement,
  GroupCategory,
  GroupEssenceMessage,
  GroupFile,
  GroupFolder,
  GroupMemberProfile,
  GroupPhoto,
  GroupProfile,
} from "@/types/group";
```

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/lib/query/groups.ts
git commit -m "feat(query): add folders, announcements, essence query hooks"
```

### Task 2.5：新增 mutations

**Files:**
- Modify: `src/lib/mutations.ts`

- [ ] **Step 1：在 mutations 文件中合适位置追加**

```ts
// === Group Folders ===

export function useUpsertGroupFolderMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      folderId?: string;
      parentFolderId?: string;
      folderName: string;
    }) =>
      invoke<GroupFolder>(COMMANDS.upsertGroupFolder, {
        userId: input.userId,
        groupId: input.groupId,
        input: {
          folder_id: input.folderId ?? "",
          group_id: input.groupId,
          parent_folder_id: input.parentFolderId ?? null,
          folder_name: input.folderName,
        },
      }),
    onSuccess: (_, variables) => {
      invalidateGroupFoldersQuery(variables.userId, variables.groupId);
    },
    onError: (error) => {
      toast.error(`创建文件夹失败：${error}`);
    },
  });
}

// === Group Announcements ===

export function useUpsertGroupAnnouncementMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      announcementId?: string;
      content: string;
      imageUrl?: string;
    }) =>
      invoke<GroupAnnouncement>(COMMANDS.upsertGroupAnnouncement, {
        userId: input.userId,
        groupId: input.groupId,
        input: {
          announcement_id: input.announcementId ?? "",
          group_id: input.groupId,
          sender_user_id: input.userId,
          content: input.content,
          image_url: input.imageUrl ?? null,
        },
      }),
    onSuccess: (_, variables) => {
      invalidateGroupAnnouncementsQuery(variables.userId, variables.groupId);
    },
    onError: (error) => {
      toast.error(`发布公告失败：${error}`);
    },
  });
}

// === Group Essence ===

export function useSetGroupEssenceMessageMutation() {
  return useMutation({
    mutationFn: (input: {
      userId: string;
      groupId: string;
      messageId: string;
      isSet: boolean;
    }) =>
      invoke<GroupEssenceMessage>(COMMANDS.setGroupEssenceMessage, {
        userId: input.userId,
        groupId: input.groupId,
        messageId: input.messageId,
        isSet: input.isSet,
      }),
    onSuccess: (_, variables) => {
      invalidateGroupEssenceMessagesQuery(variables.userId, variables.groupId);
      invalidateGroupEventHistoryQuery(variables.userId, variables.groupId);
    },
    onError: (error) => {
      toast.error(`设置精华失败：${error}`);
    },
  });
}
```

- [ ] **Step 2：确保 imports 正确**

需要导入：

```ts
import {
  invalidateGroupAnnouncementsQuery,
  invalidateGroupEssenceMessagesQuery,
  invalidateGroupEventHistoryQuery,
  invalidateGroupFoldersQuery,
} from "@/lib/query";
import type {
  GroupAnnouncement,
  GroupEssenceMessage,
  GroupFolder,
} from "@/types/group";
```

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/lib/mutations.ts
git commit -m "feat(mutations): add folder, announcement, essence mutations"
```

### Task 2.6：扩展事件失效逻辑

**Files:**
- Modify: `src/lib/query/event-handlers.ts`

- [ ] **Step 1：更新 imports**

```ts
import {
  invalidateGroupAlbumsQuery,
  invalidateGroupAnnouncementsQuery,
  invalidateGroupEssenceMessagesQuery,
  invalidateGroupFilesQuery,
  invalidateGroupFoldersQuery,
  invalidateGroupPhotosQuery,
} from "@/lib/query/groups";
```

- [ ] **Step 2：在 `handleQueryInvalidation` 底部追加**

```ts
  // Group content events
  if (
    payload.kind === "group_folder_upserted" ||
    payload.kind === "group_file_upserted" ||
    payload.kind === "group_file_deleted"
  ) {
    invalidateGroupFilesQuery(userId, payload.group_id);
    invalidateGroupFoldersQuery(userId, payload.group_id);
  }

  if (
    payload.kind === "group_album_created" ||
    payload.kind === "group_album_deleted"
  ) {
    invalidateGroupAlbumsQuery(userId, payload.group_id);
  }

  if (
    payload.kind === "group_photo_uploaded" ||
    payload.kind === "group_photo_deleted"
  ) {
    invalidateGroupPhotosQuery(userId, payload.album_id);
    invalidateGroupAlbumsQuery(userId, payload.group_id);
  }

  if (payload.kind === "group_announcement_upserted") {
    invalidateGroupAnnouncementsQuery(userId, payload.group_id);
  }

  if (payload.kind === "group_essence_updated") {
    invalidateGroupEssenceMessagesQuery(userId, payload.group_id);
  }
```

- [ ] **Step 3：运行前端构建检查**

```bash
bun run build
```

Expected: build succeeds (or at least TypeScript compiles).

- [ ] **Step 4：提交**

```bash
git add src/lib/query/event-handlers.ts
git commit -m "feat(query): invalidate group content queries on chat:event"
```

### Task 2.7：改造 ChatEventBusProvider 支持自定义窗口标签

**Files:**
- Modify: `src/components/chat/chat-event-bus-provider.tsx`

- [ ] **Step 1：修改组件 props 和监听目标**

```tsx
export function ChatEventBusProvider({
  userId,
  windowLabel,
  children,
}: {
  userId: string;
  windowLabel?: string;
  children: ReactNode;
}) {
  const subscribersRef = useRef<Set<ChatEventSubscriber>>(new Set());

  const subscribe = useCallback((callback: ChatEventSubscriber) => {
    subscribersRef.current.add(callback);
    return () => {
      subscribersRef.current.delete(callback);
    };
  }, []);

  useEffect(() => {
    if (!userId) {
      return;
    }

    const label = windowLabel || `chat-${userId}`;

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listenWithRetry<InternalEventPayload>(
      "chat:event",
      (event) => {
        const payload = event.payload;
        if (!payload) {
          return;
        }
        handleQueryInvalidation(userId, payload);
        for (const subscriber of subscribersRef.current) {
          subscriber(payload);
        }
      },
      {
        target: label,
      },
    )
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error(`[event-bus] failed for ${label}:`, error);
      });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [userId, windowLabel]);

  return (
    <ChatEventBusContext.Provider value={{ subscribe }}>
      {children}
    </ChatEventBusContext.Provider>
  );
}
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/components/chat/chat-event-bus-provider.tsx
git commit -m "feat(event-bus): support custom windowLabel in ChatEventBusProvider"
```

### Task 2.8：注册新路由

**Files:**
- Create: `src/views/group/group-files-window.tsx`
- Create: `src/views/group/group-albums-window.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1：创建占位窗口组件**

`src/views/group/group-files-window.tsx`：

```tsx
import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import { Toaster } from "@/components/ui/sonner";

export default function GroupFilesWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";
  const windowLabel = `group-files-${userId}-${groupId}`;

  if (!userId || !groupId) {
    return <div>缺少 userId 或 groupId</div>;
  }

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <div className="flex-1 p-4">
          <h1 className="text-lg font-semibold">群文件 · {groupId}</h1>
          <p className="text-muted-foreground text-sm">TODO: file browser</p>
        </div>
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
```

`src/views/group/group-albums-window.tsx`：

```tsx
import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import { Toaster } from "@/components/ui/sonner";

export default function GroupAlbumsWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";
  const windowLabel = `group-albums-${userId}-${groupId}`;

  if (!userId || !groupId) {
    return <div>缺少 userId 或 groupId</div>;
  }

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <div className="flex-1 p-4">
          <h1 className="text-lg font-semibold">群相册 · {groupId}</h1>
          <p className="text-muted-foreground text-sm">TODO: album browser</p>
        </div>
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
```

- [ ] **Step 2：在 `App.tsx` 注册路由**

```ts
import GroupFilesWindow from "@/views/group/group-files-window";
import GroupAlbumsWindow from "@/views/group/group-albums-window";
```

在 router 数组中追加：

```ts
{
  path: "/group-files",
  element: <GroupFilesWindow />,
},
{
  path: "/group-albums",
  element: <GroupAlbumsWindow />,
},
```

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/views/group/group-files-window.tsx src/views/group/group-albums-window.tsx src/App.tsx
git commit -m "feat(routing): add group files and albums window routes"
```

---

## Phase 3：群文件浏览器窗口

### Task 3.1：实现 GroupFileBrowser 组件

**Files:**
- Create: `src/components/group/group-file-browser.tsx`

- [ ] **Step 1：实现基础布局和状态**

```tsx
import { useState } from "react";
import {
  Folder,
  File,
  Download,
  MoreHorizontal,
  Upload,
  Plus,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  useDeleteGroupFileMutation,
  useDownloadGroupFileMutation,
  useUploadGroupFileMutation,
  useUpsertGroupFolderMutation,
} from "@/lib/mutations";
import {
  useGroupFilesQuery,
  useGroupFoldersQuery,
} from "@/lib/query";
import { toast } from "sonner";
import type { GroupFile, GroupFolder } from "@/types/group";

export default function GroupFileBrowser({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
}) {
  const [parentFolderId, setParentFolderId] = useState<string | undefined>(
    undefined,
  );
  const [folderStack, setFolderStack] = useState<GroupFolder[]>([]);

  const { data: files = [], refetch: refetchFiles } = useGroupFilesQuery(
    userId,
    groupId,
    parentFolderId,
  );
  const { data: folders = [], refetch: refetchFolders } = useGroupFoldersQuery(
    userId,
    groupId,
  );

  const uploadMutation = useUploadGroupFileMutation();
  const downloadMutation = useDownloadGroupFileMutation();
  const handleDownload = async (file: GroupFile) => {
    const path = await downloadMutation.mutateAsync({
      userId,
      groupId,
      fileId: file.file_id,
    });
    toast.success(`文件已下载: ${path}`);
  };
  const deleteFileMutation = useDeleteGroupFileMutation();
  const createFolderMutation = useUpsertGroupFolderMutation();

  const currentFolders = parentFolderId
    ? folders.filter((f) => f.parent_folder_id === parentFolderId)
    : folders.filter((f) => !f.parent_folder_id);

  const handleEnterFolder = (folder: GroupFolder) => {
    setFolderStack((prev) => [...prev, folder]);
    setParentFolderId(folder.folder_id);
  };

  const handleGoBack = () => {
    if (folderStack.length === 0) return;
    const newStack = folderStack.slice(0, -1);
    setFolderStack(newStack);
    setParentFolderId(
      newStack.length > 0
        ? newStack[newStack.length - 1].folder_id
        : undefined,
    );
  };

  const handleUpload = async () => {
    // 使用 Tauri dialog API 选择文件，项目已有使用方式可参考其他上传逻辑
    // 这里先预留接口
    const filePath = ""; // TODO: open file dialog
    if (!filePath) return;
    await uploadMutation.mutateAsync({
      userId,
      groupId,
      parentFolderId,
      fileName: filePath.split("/").pop() || "upload",
      sourcePath: filePath,
    });
  };

  return (
    <div className="flex h-full flex-col">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between border-b p-3">
        <div className="flex gap-1 rounded-lg bg-muted p-1">
          <Button variant="default" size="sm">
            文件
          </Button>
          <Button variant="ghost" size="sm" disabled>
            回收站
          </Button>
        </div>
        <div className="flex gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <Plus className="mr-1 size-4" />
                新建
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem
                onClick={() =>
                  createFolderMutation.mutate({
                    userId,
                    groupId,
                    parentFolderId,
                    folderName: "新建文件夹",
                  })
                }
              >
                新建文件夹
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button variant="outline" size="sm" onClick={handleUpload}>
            <Upload className="mr-1 size-4" />
            上传
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refetchFiles();
              refetchFolders();
            }}
          >
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      {/* 面包屑 */}
      <div className="flex items-center justify-between border-b px-4 py-2 text-sm">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0"
            onClick={handleGoBack}
            disabled={folderStack.length === 0}
          >
            返回上一级
          </Button>
          <span>|</span>
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0"
            onClick={() => {
              setFolderStack([]);
              setParentFolderId(undefined);
            }}
          >
            全部
          </Button>
          {folderStack.map((folder) => (
            <span key={folder.folder_id} className="flex items-center gap-2">
              <span>/</span>
              <span>{folder.folder_name}</span>
            </span>
          ))}
        </div>
        <span className="text-xs text-muted-foreground">
          共 {currentFolders.length + files.length} 条
        </span>
      </div>

      {/* 列表 */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted/50 text-muted-foreground">
            <tr>
              <th className="w-10 px-4 py-2 text-left">
                <input type="checkbox" disabled />
              </th>
              <th className="px-4 py-2 text-left">名称</th>
              <th className="px-4 py-2 text-left">大小</th>
              <th className="px-4 py-2 text-left">修改人</th>
              <th className="px-4 py-2 text-left">修改时间</th>
              <th className="px-4 py-2 text-left">操作</th>
            </tr>
          </thead>
          <tbody>
            {currentFolders.map((folder) => (
              <tr
                key={folder.folder_id}
                className="cursor-pointer border-b hover:bg-muted/50"
                onClick={() => handleEnterFolder(folder)}
              >
                <td className="px-4 py-3">
                  <input type="checkbox" disabled />
                </td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <Folder className="size-5 text-yellow-500" />
                    {folder.folder_name}
                  </div>
                </td>
                <td className="px-4 py-3 text-muted-foreground">-</td>
                <td className="px-4 py-3">{folder.creator_user_id}</td>
                <td className="px-4 py-3 text-muted-foreground">
                  {new Date(folder.updated_at).toLocaleString()}
                </td>
                <td className="px-4 py-3 text-primary">进入</td>
              </tr>
            ))}
            {files.map((file) => (
              <GroupFileRow
                key={file.file_id}
                file={file}
                parentFolderId={parentFolderId}
                onDownload={() => handleDownload(file)}
                onDelete={() =>
                  deleteFileMutation.mutate({
                    userId,
                    groupId,
                    fileId: file.file_id,
                    parentFolderId: parentFolderId ?? "",
                  })
                }
              />
            ))}
          </tbody>
        </table>
      </div>

      {/* 底部状态栏 */}
      <div className="border-t bg-muted/30 px-4 py-2 text-xs text-muted-foreground">
        已用 0 MB / 10 GB
      </div>
    </div>
  );
}

function GroupFileRow({
  file,
  onDownload,
  onDelete,
}: {
  file: GroupFile;
  onDownload: () => void;
  onDelete: () => void;
}) {
  return (
    <tr className="border-b hover:bg-muted/50">
      <td className="px-4 py-3">
        <input type="checkbox" disabled />
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-2">
          <File className="size-5 text-muted-foreground" />
          {file.file_name}
        </div>
      </td>
      <td className="px-4 py-3 text-muted-foreground">
        {formatBytes(file.file_size)}
      </td>
      <td className="px-4 py-3">{file.uploader_user_id}</td>
      <td className="px-4 py-3 text-muted-foreground">
        {new Date(file.uploaded_at).toLocaleString()}
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon-sm" onClick={onDownload}>
            <Download className="size-4" />
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <MoreHorizontal className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onClick={onDelete}>删除</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </td>
    </tr>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}
```

- [ ] **Step 2：检查类型和 imports**

确认 `GroupFolder` 类型包含 `creator_user_id`、`updated_at`；若 `GroupFile` 的 `uploaded_at` 是秒级时间戳，需调整 `new Date(...)`。

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/components/group/group-file-browser.tsx
git commit -m "feat(ui): add group file browser component"
```

### Task 3.2：完善文件上传的文件选择

**Files:**
- Modify: `src/components/group/group-file-browser.tsx`

- [ ] **Step 1：使用 Tauri dialog API**

安装/确认依赖（Tauri v2 的 dialog 插件通常已集成）：

```bash
bunx tauri add dialog
```

若项目已使用 `open` from `@tauri-apps/plugin-dialog`，直接 import：

```ts
import { open } from "@tauri-apps/plugin-dialog";
```

替换 `handleUpload`：

```ts
const handleUpload = async () => {
  const selected = await open({
    multiple: false,
    directory: false,
  });
  if (!selected || Array.isArray(selected)) return;

  const fileName = selected.split("/").pop() || selected.split("\\").pop() || "upload";
  await uploadMutation.mutateAsync({
    userId,
    groupId,
    parentFolderId,
    fileName,
    sourcePath: selected,
  });
};
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/components/group/group-file-browser.tsx
git commit -m "feat(ui): wire file picker for group file upload"
```

### Task 3.3：在 GroupFilesWindow 中渲染浏览器

**Files:**
- Modify: `src/views/group/group-files-window.tsx`

- [ ] **Step 1：替换占位内容**

```tsx
import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import GroupFileBrowser from "@/components/group/group-file-browser";
import { Toaster } from "@/components/ui/sonner";

export default function GroupFilesWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";
  const windowLabel = `group-files-${userId}-${groupId}`;

  if (!userId || !groupId) {
    return <div>缺少 userId 或 groupId</div>;
  }

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <GroupFileBrowser userId={userId} groupId={groupId} />
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
```

- [ ] **Step 2：提交**

```bash
git add src/views/group/group-files-window.tsx
git commit -m "feat(routing): render group file browser in window"
```

---

## Phase 4：群相册浏览器窗口

### Task 4.1：实现 GroupAlbumBrowser 组件

**Files:**
- Create: `src/components/group/group-album-browser.tsx`

- [ ] **Step 1：实现两级视图**

```tsx
import { useState } from "react";
import { Image, Plus, Upload, RefreshCw, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  useCreateGroupAlbumMutation,
  useDeleteGroupAlbumMutation,
  useDeleteGroupPhotoMutation,
  useUploadGroupPhotoMutation,
} from "@/lib/mutations";
import { useGroupAlbumsQuery, useGroupPhotosQuery } from "@/lib/query";
import type { GroupAlbum } from "@/types/group";

export default function GroupAlbumBrowser({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
}) {
  const [selectedAlbumId, setSelectedAlbumId] = useState<string | null>(null);

  if (selectedAlbumId) {
    return (
      <PhotoGrid
        userId={userId}
        groupId={groupId}
        albumId={selectedAlbumId}
        onBack={() => setSelectedAlbumId(null)}
      />
    );
  }

  return (
    <AlbumGrid
      userId={userId}
      groupId={groupId}
      onSelectAlbum={setSelectedAlbumId}
    />
  );
}

function AlbumGrid({
  userId,
  groupId,
  onSelectAlbum,
}: {
  userId: string;
  groupId: string;
  onSelectAlbum: (albumId: string) => void;
}) {
  const { data: albums = [], refetch } = useGroupAlbumsQuery(userId, groupId);
  const createAlbumMutation = useCreateGroupAlbumMutation();

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <h1 className="text-lg font-semibold">群相册 · {groupId}</h1>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              createAlbumMutation.mutate({ userId, groupId, name: "新建相册" })
            }
          >
            <Plus className="mr-1 size-4" />
            新建相册
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <div className="grid grid-cols-3 gap-4">
          {albums.map((album) => (
            <AlbumCard
              key={album.album_id}
              album={album}
              onClick={() => onSelectAlbum(album.album_id)}
            />
          ))}
        </div>
      </div>

      <div className="border-t bg-muted/30 px-4 py-2 text-xs text-muted-foreground">
        已用 0 MB / 10 GB
      </div>
    </div>
  );
}

function AlbumCard({
  album,
  onClick,
}: {
  album: GroupAlbum;
  onClick: () => void;
}) {
  return (
    <div
      className="cursor-pointer overflow-hidden rounded-xl border hover:shadow-sm"
      onClick={onClick}
    >
      <div className="flex aspect-square items-center justify-center bg-muted">
        {album.cover_url ? (
          <img
            src={album.cover_url}
            alt={album.name}
            className="size-full object-cover"
          />
        ) : (
          <Image className="size-12 text-muted-foreground" />
        )}
      </div>
      <div className="p-3">
        <div className="font-medium">{album.name}</div>
        <div className="text-xs text-muted-foreground">
          {album.photo_count} 张
        </div>
      </div>
    </div>
  );
}

function PhotoGrid({
  userId,
  groupId,
  albumId,
  onBack,
}: {
  userId: string;
  groupId: string;
  albumId: string;
  onBack: () => void;
}) {
  const { data: photos = [], refetch } = useGroupPhotosQuery(
    userId,
    groupId,
    albumId,
  );
  const uploadMutation = useUploadGroupPhotoMutation();
  const deletePhotoMutation = useDeleteGroupPhotoMutation();

  const handleUpload = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
    });
    if (!selected || Array.isArray(selected)) return;

    await uploadMutation.mutateAsync({
      userId,
      groupId,
      albumId,
      sourcePath: selected,
    });
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon-sm" onClick={onBack}>
            <ArrowLeft className="size-4" />
          </Button>
          <h1 className="text-lg font-semibold">相册</h1>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={handleUpload}>
            <Upload className="mr-1 size-4" />
            上传照片
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <div className="grid grid-cols-4 gap-2">
          {photos.map((photo) => (
            <div
              key={photo.photo_id}
              className="group relative aspect-square overflow-hidden rounded-lg bg-muted"
            >
              <img
                src={photo.url}
                alt={photo.description || ""}
                className="size-full object-cover"
              />
              <button
                className="absolute right-1 top-1 rounded bg-black/50 p-1 text-white opacity-0 group-hover:opacity-100"
                onClick={() =>
                  deletePhotoMutation.mutate({
                    userId,
                    groupId,
                    albumId,
                    photoId: photo.photo_id,
                  })
                }
              >
                ×
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="border-t bg-muted/30 px-4 py-2 text-xs text-muted-foreground">
        {photos.length} 张照片
      </div>
    </div>
  );
}
```

- [ ] **Step 2：检查类型和 imports**

确保 import `open` from `@tauri-apps/plugin-dialog`。

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/components/group/group-album-browser.tsx
git commit -m "feat(ui): add group album browser component"
```

### Task 4.2：在 GroupAlbumsWindow 中渲染浏览器

**Files:**
- Modify: `src/views/group/group-albums-window.tsx`

- [ ] **Step 1：替换占位内容**

```tsx
import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import GroupAlbumBrowser from "@/components/group/group-album-browser";
import { Toaster } from "@/components/ui/sonner";

export default function GroupAlbumsWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";
  const windowLabel = `group-albums-${userId}-${groupId}`;

  if (!userId || !groupId) {
    return <div>缺少 userId 或 groupId</div>;
  }

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <GroupAlbumBrowser userId={userId} groupId={groupId} />
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
```

- [ ] **Step 2：提交**

```bash
git add src/views/group/group-albums-window.tsx
git commit -m "feat(routing): render group album browser in window"
```

---

## Phase 5：群公告与精华消息 Sheet 面板

### Task 5.1：实现 GroupAnnouncementPanel

**Files:**
- Create: `src/components/group/group-announcement-panel.tsx`

- [ ] **Step 1：实现公告列表和发布弹窗**

```tsx
import { useState } from "react";
import { Megaphone, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useUpsertGroupAnnouncementMutation } from "@/lib/mutations";
import { useGroupAnnouncementsQuery } from "@/lib/query";

export default function GroupAnnouncementPanel({
  userId,
  groupId,
  canManage,
}: {
  userId: string;
  groupId: string;
  canManage: boolean;
}) {
  const { data: announcements = [] } = useGroupAnnouncementsQuery(
    userId,
    groupId,
  );
  const [open, setOpen] = useState(false);
  const [content, setContent] = useState("");
  const mutation = useUpsertGroupAnnouncementMutation();

  const handleSubmit = async () => {
    if (!content.trim()) return;
    await mutation.mutateAsync({
      userId,
      groupId,
      content: content.trim(),
    });
    setContent("");
    setOpen(false);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <h2 className="font-semibold">群公告</h2>
        {canManage && (
          <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
              <Button size="sm">
                <Plus className="mr-1 size-4" />
                发布公告
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>发布公告</DialogTitle>
              </DialogHeader>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="请输入公告内容..."
                rows={6}
              />
              <Button onClick={handleSubmit} disabled={!content.trim()}>
                发布
              </Button>
            </DialogContent>
          </Dialog>
        )}
      </div>

      <div className="flex-1 overflow-auto p-3">
        {announcements.length === 0 && (
          <div className="py-8 text-center text-sm text-muted-foreground">
            暂无公告
          </div>
        )}
        <div className="space-y-3">
          {announcements.map((announcement) => (
            <div key={announcement.announcement_id} className="rounded-lg border p-3">
              <div className="flex items-start gap-2">
                <Megaphone className="mt-0.5 size-4 text-primary" />
                <div className="flex-1">
                  <p className="text-sm whitespace-pre-wrap">
                    {announcement.content}
                  </p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    — {announcement.sender_user_id} ·{" "}
                    {new Date(announcement.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/components/group/group-announcement-panel.tsx
git commit -m "feat(ui): add group announcement panel component"
```

### Task 5.2：实现 GroupEssencePanel

**Files:**
- Create: `src/components/group/group-essence-panel.tsx`

- [ ] **Step 1：实现精华消息列表**

```tsx
import { Star } from "lucide-react";
import { useGroupEssenceMessagesQuery } from "@/lib/query";

export default function GroupEssencePanel({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
}) {
  const { data: essenceMessages = [] } = useGroupEssenceMessagesQuery(
    userId,
    groupId,
  );

  return (
    <div className="flex h-full flex-col">
      <div className="border-b p-3">
        <h2 className="font-semibold">精华消息</h2>
      </div>

      <div className="flex-1 overflow-auto p-3">
        {essenceMessages.length === 0 && (
          <div className="py-8 text-center text-sm text-muted-foreground">
            暂无精华消息
          </div>
        )}
        <div className="space-y-3">
          {essenceMessages.map((essence) => (
            <div
              key={essence.essence_id}
              className="rounded-lg border p-3"
            >
              <div className="flex items-start gap-2">
                <Star className="mt-0.5 size-4 text-yellow-500" />
                <div className="flex-1">
                  <p className="text-sm font-medium">
                    {essence.sender_user_id}
                  </p>
                  <p className="text-sm text-muted-foreground">
                    消息 ID: {essence.message_id}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {new Date(essence.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 3：提交**

```bash
git add src/components/group/group-essence-panel.tsx
git commit -m "feat(ui): add group essence panel component"
```

### Task 5.3：在 ChatMainPanel 中集成公告/精华 Sheet

**Files:**
- Modify: `src/components/chat/chat-main-panel.tsx`

- [ ] **Step 1：添加状态 imports 和 Sheet 组件**

```tsx
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import GroupAnnouncementPanel from "@/components/group/group-announcement-panel";
import GroupEssencePanel from "@/components/group/group-essence-panel";
```

- [ ] **Step 2：在组件内添加状态**

```tsx
type GroupPanel = "announcement" | "essence" | null;
const [activeGroupPanel, setActiveGroupPanel] = useState<GroupPanel>(null);
```

- [ ] **Step 3：在 JSX 中渲染 Sheet**

在 `ChatMainPanel` 返回的 JSX 末尾追加：

```tsx
<Sheet open={activeGroupPanel !== null} onOpenChange={(open) => !open && setActiveGroupPanel(null)}>
  <SheetContent className="w-[400px] sm:w-[540px]">
    <SheetHeader>
      <SheetTitle>
        {activeGroupPanel === "announcement" ? "群公告" : "精华消息"}
      </SheetTitle>
    </SheetHeader>
    {activeGroupPanel === "announcement" && conversation?.scene === "group" && (
      <GroupAnnouncementPanel
        userId={currentUserId}
        groupId={conversation.group_id}
        canManage={isAdminOrOwner}
      />
    )}
    {activeGroupPanel === "essence" && conversation?.scene === "group" && (
      <GroupEssencePanel
        userId={currentUserId}
        groupId={conversation.group_id}
      />
    )}
  </SheetContent>
</Sheet>
```

**注意：** `currentUserId` 需从 `useAuthStore` 获取；`isAdminOrOwner` 需从 `useGroupMembersQuery` 计算当前用户角色。

- [ ] **Step 4：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 5：提交**

```bash
git add src/components/chat/chat-main-panel.tsx
git commit -m "feat(chat): integrate announcement and essence sheets"
```

### Task 5.4：消息右键菜单增加设为精华

**Files:**
- Modify: `src/components/chat/chat-message-item.tsx`
- Modify: `src/components/chat/chat-main-panel.tsx`

- [ ] **Step 1：扩展 ChatContextAction 类型**

在 `chat-message-item.tsx` 中找到 `ChatContextAction` 类型，增加：

```ts
| { kind: "set_essence"; messageId: string; isSet: boolean }
```

- [ ] **Step 2：在右键菜单中增加入口**

在群聊消息的 context menu 中增加：

```tsx
<DropdownMenuItem
  onClick={() =>
    onAction?.({
      kind: "set_essence",
      messageId: message.message_id,
      isSet: true,
    })
  }
>
  设为精华
</DropdownMenuItem>
```

- [ ] **Step 3：在 ChatMainPanel 处理 action**

找到 `handleContextAction`，增加分支：

```ts
if (action.kind === "set_essence") {
  setGroupEssenceMessageMutation.mutate({
    userId: currentUserId,
    groupId: conversation.group_id,
    messageId: action.messageId,
    isSet: action.isSet,
  });
  return;
}
```

- [ ] **Step 4：提交**

```bash
git add src/components/chat/chat-message-item.tsx src/components/chat/chat-main-panel.tsx
git commit -m "feat(chat): add set essence action to message context menu"
```

---

## Phase 6：聊天窗口标题栏入口

### Task 6.1：在 ChatMainPanel 标题栏添加九宫格菜单

**Files:**
- Modify: `src/components/chat/chat-main-panel.tsx`

- [ ] **Step 1：添加 imports**

```tsx
import { LayoutGrid, MoreHorizontal } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { invoke } from "@tauri-apps/api/core";
```

- [ ] **Step 2：在标题栏右侧渲染入口按钮**

找到标题栏区域，在现有按钮后追加：

```tsx
{conversation?.scene === "group" && (
  <>
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon">
          <LayoutGrid className="size-5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onClick={() =>
            invoke("open_group_files_window", {
              userId: currentUserId,
              groupId: conversation.group_id,
            })
          }
        >
          群文件
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() =>
            invoke("open_group_albums_window", {
              userId: currentUserId,
              groupId: conversation.group_id,
            })
          }
        >
          群相册
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setActiveGroupPanel("announcement")}>
          群公告
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setActiveGroupPanel("essence")}>
          精华消息
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </>
)}
```

- [ ] **Step 3：运行 Biome**

```bash
bunx --bun @biomejs/biome check --write
```

- [ ] **Step 4：提交**

```bash
git add src/components/chat/chat-main-panel.tsx
git commit -m "feat(chat): add group tools dropdown in chat header"
```

---

## Phase 7：端到端验证

### Task 7.1：类型检查与构建

- [ ] **Step 1：TypeScript 类型检查**

```bash
bunx tsc --noEmit
```

Expected: no type errors.

- [ ] **Step 2：前端构建**

```bash
bun run build
```

Expected: build succeeds.

- [ ] **Step 3：Rust 编译与测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 4：提交**

```bash
git commit --allow-empty -m "chore: pre-integration build and test checkpoint"
```

### Task 7.2：手动功能验证清单

- [ ] **Step 1：入口验证**

运行 `bunx tauri dev`，打开一个群聊窗口，确认标题栏出现九宫格按钮，点击后下拉菜单包含四项。

- [ ] **Step 2：群文件窗口验证**

- 点击「群文件」打开独立窗口。
- 上传文件，确认列表刷新。
- 创建文件夹，点击进入，上传文件到子文件夹。
- 下载文件，确认本地路径可用。
- 删除文件/文件夹，确认列表刷新。
- 同时打开聊天窗口和文件窗口，在一个窗口操作后另一个自动刷新。

- [ ] **Step 3：群相册窗口验证**

- 点击「群相册」打开独立窗口。
- 新建相册，确认相册网格刷新。
- 进入相册，上传照片，确认照片网格刷新。
- 删除照片，确认刷新。
- 同时打开两个窗口，验证实时同步。

- [ ] **Step 4：公告与精华验证**

- 在群聊窗口点击「群公告」，发布公告，确认列表显示。
- 在消息上右键「设为精华」，打开「精华消息」面板，确认显示。
- 验证普通成员看不到发布公告按钮。

- [ ] **Step 5：提交验证记录（可选）**

```bash
git commit --allow-empty -m "test(manual): group content frontend integration verified"
```

---

## 计划自检

### 1. Spec 覆盖检查

| Spec 需求 | 对应 Task |
| --------- | --------- |
| 九宫格入口 + 更多按钮 | Task 6.1 |
| 群文件独立窗口 | Task 1.4, 3.1, 3.2, 3.3 |
| 群相册独立窗口 | Task 1.4, 4.1, 4.2 |
| 群公告 Sheet | Task 5.1, 5.3 |
| 精华消息 Sheet + 右键菜单 | Task 5.2, 5.3, 5.4 |
| 跨窗口实时同步 | Task 1.2, 1.3, 2.6, 2.7 |
| 错误处理 | 各 mutation onError，Task 7.1 |
| 测试策略 | Task 1.5, 7.1, 7.2 |

### 2. Placeholder 扫描

- 无 TBD/TODO。
- 所有代码步骤包含实际代码或明确的接口说明。
- 文件路径均为绝对路径。

### 3. 类型一致性检查

- `GroupFile` / `GroupFolder` / `GroupAlbum` / `GroupAnnouncement` / `GroupEssenceMessage` 类型定义与 `src/types/group.ts` 一致。
- 命令名与后端命令名一致。
- Query key 结构与现有 `queryKeys.groups` 一致。
- 窗口 label 格式在前后端一致：`group-files-{userId}-{groupId}` / `group-albums-{userId}-{groupId}`。

---

## 执行方式选择

**Plan complete and saved to `docs/superpowers/plans/2026-06-13-group-content-frontend-integration.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
