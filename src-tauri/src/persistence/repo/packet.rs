use sqlx::{QueryBuilder, SqlitePool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProtocolPacketRecord {
    pub packet_id: String,
    pub bot_id: String,
    pub profile_id: String,
    pub protocol_type: String,
    pub direction: String,
    pub action_name: String,
    pub file_path: String,
    pub related_object_type: Option<String>,
    pub related_object_id: Option<String>,
    pub is_error: i32,
    pub session_id: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct PacketRepo {
    pool: SqlitePool,
}

impl PacketRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_packets(
        &self,
        bot_id: Option<&str>,
        direction: Option<&str>,
        action_name: Option<&str>,
        since: Option<u64>,
        limit: i64,
    ) -> Result<Vec<ProtocolPacketRecord>, sqlx::Error> {
        let limit = limit.min(1000);

        let mut builder: QueryBuilder<'_, sqlx::Sqlite> =
            QueryBuilder::new("SELECT * FROM protocol_packets");

        let mut has_where = false;

        if let Some(bot_id) = bot_id {
            builder.push(" WHERE bot_id = ");
            builder.push_bind(bot_id);
            has_where = true;
        }

        if let Some(direction) = direction {
            if has_where {
                builder.push(" AND direction = ");
            } else {
                builder.push(" WHERE direction = ");
                has_where = true;
            }
            builder.push_bind(direction);
        }

        if let Some(action_name) = action_name {
            if has_where {
                builder.push(" AND action_name = ");
            } else {
                builder.push(" WHERE action_name = ");
                has_where = true;
            }
            builder.push_bind(action_name);
        }

        if let Some(since) = since {
            if has_where {
                builder.push(" AND created_at >= ");
            } else {
                builder.push(" WHERE created_at >= ");
            }
            builder.push_bind(since as i64);
        }

        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(limit);

        builder
            .build_query_as::<ProtocolPacketRecord>()
            .fetch_all(&self.pool)
            .await
    }

    pub async fn get_packet_by_id(
        &self,
        packet_id: &str,
    ) -> Result<Option<ProtocolPacketRecord>, sqlx::Error> {
        sqlx::query_as::<_, ProtocolPacketRecord>(
            "SELECT * FROM protocol_packets WHERE packet_id = ?1",
        )
        .bind(packet_id)
        .fetch_optional(&self.pool)
        .await
    }
}
