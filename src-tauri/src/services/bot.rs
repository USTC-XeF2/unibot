use crate::error::{AppError, AppResult};
use crate::models::{BotProfile, DebugSession};
use crate::persistence::BotRepo;
use crate::protocol::types::{BotConfig, HttpConfig};
use crate::utils::{new_db_id, now_ts};
use serde::Serialize;
use std::io::ErrorKind;
use tauri::Manager;

#[derive(Clone)]
pub struct BotService {
    repo: BotRepo,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatsResult {
    pub total_messages: i64,
    pub online_bots: i64,
}

impl BotService {
    pub fn new(repo: BotRepo) -> Self {
        Self { repo }
    }

    pub async fn create_bot(
        &self,
        app: &tauri::AppHandle,
        bound_user_id: String,
        display_name: String,
    ) -> AppResult<BotProfile> {
        if bound_user_id.trim().is_empty() {
            return Err(AppError::validation("bound user id cannot be empty"));
        }
        if display_name.trim().is_empty() {
            return Err(AppError::validation("display name cannot be empty"));
        }

        if self
            .repo
            .find_bot_by_bound_user_id(&bound_user_id)
            .await?
            .is_some()
        {
            return Err(AppError::conflict("user already has a bot"));
        }

        let bot_id = new_db_id();
        let bots_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| AppError::internal(format!("app dir error: {err}")))?
            .join("bots");
        let config_path = bots_dir.join(format!("{bot_id}.json"));
        let config_path_string = config_path
            .to_str()
            .ok_or_else(|| AppError::internal("bot config path is not valid UTF-8"))?
            .to_string();

        // 分配最小可用端口（从 3001 开始）
        let port = self.allocate_port().await?;
        let config = BotConfig {
            version: 1,
            protocol: "milky".to_string(),
            http: HttpConfig {
                host: "127.0.0.1".to_string(),
                port,
            },
            access_token: uuid::Uuid::new_v4().to_string(),
            event_transport: "sse".to_string(),
        };
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::internal(format!("serialize config: {e}")))?;

        tokio::fs::create_dir_all(&bots_dir)
            .await
            .map_err(|err| AppError::internal(format!("create bots dir: {err}")))?;
        tokio::fs::write(&config_path, config_json)
            .await
            .map_err(|err| AppError::internal(format!("write config: {err}")))?;

        let bot = match self
            .repo
            .insert_bot(&bot_id, &bound_user_id, &display_name, &config_path_string)
            .await
        {
            Ok(bot) => bot,
            Err(err) => {
                let _ = tokio::fs::remove_file(&config_path).await;
                return Err(err.into());
            }
        };

        bot.try_into()
    }

    pub async fn list_bots(&self) -> AppResult<Vec<BotProfile>> {
        self.repo
            .list_bots()
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub async fn delete_bot(&self, bot_id: String) -> AppResult<()> {
        let bot = self
            .repo
            .delete_bot_with_sessions(&bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        match tokio::fs::remove_file(&bot.config_path).await {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                eprintln!(
                    "failed to delete bot config file at {} after deleting bot {bot_id}: {err}",
                    bot.config_path
                );
            }
        }

        Ok(())
    }

    pub async fn start_bot(&self, bot_id: String) -> AppResult<DebugSession> {
        let session_id = new_db_id();
        let session_name = format!("调试会话 {}", now_ts());

        let row = match self
            .repo
            .start_session(&session_id, &bot_id, &session_name)
            .await
        {
            Ok(row) => row,
            Err(sqlx::Error::RowNotFound) => {
                // start_session returns RowNotFound when:
                // - bot does not exist, OR
                // - bot is already running (runtime_status == 'running' or active session exists)
                // Distinguish them to give accurate error message.
                match self.repo.get_bot_by_id(&bot_id).await {
                    Ok(None) => {
                        return Err(AppError::not_found(format!("bot {bot_id} not found")));
                    }
                    Ok(Some(_)) => {
                        return Err(AppError::conflict("bot is already running"));
                    }
                    Err(err) => return Err(err.into()),
                }
            }
            Err(err) => return Err(err.into()),
        };
        row.try_into()
    }

    pub async fn stop_bot(&self, bot_id: String) -> AppResult<()> {
        let bot = self
            .repo
            .get_bot_by_id(&bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        if bot.runtime_status != "running" && !self.repo.has_active_session(&bot_id).await? {
            return Err(AppError::validation("bot is not running"));
        }

        self.repo.stop_active_sessions(&bot_id).await?;
        Ok(())
    }

    pub async fn list_sessions(&self, bot_id: String) -> AppResult<Vec<DebugSession>> {
        self.repo
            .list_sessions_by_bot(&bot_id)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub async fn get_online_bot_count(&self) -> AppResult<i64> {
        self.repo.get_online_bot_count().await.map_err(Into::into)
    }

    async fn allocate_port(&self) -> AppResult<u16> {
        let bots = self.repo.list_bots().await?;
        let mut used = Vec::new();
        for bot in bots {
            if let Ok(text) = tokio::fs::read_to_string(&bot.config_path).await {
                if let Ok(cfg) = serde_json::from_str::<BotConfig>(&text) {
                    used.push(cfg.http.port);
                }
            }
        }
        for port in 3001..=65535 {
            if !used.contains(&port) {
                return Ok(port);
            }
        }
        Err(AppError::validation(
            "no available port in range 3001-65535",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserProfile;
    use crate::persistence::{UserRepo, migrator};

    #[sqlx::test]
    async fn delete_bot_removes_database_record_when_config_cleanup_fails(
        pool: sqlx::SqlitePool,
    ) -> Result<(), sqlx::Error> {
        migrator::run_migrations(&pool)
            .await
            .map_err(sqlx::Error::Protocol)?;

        UserRepo::new(pool.clone())
            .upsert_user(&UserProfile {
                user_id: "10001".to_string(),
                nickname: "Alice".to_string(),
                avatar: String::new(),
                signature: String::new(),
                account_status: Default::default(),
            })
            .await?;

        let config_path = std::env::temp_dir().join(format!("unibot-config-dir-{}", new_db_id()));
        tokio::fs::create_dir(&config_path)
            .await
            .expect("test config directory should be created");

        let repo = BotRepo::new(pool);
        repo.insert_bot(
            "bot_10001",
            "10001",
            "Alice Bot",
            config_path
                .to_str()
                .expect("temporary path should be valid UTF-8"),
        )
        .await?;

        let result = BotService::new(repo.clone())
            .delete_bot("bot_10001".to_string())
            .await;
        let bot = repo.get_bot_by_id("bot_10001").await?;

        tokio::fs::remove_dir(&config_path)
            .await
            .expect("test config directory should be removed");

        assert!(result.is_ok());
        assert!(bot.is_none());
        Ok(())
    }
}
