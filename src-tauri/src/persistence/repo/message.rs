use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub sender_user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub receiver_user_id: Option<String>,
    pub group_id: Option<String>,
    pub bot_id: Option<String>,
    pub content_json: String,
    pub quoted_message_id: Option<String>,
    pub is_recalled: bool,
    pub recalled_by_user_id: Option<String>,
    pub created_at: u64,
    pub message_seq: i64,
}

#[derive(Debug, Clone)]
pub struct NewMessageRecord {
    pub owner_user_id: String,
    pub sender_user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub content_json: String,
    pub quoted_message_id: Option<String>,
    pub created_at: u64,
    pub bot_id: Option<String>,
}

#[derive(Clone)]
pub struct MessageRepo {
    pool: SqlitePool,
}

impl MessageRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_message(
        &self,
        record: NewMessageRecord,
    ) -> Result<MessageRecord, sqlx::Error> {
        let is_private = record.source_type == "private" || record.source_type == "temp";
        let receiver_user_id: Option<&str> = if is_private {
            if record.sender_user_id == record.owner_user_id {
                Some(&record.source_id)
            } else {
                Some(&record.owner_user_id)
            }
        } else {
            None
        };
        let group_id: Option<&str> = if !is_private {
            Some(&record.source_id)
        } else {
            None
        };

        let mut tx = self.pool.begin().await?;

        let id = crate::utils::new_db_id();

        let seq: i64 = sqlx::query_scalar(
            "UPDATE message_seq_counter SET next_seq = next_seq + 1 WHERE id = 1 RETURNING next_seq - 1",
        )
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query_as::<_, MessageRecord>(
            r#"
            INSERT INTO messages (
                message_id, message_scene, peer_id, message_seq, sender_user_id,
                receiver_user_id, group_id, bot_id, content_json, quoted_message_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            RETURNING message_id AS id,
                      sender_user_id,
                      message_scene AS source_type,
                      peer_id AS source_id,
                      receiver_user_id,
                      group_id,
                      bot_id,
                      content_json,
                      quoted_message_id,
                      is_recalled,
                      recalled_by_user_id,
                      created_at,
                      message_seq
            "#,
        )
        .bind(&id)
        .bind(&record.source_type)
        .bind(&record.source_id)
        .bind(seq)
        .bind(&record.sender_user_id)
        .bind(receiver_user_id)
        .bind(group_id)
        .bind(record.bot_id.as_deref())
        .bind(&record.content_json)
        .bind(record.quoted_message_id.as_deref())
        .bind(record.created_at as i64)
        .fetch_one(&mut *tx)
        .await?;

        let conversation_id = if is_private {
            format!(
                "{}:{}:{}",
                record.owner_user_id, record.source_type, record.source_id
            )
        } else {
            format!("{}:group:{}", record.owner_user_id, record.source_id)
        };

        if is_private {
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    last_message_id, unread_count, updated_at
                ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, 0, ?6)
                ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
                WHERE conversation_scene IN ('private', 'temp')
                DO UPDATE SET
                    last_message_id = excluded.last_message_id,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(&record.owner_user_id)
            .bind(&record.source_type)
            .bind(&record.source_id)
            .bind(&row.id)
            .bind(record.created_at as i64)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    last_message_id, unread_count, updated_at
                ) VALUES (?1, ?2, 'group', NULL, ?3, ?4, 0, ?5)
                ON CONFLICT(owner_user_id, conversation_scene, group_id)
                WHERE conversation_scene = 'group'
                DO UPDATE SET
                    last_message_id = excluded.last_message_id,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(&record.owner_user_id)
            .bind(&record.source_id)
            .bind(&row.id)
            .bind(record.created_at as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(row)
    }

    pub async fn mark_message_recalled(
        &self,
        message_id: &str,
        recalled_by_user_id: &str,
    ) -> Result<Option<MessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            UPDATE messages
            SET is_recalled = 1,
                recalled_by_user_id = ?2,
                recalled_at = unixepoch() * 1000
            WHERE message_id = ?1 AND is_recalled = 0
            RETURNING message_id AS id,
                      sender_user_id,
                      message_scene AS source_type,
                      peer_id AS source_id,
                      receiver_user_id,
                      group_id,
                      bot_id,
                      content_json,
                      quoted_message_id,
                      is_recalled,
                      recalled_by_user_id,
                      created_at,
                      message_seq
            "#,
        )
        .bind(message_id)
        .bind(recalled_by_user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_message_by_id(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT message_id AS id,
                   sender_user_id,
                   message_scene AS source_type,
                   COALESCE(group_id, peer_id) AS source_id,
                   receiver_user_id,
                   group_id,
                   bot_id,
                   content_json,
                   quoted_message_id,
                   is_recalled,
                   recalled_by_user_id,
                   created_at,
                   message_seq
            FROM messages
            WHERE message_id = ?1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_messages(
        &self,
        user_id: &str,
        source_type: &str,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        if source_type == "private" {
            return sqlx::query_as::<_, MessageRecord>(
                r#"
                SELECT message_id AS id,
                       sender_user_id,
                       message_scene AS source_type,
                       CASE
                           WHEN sender_user_id = ?1 THEN receiver_user_id
                           ELSE sender_user_id
                       END AS source_id,
                       receiver_user_id,
                       group_id,
                       bot_id,
                       content_json,
                       quoted_message_id,
                       is_recalled,
                       recalled_by_user_id,
                       created_at,
                       message_seq
                FROM messages
                WHERE message_scene = 'private'
                  AND (
                    (sender_user_id = ?1 AND receiver_user_id = ?2)
                    OR (sender_user_id = ?2 AND receiver_user_id = ?1)
                  )
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(user_id)
            .bind(source_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await;
        }

        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT message_id AS id,
                   sender_user_id,
                   message_scene AS source_type,
                   group_id AS source_id,
                   receiver_user_id,
                   group_id,
                   bot_id,
                   content_json,
                   quoted_message_id,
                   is_recalled,
                   recalled_by_user_id,
                   created_at,
                   message_seq
            FROM messages
            WHERE message_scene = 'group' AND group_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(source_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_message_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn get_message_by_seq(&self, seq: i64) -> Result<Option<MessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT message_id AS id,
                   sender_user_id,
                   message_scene AS source_type,
                   COALESCE(group_id, peer_id) AS source_id,
                   receiver_user_id,
                   group_id,
                   bot_id,
                   content_json,
                   quoted_message_id,
                   is_recalled,
                   recalled_by_user_id,
                   created_at,
                   message_seq
            FROM messages
            WHERE message_seq = ?1
            "#,
        )
        .bind(seq)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_seq_by_message_id(
        &self,
        message_id: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar("SELECT message_seq FROM messages WHERE message_id = ?1")
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await
    }
}
