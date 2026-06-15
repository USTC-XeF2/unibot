use crate::models::GroupEssenceMessageEntity;

use super::super::GroupRepo;
use super::super::types::GroupEssenceRow;

impl GroupRepo {
    pub async fn create_group_essence_message(
        &self,
        group_id: &str,
        message_id: &str,
        sender_user_id: &str,
        operator_user_id: &str,
        is_set: bool,
        created_at: u64,
    ) -> Result<GroupEssenceMessageEntity, sqlx::Error> {
        if is_set {
            let id = crate::utils::new_db_id();
            let row = sqlx::query_as::<_, GroupEssenceRow>(
                r#"
                INSERT INTO group_essence_messages (
                    essence_id, group_id, message_id, sender_user_id, operator_user_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(group_id, message_id) DO UPDATE SET
                    sender_user_id = excluded.sender_user_id,
                    operator_user_id = excluded.operator_user_id,
                    created_at = excluded.created_at
                RETURNING
                    essence_id AS id,
                    group_id,
                    message_id,
                    sender_user_id,
                    operator_user_id,
                    1 AS is_set,
                    created_at,
                    (SELECT content_json FROM messages WHERE message_id = group_essence_messages.message_id) AS content_json
                "#,
            )
            .bind(&id)
            .bind(group_id)
            .bind(message_id)
            .bind(sender_user_id)
            .bind(operator_user_id)
            .bind(created_at as i64)
            .fetch_one(&self.pool)
            .await?;

            row.try_into()
        } else {
            sqlx::query(
                r#"
                DELETE FROM group_essence_messages
                WHERE group_id = ?1 AND message_id = ?2
                "#,
            )
            .bind(group_id)
            .bind(message_id)
            .execute(&self.pool)
            .await?;

            Ok(GroupEssenceMessageEntity {
                essence_id: String::new(),
                group_id: group_id.to_string(),
                message_id: message_id.to_string(),
                sender_user_id: sender_user_id.to_string(),
                operator_user_id: operator_user_id.to_string(),
                is_set: false,
                content: Vec::new(),
                created_at,
            })
        }
    }

    pub async fn list_group_essence_messages(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupEssenceMessageEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupEssenceRow>(
            r#"
            SELECT
                e.essence_id AS id,
                e.group_id,
                e.message_id,
                e.sender_user_id,
                e.operator_user_id,
                1 AS is_set,
                e.created_at,
                m.content_json
            FROM group_essence_messages e
            LEFT JOIN messages m ON m.message_id = e.message_id
            WHERE e.group_id = ?1
            ORDER BY e.created_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}
