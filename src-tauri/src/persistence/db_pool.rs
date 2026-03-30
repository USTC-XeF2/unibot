use std::fs;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use tauri::Manager;

use super::{GroupRepo, InteractionRepo, MessageRepo, UserRepo};

pub async fn init_sqlite_pool(app: &tauri::AppHandle) -> Result<SqlitePool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data dir: {err}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|err| format!("failed to create app data dir: {err}"))?;

    let db_path = app_data_dir.join("unibot.db");
    let connect_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .map_err(|err| format!("failed to connect sqlite: {err}"))?;

    UserRepo::init_schema(&pool)
        .await
        .map_err(|err| format!("failed to init users schema: {err}"))?;
    MessageRepo::init_schema(&pool)
        .await
        .map_err(|err| format!("failed to init messages schema: {err}"))?;
    GroupRepo::init_schema(&pool)
        .await
        .map_err(|err| format!("failed to init groups schema: {err}"))?;
    InteractionRepo::init_schema(&pool)
        .await
        .map_err(|err| format!("failed to init interaction schema: {err}"))?;

    Ok(pool)
}
