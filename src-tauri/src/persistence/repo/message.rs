use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: i64,
    pub sender_user_id: u64,
    pub source_type: String,
    pub source_id: u64,
    pub content_json: String,
    pub quoted_message_id: Option<i64>,
    pub is_recalled: bool,
    pub recalled_by_user_id: Option<u64>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewMessageRecord {
    pub sender_user_id: u64,
    pub source_type: String,
    pub source_id: u64,
    pub content_json: String,
    pub quoted_message_id: Option<i64>,
    pub created_at: u64,
}

#[derive(Clone)]
pub struct MessageRepo {
    pool: SqlitePool,
}

impl MessageRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sender_user_id INTEGER NOT NULL,
                source_type TEXT NOT NULL,
                source_id INTEGER NOT NULL,
                content_json TEXT NOT NULL,
                quoted_message_id INTEGER,
                is_recalled INTEGER NOT NULL DEFAULT 0,
                recalled_by_user_id INTEGER,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (sender_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (quoted_message_id) REFERENCES messages(id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE,
                FOREIGN KEY (recalled_by_user_id) REFERENCES users(user_id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_sender_source_time
            ON messages(sender_user_id, source_type, source_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_source_time
            ON messages(source_type, source_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_recalled
            ON messages(is_recalled)
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn insert_message(
        &self,
        record: NewMessageRecord,
    ) -> Result<MessageRecord, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            INSERT INTO messages (sender_user_id, source_type, source_id, content_json, quoted_message_id, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
            "#,
        )
        .bind(record.sender_user_id as i64)
        .bind(record.source_type)
        .bind(record.source_id as i64)
        .bind(record.content_json)
        .bind(record.quoted_message_id)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn mark_message_recalled(
        &self,
        message_id: i64,
        recalled_by_user_id: u64,
    ) -> Result<Option<MessageRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, MessageRecord>(
            r#"
            UPDATE messages
            SET is_recalled = 1,
                recalled_by_user_id = ?2
            WHERE id = ?1 AND is_recalled = 0
            RETURNING id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
            "#,
        )
        .bind(message_id)
        .bind(recalled_by_user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_message_by_id(
        &self,
        message_id: i64,
    ) -> Result<Option<MessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
            FROM messages
            WHERE id = ?1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_messages(
        &self,
        user_id: u64,
        source_type: &str,
        source_id: u64,
        limit: i64,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        if source_type == "private" {
            return sqlx::query_as::<_, MessageRecord>(
                r#"
                SELECT id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
                FROM messages
                WHERE source_type = 'private'
                  AND (
                    (sender_user_id = ?1 AND source_id = ?2)
                    OR (sender_user_id = ?2 AND source_id = ?1)
                  )
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(user_id as i64)
            .bind(source_id as i64)
            .bind(limit)
            .fetch_all(&self.pool)
            .await;
        }

        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT id, sender_user_id, source_type, source_id, content_json, quoted_message_id, is_recalled, recalled_by_user_id, created_at
            FROM messages
            WHERE source_type = ?1 AND source_id = ?2
            ORDER BY created_at DESC
            LIMIT ?3
            "#,
        )
        .bind(source_type)
        .bind(source_id as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}
