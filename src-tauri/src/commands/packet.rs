use crate::commands::IntoCommandResult;
use crate::persistence::ProtocolPacketRecord;
use crate::services::ServiceHub;

use tauri::Manager;

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_protocol_packets(
    services: tauri::State<'_, ServiceHub>,
    bot_id: Option<String>,
    direction: Option<String>,
    action_name: Option<String>,
    since: Option<u64>,
    until: Option<u64>,
    is_error: Option<bool>,
    before: Option<u64>,
    limit: Option<i64>,
) -> Result<Vec<ProtocolPacketRecord>, String> {
    let limit = limit.unwrap_or(100);
    services
        .packet
        .list_packets(
            bot_id,
            direction,
            action_name,
            since,
            until,
            is_error,
            before,
            limit,
        )
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn read_protocol_packet(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    packet_id: String,
) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;

    services
        .packet
        .read_packet(packet_id, app_data_dir)
        .await
        .into_command_result()
}
