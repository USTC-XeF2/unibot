use crate::models::{BotProfile, DebugSession};
use crate::services::ServiceHub;

use super::IntoCommandResult;

#[tauri::command]
pub async fn create_bot(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    bound_user_id: String,
    display_name: String,
) -> Result<BotProfile, String> {
    services
        .bot
        .create_bot(&app, bound_user_id, display_name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_bots(services: tauri::State<'_, ServiceHub>) -> Result<Vec<BotProfile>, String> {
    services.bot.list_bots().await.into_command_result()
}

#[tauri::command]
pub async fn delete_bot(
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<(), String> {
    services.bot.delete_bot(bot_id).await.into_command_result()
}

#[tauri::command]
pub async fn start_bot(
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<DebugSession, String> {
    services.bot.start_bot(bot_id).await.into_command_result()
}

#[tauri::command]
pub async fn stop_bot(
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<(), String> {
    services.bot.stop_bot(bot_id).await.into_command_result()
}

#[tauri::command]
pub async fn list_debug_sessions(
    services: tauri::State<'_, ServiceHub>,
    bot_id: String,
) -> Result<Vec<DebugSession>, String> {
    services
        .bot
        .list_sessions(bot_id)
        .await
        .into_command_result()
}
