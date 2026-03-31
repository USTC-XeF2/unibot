use crate::core::CoreContainer;
use crate::models::{
    MessageEntity, MessageReactionEntity, MessageSegment, MessageSource, PokeEntity,
};
use crate::services::{SendMessageResult, ServiceHub};

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn send_message(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    source: MessageSource,
    content: Vec<MessageSegment>,
) -> Result<SendMessageResult, String> {
    services
        .message
        .send(&core, user_id, source, content)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_message_history(
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    source: MessageSource,
    limit: Option<usize>,
) -> Result<Vec<SendMessageResult>, String> {
    services
        .message
        .list_history(user_id, source, limit.unwrap_or(50))
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn recall_message(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    message_id: i64,
) -> Result<MessageEntity, String> {
    services
        .message
        .recall_message(&core, user_id, message_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn react_to_message(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    message_id: i64,
    face_id: String,
    is_add: bool,
) -> Result<MessageReactionEntity, String> {
    services
        .interaction
        .react_to_message(&core, user_id, message_id, face_id, is_add)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn poke_user(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    source: MessageSource,
    target_user_id: u64,
) -> Result<PokeEntity, String> {
    services
        .interaction
        .poke(&core, user_id, source, target_user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_poke_history(
    services: tauri::State<'_, ServiceHub>,
    user_id: u64,
    source: MessageSource,
    limit: Option<usize>,
) -> Result<Vec<PokeEntity>, String> {
    services
        .interaction
        .list_poke_history(user_id, source, limit.unwrap_or(50))
        .await
        .into_command_result()
}
