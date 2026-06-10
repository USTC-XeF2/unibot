use crate::error::AppResult;
use crate::persistence::SettingsRepo;

#[derive(Clone)]
pub struct SettingsService {
    repo: SettingsRepo,
}

impl SettingsService {
    pub fn new(repo: SettingsRepo) -> Self {
        Self { repo }
    }

    pub async fn get_log_level(&self) -> String {
        self.repo.get_string("log.level", "info").await
    }

    pub async fn set_log_level(&self, level: &str) -> AppResult<()> {
        self.repo
            .set_string(
                "log.level",
                level,
                Some("system log level (trace/debug/info/warn/error)"),
            )
            .await?;
        Ok(())
    }

    pub async fn get_log_retention_days(&self) -> i64 {
        self.repo.get_i64("log.retention_days", 7).await
    }

    pub async fn set_log_retention_days(&self, days: i64) -> AppResult<()> {
        self.repo
            .set_i64(
                "log.retention_days",
                days,
                Some("system log retention days, 0 = unlimited"),
            )
            .await?;
        Ok(())
    }

    /// Get the timestamp of the last successful log cleanup (0 = never).
    pub async fn get_log_last_cleanup_at(&self) -> i64 {
        self.repo.get_i64("log.last_cleanup_at", 0).await
    }

    /// Record the timestamp of the last successful log cleanup.
    pub async fn set_log_last_cleanup_at(&self, ts: i64) -> AppResult<()> {
        self.repo
            .set_i64(
                "log.last_cleanup_at",
                ts,
                Some("timestamp of last log cleanup"),
            )
            .await?;
        Ok(())
    }
}
