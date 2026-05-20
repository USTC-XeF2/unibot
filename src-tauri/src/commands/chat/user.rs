use crate::core::CoreContainer;
use crate::models::{DbId, GroupProfile, UserProfile};
use crate::services::ServiceHub;

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn update_user_profile(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    nickname: Option<String>,
    avatar: Option<String>,
    signature: Option<String>,
) -> Result<UserProfile, String> {
    services
        .user
        .update_user_profile(user_id, nickname, avatar, signature)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_friends(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<DbId>, String> {
    services
        .user
        .list_friends(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_user_groups(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<GroupProfile>, String> {
    services
        .group
        .list_user_groups(&core, user_id)
        .await
        .into_command_result()
}