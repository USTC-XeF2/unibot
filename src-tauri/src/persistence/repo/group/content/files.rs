use crate::models::GroupFileEntity;

use super::super::GroupRepo;
use super::super::types::GroupFileRow;

impl GroupRepo {
    pub async fn upsert_group_file(&self, file: &GroupFileEntity) -> Result<(), sqlx::Error> {
        if let Some(parent_folder_id) = file.parent_folder_id.as_deref() {
            let parent_group_id: Option<String> =
                sqlx::query_scalar("SELECT group_id FROM group_folders WHERE folder_id = ?1")
                    .bind(parent_folder_id)
                    .fetch_optional(&self.pool)
                    .await?;
            if parent_group_id.as_deref() != Some(file.group_id.as_str()) {
                return Err(sqlx::Error::RowNotFound);
            }
        }

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
                WHERE group_files.group_id = excluded.group_id
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
            WHERE group_id = ?1
              AND parent_folder_id IS ?2
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id)
        .bind(parent_folder_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn list_group_files_in_folder_tree(
        &self,
        group_id: &str,
        folder_id: &str,
    ) -> Result<Vec<GroupFileEntity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupFileRow>(
            r#"
            WITH RECURSIVE descendants(folder_id) AS (
                SELECT folder_id
                FROM group_folders
                WHERE group_id = ?1
                  AND folder_id = ?2
                UNION
                SELECT child.folder_id
                FROM group_folders child
                JOIN descendants d ON child.parent_folder_id = d.folder_id
                WHERE child.group_id = ?1
            )
            SELECT file_id, group_id, parent_folder_id, file_name, file_size, file_hash,
                   uploader_user_id, created_at AS uploaded_at, expire_at, file_path, download_count
            FROM group_files
            WHERE group_id = ?1
              AND parent_folder_id IN (SELECT folder_id FROM descendants)
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id)
        .bind(folder_id)
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

    pub async fn increment_group_file_download_count(
        &self,
        file_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE group_files SET download_count = download_count + 1 WHERE file_id = ?1",
        )
        .bind(file_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
