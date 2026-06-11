# 前端事件总线重构设计

## 背景

当前 `src/hooks/use-chat-event-bus.ts` 使用模块级单例（`activeUserId`、`activeUnlisten`、`subscribers`）管理 Tauri `chat:event` 事件监听。当多个聊天窗口同时打开时，后打开的窗口会覆盖前一个窗口的 listener，导致只有最后一个窗口能收到后端事件。这是一个影响多窗口聊天功能的基础 bug。

## 目标

- 修复多窗口下事件监听被覆盖的问题
- 明确事件总线各层职责：Provider 负责窗口级监听 + query 刷新，Hook 负责组件级回调订阅
- 保持现有 `useChatEventBus(userId, onEvent?)` 调用签名不变，减少调用方改动
- 与 Tauri 多窗口模型对齐：每个窗口独立监听、独立清理

## 参考

参考 VS Code / Electron 多窗口应用的事件模型：

- 后端/主进程按窗口路由事件
- 每个窗口（webview/renderer）独立建立 IPC 监听
- 窗口关闭时自行清理监听和订阅
- 同一窗口内多个组件通过 local pub/sub 共享同一个 listener

## 设计

### 架构

```
ChatWindowView (每个 Tauri 窗口一个 React tree)
└── ChatEventBusProvider userId={currentUserId}
    ├── 维护 1 个 Tauri listener("chat:event")
    ├── 收到事件后：
    │   ├── 调用 handleQueryInvalidation(userId, payload) 一次
    │   └── 遍历 subscribers，分发 payload
    └── ConversationList / ChatMainPanel 通过 useChatEventBus 订阅
```

### 关键文件

| 文件 | 职责 |
|------|------|
| `src/components/chat/chat-event-bus-provider.tsx` | 新建：窗口级 Provider，管理 listener 和 subscribers |
| `src/hooks/use-chat-event-bus.ts` | 重构：移除全局状态，改为基于 Context 的订阅 hook |
| `src/lib/query/event-handlers.ts` | 新建：抽出 `handleQueryInvalidation` 逻辑 |
| `src/views/chat/chat-window.tsx` | 包裹 `ChatEventBusProvider` |
| `src/components/chat/conversation-list.tsx` | 移除 `useChatEventBus(currentUserId)` 调用 |

### Provider API

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

1. `useEffect(() => { ... }, [userId])` 中调用 `listen("chat:event", handler)`
2. Handler 内部：
   - 调用 `handleQueryInvalidation(userId, payload)`
   - 遍历 `subscribers` Set，调用每个 callback
3. Cleanup 中调用 `unlisten()`
4. 当 `userId` 变化时，先 unlisten 旧 listener，再 listen 新 listener
5. `subscribe` 函数返回 `unsubscribe`，用于组件清理

### Hook API

签名保持不变：

```ts
export function useChatEventBus(
  userId: string,
  onEvent?: (payload: InternalEventPayload) => void,
);
```

实现改为：

1. 从 `ChatEventBusContext` 读取 `subscribe`
2. 使用 `useRef` 保存 `onEvent` 最新引用
3. `useEffect` 中：如果 `userId` 和 `onEvent` 存在，则 subscribe
4. Cleanup 时 unsubscribe

`userId` 参数仍然保留，用于 enabled 判断；实际 listener 由 Provider 按窗口管理。

### Query Invalidation

将 `handleQueryInvalidation` 从 hook 文件移到 `src/lib/query/event-handlers.ts`。

原因：

- 它与 query invalidation 函数高度相关，放在 `lib/query/` 更内聚
- Provider 直接导入调用，逻辑独立可测试
- Hook 文件只关心 Context 消费

### 调用方调整

- `ChatMainPanel`：继续使用 `useChatEventBus(currentUserId, (payload) => { ... })`
- `ConversationList`：移除 `useChatEventBus(currentUserId)` 调用，因为它没有组件级回调
- `ChatWindowView`：用 `ChatEventBusProvider` 包裹整个子树

### 生命周期

#### 窗口打开

1. 后端 `open_user_chat_window` 创建窗口，启动向该窗口 emit 事件的任务
2. `ChatWindowView` mount，设置 `currentUserId`
3. `ChatEventBusProvider` mount，建立 `chat:event` 监听

#### 事件到达

```
Tauri chat:event
    ↓
ChatEventBusProvider handler
    ├── handleQueryInvalidation(userId, payload)  // 一次
    └── subscribers.forEach(cb => cb(payload))     // ChatMainPanel 等
```

#### 窗口关闭

1. `ChatWindowView` unmount
2. `ChatEventBusProvider` cleanup 调用 `unlisten()`
3. 后端 `WindowEvent::Destroyed` 清理 `UserContext.chat_window_label`

#### 多窗口场景

- 窗口 A（user-1）和窗口 B（user-2）各自有独立 Provider
- 两个 Provider 各自 `listen("chat:event")`
- 后端把 user-1 事件 emit 到窗口 A，user-2 事件 emit 到窗口 B
- 两个窗口都能正常接收，互不覆盖

## 边界情况

### userId 变化

`ChatEventBusProvider` 监听 `userId` prop 变化，旧 listener 先 `unlisten()`，再创建新 listener。窗口场景下 `userId` 基本不变，但这是 defensive 处理。

### 空 userId

Provider 只在 `userId` 非空时建立监听。`ChatWindowView` 中可以在 `currentUserId` 有效后再渲染 Provider，或者由 Provider 内部判断。

### listen 失败

如果 `listen()` 抛异常，Provider 记录 `console.error`，子树仍然渲染。当前不需要暴露 `isListening` 状态给 UI。

### 同一窗口多个订阅者

`ConversationList` 和 `ChatMainPanel` 可能同时存在。只有 `ChatMainPanel` 会订阅组件级回调；`ConversationList` 不再订阅。Query invalidation 由 Provider 统一处理一次，不会重复。

## 验证

### 编译检查

```bash
bun run build
```

### 手动验证

1. 打开用户 A 的聊天窗口，发送消息 → 本窗口收到事件、消息列表刷新
2. 再打开用户 B 的聊天窗口 → 两个窗口都能独立收到各自的事件
3. 关闭用户 A 的窗口 → 用户 B 窗口仍然正常接收
4. 在一个窗口内切换会话 → 事件分发和 query 刷新正常

### Code Review 要点

- `use-chat-event-bus.ts` 中没有模块级 listener 状态残留
- `useChatEventBus` 不再直接调用 `listen()`
- `ConversationList` 中没有 `useChatEventBus` 调用
- Provider 的 `subscribe` 使用稳定引用
- Provider 在 unmount 时一定调用 `unlisten()`

## 风险

| 风险 | 说明 | 缓解 |
|---|---|---|
| Provider 位置/层级问题 | 如果未来主窗口也需要监听事件，需要额外 Provider | 当前主窗口不监听聊天事件，暂不处理 |
| `listen()` 在测试环境不可用 | Tauri API 在单元测试中难 mock | 本次以编译检查和手动验证为主 |
| `userId` 变化时短暂 listener 空缺 | 切换 userId 时旧 listener 已关、新 listener 未建 | 窗口场景 userId 不变，可接受 |

## 后续可选优化

- 如果后续组件订阅变多，可以考虑把 `subscribers` 改成按事件 kind 分桶，减少无意义回调
- 如果需要主窗口监听聊天事件，可以复用 `ChatEventBusProvider`
