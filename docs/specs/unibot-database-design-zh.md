# UniBot 数据库设计概览

本文档是 UniBot 数据库设计的入口文档，目标是在 1–2 分钟内说明数据库整体架构、核心数据域、关键表关系和主要设计决策。完整字段、索引、DDL、迁移和清理策略见 `database/` 目录下的细节文档。

## 1. 设计目标

UniBot 数据库服务于"多 Bot 本地调试与管理平台"这一定位，主要目标是：

1. 保存本地调试所需的结构化数据——账号、Bot、会话、消息、协议包索引、调试会话和审计事件。
2. 支持"消息 → 协议包 → Bot 调试会话"双向追踪。
3. 支持会话列表、消息分页、协议包筛选等高频查询。
4. 大体积协议原文和 Bot 配置文件交给文件系统，数据库只存 `file_path` 和结构化索引。
5. 调试资产默认长期保留，消息不随协议包 TTL 自动删除。

## 2. 总体架构

UniBot 采用 SQLite + 文件系统的本地存储架构。

```text
┌─────────────────────────────────┐
│           UniBot App             │
│  Bot 管理 / 消息查看 / 协议调试    │
└──────────────┬──────────────────┘
               │
               ▼
┌─────────────────────────────────┐
│         SQLite (WAL)             │
│  账号 / Bot / 会话 / 消息        │
│  协议索引 / 审计 / 配置          │
└──────────────┬──────────────────┘
               │ path
               ▼
┌─────────────────────────────────┐
│           File System            │
│  协议原文 JSON (data/packets/)    │
│  Bot 配置文件 (configs/bots/)    │
│  导出文件 / 日志                  │
└─────────────────────────────────┘
```

数据库负责结构化查询、关联关系和约束；文件系统负责大体积、低频访问的原始内容。

## 3. 数据域划分

数据库按 5 个数据域组织，共 26 张表。完整表清单见 [table-dictionary.md](database/table-dictionary.md)。

| 数据域 | 职责 | 核心表 |
|---|---|---|
| 身份与社交域 | IM 账号身份、社交关系与分类 | `im_accounts`, `friendships`, `friend_categories`, `group_categories` |
| 群组与内容域 | 群组织结构与群内内容资产 | `chat_groups`, `group_members`, `group_requests` |
| 会话与消息域 | 会话列表、消息与互动 | `conversations`, `messages` |
| Bot 与调试域 | Bot 实例与调试会话 | `bots`, `debug_sessions` |
| 系统与审计域 | 协议报文、操作审计、应用配置 | `protocol_packets`, `audit_events`, `app_settings` |

Schema 版本通过 `app_settings` 中 `schema.version` 键管理。

## 4. 核心表与关系

核心关系（仅列主链路）：

```text
im_accounts ──── 账号身份，所有交互的起点
  │
  ├── bots ──── Bot 实例，绑定账号 + config_path 指向配置文件
  │     └── debug_sessions ──── 调试会话，按 bot_id + 时间区间聚合
  │           └── protocol_packets
  │
  ├── conversations ──── 用户视角会话列表（未读/置顶/免打扰）
  │     └── messages ──── 消息记录
  │
  └── social caches ──── 好友关系、群组、成员等社交缓存

messages ←→ protocol_packets ──── 双向追踪，file_path 指向原始 JSON

system ──── 横切层
  ├── app_settings ──── 键值配置 (含 schema.version)
  └── audit_events ──── 操作审计 (保留期内只追加)
```

核心表说明：

| 表 | 作用 |
|---|---|
| `im_accounts` | 统一 IM 身份，模拟或真实账号 |
| `bots` | Bot 实例，绑定账号，配置文件路径 |
| `conversations` | 会话视图，维护未读数、最后消息、置顶 |
| `messages` | 消息记录，支持追踪到协议包 |
| `debug_sessions` | 一次 Bot 调试运行，聚合消息与协议包 |
| `protocol_packets` | 协议报文结构化索引，原文走文件系统，`file_path` 指向原始 JSON |
| `audit_events` | 操作审计，保留期内只追加，按类型执行保留策略 |

## 5. 核心数据流

### 5.1 消息入库

```text
协议事件
  → protocol_packets (结构化索引)
  → messages (字段归一化)
  → conversations (last_message_id + unread_count 更新)
```

### 5.2 消息与协议包追踪

```text
message ←→ protocol_packet (related_object_type + related_object_id)
         └── file_path → data/packets/*.json
```

消息详情可追溯产生该消息的协议事件；协议调试页可从协议包反查关联消息。

### 5.3 Bot 调试会话

```text
bot → debug_session → protocol_packets / messages
```

每次 Bot 调试运行创建一个会话，通过 `session_id` 聚合该次运行的消息和协议调用。

## 6. 关键设计决策

| 决策 | 说明 | 细节 |
|---|---|---|
| SQLite + WAL | 本地嵌入式数据库，适合桌面调试场景；WAL 提升读写体验，仍是单写者模型 | [migrations.md](database/migrations.md) |
| 协议原文文件化 | 大体积协议 JSON 不直接入库，数据库只保存 `file_path` 和结构化索引；查看原文时懒惰读取文件 | [lifecycle-and-cleanup.md](database/lifecycle-and-cleanup.md) |
| Bot 配置文件外置 | Bot 连接与行为配置变化快，数据库存 `config_path` 指针 | [table-dictionary.md](database/table-dictionary.md) |
| 会话独立建表 | 会话列表、未读数、置顶、免打扰不从消息表临时聚合，支持空会话 | [er-model.md](database/er-model.md) |
| 消息默认长期保留 | 消息是调试核心资产，不随协议包 TTL 自动删除；账号删除时默认解除强引用并保留历史消息 | [lifecycle-and-cleanup.md](database/lifecycle-and-cleanup.md) |
| 文件最终一致性 | 数据库事务只覆盖结构化记录，协议文件通过临时文件+原子重命名减少半写入风险；文件缺失在查看或导出时懒惰提示 | [lifecycle-and-cleanup.md](database/lifecycle-and-cleanup.md) |

其他实现层决策（`bot_id` 反规范化、FRIENDSHIP owner 视角、环境隔离、ID 统一 TEXT 类型）见 [table-dictionary.md](database/table-dictionary.md) 和 [er-model.md](database/er-model.md)。

## 7. 细节文档索引

| 文件 | 内容 | 何时阅读 |
|---|---|---|
| [database/er-model.md](database/er-model.md) | ER 图、数据域关系、关系基数 | 需要理解表间关系时 |
| [database/table-dictionary.md](database/table-dictionary.md) | 26 张表完整字段、类型、约束 | 需要查具体字段时 |
| [database/indexes.md](database/indexes.md) | 全量索引设计、查询场景 | 需要优化查询时 |
| [database/lifecycle-and-cleanup.md](database/lifecycle-and-cleanup.md) | 数据生命周期、清理策略、文件一致性 | 需要理解数据保留/删除时 |
| [database/migrations.md](database/migrations.md) | Schema 版本管理、迁移命名与执行 | 需要修改表结构时 |
| [database/ddl/](database/ddl/) | 可执行建表 SQL | 需要初始化/重建数据库时 |
| [database/traceability.md](database/traceability.md) | FR → 表 → 字段追踪矩阵 | 需要验证需求覆盖时 |
