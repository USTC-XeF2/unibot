# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

UniBot is a Tauri v2 desktop app that acts as a local debugging platform for bot developers. It bundles a virtual IM server and simulated client, with a React/TypeScript frontend and Rust backend. All data is local (SQLite + JSON); there is no network dependency at runtime.

## Common commands

Use `bun` as the package manager and runtime.

- **Start the dev app** (frontend + Tauri window hot-reload):
  ```bash
  bunx tauri dev
  ```
- **Start frontend only** (Vite on port 1420):
  ```bash
  bun run dev
  ```
- **Build the full desktop app**:
  ```bash
  bunx tauri build
  ```
- **Build frontend only**:
  ```bash
  bun run build
  ```
- **Format/lint TypeScript and TSX**:
  ```bash
  bunx --bun @biomejs/biome check --write
  ```
  Biome is configured in [biome.json](biome.json). `lint-staged` runs this on pre-commit.
- **Format Rust**:
  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml --
  ```
- **Run Rust tests** (uses `#[sqlx::test]` with an ephemeral SQLite database):
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```
- **Run a single Rust test**:
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml smoke_crud_users
  ```

There are currently no frontend unit tests configured.

## High-level architecture

### Frontend (`src/`)

- **Stack**: Vite + React 19 + TypeScript 5.8, Tailwind CSS v4, shadcn/radix-nova components, Bun lockfile (`bun.lock`).
- **Routing**: Hash router via `react-router`. Main layout at `/`, per-user chat windows at `/#/chat/:userId`. Routes are defined in [src/App.tsx](src/App.tsx).
- **Windowing**: The Rust backend creates one main window (`main`) and one chat window per user (`chat-{user_id}`). Chat windows use a separate Tauri capability file at [src-tauri/capabilities/chat.json](src-tauri/capabilities/chat.json).
- **Server state**: TanStack Query (React Query) with a single global `queryClient`. Query hooks live in [src/lib/query/](src/lib/query/) with keys centralized in [src/lib/query/keys.ts](src/lib/query/keys.ts).
- **Mutations**: Most mutations are wrapped in [src/lib/mutations.ts](src/lib/mutations.ts) and invalidate the appropriate query keys on success.
- **Real-time updates**: Backend emits Tauri events on the `chat:event` channel. The singleton hook in [src/hooks/use-chat-event-bus.ts](src/hooks/use-chat-event-bus.ts) subscribes once per user, invalidates affected queries, and dispatches to any component-level callback. This is the primary mechanism for keeping chat history, poke history, and group state in sync.
- **Local UI state**: Zustand stores in [src/store/](src/store/) hold small pieces of UI state such as the current user id.
- **Components**: shadcn/ui-style primitives are in [src/components/ui/](src/components/ui/). Chat-specific components are in [src/components/chat/](src/components/chat/).
- **Styling**: Tailwind v4 with CSS-only configuration in [src/App.css](src/App.css). Biome includes a nursery rule for sorted Tailwind classes.

### Backend (`src-tauri/src/`)

- **Entry point**: [src-tauri/src/lib.rs](src-tauri/src/lib.rs) builds the Tauri app, registers the invoke handler, and wires up SQLite + services + the in-memory core on startup.
- **In-memory core**: [src-tauri/src/core.rs](src-tauri/src/core.rs) defines `CoreContainer`, which holds `UserContext`s in a `RwLock<HashMap>`. Each `UserContext` owns a `tokio::sync::broadcast` channel for real-time events and tracks its chat window label. Users are loaded from SQLite on startup and registered here.
- **Commands**: [src-tauri/src/commands/](src-tauri/src/commands/) are thin Tauri command handlers. They extract `tauri::State<ServiceHub>` and `tauri::State<CoreContainer>` and delegate to services. They use `IntoCommandResult` to convert `AppResult<T>` into `Result<T, String>` for the frontend.
- **Services**: [src-tauri/src/services/](src-tauri/src/services/) contain business logic. `ServiceHub` bundles all services and is managed as Tauri state. Services validate rules (e.g., group mute, recall permissions), persist via repos, and emit events through the core.
- **Persistence**:
  - SQLite is initialized in [src-tauri/src/persistence/db_pool.rs](src-tauri/src/persistence/db_pool.rs) using `sqlx` with WAL mode. The database file lives in the Tauri app data directory (`unibot.db`).
  - Migrations are hand-written in [src-tauri/src/persistence/migrations/](src-tauri/src/persistence/migrations/) and driven by a custom migrator in [src-tauri/src/persistence/migrator.rs](src-tauri/src/persistence/migrator.rs) that stores the current schema version in `app_settings.schema_version`. Initial schema is `0001_initial_schema.sql`.
  - Repos in [src-tauri/src/persistence/repo/](src-tauri/src/persistence/repo/) encapsulate SQL queries and return entity structs. Repo tests are in [src-tauri/src/persistence/repo/tests.rs](src-tauri/src/persistence/repo/tests.rs) and use `#[sqlx::test]`.
- **Models**: [src-tauri/src/models/entities.rs](src-tauri/src/models/entities.rs) defines domain entities that are serialized across the Tauri boundary (and therefore shared with the frontend TypeScript types). [src-tauri/src/models/internal.rs](src-tauri/src/models/internal.rs) defines `InternalEvent`, the backend event enum broadcast to chat windows.
- **Errors**: [src-tauri/src/error.rs](src-tauri/src/error.rs) defines `AppError` (`Validation`, `NotFound`, `Conflict`, `Storage`, `Internal`) and `AppResult<T>`. `sqlx::Error` maps to `AppError::Storage` and `serde_json::Error` maps to `AppError::Internal`.
- **Utilities**: [src-tauri/src/utils.rs](src-tauri/src/utils.rs) provides `now_ts()` (millis since epoch), `emit_to_users`, and `recipients_for_source` for computing which registered users should receive a given event.

### Data flow for a chat message

1. Frontend calls `invoke("send_message", { userId, source, content, quotedMessageId })`.
2. `commands/chat/message.rs` delegates to `MessageService::send`.
3. Service validates sender permissions, persists the message via `MessageRepo`, then constructs an `InternalEvent::Message`.
4. `recipients_for_source` computes the target user ids (private peer, or all group members) and emits the event via each user's `UserContext` broadcast channel.
5. If the user has an open chat window, the backend in `core.rs` forwards the event over Tauri's `chat:event` channel to that window.
6. Frontend's `useChatEventBus` receives the event, invalidates the relevant message history query, and calls any registered subscriber.

### Conventions

- Use `DbId = String` for all IDs. IDs are generated as UUID-like strings by the repos.
- Message content is a JSON array of `MessageSegment` (`Text`, `Image`, `At`, `AtAll`, `Face`).
- Message source is an enum tagged by `scene`: `private` (with `peer_user_id`) or `group` (with `group_id`).
- Query keys mirror the Rust command names roughly: `chat:history`, `chat:poke-history`, `groups`, `friends`, `requests`, etc.
- When adding a new backend command, register it in `lib.rs`'s `invoke_handler!` macro and expose a frontend mutation in [src/lib/mutations.ts](src/lib/mutations.ts) (or query hook in [src/lib/query/](src/lib/query/)) with matching types in [src/types/](src/types/).
- When changing the SQLite schema, add a new migration file and bump the version in [src-tauri/src/persistence/migrations/mod.rs](src-tauri/src/persistence/migrations/mod.rs). The migrator's `split_sql_statements` correctly handles triggers and semicolons inside strings/comments.

## Important files to know

- [package.json](package.json) — frontend dependencies and scripts.
- [src-tauri/Cargo.toml](src-tauri/Cargo.toml) — Rust dependencies and Tauri configuration.
- [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) — Tauri window and bundle config.
- [biome.json](biome.json) — formatter/linter rules.
- [vite.config.ts](vite.config.ts) — Vite config with Tauri dev settings (port 1420, ignores `src-tauri`).
- [components.json](components.json) — shadcn/ui registry config.
