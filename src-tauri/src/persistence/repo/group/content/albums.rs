use crate::models::GroupAlbumEntity;

use super::super::GroupRepo;
use super::super::types::GroupAlbumRow;

impl GroupRepo {
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

    pub async fn get_group_album_by_id(
        &self,
        album_id: &str,
    ) -> Result<Option<GroupAlbumEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupAlbumRow>(
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
            WHERE a.album_id = ?1
            "#,
        )
        .bind(album_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_group_album(
        &self,
        album_id: &str,
        group_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            DELETE FROM group_photos
            WHERE album_id = ?1
              AND EXISTS (
                  SELECT 1
                  FROM group_albums
                  WHERE album_id = ?1
                    AND group_id = ?2
              )
            "#,
        )
        .bind(album_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;

        let result = sqlx::query("DELETE FROM group_albums WHERE album_id = ?1 AND group_id = ?2")
            .bind(album_id)
            .bind(group_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_album_cover_url(
        &self,
        album_id: &str,
        cover_url: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE group_albums SET cover_url = ?1 WHERE album_id = ?2")
            .bind(cover_url)
            .bind(album_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
