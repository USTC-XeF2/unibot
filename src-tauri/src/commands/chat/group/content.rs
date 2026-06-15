use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity,
    GroupFolderEntity, GroupPhotoEntity,
};
use crate::services::ServiceHub;

use super::super::super::IntoCommandResult;

/// Validates a user-selected source path before it is copied into the app.
///
/// Returns the canonicalized absolute path, or a validation error if the path
/// is empty, relative, points inside the application's data directory, does
/// not exist, or is not a regular file.
fn validate_source_path(source_path: &str, app_data_dir: &Path) -> AppResult<PathBuf> {
    if source_path.trim().is_empty() {
        return Err(AppError::validation("source path is empty"));
    }

    let path = Path::new(source_path);

    // Reject relative-looking paths early. `canonicalize` would resolve them,
    // but an explicit check gives a clearer error and guards against unexpected
    // working-directory resolution.
    if !path.is_absolute() {
        return Err(AppError::validation("source path must be an absolute path"));
    }

    let canonical = std::fs::canonicalize(path)
        .map_err(|_| AppError::validation("source path does not exist or is not accessible"))?;

    // canonicalize resolves symlinks and `..`; verify the result is still a
    // regular file and not a directory.
    let metadata = std::fs::metadata(&canonical)
        .map_err(|_| AppError::validation("source path is not readable"))?;
    if !metadata.is_file() {
        return Err(AppError::validation("source path is not a file"));
    }

    // Prevent exfiltration of the application's own data (database, keys, etc.).
    let canonical_app_data = std::fs::canonicalize(app_data_dir).unwrap_or_else(|_| {
        // If the app data dir itself cannot be canonicalized, fall back to the
        // absolute path as a best-effort guard.
        app_data_dir.to_path_buf()
    });
    if canonical.starts_with(&canonical_app_data) {
        return Err(AppError::validation(
            "source path cannot be inside the application data directory",
        ));
    }

    tracing::info!(
        target: "commands",
        "validated upload source path: {}",
        canonical.display()
    );

    Ok(canonical)
}

#[tauri::command]
pub async fn upsert_group_announcement(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupAnnouncementEntity,
) -> Result<GroupAnnouncementEntity, String> {
    services
        .group
        .upsert_announcement(&app, &core, input)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_announcement(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    announcement_id: String,
) -> Result<(), String> {
    services
        .group
        .delete_announcement(&app, &core, user_id, group_id, announcement_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_announcements(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupAnnouncementEntity>, String> {
    services
        .group
        .list_announcements(user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn upsert_group_folder(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupFolderEntity,
) -> Result<GroupFolderEntity, String> {
    services
        .group
        .upsert_group_folder(&app, &core, input)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_folders(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupFolderEntity>, String> {
    services
        .group
        .list_group_folders(user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_folder(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    folder_id: String,
) -> Result<(), String> {
    services
        .group
        .delete_group_folder(&app, &core, user_id, group_id, folder_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_files(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    parent_folder_id: Option<String>,
) -> Result<Vec<GroupFileEntity>, String> {
    services
        .group
        .list_group_files(user_id, group_id, parent_folder_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_group_essence_message(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    message_id: String,
    is_set: bool,
) -> Result<GroupEssenceMessageEntity, String> {
    services
        .group
        .set_group_essence_message(&app, &core, user_id, group_id, message_id, is_set)
        .await
        .into_command_result()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn upload_group_file(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    parent_folder_id: Option<String>,
    file_name: Option<String>,
    source_path: String,
) -> Result<GroupFileEntity, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    let source_path = validate_source_path(&source_path, &app_data_dir).into_command_result()?;

    let file_name = file_name
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            source_path
                .file_name()
                .and_then(|name| name.to_str().map(String::from))
        })
        .unwrap_or_else(|| "upload".to_string());

    let source_path_str = source_path
        .to_str()
        .ok_or_else(|| "source path contains invalid UTF-8".to_string())?
        .to_string();

    services
        .group
        .upload_group_file(
            &app,
            &core,
            user_id,
            group_id,
            parent_folder_id,
            file_name,
            source_path_str,
            app_data_dir,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn download_group_file(
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    file_id: String,
) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .download_group_file(user_id, group_id, file_id, app_data_dir)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_file(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    file_id: String,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .delete_group_file(&app, &core, user_id, group_id, file_id, app_data_dir)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_essence_messages(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupEssenceMessageEntity>, String> {
    services
        .group
        .list_group_essence_messages(user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn create_group_album(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    name: String,
) -> Result<GroupAlbumEntity, String> {
    services
        .group
        .create_group_album(&app, &core, user_id, group_id, name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_albums(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupAlbumEntity>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .list_group_albums(user_id, group_id, &app_data_dir)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_album(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    album_id: String,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .delete_group_album(&app, &core, user_id, group_id, album_id, app_data_dir)
        .await
        .into_command_result()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn upload_group_photo(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    album_id: String,
    source_path: String,
    description: Option<String>,
) -> Result<GroupPhotoEntity, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    let source_path = validate_source_path(&source_path, &app_data_dir).into_command_result()?;
    let source_path_str = source_path
        .to_str()
        .ok_or_else(|| "source path contains invalid UTF-8".to_string())?
        .to_string();

    services
        .group
        .upload_group_photo(
            &app,
            &core,
            user_id,
            group_id,
            album_id,
            source_path_str,
            description,
            app_data_dir,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_photos(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    album_id: String,
) -> Result<Vec<GroupPhotoEntity>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .list_group_photos(user_id, group_id, album_id, &app_data_dir)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_photo(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    photo_id: String,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .delete_group_photo(&app, &core, user_id, group_id, photo_id, app_data_dir)
        .await
        .into_command_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_app_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("unibot-test-app-data-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn validate_source_path_accepts_regular_file() {
        let app_data_dir = temp_app_data_dir();
        std::fs::create_dir_all(&app_data_dir).unwrap();
        let file_path = app_data_dir.join("../upload.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"hello").unwrap();

        let result = validate_source_path(file_path.to_str().unwrap(), &app_data_dir);
        assert!(result.is_ok(), "{result:?}");

        std::fs::remove_file(&file_path).unwrap();
        std::fs::remove_dir(&app_data_dir).unwrap();
    }

    #[test]
    fn validate_source_path_rejects_relative_path() {
        let app_data_dir = temp_app_data_dir();
        let result = validate_source_path("./file.txt", &app_data_dir);
        assert!(result.is_err());
    }

    #[test]
    fn validate_source_path_rejects_non_existent() {
        let app_data_dir = temp_app_data_dir();
        let result = validate_source_path("/non/existent/file.txt", &app_data_dir);
        assert!(result.is_err());
    }

    #[test]
    fn validate_source_path_rejects_directory() {
        let app_data_dir = temp_app_data_dir();
        std::fs::create_dir_all(&app_data_dir).unwrap();
        let dir_path = app_data_dir.join("../some-dir");
        std::fs::create_dir(&dir_path).unwrap();

        let result = validate_source_path(dir_path.to_str().unwrap(), &app_data_dir);
        assert!(result.is_err());

        std::fs::remove_dir(&dir_path).unwrap();
        std::fs::remove_dir(&app_data_dir).unwrap();
    }

    #[test]
    fn validate_source_path_rejects_inside_app_data_dir() {
        let app_data_dir = temp_app_data_dir();
        std::fs::create_dir_all(&app_data_dir).unwrap();
        let file_path = app_data_dir.join("secret.db");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"secret").unwrap();

        let result = validate_source_path(file_path.to_str().unwrap(), &app_data_dir);
        assert!(result.is_err());

        std::fs::remove_file(&file_path).unwrap();
        std::fs::remove_dir(&app_data_dir).unwrap();
    }

    #[test]
    fn validate_source_path_rejects_empty() {
        let app_data_dir = temp_app_data_dir();
        let result = validate_source_path("", &app_data_dir);
        assert!(result.is_err());
    }
}
