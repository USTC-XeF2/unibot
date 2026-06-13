# 接入 tauri-specta：前后端命令契约自动生成

**日期:** 2026-06-13

## 背景与动机

SQL 面板曾因前端 `invoke("is_write_query")` 与后端注册名 `is_write_query_command`
不一致而整条执行链失败。根因是 Tauri 命令名是 **stringly-typed**：前端字符串字面量
与后端命令名各自独立维护，编译器对两边都看不见对方，拼写漂移要到运行时点按钮才暴露。

本设计接入 [tauri-specta](https://github.com/specta-rs/tauri-specta) v2，从 Rust 的
`#[tauri::command]` 单一真相源**自动生成 TypeScript 绑定**（命令名、参数名、参数类型、
返回类型）。后端改名或改参数后，重新生成绑定，前端 `tsc` 在**编译期**直接报错，把
这类 bug 从“运行时”提前到“编译期、由机器发现”。

## 范围

- **后端：全量接入**。tauri-specta 的 `collect_commands!` 整体替换 `generate_handler!`，
  这是原子操作——所有 80 个命令必须加 `#[specta::specta]`，所有边界类型必须加
  `specta::Type` derive，否则不编译。
- **前端：仅迁移 dev-tools 调用点**到生成的 `commands.*`。其余 60+ 调用点保留现有
  raw `invoke("name", ...)`（命令名不变，仍然可用），后续可渐进迁移。
- **事件不在本次范围**。`devtools:event` / `chat:event` 经手写 `emit_to(label, channel, payload)`
  发送，未使用 tauri-specta 的事件系统。本次只迁移 commands，事件保持原样。

## 现状盘点（已探查）

- 命令：80 个，分布在 `commands/{main,log,bot,user,packet,dev_tools,chat/*}.rs`。
  全部统一返回 `Result<T, String>`，便于机械化加注解。
- 边界类型：约 40 个。
  - `models/entities.rs`：29 个 struct/enum（含 tagged enum：`MessageSource` 用
    `#[serde(tag = "scene")]`、`InternalEvent` 用 `#[serde(tag = "type")]`）。
  - `commands/*.rs` 内的结果类型 10 个：`DbColumn` `DbIndex` `DbSchema` `DbStatus`
    `DbTable` `LogCleanupResult` `LogSettings` `SqlQueryResult` `SystemLogEntry`
    `TableRowPreview`。
  - `models/internal.rs`：`MessageSegment` `NoticeType` `InternalEvent`。
- `serde_json::Value` 边界点 3 处：`SystemLogEntry.fields: Option<serde_json::Value>`、
  `SqlQueryResult.rows: Vec<Vec<serde_json::Value>>`、`TableRowPreview.rows` 同样。
  需要 specta 的 `serde-json` 特性，生成类型为 `JsonValue`（TS `any`/`unknown`）。
- 运行时为 `tauri::Wry`（默认）。已有 `src-tauri/build.rs`。
- 前端 dev-tools 调用点（`src/lib/query/dev-tools.ts`）：
  `get_db_schema` `open_developer_tools` `execute_sql` `is_write_query_command`
  `preview_table_rows`，共 5 个。

## 架构设计

### 依赖（src-tauri/Cargo.toml）

specta 需启用 `serde-json` 特性以支持 `serde_json::Value` 边界类型：

```toml
specta = { version = "=2.0.0-rc.21", features = ["serde-json"] }
specta-typescript = "0.0.9"
tauri-specta = { version = "=2.0.0-rc.21", features = ["derive", "typescript"] }
```

### Builder 接线（src-tauri/src/lib.rs）

`run()` 顶部构造 specta Builder，debug 构建时导出绑定，再把 `builder.invoke_handler()`
交给 `tauri::Builder`：

```rust
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

pub fn run() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        main::register_user,
        // …全部 80 个命令，与原 generate_handler! 列表一一对应…
        dev_tools::is_write_query_command,
    ]);

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/lib/bindings.ts")
        .expect("failed to export typescript bindings");

    tauri::Builder::default()
        .setup(|app| { /* 现有 setup 不变 */ Ok(()) })
        // …plugins 不变…
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

要点：
- `collect_commands!` 列表替换掉原 `generate_handler!`，命令顺序/集合保持一致。
- 绑定输出到 `src/lib/bindings.ts`，纳入 git（不进 .gitignore），保证 CI/其他成员
  无需跑一次 app 也能编译前端。
- 导出仅在 `debug_assertions` 下进行：开发时跑一次 app 即刷新 `bindings.ts`。

### 命令注解

每个 `#[tauri::command]` 下方加 `#[specta::specta]`：

```rust
#[tauri::command]
#[specta::specta]
pub fn is_write_query_command(query: String) -> bool { is_write_query(&query) }
```

### 类型注解

每个边界类型的 `#[derive(...)]` 追加 `specta::Type`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus { /* … */ }
```

tagged enum（`MessageSource` `InternalEvent`）specta v2 支持 `#[serde(tag = ...)]`，
derive 后照常生成可辨识联合类型。

### 前端迁移（仅 dev-tools）

`src/lib/query/dev-tools.ts` 的 5 个 `invoke<T>("name", args)` 改为从生成的
`@/lib/bindings` 导入 `commands`：

```ts
import { commands } from "@/lib/bindings";

// before: invoke<boolean>("is_write_query_command", { query })
// after:
const isWrite = await commands.isWriteQueryCommand(query);
```

生成的 `commands.*` 方法名为 camelCase，参数为位置参数（具体签名以生成结果为准，
迁移时对照 `bindings.ts`）。`@/types/dev-tools` 中被 dev-tools 复用的手写类型，
迁移调用点时改为从 `bindings.ts` 导入对应生成类型；其余非 dev-tools 仍用旧类型，
本次不动。

## 验证

1. `cargo build --manifest-path src-tauri/Cargo.toml`：后端编译通过（证明 80 命令 +
   40 类型注解齐全，否则 `collect_commands!` 不编译）。
2. 跑一次 `bun run tauri dev` 触发 debug 导出，确认 `src/lib/bindings.ts` 生成且含
   全部命令。
3. `bun run build`（tsc + vite）：前端编译通过，dev-tools 调用点使用生成类型无报错。
4. `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::dev_tools`：现有
   13 个测试仍通过。
5. 手动：打开开发者工具，SQL 面板执行 `SELECT`，确认整条链路（含写检测）正常。

## 验收点 → 实现映射

| 验收点 | 实现位置 |
|--------|----------|
| 命令契约单一真相源 | Cargo 依赖 + Builder 接线 |
| 80 命令全部可被 specta 收集 | 每个命令加 `#[specta::specta]` |
| 40 边界类型可生成 TS | 每个类型加 `specta::Type` derive |
| `serde_json::Value` 边界可生成 | specta `serde-json` 特性 |
| 前端 dev-tools 走类型安全调用 | `dev-tools.ts` 改用 `commands.*` |
| 命令名漂移在编译期暴露 | `bindings.ts` 入库 + tsc 校验 |

## 风险与权衡

- **原子性**：后端任一命令/类型漏注解都会导致 `collect_commands!` 编译失败。Task 拆分
  按文件推进，每个文件改完单独 `cargo build`，把失败定位在最小范围。
- **版本钉死**：tauri-specta v2 仍是 rc，依赖用 `=` 精确钉版，避免 minor 漂移破坏构建。
- **tagged enum / serde 属性**：specta 对部分 serde 属性支持有边界。若某类型 derive 后
  生成异常，回退策略是对该类型用 specta 的等价属性显式标注，而非放弃整体接入。
- **bindings.ts 入库**：生成物入库会带来 review 噪音，但换来“无需跑 app 即可编译前端”
  与 CI 可校验，权衡后选择入库。
