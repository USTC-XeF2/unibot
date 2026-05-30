use sqlx::SqlitePool;

use super::migrations::{self, Migration};

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), String> {
    let current_version = read_schema_version(pool).await?;
    let pending: Vec<Migration> = migrations::all_migrations()
        .into_iter()
        .filter(|migration| migration.version > current_version.as_str())
        .collect();

    for migration in pending {
        apply_migration(pool, &migration).await?;
    }

    Ok(())
}

async fn read_schema_version(pool: &SqlitePool) -> Result<String, String> {
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_settings'",
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("failed to check app_settings existence: {err}"))?;

    if table_exists == 0 {
        return Ok("0000".to_string());
    }

    let version: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("failed to read schema version: {err}"))?;

    Ok(version.unwrap_or_else(|| "0000".to_string()))
}

async fn apply_migration(pool: &SqlitePool, migration: &Migration) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| format!("migration {}: failed to begin tx: {err}", migration.version))?;

    for (index, statement) in split_sql_statements(migration.sql).iter().enumerate() {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                format!(
                    "migration {} statement {} failed: {err}\n{statement}",
                    migration.version,
                    index + 1
                )
            })?;
    }

    sqlx::query(
        "INSERT INTO app_settings (setting_key, setting_value, value_type, description, updated_at)
         VALUES ('schema.version', ?1, 'string', '当前数据库 schema 迁移版本', unixepoch() * 1000)
         ON CONFLICT(setting_key) DO UPDATE SET
             setting_value = excluded.setting_value,
             description = excluded.description,
             updated_at = excluded.updated_at",
    )
    .bind(migration.version)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        format!(
            "migration {}: failed to update version: {err}",
            migration.version
        )
    })?;

    tx.commit()
        .await
        .map_err(|err| format!("migration {}: failed to commit: {err}", migration.version))?;

    Ok(())
}

#[derive(PartialEq)]
enum ScanState {
    Normal,
    SingleQuoted,
    DoubleQuoted,
    LineComment,
    BlockComment,
}

pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut state = ScanState::Normal;
    let mut in_trigger_body = false;
    let mut statement_start = 0;
    let mut iter = sql.char_indices().peekable();

    if is_trigger_start(&sql[statement_start..]) {
        in_trigger_body = true;
    }

    while let Some(&(byte_pos, ch)) = iter.peek() {
        let ch_len = ch.len_utf8();
        iter.next();

        let next_ch = iter.peek().map(|&(_, c)| c);

        match state {
            ScanState::Normal => {
                if ch == '\'' {
                    state = ScanState::SingleQuoted;
                } else if ch == '"' {
                    state = ScanState::DoubleQuoted;
                } else if ch == '-' && next_ch == Some('-') {
                    state = ScanState::LineComment;
                } else if ch == '/' && next_ch == Some('*') {
                    state = ScanState::BlockComment;
                } else if ch == ';' {
                    if in_trigger_body {
                        let stmt_text = &sql[statement_start..byte_pos];
                        if is_trigger_end(stmt_text) {
                            let stmt = sql[statement_start..byte_pos + ch_len].trim().to_string();
                            if !stmt.is_empty() {
                                statements.push(stmt);
                            }
                            statement_start = byte_pos + ch_len;
                            in_trigger_body = false;
                            if is_trigger_start(&sql[statement_start..]) {
                                in_trigger_body = true;
                            }
                        }
                    } else {
                        let stmt = sql[statement_start..byte_pos].trim().to_string();
                        if !stmt.is_empty() {
                            statements.push(stmt);
                        }
                        statement_start = byte_pos + ch_len;
                        if is_trigger_start(&sql[statement_start..]) {
                            in_trigger_body = true;
                        }
                    }
                }
            }
            ScanState::SingleQuoted => {
                if ch == '\'' {
                    if next_ch == Some('\'') {
                        iter.next();
                    } else {
                        state = ScanState::Normal;
                    }
                }
            }
            ScanState::DoubleQuoted => {
                if ch == '"' {
                    state = ScanState::Normal;
                }
            }
            ScanState::LineComment => {
                if ch == '\n' {
                    state = ScanState::Normal;
                }
            }
            ScanState::BlockComment => {
                if ch == '*' && next_ch == Some('/') {
                    iter.next();
                    state = ScanState::Normal;
                }
            }
        }
    }

    // Handle any remaining SQL
    let remaining = sql[statement_start..].trim().to_string();
    if !remaining.is_empty() {
        statements.push(remaining);
    }

    statements
}

fn skip_comments_and_whitespace(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        if s.starts_with("--") {
            if let Some(pos) = s.find('\n') {
                s = &s[pos + 1..];
                continue;
            } else {
                return "";
            }
        }
        if s.starts_with("/*") {
            if let Some(pos) = s.find("*/") {
                s = &s[pos + 2..];
                continue;
            } else {
                return "";
            }
        }
        break;
    }
    s
}

fn is_trigger_start(s: &str) -> bool {
    let trimmed = skip_comments_and_whitespace(s);
    let upper = trimmed.to_uppercase();
    upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE OR REPLACE TRIGGER")
}

fn is_trigger_end(s: &str) -> bool {
    let trimmed = s.trim_end();
    let bytes = trimmed.as_bytes();
    let len = bytes.len();
    if len < 3 {
        return false;
    }
    let last3 = &bytes[len - 3..];
    if !last3.eq_ignore_ascii_case(b"END") {
        return false;
    }
    if len > 3 {
        let before = bytes[len - 4];
        if !before.is_ascii_whitespace() && before != b';' {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_statements() {
        assert_eq!(
            split_sql_statements("SELECT 1; SELECT 2;"),
            vec!["SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn preserves_trigger_body_semicolons() {
        let sql = "CREATE TRIGGER foo AFTER INSERT ON t FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'END; still string'); -- END; comment\nUPDATE t SET a = 1; END;";
        let result = split_sql_statements(sql);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("RAISE(ABORT, 'END; still string');"));
        assert!(result[0].contains("UPDATE t SET a = 1;"));
    }

    #[test]
    fn ignores_semicolons_in_strings_and_comments() {
        let sql = "SELECT ';'; -- comment ;\nSELECT \"x;y\"; /* block ; */ SELECT 3;";
        let result = split_sql_statements(sql);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn parses_initial_schema_as_expected() {
        let statements =
            split_sql_statements(crate::persistence::migrations::all_migrations()[0].sql);
        assert!(
            statements
                .iter()
                .any(|stmt| stmt.contains("CREATE TABLE IF NOT EXISTS im_accounts"))
        );
        assert!(
            statements
                .iter()
                .any(|stmt| stmt.contains("CREATE TRIGGER IF NOT EXISTS trg_member_count_inc"))
        );
        assert!(
            statements
                .iter()
                .any(|stmt| stmt.contains("INSERT INTO app_settings"))
        );
    }

    #[sqlx::test]
    async fn applies_initial_schema(pool: SqlitePool) -> Result<(), sqlx::Error> {
        run_migrations(&pool).await.map_err(sqlx::Error::Protocol)?;

        let table_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(table_count, 26);

        let version: String = sqlx::query_scalar(
            "SELECT setting_value FROM app_settings WHERE setting_key = 'schema.version'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(version, "0001");

        run_migrations(&pool).await.map_err(sqlx::Error::Protocol)?;
        Ok(())
    }
}
