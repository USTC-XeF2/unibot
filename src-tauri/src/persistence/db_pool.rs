use std::fs;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tauri::Manager;

use super::migrator;

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
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .map_err(|err| format!("failed to connect sqlite: {err}"))?;

    migrator::run_migrations(&pool)
        .await
        .map_err(|err| format!("database migration failed: {err}"))?;

    Ok(pool)
}
