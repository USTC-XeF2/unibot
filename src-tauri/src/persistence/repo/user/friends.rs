use crate::models::{FriendRequestEntity, RequestState};
use crate::persistence::repo::codecs;

use super::types::FriendRequestRow;
use super::{FriendshipRow, NewFriendRequestRecord, UserRepo};

impl UserRepo {
    pub async fn handle_friend_request_for_target(
        &self,
        request_id: &str,
        state: RequestState,
        target_user_id: &str,
        handled_at: u64,
    ) -> Result<Option<FriendRequestEntity>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            UPDATE friend_requests
            SET state = ?2,
                handled_at = ?3
            WHERE request_id = ?1
              AND state = 'pending'
              AND target_user_id = ?4
            RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
            "#,
        )
        .bind(request_id)
        .bind(codecs::request_state_to_db(state))
        .bind(handled_at as i64)
        .bind(target_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let updated: FriendRequestEntity = row.try_into()?;

        if state == RequestState::Accepted {
            let initiator_category = format!("{}:friend:default", updated.initiator_user_id);
            let target_category = format!("{}:friend:default", updated.target_user_id);

            sqlx::query(
                r#"
                INSERT OR IGNORE INTO friendships (
                    owner_user_id, friend_user_id, friend_category_id, created_at
                ) VALUES (?1, ?2, ?3, ?4)
                "#,
            )
            .bind(&updated.initiator_user_id)
            .bind(&updated.target_user_id)
            .bind(&initiator_category)
            .bind(handled_at as i64)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT OR IGNORE INTO friendships (
                    owner_user_id, friend_user_id, friend_category_id, created_at
                ) VALUES (?1, ?2, ?3, ?4)
                "#,
            )
            .bind(&updated.target_user_id)
            .bind(&updated.initiator_user_id)
            .bind(&target_category)
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
            WITH next_id(value) AS (
                SELECT CAST(COALESCE(MAX(CAST(request_id AS INTEGER)), 0) + 1 AS TEXT)
                FROM friend_requests
            )
            INSERT INTO friend_requests (
                request_id, initiator_user_id, target_user_id, comment, state, created_at
            ) SELECT value, ?1, ?2, ?3, 'pending', ?4
            FROM next_id
            RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
            "#,
        )
        .bind(&record.initiator_user_id)
        .bind(&record.target_user_id)
        .bind(&record.comment)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn list_friend_requests(
        &self,
        user_id: &str,
    ) -> Result<Vec<FriendRequestEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            SELECT request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
            FROM friend_requests
            WHERE initiator_user_id = ?1 OR target_user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_friend_request_by_id(
        &self,
        request_id: &str,
    ) -> Result<Option<FriendRequestEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            SELECT request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
            FROM friend_requests
            WHERE request_id = ?1
            "#,
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn has_pending_friend_request_between(
        &self,
        user_a: &str,
        user_b: &str,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM friend_requests
                WHERE state = 'pending'
                  AND (
                    (initiator_user_id = ?1 AND target_user_id = ?2)
                    OR (initiator_user_id = ?2 AND target_user_id = ?1)
                  )
            )
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn are_friends(&self, user_a: &str, user_b: &str) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM friendships
                WHERE owner_user_id = ?1 AND friend_user_id = ?2
            )
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists != 0)
    }

    pub async fn remove_friendship_pair(
        &self,
        user_a: &str,
        user_b: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let affected = sqlx::query(
            r#"
            DELETE FROM friendships
            WHERE (owner_user_id = ?1 AND friend_user_id = ?2)
               OR (owner_user_id = ?2 AND friend_user_id = ?1)
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;
        Ok(affected > 0)
    }

    pub async fn list_friends(&self, user_id: &str) -> Result<Vec<FriendshipRow>, sqlx::Error> {
        sqlx::query_as::<_, FriendshipRow>(
            r#"
            SELECT friend_user_id
            FROM friendships
            WHERE owner_user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }
}
