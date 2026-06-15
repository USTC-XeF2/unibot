use crate::models::GroupFolderEntity;

use super::super::GroupRepo;
use super::super::types::GroupFolderRow;

impl GroupRepo {
    pub async fn upsert_group_folder(&self, folder: &GroupFolderEntity) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        if let Some(parent_folder_id) = folder.parent_folder_id.as_deref() {
            if parent_folder_id == folder.folder_id {
                return Err(sqlx::Error::Protocol(
                    "folder cannot be its own parent".to_string(),
                ));
            }

            let parent_group_id: Option<String> =
                sqlx::query_scalar("SELECT group_id FROM group_folders WHERE folder_id = ?1")
                    .bind(parent_folder_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if parent_group_id.as_deref() != Some(folder.group_id.as_str()) {
                return Err(sqlx::Error::RowNotFound);
            }

            let is_descendant: i64 = sqlx::query_scalar(
                r#"
                WITH RECURSIVE descendants(folder_id) AS (
                    SELECT folder_id
                    FROM group_folders
                    WHERE parent_folder_id = ?1
                      AND group_id = ?2
                    UNION
                    SELECT child.folder_id
                    FROM group_folders child
                    JOIN descendants d ON child.parent_folder_id = d.folder_id
                    WHERE child.group_id = ?2
                )
                SELECT EXISTS (
                    SELECT 1
                    FROM descendants
                    WHERE folder_id = ?3
                )
                "#,
            )
            .bind(&folder.folder_id)
            .bind(&folder.group_id)
            .bind(parent_folder_id)
            .fetch_one(&mut *tx)
            .await?;
            if is_descendant != 0 {
                return Err(sqlx::Error::Protocol(
                    "folder parent cannot be a descendant of itself".to_string(),
                ));
            }
        }

        sqlx::query(
            r#"
            INSERT INTO group_folders (
                folder_id, group_id, parent_folder_id, folder_name, creator_user_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(folder_id) DO UPDATE SET
                parent_folder_id = excluded.parent_folder_id,
                folder_name = excluded.folder_name,
                updated_at = excluded.updated_at
                WHERE group_folders.group_id = excluded.group_id
            "#,
        )
        .bind(&folder.folder_id)
        .bind(&folder.group_id)
        .bind(&folder.parent_folder_id)
        .bind(&folder.folder_name)
        .bind(&folder.creator_user_id)
        .bind(folder.created_at as i64)
        .bind(folder.updated_at as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
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
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn delete_group_folder(&self, folder_id: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Prevent deleting folders that still contain files or child folders.
        // Counts and the delete run in one transaction so a concurrent upload
        // cannot slip a file in between the check and the delete (TOCTOU).
        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM group_files WHERE parent_folder_id = ?1")
                .bind(folder_id)
                .fetch_one(&mut *tx)
                .await?;
        let child_folder_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM group_folders WHERE parent_folder_id = ?1")
                .bind(folder_id)
                .fetch_one(&mut *tx)
                .await?;
        if file_count > 0 || child_folder_count > 0 {
            return Ok(false);
        }

        let result = sqlx::query("DELETE FROM group_folders WHERE folder_id = ?1")
            .bind(folder_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_group_folder_by_id(
        &self,
        folder_id: &str,
    ) -> Result<Option<GroupFolderEntity>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupFolderRow>(
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
            WHERE gf.folder_id = ?1
            "#,
        )
        .bind(folder_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }
}
