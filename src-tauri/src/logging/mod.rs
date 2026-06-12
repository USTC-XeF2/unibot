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

    tracing_subscriber::registry()
        .with(filter)
        .with(JsonLayer::new(non_blocking))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
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
/// Returns entries sorted by timestamp descending (newest first).
pub async fn read_system_logs(
    log_dir: impl AsRef<Path>,
    since: Option<u64>,
    before: Option<u64>,
    limit: usize,
) -> AppResult<Vec<serde_json::Value>> {
    let log_dir = log_dir.as_ref();
    if !log_dir.exists() {
        return Ok(vec![]);
    }

    let since_i64 = since.map(|ts| i64::try_from(ts).unwrap_or(i64::MAX));
    let before_i64 = before.map(|ts| i64::try_from(ts).unwrap_or(i64::MAX));

    let min_file_date = since_i64
        .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.date_naive()));
    let max_file_date = before_i64
        .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.date_naive()));

    let mut entries = Vec::new();
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

        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| AppError::storage(format!("failed to open log file: {e}")))?;
        let reader = tokio::io::BufReader::new(file);
        let mut lines = tokio::io::AsyncBufReadExt::lines(reader);

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
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

                    if let Some(since_ts) = since_i64 {
                        if entry_ts < since_ts {
                            continue;
                        }
                    }

                    if let Some(before_ts) = before_i64 {
                        if entry_ts >= before_ts {
                            continue;
                        }
                    }

                    entries.push(value);
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(
                        target: "log_reader",
                        path = %path.display(),
                        error = %e,
                        "failed to read log line; stopping file"
                    );
                    break;
                }
            }
        }
    }

    entries.sort_by(|a, b| {
        let a_ts = a.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
        let b_ts = b.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
        b_ts.cmp(&a_ts)
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
