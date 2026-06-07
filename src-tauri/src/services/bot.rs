use crate::error::{AppError, AppResult};
use crate::models::{BotProfile, DebugSession};
use crate::persistence::BotRepo;
use crate::utils::{new_db_id, now_ts};
use serde::Serialize;
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
        std::fs::create_dir_all(&bots_dir)
            .map_err(|err| AppError::internal(format!("create bots dir: {err}")))?;
        let config_path = bots_dir.join(format!("{bot_id}.json"));
        std::fs::write(&config_path, "{}")
            .map_err(|err| AppError::internal(format!("write config: {err}")))?;
        let config_path = config_path
            .to_str()
            .ok_or_else(|| AppError::internal("bot config path is not valid UTF-8"))?;

        self.repo
            .insert_bot(&bot_id, &bound_user_id, &display_name, config_path)
            .await?
            .try_into()
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
            .get_bot_by_id(&bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        if bot.runtime_status == "running" {
            self.repo.end_active_sessions(&bot_id).await?;
            self.repo.update_runtime_status(&bot_id, "stopped").await?;
        }

        self.repo.delete_bot(&bot_id).await?;
        Ok(())
    }

    pub async fn start_bot(&self, bot_id: String) -> AppResult<DebugSession> {
        let bot = self
            .repo
            .get_bot_by_id(&bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        if bot.runtime_status == "running" || self.repo.has_active_session(&bot_id).await? {
            return Err(AppError::conflict("bot is already running"));
        }

        let session_id = new_db_id();
        let session_name = format!("调试会话 {}", now_ts());
        let row = self
            .repo
            .start_session(&session_id, &bot_id, &session_name)
            .await?;
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
}
