# 开发者工具窗口 UI 重设计

## 目标

统一开发者工具窗口四个面板（日志、事件流、数据库、SQL）的布局与交互语言，解决当前存在的滚动失效、结果不可见、焦点样式突兀、筛选器体验不一致等问题，使其与主界面日志风格对齐。

## 背景与问题

当前 `feat/developer-mode-pr2` 分支上的开发者工具窗口存在以下问题：

1. **日志面板**：根容器缺少 `min-h-0`，当日志条目较多时整个面板被内容撑开，滚动不生效，底部分页被挤出可视区域。
2. **数据库面板**：左侧表列表与右侧表详情区域都未形成稳定的滚动容器，表数量或表结构较长时无法滚动。
3. **SQL 面板**：执行结果直接显示在输入框下方，但缺乏明确的视觉分区，用户难以感知结果已返回。
4. **焦点阴影**：日志面板搜索 Input、Select 等组件的焦点环/阴影与项目主界面日志风格不一致，显得突兀。
5. **日志筛选器**：当前使用 `Select` 下拉切换 level（ALL / TRACE / DEBUG …），下拉展开时会撑开/移动窗口；应复用主界面日志的 Combobox 多选弹窗逻辑。

## 设计原则

- **统一骨架**：所有面板采用“头部工具栏 + 可滚动内容区 + 可选底部栏”结构。
- **明确滚动边界**：每个可滚动区域都有独立、显式的 `overflow-auto` 容器，并配合 `min-h-0` / `flex-1` 限制尺寸。
- **结果可见**：SQL 面板采用上下分栏，结果区固定且独立滚动。
- **风格一致**：焦点环、卡片背景、筛选器全部复用主界面日志已验证的组件与样式。
- **保留事件订阅**：事件流面板继续通过 `forceMount` 保持监听，但非激活时隐藏。

## 详细设计

### 1. 通用面板骨架

每个面板组件内部结构统一为：

```tsx
<div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
  {/* 头部工具栏：筛选、搜索、操作按钮 */}
  <div className="shrink-0 ...">...</div>

  {/* 可滚动内容区 */}
  <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
    ...
  </div>

  {/* 可选底部栏：分页、统计 */}
  <div className="shrink-0 ...">...</div>
</div>
```

### 2. 日志面板

布局变更：

- 根容器改为 `flex h-full min-h-0 flex-col gap-3 overflow-hidden`。
- 顶部筛选栏使用 `Combobox` 多选组件（等级、事件类型、时间范围），与 `src/views/main/logs.tsx` 保持一致。
- 中部日志列表放入 `min-h-0 flex-1 overflow-auto` 容器。
- 底部分页栏始终可见，不再被内容推出可视区域。
- 单条日志卡片样式与主界面日志的 `LogRow` 一致：时间、level badge、target、可展开消息。

数据结构复用现有 `useSystemLogsQuery`，分页继续按 `PAGE_SIZE = 100` 客户端分页。

### 3. 事件流面板

布局变更：

- 统一为通用骨架。
- kind 筛选从普通 `Input` 升级为 `Combobox` 多选（预留多 kind 筛选能力）。
- 事件列表放入独立滚动容器。
- 保留 `forceMount` 与后台监听；`TabsContent` 已添加 `data-[state=inactive]:hidden` 避免非激活时占位。

### 4. 数据库面板

布局变更：

- 根容器 `flex h-full min-h-0 gap-4`。
- 左侧表列表：`flex h-full w-56 flex-col overflow-hidden rounded-xl border bg-card`，内部内容区 `flex-1 overflow-auto`。
- 右侧详情：`min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-4`。
- 表结构表格、索引卡片、DDL、数据预览均复用 `overflow-auto` 子容器，避免单张大表把整栏撑开。

### 5. SQL 面板

布局变更：

- 根容器 `flex h-full min-h-0 flex-col gap-3 overflow-hidden`。
- 上半部分固定：`textarea` + 执行按钮 + “允许写操作”开关。
- 下半部分为结果面板：`min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3`。
- 结果面板内依次展示：
  - 错误提示（红色条）。
  - 无数据提示 / 受影响行数。
  - 结果表格（带横向滚动）。
- 执行中按钮显示“执行中...”。

### 6. 焦点样式统一

- 不再在局部覆盖 `ring` / `shadow`。
- `Input`、`SelectTrigger`、`Button` 直接使用项目 `@/components/ui` 默认样式（已与主界面日志一致）。
- 若默认组件焦点环仍显突兀，则去 `@/components/ui/input.tsx` / `select.tsx` 统一调整，而不是在每个面板里修修补补。

### 7. TabsContent 隐藏修复（已落地）

`src/components/ui/tabs.tsx` 的 `TabsContent` 已增加 `data-[state=inactive]:hidden`，确保 `forceMount` 的面板在非激活状态下不占据可视空间。

## 文件变更

- `src/views/dev-tools/logs-panel.tsx`：重构成通用骨架 + Combobox 筛选 + 固定分页。
- `src/views/dev-tools/events-panel.tsx`：重构成通用骨架 + Combobox kind 筛选。
- `src/views/dev-tools/schema-panel.tsx`：重构成稳定双栏滚动布局。
- `src/views/dev-tools/sql-panel.tsx`：重构成上下分栏，底部结果面板。
- `src/components/ui/tabs.tsx`：已修复 inactive 隐藏（无需再改）。
- 可选：`src/components/ui/input.tsx`、`select.tsx`：若焦点环需要全局微调则一起改。

## 测试计划

1. **手动验证**：
   - 打开开发者工具窗口，分别切换到四个 tab。
   - 日志：生成/搜索超过 100 条日志，验证列表可滚动且分页始终可见。
   - 数据库：验证表列表可滚动，点击长表后右侧详情可独立滚动。
   - SQL：执行 SELECT / INSERT / 错误 SQL，验证结果/错误在底部面板显示。
   - 焦点：点击搜索框、Select，确认焦点环与主界面日志一致。
   - 事件流：切到事件流 tab，产生事件，再切到数据库/SQL，确认事件流仍在后台接收但不遮挡。

2. **自动检查**：
   - `npm run build` TypeScript 编译通过。
   - `cargo test --lib commands::dev_tools` 通过（SQL 相关逻辑不受影响）。

## 验收标准

- [ ] 日志面板滚动正常，分页始终可见。
- [ ] 数据库左右两栏均可独立滚动。
- [ ] SQL 执行结果明确显示在底部结果区，错误信息可见。
- [ ] 搜索框、Select 等焦点样式与主界面日志一致，无突兀阴影。
- [ ] 日志 level 筛选采用 Combobox 多选弹窗，不再撑开窗口。
- [ ] 事件流面板在切换到其他 tab 后仍继续接收事件，但不影响当前 tab 布局。

## 参考

- 主界面日志实现：`src/views/main/logs.tsx`
- 当前开发者工具窗口：`src/views/dev-tools/dev-tools-window.tsx`
- Tabs 组件：`src/components/ui/tabs.tsx`
