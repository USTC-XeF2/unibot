use sqlx::SqlitePool;

use crate::models::{MessageReactionEntity, MessageSource, PokeEntity};

#[derive(sqlx::FromRow)]
struct MessageReactionRow {
    id: i64,
    message_id: i64,
    source_type: String,
    source_id: u64,
    operator_user_id: u64,
    face_id: String,
    is_add: bool,
    created_at: u64,
}

#[derive(sqlx::FromRow)]
struct PokeRow {
    id: i64,
    source_type: String,
    source_id: u64,
    sender_user_id: u64,
    target_user_id: u64,
    created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewMessageReactionRecord {
    pub message_id: i64,
    pub source_type: String,
    pub source_id: u64,
    pub operator_user_id: u64,
    pub face_id: String,
    pub is_add: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewPokeRecord {
    pub source_type: String,
    pub source_id: u64,
    pub sender_user_id: u64,
    pub target_user_id: u64,
    pub created_at: u64,
}

#[derive(Clone)]
pub struct InteractionRepo {
    pool: SqlitePool,
}

impl InteractionRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS message_reactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                source_type TEXT NOT NULL,
                source_id INTEGER NOT NULL,
                operator_user_id INTEGER NOT NULL,
                face_id TEXT NOT NULL,
                is_add INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (message_id) REFERENCES messages(id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (operator_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pokes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL,
                source_id INTEGER NOT NULL,
                sender_user_id INTEGER NOT NULL,
                target_user_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (sender_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (target_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_message_reactions_operator_created
            ON message_reactions(operator_user_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_message_reactions_message_created
            ON message_reactions(message_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pokes_sender_created
            ON pokes(sender_user_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pokes_target_created
            ON pokes(target_user_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pokes_source_created
            ON pokes(source_type, source_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn insert_message_reaction(
        &self,
        record: NewMessageReactionRecord,
    ) -> Result<MessageReactionEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, MessageReactionRow>(
            r#"
            INSERT INTO message_reactions (
                message_id, source_type, source_id, operator_user_id, face_id, is_add, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING id, message_id, source_type, source_id, operator_user_id, face_id, is_add, created_at
            "#,
        )
        .bind(record.message_id)
        .bind(record.source_type)
        .bind(record.source_id as i64)
        .bind(record.operator_user_id as i64)
        .bind(record.face_id)
        .bind(record.is_add)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn insert_poke(&self, record: NewPokeRecord) -> Result<PokeEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, PokeRow>(
            r#"
            INSERT INTO pokes (
                source_type, source_id, sender_user_id, target_user_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING id, source_type, source_id, sender_user_id, target_user_id, created_at
            "#,
        )
        .bind(record.source_type)
        .bind(record.source_id as i64)
        .bind(record.sender_user_id as i64)
        .bind(record.target_user_id as i64)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn list_pokes(
        &self,
        user_id: u64,
        source_type: &str,
        source_id: u64,
        limit: i64,
    ) -> Result<Vec<PokeEntity>, sqlx::Error> {
        let rows = if source_type == "private" {
            sqlx::query_as::<_, PokeRow>(
                r#"
                SELECT id, source_type, source_id, sender_user_id, target_user_id, created_at
                FROM pokes
                WHERE source_type = 'private'
                  AND (
                    (sender_user_id = ?1 AND target_user_id = ?2)
                    OR (sender_user_id = ?2 AND target_user_id = ?1)
                  )
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(user_id as i64)
            .bind(source_id as i64)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PokeRow>(
                r#"
                SELECT id, source_type, source_id, sender_user_id, target_user_id, created_at
                FROM pokes
                WHERE source_type = ?1 AND source_id = ?2
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(source_type)
            .bind(source_id as i64)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

impl TryFrom<MessageReactionRow> for MessageReactionEntity {
    type Error = sqlx::Error;

    fn try_from(row: MessageReactionRow) -> Result<Self, Self::Error> {
        let source: MessageSource = (row.source_type.as_str(), row.source_id)
            .try_into()
            .map_err(sqlx::Error::Protocol)?;

        Ok(Self {
            reaction_id: row.id,
            message_id: row.message_id,
            source,
            operator_user_id: row.operator_user_id,
            face_id: row.face_id,
            is_add: row.is_add,
            created_at: row.created_at,
        })
    }
}

impl TryFrom<PokeRow> for PokeEntity {
    type Error = sqlx::Error;

    fn try_from(row: PokeRow) -> Result<Self, Self::Error> {
        let source: MessageSource = (row.source_type.as_str(), row.source_id)
            .try_into()
            .map_err(sqlx::Error::Protocol)?;

        Ok(Self {
            poke_id: row.id,
            source,
            sender_user_id: row.sender_user_id,
            target_user_id: row.target_user_id,
            created_at: row.created_at,
        })
    }
}
