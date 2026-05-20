use super::{GroupEventRecord, GroupRepo, NewGroupEventRecord};

impl GroupRepo {
    pub async fn insert_group_event(
        &self,
        record: NewGroupEventRecord,
    ) -> Result<GroupEventRecord, sqlx::Error> {
        sqlx::query_as::<_, GroupEventRecord>(
            r#"
            WITH next_id(value) AS (
                SELECT CAST(COALESCE(MAX(CAST(event_id AS INTEGER)), 0) + 1 AS TEXT)
                FROM group_events
            )
            INSERT INTO group_events (event_id, group_id, event_type, payload_json, created_at)
            SELECT value, ?1, 'generic', ?2, ?3
            FROM next_id
            RETURNING event_id AS id, group_id, payload_json AS payload, created_at
            "#,
        )
        .bind(&record.group_id)
        .bind(&record.payload)
        .bind(record.created_at as i64)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_group_events(
        &self,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<GroupEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, GroupEventRecord>(
            r#"
            SELECT event_id AS id, group_id, payload_json AS payload, created_at
            FROM group_events
            WHERE group_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(group_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}