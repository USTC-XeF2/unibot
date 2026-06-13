pub mod json_layer;

use crate::error::{AppError, AppResult};
use crate::utils::now_ts;
use json_layer::JsonLayer;
use parking_lot::Mutex;
use std::path::Path;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Global reload handle for runtime log level switching.
static RELOAD_HANDLE: Mutex<Option<Handle<EnvFilter, Registry>>> = Mutex::new(None);

fn parse_level(level: &str) -> LevelFilter {
    match level.parse::<LevelFilter>() {
        Ok(filter) => filter,
        Err(_) => {
            // tracing is not guaranteed to be initialized here, so fall back to stderr.
            eprintln!("invalid log level '{level}', falling back to INFO");
            LevelFilter::INFO
        }
    }
}

/// Guard that keeps the non-blocking writer thread alive.
pub struct LogGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl LogGuard {
    pub fn new(guard: tracing_appender::non_blocking::WorkerGuard) -> Self {
        Self { _guard: guard }
    }
}

/// Initialize the tracing subscriber with JSON output and a reloadable env filter.
///
/// # Arguments
/// * `log_dir` - Directory where log files are written (daily rotation)
/// * `default_level` - Default log level (e.g. "info" or "debug")
pub fn init_logging(log_dir: impl AsRef<Path>, default_level: &str) -> AppResult<LogGuard> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(parse_level(default_level).into())
        .from_env_lossy();

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("unibot")
        .filename_suffix("log")
        .build(log_dir)
        .map_err(|e| AppError::internal(format!("failed to create log appender: {e}")))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    let (filter, reload_handle) = tracing_subscriber::reload::Layer::new(env_filter);

    // Only mirror logs to stderr in dev builds. Release builds rely solely on
    // the JSON log file appender to avoid unnecessary runtime overhead.
    let stderr_layer = if cfg!(debug_assertions) {
        Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(JsonLayer::new(non_blocking))
        .with(stderr_layer)
        .init();

    *RELOAD_HANDLE.lock() = Some(reload_handle);

    Ok(LogGuard::new(guard))
}

/// Change the log level at runtime.
///
/// Returns Ok(true) if the level was changed, Ok(false) if logging is not initialized.
pub fn set_log_level(level: &str) -> AppResult<bool> {
    let new_filter = EnvFilter::builder()
        .with_default_directive(parse_level(level).into())
        .from_env_lossy();

    let handle = RELOAD_HANDLE.lock();
    match handle.as_ref() {
        Some(h) => {
            h.reload(new_filter)
                .map_err(|e| AppError::internal(format!("failed to reload log filter: {e}")))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Read system log entries from JSON Lines files.
/// Returns entries sorted by timestamp descending (newest first), with a
/// monotonic `seq` tie-breaker so that cursor pagination is stable when
/// multiple entries share the same millisecond timestamp.
pub async fn read_system_logs(
    log_dir: impl AsRef<Path>,
    since: Option<u64>,
    before: Option<u64>,
    before_seq: Option<u64>,
    limit: usize,
    keyword: Option<&str>,
    levels: &[String],
) -> AppResult<Vec<serde_json::Value>> {
    let log_dir = log_dir.as_ref();
    if !log_dir.exists() {
        return Ok(vec![]);
    }

    let keyword_lower = keyword
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_lowercase);
    let level_set: std::collections::HashSet<String> =
        levels.iter().map(|l| l.to_lowercase()).collect();

    let since_i64 = since.map(|ts| i64::try_from(ts).unwrap_or(i64::MAX));
    let before_i64 = before.map(|ts| i64::try_from(ts).unwrap_or(i64::MAX));

    let min_file_date = since_i64
        .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.date_naive()));
    let max_file_date = before_i64
        .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.date_naive()));

    // Collect matching files and sort by filename ascending. Filenames are
    // unibot.YYYY-MM-DD.log, so lexical order is chronological order.
    let mut paths = Vec::new();
    let mut read_files = tokio::fs::read_dir(log_dir)
        .await
        .map_err(|e| AppError::storage(format!("failed to read log directory: {e}")))?;

    while let Some(entry) = read_files
        .next_entry()
        .await
        .map_err(|e| AppError::storage(format!("failed to read directory entry: {e}")))?
    {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".log") => n,
            _ => continue,
        };

        let file_date = name
            .strip_prefix("unibot.")
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        if let Some(min_date) = min_file_date {
            if let Some(date) = file_date {
                if date < min_date {
                    continue;
                }
            }
        }
        if let Some(max_date) = max_file_date {
            if let Some(date) = file_date {
                if date > max_date {
                    continue;
                }
            }
        }

        paths.push(path);
    }

    paths.sort();

    let mut entries = Vec::new();
    let mut base_seq: u64 = 0;

    for path in paths {
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| AppError::storage(format!("failed to read log file metadata: {e}")))?;
        let file_size = metadata.len();

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let file_date = name
            .strip_prefix("unibot.")
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| AppError::storage(format!("failed to open log file: {e}")))?;
        let mut reader = tokio::io::BufReader::new(file);
        let mut line = String::new();
        let mut line_offset: u64 = 0;

        loop {
            line.clear();
            let bytes_read = tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line)
                .await
                .map_err(|e| {
                    tracing::warn!(
                        target: "log_reader",
                        path = %path.display(),
                        error = %e,
                        "failed to read log line; stopping file"
                    );
                    AppError::storage(format!("failed to read log line: {e}"))
                })?;

            if bytes_read == 0 {
                break;
            }

            let line_start = base_seq + line_offset;
            line_offset += bytes_read as u64;

            if line.trim().is_empty() {
                continue;
            }

            let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };

            if value.get("ts").is_none() {
                if let Some(date) = file_date {
                    let ts = date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc()
                        .timestamp_millis();
                    value["ts"] = ts.into();
                }
            }

            let entry_ts = value.get("ts").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
            value["seq"] = line_start.into();

            if let Some(since_ts) = since_i64 {
                if entry_ts < since_ts {
                    continue;
                }
            }

            if let Some(before_ts) = before_i64 {
                if entry_ts > before_ts {
                    continue;
                }
                if entry_ts == before_ts {
                    if let Some(before_seq_val) = before_seq {
                        if line_start >= before_seq_val {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            }

            if !level_set.is_empty() {
                let level_matches = value
                    .get("level")
                    .and_then(|v| v.as_str())
                    .map(|l| level_set.contains(&l.to_lowercase()))
                    .unwrap_or(false);
                if !level_matches {
                    continue;
                }
            }

            if let Some(needle) = &keyword_lower {
                if !value.to_string().to_lowercase().contains(needle) {
                    continue;
                }
            }

            entries.push(value);
        }

        base_seq += file_size;
    }

    entries.sort_by(|a, b| {
        let a_ts = a.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
        let b_ts = b.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
        let a_seq = a.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
        let b_seq = b.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
        b_ts.cmp(&a_ts).then_with(|| b_seq.cmp(&a_seq))
    });

    entries.truncate(limit);
    Ok(entries)
}

/// Clean up log files older than retention_days.
pub async fn cleanup_old_logs(log_dir: impl AsRef<Path>, retention_days: i64) -> AppResult<usize> {
    if retention_days <= 0 {
        return Ok(0);
    }

    let log_dir = log_dir.as_ref();
    if !log_dir.exists() {
        return Ok(0);
    }

    let retention_ms = i128::from(retention_days) * 24 * 60 * 60 * 1000;
    let now = i128::from(now_ts());
    let cutoff = (now - retention_ms).max(0) as i64;
    let mut deleted = 0usize;
    let mut read_files = tokio::fs::read_dir(log_dir)
        .await
        .map_err(|e| AppError::storage(format!("failed to read log directory: {e}")))?;

    while let Some(entry) = read_files
        .next_entry()
        .await
        .map_err(|e| AppError::storage(format!("failed to read directory entry: {e}")))?
    {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".log") => n,
            _ => continue,
        };

        let file_date = name
            .strip_prefix("unibot.")
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let should_delete = match file_date {
            Some(date) => {
                let date_ts = date
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_default()
                    .and_utc()
                    .timestamp_millis();
                date_ts < cutoff
            }
            None => false,
        };

        if should_delete {
            if let Err(e) = tokio::fs::remove_file(&path).await {
                tracing::warn!(target: "log_cleanup", "failed to delete old log file {}: {}", path.display(), e);
            } else {
                deleted += 1;
            }
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::{AppResult, read_system_logs};
    use std::io::Write;
    use std::path::PathBuf;

    const EMPTY_LEVELS: &[String] = &[];

    fn tmp_log_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("unibot-log-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_log_file(dir: &std::path::Path, name: &str, lines: &[String]) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
    }

    fn run<F, T>(f: F) -> T
    where
        F: std::future::Future<Output = AppResult<T>>,
    {
        tokio::runtime::Runtime::new().unwrap().block_on(f).unwrap()
    }

    #[test]
    fn read_system_logs_assigns_seq() {
        let dir = tmp_log_dir();
        write_log_file(
            &dir,
            "unibot.2024-06-12.log",
            &[
                r#"{"level":"INFO","target":"t","msg":"a"}"#.to_string(),
                r#"{"level":"INFO","target":"t","msg":"b"}"#.to_string(),
                r#"{"level":"INFO","target":"t","msg":"c"}"#.to_string(),
            ],
        );

        let entries = run(read_system_logs(
            &dir,
            None,
            None,
            None,
            10,
            None,
            EMPTY_LEVELS,
        ));
        assert_eq!(entries.len(), 3);

        let seqs: Vec<u64> = entries
            .iter()
            .map(|e| e.get("seq").unwrap().as_u64().unwrap())
            .collect();
        // seq is now a stable byte offset, globally monotonic across files.
        // Newest-first order means seq values decrease through the result array.
        assert!(seqs.windows(2).all(|w| w[0] > w[1]));
    }

    #[test]
    fn read_system_logs_cursor_with_seq() {
        let dir = tmp_log_dir();
        let lines: Vec<String> = (0..5)
            .map(|i| format!(r#"{{"level":"INFO","target":"t","msg":"{i}"}}"#))
            .collect();
        write_log_file(&dir, "unibot.2024-06-12.log", &lines);

        let page1 = run(read_system_logs(
            &dir,
            None,
            None,
            None,
            2,
            None,
            EMPTY_LEVELS,
        ));
        assert_eq!(page1.len(), 2);

        let oldest = page1.last().unwrap();
        let before = oldest.get("ts").unwrap().as_u64().unwrap();
        let before_seq = oldest.get("seq").unwrap().as_u64().unwrap();

        let page2 = run(read_system_logs(
            &dir,
            None,
            Some(before),
            Some(before_seq),
            2,
            None,
            EMPTY_LEVELS,
        ));
        assert_eq!(page2.len(), 2);

        let page2_seqs: Vec<u64> = page2
            .iter()
            .map(|e| e.get("seq").unwrap().as_u64().unwrap())
            .collect();
        // Byte-offset seqs are stable; page2 entries are strictly older than the cursor.
        assert!(page2_seqs.iter().all(|s| *s < before_seq));
        assert!(page2_seqs.windows(2).all(|w| w[0] > w[1]));

        // Load the final page and confirm it contains the remaining entry.
        let oldest2 = page2.last().unwrap();
        let page3 = run(read_system_logs(
            &dir,
            None,
            Some(before),
            Some(oldest2.get("seq").unwrap().as_u64().unwrap()),
            2,
            None,
            EMPTY_LEVELS,
        ));
        assert_eq!(page3.len(), 1);
        assert!(page3[0].get("seq").unwrap().as_u64().unwrap() < page2_seqs[1]);
    }

    #[test]
    fn read_system_logs_sorts_by_ts_then_seq() {
        let dir = tmp_log_dir();
        write_log_file(
            &dir,
            "unibot.2024-06-12.log",
            &[r#"{"level":"INFO","target":"t","msg":"old"}"#.to_string()],
        );
        write_log_file(
            &dir,
            "unibot.2024-06-13.log",
            &[r#"{"level":"INFO","target":"t","msg":"new"}"#.to_string()],
        );

        let entries = run(read_system_logs(
            &dir,
            None,
            None,
            None,
            10,
            None,
            EMPTY_LEVELS,
        ));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].get("msg").unwrap().as_str().unwrap(), "new");
        assert_eq!(entries[1].get("msg").unwrap().as_str().unwrap(), "old");
    }
}
