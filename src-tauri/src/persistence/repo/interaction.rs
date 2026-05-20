use sqlx::SqlitePool;

use crate::models::{MessageReactionEntity, PokeEntity};

#[derive(sqlx::FromRow)]
struct MessageReactionRow {
    id: String,
    message_id: String,
    source_type: String,
    source_id: String,
    operator_user_id: String,
    face_id: String,
    is_add: bool,
    created_at: u64,
}

#[derive(sqlx::FromRow)]
struct MessageReactionIdRow {
    id: String,
}

#[derive(sqlx::FromRow)]
struct PokeRow {
    id: String,
    source_type: String,
    source_id: String,
    sender_user_id: String,
    target_user_id: String,
    created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewMessageReactionRecord {
    pub message_id: String,
    pub operator_user_id: String,
    pub face_id: String,
    pub is_add: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewPokeRecord {
    pub source_type: String,
    pub source_id: String,
    pub sender_user_id: String,
    pub target_user_id: String,
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

    pub async fn insert_message_reaction(
        &self,
        record: NewMessageReactionRecord,
    ) -> Result<MessageReactionEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, MessageReactionIdRow>(
            r#"
            WITH next_id(value) AS (
                SELECT CAST(COALESCE(MAX(CAST(reaction_id AS INTEGER)), 0) + 1 AS TEXT)
                FROM message_reactions
            )
            INSERT INTO message_reactions (
                reaction_id, message_id, operator_user_id, face_id, is_add, created_at
            ) SELECT value, ?1, ?2, ?3, ?4, ?5
            FROM next_id
            RETURNING reaction_id AS id
            "#,
        )
        .bind(&record.message_id)
        .bind(&record.operator_user_id)
        .bind(&record.face_id)
        .bind(record.is_add)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        let full_row = sqlx::query_as::<_, MessageReactionRow>(
            r#"
            SELECT
                r.reaction_id AS id,
                r.message_id,
                m.message_scene AS source_type,
                CASE
                    WHEN m.message_scene IN ('private', 'temp') AND m.sender_user_id = r.operator_user_id THEN m.receiver_user_id
                    WHEN m.message_scene IN ('private', 'temp') THEN m.sender_user_id
                    ELSE m.group_id
                END AS source_id,
                r.operator_user_id,
                r.face_id,
                r.is_add,
                r.created_at
            FROM message_reactions r
            INNER JOIN messages m ON m.message_id = r.message_id
            WHERE r.reaction_id = ?1
            "#,
        )
        .bind(&row.id)
        .fetch_one(&self.pool)
        .await?;

        full_row.try_into()
    }

    pub async fn insert_poke(&self, record: NewPokeRecord) -> Result<PokeEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, PokeRow>(
            r#"
            WITH next_id(value) AS (
                SELECT CAST(COALESCE(MAX(CAST(poke_id AS INTEGER)), 0) + 1 AS TEXT)
                FROM pokes
            )
            INSERT INTO pokes (
                poke_id, message_scene, peer_id, sender_user_id, target_user_id, created_at
            ) SELECT value, ?1, ?2, ?3, ?4, ?5
            FROM next_id
            RETURNING poke_id AS id,
                      message_scene AS source_type,
                      peer_id AS source_id,
                      sender_user_id,
                      target_user_id,
                      created_at
            "#,
        )
        .bind(&record.source_type)
        .bind(&record.source_id)
        .bind(&record.sender_user_id)
        .bind(&record.target_user_id)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn list_pokes(
        &self,
        user_id: &str,
        source_type: &str,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<PokeEntity>, sqlx::Error> {
        let rows = if source_type == "private" {
            sqlx::query_as::<_, PokeRow>(
                r#"
                SELECT poke_id AS id,
                       message_scene AS source_type,
                       CASE
                           WHEN sender_user_id = ?1 THEN target_user_id
                           ELSE sender_user_id
                       END AS source_id,
                       sender_user_id,
                       target_user_id,
                       created_at
                FROM pokes
                WHERE message_scene = 'private'
                  AND (
                    (sender_user_id = ?1 AND target_user_id = ?2)
                    OR (sender_user_id = ?2 AND target_user_id = ?1)
                  )
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(user_id)
            .bind(source_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PokeRow>(
                r#"
                SELECT poke_id AS id,
                       message_scene AS source_type,
                       peer_id AS source_id,
                       sender_user_id,
                       target_user_id,
                       created_at
                FROM pokes
                WHERE message_scene = ?1 AND peer_id = ?2
                ORDER BY created_at DESC
                LIMIT ?3
                "#,
            )
            .bind(source_type)
            .bind(source_id)
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
        use crate::models::MessageSource;

        let source = MessageSource::try_from((row.source_type.as_str(), row.source_id))
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
        use crate::models::MessageSource;

        let source = MessageSource::try_from((row.source_type.as_str(), row.source_id))
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
