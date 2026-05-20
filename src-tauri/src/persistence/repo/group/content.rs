use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity, GroupFolderEntity,
};

use super::types::{GroupAnnouncementRow, GroupEssenceRow, GroupFileRow, GroupFolderRow};
use super::GroupRepo;

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

    pub async fn upsert_group_folder(&self, folder: &GroupFolderEntity) -> Result<(), sqlx::Error> {
        let parent = if folder.parent_folder_id.is_empty() || folder.parent_folder_id == "/" {
            None::<String>
        } else {
            Some(folder.parent_folder_id.clone())
        };

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
        .bind(&folder.group_id)
        .bind(parent)
        .bind(&folder.folder_name)
        .bind(&folder.creator_user_id)
        .bind(folder.created_at as i64)
        .bind(folder.updated_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_folders(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupFolderEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFolderRow>(
            r#"
            SELECT
                gf.folder_id,
                gf.group_id,
                COALESCE(gf.parent_folder_id, '') AS parent_folder_id,
                gf.folder_name,
                gf.creator_user_id,
                gf.created_at,
                gf.updated_at,
                COALESCE(gf.file_count, 0) AS file_count
            FROM group_folders gf
            WHERE gf.group_id = ?1
            ORDER BY gf.created_at ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
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
        .bind(&file.group_id)
        .bind(&file.parent_folder_id)
        .bind(&file.file_name)
        .bind(file.file_size as i64)
        .bind(&file.file_hash)
        .bind(&file.uploader_user_id)
        .bind(file.uploaded_at as i64)
        .bind(file.expire_at.map(|ts| ts as i64))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_files(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupFileEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFileRow>(
            r#"
            SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash,
                   uploader_user_id, uploaded_at AS uploaded_at, expire_at, download_count
            FROM group_files
            WHERE group_id = ?1
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

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
            let row = sqlx::query_as::<_, GroupEssenceRow>(
                r#"
                WITH next_id(value) AS (
                    SELECT CAST(COALESCE(MAX(CAST(essence_id AS INTEGER)), 0) + 1 AS TEXT)
                    FROM group_essence_messages
                )
                INSERT INTO group_essence_messages (
                    essence_id, group_id, message_id, sender_user_id, operator_user_id, is_set, created_at
                ) SELECT value, ?1, ?2, ?3, ?4, 1, ?5
                FROM next_id
                ON CONFLICT(group_id, message_id) DO UPDATE SET
                    sender_user_id = excluded.sender_user_id,
                    operator_user_id = excluded.operator_user_id,
                    is_set = 1,
                    created_at = excluded.created_at
                RETURNING essence_id AS id, group_id, message_id, sender_user_id, operator_user_id, is_set, created_at
                "#,
            )
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
            SELECT essence_id AS id, group_id, message_id, sender_user_id, operator_user_id, 1 AS is_set, created_at
            FROM group_essence_messages
            WHERE group_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}