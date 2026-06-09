# CTE MAX+1 ID 生成原子性修复 Plan

**状态：待实施**  
**来源：** [PR #3 Copilot review](https://github.com/USTC-XeF2/unibot/pull/3)  
**影响范围：** `friend_requests`、`group_requests`、`messages`、`message_reactions`、`pokes`、`group_essence_messages` 中所有使用 CTE `MAX(CAST(id AS INTEGER)) + 1` 的 ID 生成路径。

## 问题

当前多个 repo 的 INSERT 语句使用以下模式生成自增 TEXT ID：

```sql
WITH next_id(value) AS (
    SELECT CAST(COALESCE(MAX(CAST(id_column AS INTEGER)), 0) + 1 AS TEXT)
    FROM table_name
)
INSERT INTO table_name (id_column, ...)
SELECT value, ...
FROM next_id
```

在 WAL 模式 + 多连接（当前 max_connections=5）下，两个事务可能同时读到同一个 `MAX`，计算出相同的下一个 ID，导致后提交的事务触发 `UNIQUE` 约束失败。

## 方案

### 方案 A：Rust 端生成 ID（推荐）

在 Rust 侧用 UUID v7（时间有序）或 nanoid 生成 ID，INSERT 时直接绑定字符串，不再依赖 CTE。

**优点：**

- 彻底消除竞态，无需 DB 锁
- 支持分布式扩展
- ID 不暴露插入顺序

**缺点：**

- ID 不再是短数字串，调试时肉眼辨识度略低
- 需引入 `uuid` 或 `nanoid` crate

### 方案 B：SQLite INTEGER PRIMARY KEY + TEXT 映射

将表的主键改为 `INTEGER PRIMARY KEY`（利用 SQLite 内置自增 rowid），同时保留当前的 TEXT 列作为对外 ID（或用 `CAST(rowid AS TEXT)` 直接暴露）。

**优点：**

- 利用 SQLite 内置原子自增
- ID 仍为数字串，兼容当前前端展示

**缺点：**

- 需要 DDL 迁移修改主键
- `INTEGER PRIMARY KEY` 只能有一个，和当前的 TEXT PK 语义冲突

### 方案 C：序列表 + 显式事务锁

创建专用的 `id_sequences` 表，在事务中 `UPDATE ... SET next_val = next_val + 1` 并依赖 SQLite 的行级写锁保证原子性。

**优点：**

- 改动最小，不需要改表结构

**缺点：**

- 每条 INSERT 多一次 UPDATE
- 序列表成为写入热点

## 推荐

**方案 A**（Rust 端生成），用 `uuid` crate 的 v7 变体，时间有序且全局唯一。

## 涉及文件

- `src-tauri/src/persistence/repo/user/friends.rs` — `create_friend_request`
- `src-tauri/src/persistence/repo/group/requests.rs` — `create_group_request`
- `src-tauri/src/persistence/repo/group/content.rs` — `create_group_essence_message`
- `src-tauri/src/persistence/repo/group/events.rs` — `insert_group_event`
- `src-tauri/src/persistence/repo/message.rs` — `insert_message`
- `src-tauri/src/persistence/repo/interaction.rs` — `insert_message_reaction`、`insert_poke`
