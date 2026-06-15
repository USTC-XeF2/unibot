use crate::models::GroupAnnouncementEntity;

use super::super::GroupRepo;
use super::super::types::GroupAnnouncementRow;

impl GroupRepo {
    pub async fn upsert_announcement(
        &self,
        announcement: &GroupAnnouncementEntity,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_announcements (
                announcement_id, group_id, sender_user_id, content, image_url, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(announcement_id) DO UPDATE SET
                content = excluded.content,
                image_url = excluded.image_url,
                updated_at = excluded.updated_at
                WHERE group_announcements.group_id = excluded.group_id
            "#,
        )
        .bind(&announcement.announcement_id)
        .bind(&announcement.group_id)
        .bind(&announcement.sender_user_id)
        .bind(&announcement.content)
        .bind(&announcement.image_url)
        .bind(announcement.created_at as i64)
        .bind(announcement.updated_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_announcements(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupAnnouncementEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupAnnouncementRow>(
            r#"
            SELECT announcement_id, group_id, sender_user_id, content, image_url, created_at, updated_at
            FROM group_announcements
            WHERE group_id = ?1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn delete_announcement(
        &self,
        group_id: &str,
        announcement_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM group_announcements WHERE group_id = ?1 AND announcement_id = ?2",
        )
        .bind(group_id)
        .bind(announcement_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_announcement_or_not_found(
        &self,
        group_id: &str,
        announcement_id: &str,
    ) -> Result<(), sqlx::Error> {
        if self.delete_announcement(group_id, announcement_id).await? {
            Ok(())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }
}
