use crate::persistence::{PacketRepo, ProtocolPacketRecord};
use tauri::Manager;

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_protocol_packets(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    bot_id: Option<String>,
    direction: Option<String>,
    action_name: Option<String>,
    since: Option<u64>,
    until: Option<u64>,
    is_error: Option<bool>,
    before: Option<u64>,
    limit: Option<i64>,
) -> Result<Vec<ProtocolPacketRecord>, String> {
    let repo = PacketRepo::new(pool.inner().clone());
    let limit = limit.unwrap_or(100).min(1000);
    repo.list_packets(
        bot_id.as_deref(),
        direction.as_deref(),
        action_name.as_deref(),
        since,
        until,
        is_error,
        before,
        limit,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_protocol_packet(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    packet_id: String,
) -> Result<String, String> {
    let repo = PacketRepo::new(pool.inner().clone());
    let packet = repo
        .get_packet_by_id(&packet_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "packet not found".to_string())?;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let file_path = app_data_dir.join(&packet.file_path);

    tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("failed to read packet file: {}", e))
}
