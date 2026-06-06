use crate::core::CoreContainer;
use crate::models::{GroupProfile, UserProfile};
use crate::services::ServiceHub;

use super::IntoCommandResult;
use sqlx::SqlitePool;
use tauri::Manager;

#[tauri::command]
pub async fn register_user(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    nickname: String,
    avatar: String,
    signature: String,
) -> Result<UserProfile, String> {
    services
        .user
        .register_user(&core, user_id, nickname, avatar, signature)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_users(
    services: tauri::State<'_, ServiceHub>,
) -> Result<Vec<UserProfile>, String> {
    services.user.list_users().await.into_command_result()
}

#[tauri::command]
pub async fn list_groups(
    services: tauri::State<'_, ServiceHub>,
) -> Result<Vec<GroupProfile>, String> {
    services.group.list_groups().await.into_command_result()
}

#[tauri::command]
pub async fn delete_user(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<(), String> {
    services
        .user
        .delete_user(&core, user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn open_user_chat_window(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<(), String> {
    let inferred_nickname = services
        .user
        .get_user_by_id(&user_id)
        .await
        .map_err(|err| err.to_string())?
        .map(|profile| profile.nickname);

    core.open_user_chat_window(app, user_id, inferred_nickname)
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[derive(serde::Serialize)]
pub struct DbStatus {
    schema_version: String,
    table_count: i64,
    db_size_bytes: u64,
    integrity_check: String,
    foreign_key_check: Vec<String>,
}

#[tauri::command]
pub async fn get_db_status(app: tauri::AppHandle) -> Result<DbStatus, String> {
    let result: crate::error::AppResult<DbStatus> = async {
        let pool = app.state::<SqlitePool>().inner().clone();

        let schema_version: String = sqlx::query_scalar(
            "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
        )
        .fetch_one(&pool)
        .await?;

        let table_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'")
                .fetch_one(&pool)
                .await?;

        let db_path = app
            .path()
            .app_data_dir()
            .map_err(|err| {
                crate::error::AppError::internal(format!("failed to get app data dir: {err}"))
            })?
            .join("unibot.db");
        let db_size_bytes = std::fs::metadata(&db_path)
            .map(|m| m.len())
            .map_err(|err| {
                crate::error::AppError::internal(format!("failed to read db metadata: {err}"))
            })?;

        let integrity_check: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await?;

        // PRAGMA foreign_key_check returns (table, rowid, parent, fkid) for each violation.
        // query_scalar returns the first column (table name) for each row.
        let fk_issues: Vec<String> = sqlx::query_scalar("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await?;

        Ok(DbStatus {
            schema_version,
            table_count,
            db_size_bytes,
            integrity_check,
            foreign_key_check: fk_issues,
        })
    }
    .await;

    result.into_command_result()
}
