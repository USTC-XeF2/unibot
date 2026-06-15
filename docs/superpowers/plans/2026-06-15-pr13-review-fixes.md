# PR #13 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all five still-valid findings from the second review of PR #13 with tested download, revocation, path-safety, essence-removal, and frontend error behavior.

**Architecture:** Keep authorization and filesystem policy in Rust services, use repository methods for atomic database updates, and keep destination selection plus user feedback in React. Reuse the existing internal event bus and standalone-window labels for membership revocation instead of adding a second notification channel.

**Tech Stack:** Rust, Tokio, sqlx/SQLite, Tauri v2, React 19, TypeScript, TanStack Query, Vitest, Testing Library

---

## File Map

- `src-tauri/src/services/group/storage.rs`: validated source/destination file operations.
- `src-tauri/src/persistence/repo/group/content/files.rs`: atomic download count update.
- `src-tauri/src/services/group/content.rs`: download orchestration and essence set/unset branching.
- `src-tauri/src/persistence/repo/group/content/essence.rs`: delete essence by stable `essence_id`.
- `src-tauri/src/services/group/management.rs`: membership-left events and cleanup inputs.
- `src-tauri/src/commands/chat/group/management.rs`: pass `AppHandle` into lifecycle operations.
- `src-tauri/src/commands/chat/group/window.rs`: membership check before focus/create and reusable window cleanup.
- `src-tauri/src/commands/chat/group/content.rs`: download destination and essence identifier command inputs.
- `src-tauri/capabilities/group-content.json`: save-dialog permission.
- `src/types/event.ts`: frontend `group_member_left` event shape.
- `src/lib/query/groups.ts`: remove stale group-content queries.
- `src/lib/query/event-handlers.ts`: invalidate/remove data on membership loss.
- `src/lib/mutations.ts`: command payloads and operation-specific error toasts.
- `src/components/group/group-file-browser.tsx`: save dialog, query errors, and handled async failures.
- `src/components/group/group-album-browser.tsx`: query errors and handled async failures.
- `src/components/group/group-essence-panel.tsx`: unset by `essence_id`.
- `src/components/chat/chat-main-panel.tsx`: handled open-window failures.
- Focused Rust and frontend test files listed in each task.

### Task 1: Harden Group File Disk Operations

**Files:**
- Modify: `src-tauri/src/services/group/storage.rs`

- [ ] **Step 1: Add failing storage tests**

Add `#[cfg(test)] mod tests` covering:

```rust
#[tokio::test]
async fn delete_rejects_absolute_and_parent_paths() {
    let app_data = temp_dir();
    assert!(delete_group_file_disk("/tmp/outside", &app_data).await.is_err());
    assert!(delete_group_file_disk("../outside", &app_data).await.is_err());
}

#[tokio::test]
async fn delete_removes_valid_file_and_allows_missing_valid_file() {
    let app_data = temp_dir();
    let groups = app_data.join("groups/g1/files");
    tokio::fs::create_dir_all(&groups).await.unwrap();
    tokio::fs::write(groups.join("f.txt"), b"content").await.unwrap();

    delete_group_file_disk("groups/g1/files/f.txt", &app_data)
        .await
        .unwrap();
    delete_group_file_disk("groups/g1/files/missing.txt", &app_data)
        .await
        .unwrap();
}
```

On Unix, add a symlink escape test using `std::os::unix::fs::symlink`.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml services::group::storage::tests -- --nocapture
```

Expected: traversal/absolute-path tests fail because `delete_group_file_disk`
currently joins and deletes without validation.

- [ ] **Step 3: Implement strict stored-path validation**

Add a lexical validator that rejects empty, absolute, parent, root, and prefix
components. Make `validate_group_file_path` use it. Update
`delete_group_file_disk` to:

```rust
validate_stored_group_file_path(file_path)?;
let groups_root = app_data_dir.join("groups");
let candidate = app_data_dir.join(file_path);

match tokio::fs::symlink_metadata(&candidate).await {
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
    Err(error) => return Err(AppError::storage(...)),
    Ok(_) => {}
}

let canonical_groups = tokio::fs::canonicalize(&groups_root).await?;
let canonical_candidate = tokio::fs::canonicalize(&candidate).await?;
if !canonical_candidate.starts_with(&canonical_groups) {
    return Err(AppError::validation("file path escapes allowed directory"));
}
tokio::fs::remove_file(canonical_candidate).await?;
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the Step 2 command. Expected: all storage tests pass.

### Task 2: Implement Real Downloads and Atomic Counts

**Files:**
- Modify: `src-tauri/src/persistence/repo/group/content/files.rs`
- Modify: `src-tauri/src/persistence/repo/tests/group_files.rs`
- Modify: `src-tauri/src/services/group/storage.rs`
- Modify: `src-tauri/src/services/group/content.rs`
- Modify: `src-tauri/src/commands/chat/group/content.rs`
- Modify: `src-tauri/capabilities/group-content.json`
- Modify: `src/test/setup.ts`
- Modify: `src/components/group/__tests__/group-file-browser.test.tsx`
- Modify: `src/lib/mutations.ts`
- Modify: `src/components/group/group-file-browser.tsx`

- [ ] **Step 1: Add failing repository count test**

Add:

```rust
#[sqlx::test]
async fn increment_group_file_download_count_is_atomic(pool: sqlx::SqlitePool) {
    // Setup group/file using existing helpers.
    let repo = GroupRepo::new(pool);
    assert!(repo.increment_group_file_download_count("file-a").await?);
    assert_eq!(
        repo.get_group_file_by_id("file-a").await?.unwrap().download_count,
        1
    );
}
```

- [ ] **Step 2: Verify repository test is RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml increment_group_file_download_count_is_atomic -- --nocapture
```

Expected: compile failure because the repository method does not exist.

- [ ] **Step 3: Add the repository method**

Implement one SQL statement:

```rust
pub async fn increment_group_file_download_count(
    &self,
    file_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE group_files SET download_count = download_count + 1 WHERE file_id = ?1",
    )
    .bind(file_id)
    .execute(&self.pool)
    .await?;
    Ok(result.rows_affected() == 1)
}
```

- [ ] **Step 4: Verify repository test is GREEN**

Run the Step 2 command. Expected: PASS.

- [ ] **Step 5: Add failing download copy tests**

Add storage tests for:

```rust
#[tokio::test]
async fn copy_group_file_to_destination_copies_contents() { ... }

#[tokio::test]
async fn copy_group_file_to_destination_rejects_relative_destination() { ... }

#[tokio::test]
async fn copy_group_file_to_destination_rejects_source_as_destination() { ... }
```

- [ ] **Step 6: Verify download copy tests are RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml copy_group_file_to_destination -- --nocapture
```

Expected: compile failure because the helper does not exist.

- [ ] **Step 7: Implement the copy helper and service flow**

Add:

```rust
pub async fn copy_group_file_to_destination(
    file_path: &str,
    destination_path: &Path,
    app_data_dir: &Path,
) -> AppResult<PathBuf>
```

It validates the persisted source, requires an absolute destination, rejects
the canonical source path as destination, creates no parent directories, and
uses `tokio::fs::copy`.

Change `GroupService::download_group_file` to accept `destination_path:
PathBuf`, call the helper, increment the count only after copy success, and
return the destination string.

Change the Tauri command to require `destination_path: String`.

- [ ] **Step 8: Verify Rust download tests are GREEN**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml copy_group_file_to_destination -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml increment_group_file_download_count_is_atomic -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Add failing frontend download tests**

Mock both dialog functions in `src/test/setup.ts`:

```ts
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));
```

In the browser test, render one file and assert:

```ts
it("does not invoke download when save is cancelled", async () => { ... });

it("passes selected destination and reports success after download", async () => {
  mockedSave.mockResolvedValue("/downloads/report.txt");
  // click download
  expect(mockedInvoke).toHaveBeenCalledWith("download_group_file", {
    userId: "12345",
    groupId: "group_1",
    fileId: "file-1",
    destinationPath: "/downloads/report.txt",
  });
});
```

- [ ] **Step 10: Verify frontend download tests are RED**

Run:

```bash
bun run test src/components/group/__tests__/group-file-browser.test.tsx
```

Expected: tests fail because the component does not call `save` or pass a
destination.

- [ ] **Step 11: Implement frontend download flow**

Import `save`, choose `defaultPath: file.file_name`, cancel cleanly, pass
`destinationPath` through the mutation, and show success only after the command
resolves. Add `dialog:allow-save` to the capability.

- [ ] **Step 12: Verify frontend download tests are GREEN**

Run the Step 10 command. Expected: PASS.

### Task 3: Remove Essence by Stable Identity

**Files:**
- Modify: `src-tauri/src/persistence/repo/group/content/essence.rs`
- Modify: `src-tauri/src/persistence/repo/tests/group_essence.rs`
- Modify: `src-tauri/src/services/group/content.rs`
- Modify: `src-tauri/src/commands/chat/group/content.rs`
- Modify: `src/lib/mutations.ts`
- Modify: `src/components/group/group-essence-panel.tsx`

- [ ] **Step 1: Add failing repository tests**

Extend the existing message-deletion test to call:

```rust
let removed = group_repo
    .delete_group_essence_message("20001", &essence.essence_id)
    .await?
    .unwrap();
assert_eq!(removed.essence_id, essence.essence_id);
assert!(group_repo.list_group_essence_messages("20001").await?.is_empty());
```

Add the same assertion after marking a source message recalled.

- [ ] **Step 2: Verify essence tests are RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml group_essence -- --nocapture
```

Expected: compile failure because delete-by-essence method does not exist.

- [ ] **Step 3: Implement repository delete with snapshot**

Use a transaction to select the essence row with a left join to messages,
delete by `group_id + essence_id`, then return the selected entity with
`is_set = false`.

- [ ] **Step 4: Split service set/unset branches**

Change the command/service input to include optional `message_id` and
`essence_id`. Enforce:

```rust
if is_set {
    let message_id = message_id.ok_or_else(...)?;
    // Existing source/recalled validation and upsert.
} else {
    let essence_id = essence_id.ok_or_else(...)?;
    self.repo
        .delete_group_essence_message(&group_id, &essence_id)
        .await?
        .ok_or_else(|| AppError::not_found(...))?
}
```

Emit the existing update event from the returned entity.

- [ ] **Step 5: Update frontend identifiers**

The mutation accepts `messageId?: string` and `essenceId?: string`. The context
menu sends `messageId`; the essence panel sends `essenceId`.

- [ ] **Step 6: Verify essence tests are GREEN**

Run the Step 2 command and `bun run build`. Expected: PASS.

### Task 4: Revoke Standalone Window Access on Membership Loss

**Files:**
- Modify: `src-tauri/src/commands/chat/group/window.rs`
- Modify: `src-tauri/src/commands/chat/group/management.rs`
- Modify: `src-tauri/src/services/group/management.rs`
- Modify: `src/types/event.ts`
- Modify: `src/lib/query/groups.ts`
- Modify: `src/lib/query/event-handlers.ts`
- Modify: `src/lib/query/__tests__/event-handlers.test.ts`
- Modify: `src/components/chat/chat-event-bus-provider.tsx`
- Modify: `src/components/chat/__tests__/chat-event-bus-provider.test.tsx`

- [ ] **Step 1: Add failing Rust window-policy tests**

Add pure helper tests for:

```rust
assert_eq!(
    group_content_window_labels("10002", "20001"),
    ["group-files-10002-20001", "group-albums-10002-20001"]
);
```

Add an async service-level test proving `open_group_*` authorization rejects a
non-member before window creation by extracting membership validation into a
testable helper.

- [ ] **Step 2: Verify Rust revocation tests are RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml commands::chat::group::window::tests -- --nocapture
```

Expected: compile failure for the new helpers or authorization assertion.

- [ ] **Step 3: Implement backend authorization and cleanup**

Before `ensure_or_focus_window`, call
`services.group.ensure_group_member(&group_id, &user_id).await?`.

Expose helpers in the window module:

```rust
pub(crate) fn close_group_content_windows(
    app: &tauri::AppHandle,
    user_id: &str,
    group_id: &str,
) { ... }
```

Pass `AppHandle` to kick/leave/dissolve commands. Snapshot dissolve members
before deleting the group. Emit `InternalEvent::GroupMemberLeft` to remaining
and removed users, then close affected standalone windows.

- [ ] **Step 4: Verify Rust revocation tests are GREEN**

Run the Step 2 command. Expected: PASS.

- [ ] **Step 5: Add failing frontend event tests**

Extend `InternalEventPayload` test fixtures with:

```ts
{
  kind: "group_member_left",
  group_id: "20001",
  operator_user_id: "10001",
  target_user_id: "10002",
  time: 100,
}
```

Assert query removal for the target user and that a provider subscriber can
request current-window closure for a matching target/group.

- [ ] **Step 6: Verify frontend revocation tests are RED**

Run:

```bash
bun run test src/lib/query/__tests__/event-handlers.test.ts src/components/chat/__tests__/chat-event-bus-provider.test.tsx
```

Expected: failure because the event is missing from TypeScript and no stale
query removal/closure behavior exists.

- [ ] **Step 7: Implement frontend revocation handling**

Add `group_member_left` to the union. Add a query helper using
`queryClient.removeQueries` for files, folders, albums, photos,
announcements, and essence under the affected user/group.

In the provider, when the current user is the target, remove group content
queries and close the current standalone window when its label matches the
removed group. Keep main chat windows open.

- [ ] **Step 8: Verify frontend revocation tests are GREEN**

Run the Step 6 command. Expected: PASS.

### Task 5: Make Group Content Failures Visible

**Files:**
- Modify: `src/lib/mutations.ts`
- Modify: `src/components/group/group-file-browser.tsx`
- Modify: `src/components/group/group-album-browser.tsx`
- Modify: `src/components/chat/chat-main-panel.tsx`
- Modify: `src/components/group/__tests__/group-file-browser.test.tsx`
- Create: `src/components/group/__tests__/group-album-browser.test.tsx`

- [ ] **Step 1: Add failing frontend error tests**

Cover:

```ts
it("renders a retryable error instead of an empty file list", async () => { ... });
it("shows upload failure and handles the rejected promise", async () => { ... });
it("renders album query failure instead of an empty grid", async () => { ... });
it("shows a toast when opening a standalone window fails", async () => { ... });
```

- [ ] **Step 2: Verify error tests are RED**

Run:

```bash
bun run test src/components/group/__tests__/group-file-browser.test.tsx src/components/group/__tests__/group-album-browser.test.tsx
```

Expected: missing error UI/toasts make the assertions fail.

- [ ] **Step 3: Add mutation error ownership**

Add operation-specific `onError` to missing group-content mutations, including
upload/download file, create album, and upload photo. Preserve existing
delete/folder/announcement/essence handlers.

Wrap each `mutateAsync` event handler in `try/catch {}` because the mutation's
`onError` owns the toast.

- [ ] **Step 4: Add retryable query error UI**

Use `isError`, `error`, and `refetch` from file/folder/album/photo queries.
Before rendering empty collections, show an inline panel containing the
operation error and a Retry button.

Replace raw open-window `invoke` handlers with an async helper:

```ts
try {
  await invoke(COMMANDS.openGroupFilesWindow, ...);
} catch (error) {
  toast.error(`打开群文件失败：${error}`);
}
```

- [ ] **Step 5: Verify error tests are GREEN**

Run the Step 2 command. Expected: PASS with no unhandled rejection output.

### Task 6: Full Verification and Review

**Files:**
- Modify only if verification identifies a regression.

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --
bunx biome check --write src src-tauri/capabilities/group-content.json
```

- [ ] **Step 2: Run full frontend verification**

Run:

```bash
bun run test
bun run build
```

Expected: all tests pass and the production build exits 0.

- [ ] **Step 3: Run full Rust verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: all tests pass and clippy reports no warnings.

- [ ] **Step 4: Review the diff against the approved spec**

Run:

```bash
git diff 160e939 --check
git diff --stat 160e939
git status --short
```

Confirm all five findings are covered and unrelated untracked files remain
untouched.
