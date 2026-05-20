use crate::models::UserProfile;

use super::UserRepo;
use super::types::UserRow;

impl UserRepo {
    pub async fn upsert_user(&self, profile: &UserProfile) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO im_accounts (
                user_id, nickname, avatar_url, signature, account_source,
                account_status, unavailable_at, deleted_at
            )
            VALUES (?1, ?2, ?3, ?4, 'simulated', 'active', NULL, NULL)
            ON CONFLICT(user_id) DO UPDATE SET
                nickname = excluded.nickname,
                avatar_url = excluded.avatar_url,
                signature = excluded.signature,
                account_status = 'active',
                unavailable_at = NULL,
                deleted_at = NULL,
                updated_at = unixepoch() * 1000
            "#,
        )
        .bind(&profile.user_id)
        .bind(&profile.nickname)
        .bind(&profile.avatar)
        .bind(&profile.signature)
        .execute(&self.pool)
        .await?;

        let default_friend_category = format!("{}:friend:default", profile.user_id);
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO friend_categories (category_id, owner_user_id, name, sort_order)
            VALUES (?1, ?2, '默认分组', 0)
            "#,
        )
        .bind(&default_friend_category)
        .bind(&profile.user_id)
        .execute(&self.pool)
        .await?;

        let default_group_category = format!("{}:group:default", profile.user_id);
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO group_categories (category_id, owner_user_id, name, sort_order)
            VALUES (?1, ?2, '默认分组', 0)
            "#,
        )
        .bind(&default_group_category)
        .bind(&profile.user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<UserProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT user_id, nickname, avatar_url, signature, account_status
            FROM im_accounts
            WHERE account_status != 'deleted'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<UserProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT user_id, nickname, avatar_url, signature, account_status
            FROM im_accounts
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let rows = sqlx::query(
            r#"
            UPDATE im_accounts
            SET account_status = 'deleted',
                deleted_at = unixepoch() * 1000,
                updated_at = unixepoch() * 1000
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows == 0 {
            tx.commit().await?;
            return Ok(false);
        }

        sqlx::query(
            r#"
            UPDATE chat_groups
            SET group_status = 'dissolved',
                dissolved_at = COALESCE(dissolved_at, unixepoch() * 1000),
                updated_at = unixepoch() * 1000
            WHERE group_owner_user_id = ?1
              AND group_status != 'dissolved'
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}
