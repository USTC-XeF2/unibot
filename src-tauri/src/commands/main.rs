use crate::core::CoreContainer;
use crate::models::{GroupProfile, UserProfile};
use crate::services::ServiceHub;

use super::IntoCommandResult;

#[tauri::command]
pub async fn register_user(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    nickname: String,
    avatar: String,
    signature: String,
) -> Result<UserProfile, String> {
    services
        .user
        .register_user(&core, user_id, nickname, avatar, signature)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_users(
    services: tauri::State<'_, ServiceHub>,
) -> Result<Vec<UserProfile>, String> {
    services.user.list_users().await.into_command_result()
}

#[tauri::command]
pub async fn list_groups(
    services: tauri::State<'_, ServiceHub>,
) -> Result<Vec<GroupProfile>, String> {
    services.group.list_groups().await.into_command_result()
}

#[tauri::command]
pub async fn delete_user(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
) -> Result<(), String> {
    services
        .user
        .delete_user(&core, user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn open_user_chat_window(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
) -> Result<(), String> {
    let inferred_nickname = services
        .user
        .get_user_by_id(user_id)
        .await
        .map_err(|err| err.to_string())?
        .map(|profile| profile.nickname);

    core.open_user_chat_window(app, user_id, inferred_nickname)
        .map_err(|err| err.to_string())?;

    Ok(())
}
