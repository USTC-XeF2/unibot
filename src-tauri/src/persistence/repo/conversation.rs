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

    #[allow(clippy::too_many_arguments)]
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

        if is_private {
            let peer = peer_user_id.unwrap_or("");
            let conversation_id = format!("{owner_user_id}:{scene}:{peer}");
            let (pinned_i, muted_i) = if flag_column == "is_pinned" {
                (flag_i, 0)
            } else {
                (0, flag_i)
            };
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    is_pinned, is_muted, updated_at
                ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7)
                ON CONFLICT(owner_user_id, conversation_scene, peer_user_id)
                    WHERE conversation_scene IN ('private', 'temp')
                DO UPDATE SET
                    is_pinned = CASE WHEN ?8 = 'is_pinned' THEN excluded.is_pinned ELSE is_pinned END,
                    is_muted = CASE WHEN ?8 = 'is_muted' THEN excluded.is_muted ELSE is_muted END,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(owner_user_id)
            .bind(scene)
            .bind(peer)
            .bind(pinned_i)
            .bind(muted_i)
            .bind(updated_at)
            .bind(flag_column)
            .execute(&self.pool)
            .await?;
        } else {
            let group = group_id.unwrap_or("");
            let conversation_id = format!("{owner_user_id}:group:{group}");
            let (pinned_i, muted_i) = if flag_column == "is_pinned" {
                (flag_i, 0)
            } else {
                (0, flag_i)
            };
            sqlx::query(
                r#"
                INSERT INTO conversations (
                    conversation_id, owner_user_id, conversation_scene, peer_user_id, group_id,
                    is_pinned, is_muted, updated_at
                ) VALUES (?1, ?2, 'group', NULL, ?3, ?4, ?5, ?6)
                ON CONFLICT(owner_user_id, conversation_scene, group_id)
                    WHERE conversation_scene = 'group'
                DO UPDATE SET
                    is_pinned = CASE WHEN ?7 = 'is_pinned' THEN excluded.is_pinned ELSE is_pinned END,
                    is_muted = CASE WHEN ?7 = 'is_muted' THEN excluded.is_muted ELSE is_muted END,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&conversation_id)
            .bind(owner_user_id)
            .bind(group)
            .bind(pinned_i)
            .bind(muted_i)
            .bind(updated_at)
            .bind(flag_column)
            .execute(&self.pool)
            .await?;
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
