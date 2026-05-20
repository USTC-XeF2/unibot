use crate::models::{GroupRequestEntity, GroupRequestType, RequestState};
use crate::persistence::repo::codecs;

use super::types::GroupRequestRow;
use super::{GroupRepo, NewGroupRequestRecord};

impl GroupRepo {
    pub async fn create_group_request(
        &self,
        record: NewGroupRequestRecord,
    ) -> Result<GroupRequestEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            WITH next_id(value) AS (
                SELECT CAST(COALESCE(MAX(CAST(notification_seq AS INTEGER)), 0) + 1 AS TEXT)
                FROM group_requests
            )
            INSERT INTO group_requests (
                group_id, notification_seq, notification_type, initiator_user_id,
                target_user_id, comment, state, created_at
            ) SELECT
                ?1,
                value,
                ?2, ?3, ?4, ?5, 'pending', ?6
            FROM next_id
            RETURNING notification_seq AS id, group_id, notification_type AS request_type,
                      initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(&record.group_id)
        .bind(codecs::group_request_type_to_db(record.request_type))
        .bind(&record.initiator_user_id)
        .bind(record.target_user_id.as_deref())
        .bind(record.comment.as_deref())
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn list_group_requests(
        &self,
        user_id: &str,
    ) -> Result<Vec<GroupRequestEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            SELECT
                gr.notification_seq AS id,
                gr.group_id,
                gr.notification_type AS request_type,
                gr.initiator_user_id,
                gr.target_user_id,
                gr.comment,
                gr.state,
                gr.created_at,
                gr.handled_at,
                gr.operator_user_id
            FROM group_requests gr
            LEFT JOIN group_members gm
                ON gm.group_id = gr.group_id
               AND gm.user_id = ?1
            WHERE gr.state = 'pending'
              AND (
                (
                    gr.notification_type = 'join'
                    AND gm.role IN ('owner', 'admin')
                )
                OR (
                    gr.notification_type = 'invite'
                    AND (
                        gr.target_user_id = ?1
                        OR gm.role IN ('owner', 'admin')
                    )
                )
              )
            ORDER BY gr.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_group_request_by_id(
        &self,
        request_id: &str,
    ) -> Result<Option<GroupRequestEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            SELECT notification_seq AS id, group_id, notification_type AS request_type,
                   initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            FROM group_requests
            WHERE notification_seq = ?1
            "#,
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn has_pending_group_request(
        &self,
        group_id: &str,
        request_type: GroupRequestType,
        initiator_user_id: &str,
        target_user_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM group_requests
                WHERE group_id = ?1
                  AND notification_type = ?2
                  AND initiator_user_id = ?3
                  AND ((target_user_id IS NULL AND ?4 IS NULL) OR target_user_id = ?4)
                  AND state = 'pending'
            )
            "#,
        )
        .bind(group_id)
        .bind(codecs::group_request_type_to_db(request_type))
        .bind(initiator_user_id)
        .bind(target_user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn handle_group_request(
        &self,
        request_id: &str,
        state: RequestState,
        operator_user_id: &str,
        handled_at: u64,
        joined_at: u64,
    ) -> Result<Option<GroupRequestEntity>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            UPDATE group_requests
            SET state = ?2,
                handled_at = ?3,
                operator_user_id = ?4
            WHERE notification_seq = ?1 AND state = 'pending'
            RETURNING notification_seq AS id, group_id, notification_type AS request_type,
                      initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(request_id)
        .bind(codecs::request_state_to_db(state))
        .bind(handled_at as i64)
        .bind(operator_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let handled: GroupRequestEntity = row.try_into()?;

        if state == RequestState::Accepted {
            let joined_user_id = match handled.request_type {
                GroupRequestType::Join => &handled.initiator_user_id,
                GroupRequestType::Invite => handled.target_user_id.as_ref().map(|s| s.as_str()).unwrap_or(""),
            };

            if !joined_user_id.is_empty() {
                sqlx::query(
                    r#"
                    INSERT INTO group_members (
                        group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
                    ) VALUES (?1, ?2, '', '', 'member', ?3, 0, NULL)
                    ON CONFLICT(group_id, user_id) DO NOTHING
                    "#,
                )
                .bind(&handled.group_id)
                .bind(joined_user_id)
                .bind(joined_at as i64)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(Some(handled))
    }
}