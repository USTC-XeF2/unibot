use sqlx::SqlitePool;

use crate::models::GroupRequestType;

mod basic;
mod content;
mod events;
mod requests;
mod schema;
mod types;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct GroupEventRecord {
    pub id: i64,
    pub group_id: u64,
    pub payload: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewGroupEventRecord {
    pub group_id: u64,
    pub payload: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct NewGroupRequestRecord {
    pub group_id: u64,
    pub request_type: GroupRequestType,
    pub initiator_user_id: u64,
    pub target_user_id: Option<u64>,
    pub comment: Option<String>,
    pub created_at: u64,
}

#[derive(Clone)]
pub struct GroupRepo {
    pool: SqlitePool,
}

impl GroupRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
