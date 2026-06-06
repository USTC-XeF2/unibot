# Service 层鉴权补齐 Plan

**状态：待实施**  
**来源：** [PR #3 Copilot review](https://github.com/USTC-XeF2/unibot/pull/3)  
**影响范围：** `MessageService::list_history`、`MessageRepo::list_messages`（group 分支）、`InteractionService::list_poke_history`、`InteractionRepo::list_pokes`（group 分支）

## 问题

当前多处 service/repo 方法不做调用者身份或群成员校验：

1. **`MessageService::list_history`** — 没有 `core.require_user_context`，不校验群成员身份。结合 repo 层 group 分支不用 `user_id`，任何人可传入任意 `group_id` / `user_id` 拉取消息历史。

2. **`MessageRepo::list_messages` group 分支** — 硬编码 `message_scene = 'group'`，不使用 `user_id`，不用 `source_type` 参数。调用者传入任何 `source_id` 都能查到对应群的消息。

3. **`InteractionService::list_poke_history`** — 同样无 caller 验证。结合 repo 层 group 分支 `list_pokes` 不用 `user_id`，可读取任意群/私聊的戳一戳记录。

## 方案

### Service 层

- `MessageService::list_history` 开头加 `core.require_user_context(&user_id)`，群消息场景加 `ensure_group_member` 校验
- `InteractionService::list_poke_history` 同上：`require_user_context` + 群场景 membership 校验；私聊场景校验 `user_id` 是会话参与者之一

### Repo 层

- `MessageRepo::list_messages` group 分支改为使用 `source_type` 参数（而非硬编码 `'group'`），可选加 `user_id` JOIN `group_members` 作为授权过滤
- `InteractionRepo::list_pokes` group 分支可选加 `user_id` JOIN `group_members`

## 涉及文件

- `src-tauri/src/services/message.rs`
- `src-tauri/src/services/interaction.rs`
- `src-tauri/src/persistence/repo/message.rs`
- `src-tauri/src/persistence/repo/interaction.rs`