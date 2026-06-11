use crate::core::CoreContainer;
use crate::models::{GroupMemberProfile, GroupProfile};
use crate::services::ServiceHub;

use super::super::super::IntoCommandResult;

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
