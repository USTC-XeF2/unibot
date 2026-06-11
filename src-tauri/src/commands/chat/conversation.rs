use crate::models::ConversationState;
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
