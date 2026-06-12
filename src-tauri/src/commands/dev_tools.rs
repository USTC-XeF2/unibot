use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

use super::IntoCommandResult;
use crate::core::CoreContainer;
use crate::error::AppError;

const WRITE_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "REPLACE", "DROP", "CREATE", "ALTER", "TRUNCATE",
];

fn is_write_query(query: &str) -> bool {
    let normalized = query.to_uppercase();
    WRITE_KEYWORDS.iter().any(|kw| normalized.contains(kw))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: Option<u64>,
}

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
pub async fn execute_sql(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    query: String,
    allow_write: bool,
) -> Result<SqlQueryResult, String> {
    let result = async {
        if !allow_write && is_write_query(&query) {
            return Err(AppError::validation(
                "write queries require allow_write=true",
            ));
        }

        if query.trim().is_empty() {
            return Err(AppError::validation("query is empty"));
        }

        let rows = sqlx::query(&query)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| AppError::storage(format!("failed to execute sql: {e}")))?;

        if rows.is_empty() {
            return Ok(SqlQueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: None,
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
                    serde_json::Number::from_f64(v)
                        .map_or(serde_json::Value::Null, serde_json::Value::Number)
                } else if let Ok(v) = row.try_get::<String, _>(idx) {
                    serde_json::Value::String(v)
                } else if let Ok(v) = row.try_get::<bool, _>(idx) {
                    serde_json::Value::Bool(v)
                } else if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
                    serde_json::Value::String(format!("<BLOB {} bytes>", v.len()))
                } else {
                    serde_json::Value::Null
                };
                values.push(value);
            }
            result_rows.push(values);
        }

        Ok(SqlQueryResult {
            columns,
            rows: result_rows,
            rows_affected: None,
        })
    }
    .await;
    result.into_command_result()
}

#[tauri::command]
pub fn open_developer_tools(
    app: tauri::AppHandle,
    core: tauri::State<CoreContainer>,
) -> Result<bool, String> {
    let label = "developer-tools";

    if let Some(existing) = app.get_webview_window(label) {
        core.set_devtools_window_open(true);
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
    let devtools_window = tauri::WebviewWindowBuilder::new(&app, label, webview_url)
        .title("开发者工具")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .build()
        .map_err(|e| {
            AppError::internal(format!("failed to create developer tools window: {e}")).to_string()
        })?;

    core.set_devtools_window_open(true);

    let core_for_window_event = core.inner().clone();
    devtools_window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            core_for_window_event.set_devtools_window_open(false);
        }
    });

    let mut devtools_rx = core.subscribe_devtools_events();
    let app_handle = app.clone();
    let label_owned = label.to_string();
    let core_for_task = core.inner().clone();

    tauri::async_runtime::spawn(async move {
        loop {
            match devtools_rx.recv().await {
                Ok(devtools_event) => {
                    if app_handle.get_webview_window(&label_owned).is_none() {
                        core_for_task.set_devtools_window_open(false);
                        break;
                    }
                    if let Err(e) =
                        app_handle.emit_to(&label_owned, "devtools:event", &devtools_event)
                    {
                        tracing::error!(target: "dev_tools", "emit_to developer-tools failed: {}", e);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => {
                    core_for_task.set_devtools_window_open(false);
                    break;
                }
            }
        }
    });

    Ok(true)
}

#[tauri::command]
pub async fn get_db_schema(pool: tauri::State<'_, sqlx::SqlitePool>) -> Result<DbSchema, String> {
    fetch_db_schema(pool.inner()).await.into_command_result()
}

async fn fetch_db_schema(pool: &sqlx::SqlitePool) -> crate::error::AppResult<DbSchema> {
    fn validate_sqlite_identifier(name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    fn escape_sqlite_identifier(name: &str) -> String {
        name.replace('\'', "''")
    }

    // SQLite's table-valued pragma functions (e.g. `FROM pragma_table_info(?)`)
    // are not parsed correctly by the SQLite version used in sqlx's test
    // environment, so we use the direct PRAGMA statement form here. The table
    // name has already been validated as a plain SQLite identifier above, and
    // single quotes are escaped defensively.
    fn pragma_table_info_sql(table_name: &str) -> String {
        format!(
            "PRAGMA table_info('{}')",
            escape_sqlite_identifier(table_name)
        )
    }

    fn pragma_index_list_sql(table_name: &str) -> String {
        format!(
            "PRAGMA index_list('{}')",
            escape_sqlite_identifier(table_name)
        )
    }

    fn pragma_index_info_sql(index_name: &str) -> String {
        format!(
            "PRAGMA index_info('{}')",
            escape_sqlite_identifier(index_name)
        )
    }

    let tables: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::storage(format!("failed to list tables: {e}")))?;

    let mut result = Vec::new();
    for (name, sql) in tables {
        if !validate_sqlite_identifier(&name) {
            continue;
        }

        let columns: Vec<DbColumn> = sqlx::query(&pragma_table_info_sql(&name))
            .fetch_all(pool)
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

        let index_list: Vec<(i64, String, bool, String, bool)> =
            sqlx::query_as(&pragma_index_list_sql(&name))
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    AppError::storage(format!("failed to list indexes for {name}: {e}"))
                })?;

        let mut indexes = Vec::new();
        for (seq, idx_name, unique, origin, partial) in index_list {
            let index_columns: Vec<String> = sqlx::query(&pragma_index_info_sql(&idx_name))
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    AppError::storage(format!("failed to get index info for {idx_name}: {e}"))
                })?
                .into_iter()
                .map(|row: sqlx::sqlite::SqliteRow| row.get::<String, _>("name"))
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
    fetch_table_rows(pool.inner(), &table, limit)
        .await
        .into_command_result()
}

async fn fetch_table_rows(
    pool: &sqlx::SqlitePool,
    table: &str,
    limit: i64,
) -> crate::error::AppResult<TableRowPreview> {
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
        .fetch_all(pool)
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

#[cfg(test)]
mod tests {
    use super::{fetch_db_schema, fetch_table_rows};
    use crate::models::UserProfile;
    use crate::persistence::{UserRepo, migrator};

    fn make_profile(user_id: &str, nickname: &str) -> UserProfile {
        UserProfile {
            user_id: user_id.to_string(),
            nickname: nickname.to_string(),
            avatar: "".to_string(),
            signature: "".to_string(),
            account_status: Default::default(),
        }
    }

    #[sqlx::test]
    async fn get_db_schema_returns_migrated_tables(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let schema = fetch_db_schema(&pool).await.unwrap();
        assert!(!schema.tables.is_empty());

        let im_accounts_table = schema.tables.iter().find(|t| t.name == "im_accounts");
        assert!(im_accounts_table.is_some());

        let im_accounts_table = im_accounts_table.unwrap();
        assert!(
            im_accounts_table
                .columns
                .iter()
                .any(|c| c.name == "user_id")
        );
        assert!(!im_accounts_table.indexes.is_empty());
    }

    #[sqlx::test]
    async fn preview_table_rows_returns_data(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let repo = UserRepo::new(pool.clone());
        repo.upsert_user(&make_profile("10001", "Alice"))
            .await
            .unwrap();

        let preview = fetch_table_rows(&pool, "im_accounts", 10).await.unwrap();
        assert_eq!(preview.rows.len(), 1);
        assert!(
            preview
                .columns
                .iter()
                .any(|c| c == "user_id" || c == "nickname")
        );
    }

    #[sqlx::test]
    async fn preview_table_rows_rejects_invalid_limit(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = fetch_table_rows(&pool, "im_accounts", 0).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("limit must be between 1 and 1000")
        );

        let result = fetch_table_rows(&pool, "im_accounts", 1001).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("limit must be between 1 and 1000")
        );
    }

    #[sqlx::test]
    async fn preview_table_rows_rejects_invalid_table_name(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = fetch_table_rows(&pool, "im_accounts; DROP TABLE im_accounts;", 10).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid table name")
        );
    }
}
