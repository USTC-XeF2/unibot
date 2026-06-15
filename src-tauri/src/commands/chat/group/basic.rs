use crate::core::CoreContainer;
use crate::models::{
    GroupCategoryEntity, GroupEventEntity, GroupMemberProfile, GroupProfile, GroupWholeMuteState,
};
use crate::services::{MuteGroupMemberResult, ServiceHub};

use super::super::super::IntoCommandResult;

#[tauri::command]
pub async fn list_groups(
    services: tauri::State<'_, ServiceHub>,
) -> Result<Vec<GroupProfile>, String> {
    services.group.list_groups().await.into_command_result()
}

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
    limit: Option<i64>,
) -> Result<Vec<GroupEventEntity>, String> {
    let limit = limit.unwrap_or(50).clamp(1, 1000);
    services
        .group
        .list_group_event_history(user_id, group_id, limit)
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
pub async fn list_group_categories(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<GroupCategoryEntity>, String> {
    services
        .group
        .list_group_categories(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn create_group_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    name: String,
) -> Result<GroupCategoryEntity, String> {
    services
        .group
        .create_group_category(user_id, name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_group_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    category_id: String,
) -> Result<(), String> {
    services
        .group
        .delete_group_category(user_id, category_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_group_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
    category_id: Option<String>,
) -> Result<(), String> {
    services
        .group
        .set_group_category(user_id, group_id, category_id)
        .await
        .into_command_result()
}
