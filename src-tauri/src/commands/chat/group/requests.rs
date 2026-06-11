use crate::core::CoreContainer;
use crate::models::{GroupRequestEntity, GroupRequestType, RequestState};
use crate::services::ServiceHub;

use super::super::super::IntoCommandResult;

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
