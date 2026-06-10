use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct SettingRecord {
    pub setting_key: String,
    pub setting_value: String,
    pub value_type: String,
    pub description: Option<String>,
    pub updated_at: i64,
}

#[derive(Clone)]
pub struct SettingsRepo {
    pool: SqlitePool,
}

impl SettingsRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_string(&self, key: &str, default: &str) -> String {
        match self.get_raw(key).await {
            Ok(Some(record)) => record.setting_value,
            _ => default.to_string(),
        }
    }

    pub async fn get_i64(&self, key: &str, default: i64) -> i64 {
        match self.get_raw(key).await {
            Ok(Some(record)) => record.setting_value.parse().unwrap_or(default),
            _ => default,
        }
    }

    pub async fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.get_raw(key).await {
            Ok(Some(record)) => matches!(record.setting_value.as_str(), "true" | "1" | "yes"),
            _ => default,
        }
    }

    pub async fn get_raw(&self, key: &str) -> Result<Option<SettingRecord>, sqlx::Error> {
        sqlx::query_as::<_, SettingRecord>("SELECT * FROM app_settings WHERE setting_key = ?1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn set_string(
        &self,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO app_settings (setting_key, setting_value, value_type, description, updated_at)
            VALUES (?1, ?2, 'string', ?3, unixepoch() * 1000)
            ON CONFLICT(setting_key) DO UPDATE SET
                setting_value = excluded.setting_value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_i64(
        &self,
        key: &str,
        value: i64,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO app_settings (setting_key, setting_value, value_type, description, updated_at)
            VALUES (?1, ?2, 'int', ?3, unixepoch() * 1000)
            ON CONFLICT(setting_key) DO UPDATE SET
                setting_value = excluded.setting_value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value.to_string())
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_bool(
        &self,
        key: &str,
        value: bool,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO app_settings (setting_key, setting_value, value_type, description, updated_at)
            VALUES (?1, ?2, 'bool', ?3, unixepoch() * 1000)
            ON CONFLICT(setting_key) DO UPDATE SET
                setting_value = excluded.setting_value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(if value { "true" } else { "false" })
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM app_settings WHERE setting_key = ?1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::migrator;

    #[sqlx::test]
    async fn settings_crud(pool: sqlx::SqlitePool) -> sqlx::Result<()> {
        migrator::run_migrations(&pool)
            .await
            .map_err(sqlx::Error::Protocol)?;

        let repo = SettingsRepo::new(pool);

        // get non-existent returns default
        assert_eq!(repo.get_string("log.level", "info").await, "info");
        assert_eq!(repo.get_i64("log.retention_days", 7).await, 7);
        assert!(!repo.get_bool("log.debug_enabled", false).await);

        // set and get string
        repo.set_string("log.level", "debug", Some("log level"))
            .await?;
        assert_eq!(repo.get_string("log.level", "info").await, "debug");

        // get raw returns full record
        let raw = repo.get_raw("log.level").await?.unwrap();
        assert_eq!(raw.value_type, "string");
        assert_eq!(raw.description, Some("log level".to_string()));

        // set and get i64
        repo.set_i64("log.retention_days", 14, Some("retention"))
            .await?;
        assert_eq!(repo.get_i64("log.retention_days", 7).await, 14);

        // set and get bool
        repo.set_bool("log.debug_enabled", true, None).await?;
        assert!(repo.get_bool("log.debug_enabled", false).await);

        repo.set_bool("log.debug_enabled", false, None).await?;
        assert!(!repo.get_bool("log.debug_enabled", true).await);

        // delete
        assert!(repo.delete("log.level").await?);
        assert_eq!(repo.get_string("log.level", "info").await, "info");

        Ok(())
    }
}
