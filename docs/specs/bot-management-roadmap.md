# Bot 管理 Roadmap

本文档记录 Bot 管理模块已确认的技术决策和未来改进方向，防止后续遗忘。

---

## 已确认的技术决策

### 1. start_bot 竞态处理模式

采用"先尝试原子操作，失败后再诊断原因"的模式：

- `start_session` 使用 `UPDATE ... WHERE runtime_status != 'running' AND NOT EXISTS (...)` 保证原子性
- `RowNotFound` 后通过 `get_bot_by_id` 区分：bot 不存在 vs bot 已在运行
- 避免了额外的预检查查询，消除了竞态窗口

### 2. stop_bot 保留预检查（不照搬 start_bot 模式）

虽然审查建议简化 `stop_bot` 的预检查，但经分析**保留当前实现**：

- `stop_active_sessions` 的 `UPDATE bots SET ... WHERE bot_id = ?1` 只带主键条件
- 其 `RowNotFound` 仅能表示"bot 不存在"，无法区分"bot 未运行"
- SQLite 在值未改变时 `rows_affected()` 行为不确定，对已停止的 bot 可能错误返回 `RowNotFound`
- stop 是低频手动操作，多一次查询的开销可忽略

### 3. 删除 bot 时的文件清理失败不阻断流程

文件系统清理失败时：

- 数据库记录已正确删除（事务内）
- 文件清理失败仅 `eprintln!` 日志输出，不阻断流程
- 避免"孤儿文件"导致用户无法删除 bot

---

## 待办：批量操作支持

### 背景

当前 `DashboardView` 中 BotCard 的 pending 状态是全局的：

```tsx
// src/views/main/dashboard.tsx
<BotCard
  isStartPending={startBot.isPending}
  isStopPending={stopBot.isPending}
/>
```

当任一 bot 正在启动/停止时，**所有** bot 的对应按钮都会被禁用。这是防止并发冲突的安全设计。

### 改进方向

如果未来需要支持批量操作（如一键启动/停止多个 bot），需要改为 per-bot 的 pending 状态：

```tsx
// 可能的未来实现
<BotCard
  isStartPending={pendingBots.has(bot.bot_id)}
  isStopPending={stoppingBots.has(bot.bot_id)}
/>
```

### 影响范围

- **Frontend**: `useStartBotMutation` / `useStopBotMutation` 需支持批量调用或独立的 per-bot pending tracking
- **Backend**: `start_session` 的原子 `UPDATE` 天然支持并发，多个 bot 同时启动无竞态问题
- **UX**: 需设计批量操作的反馈机制（进度、部分失败处理、撤销）

### 优先级

低 —— 当前单 bot 操作是主要使用场景，全局 pending 的保护机制足够。
