use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity,
    GroupFolderEntity, GroupPhotoEntity,
};

use super::GroupRepo;
use super::types::{
    GroupAlbumRow, GroupAnnouncementRow, GroupEssenceRow, GroupFileRow, GroupFolderRow,
    GroupPhotoRow,
};

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
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn upsert_group_file(&self, file: &GroupFileEntity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_files (
                file_id, group_id, parent_folder_id, file_name, file_size, file_hash, uploader_user_id, created_at, expire_at, file_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(file_id) DO UPDATE SET
                parent_folder_id = excluded.parent_folder_id,
                file_name = excluded.file_name,
                file_size = excluded.file_size,
                file_hash = excluded.file_hash,
                expire_at = excluded.expire_at,
                file_path = excluded.file_path
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
        .bind(&file.file_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_files(
        &self,
        group_id: &str,
        parent_folder_id: Option<&str>,
    ) -> Result<Vec<GroupFileEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFileRow>(
            r#"
            SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash,
                   uploader_user_id, created_at AS uploaded_at, expire_at, file_path, download_count
            FROM group_files
            WHERE group_id = ?1 AND (parent_folder_id = ?2 OR (?2 IS NULL AND parent_folder_id IS NULL))
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id)
        .bind(parent_folder_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_group_file_by_id(
        &self,
        file_id: &str,
    ) -> Result<Option<GroupFileEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupFileRow>(
            r#"
            SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash,
                   uploader_user_id, created_at AS uploaded_at, expire_at, file_path, download_count
            FROM group_files
            WHERE file_id = ?1
            "#,
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_group_file(&self, file_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM group_files WHERE file_id = ?1")
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_group_album(&self, album: &GroupAlbumEntity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_albums (
                album_id, group_id, name, cover_url, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&album.album_id)
        .bind(&album.group_id)
        .bind(&album.name)
        .bind(&album.cover_url)
        .bind(album.created_at as i64)
        .bind(album.updated_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_albums(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupAlbumEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupAlbumRow>(
            r#"
            SELECT
                a.album_id,
                a.group_id,
                a.name,
                a.cover_url,
                COALESCE((SELECT COUNT(*) FROM group_photos p WHERE p.album_id = a.album_id), 0) AS photo_count,
                a.created_at,
                a.updated_at
            FROM group_albums a
            WHERE a.group_id = ?1
            ORDER BY a.updated_at DESC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn delete_group_album(&self, album_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM group_albums WHERE album_id = ?1")
            .bind(album_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_group_photo(&self, photo: &GroupPhotoEntity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_photos (
                photo_id, album_id, url, file_path, description, uploader_user_id, file_size, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(&photo.photo_id)
        .bind(&photo.album_id)
        .bind(&photo.url)
        .bind(&photo.file_path)
        .bind(&photo.description)
        .bind(&photo.uploader_user_id)
        .bind(photo.file_size.map(|s| s as i64))
        .bind(photo.created_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_photos(
        &self,
        album_id: &str,
    ) -> Result<Vec<GroupPhotoEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupPhotoRow>(
            r#"
            SELECT
                p.photo_id,
                p.album_id,
                a.group_id,
                p.url,
                p.file_path,
                p.description,
                p.uploader_user_id,
                p.file_size,
                p.created_at
            FROM group_photos p
            JOIN group_albums a ON a.album_id = p.album_id
            WHERE p.album_id = ?1
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_group_photo_by_id(
        &self,
        photo_id: &str,
    ) -> Result<Option<GroupPhotoEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupPhotoRow>(
            r#"
            SELECT
                p.photo_id,
                p.album_id,
                a.group_id,
                p.url,
                p.file_path,
                p.description,
                p.uploader_user_id,
                p.file_size,
                p.created_at
            FROM group_photos p
            JOIN group_albums a ON a.album_id = p.album_id
            WHERE p.photo_id = ?1
            "#,
        )
        .bind(photo_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_group_photo(&self, photo_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM group_photos WHERE photo_id = ?1")
            .bind(photo_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
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
            let id = crate::utils::new_db_id();
            let row = sqlx::query_as::<_, GroupEssenceRow>(
                r#"
                INSERT INTO group_essence_messages (
                    essence_id, group_id, message_id, sender_user_id, operator_user_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(group_id, message_id) DO UPDATE SET
                    sender_user_id = excluded.sender_user_id,
                    operator_user_id = excluded.operator_user_id,
                    created_at = excluded.created_at
                RETURNING essence_id AS id, group_id, message_id, sender_user_id, operator_user_id, 1 AS is_set, created_at
                "#,
            )
            .bind(&id)
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
