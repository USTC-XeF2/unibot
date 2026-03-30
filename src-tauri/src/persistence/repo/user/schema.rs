use sqlx::SqlitePool;

use super::UserRepo;

impl UserRepo {
    pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                user_id INTEGER PRIMARY KEY,
                nickname TEXT NOT NULL,
                avatar TEXT NOT NULL DEFAULT '',
                signature TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS friend_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                initiator_user_id INTEGER NOT NULL,
                target_user_id INTEGER NOT NULL,
                comment TEXT NOT NULL,
                state INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                handled_at INTEGER,
                operator_user_id INTEGER,
                FOREIGN KEY (initiator_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (target_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (operator_user_id) REFERENCES users(user_id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS friendships (
                user_low_id INTEGER NOT NULL,
                user_high_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (user_low_id, user_high_id),
                CHECK (user_low_id < user_high_id),
                FOREIGN KEY (user_low_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (user_high_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_friend_requests_operator
            ON friend_requests(operator_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_friend_requests_target_state_created
            ON friend_requests(target_user_id, state, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_friendships_high_created
            ON friendships(user_high_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_friend_requests_initiator_state_created
            ON friend_requests(initiator_user_id, state, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_friendships_low_created
            ON friendships(user_low_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
