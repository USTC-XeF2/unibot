use crate::models::{GroupRequestEntity, GroupRequestType, RequestState};

use super::types::GroupRequestRow;
use super::{GroupRepo, NewGroupRequestRecord};

impl GroupRepo {
    pub async fn create_group_request(
        &self,
        record: NewGroupRequestRecord,
    ) -> Result<GroupRequestEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            INSERT INTO group_requests (
                group_id, request_type, initiator_user_id, target_user_id, comment, state, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            RETURNING id, group_id, request_type, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(record.group_id as i64)
        .bind(record.request_type)
        .bind(record.initiator_user_id as i64)
        .bind(record.target_user_id.map(|id| id as i64))
        .bind(record.comment)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(GroupRequestEntity::from(row))
    }

    pub async fn list_group_requests(
        &self,
        group_id: u64,
    ) -> Result<Vec<GroupRequestEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            SELECT id, group_id, request_type, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            FROM group_requests
            WHERE group_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupRequestEntity::from).collect())
    }

    pub async fn get_group_request_by_id(
        &self,
        request_id: i64,
    ) -> Result<Option<GroupRequestEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRequestRow>(
            r#"
            SELECT id, group_id, request_type, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            FROM group_requests
            WHERE id = ?1
            "#,
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupRequestEntity::from))
    }

    pub async fn has_pending_group_request(
        &self,
        group_id: u64,
        request_type: GroupRequestType,
        initiator_user_id: u64,
        target_user_id: Option<u64>,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM group_requests
                WHERE group_id = ?1
                  AND request_type = ?2
                  AND initiator_user_id = ?3
                  AND ((target_user_id IS NULL AND ?4 IS NULL) OR target_user_id = ?4)
                  AND state = 0
            )
            "#,
        )
        .bind(group_id as i64)
        .bind(request_type)
        .bind(initiator_user_id as i64)
        .bind(target_user_id.map(|id| id as i64))
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn handle_group_request(
        &self,
        request_id: i64,
        state: RequestState,
        operator_user_id: u64,
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
            WHERE id = ?1 AND state = 0
            RETURNING id, group_id, request_type, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(request_id)
        .bind(state)
        .bind(handled_at as i64)
        .bind(operator_user_id as i64)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let handled = GroupRequestEntity::from(row);

        if state == RequestState::Accepted {
            let target_user_id = match handled.request_type {
                GroupRequestType::Join => handled.initiator_user_id,
                GroupRequestType::Invite => handled.target_user_id.unwrap_or(0),
            };

            if target_user_id != 0 {
                sqlx::query(
                    r#"
                    INSERT INTO group_members (
                        group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
                    ) VALUES (?1, ?2, '', '', 2, ?3, 0, NULL)
                    ON CONFLICT(group_id, user_id) DO NOTHING
                    "#,
                )
                .bind(handled.group_id as i64)
                .bind(target_user_id as i64)
                .bind(joined_at as i64)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(Some(handled))
    }
}
