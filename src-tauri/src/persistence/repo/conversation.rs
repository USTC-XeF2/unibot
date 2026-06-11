use sqlx::SqlitePool;

use crate::models::ConversationState;

#[derive(Clone)]
pub struct ConversationRepo {
    pool: SqlitePool,
}

impl ConversationRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_conversation_pinned(
        &self,
        owner_user_id: &str,
        scene: &str,
        peer_user_id: Option<&str>,
        group_id: Option<&str>,
        is_pinned: bool,
        updated_at: i64,
    ) -> Result<(), sqlx::Error> {
        self.upsert_conversation_flag(
            owner_user_id,
            scene,
            peer_user_id,
            group_id,
            "is_pinned",
            is_pinned,
            updated_at,
        )
        .await
    }

    pub async fn upsert_conversation_muted(
        &self,
        owner_user_id: &str,
        scene: &str,
        peer_user_id: Option<&str>,
        group_id: Option<&str>,
        is_muted: bool,
        updated_at: i64,
    ) -> Result<(), sqlx::Error> {
        self.upsert_conversation_flag(
            owner_user_id,
            scene,
            peer_user_id,
            group_id,
            "is_muted",
            is_muted,
            updated_at,
        )
        .await
    }

    async fn upsert_conversation_flag(
        &self,
        owner_user_id: &str,
        scene: &str,
        peer_user_id: Option<&str>,
        group_id: Option<&str>,
        flag_column: &str,
        flag_value: bool,
        updated_at: i64,
    ) -> Result<(), sqlx::Error> {
        let is_private = scene == "private" || scene == "temp";
        let flag_i: i64 = if flag_value { 1 } else { 0 };

        match (is_private, flag_column) {
            (true, "is_pinned") => {
                let peer = peer_user_id.unwrap_or("");
                let conversation_id = format!("{owner_user_id}:{scene}:{peer}");
                sqlx::query(
                    r#"
                    INSERT INTO conversations (
                        conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                        is_pinned, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)
                    ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
                        WHERE conversation_scene IN ('private', 'temp')
                    DO UPDATE SET
                        is_pinned = excluded.is_pinned,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&conversation_id)
                .bind(owner_user_id)
                .bind(scene)
                .bind(peer)
                .bind(flag_i)
                .bind(updated_at)
                .execute(&self.pool)
                .await?;
            }
            (true, "is_muted") => {
                let peer = peer_user_id.unwrap_or("");
                let conversation_id = format!("{owner_user_id}:{scene}:{peer}");
                sqlx::query(
                    r#"
                    INSERT INTO conversations (
                        conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                        is_muted, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)
                    ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
                        WHERE conversation_scene IN ('private', 'temp')
                    DO UPDATE SET
                        is_muted = excluded.is_muted,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&conversation_id)
                .bind(owner_user_id)
                .bind(scene)
                .bind(peer)
                .bind(flag_i)
                .bind(updated_at)
                .execute(&self.pool)
                .await?;
            }
            (false, "is_pinned") => {
                let group = group_id.unwrap_or("");
                let conversation_id = format!("{owner_user_id}:group:{group}");
                sqlx::query(
                    r#"
                    INSERT INTO conversations (
                        conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                        is_pinned, updated_at
                    ) VALUES (?1, ?2, 'group', NULL, ?3, ?4, ?5)
                    ON CONFLICT(owner_user_id, conversation_scene, group_id)
                        WHERE conversation_scene = 'group'
                    DO UPDATE SET
                        is_pinned = excluded.is_pinned,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&conversation_id)
                .bind(owner_user_id)
                .bind(group)
                .bind(flag_i)
                .bind(updated_at)
                .execute(&self.pool)
                .await?;
            }
            (false, "is_muted") => {
                let group = group_id.unwrap_or("");
                let conversation_id = format!("{owner_user_id}:group:{group}");
                sqlx::query(
                    r#"
                    INSERT INTO conversations (
                        conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                        is_muted, updated_at
                    ) VALUES (?1, ?2, 'group', NULL, ?3, ?4, ?5)
                    ON CONFLICT(owner_user_id, conversation_scene, group_id)
                        WHERE conversation_scene = 'group'
                    DO UPDATE SET
                        is_muted = excluded.is_muted,
                        updated_at = excluded.updated_at
                    "#,
                )
                .bind(&conversation_id)
                .bind(owner_user_id)
                .bind(group)
                .bind(flag_i)
                .bind(updated_at)
                .execute(&self.pool)
                .await?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    pub async fn list_conversation_states(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<ConversationState>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ConversationStateRow>(
            r#"
            SELECT
                conversation_scene,
                peer_user_id,
                group_id,
                is_pinned,
                is_muted
            FROM conversations
            WHERE owner_user_id = ?1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ConversationStateRow {
    conversation_scene: String,
    peer_user_id: Option<String>,
    group_id: Option<String>,
    is_pinned: i64,
    is_muted: i64,
}

impl From<ConversationStateRow> for ConversationState {
    fn from(row: ConversationStateRow) -> Self {
        Self {
            conversation_scene: row.conversation_scene,
            peer_user_id: row.peer_user_id,
            group_id: row.group_id,
            is_pinned: row.is_pinned != 0,
            is_muted: row.is_muted != 0,
        }
    }
}
