use crate::commands::IntoCommandResult;
use crate::persistence::{PacketFilters, ProtocolPacketRecord};
use crate::services::ServiceHub;

use tauri::Manager;

#[tauri::command]
pub async fn list_protocol_packets(
    services: tauri::State<'_, ServiceHub>,
    filters: PacketFilters,
) -> Result<Vec<ProtocolPacketRecord>, String> {
    services
        .packet
        .list_packets(filters)
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
