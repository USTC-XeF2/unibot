use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Sanitize a file name by replacing path separators, control characters,
/// and directory traversal sequences.
/// Falls back to `file_id` if the sanitized name is empty.
pub fn sanitize_file_name(name: &str, file_id: &str) -> String {
    // Replace path separators and null byte first so that `..` cannot form
    // a path component boundary later.
    let mut sanitized = name
        .replace(['/', '\\', '\0'], "_")
        .replace("..", "_")
        .replace(['\n', '\r'], "_");

    // Replace other control characters (0x01-0x1F and 0x7F)
    sanitized = sanitized
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect();

    // Defense in depth: any remaining ".." sequence (should not happen after
    // the replacement above) is collapsed.
    while sanitized.contains("..") {
        sanitized = sanitized.replace("..", "_");
    }

    let trimmed = sanitized.trim();
    if trimmed.is_empty() || trimmed == "." {
        file_id.to_string()
    } else {
        sanitized
    }
}

/// Copy a source file into `app_data_dir/groups/{group_id}/files/{file_id}_{sanitized_name}`
/// and return the relative path fragment.
pub async fn copy_file_to_groups_dir(
    src: &Path,
    group_id: &str,
    file_id: &str,
    file_name: &str,
    app_data_dir: &Path,
) -> AppResult<String> {
    // Defense in depth: ensure the source is a regular file and not inside the
    // application's own data directory. Callers are expected to have already
    // validated the path, but this guards against future command additions.
    let metadata = tokio::fs::metadata(src)
        .await
        .map_err(|e| AppError::validation(format!("source file is not accessible: {e}")))?;
    if !metadata.is_file() {
        return Err(AppError::validation("source path is not a regular file"));
    }

    let canonical_app_data = tokio::fs::canonicalize(app_data_dir)
        .await
        .unwrap_or_else(|_| app_data_dir.to_path_buf());
    if let Ok(canonical_src) = tokio::fs::canonicalize(src).await
        && canonical_src.starts_with(&canonical_app_data)
    {
        return Err(AppError::validation(
            "source file cannot be inside the application data directory",
        ));
    }

    let safe_name = sanitize_file_name(file_name, file_id);
    let dest_dir = app_data_dir.join("groups").join(group_id).join("files");
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .map_err(|e| AppError::storage(format!("failed to create groups dir: {e}")))?;

    let dest_filename = format!("{}_{}", file_id, safe_name);
    let dest_path = dest_dir.join(&dest_filename);

    tokio::fs::copy(src, &dest_path)
        .await
        .map_err(|e| AppError::storage(format!("failed to copy file: {e}")))?;

    let relative = PathBuf::from("groups")
        .join(group_id)
        .join("files")
        .join(dest_filename);

    Ok(relative.to_string_lossy().to_string())
}

/// Compute SHA-256 hex digest of a file (streaming).
pub async fn compute_sha256(path: &Path) -> AppResult<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| AppError::storage(format!("failed to open file for hash: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];

    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buf)
            .await
            .map_err(|e| AppError::storage(format!("failed to read file for hash: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Validate that a stored file_path is within `app_data_dir/groups/`.
/// Returns the canonical absolute path on success.
pub async fn validate_group_file_path(file_path: &str, app_data_dir: &Path) -> AppResult<PathBuf> {
    validate_stored_group_file_path(file_path)?;

    let full = app_data_dir.join(file_path);
    let canonical = tokio::fs::canonicalize(&full)
        .await
        .map_err(|e| AppError::storage(format!("file not found on disk: {e}")))?;

    let allowed_prefix = tokio::fs::canonicalize(app_data_dir.join("groups"))
        .await
        .map_err(|e| AppError::storage(format!("groups dir not accessible: {e}")))?;

    if !canonical.starts_with(&allowed_prefix) {
        return Err(AppError::validation("file path escapes allowed directory"));
    }

    Ok(canonical)
}

fn validate_stored_group_file_path(file_path: &str) -> AppResult<()> {
    if file_path.trim().is_empty() {
        return Err(AppError::validation("file path is empty"));
    }

    let path = Path::new(file_path);
    if path.is_absolute() {
        return Err(AppError::validation("file path must be relative"));
    }

    let mut components = path.components();
    if components.next() != Some(Component::Normal("groups".as_ref())) {
        return Err(AppError::validation(
            "file path must be inside the groups directory",
        ));
    }

    if components.any(|component| !matches!(component, Component::Normal(_))) {
        return Err(AppError::validation(
            "file path contains invalid path components",
        ));
    }

    Ok(())
}

pub async fn copy_group_file_to_destination(
    file_path: &str,
    destination_path: &Path,
    app_data_dir: &Path,
) -> AppResult<PathBuf> {
    let source = validate_group_file_path(file_path, app_data_dir).await?;

    if !destination_path.is_absolute() {
        return Err(AppError::validation(
            "download destination must be an absolute path",
        ));
    }

    let resolved_destination = if tokio::fs::symlink_metadata(destination_path).await.is_ok() {
        tokio::fs::canonicalize(destination_path)
            .await
            .map_err(|e| {
                AppError::storage(format!("failed to resolve download destination: {e}"))
            })?
    } else {
        let parent = destination_path
            .parent()
            .ok_or_else(|| AppError::validation("download destination has no parent directory"))?;
        let file_name = destination_path
            .file_name()
            .ok_or_else(|| AppError::validation("download destination has no file name"))?;
        tokio::fs::canonicalize(parent)
            .await
            .map_err(|e| {
                AppError::storage(format!(
                    "download destination directory is not accessible: {e}"
                ))
            })?
            .join(file_name)
    };

    if resolved_destination == source {
        return Err(AppError::validation(
            "download destination cannot overwrite the stored source file",
        ));
    }

    tokio::fs::copy(&source, &resolved_destination)
        .await
        .map_err(|e| AppError::storage(format!("failed to copy group file: {e}")))?;

    Ok(resolved_destination)
}

/// Delete a file from disk. Returns an error if the file exists but could not be removed.
pub async fn delete_group_file_disk(file_path: &str, app_data_dir: &Path) -> AppResult<()> {
    validate_stored_group_file_path(file_path)?;

    let candidate = app_data_dir.join(file_path);
    match tokio::fs::symlink_metadata(&candidate).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(AppError::storage(format!(
                "failed to inspect group file {}: {e}",
                candidate.display()
            )));
        }
        Ok(_) => {}
    }

    let canonical_groups = tokio::fs::canonicalize(app_data_dir.join("groups"))
        .await
        .map_err(|e| AppError::storage(format!("groups dir not accessible: {e}")))?;
    let canonical_candidate = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|e| AppError::storage(format!("failed to resolve group file path: {e}")))?;

    if !canonical_candidate.starts_with(&canonical_groups) {
        return Err(AppError::validation("file path escapes allowed directory"));
    }

    match tokio::fs::remove_file(&canonical_candidate).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::storage(format!(
            "failed to delete group file {} from disk: {e}",
            canonical_candidate.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("unibot-storage-test-{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn delete_rejects_absolute_and_parent_paths() {
        let app_data_dir = temp_app_data_dir();
        tokio::fs::create_dir_all(app_data_dir.join("groups"))
            .await
            .unwrap();

        assert!(
            delete_group_file_disk("/tmp/unibot-outside-file", &app_data_dir)
                .await
                .is_err()
        );
        assert!(
            delete_group_file_disk("../unibot-outside-file", &app_data_dir)
                .await
                .is_err()
        );

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    }

    #[tokio::test]
    async fn delete_removes_valid_file_and_allows_missing_valid_file() {
        let app_data_dir = temp_app_data_dir();
        let files_dir = app_data_dir.join("groups/g1/files");
        tokio::fs::create_dir_all(&files_dir).await.unwrap();
        let stored_path = "groups/g1/files/f.txt";
        tokio::fs::write(app_data_dir.join(stored_path), b"content")
            .await
            .unwrap();

        delete_group_file_disk(stored_path, &app_data_dir)
            .await
            .unwrap();
        assert!(!app_data_dir.join(stored_path).exists());

        delete_group_file_disk("groups/g1/files/missing.txt", &app_data_dir)
            .await
            .unwrap();

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn delete_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let app_data_dir = temp_app_data_dir();
        let outside_dir =
            std::env::temp_dir().join(format!("unibot-storage-outside-{}", uuid::Uuid::new_v4()));
        let files_dir = app_data_dir.join("groups/g1/files");
        tokio::fs::create_dir_all(&files_dir).await.unwrap();
        tokio::fs::create_dir_all(&outside_dir).await.unwrap();
        let outside_file = outside_dir.join("outside.txt");
        tokio::fs::write(&outside_file, b"keep").await.unwrap();
        symlink(&outside_file, files_dir.join("link.txt")).unwrap();

        assert!(
            delete_group_file_disk("groups/g1/files/link.txt", &app_data_dir)
                .await
                .is_err()
        );
        assert!(outside_file.exists());

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
        let _ = tokio::fs::remove_dir_all(outside_dir).await;
    }

    #[tokio::test]
    async fn copy_group_file_to_destination_copies_contents() {
        let app_data_dir = temp_app_data_dir();
        let files_dir = app_data_dir.join("groups/g1/files");
        tokio::fs::create_dir_all(&files_dir).await.unwrap();
        let stored_path = "groups/g1/files/report.txt";
        tokio::fs::write(app_data_dir.join(stored_path), b"report")
            .await
            .unwrap();
        let destination =
            std::env::temp_dir().join(format!("unibot-download-{}.txt", uuid::Uuid::new_v4()));

        let copied = copy_group_file_to_destination(stored_path, &destination, &app_data_dir)
            .await
            .unwrap();

        assert_eq!(copied, destination);
        assert_eq!(tokio::fs::read(&copied).await.unwrap(), b"report");

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
        let _ = tokio::fs::remove_file(destination).await;
    }

    #[tokio::test]
    async fn copy_group_file_to_destination_rejects_relative_destination() {
        let app_data_dir = temp_app_data_dir();
        let files_dir = app_data_dir.join("groups/g1/files");
        tokio::fs::create_dir_all(&files_dir).await.unwrap();
        let stored_path = "groups/g1/files/report.txt";
        tokio::fs::write(app_data_dir.join(stored_path), b"report")
            .await
            .unwrap();

        assert!(
            copy_group_file_to_destination(
                stored_path,
                Path::new("relative-download.txt"),
                &app_data_dir,
            )
            .await
            .is_err()
        );

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    }

    #[tokio::test]
    async fn copy_group_file_to_destination_rejects_source_as_destination() {
        let app_data_dir = temp_app_data_dir();
        let files_dir = app_data_dir.join("groups/g1/files");
        tokio::fs::create_dir_all(&files_dir).await.unwrap();
        let stored_path = "groups/g1/files/report.txt";
        let source = app_data_dir.join(stored_path);
        tokio::fs::write(&source, b"report").await.unwrap();

        assert!(
            copy_group_file_to_destination(stored_path, &source, &app_data_dir)
                .await
                .is_err()
        );

        let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    }
}
