# 多窗口事件投递修复 + 前端事件总线重构设计

## 背景

UniBot 为每个用户的聊天窗口创建一个独立的 Tauri webview 窗口（`chat-{user_id}`，见 `src-tauri/src/core.rs` 的 `open_user_chat_window`），加载 `index.html#/chat/{user_id}`。每个窗口是独立的 JS 运行时，因此 `src/hooks/use-chat-event-bus.ts` 里的模块级单例（`subscribers`、`activeUnlisten` 等）**不跨窗口共享**——这一点与早期推测相反，不存在“后开窗口覆盖前一个 listener”的问题。

真正的 bug 在事件投递语义：后端 `core.rs` 中向窗口推送事件用的是

```rust
let _ = window.emit("chat:event", &event);
```

`WebviewWindow::emit` 实现的是 `Emitter` trait 的**全局** `emit`（Tauri v2 官方文档：“Emits an event to all targets” / “Global events are delivered to **all** listeners”）。而前端各窗口用的是 `@tauri-apps/api/event` 的全局 `listen("chat:event")`。结果是：

- 窗口 A（user-1）后端任务收到 user-1 的事件后全局广播
- 窗口 A **和** 窗口 B（user-2）的 `listen("chat:event")` **都**收到这条 user-1 的事件
- 窗口 B 用自己的 `userId="user-2"` 去执行 `handleQueryInvalidation`，错误刷新 user-2 的 query，组件级回调（`chat-main-panel.tsx`）也被错误 payload 触发

即多窗口下发生**跨用户事件串扰**。

## 目标

- **根因修复**：后端按窗口定向投递事件，前端按窗口监听，消除跨用户串扰
- **结构重构**（附带）：明确事件总线分层——Provider 负责窗口级监听 + query 刷新，Hook 负责组件级回调订阅，移除模块级单例
- 保持现有 `useChatEventBus(userId, onEvent?)` 调用签名不变，减少调用方改动
- 与 Tauri 多窗口模型对齐：每个窗口独立监听、独立清理

## 参考

- Tauri v2 事件模型：`emit` 全局广播 / `emit_to(label, ...)` 定向到单个窗口且**不触发**常规全局 `listen()`；前端用 `getCurrentWebviewWindow().listen()` 接收 webview-targeted 事件
- VS Code / Electron 多窗口：主进程按窗口路由事件，每个 renderer 独立建立 IPC 监听，窗口关闭时自行清理，窗口内多个组件通过 local pub/sub 共享同一个 listener

## 设计

整体分两部分：**后端定向投递**（修 bug，必需）+ **前端事件总线重构**（结构改进 + 配合后端监听方式调整）。

### 第一部分：后端定向投递（根因）

将 `core.rs` 中向聊天窗口推送事件的调用从全局 `emit` 改为定向 `emit_to`：

```rust
// 当前（全局广播，导致串扰）
let _ = window.emit("chat:event", &event);

// 改为（定向到该窗口）
let _ = app_handle_for_events.emit_to(&event_window_label, "chat:event", &event);
```

要点：

- `emit_to(label, ...)` 只投递给 label 对应窗口，且**不会**触发常规全局 `listen()`
- 需要 `use tauri::Emitter;` 在作用域内（`emit_to` 是 `Emitter` trait 方法）
- 仍保留“窗口不存在则 `break` 终止任务”的逻辑：先 `get_webview_window(&event_window_label)` 判断窗口是否还在，再 `emit_to`

因为 `emit_to` 不触发全局 `listen`，**前端必须改用 webview-scoped 监听**（见下文 Provider）。

### 第二部分：前端事件总线重构

#### 架构

```
ChatWindowView (每个 Tauri 窗口一个 React tree)
└── ChatEventBusProvider userId={currentUserId}
    ├── 维护 1 个 webview-scoped listener
    │   getCurrentWebviewWindow().listen("chat:event", handler)
    ├── 收到事件后：
    │   ├── 调用 handleQueryInvalidation(userId, payload) 一次
    │   └── 遍历 subscribers，分发 payload
    └── ConversationList / ChatMainPanel 通过 useChatEventBus 订阅
```

#### 关键文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/core.rs` | 修改：`emit` → `emit_to`，并 `use tauri::Emitter;` |
| `src/components/chat/chat-event-bus-provider.tsx` | 新建：窗口级 Provider，用 `getCurrentWebviewWindow().listen` 管理 listener 和 subscribers |
| `src/lib/query/event-handlers.ts` | 新建：抽出 `handleQueryInvalidation` 逻辑 |
| `src/hooks/use-chat-event-bus.ts` | 重构：移除全局状态，改为基于 Context 的订阅 hook |
| 组件 `ChatWindowView`（`src/views/chat/chat-window.tsx`） | 用 `ChatEventBusProvider` 包裹子树 |
| `src/components/chat/conversation-list.tsx` | 移除 `useChatEventBus(currentUserId)` 调用 |

#### Provider API

```tsx
type ChatEventBusContextValue = {
  subscribe: (callback: ChatEventSubscriber) => () => void;
};

function ChatEventBusProvider({
  userId,
  children,
}: {
  userId: string;
  children: React.ReactNode;
});
```

Provider 行为：

1. `useEffect(() => { ... }, [userId])` 中调用 `getCurrentWebviewWindow().listen<InternalEventPayload>("chat:event", handler)`
2. Handler 内部：
   - 调用 `handleQueryInvalidation(userId, payload)`
   - 遍历 `subscribers` Set，调用每个 callback
3. Cleanup 中调用 `unlisten()`
4. 当 `userId` 变化时，先 unlisten 旧 listener，再 listen 新 listener
5. `subscribe` 函数返回 `unsubscribe`，用于组件清理；`subscribe` 用 `useCallback` 保持稳定引用
6. `listen` 是异步的：用一个标志位避免组件在 listen resolve 前已 unmount 时的悬挂监听（resolve 后若已失效立即 `unlisten()`）

#### Hook API

签名保持不变：

```ts
export function useChatEventBus(
  userId: string,
  onEvent?: (payload: InternalEventPayload) => void,
);
```

实现改为：

1. 从 `ChatEventBusContext` 读取 `subscribe`
2. 使用 `useRef` 保存 `onEvent` 最新引用，避免因回调引用变化反复 resubscribe
3. `useEffect` 中：若 `userId` 非空且 `onEvent` 存在，则 subscribe
4. Cleanup 时 unsubscribe

`userId` 参数保留用于 enabled 判断；实际 listener 由 Provider 按窗口管理。

#### Query Invalidation

将 `handleQueryInvalidation` 从 hook 文件移到 `src/lib/query/event-handlers.ts`，由 Provider 导入调用。原因：与 query invalidation 函数高度相关，放在 `lib/query/` 更内聚；逻辑独立可测试；Hook 文件只关心 Context 消费。

逻辑本身原样迁移（`sourceFromInternalEvent` + 各 `invalidate*` 调用），不改变行为。

#### 调用方调整

- `ChatMainPanel`：继续使用 `useChatEventBus(currentUserId, (payload) => { ... })`（自带 source 过滤逻辑，保持不变）
- `ConversationList`：移除 `useChatEventBus(currentUserId)` 调用——它没有组件级回调，只为触发 listener 建立；重构后 listener 由 Provider 统一建立
- 组件 `ChatWindowView`：在 `currentUserId` 有效后，用 `ChatEventBusProvider` 包裹 `ConversationList` + `ChatMainPanel` 子树

### 生命周期

#### 窗口打开

1. 后端 `open_user_chat_window` 创建窗口，启动向该窗口 `emit_to` 事件的任务
2. `ChatWindowView` mount，设置 `currentUserId`
3. `ChatEventBusProvider` mount，建立 `getCurrentWebviewWindow().listen("chat:event")` 监听

#### 事件到达

```
后端 emit_to("chat-{user}", "chat:event", payload)
    ↓ (仅投递到该窗口，不触发全局 listen)
该窗口 getCurrentWebviewWindow().listen handler（Provider 内）
    ├── handleQueryInvalidation(userId, payload)  // 一次
    └── subscribers.forEach(cb => cb(payload))     // ChatMainPanel 等
```

#### 窗口关闭

1. `ChatWindowView` unmount
2. `ChatEventBusProvider` cleanup 调用 `unlisten()`
3. 后端 `WindowEvent::Destroyed` 清理 `UserContext.chat_window_label`；推送任务下次 `get_webview_window` 拿不到窗口而 `break`

#### 多窗口场景（修复后）

- 窗口 A（user-1）和窗口 B（user-2）各自有独立 Provider，各自 webview-scoped 监听
- 后端用 `emit_to` 把 user-1 事件**只**投递到窗口 A，user-2 事件**只**投递到窗口 B
- 窗口 B 不再收到 user-1 的事件，跨用户串扰消除

## 边界情况

### userId 变化

`ChatEventBusProvider` 监听 `userId` prop 变化，旧 listener 先 `unlisten()`，再创建新 listener。窗口场景下 `userId` 基本不变（一个窗口固定一个 userId），但作为 defensive 处理保留。

### 空 userId

Provider 只在 `userId` 非空时建立监听。`ChatWindowView` 已在 `currentUserId` 无效时返回 `null`，Provider 内部再做一次防御判断。

### listen 异步与快速卸载

`getCurrentWebviewWindow().listen` 返回 Promise。effect 内用局部 `cancelled` 标志，listen resolve 后若 effect 已 cleanup 则立即调用返回的 `unlisten`，避免监听泄漏。

### listen 失败

若 `listen()` reject，Provider 记录 `console.error`，子树仍正常渲染。当前不需要向 UI 暴露 `isListening` 状态。

### 同一窗口多个订阅者

`ConversationList` 和 `ChatMainPanel` 同时存在，只有 `ChatMainPanel` 订阅组件级回调；`ConversationList` 不再订阅。Query invalidation 由 Provider 统一处理一次，不会重复。

## 验证

### 编译检查

```bash
bun run build
cargo build --manifest-path src-tauri/Cargo.toml
```

### 手动验证

1. 打开用户 A 的聊天窗口，发送/接收消息 → 本窗口收到事件、消息列表刷新
2. 同时打开用户 B 的聊天窗口 → 给 A 发事件时，**只有 A 窗口**刷新，B 窗口无反应（验证串扰已消除）
3. 关闭用户 A 的窗口 → 用户 B 窗口仍正常接收自己的事件
4. 在一个窗口内切换会话 → 事件分发和 query 刷新正常

### Code Review 要点

- `core.rs` 用 `emit_to(&event_window_label, ...)` 而非全局 `emit`，且 `use tauri::Emitter;` 已引入
- Provider 用 `getCurrentWebviewWindow().listen`（webview-scoped），不是 `@tauri-apps/api/event` 的全局 `listen`
- `use-chat-event-bus.ts` 中没有模块级 listener 状态残留，`useChatEventBus` 不再直接调用 `listen()`
- `ConversationList` 中没有 `useChatEventBus` 调用
- Provider 的 `subscribe` 使用稳定引用（`useCallback`）
- Provider 在 unmount 时一定调用 `unlisten()`，且处理 listen 异步卸载竞态

## 风险

| 风险 | 说明 | 缓解 |
|---|---|---|
| 前端监听方式必须配合后端改动 | `emit_to` 不触发全局 `listen`；若只改后端不改前端，所有窗口都收不到事件 | 后端 `emit_to` 与前端 `getCurrentWebviewWindow().listen` 必须同一次改动一起上线，验证步骤 1 即可捕获 |
| Provider 位置/层级 | 未来主窗口若需监听聊天事件需额外 Provider | 当前主窗口不监听，暂不处理 |
| `listen()` 在测试环境不可用 | Tauri API 单元测试难 mock | 本次以编译检查 + 手动验证为主 |
| `userId` 变化时短暂 listener 空缺 | 切换 userId 时旧 listener 已关、新 listener 未建 | 窗口场景 userId 不变，可接受 |

## 后续可选优化

- 若组件订阅变多，可把 `subscribers` 按事件 kind 分桶，减少无意义回调
- 若主窗口需监听聊天事件，可复用 `ChatEventBusProvider`
