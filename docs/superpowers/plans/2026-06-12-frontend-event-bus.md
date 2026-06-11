# 多窗口事件投递修复 + 前端事件总线重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复多窗口聊天下的跨用户事件串扰，并将前端事件总线从模块级单例重构为窗口级 Provider + 组件级订阅 Hook。

**Architecture:** 后端 `core.rs` 将向聊天窗口推送事件的调用从全局 `emit` 改为定向 `emit_to`，配合前端从全局 `listen` 改为 webview-scoped `getCurrentWebviewWindow().listen`。前端新增 `ChatEventBusProvider`（每窗口一个，持有唯一 listener + subscriber set + 统一 query invalidation），`useChatEventBus` 重构为消费 Context 的订阅 hook。

**Tech Stack:** Tauri v2（Rust `Emitter` trait）、React 19、TypeScript、`@tauri-apps/api/webviewWindow`、TanStack Query。

**参考 spec:** `docs/superpowers/specs/2026-06-12-frontend-event-bus-design.md`

---

## File Structure

| 文件 | 操作 | 责任 |
|------|------|------|
| `src-tauri/src/core.rs` | 修改 | 推送事件用 `emit_to` 定向到窗口 label |
| `src/lib/query/event-handlers.ts` | 新建 | `handleQueryInvalidation(userId, payload)` —— 收到事件后的 query 刷新逻辑 |
| `src/lib/query/index.ts` | 修改 | 导出 `event-handlers` |
| `src/components/chat/chat-event-bus-provider.tsx` | 新建 | 窗口级 Provider：唯一 webview listener + subscriber set + Context |
| `src/hooks/use-chat-event-bus.ts` | 重构 | 移除模块级单例，改为消费 Context 的订阅 hook |
| `src/views/chat/chat-window.tsx` | 修改 | 用 `ChatEventBusProvider` 包裹子树 |
| `src/components/chat/conversation-list.tsx` | 修改 | 移除 `useChatEventBus(currentUserId)` 调用 |

**实现顺序说明：** 前端 listener 方式（webview-scoped）必须与后端 `emit_to` 同时生效，否则窗口收不到任何事件。因此后端改动（Task 1）和前端 listener 改动（Task 3）在最终合并时是一组，但可分别提交。中间任务（Task 2 抽取 invalidation）不影响运行时行为，先做。

---

## Task 1: 后端 `emit_to` 定向投递

**Files:**
- Modify: `src-tauri/src/core.rs:176-196`（`open_user_chat_window` 内的事件转发任务）

**背景：** 当前代码 `let _ = window.emit("chat:event", &event);` 调用的是 Tauri v2 `Emitter` trait 的**全局** emit，会被所有窗口的全局 `listen` 收到，造成跨用户串扰。改为 `app_handle.emit_to(&label, ...)` 只投递到目标窗口。`core.rs` 顶部已有 `use tauri::{Emitter, Manager};`（第 4 行），无需新增 import。

- [ ] **Step 1: 修改事件转发任务**

将 `src-tauri/src/core.rs` 中的转发任务（当前第 180-196 行）替换为：

```rust
        tauri::async_runtime::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // 窗口仍存在才投递；否则终止任务
                        if app_handle_for_events
                            .get_webview_window(&event_window_label)
                            .is_some()
                        {
                            let _ = app_handle_for_events.emit_to(
                                &event_window_label,
                                "chat:event",
                                &event,
                            );
                        } else {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });
```

说明：保留原有「窗口不存在则 break」逻辑；`emit_to` 不触发全局 `listen`，所以前端必须在 Task 3 改用 webview-scoped listen。

- [ ] **Step 2: 编译验证**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过，无警告（特别确认 `Emitter` trait 在作用域内、`window` 变量未变成未使用——本任务删掉了对 `window` 的使用，改用 `app_handle_for_events.get_webview_window`，确认没有遗留未使用变量警告）

- [ ] **Step 3: 格式化**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --`
Expected: 无输出

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/core.rs
git commit -m "fix(core): emit chat events to target window instead of global broadcast"
```

---

## Task 2: 抽取 `handleQueryInvalidation` 到独立文件

**Files:**
- Create: `src/lib/query/event-handlers.ts`
- Modify: `src/lib/query/index.ts:1-12`

**背景：** `handleQueryInvalidation` 当前内联在 `src/hooks/use-chat-event-bus.ts`（第 30-78 行）。它依赖 `sourceFromInternalEvent` 和多个 `invalidate*` 函数，全部来自 `@/lib/query`。把它移到 `lib/query/event-handlers.ts` 更内聚，供 Provider 导入。逻辑**原样迁移，不改行为**。

- [ ] **Step 1: 创建 `event-handlers.ts`**

Create `src/lib/query/event-handlers.ts`:

```ts
import type { InternalEventPayload } from "@/types/event";
import { sourceFromInternalEvent } from "@/lib/query/chat";
import {
  invalidateMessageHistoryQueries,
  invalidateMessageHistoryQuery,
  invalidatePokeHistoryQueries,
  invalidatePokeHistoryQuery,
} from "@/lib/query/chat";
import {
  invalidateFriendRequestsQuery,
  invalidateGroupRequestsQueries,
} from "@/lib/query/requests";
import { invalidateFriendsQuery } from "@/lib/query/friends";
import {
  invalidateGroupEventHistoryQuery,
  invalidateGroupsQuery,
} from "@/lib/query/groups";

/**
 * 收到 chat:event 后统一执行的 query 失效逻辑。
 * 由 ChatEventBusProvider 在每个窗口内调用一次。
 */
export function handleQueryInvalidation(
  userId: string,
  payload: InternalEventPayload,
) {
  const source = sourceFromInternalEvent(payload, userId);
  if (source) {
    invalidateMessageHistoryQuery(userId, source);
    invalidatePokeHistoryQuery(userId, source);
    if (source.scene === "group") {
      invalidateGroupEventHistoryQuery(userId, source.group_id);
    }
  } else {
    invalidateMessageHistoryQueries(userId);
    invalidatePokeHistoryQueries(userId);
  }

  if (
    payload.kind === "friend_request_created" ||
    payload.kind === "friend_request_handled" ||
    payload.kind === "group_request_created" ||
    payload.kind === "group_request_handled"
  ) {
    invalidateFriendRequestsQuery(userId);
    invalidateGroupRequestsQueries(userId);

    if (payload.kind === "friend_request_handled") {
      invalidateFriendsQuery(userId);
    }

    if (
      payload.kind === "group_request_handled" &&
      payload.state === "accepted"
    ) {
      const shouldRefreshGroups =
        payload.initiator_user_id === userId ||
        payload.target_user_id === userId;
      if (shouldRefreshGroups) {
        invalidateGroupsQuery();
      }
    }
  }

  if (
    payload.kind === "group_member_joined" &&
    payload.target_user_id === userId
  ) {
    invalidateGroupsQuery();
  }
}
```

> **执行者注意：** 上面的 import 来源已核实：`invalidateMessageHistoryQuery(ies)`、`invalidatePokeHistoryQuery(ies)`、`sourceFromInternalEvent` 在 `@/lib/query/chat`；`invalidateGroupEventHistoryQuery`、`invalidateGroupsQuery` 在 `@/lib/query/groups`；`invalidateFriendRequestsQuery`、`invalidateGroupRequestsQueries` 在 `@/lib/query/requests`；`invalidateFriendsQuery` 在 `@/lib/query/friends`。务必按具体子模块导入，**不要**从 `@/lib/query` 桶导入——`event-handlers.ts` 自身会被 `index.ts` re-export，从桶导入会引入循环依赖。

- [ ] **Step 2: 在 `index.ts` 导出**

Modify `src/lib/query/index.ts`，在 `chat` 导出之后新增一行（保持字母序在 `db` 之前）：

```ts
export * from "@/lib/query/bots";
export * from "@/lib/query/chat";
export * from "@/lib/query/common";
export * from "@/lib/query/db";
export * from "@/lib/query/event-handlers";
export * from "@/lib/query/friends";
export * from "@/lib/query/groups";
export * from "@/lib/query/keys";
export * from "@/lib/query/logs";
export * from "@/lib/query/packets";
export * from "@/lib/query/requests";
export * from "@/lib/query/users";
```

- [ ] **Step 3: 类型检查**

Run: `bunx tsc --noEmit`
Expected: 通过（此时 `use-chat-event-bus.ts` 内仍有自己的 `handleQueryInvalidation`，两处并存不冲突，因为本任务未删旧的）

- [ ] **Step 4: Commit**

```bash
git add src/lib/query/event-handlers.ts src/lib/query/index.ts
git commit -m "refactor(query): extract handleQueryInvalidation into event-handlers module"
```

---

## Task 3: 新建 `ChatEventBusProvider`

**Files:**
- Create: `src/components/chat/chat-event-bus-provider.tsx`

**背景：** Provider 是窗口级事件入口。它用 `getCurrentWebviewWindow().listen("chat:event")` 建立唯一 listener，收到事件后先调用一次 `handleQueryInvalidation`，再分发给所有 subscriber。通过 Context 暴露稳定的 `subscribe` 函数。

- [ ] **Step 1: 创建 Provider 与 Context**

Create `src/components/chat/chat-event-bus-provider.tsx`:

```tsx
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  type ReactNode,
} from "react";
import { handleQueryInvalidation } from "@/lib/query";
import type { InternalEventPayload } from "@/types/event";

export type ChatEventSubscriber = (payload: InternalEventPayload) => void;

type ChatEventBusContextValue = {
  subscribe: (callback: ChatEventSubscriber) => () => void;
};

const ChatEventBusContext = createContext<ChatEventBusContextValue | null>(
  null,
);

export function useChatEventBusContext(): ChatEventBusContextValue {
  const value = useContext(ChatEventBusContext);
  if (!value) {
    throw new Error(
      "useChatEventBus must be used within a ChatEventBusProvider",
    );
  }
  return value;
}

export function ChatEventBusProvider({
  userId,
  children,
}: {
  userId: string;
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

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    const webview = getCurrentWebviewWindow();
    webview
      .listen<InternalEventPayload>("chat:event", (event) => {
        const payload = event.payload;
        if (!payload) {
          return;
        }
        handleQueryInvalidation(userId, payload);
        for (const subscriber of subscribersRef.current) {
          subscriber(payload);
        }
      })
      .then((fn) => {
        if (cancelled) {
          // effect 已在 listen resolve 前 cleanup，立即解绑避免泄漏
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error("failed to listen chat:event", error);
      });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [userId]);

  return (
    <ChatEventBusContext.Provider value={{ subscribe }}>
      {children}
    </ChatEventBusContext.Provider>
  );
}
```

- [ ] **Step 2: 类型检查**

Run: `bunx tsc --noEmit`
Expected: 通过。若报 `@tauri-apps/api/webviewWindow` 找不到，确认包已安装：`grep getCurrentWebviewWindow node_modules/@tauri-apps/api/webviewWindow.d.ts`（应有该导出）。

- [ ] **Step 3: 格式化检查**

Run: `bunx --bun @biomejs/biome check --write src/components/chat/chat-event-bus-provider.tsx`
Expected: 无错误（可能自动调整 import 顺序）

- [ ] **Step 4: Commit**

```bash
git add src/components/chat/chat-event-bus-provider.tsx
git commit -m "feat(chat): add window-scoped ChatEventBusProvider"
```

---

## Task 4: 重构 `useChatEventBus` 为订阅 hook

**Files:**
- Modify: `src/hooks/use-chat-event-bus.ts`（整体替换）

**背景：** 移除模块级单例（`subscribers`、`activeUserId`、`activeUnlisten`、`setupPromise`）和内联的 `handleQueryInvalidation`。Hook 改为：从 Context 取 `subscribe`，注册 `onEvent` 回调。invalidation 已由 Provider 统一处理，hook 不再负责。

- [ ] **Step 1: 整体替换 `use-chat-event-bus.ts`**

将 `src/hooks/use-chat-event-bus.ts` 全部内容替换为：

```ts
import { useEffect, useRef } from "react";
import {
  type ChatEventSubscriber,
  useChatEventBusContext,
} from "@/components/chat/chat-event-bus-provider";
import type { InternalEventPayload } from "@/types/event";

/**
 * 订阅当前窗口的聊天事件，执行组件级回调。
 * Query 失效由 ChatEventBusProvider 统一处理，此 hook 只负责组件回调。
 *
 * @param userId 当前窗口用户 id；为空时不订阅。
 * @param onEvent 组件级回调；不传则不订阅。
 */
export function useChatEventBus(
  userId: string,
  onEvent?: (payload: InternalEventPayload) => void,
) {
  const { subscribe } = useChatEventBusContext();
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!userId || !onEvent) {
      return;
    }

    const subscriber: ChatEventSubscriber = (payload) => {
      onEventRef.current?.(payload);
    };

    return subscribe(subscriber);
    // onEvent 通过 onEventRef 间接调用，故不放进依赖；userId 保留用于 enabled 判断
  }, [userId, subscribe]);
}
```

> **执行者注意：** `onEvent` 通过 `onEventRef` 引用，依赖数组故意只含 `[userId, subscribe]`。Biome 的 `useExhaustiveDependencies` 可能告警。如告警，在该 `useEffect` 上方加 `// biome-ignore lint/correctness/useExhaustiveDependencies: onEvent accessed via ref` 注释，与项目现有忽略风格一致（先 `grep -rn "biome-ignore" src/` 确认项目用法）。

- [ ] **Step 2: 类型检查**

Run: `bunx tsc --noEmit`
Expected: 通过

- [ ] **Step 3: Commit**

```bash
git add src/hooks/use-chat-event-bus.ts
git commit -m "refactor(chat): make useChatEventBus a context-based subscription hook"
```

---

## Task 5: `ChatWindowView` 包裹 Provider

**Files:**
- Modify: `src/views/chat/chat-window.tsx:36-60`

**背景：** Provider 必须包裹会订阅事件的子树（`ConversationList` + `ChatMainPanel`）。`ChatWindowView` 已在 `currentUserId` 为 `null` 时返回 `null`（第 32-34 行），所以 Provider 渲染时 `currentUserId` 一定非空。

- [ ] **Step 1: 包裹 Provider**

修改 `src/views/chat/chat-window.tsx`，在文件顶部 import 区新增：

```tsx
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
```

将 `return (...)` 中的 `<main>...</main>` 用 Provider 包裹：

```tsx
  return (
    <ChatEventBusProvider userId={currentUserId}>
      <main className="flex h-screen w-screen overflow-hidden bg-background">
        <ResizablePanelGroup orientation="horizontal" className="flex-1">
          <ResizablePanel defaultSize={240} minSize={200} maxSize={280}>
            <ConversationList
              onSelectedConversationChange={setSelectedConversation}
            />
          </ResizablePanel>

          <ResizableHandle />

          <ResizablePanel>
            {selectedConversation ? (
              <ChatMainPanel conversation={selectedConversation} />
            ) : (
              <div className="flex h-full items-center justify-center text-muted-foreground text-sm">
                请选择一个会话开始聊天
              </div>
            )}
          </ResizablePanel>
        </ResizablePanelGroup>

        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
```

- [ ] **Step 2: 类型检查**

Run: `bunx tsc --noEmit`
Expected: 通过

- [ ] **Step 3: Commit**

```bash
git add src/views/chat/chat-window.tsx
git commit -m "feat(chat): wrap chat window subtree with ChatEventBusProvider"
```

---

## Task 6: `ConversationList` 移除 `useChatEventBus` 调用

**Files:**
- Modify: `src/components/chat/conversation-list.tsx:34,114`

**背景：** `ConversationList` 原来调用 `useChatEventBus(currentUserId)`（无 `onEvent`），仅为触发 listener 建立和 query 失效。重构后 listener 由 Provider 建立、invalidation 由 Provider 统一处理，此调用多余，移除。

- [ ] **Step 1: 移除 import 与调用**

删除 `src/components/chat/conversation-list.tsx` 第 34 行：

```tsx
import { useChatEventBus } from "@/hooks/use-chat-event-bus";
```

删除第 114 行：

```tsx
  useChatEventBus(currentUserId);
```

- [ ] **Step 2: 类型检查 + lint**

Run: `bunx tsc --noEmit && bunx --bun @biomejs/biome check src/components/chat/conversation-list.tsx`
Expected: 通过，无「未使用 import」残留

- [ ] **Step 3: Commit**

```bash
git add src/components/chat/conversation-list.tsx
git commit -m "refactor(chat): drop redundant useChatEventBus call in ConversationList"
```

---

## Task 7: 全量构建与手动验证

**Files:** 无（验证任务）

- [ ] **Step 1: 前端构建**

Run: `bun run build`
Expected: 构建成功，无类型错误

- [ ] **Step 2: 后端构建**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功

- [ ] **Step 3: Biome 全量检查**

Run: `bunx --bun @biomejs/biome check --write`
Expected: 无错误（如有自动修复，re-stage）

- [ ] **Step 4: 启动 dev 手动验证**

Run: `bunx tauri dev`

逐项验证（spec 验证清单）：
1. 打开用户 A 聊天窗口，发送/接收消息 → 本窗口收到事件、消息列表刷新
2. 同时打开用户 B 聊天窗口 → 给 A 发事件时**只有 A 窗口**刷新，B 窗口无反应（验证串扰消除）
3. 关闭用户 A 窗口 → 用户 B 窗口仍正常接收自己的事件
4. 在一个窗口内切换会话 → 事件分发和 query 刷新正常

Expected: 全部符合。重点是第 2 项——这是本次修复的核心验收点。

- [ ] **Step 5: 若验证通过，最终确认提交**

```bash
git status
git log --oneline -7
```
Expected: 工作区干净，7 个任务的提交均在分支上。

---

## 实现完成标准

- [ ] `core.rs` 用 `emit_to(&label, "chat:event", ...)` 定向投递
- [ ] `handleQueryInvalidation` 迁移到 `src/lib/query/event-handlers.ts`
- [ ] `ChatEventBusProvider` 用 `getCurrentWebviewWindow().listen` 建立窗口级唯一 listener
- [ ] `useChatEventBus` 不再含模块级单例、不再直接 `listen`
- [ ] `ChatWindowView` 用 Provider 包裹子树
- [ ] `ConversationList` 不再调用 `useChatEventBus`
- [ ] 多窗口手动验证：跨用户串扰消除
