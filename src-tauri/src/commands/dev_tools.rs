use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

use super::IntoCommandResult;
use crate::core::CoreContainer;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbColumn {
    pub cid: i64,
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbIndex {
    pub seq: i64,
    pub name: String,
    pub unique: bool,
    pub origin: String,
    pub partial: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTable {
    pub name: String,
    pub sql: Option<String>,
    pub columns: Vec<DbColumn>,
    pub indexes: Vec<DbIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSchema {
    pub tables: Vec<DbTable>,
}

#[tauri::command]
pub fn open_developer_tools(
    app: tauri::AppHandle,
    core: tauri::State<CoreContainer>,
) -> Result<bool, String> {
    let label = "developer-tools";

    if let Some(existing) = app.get_webview_window(label) {
        existing.show().map_err(|e| {
            AppError::internal(format!("failed to show developer tools window: {e}")).to_string()
        })?;
        existing.unminimize().map_err(|e| {
            AppError::internal(format!("failed to unminimize developer tools window: {e}"))
                .to_string()
        })?;
        existing.set_focus().map_err(|e| {
            AppError::internal(format!("failed to focus developer tools window: {e}")).to_string()
        })?;
        return Ok(false);
    }

    let webview_url = tauri::WebviewUrl::App("index.html#/developer-tools".into());
    let _ = tauri::WebviewWindowBuilder::new(&app, label, webview_url)
        .title("开发者工具")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .build()
        .map_err(|e| {
            AppError::internal(format!("failed to create developer tools window: {e}")).to_string()
        })?;

    let mut devtools_rx = core.subscribe_devtools_events();
    let app_handle = app.clone();
    let label_owned = label.to_string();

    tauri::async_runtime::spawn(async move {
        loop {
            match devtools_rx.recv().await {
                Ok(devtools_event) => {
                    if app_handle.get_webview_window(&label_owned).is_none() {
                        break;
                    }
                    if let Err(e) =
                        app_handle.emit_to(&label_owned, "devtools:event", &devtools_event)
                    {
                        tracing::error!(target: "dev_tools", "emit_to developer-tools failed: {}", e);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    Ok(true)
}

#[tauri::command]
pub async fn get_db_schema(pool: tauri::State<'_, sqlx::SqlitePool>) -> Result<DbSchema, String> {
    let pool = pool.inner().clone();
    let result: crate::error::AppResult<DbSchema> = async {
        let tables: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::storage(format!("failed to list tables: {e}")))?;

        let mut result = Vec::new();
        for (name, sql) in tables {
            let columns: Vec<DbColumn> = sqlx::query(
                "SELECT cid, name, type, notnull, dflt_value, pk FROM pragma_table_info(?)",
            )
            .bind(&name)
            .fetch_all(&pool)
            .await
            .map_err(|e| AppError::storage(format!("failed to get table info for {name}: {e}")))?
            .into_iter()
            .map(|row: sqlx::sqlite::SqliteRow| DbColumn {
                cid: row.get::<i64, _>("cid"),
                name: row.get::<String, _>("name"),
                type_name: row.get::<String, _>("type"),
                not_null: row.get::<bool, _>("notnull"),
                default_value: row.get::<Option<String>, _>("dflt_value"),
                primary_key: row.get::<bool, _>("pk"),
            })
            .collect();

            let index_list: Vec<(i64, String, bool, String, bool)> = sqlx::query_as(
                "SELECT seq, name, \"unique\", origin, partial FROM pragma_index_list(?)",
            )
            .bind(&name)
            .fetch_all(&pool)
            .await
            .map_err(|e| AppError::storage(format!("failed to list indexes for {name}: {e}")))?;

            let mut indexes = Vec::new();
            for (seq, idx_name, unique, origin, partial) in index_list {
                let index_columns: Vec<String> =
                    sqlx::query_as("SELECT name FROM pragma_index_info(?)")
                        .bind(&idx_name)
                        .fetch_all(&pool)
                        .await
                        .map_err(|e| {
                            AppError::storage(format!("failed to get index info for {idx_name}: {e}"))
                        })?
                        .into_iter()
                        .map(|(name,): (String,)| name)
                        .collect();

                indexes.push(DbIndex {
                    seq,
                    name: idx_name,
                    unique,
                    origin,
                    partial,
                    columns: index_columns,
                });
            }

            result.push(DbTable {
                name,
                sql,
                columns,
                indexes,
            });
        }

        Ok(DbSchema { tables: result })
    }
    .await;
    result.into_command_result()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRowPreview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[tauri::command]
pub async fn preview_table_rows(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    table: String,
    limit: i64,
) -> Result<TableRowPreview, String> {
    let pool = pool.inner().clone();
    let result: crate::error::AppResult<TableRowPreview> = async {
        // Basic allow-list validation: only permit simple identifiers
        if !table.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AppError::validation(format!("invalid table name: {table}")));
        }

        if limit <= 0 || limit > 1000 {
            return Err(AppError::validation(format!(
                "limit must be between 1 and 1000, got {limit}"
            )));
        }

        let sql = format!("SELECT * FROM \"{}\" LIMIT {}", table, limit);
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| AppError::storage(format!("failed to preview table {table}: {e}")))?;

        if rows.is_empty() {
            return Ok(TableRowPreview {
                columns: vec![],
                rows: vec![],
            });
        }

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        let mut result_rows = Vec::new();
        for row in rows {
            let mut values = Vec::new();
            for (idx, _) in columns.iter().enumerate() {
                let value: serde_json::Value = if let Ok(v) = row.try_get::<i64, _>(idx) {
                    serde_json::Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<f64, _>(idx) {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into()),
                    )
                } else if let Ok(v) = row.try_get::<String, _>(idx) {
                    serde_json::Value::String(v)
                } else if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
                    serde_json::Value::String(format!("<BLOB: {} bytes>", v.len()))
                } else {
                    serde_json::Value::Null
                };
                values.push(value);
            }
            result_rows.push(values);
        }

        Ok(TableRowPreview {
            columns,
            rows: result_rows,
        })
    }
    .await;
    result.into_command_result()
}
