use crate::models::UserProfile;

use super::UserRepo;
use super::types::UserRow;

impl UserRepo {
    pub async fn upsert_user(&self, profile: &UserProfile) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO users (user_id, nickname, avatar, signature)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(user_id) DO UPDATE SET
                nickname = excluded.nickname,
                avatar = excluded.avatar,
                signature = excluded.signature
            "#,
        )
        .bind(profile.user_id as i64)
        .bind(&profile.nickname)
        .bind(&profile.avatar)
        .bind(&profile.signature)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<UserProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT user_id, nickname, avatar, signature
            FROM users
            ORDER BY user_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(UserProfile::from).collect())
    }

    pub async fn get_user_by_id(&self, user_id: u64) -> Result<Option<UserProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT user_id, nickname, avatar, signature
            FROM users
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(UserProfile::from))
    }

    pub async fn delete_user(&self, user_id: u64) -> Result<bool, sqlx::Error> {
        let deleted = sqlx::query(
            r#"
            DELETE FROM users
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id as i64)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0;

        Ok(deleted)
    }
}
