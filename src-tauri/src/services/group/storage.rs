use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Sanitize a file name by replacing path separators, control characters,
/// and directory traversal sequences.
/// Falls back to `file_id` if the sanitized name is empty.
pub fn sanitize_file_name(name: &str, file_id: &str) -> String {
    // Replace path separators and null byte
    let mut sanitized = name
        .replace(['/', '\\', '\0'], "_")
        .replace("..", "_")
        .replace('\n', "_")
        .replace('\r', "_");

    // Replace other control characters (0x01-0x1F and 0x7F)
    sanitized = sanitized
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect();

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
    let mut buf = [0u8; 8192];

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
    let full = app_data_dir.join(file_path);
    let canonical = tokio::fs::canonicalize(&full)
        .await
        .map_err(|e| AppError::storage(format!("file not found on disk: {e}")))?;

    let allowed_prefix = tokio::fs::canonicalize(app_data_dir.join("groups"))
        .await
        .unwrap_or_else(|_| app_data_dir.join("groups"));

    if !canonical.starts_with(&allowed_prefix) {
        return Err(AppError::validation("file path escapes allowed directory"));
    }

    Ok(canonical)
}

/// Delete a file from disk. Logs errors but never fails.
pub async fn delete_group_file_disk(file_path: &str, app_data_dir: &Path) {
    let full = app_data_dir.join(file_path);
    if let Err(e) = tokio::fs::remove_file(&full).await {
        eprintln!(
            "failed to delete group file from disk {}: {}",
            full.display(),
            e
        );
    }
}
