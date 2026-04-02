use super::{GroupEventRecord, GroupRepo, NewGroupEventRecord};

impl GroupRepo {
    pub async fn insert_group_event(
        &self,
        record: NewGroupEventRecord,
    ) -> Result<GroupEventRecord, sqlx::Error> {
        sqlx::query_as::<_, GroupEventRecord>(
            r#"
            INSERT INTO group_events (group_id, payload, created_at)
            VALUES (?1, ?2, ?3)
            RETURNING id, group_id, payload, created_at
            "#,
        )
        .bind(record.group_id as i64)
        .bind(record.payload)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_group_events(
        &self,
        group_id: u64,
        limit: i64,
    ) -> Result<Vec<GroupEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, GroupEventRecord>(
            r#"
            SELECT id, group_id, payload, created_at
            FROM group_events
            WHERE group_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(group_id as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}
