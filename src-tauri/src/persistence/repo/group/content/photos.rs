use crate::models::GroupPhotoEntity;

use super::super::GroupRepo;
use super::super::types::GroupPhotoRow;

impl GroupRepo {
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
        group_id: &str,
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
              AND a.group_id = ?2
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(album_id)
        .bind(group_id)
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

    pub async fn delete_group_photo_and_refresh_cover(
        &self,
        photo_id: &str,
        group_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let deleted_photo: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT p.album_id, p.url
            FROM group_photos p
            JOIN group_albums a ON a.album_id = p.album_id
            WHERE p.photo_id = ?1
              AND a.group_id = ?2
            "#,
        )
        .bind(photo_id)
        .bind(group_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((album_id, deleted_url)) = deleted_photo else {
            return Ok(false);
        };

        sqlx::query("DELETE FROM group_photos WHERE photo_id = ?1")
            .bind(photo_id)
            .execute(&mut *tx)
            .await?;

        let cover_url: Option<String> =
            sqlx::query_scalar("SELECT cover_url FROM group_albums WHERE album_id = ?1")
                .bind(&album_id)
                .fetch_one(&mut *tx)
                .await?;

        if cover_url.as_deref() == Some(deleted_url.as_str()) {
            let next_cover_url: Option<String> = sqlx::query_scalar(
                r#"
                SELECT url
                FROM group_photos
                WHERE album_id = ?1
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(&album_id)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(next_cover_url) = next_cover_url {
                sqlx::query("UPDATE group_albums SET cover_url = ?1 WHERE album_id = ?2")
                    .bind(next_cover_url)
                    .bind(&album_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query("UPDATE group_albums SET cover_url = NULL WHERE album_id = ?1")
                    .bind(&album_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(true)
    }
}
