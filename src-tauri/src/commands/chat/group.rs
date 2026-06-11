use tauri::Manager;

use crate::core::CoreContainer;
use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupEventEntity, GroupFileEntity,
    GroupFolderEntity, GroupMemberProfile, GroupProfile, GroupRequestEntity, GroupRequestType,
    GroupWholeMuteState, RequestState,
};
use crate::services::{MuteGroupMemberResult, ServiceHub};

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn upsert_group(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    group_name: String,
    max_member_count: u32,
    initial_member_user_ids: Vec<String>,
) -> Result<GroupProfile, String> {
    services
        .group
        .upsert_group(
            &core,
            user_id,
            group_id,
            group_name,
            max_member_count,
            initial_member_user_ids,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn upsert_group_member(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    target_user_id: String,
) -> Result<GroupMemberProfile, String> {
    services
        .group
        .upsert_group_member(&core, user_id, group_id, target_user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_members(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Vec<GroupMemberProfile>, String> {
    services
        .group
        .list_group_members(user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_event_history(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    limit: Option<usize>,
) -> Result<Vec<GroupEventEntity>, String> {
    services
        .group
        .list_group_event_history(user_id, group_id, limit.unwrap_or(50))
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn mute_group_member(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    target_user_id: String,
    duration_seconds: u64,
) -> Result<MuteGroupMemberResult, String> {
    services
        .group
        .mute_group_member(&core, user_id, group_id, target_user_id, duration_seconds)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_group_whole_mute(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    duration_seconds: u64,
) -> Result<GroupWholeMuteState, String> {
    services
        .group
        .set_group_whole_mute(&core, user_id, group_id, duration_seconds)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn get_group_whole_mute(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<Option<GroupWholeMuteState>, String> {
    services
        .group
        .get_group_whole_mute(user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn create_group_request(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    request_type: GroupRequestType,
    target_user_id: Option<String>,
    comment: Option<String>,
) -> Result<GroupRequestEntity, String> {
    services
        .group
        .create_group_request(
            &core,
            user_id,
            group_id,
            request_type,
            target_user_id,
            comment,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_group_requests(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<GroupRequestEntity>, String> {
    services
        .group
        .list_group_requests(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn handle_group_request(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    request_id: String,
    state: RequestState,
) -> Result<GroupRequestEntity, String> {
    services
        .group
        .handle_group_request(&core, user_id, request_id, state)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn kick_group_member(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    target_user_id: String,
) -> Result<(), String> {
    services
        .group
        .kick_group_member(&core, user_id, group_id, target_user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_group_member_role(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    target_user_id: String,
    is_admin: bool,
) -> Result<GroupMemberProfile, String> {
    services
        .group
        .set_group_member_role(&core, user_id, group_id, target_user_id, is_admin)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_group_member_title(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    target_user_id: String,
    title: String,
) -> Result<GroupMemberProfile, String> {
    services
        .group
        .set_group_member_title(&core, user_id, group_id, target_user_id, title)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn rename_group(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    group_name: String,
) -> Result<GroupProfile, String> {
    services
        .group
        .rename_group(&core, user_id, group_id, group_name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn leave_group(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<(), String> {
    services
        .group
        .leave_group(&core, user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn dissolve_group(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<(), String> {
    services
        .group
        .dissolve_group(&core, user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn upsert_group_announcement(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupAnnouncementEntity,
) -> Result<GroupAnnouncementEntity, String> {
    services
        .group
        .upsert_announcement(&core, input)
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
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupFolderEntity,
) -> Result<GroupFolderEntity, String> {
    services
        .group
        .upsert_group_folder(&core, input)
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
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    input: GroupFileEntity,
) -> Result<GroupFileEntity, String> {
    services
        .group
        .upsert_group_file(&core, input)
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
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    message_id: String,
    is_set: bool,
) -> Result<GroupEssenceMessageEntity, String> {
    services
        .group
        .set_group_essence_message(&core, user_id, group_id, message_id, is_set)
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
    file_id: String,
    parent_folder_id: String,
    file_name: String,
    file_size: u64,
    source_path: String,
) -> Result<GroupFileEntity, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;
    let src_path = std::path::Path::new(&source_path);

    let file_path = crate::services::group::storage::copy_file_to_groups_dir(
        src_path,
        &group_id,
        &file_id,
        &file_name,
        &app_data_dir,
    )
    .await
    .map_err(|e| e.to_string())?;

    let file_hash = crate::services::group::storage::compute_sha256(&app_data_dir.join(&file_path))
        .await
        .map_err(|e| e.to_string())?;

    let file = GroupFileEntity {
        file_id,
        group_id,
        parent_folder_id,
        file_name,
        file_size,
        file_hash: Some(file_hash),
        uploader_user_id: user_id.clone(),
        uploaded_at: crate::utils::now_ts(),
        expire_at: None,
        download_count: 0,
        file_path: Some(file_path),
    };

    services
        .group
        .upsert_group_file(&core, file)
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
        .delete_group_file(&core, user_id, group_id, file_id, app_data_dir)
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
