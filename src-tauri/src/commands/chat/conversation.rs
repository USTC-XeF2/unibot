use crate::models::{ConversationState, GroupCategoryEntity};
use crate::services::ServiceHub;

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn set_conversation_pinned(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    scene: String,
    peer_user_id: Option<String>,
    group_id: Option<String>,
    is_pinned: bool,
) -> Result<(), String> {
    services
        .conversation
        .set_conversation_pinned(user_id, scene, peer_user_id, group_id, is_pinned)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_conversation_muted(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    scene: String,
    peer_user_id: Option<String>,
    group_id: Option<String>,
    is_muted: bool,
) -> Result<(), String> {
    services
        .conversation
        .set_conversation_muted(user_id, scene, peer_user_id, group_id, is_muted)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_conversation_states(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<ConversationState>, String> {
    services
        .conversation
        .list_conversation_states(user_id)
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
