# 群功能优化实施计划

> 基于 [群功能优化设计 Spec](../specs/2026-06-10-group-optimization.md)（v2 修正版）。
>
> 从 `main` 分支切出 `feat/group-optimization`，最终 squash merge。

---

## 0. 实施策略

**两条独立线并行推进**：

| 线 | 内容 | 文件交集 |
|---|---|---|
| **文件线** | Phase 1 → Phase 2 → Phase 3 | `Cargo.toml`, `package.json`, `tauri.conf.json`, `models/entities.rs`, `lib.rs`, `repo/group/`, `services/group/`, `commands/chat/group.rs`, 前端 types/query/mutations |
| **会话线** | Phase 4 → Phase 5 | `repo/group/basic.rs`, `repo/conversation.rs`（新建）, `services/`, `commands/`, `conversation-list.tsx`, `chat-main-panel.tsx` |

两条线只在前端 mutations/types 和 `lib.rs` 的 command 注册上有轻微交集，可在 Step 5（合并点）统一解决。

---

## Step 1：Phase 1 基础设施（文件线起点）

**目标**：让编译通过、migration 可运行、所有实体类型就绪。

### Rust
1. `src-tauri/Cargo.toml` — 加 `tauri-plugin-dialog = "2"`, `sha2 = "0.10"`
2. `src-tauri/tauri.conf.json` — 加 assetProtocol `scope: ["$APPDATA/groups/**"]`
3. `src-tauri/capabilities/default.json` — permissions 加 `"dialog:default"`
4. `src-tauri/src/lib.rs` — `.plugin(tauri_plugin_dialog::init())`
5. `src-tauri/src/persistence/migrations/0003_group_file_photo_paths.sql` — 新建
6. `src-tauri/src/persistence/migrations/mod.rs` — 注册 0003
7. `src-tauri/src/models/entities.rs` — `GroupFileEntity` 加 `file_path`；新增 `GroupAlbumEntity`, `GroupPhotoEntity`, `GroupCategoryEntity`
8. `cargo test --manifest-path src-tauri/Cargo.toml` — 确认 migration 测试通过

### 前端
9. `package.json` — 加 `@tauri-apps/plugin-dialog`
10. `bun install` — 装新依赖
11. `src/types/group.ts` — 加 `GroupFile`, `GroupAlbum`, `GroupPhoto`, `GroupCategory`, `ConversationState`
12. `bun run build` — 确认前端编译通过

**检查点**：`cargo test` 全绿 + `bun run build` 无报错。

---

## Step 2：Phase 2 群文件（文件线）

**目标**：群文件可实际上传、下载、删除，磁盘文件真实落盘。

### Rust
1. 新建 `src-tauri/src/services/group/storage.rs` — 文件落盘 helper：
   - `sanitize_file_name(name: &str, file_id: &str) -> String`
   - `copy_file_to_groups_dir(src: &Path, group_id: &str, file_id: &str, file_name: &str, app_data_dir: &Path) -> AppResult<String>`（返回相对 file_path）
   - `compute_sha256(path: &Path) -> AppResult<String>`（流式读取，64 字符 hex）
   - `validate_group_file_path(file_path: &str, app_data_dir: &Path) -> AppResult<PathBuf>`（canonicalize + 前缀校验）
   - `delete_group_file_disk(file_path: &str, app_data_dir: &Path) -> ()`（仅 eprintln! 不阻断）
2. `src-tauri/src/services/group/mod.rs` — `pub mod storage;`
3. `src-tauri/src/persistence/repo/group/mod.rs` — 新增 Row types：`GroupFileRow` 加 `file_path`, `GroupAlbumRow`, `GroupPhotoRow`
4. `src-tauri/src/persistence/repo/group/content.rs`：
   - `upsert_group_file` INSERT 加 `file_path`
   - `list_group_files` SELECT 加 `file_path`，增加 `parent_folder_id` 过滤参数
   - 新增 `get_group_file_by_id`, `delete_group_file`
5. `src-tauri/src/services/group/content.rs` — `upsert_group_file` / `list_group_files` 签名不变（entity 已含 `file_path`），`download_group_file` 和 `delete_group_file` 的逻辑（校验权限 + 调 repo）
6. `src-tauri/src/commands/chat/group.rs` — 新增 `upload_group_file`, `download_group_file`, `delete_group_file`
7. `src-tauri/src/lib.rs` — `generate_handler!` 注册 3 个新 command

### 前端
8. `src/lib/query/groups.ts` — `useGroupFiles(groupId, parentFolderId?)`, `invalidateGroupFilesQuery`
9. `src/lib/mutations.ts` — `useUploadGroupFileMutation`, `useDownloadGroupFileMutation`, `useDeleteGroupFileMutation`
10. `src/components/chat/group-files-panel.tsx` — 新建：面包屑 + 上传按钮 + 文件列表（含下载/删除）
11. 把 `group-files-panel` 接入群详情 Tab（先不接入也可以——但最好有个最小可用入口验证）

**检查点**：
- `cargo test` 全绿
- 手动验证：上传文件 → DB 有记录 + 磁盘有文件 → 下载成功 → 删除后 DB 和磁盘都干净

---

## Step 3：Phase 3 群相册（文件线终点）

**目标**：相册 CRUD + 照片上传/展示，本地图片通过 `convertFileSrc` 渲染。

### Rust
1. `src-tauri/src/persistence/repo/group/content.rs` — 新增：
   - `create_group_album`, `list_group_albums`（单条 SQL 含 photo_count + cover_file_path）, `delete_group_album`
   - `create_group_photo`, `list_group_photos`（返回中带 abs_path）, `get_group_photo_by_id`, `delete_group_photo`
2. `src-tauri/src/services/group/content.rs` — 照片上传（复用 storage helper，校验图片扩展名 + 20MB 上限）
3. `src-tauri/src/commands/chat/group.rs` — 新增 `create_group_album`, `list_group_albums`, `delete_group_album`, `upload_group_photo`, `list_group_photos`, `delete_group_photo`
4. `src-tauri/src/lib.rs` — 注册 6 个新 command

### 前端
5. `src/lib/query/groups.ts` — `useGroupAlbums`, `useGroupPhotos`, invalidate hooks
6. `src/lib/mutations.ts` — 相册/照片 mutations
7. `src/components/chat/group-albums-panel.tsx` — 相册网格卡片
8. `src/components/chat/group-photos-panel.tsx` — 照片网格 + 大图 Modal

**检查点**：
- 创建相册 → 上传照片 → 网格显示原图 → 点击放大 → 删除照片 → 删除相册后磁盘目录干净

---

## Step 4：Phase 4 会话置顶/免打扰 + 群分类（会话线起点）

**目标**：`conversations` 表读写 command 就绪，会话列表支持置顶排序和分类筛选。

### Rust
1. 新建 `src-tauri/src/persistence/repo/conversation.rs`：
   - `upsert_conversation_pinned`（private/group 分两条 SQL）
   - `upsert_conversation_muted`（同上）
   - `list_conversation_states(owner_user_id)`
2. `src-tauri/src/persistence/repo/group/basic.rs` — 新增：
   - `list_group_categories`, `create_group_category`, `delete_group_category`
   - `set_group_category`
3. 新建 `src-tauri/src/services/conversation.rs` — `ConversationService`
4. `src-tauri/src/services/mod.rs` — `ServiceHub` 加 `conversation: ConversationService`
5. `src-tauri/src/lib.rs`：
   - 构造 `ConversationService`（在 `ServiceHub::new` 中注入）
   - 新增 commands：`set_conversation_pinned`, `set_conversation_muted`, `list_conversation_states`
   - 新增 commands：`list_group_categories`, `create_group_category`, `delete_group_category`, `set_group_category`
   - `generate_handler!` 注册 7 个新 command

### 前端
6. `src/lib/query/groups.ts` — `useConversationStates`, `useGroupCategories`
7. `src/lib/mutations.ts` — `useSetConversationPinned`, `useSetConversationMuted`, `useCreateGroupCategory`, `useDeleteGroupCategory`, `useSetGroupCategory`
8. `src/components/chat/conversation-list.tsx`：
   - merge `list_conversation_states` 结果
   - 置顶分组排序
   - 免打扰 `BellOff` 图标
   - 右键菜单加"置顶/取消置顶"、"免打扰/开启通知"、"移动到分类"
   - 顶部加分类筛选下拉

**检查点**：
- 右键置顶会话 → 列表重排置顶在前
- 设置免打扰 → `BellOff` 图标出现
- 新建分类 → 把群移到分类 → 筛选下拉生效

---

## Step 5：Phase 5 群管理 UI（会话线终点）

**目标**：群成员列表面板 + 群信息编辑 Sheet。

### 前端（纯前端，无新 Rust command）
1. `src/components/chat/group-members-panel.tsx` — 新建：
   - 成员列表（Avatar + 昵称/名片 + 角色标签）
   - DropdownMenu 操作：禁言、设管理、踢人、改名片
   - hover 快捷图标（禁言时钟、踢人垃圾桶）
2. 把 panel 接入 `chat-main-panel.tsx` 的 ResizablePanelGroup（右侧新增面板）或 Sheet
3. `src/components/chat/group-info-sheet.tsx` — 群信息编辑 Sheet：
   - 群名称（`useRenameGroupMutation`）
   - 群公告编辑入口（预留，调用 `upsert_group_announcement`）
   - 群简介（若做了 0004 migration）

**可选 Rust**（若决定加群简介）：
4. `src-tauri/src/persistence/migrations/0004_group_description.sql` — `ALTER TABLE chat_groups ADD COLUMN description TEXT;`
5. `src-tauri/src/persistence/migrations/mod.rs` — 注册 0004
6. `src-tauri/src/models/entities.rs` — `GroupProfile` 加 `description: Option<String>`
7. `src-tauri/src/persistence/repo/group/basic.rs` — `update_group_description`
8. 前端 `GroupProfile` 类型同步

**检查点**：
- 打开群成员面板 → 看到所有成员角色标签
- 对成员点禁言 → 触发 mutation → 成员 mute_until 更新
- 群信息 Sheet 可改名

---

## 6. 合并与收尾

1. **两条线都完成后**，统一跑 `cargo test` + `bunx --bun @biomejs/biome check --write`
2. **squash merge** `feat/group-optimization` → `main`
3. **更新 docs**：在 `project-meta` 分支更新 roadmap，标记群优化已完成

---

## 7. 风险与对策

| 风险 | 对策 |
|------|------|
| `tauri-plugin-dialog` 与现有权限系统冲突 | 已在 spec 中明确 `dialog:default` capability；若仍报错，检查 Tauri 版本兼容性 |
| SQLite `ON CONFLICT ... WHERE` 在新环境不生效 | 已在 `message.rs` 跑通（约 1 年），风险极低；fallback 是 `INSERT OR REPLACE` 整行替换 |
| 文件落盘后 DB 写入失败， orphaned 文件 | command 层 catch DB Err → 立即 `tokio::fs::remove_file` 回滚；spec 2.5 已明确 |
| `convertFileSrc` 在 dev 模式不生效 | assetProtocol scope `$APPDATA` 在 dev 模式下指向 `src-tauri/target/debug/` 旁；确认路径正确 |
| 两条线同时改 `lib.rs` 的 `generate_handler!` | Step 5 合并时统一解决；也可文件线先注册占位，会话线补全 |

---

## 8. 实施顺序速查

```
main ──┬──► feat/group-optimization
       │
       │  Step 1: Phase 1 基础设施 ──► cargo test 绿
       │
       │  ┌─────────────────────┐
       ├──┤ Step 2: Phase 2 文件 ├──► cargo test + 手动上传/下载/删除
       │  └─────────────────────┘
       │           │
       │           ▼
       │  ┌─────────────────────┐
       ├──┤ Step 3: Phase 3 相册 ├──► cargo test + 手动相册/照片
       │  └─────────────────────┘
       │
       │  ┌─────────────────────────────────┐
       ├──┤ Step 4: Phase 4 会话状态+分类  ├──► cargo test + 手动置顶/分类
       │  └─────────────────────────────────┘
       │           │
       │           ▼
       │  ┌──────────────────────────────┐
       └──┤ Step 5: Phase 5 群管理 UI    ├──► 纯前端验证
          └──────────────────────────────┘
```

> Step 2/3（文件线）和 Step 4（会话线）可**并行开发**——只在 `lib.rs` command 注册和前端 types 有轻微交集。
