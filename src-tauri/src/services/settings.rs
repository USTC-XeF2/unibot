use crate::error::{AppError, AppResult};
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
        const MAX_DAYS: i64 = 36_500; // ~100 years

        if days < 0 {
            return Err(AppError::validation("retention_days cannot be negative"));
        }
        if days > MAX_DAYS {
            return Err(AppError::validation(format!(
                "retention_days cannot exceed {MAX_DAYS}"
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::SettingsService;
    use crate::persistence::{SettingsRepo, migrator};

    #[sqlx::test]
    async fn set_log_retention_days_rejects_negative(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");
        let service = SettingsService::new(SettingsRepo::new(pool));
        let result = service.set_log_retention_days(-1).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("retention_days cannot be negative")
        );
    }

    #[sqlx::test]
    async fn set_log_retention_days_rejects_extreme_values(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");
        let service = SettingsService::new(SettingsRepo::new(pool));
        let result = service.set_log_retention_days(100_000).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("retention_days cannot exceed")
        );
    }

    #[sqlx::test]
    async fn set_log_retention_days_accepts_zero_and_typical_values(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool)
            .await
            .expect("migrations should succeed");
        let service = SettingsService::new(SettingsRepo::new(pool));
        assert!(service.set_log_retention_days(0).await.is_ok());
        assert!(service.set_log_retention_days(7).await.is_ok());
        assert!(service.set_log_retention_days(36500).await.is_ok());
    }
}
