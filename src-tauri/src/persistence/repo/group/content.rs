use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity, GroupFolderEntity,
};

use super::GroupRepo;
use super::types::{GroupAnnouncementRow, GroupEssenceRow, GroupFileRow, GroupFolderRow};

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
            "#,
        )
        .bind(&announcement.announcement_id)
        .bind(announcement.group_id as i64)
        .bind(announcement.sender_user_id as i64)
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
        group_id: u64,
    ) -> Result<Vec<GroupAnnouncementEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupAnnouncementRow>(
            r#"
            SELECT announcement_id, group_id, sender_user_id, content, image_url, created_at, updated_at
            FROM group_announcements
            WHERE group_id = ?1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(GroupAnnouncementEntity::from)
            .collect())
    }

    pub async fn upsert_group_folder(&self, folder: &GroupFolderEntity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_folders (
                folder_id, group_id, parent_folder_id, folder_name, creator_user_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(folder_id) DO UPDATE SET
                parent_folder_id = excluded.parent_folder_id,
                folder_name = excluded.folder_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&folder.folder_id)
        .bind(folder.group_id as i64)
        .bind(&folder.parent_folder_id)
        .bind(&folder.folder_name)
        .bind(folder.creator_user_id as i64)
        .bind(folder.created_at as i64)
        .bind(folder.updated_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_folders(
        &self,
        group_id: u64,
    ) -> Result<Vec<GroupFolderEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFolderRow>(
            r#"
            SELECT
                gf.folder_id,
                gf.group_id,
                gf.parent_folder_id,
                gf.folder_name,
                gf.creator_user_id,
                gf.created_at,
                gf.updated_at,
                (
                    SELECT COUNT(*)
                    FROM group_files f
                    WHERE f.group_id = gf.group_id
                      AND f.parent_folder_id = gf.folder_id
                ) AS file_count
            FROM group_folders gf
            WHERE gf.group_id = ?1
            ORDER BY gf.created_at ASC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupFolderEntity::from).collect())
    }

    pub async fn upsert_group_file(&self, file: &GroupFileEntity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_files (
                file_id, group_id, parent_folder_id, file_name, file_size, file_hash, uploader_user_id, uploaded_at, expire_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(file_id) DO UPDATE SET
                parent_folder_id = excluded.parent_folder_id,
                file_name = excluded.file_name,
                file_size = excluded.file_size,
                file_hash = excluded.file_hash,
                expire_at = excluded.expire_at
            "#,
        )
        .bind(&file.file_id)
        .bind(file.group_id as i64)
        .bind(&file.parent_folder_id)
        .bind(&file.file_name)
        .bind(file.file_size as i64)
        .bind(&file.file_hash)
        .bind(file.uploader_user_id as i64)
        .bind(file.uploaded_at as i64)
        .bind(file.expire_at.map(|ts| ts as i64))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_files(
        &self,
        group_id: u64,
    ) -> Result<Vec<GroupFileEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFileRow>(
            r#"
            SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash, uploader_user_id, uploaded_at, expire_at
            FROM group_files
            WHERE group_id = ?1
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupFileEntity::from).collect())
    }

    pub async fn create_group_essence_message(
        &self,
        group_id: u64,
        message_id: i64,
        sender_user_id: u64,
        operator_user_id: u64,
        is_set: bool,
        created_at: u64,
    ) -> Result<GroupEssenceMessageEntity, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupEssenceRow>(
            r#"
            INSERT INTO group_essence_messages (
                group_id, message_id, sender_user_id, operator_user_id, is_set, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING id, group_id, message_id, sender_user_id, operator_user_id, is_set, created_at
            "#,
        )
        .bind(group_id as i64)
        .bind(message_id)
        .bind(sender_user_id as i64)
        .bind(operator_user_id as i64)
        .bind(is_set)
        .bind(created_at as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(GroupEssenceMessageEntity::from(row))
    }

    pub async fn list_group_essence_messages(
        &self,
        group_id: u64,
    ) -> Result<Vec<GroupEssenceMessageEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupEssenceRow>(
            r#"
            SELECT id, group_id, message_id, sender_user_id, operator_user_id, is_set, created_at
            FROM group_essence_messages
            WHERE group_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(GroupEssenceMessageEntity::from)
            .collect())
    }
}
