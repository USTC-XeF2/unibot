use crate::models::{FriendRequestEntity, RequestState};

use super::types::FriendRequestRow;
use super::{FriendshipRow, NewFriendRequestRecord, UserRepo};

fn friendship_pair(user_a: u64, user_b: u64) -> (u64, u64) {
    if user_a < user_b {
        (user_a, user_b)
    } else {
        (user_b, user_a)
    }
}

impl UserRepo {
    pub async fn handle_friend_request_for_target(
        &self,
        request_id: i64,
        state: RequestState,
        target_user_id: u64,
        operator_user_id: u64,
        handled_at: u64,
    ) -> Result<Option<FriendRequestEntity>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            UPDATE friend_requests
            SET state = ?2,
                handled_at = ?3,
                operator_user_id = ?4
            WHERE id = ?1 AND state = 0 AND target_user_id = ?5
            RETURNING id, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(request_id)
        .bind(state)
        .bind(handled_at as i64)
        .bind(operator_user_id as i64)
        .bind(target_user_id as i64)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let updated = FriendRequestEntity::from(row);

        if state == RequestState::Accepted {
            let (user_low_id, user_high_id) =
                friendship_pair(updated.initiator_user_id, updated.target_user_id);

            sqlx::query(
                r#"
                INSERT OR IGNORE INTO friendships (user_low_id, user_high_id, created_at)
                VALUES (?1, ?2, ?3)
                "#,
            )
            .bind(user_low_id as i64)
            .bind(user_high_id as i64)
            .bind(handled_at as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(Some(updated))
    }

    pub async fn create_friend_request(
        &self,
        record: NewFriendRequestRecord,
    ) -> Result<FriendRequestEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            INSERT INTO friend_requests (
                initiator_user_id, target_user_id, comment, state, created_at
            ) VALUES (?1, ?2, ?3, 0, ?4)
            RETURNING id, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            "#,
        )
        .bind(record.initiator_user_id as i64)
        .bind(record.target_user_id as i64)
        .bind(record.comment)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(FriendRequestEntity::from(row))
    }

    pub async fn list_friend_requests(
        &self,
        user_id: u64,
    ) -> Result<Vec<FriendRequestEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            SELECT id, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            FROM friend_requests
            WHERE initiator_user_id = ?1 OR target_user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(FriendRequestEntity::from).collect())
    }

    pub async fn get_friend_request_by_id(
        &self,
        request_id: i64,
    ) -> Result<Option<FriendRequestEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            SELECT id, initiator_user_id, target_user_id, comment, state, created_at, handled_at, operator_user_id
            FROM friend_requests
            WHERE id = ?1
            "#,
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(FriendRequestEntity::from))
    }

    pub async fn has_pending_friend_request_between(
        &self,
        user_a: u64,
        user_b: u64,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM friend_requests
                WHERE state = 0
                  AND (
                    (initiator_user_id = ?1 AND target_user_id = ?2)
                    OR (initiator_user_id = ?2 AND target_user_id = ?1)
                  )
            )
            "#,
        )
        .bind(user_a as i64)
        .bind(user_b as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn are_friends(&self, user_a: u64, user_b: u64) -> Result<bool, sqlx::Error> {
        let (user_low_id, user_high_id) = friendship_pair(user_a, user_b);

        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM friendships
                WHERE user_low_id = ?1 AND user_high_id = ?2
            )
            "#,
        )
        .bind(user_low_id as i64)
        .bind(user_high_id as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn remove_friendship_pair(
        &self,
        user_a: u64,
        user_b: u64,
    ) -> Result<bool, sqlx::Error> {
        let (user_low_id, user_high_id) = friendship_pair(user_a, user_b);

        let mut tx = self.pool.begin().await?;

        let affected = sqlx::query(
            r#"
            DELETE FROM friendships
            WHERE user_low_id = ?1 AND user_high_id = ?2
            "#,
        )
        .bind(user_low_id as i64)
        .bind(user_high_id as i64)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;
        Ok(affected > 0)
    }

    pub async fn list_friends(&self, user_id: u64) -> Result<Vec<FriendshipRow>, sqlx::Error> {
        sqlx::query_as::<_, FriendshipRow>(
            r#"
            SELECT
                CASE
                    WHEN user_low_id = ?1 THEN user_high_id
                    ELSE user_low_id
                END AS friend_user_id
            FROM friendships
            WHERE user_low_id = ?1 OR user_high_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await
    }
}
