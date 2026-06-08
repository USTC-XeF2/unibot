use sqlx::SqlitePool;

#[derive(Clone)]
pub struct BotRepo {
    pool: SqlitePool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BotRow {
    pub bot_id: String,
    pub bound_user_id: String,
    pub display_name: String,
    pub runtime_status: String,
    pub config_path: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DebugSessionRow {
    pub session_id: String,
    pub bot_id: String,
    pub session_name: String,
    pub description: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

impl BotRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_bot(
        &self,
        bot_id: &str,
        bound_user_id: &str,
        display_name: &str,
        config_path: &str,
    ) -> Result<BotRow, sqlx::Error> {
        sqlx::query_as::<_, BotRow>(
            r#"
            INSERT INTO bots (
                bot_id, bound_user_id, display_name, runtime_status, config_path, created_at, updated_at
            )
            SELECT ?1, ?2, ?3, 'stopped', ?4, unixepoch() * 1000, unixepoch() * 1000
            WHERE NOT EXISTS (
                SELECT 1 FROM bots WHERE bound_user_id = ?2
            )
            RETURNING bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
            "#,
        )
        .bind(bot_id)
        .bind(bound_user_id)
        .bind(display_name)
        .bind(config_path)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_bots(&self) -> Result<Vec<BotRow>, sqlx::Error> {
        sqlx::query_as::<_, BotRow>(
            r#"
            SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
            FROM bots
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_bot_by_id(&self, bot_id: &str) -> Result<Option<BotRow>, sqlx::Error> {
        sqlx::query_as::<_, BotRow>(
            r#"
            SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
            FROM bots
            WHERE bot_id = ?1
            "#,
        )
        .bind(bot_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_bot_by_bound_user_id(
        &self,
        user_id: &str,
    ) -> Result<Option<BotRow>, sqlx::Error> {
        sqlx::query_as::<_, BotRow>(
            r#"
            SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
            FROM bots
            WHERE bound_user_id = ?1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_bot_with_sessions(
        &self,
        bot_id: &str,
    ) -> Result<Option<BotRow>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let bot = sqlx::query_as::<_, BotRow>(
            r#"
            SELECT bot_id, bound_user_id, display_name, runtime_status, config_path, created_at
            FROM bots
            WHERE bot_id = ?1
            "#,
        )
        .bind(bot_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(bot) = bot else {
            drop(tx);
            return Ok(None);
        };

        sqlx::query(
            "UPDATE debug_sessions SET ended_at = unixepoch() * 1000 WHERE bot_id = ?1 AND ended_at IS NULL",
        )
        .bind(bot_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM bots WHERE bot_id = ?1")
            .bind(bot_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(Some(bot))
    }

    pub async fn start_session(
        &self,
        session_id: &str,
        bot_id: &str,
        session_name: &str,
    ) -> Result<DebugSessionRow, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let update = sqlx::query(
            r#"
            UPDATE bots
            SET runtime_status = 'running',
                updated_at = unixepoch() * 1000
            WHERE bot_id = ?1
              AND runtime_status != 'running'
              AND NOT EXISTS (
                  SELECT 1 FROM debug_sessions
                  WHERE bot_id = ?1 AND ended_at IS NULL
              )
            "#,
        )
        .bind(bot_id)
        .execute(&mut *tx)
        .await?;

        if update.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        let row = sqlx::query_as::<_, DebugSessionRow>(
            r#"
            INSERT INTO debug_sessions (session_id, bot_id, session_name, started_at)
            VALUES (?1, ?2, ?3, unixepoch() * 1000)
            RETURNING session_id, bot_id, session_name, description, started_at, ended_at
            "#,
        )
        .bind(session_id)
        .bind(bot_id)
        .bind(session_name)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    pub async fn stop_active_sessions(&self, bot_id: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE debug_sessions SET ended_at = unixepoch() * 1000 WHERE bot_id = ?1 AND ended_at IS NULL",
        )
        .bind(bot_id)
        .execute(&mut *tx)
        .await?;

        let update = sqlx::query(
            "UPDATE bots SET runtime_status = 'stopped', updated_at = unixepoch() * 1000 WHERE bot_id = ?1",
        )
        .bind(bot_id)
        .execute(&mut *tx)
        .await?;

        if update.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn has_active_session(&self, bot_id: &str) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM debug_sessions WHERE bot_id = ?1 AND ended_at IS NULL",
        )
        .bind(bot_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn list_sessions_by_bot(
        &self,
        bot_id: &str,
    ) -> Result<Vec<DebugSessionRow>, sqlx::Error> {
        sqlx::query_as::<_, DebugSessionRow>(
            r#"
            SELECT session_id, bot_id, session_name, description, started_at, ended_at
            FROM debug_sessions
            WHERE bot_id = ?1
            ORDER BY started_at DESC
            "#,
        )
        .bind(bot_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_online_bot_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT bot_id) FROM debug_sessions WHERE ended_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await
    }
}

impl TryFrom<BotRow> for crate::models::BotProfile {
    type Error = crate::error::AppError;

    fn try_from(row: BotRow) -> Result<Self, Self::Error> {
        let runtime_status = row
            .runtime_status
            .as_str()
            .try_into()
            .map_err(crate::error::AppError::internal)?;

        Ok(Self {
            bot_id: row.bot_id,
            bound_user_id: row.bound_user_id,
            display_name: row.display_name,
            runtime_status,
            config_path: row.config_path,
            created_at: row.created_at as u64,
        })
    }
}

impl TryFrom<DebugSessionRow> for crate::models::DebugSession {
    type Error = crate::error::AppError;

    fn try_from(row: DebugSessionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            session_id: row.session_id,
            bot_id: row.bot_id,
            session_name: row.session_name,
            description: row.description,
            started_at: row.started_at as u64,
            ended_at: row.ended_at.map(|value| value as u64),
        })
    }
}
