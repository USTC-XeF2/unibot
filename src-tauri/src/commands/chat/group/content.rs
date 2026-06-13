use tauri::Manager;

use crate::core::CoreContainer;
use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity,
    GroupFolderEntity, GroupPhotoEntity,
};
use crate::services::ServiceHub;

use super::super::super::IntoCommandResult;

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
pub async fn upsert_group_file(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupFileEntity,
) -> Result<GroupFileEntity, String> {
    services
        .group
        .upsert_group_file(&app, &core, input)
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
pub async fn upload_group_file(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    app: tauri::AppHandle,
    user_id: String,
    group_id: String,
    parent_folder_id: Option<String>,
    file_name: String,
    source_path: String,
) -> Result<GroupFileEntity, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .group
        .upload_group_file(
            &app,
            &core,
            user_id,
            group_id,
            parent_folder_id,
            file_name,
            source_path,
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
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupAlbumEntity>, String> {
    services
        .group
        .list_group_albums(user_id, group_id)
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

    services
        .group
        .upload_group_photo(
            &app,
            &core,
            user_id,
            group_id,
            album_id,
            source_path,
            description,
            app_data_dir,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_photos(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    album_id: String,
) -> Result<Vec<GroupPhotoEntity>, String> {
    services
        .group
        .list_group_photos(user_id, group_id, album_id)
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
