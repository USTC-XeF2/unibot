use crate::models::{FriendCategoryEntity, FriendRequestEntity, FriendshipEntity, RequestState};
use crate::persistence::repo::codecs;

use super::types::{FriendCategoryRow, FriendRequestRow, FriendshipDetailRow};
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
        let id = crate::utils::new_db_id();
        let row = sqlx::query_as::<_, FriendRequestRow>(
            r#"
            INSERT INTO friend_requests (
                request_id, initiator_user_id, target_user_id, comment, state, created_at
            ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5)
            RETURNING request_id, initiator_user_id, target_user_id, comment, state, created_at, handled_at
            "#,
        )
        .bind(&id)
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

    pub async fn list_friendships(
        &self,
        user_id: &str,
    ) -> Result<Vec<FriendshipEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FriendshipDetailRow>(
            r#"
            SELECT friend_user_id, friend_category_id
            FROM friendships
            WHERE owner_user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_friend_categories(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<FriendCategoryEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FriendCategoryRow>(
            r#"
            SELECT category_id, owner_user_id, name, sort_order, created_at, updated_at
            FROM friend_categories
            WHERE owner_user_id = ?1
            ORDER BY sort_order ASC, created_at ASC
            "#,
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn create_friend_category(
        &self,
        owner_user_id: &str,
        name: &str,
    ) -> Result<FriendCategoryEntity, sqlx::Error> {
        let category_id = crate::utils::new_db_id();
        let now = crate::utils::now_ts() as i64;

        sqlx::query(
            r#"
            INSERT INTO friend_categories (
                category_id, owner_user_id, name, sort_order, created_at, updated_at
            ) VALUES (?1, ?2, ?3, 0, ?4, ?4)
            "#,
        )
        .bind(&category_id)
        .bind(owner_user_id)
        .bind(name)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(FriendCategoryEntity {
            category_id,
            owner_user_id: owner_user_id.to_string(),
            name: name.to_string(),
            sort_order: 0,
            created_at: now as u64,
            updated_at: now as u64,
        })
    }

    pub async fn get_friend_category_by_id(
        &self,
        category_id: &str,
    ) -> Result<Option<FriendCategoryEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, FriendCategoryRow>(
            r#"
            SELECT category_id, owner_user_id, name, sort_order, created_at, updated_at
            FROM friend_categories
            WHERE category_id = ?1
            "#,
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn rename_friend_category(
        &self,
        owner_user_id: &str,
        category_id: &str,
        name: &str,
    ) -> Result<FriendCategoryEntity, sqlx::Error> {
        let now = crate::utils::now_ts() as i64;
        let row = sqlx::query_as::<_, FriendCategoryRow>(
            r#"
            UPDATE friend_categories
            SET name = ?3,
                updated_at = ?4
            WHERE owner_user_id = ?1 AND category_id = ?2
            RETURNING category_id, owner_user_id, name, sort_order, created_at, updated_at
            "#,
        )
        .bind(owner_user_id)
        .bind(category_id)
        .bind(name)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn delete_friend_category(&self, category_id: &str) -> Result<(), sqlx::Error> {
        // FK ON DELETE SET NULL will clear friendships.friend_category_id
        sqlx::query("DELETE FROM friend_categories WHERE category_id = ?1")
            .bind(category_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn set_friend_category(
        &self,
        owner_user_id: &str,
        friend_user_id: &str,
        category_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE friendships
            SET friend_category_id = ?3
            WHERE owner_user_id = ?1 AND friend_user_id = ?2
            "#,
        )
        .bind(owner_user_id)
        .bind(friend_user_id)
        .bind(category_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
