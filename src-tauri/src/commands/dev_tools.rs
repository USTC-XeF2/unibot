use serde::{Deserialize, Serialize};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;
use tokio_stream::StreamExt;

use super::IntoCommandResult;
use crate::core::CoreContainer;
use crate::error::AppError;

const READ_KEYWORDS: &[&str] = &["SELECT", "EXPLAIN"];
const SAFE_PRAGMAS: &[&str] = &[
    // Schema introspection used by the schema panel.
    "table_info",
    "index_list",
    "index_info",
    "foreign_key_list",
    "index_xinfo",
    "table_xinfo",
    // Other read-only pragma introspection.
    "collation_list",
    "database_list",
    "function_list",
    "module_list",
    "pragma_list",
    "user_version",
    "application_id",
    "encoding",
    "page_size",
    "page_count",
    "freelist_count",
    "journal_mode",
    "synchronous",
    "temp_store",
    "locking_mode",
    "wal_autocheckpoint",
    "cache_size",
    "foreign_keys",
    "recursive_triggers",
    "case_sensitive_like",
    "legacy_alter_table",
    "reverse_unordered_selects",
    "query_only",
    "integrity_check",
    "quick_check",
];

/// Strip leading whitespace and simple SQL comments so that commented-out
/// write statements are still detected while "SELECT 'INSERT' ..." is not.
fn strip_leading_comments_and_whitespace(query: &str) -> String {
    let mut normalized = query.to_string();
    loop {
        normalized = normalized.trim_start().to_string();
        if normalized.starts_with("--") {
            if let Some(idx) = normalized.find('\n') {
                normalized = normalized[idx..].to_string();
                continue;
            }
            return String::new();
        }
        if normalized.starts_with("/*") {
            if let Some(idx) = normalized.find("*/") {
                normalized = normalized[idx + 2..].to_string();
                continue;
            }
            return String::new();
        }
        break;
    }
    normalized
}

fn first_token(query: &str) -> Option<String> {
    let end = query
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(query.len());
    if end == 0 {
        None
    } else {
        Some(query[..end].to_string())
    }
}

/// A query is read-only if, after stripping leading comments, it starts with
/// SELECT/EXPLAIN or with a known-safe PRAGMA that has no assignment.
/// Everything else (INSERT, UPDATE, DELETE, WITH, ATTACH, VACUUM, unsafe
/// PRAGMA, ...) is treated as a write and requires allow_write=true.
fn is_read_query(query: &str) -> bool {
    let normalized = strip_leading_comments_and_whitespace(query);
    if normalized.is_empty() {
        return false;
    }

    let token = match first_token(&normalized) {
        Some(t) => t,
        None => return false,
    };
    let token_upper = token.to_uppercase();

    if READ_KEYWORDS.contains(&token_upper.as_str()) {
        return true;
    }

    if token_upper != "PRAGMA" {
        return false;
    }

    let rest = normalized[token.len()..].trim_start();
    let pragma_name = match first_token(rest) {
        Some(t) => t,
        None => return false,
    };

    // Strip optional schema prefix, e.g. "main.table_info".
    let name = match pragma_name.rsplit_once('.') {
        Some((_, name)) => name.to_string(),
        None => pragma_name,
    }
    .to_lowercase();

    if !SAFE_PRAGMAS.contains(&name.as_str()) {
        return false;
    }

    // PRAGMA assignments like "PRAGMA journal_mode=WAL" are writes.
    !normalized.contains('=')
}

fn is_write_query(query: &str) -> bool {
    !is_read_query(query)
}

/// Convert a single SQLite row value to a JSON value.
///
/// Shared between the SQL executor and table row preview so that type handling
/// and placeholder formatting stay consistent. We inspect the actual SQLite
/// storage type to avoid converting a REAL into an integer 0, and decode into
/// `Option<T>` so that NULL values are preserved.
fn row_value_to_json(row: &sqlx::sqlite::SqliteRow, idx: usize) -> serde_json::Value {
    let raw = row.try_get_raw(idx).expect("column index should be valid");
    match raw.type_info().name() {
        "INTEGER" => row
            .try_get::<Option<i64>, _>(idx)
            .ok()
            .flatten()
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        "REAL" => row
            .try_get::<Option<f64>, _>(idx)
            .ok()
            .flatten()
            .and_then(|v| serde_json::Number::from_f64(v).map(serde_json::Value::Number))
            .unwrap_or(serde_json::Value::Null),
        "TEXT" => row
            .try_get::<Option<String>, _>(idx)
            .ok()
            .flatten()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "BLOB" => row
            .try_get::<Option<Vec<u8>>, _>(idx)
            .ok()
            .flatten()
            .map(|v| serde_json::Value::String(format!("<BLOB: {} bytes>", v.len())))
            .unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::String(format!("<UNKNOWN: {}>", raw.type_info().name())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: Option<u64>,
    pub truncated: bool,
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

/// Return true if the query contains a statement terminator (`;`) that is
/// outside of a string literal or SQL comment. This is intentionally a simple
/// scan rather than a full parser; it avoids rejecting inputs like
/// `SELECT ';'` or `SELECT 1 /* ; */` while still catching real multi-statement
/// inputs such as `SELECT 1; DROP TABLE users;`.
fn has_statement_terminator(query: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut chars = query.chars().peekable();

    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if in_single_quote {
            if c == '\\' {
                chars.next();
            } else if c == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        match c {
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                in_line_comment = true;
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line_comment = true;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block_comment = true;
            }
            '\'' => in_single_quote = true,
            ';' => return true,
            _ => {}
        }
    }

    false
}

async fn execute_sql_query(
    pool: &sqlx::SqlitePool,
    query: &str,
    allow_write: bool,
) -> crate::error::AppResult<SqlQueryResult> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("query is empty"));
    }

    // Reject multi-statement inputs so that prefix-based write detection cannot
    // be bypassed (e.g. "SELECT 1; DROP TABLE users;").
    if has_statement_terminator(trimmed) {
        return Err(AppError::validation("multiple statements are not allowed"));
    }

    let is_write = is_write_query(query);
    if is_write && !allow_write {
        return Err(AppError::validation(
            "write queries require allow_write=true",
        ));
    }

    if is_write {
        let result = sqlx::query(query)
            .execute(pool)
            .await
            .map_err(|e| AppError::storage(format!("failed to execute sql: {e}")))?;
        return Ok(SqlQueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: Some(result.rows_affected()),
            truncated: false,
        });
    }

    let mut stream = sqlx::query(query).fetch(pool);
    let mut rows = Vec::new();
    const MAX_ROWS: usize = 1000;

    while let Some(row) = stream
        .try_next()
        .await
        .map_err(|e| AppError::storage(format!("failed to execute sql: {e}")))?
    {
        rows.push(row);
        if rows.len() > MAX_ROWS {
            break;
        }
    }

    let truncated = rows.len() > MAX_ROWS;
    if truncated {
        rows.pop();
    }

    if rows.is_empty() {
        return Ok(SqlQueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: None,
            truncated: false,
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
            values.push(row_value_to_json(&row, idx));
        }
        result_rows.push(values);
    }

    Ok(SqlQueryResult {
        columns,
        rows: result_rows,
        rows_affected: None,
        truncated,
    })
}

#[tauri::command]
pub async fn execute_sql(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    query: String,
    allow_write: bool,
) -> Result<SqlQueryResult, String> {
    execute_sql_query(pool.inner(), &query, allow_write)
        .await
        .into_command_result()
}

#[tauri::command]
pub fn is_write_query_command(query: String) -> bool {
    is_write_query(&query)
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
            values.push(row_value_to_json(&row, idx));
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
    use super::{execute_sql_query, fetch_db_schema, fetch_table_rows, is_write_query};
    use crate::models::UserProfile;
    use crate::persistence::{UserRepo, migrator};

    #[test]
    fn detects_write_queries() {
        let writes = [
            "INSERT INTO users VALUES ('x')",
            "UPDATE users SET name = 'x'",
            "DELETE FROM users",
            "REPLACE INTO users VALUES ('x')",
            "DROP TABLE users",
            "CREATE TABLE foo (id INTEGER)",
            "ALTER TABLE users ADD COLUMN x TEXT",
            "TRUNCATE TABLE users",
            // Reported bypasses from PR review.
            "WITH cte AS (DELETE FROM im_accounts) SELECT 1",
            "ATTACH DATABASE ':memory:' AS malicious",
            "PRAGMA writable_schema = ON",
            "REINDEX im_accounts",
            "VACUUM",
            "ANALYZE",
        ];
        for q in writes {
            assert!(is_write_query(q), "expected write: {q}");
        }
    }

    #[test]
    fn read_queries_are_not_write() {
        let reads = [
            "SELECT * FROM users",
            "PRAGMA table_info(users)",
            "PRAGMA index_list(users)",
            "PRAGMA database_list",
            "EXPLAIN SELECT * FROM users",
            "EXPLAIN QUERY PLAN SELECT * FROM users",
            "SELECT 'INSERT' FROM users",
        ];
        for q in reads {
            assert!(!is_write_query(q), "expected read: {q}");
        }
    }

    #[test]
    fn pragma_assignments_are_detected_as_write() {
        let writes = [
            "PRAGMA writable_schema = ON",
            "PRAGMA journal_mode=WAL",
            "PRAGMA foreign_keys = ON",
        ];
        for q in writes {
            assert!(is_write_query(q), "expected write: {q}");
        }
    }

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
    async fn execute_sql_rejects_write_without_allow_write(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = execute_sql_query(
            &pool,
            "INSERT INTO im_accounts (user_id, nickname) VALUES ('99999', 'Eve')",
            false,
        )
        .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("write queries require allow_write=true")
        );
    }

    #[sqlx::test]
    async fn execute_sql_allows_write_with_allow_write(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = execute_sql_query(
            &pool,
            "INSERT INTO im_accounts (user_id, nickname) VALUES ('99999', 'Eve')",
            true,
        )
        .await;

        assert!(result.is_ok());

        let select = execute_sql_query(
            &pool,
            "SELECT user_id, nickname FROM im_accounts WHERE user_id = '99999'",
            false,
        )
        .await;

        assert!(select.is_ok());
        let select = select.unwrap();
        assert_eq!(select.rows.len(), 1);
        assert_eq!(select.columns, vec!["user_id", "nickname"]);
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

    #[sqlx::test]
    async fn execute_sql_rejects_empty_query(pool: sqlx::SqlitePool) {
        let result = execute_sql_query(&pool, "   ", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("query is empty"));
    }

    #[sqlx::test]
    async fn execute_sql_rejects_multiple_statements(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = execute_sql_query(&pool, "SELECT 1; DROP TABLE im_accounts;", false).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("multiple statements are not allowed")
        );
    }

    #[sqlx::test]
    async fn execute_sql_allows_semicolon_in_comment(pool: sqlx::SqlitePool) {
        let result = execute_sql_query(&pool, "SELECT 1 /* ; trailing comment */", false).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows.len(), 1);

        let result = execute_sql_query(&pool, "SELECT 1 -- ; line comment\n", false).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows.len(), 1);
    }

    #[sqlx::test]
    async fn execute_sql_allows_semicolon_in_string_literal(pool: sqlx::SqlitePool) {
        let result = execute_sql_query(&pool, "SELECT ';' as semi", false).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], serde_json::Value::String(";".into()));
    }

    #[test]
    fn write_detection_ignores_leading_comments() {
        assert!(!is_write_query("-- DELETE FROM im_accounts\nSELECT 1"));
        assert!(!is_write_query("/* DELETE */ SELECT 1"));
        assert!(is_write_query("/* prefix */ DELETE FROM im_accounts"));
        assert!(is_write_query(
            "-- comment\nUPDATE im_accounts SET nickname = 'x'"
        ));
    }

    #[sqlx::test]
    async fn execute_sql_returns_rows_affected_for_write(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        let result = execute_sql_query(
            &pool,
            "INSERT INTO im_accounts (user_id, nickname) VALUES ('77777', 'Bob')",
            true,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected, Some(1));
    }

    #[sqlx::test]
    async fn execute_sql_reads_return_rows_and_types(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        execute_sql_query(
            &pool,
            "CREATE TABLE test_types (id INTEGER, ratio REAL, name TEXT, data BLOB)",
            true,
        )
        .await
        .expect("create temp table should succeed");

        execute_sql_query(
            &pool,
            "INSERT INTO test_types VALUES (42, 2.5, 'hello', X'010203')",
            true,
        )
        .await
        .expect("insert temp row should succeed");

        execute_sql_query(
            &pool,
            "INSERT INTO test_types (id, name) VALUES (2, NULL)",
            true,
        )
        .await
        .expect("insert null row should succeed");

        let result = execute_sql_query(
            &pool,
            "SELECT id, ratio, name, data FROM test_types ORDER BY id",
            false,
        )
        .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.columns, vec!["id", "ratio", "name", "data"]);
        assert_eq!(result.rows.len(), 2);

        assert_eq!(result.rows[0][0], serde_json::Value::Number(2.into()));
        assert_eq!(result.rows[0][1], serde_json::Value::Null);
        assert_eq!(result.rows[0][2], serde_json::Value::Null);
        assert_eq!(result.rows[0][3], serde_json::Value::Null);

        assert_eq!(result.rows[1][0], serde_json::Value::Number(42.into()));
        assert_eq!(
            result.rows[1][1],
            serde_json::Value::Number(serde_json::Number::from_f64(2.5).unwrap())
        );
        assert_eq!(result.rows[1][2], serde_json::Value::String("hello".into()));
        assert_eq!(
            result.rows[1][3],
            serde_json::Value::String("<BLOB: 3 bytes>".into())
        );
    }

    #[sqlx::test]
    async fn execute_sql_select_truncates_at_1000(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        execute_sql_query(
            &pool,
            "CREATE TABLE truncate_test (id INTEGER PRIMARY KEY)",
            true,
        )
        .await
        .expect("create temp table should succeed");

        // Insert 1005 rows in a single statement.
        let values = (1..=1005)
            .map(|i| format!("({i})"))
            .collect::<Vec<_>>()
            .join(",");
        execute_sql_query(
            &pool,
            &format!("INSERT INTO truncate_test (id) VALUES {values}"),
            true,
        )
        .await
        .expect("insert rows should succeed");

        let result = execute_sql_query(&pool, "SELECT id FROM truncate_test", false)
            .await
            .expect("select should succeed");

        assert_eq!(result.rows.len(), 1000);
        assert!(result.truncated);
    }

    #[sqlx::test]
    async fn execute_sql_select_not_truncated_below_cap(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");

        execute_sql_query(
            &pool,
            "CREATE TABLE small_test (id INTEGER PRIMARY KEY)",
            true,
        )
        .await
        .expect("create temp table should succeed");

        execute_sql_query(
            &pool,
            "INSERT INTO small_test (id) VALUES (1), (2), (3)",
            true,
        )
        .await
        .expect("insert rows should succeed");

        let result = execute_sql_query(&pool, "SELECT id FROM small_test", false)
            .await
            .expect("select should succeed");

        assert_eq!(result.rows.len(), 3);
        assert!(!result.truncated);
    }
}
