use crate::logging;
use tauri::Manager;

#[derive(serde::Serialize)]
pub struct SystemLogEntry {
    pub ts: u64,
    pub level: String,
    pub target: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
pub struct LogSettings {
    pub level: String,
    pub retention_days: i64,
}

#[derive(serde::Serialize)]
pub struct LogCleanupResult {
    pub deleted_files: usize,
}

#[tauri::command]
pub async fn list_system_logs(
    app: tauri::AppHandle,
    since: Option<u64>,
    before: Option<u64>,
    limit: Option<usize>,
    keyword: Option<String>,
    levels: Option<Vec<String>>,
) -> Result<Vec<SystemLogEntry>, String> {
    let log_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("logs");

    let limit = limit.unwrap_or(100).min(1000);
    let levels = levels.unwrap_or_default();

    let values =
        logging::read_system_logs(&log_dir, since, before, limit, keyword.as_deref(), &levels)
            .await
            .map_err(|e| e.to_string())?;

    let entries: Vec<SystemLogEntry> = values
        .into_iter()
        .filter_map(|v| {
            let ts = v.get("ts")?.as_u64()?;
            let level = v.get("level")?.as_str()?.to_string();
            let target = v.get("target")?.as_str()?.to_string();
            let msg = v.get("msg")?.as_str()?.to_string();
            let fields = v.get("fields").cloned();
            Some(SystemLogEntry {
                ts,
                level,
                target,
                msg,
                fields,
            })
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn get_log_settings(
    services: tauri::State<'_, crate::services::ServiceHub>,
) -> Result<LogSettings, String> {
    let level = services.settings.get_log_level().await;
    let retention_days = services.settings.get_log_retention_days().await;
    Ok(LogSettings {
        level,
        retention_days,
    })
}

#[tauri::command]
pub async fn set_log_level(
    services: tauri::State<'_, crate::services::ServiceHub>,
    level: String,
) -> Result<(), String> {
    services
        .settings
        .set_log_level(&level)
        .await
        .map_err(|e| e.to_string())?;

    logging::set_log_level(&level).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn set_log_retention_days(
    services: tauri::State<'_, crate::services::ServiceHub>,
    days: i64,
) -> Result<(), String> {
    services
        .settings
        .set_log_retention_days(days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn trigger_log_cleanup(
    app: tauri::AppHandle,
    services: tauri::State<'_, crate::services::ServiceHub>,
) -> Result<LogCleanupResult, String> {
    let log_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("logs");
    let retention_days = services.settings.get_log_retention_days().await;

    let deleted = logging::cleanup_old_logs(&log_dir, retention_days)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LogCleanupResult {
        deleted_files: deleted,
    })
}
