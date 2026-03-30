use crate::core::CoreContainer;
use crate::models::FriendRequestEntity;
use crate::models::RequestState;
use crate::services::ServiceHub;

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn create_friend_request(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    target_user_id: u64,
    comment: String,
) -> Result<FriendRequestEntity, String> {
    services
        .request
        .create_friend_request(&core, user_id, target_user_id, comment)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_friend_requests(
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
) -> Result<Vec<FriendRequestEntity>, String> {
    services
        .request
        .list_friend_requests(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn handle_friend_request(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    request_id: i64,
    state: RequestState,
) -> Result<FriendRequestEntity, String> {
    services
        .request
        .handle_friend_request(&core, user_id, request_id, state)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_friend(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    friend_user_id: u64,
) -> Result<(), String> {
    services
        .request
        .delete_friend(&core, user_id, friend_user_id)
        .await
        .into_command_result()
}
