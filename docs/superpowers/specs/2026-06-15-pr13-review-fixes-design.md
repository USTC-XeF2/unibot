# PR #13 Review Fixes Design

## Goal

Resolve every still-valid item from the second review of PR #13 without
expanding the group-content feature beyond its current scope.

The first review's seven findings were already addressed by commit `160e939`.
This design covers the five remaining findings:

1. Downloading a group file must write a user-selected destination file.
2. Removed members must lose access to standalone group-content windows.
3. Disk deletion must reject persisted paths outside the group-content root.
4. An essence record must remain removable after its source message is
   recalled or deleted.
5. Group-content operations and queries must expose failures to the user.

## Architecture

The fixes stay within the existing Tauri command, service, repository, React
Query, and standalone-window boundaries. Security checks remain authoritative
in Rust. React is responsible for destination selection, progress/error
presentation, and clearing stale UI state.

No new dependency, protocol, or general-purpose abstraction is introduced.

## Download Flow

The file browser opens the Tauri save dialog with the stored file name as the
default destination. Cancellation performs no command.

The frontend passes the selected absolute destination path to
`download_group_file`. The service:

1. Confirms the requesting user is still a group member.
2. Loads the file and verifies that it belongs to the requested group.
3. Resolves and validates the persisted source path under
   `$APPDATA/groups/`.
4. Rejects a relative destination or a destination that is the source file.
5. Copies the source file to the selected destination.
6. Atomically increments `download_count` with one SQL `UPDATE`.
7. Returns the destination path for the success toast.

The filesystem copy and database update cannot form one cross-resource
transaction. The count is therefore updated only after a successful copy. A
database failure after copying is reported as a failure and does not claim a
successful counted download.

The group-content capability gains only `dialog:allow-save`.

## Membership Revocation

Opening or focusing a group-content window rechecks group membership before
touching the window. This closes the current gap where a non-member can invoke
the open command directly.

Kick, leave, and dissolve commands receive the Tauri app handle. After the
membership change succeeds:

- Kick closes both standalone windows for the removed user and group.
- Leave closes both standalone windows for the leaving user and group.
- Dissolve snapshots the member list before deletion and closes both windows
  for every former member.

Kick and leave also emit the existing `GroupMemberLeft` internal event to the
removed user and remaining members. The frontend type is extended for this
already-defined Rust event. Query invalidation removes group-content query
data for the affected user. Standalone windows additionally close themselves
when they receive a matching removal event, while backend closure remains the
authoritative fallback.

Query failure states replace cached content with an explicit error panel. A
removed member therefore cannot continue interacting with stale data if a
window-close call is delayed or fails.

This design does not claim to revoke operating-system access to bytes already
read by a local user. It revokes application-level access and prevents future
authorized queries or window creation.

## Safe Disk Deletion

`delete_group_file_disk` becomes the single validation boundary for all file,
photo, and album cleanup paths.

Before deletion it:

1. Rejects empty paths, absolute paths, and paths containing parent-directory,
   root, or platform-prefix components.
2. Resolves the candidate against `app_data_dir`.
3. Treats a genuinely missing in-root file as an idempotent success.
4. Canonicalizes an existing candidate and `$APPDATA/groups/`.
5. Rejects candidates outside the canonical group-content root, including
   symlink escapes.
6. Deletes the canonical file.

Regression tests cover absolute paths, `../` traversal, symlink escape where
supported, valid deletion, and missing valid files.

## Essence Removal

Setting and unsetting essence records use different identifiers:

- Setting requires `message_id` and continues validating group ownership and
  recall state of the source message.
- Unsetting requires `essence_id` and deletes by
  `group_id + essence_id`, without loading the source message.

The command and frontend mutation accept the identifier appropriate to the
operation. The essence panel sends `essence_id`; the message context menu
continues sending `message_id`.

The repository returns a snapshot of the deleted essence row so the service
can emit the existing `GroupEssenceUpdated` event even when `message_id` has
become `NULL`. The serialized event uses an empty message ID for that legacy
shape, while `essence_id` remains the stable identity.

Tests cover cancellation after recall and after physical message deletion.

## Error Handling

All group-content mutations gain operation-specific `onError` toasts:

- Upload/download file
- Create/delete/rename folder
- Create/delete album
- Upload/delete photo
- Open file/albums window
- Announcement and essence operations

Handlers that await `mutateAsync` catch rejection to avoid unhandled promises;
the mutation owns the user-facing error toast so errors are shown once.

File, folder, album, and photo queries expose `isError` and `error`. Their
components render a retryable error state instead of defaulting failures to an
empty collection. Empty-state UI is shown only after a successful empty query.

## Testing

Implementation follows red-green-refactor per finding.

Rust tests verify:

- Source-to-destination copy and download-count increment.
- No count increment when copying fails.
- Persisted delete-path traversal and absolute-path rejection.
- Valid and missing-path deletion behavior.
- Essence removal by `essence_id` after recall and source deletion.
- Membership validation before opening standalone windows.
- Pure window-label selection for kick, leave, and dissolve cleanup.

Frontend tests verify:

- Save-dialog cancellation invokes no download command.
- A selected destination is passed to the command and success is shown only
  after command completion.
- Download/upload rejection is visible and does not create an unhandled
  promise.
- File and album query failures render error UI rather than empty UI.
- A matching `group_member_left` event clears queries and requests window
  closure.
- Opening standalone windows reports invoke failures.

Final verification runs formatting, focused tests during each cycle, then the
complete frontend build/test suite and Rust test/clippy suite.
